use eframe::egui::{ self, Color32 };
use eframe::egui_wgpu::wgpu;

use crate::canvas::{ Canvas, Layer };
use crate::pixel::{ self, BYTES_PER_PIXEL as RGBA_CHANNELS };
use crate::undo_history::UndoHistory;

const TEXTURE_NAME: &str = "rendered_layers";

/// Wraps a canvas with its save path, current layer, and dirty-tracking state.
pub struct Document {
    pub canvas: Canvas,
    pub savefile_path: String,
    pub current_layer: usize,
    pub dirty_since_last_autosave: bool,
}

impl Document {
    /// Create a new `Document` wrapping the given canvas.
    ///
    /// Initializes with an empty save path, current layer 0,
    /// and `dirty_since_last_autosave = false`.
    pub fn new(canvas: Canvas) -> Self {
        Self {
            canvas,
            savefile_path: String::new(),
            current_layer: 0,
            dirty_since_last_autosave: false,
        }
    }

    /// Replace the current canvas with a new one and reset document state.
    ///
    /// Clears the save path, marks the canvas as not dirty, and resets
    /// the undo history (including resizing the visited buffer).
    pub fn replace_canvas(&mut self, canvas: Canvas, undo: &mut UndoHistory) {
        self.canvas = canvas;
        self.savefile_path.clear();
        self.dirty_since_last_autosave = false;
        undo.clear();
        undo.resize_visited((self.canvas.width * self.canvas.height) as usize);
        self.canvas.render_next_frame = true;
    }

    /// Blend all layers into `output_rgba` (only the dirty region if known).
    ///
    /// Returns `Some((x, y, width, height))` of the blended region,
    /// or `None` if nothing was blended (empty dirty rect).
    pub fn blend_to_output(&mut self) -> Option<(u32, u32, u32, u32)> {
        let pixel_count = (self.canvas.width as usize) * (self.canvas.height as usize);

        if self.canvas.output_rgba.len() != pixel_count * RGBA_CHANNELS {
            self.canvas.output_rgba = vec![0; pixel_count * RGBA_CHANNELS];
        }
        self.canvas.render_next_frame = false;

        let layer_slices: Vec<&[Color32]> = self.canvas.pixels
            .iter()
            .map(|l| l.pixels.as_slice())
            .collect();

        let result = if let Some(rect) = &self.canvas.dirty_rect {
            if !rect.is_empty() {
                pixel::blend_region(
                    &layer_slices,
                    &mut self.canvas.output_rgba,
                    self.canvas.width,
                    rect.min_x,
                    rect.min_y,
                    rect.max_x,
                    rect.max_y,
                );
                Some((rect.min_x, rect.min_y, rect.width(), rect.height()))
            } else {
                None
            }
        } else {
            pixel::blend_layers(&layer_slices, &mut self.canvas.output_rgba);
            Some((0, 0, self.canvas.width, self.canvas.height))
        };
        self.canvas.dirty_rect = None;

        result
    }

    /// Upload the blended `output_rgba` (or a sub-region) to a wgpu GPU texture.
    ///
    /// Only the pixels within `dirty` are uploaded; `None` uploads the full canvas.
    pub fn upload_to_gpu(
        &self,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        dirty: &Option<(u32, u32, u32, u32)>,
    ) {
        let cw = self.canvas.width;
        let ch = self.canvas.height;
        let (x, y, w, h) = dirty.unwrap_or((0, 0, cw, ch));

        if w == 0 || h == 0 {
            return;
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &self.canvas.output_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: (y as usize * cw as usize + x as usize) as u64 * 4,
                bytes_per_row: Some(cw * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }

    /// Blend all layers into `output_rgba` and upload the result via egui's texture API.
    ///
    /// Always uploads the full texture (egui's API does not support partial updates).
    /// This is the fallback path for the Glow backend.
    pub fn render_to_texture(&mut self, ui: &egui::Ui) {
        self.blend_to_output();

        let image = egui::ColorImage::from_rgba_premultiplied(
            [self.canvas.width as usize, self.canvas.height as usize],
            &self.canvas.output_rgba
        );

        match &mut self.canvas.rendered_layers {
            Some(tex) => {
                tex.set(image, egui::TextureOptions::LINEAR);
            }
            None => {
                self.canvas.rendered_layers = Some(
                    ui.ctx().load_texture(TEXTURE_NAME, image, egui::TextureOptions::LINEAR)
                );
            }
        }
    }

    /// Append a new transparent layer to the canvas.
    ///
    /// Sets `render_next_frame` to `true` so the composite is re-blended.
    pub fn add_layer(&mut self) {
        self.canvas.pixels.push(Layer {
            pixels: vec![Color32::TRANSPARENT; (self.canvas.width * self.canvas.height) as usize],
        });
        self.canvas.render_next_frame = true;
    }

    /// Remove the layer at `index` and adjust `current_layer` if needed.
    ///
    /// `current_layer` is clamped to `[0, layers.len() - 1]` after removal.
    /// Does NOT guard against deleting the last layer — the UI layer handles that.
    pub fn delete_layer(&mut self, index: usize) {
        self.canvas.pixels.remove(index);
        self.current_layer = self.current_layer
            .saturating_sub(1)
            .min(self.canvas.pixels.len().saturating_sub(1));
        self.canvas.render_next_frame = true;
    }

    /// Swap the layer at `index` with the one above it (`index - 1`).
    ///
    /// # Panics
    ///
    /// Panics if `index == 0` because there is no layer above to swap with.
    pub fn move_layer_up(&mut self, index: usize) {
        self.canvas.pixels.swap(index, index - 1);
        self.current_layer = index - 1;
        self.canvas.render_next_frame = true;
    }

    /// Swap the layer at `index` with the one below it (`index + 1`).
    ///
    /// # Panics
    ///
    /// Panics if `index >= pixels.len() - 1` because there is no layer below.
    pub fn move_layer_down(&mut self, index: usize) {
        self.canvas.pixels.swap(index, index + 1);
        self.current_layer = index + 1;
        self.canvas.render_next_frame = true;
    }

    /// Set the current (active) layer index.
    pub fn select_layer(&mut self, index: usize) {
        self.current_layer = index;
    }
}
