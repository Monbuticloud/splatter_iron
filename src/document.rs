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
    pub fn new(canvas: Canvas) -> Self {
        Self {
            canvas,
            savefile_path: String::new(),
            current_layer: 0,
            dirty_since_last_autosave: false,
        }
    }

    pub fn replace_canvas(&mut self, canvas: Canvas, undo: &mut UndoHistory) {
        self.canvas = canvas;
        self.savefile_path.clear();
        self.dirty_since_last_autosave = false;
        undo.clear();
        undo.resize_visited((self.canvas.width * self.canvas.height) as usize);
        self.canvas.render_next_frame = true;
    }

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

    pub fn add_layer(&mut self) {
        self.canvas.pixels.push(Layer {
            pixels: vec![Color32::TRANSPARENT; (self.canvas.width * self.canvas.height) as usize],
        });
        self.canvas.render_next_frame = true;
    }

    pub fn delete_layer(&mut self, index: usize) {
        self.canvas.pixels.remove(index);
        self.current_layer = self.current_layer
            .saturating_sub(1)
            .min(self.canvas.pixels.len().saturating_sub(1));
        self.canvas.render_next_frame = true;
    }

    pub fn move_layer_up(&mut self, index: usize) {
        self.canvas.pixels.swap(index, index - 1);
        self.current_layer = index - 1;
        self.canvas.render_next_frame = true;
    }

    pub fn move_layer_down(&mut self, index: usize) {
        self.canvas.pixels.swap(index, index + 1);
        self.current_layer = index + 1;
        self.canvas.render_next_frame = true;
    }

    pub fn select_layer(&mut self, index: usize) {
        self.current_layer = index;
    }
}
