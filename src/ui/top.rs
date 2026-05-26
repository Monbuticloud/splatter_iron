use eframe::egui;

use crate::app::{ MyApp, PendingFileAction };
use crate::canvas::Canvas;
use crate::undo::{ undo_stroke, redo_stroke };

impl MyApp {
    pub fn show_top_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut is_quitting = false;
        ui.horizontal(|ui| {
            // Save
            if ui.button("Save").clicked() {
                if self.savefile_path.is_empty() {
                    self.pending_file_action = Some(PendingFileAction::Save);
                    ui.ctx().request_repaint();
                } else {
                    if let Err(e) = crate::files::save_canvas(self) {
                        eprintln!("Save failed: {e}");
                    }
                }
                self.canvas.render_next_frame = true;
            }

            // Load
            if ui.button("Load").clicked() {
                self.pending_file_action = Some(PendingFileAction::Load);
                ui.ctx().request_repaint();
            }

            // New
            if ui.button("New").clicked() {
                self.replace_canvas(Canvas::default());
            }

            // Export menu with all supported formats
            ui.menu_button("Export", |ui| {
                let export_formats: &[( &str, &[&str], image::ImageFormat )] = &[
                    ("AVIF",    &["avif"],                 image::ImageFormat::Avif),
                    ("PNG",     &["png"],                  image::ImageFormat::Png),
                    ("JPEG",    &["jpg", "jpeg"],          image::ImageFormat::Jpeg),
                    ("WebP",    &["webp"],                 image::ImageFormat::WebP),
                    ("GIF",     &["gif"],                  image::ImageFormat::Gif),
                    ("TIFF",    &["tiff", "tif"],          image::ImageFormat::Tiff),
                    ("TGA",     &["tga"],                  image::ImageFormat::Tga),
                    ("ICO",     &["ico"],                  image::ImageFormat::Ico),
                    ("PNM",     &["pnm", "pgm", "ppm", "pbm", "pam"], image::ImageFormat::Pnm),
                    ("QOI",     &["qoi"],                  image::ImageFormat::Qoi),
                    ("EXR",     &["exr"],                  image::ImageFormat::OpenExr),
                    ("HDR",     &["hdr"],                  image::ImageFormat::Hdr),
                    ("Farbfeld",&["ff"],                   image::ImageFormat::Farbfeld),
                ];

                for &(label, extensions, fmt) in export_formats {
                    if ui.button(label).clicked() {
                        self.pending_file_action = Some(PendingFileAction::Export { extensions, fmt });
                        ui.ctx().request_repaint();
                        ui.close();
                    }
                }
            });

            // Import
            if ui.button("Import").clicked() {
                self.pending_file_action = Some(PendingFileAction::Import);
                ui.ctx().request_repaint();
            }

            ui.separator();

            // Undo / Redo buttons
            let undo_btn = ui.button("Undo");
            let redo_btn = ui.button("Redo");

            // Undo: button or keyboard shortcut
            if
                self.redo_index < self.stroke_stack.len() &&
                (ui.input(
                    |i| i.key_pressed(egui::Key::Z) && i.modifiers.command && !i.modifiers.shift
                ) || undo_btn.clicked())
            {
                let count = self.undo_redo_strength.min(self.stroke_stack.len() - self.redo_index);
                for _ in 0..count {
                    let idx = self.stroke_stack.len() - 1 - self.redo_index;
                    undo_stroke(&mut self.canvas, &self.stroke_stack[idx]);
                    self.redo_index += 1;
                }
                self.canvas.render_next_frame = true;
            }

            // Redo: button, cmd+shift+Z, or cmd+Y
            if
                self.redo_index > 0 &&
                (ui.input(
                    |i| i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift
                ) ||
                    ui.input(|i| i.key_pressed(egui::Key::Y) && i.modifiers.command) ||
                    redo_btn.clicked())
            {
                let count = self.undo_redo_strength.min(self.redo_index);
                for _ in 0..count {
                    let idx = self.stroke_stack.len() - self.redo_index;
                    self.redo_index -= 1;
                    redo_stroke(&mut self.canvas, &self.stroke_stack[idx]);
                }
                self.canvas.render_next_frame = true;
            }

            ui.separator();

            // Close
            if ui.button("Close").clicked() {
                is_quitting = true;
            }
        });
        is_quitting
    }
}