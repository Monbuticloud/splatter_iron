//! Wraps a [`Canvas`] with save-path tracking, layer management
//! (add/delete/move/select), and GPU texture upload logic.

use std::sync::Arc;

use eframe::egui::Color32;
use eframe::egui::{self};
use eframe::egui_wgpu::wgpu;

use crate::canvas::Canvas;
use crate::canvas::DirtyRect;
use crate::canvas::Layer;
use crate::pixel::BYTES_PER_PIXEL as RGBA_CHANNELS;
use crate::pixel::LayerBlendInfo;
use crate::pixel::{self};
use crate::undo::UndoRecord;
use crate::undo_history::UndoHistory;

const TEXTURE_NAME: &str = "rendered_layers";

/// Whether an async save operation is currently in flight.
///
/// When `InFlight`, the `Arc<Canvas>` is shared with the save thread and
/// `Arc::make_mut` would trigger an expensive clone. UI code that only needs
/// to read the canvas should skip writes when this flag is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveState {
    /// No async save is running — safe to mutate the canvas via `Arc::make_mut`.
    Idle,
    /// An async save thread holds a clone of `Arc<Canvas>`.
    InFlight,
}

/// Wraps a canvas (behind `Arc` for COW during async saves) with its save
/// path, current layer, and dirty-tracking state.
#[derive(Debug)]
pub struct Document {
    /// The canvas being edited (layers, dimensions, pixel data).
    /// Wrapped in `Arc` so async save can hold a reference while the UI
    /// thread continues drawing — `Arc::make_mut` clones only when needed.
    pub canvas: Arc<Canvas>,
    /// Filesystem path most recently saved to / loaded from, or empty.
    pub savefile_path: String,
    /// Index of the currently active layer within `canvas.pixels`.
    pub current_layer: usize,
    /// Whether unsaved changes exist since the last autosave.
    pub dirty_since_last_autosave: bool,
    /// Current save state — `InFlight` while an async save is running.
    pub save_state: SaveState,
}

impl Document {
    /// Create a new `Document` wrapping the given canvas.
    ///
    /// Initializes with an empty save path, current layer 0,
    /// and `dirty_since_last_autosave = false`.
    ///
    /// # Parameters
    ///
    /// * `canvas` — The canvas to wrap.
    pub fn new(canvas: Canvas) -> Self {
        Self {
            canvas: Arc::new(canvas),
            savefile_path: String::new(),
            current_layer: 0,
            dirty_since_last_autosave: false,
            save_state: SaveState::Idle,
        }
    }

    /// Replace the current canvas with a new one and reset document state.
    ///
    /// Clears the save path, marks the canvas as not dirty, and resets
    /// the undo history (including resizing the visited buffer).
    ///
    /// # Parameters
    ///
    /// * `canvas` — The new canvas to use.
    /// * `undo` — Undo history to clear and resize for the new canvas.
    pub fn replace_canvas(&mut self, canvas: Canvas, undo: &mut UndoHistory) {
        self.canvas = Arc::new(canvas);
        self.savefile_path.clear();
        self.dirty_since_last_autosave = false;
        undo.clear();
        undo.resize_visited((self.canvas.width * self.canvas.height) as usize);
        self.canvas_mut().dirty_rect.request_full_blend();
    }

    /// Get a mutable reference to the canvas, cloning-on-write if needed.
    ///
    /// When an async save holds an `Arc<Canvas>`, this clones the canvas
    /// (COW). When no other references exist, it's a cheap `Arc::get_mut`.
    pub fn canvas_mut(&mut self) -> &mut Canvas {
        Arc::make_mut(&mut self.canvas)
    }

    /// Blend all layers into `output_rgba` (only the dirty regions if known).
    ///
    /// Returns `Some(DirtyRect)` covering the union of all blended regions,
    /// or `None` if nothing was blended. When no dirty rects are tracked but a
    /// re-blend was requested (e.g. after undo/redo), the full canvas is blended.
    ///
    /// # Panics
    ///
    /// Panics if the underlying `blend_layers` or `blend_region` encounters
    /// mismatched layer lengths or insufficient output buffer capacity.
    pub fn blend_to_output(&mut self) -> Option<DirtyRect> {
        let canvas = self.canvas_mut();
        let pixel_count = (canvas.width as usize) * (canvas.height as usize);

        if canvas.output_rgba.len() != pixel_count * RGBA_CHANNELS {
            canvas.output_rgba = Arc::new(vec![0; pixel_count * RGBA_CHANNELS]);
        }
        // Collect pixel slices, opacity, and mode for visible layers only.
        let layer_data: Vec<LayerBlendInfo> = canvas
            .pixels
            .iter()
            .filter(|l| l.visible)
            .map(|l| LayerBlendInfo {
                pixels: l.pixels.as_slice(),
                opacity: l.opacity,
                mode: l.mode,
            })
            .collect();

        let rects = canvas.dirty_rect.take_all();
        let output = Arc::make_mut(&mut canvas.output_rgba).as_mut_slice();
        let width = canvas.width;
        let height = canvas.height;

        let result = if rects.is_empty() {
            pixel::blend_layers(&layer_data, output);
            Some(DirtyRect::new(0, 0, width - 1, height - 1))
        } else {
            let mut union_rect: Option<DirtyRect> = None;
            for rect in &rects {
                if rect.is_empty() {
                    continue;
                }
                pixel::blend_region(
                    &layer_data,
                    output,
                    width,
                    rect.min_x,
                    rect.min_y,
                    rect.max_x,
                    rect.max_y,
                );
                match union_rect {
                    Some(r) => union_rect = Some(r.union(rect)),
                    None => union_rect = Some(*rect),
                }
            }
            union_rect
        };

        result
    }

    /// Upload the blended `output_rgba` (or a sub-region) to a wgpu GPU texture.
    ///
    /// Only the pixels within `dirty` are uploaded; `None` uploads the full canvas.
    ///
    /// # Parameters
    ///
    /// * `queue` — The wgpu queue for submitting write commands.
    /// * `texture` — The destination GPU texture.
    /// * `dirty` — Optional dirty rect to upload.
    ///
    /// # Panics
    ///
    /// Panics if `dirty` coordinates exceed the texture bounds or if
    /// `output_rgba` is too small for the offset + size computation.
    pub fn upload_to_gpu(
        &self,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        dirty: &Option<DirtyRect>,
    ) {
        let canvas_width = self.canvas.width;
        let canvas_height = self.canvas.height;
        let (x, y, mut width, mut height) = match dirty {
            Some(r) => (r.min_x, r.min_y, r.width(), r.height()),
            None => (0, 0, canvas_width, canvas_height),
        };

        // Clamp to canvas bounds to prevent wgpu validation errors from
        // dirty-rect accumulation outside the canvas.
        if width == 0 || height == 0 {
            return;
        }
        width = width.min(canvas_width.saturating_sub(x));
        height = height.min(canvas_height.saturating_sub(y));

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &self.canvas.output_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: (y as usize * canvas_width as usize + x as usize) as u64 * 4,
                bytes_per_row: Some(canvas_width * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Blend all layers into `output_rgba` and upload the result via egui's texture API.
    ///
    /// Always uploads the full texture (egui's API does not support partial updates).
    /// This is the fallback path for the Glow backend.
    ///
    /// # Parameters
    ///
    /// * `ui` — The egui UI handle (used to access `load_texture`).
    pub fn render_to_texture(&mut self, ui: &egui::Ui) {
        self.blend_to_output();

        let image = egui::ColorImage::from_rgba_premultiplied(
            [self.canvas.width as usize, self.canvas.height as usize],
            &self.canvas.output_rgba[..],
        );

        let rendered = &mut self.canvas_mut().rendered_layers;
        match rendered {
            Some(tex) => {
                tex.set(image, egui::TextureOptions::LINEAR);
            }
            None => {
                *rendered = Some(ui.ctx().load_texture(
                    TEXTURE_NAME,
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
    }

    /// Append a new transparent layer to the canvas.
    ///
    /// Pushes an [`UndoRecord::AddLayer`] so that the addition can be undone.
    ///
    /// # Parameters
    ///
    /// * `undo` — Undo history to push the record onto.
    pub fn add_layer(&mut self, undo: &mut UndoHistory) {
        let layer_index = self.canvas.pixels.len();
        let width = self.canvas.width;
        let height = self.canvas.height;
        let layer = Layer {
            pixels: vec![Color32::TRANSPARENT; (width * height) as usize],
            name: format!("Layer {}", layer_index + 1),
            visible: true,
            opacity: 255,
            mode: crate::canvas::LayerMode::Normal,
        };
        undo.push_undo(UndoRecord::AddLayer {
            index: layer_index,
            layer: Box::new(layer.clone()),
        });
        {
            let canvas = self.canvas_mut();
            canvas.pixels.push(layer);
            canvas.dirty_rect.request_full_blend();
        }
        self.current_layer = layer_index;
    }

    /// Remove the layer at `index` and adjust `current_layer` if needed.
    ///
    /// `current_layer` is clamped to `[0, layers.len() - 1]` after removal.
    /// Does NOT guard against deleting the last layer — the UI layer handles that.
    /// Pushes an [`UndoRecord::DeleteLayer`] so the deletion can be undone.
    ///
    /// # Parameters
    ///
    /// * `index` — Index of the layer to remove.
    /// * `undo` — Undo history to push the record onto.
    pub fn delete_layer(&mut self, index: usize, undo: &mut UndoHistory) {
        let removed = self.canvas_mut().pixels.remove(index);
        undo.push_undo(UndoRecord::DeleteLayer {
            index,
            layer: Box::new(removed),
        });
        self.current_layer = self
            .current_layer
            .saturating_sub(1)
            .min(self.canvas.pixels.len().saturating_sub(1));
        self.canvas_mut().dirty_rect.request_full_blend();
    }

    /// Swap the layer at `index` with the one above it (`index - 1`).
    ///
    /// Pushes an [`UndoRecord::MoveLayer`] so the reorder can be undone.
    ///
    /// # Parameters
    ///
    /// * `index` — Index of the layer to move up.
    /// * `undo` — Undo history to push the record onto.
    ///
    /// # Panics
    ///
    /// Panics if `index == 0` because there is no layer above to swap with.
    pub fn move_layer_up(&mut self, index: usize, undo: &mut UndoHistory) {
        self.current_layer = index - 1;
        let canvas = self.canvas_mut();
        canvas.pixels.swap(index, index - 1);
        undo.push_undo(UndoRecord::MoveLayer {
            from_index: index,
            to_index: index - 1,
        });
        canvas.dirty_rect.request_full_blend();
    }

    /// Swap the layer at `index` with the one below it (`index + 1`).
    ///
    /// Pushes an [`UndoRecord::MoveLayer`] so the reorder can be undone.
    ///
    /// # Parameters
    ///
    /// * `index` — Index of the layer to move down.
    /// * `undo` — Undo history to push the record onto.
    ///
    /// # Panics
    ///
    /// Panics if `index >= pixels.len() - 1` because there is no layer below.
    pub fn move_layer_down(&mut self, index: usize, undo: &mut UndoHistory) {
        self.current_layer = index + 1;
        let canvas = self.canvas_mut();
        canvas.pixels.swap(index, index + 1);
        undo.push_undo(UndoRecord::MoveLayer {
            from_index: index,
            to_index: index + 1,
        });
        canvas.dirty_rect.request_full_blend();
    }

    /// Set the current (active) layer index.
    ///
    /// # Parameters
    ///
    /// * `index` — Index of the layer to select.
    pub fn select_layer(&mut self, index: usize) {
        self.current_layer = index;
    }

    /// Toggle the visibility of a layer.
    ///
    /// Pushes an [`UndoRecord::ModifyLayer`] so the change can be undone.
    ///
    /// # Parameters
    ///
    /// * `index` — Index of the layer to modify.
    /// * `undo` — Undo history to push the record onto.
    pub fn toggle_layer_visible(&mut self, index: usize, undo: &mut UndoHistory) {
        let visible_before = self.canvas.pixels.iter().filter(|p| p.visible).count();
        let canvas = self.canvas_mut();
        if let Some(l) = canvas.pixels.get_mut(index) {
            if l.visible && visible_before == 1 {
                return;
            }
            let old_visible = l.visible;
            let new_visible = !old_visible;
            l.visible = new_visible;
            undo.push_undo(UndoRecord::ModifyLayer {
                index,
                old_visible,
                old_opacity: l.opacity,
                old_name: l.name.clone(),
                old_mode: l.mode,
                new_visible,
                new_opacity: l.opacity,
                new_name: l.name.clone(),
                new_mode: l.mode,
            });
            canvas.dirty_rect.request_full_blend();
        }
    }

    /// Set the opacity (0–255) of a layer.
    ///
    /// Pushes an [`UndoRecord::ModifyLayer`] so the change can be undone.
    ///
    /// # Parameters
    ///
    /// * `index` — Index of the layer to modify.
    /// * `opacity` — New opacity value (0 = transparent, 255 = opaque).
    /// * `undo` — Undo history to push the record onto.
    pub fn set_layer_opacity(&mut self, index: usize, opacity: u8, undo: &mut UndoHistory) {
        let canvas = self.canvas_mut();
        if let Some(l) = canvas.pixels.get_mut(index) {
            let old_opacity = l.opacity;
            l.opacity = opacity;
            undo.push_undo(UndoRecord::ModifyLayer {
                index,
                old_visible: l.visible,
                old_opacity,
                old_name: l.name.clone(),
                old_mode: l.mode,
                new_visible: l.visible,
                new_opacity: opacity,
                new_name: l.name.clone(),
                new_mode: l.mode,
            });
            canvas.dirty_rect.request_full_blend();
        }
    }

    /// Set the compositing mode of a layer.
    ///
    /// Pushes an [`UndoRecord::ModifyLayer`] so the change can be undone.
    ///
    /// # Parameters
    ///
    /// * `index` — Index of the layer to modify.
    /// * `mode` — New compositing mode.
    /// * `undo` — Undo history to push the record onto.
    pub fn set_layer_mode(
        &mut self,
        index: usize,
        mode: crate::canvas::LayerMode,
        undo: &mut UndoHistory,
    ) {
        let canvas = self.canvas_mut();
        if let Some(l) = canvas.pixels.get_mut(index) {
            let old_mode = l.mode;
            l.mode = mode;
            undo.push_undo(UndoRecord::ModifyLayer {
                index,
                old_visible: l.visible,
                old_opacity: l.opacity,
                old_name: l.name.clone(),
                old_mode,
                new_visible: l.visible,
                new_opacity: l.opacity,
                new_name: l.name.clone(),
                new_mode: mode,
            });
            canvas.dirty_rect.request_full_blend();
        }
    }

    /// Rename a layer.
    ///
    /// Pushes an [`UndoRecord::ModifyLayer`] so the name change can be undone.
    ///
    /// # Parameters
    ///
    /// * `index` — Index of the layer to rename.
    /// * `name` — The new name for the layer.
    /// * `undo` — Undo history to push the record onto.
    pub fn rename_layer(&mut self, index: usize, name: String, undo: &mut UndoHistory) {
        let canvas = self.canvas_mut();
        if let Some(l) = canvas.pixels.get_mut(index) {
            let old_name = l.name.clone();
            l.name.clone_from(&name);
            undo.push_undo(UndoRecord::ModifyLayer {
                index,
                old_visible: l.visible,
                old_opacity: l.opacity,
                old_name,
                old_mode: l.mode,
                new_visible: l.visible,
                new_opacity: l.opacity,
                new_name: name,
                new_mode: l.mode,
            });
        }
    }
}
