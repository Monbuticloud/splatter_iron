use eframe::egui::{ self, Color32 };

use crate::canvas::{ Canvas, Layer };
use crate::pixel::{ self, BYTES_PER_PIXEL as RGBA_CHANNELS };
use crate::undo_history::UndoHistory;

const TEXTURE_NAME: &str = "rendered_layers";

pub struct Document {
    pub canvas: Canvas,
    pub savefile_path: String,
    pub current_layer: usize,
    pub dirty_since_last_autosave: bool,
}

impl Document {
    /// Create a new `Document` wrapping the given canvas.
    ///
    /// Initialises with an empty save path, current layer 0,
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

    /// Blend all layers into `output_rgba` and upload the result to a GPU texture.
    ///
    /// Allocates `output_rgba` if its size does not match. Creates or updates the
    /// `rendered_layers` texture handle for display in the egui UI.
    pub fn render_to_texture(&mut self, ui: &egui::Ui) {
        let pixel_count = (self.canvas.width as usize) * (self.canvas.height as usize);

        if self.canvas.output_rgba.len() != pixel_count * RGBA_CHANNELS {
            self.canvas.output_rgba = vec![0; pixel_count * RGBA_CHANNELS];
        }
        self.canvas.render_next_frame = false;

        let layer_slices: Vec<&[Color32]> = self.canvas.pixels
            .iter()
            .map(|l| l.pixels.as_slice())
            .collect();
        pixel::blend_layers(&layer_slices, &mut self.canvas.output_rgba);
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
    pub fn move_layer_up(&mut self, index: usize) {
        self.canvas.pixels.swap(index, index - 1);
        self.current_layer = index - 1;
        self.canvas.render_next_frame = true;
    }

    /// Swap the layer at `index` with the one below it (`index + 1`).
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
