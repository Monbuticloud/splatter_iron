use std::path::Path;

use eframe::egui;

use crate::app::MyApp;
use crate::canvas::Canvas;
use crate::files;

impl MyApp {
    pub fn show_top_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut is_quitting = false;
        ui.horizontal(|ui| {
            let save_button = ui.button("Save");
            if save_button.clicked() {
                if self.savefile_path.is_empty() {
                    if
                        let Some(path) = rfd::FileDialog
                            ::new()
                            .add_filter("SplatterCanvas", &["splattercanvas"])
                            .set_file_name("canvas.splattercanvas")
                            .save_file()
                    {
                        let path_str = path.display().to_string();
                        self.savefile_path = if path_str.ends_with(".splattercanvas") {
                            path_str
                        } else {
                            format!("{}.splattercanvas", path_str)
                        };
                    }
                }
                if !self.savefile_path.is_empty() {
                    if let Err(e) = files::save_canvas(self) {
                        eprintln!("Save failed: {e}");
                    }
                }
                self.canvas.render_next_frame = true;
            }

            let load_button = ui.button("Load");
            if load_button.clicked() {
                if
                    let Some(path) = rfd::FileDialog
                        ::new()
                        .add_filter("SplatterCanvas", &["splattercanvas"])
                        .pick_file()
                {
                    match files::load_data_from_file(&path) {
                        Ok(data) => {
                            match files::load_app_from_data(&data) {
                                Ok(canvas) => {
                                    self.canvas = canvas;
                                    self.savefile_path = path.display().to_string();
                                    self.stroke_stack.clear();
                                    self.redo_index = 0;
                                    self.canvas.render_next_frame = true;
                                }
                                Err(e) => {
                                    eprintln!("Failed to load canvas: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to read file: {e}");
                        }
                    }
                }
            }

            let new_button = ui.button("New");
            if new_button.clicked() {
                self.canvas = Canvas::default();
                self.stroke_stack.clear();
                self.redo_index = 0;
                self.savefile_path.clear();
                self.canvas.render_next_frame = true;
            }

            let export_button = ui.button("Export");
            if export_button.clicked() {
                if
                    let Some(path) = rfd::FileDialog
                        ::new()
                        .add_filter("AVIF Image", &["avif"])
                        .set_file_name("export.avif")
                        .save_file()
                {
                    if !self.canvas.output_rgba.is_empty() {
                        let path_str = path.display().to_string();
                        let path_str = if path_str.ends_with(".avif") {
                            path_str
                        } else {
                            format!("{path_str}.avif")
                        };
                        if
                            let Err(e) = files::export_as_image(
                                &self.canvas.output_rgba,
                                self.canvas.width,
                                self.canvas.height,
                                Path::new(&path_str),
                                image::ImageFormat::Avif
                            )
                        {
                            eprintln!("Export failed: {e}");
                        }
                    }
                }
            }

            let import_button = ui.button("Import");
            if import_button.clicked() {
                if
                    let Some(path) = rfd::FileDialog
                        ::new()
                    .add_filter(
                        "Images",
                        &["avif", "png", "jpg", "jpeg", "webp", "gif", "tiff", "tif",
                          "tga", "ico", "pnm", "pgm", "ppm", "pbm", "pam", "qoi", "exr", "hdr", "ff"],
                    )
                        .pick_file()
                {
                    match files::import_image_as_canvas(&path) {
                        Ok(canvas) => {
                            self.canvas = canvas;
                            self.savefile_path.clear();
                            self.stroke_stack.clear();
                            self.redo_index = 0;
                            self.canvas.render_next_frame = true;
                        }
                        Err(e) => {
                            eprintln!("Import failed: {e}");
                        }
                    }
                }
            }

            let close_button = ui.button("Close");
            if close_button.clicked() {
                is_quitting = true;
            }
        });
        is_quitting
    }
}
