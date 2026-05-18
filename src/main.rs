use eframe::egui::{ self, Panel, TextureHandle, Color32 };
use serde::{ Deserialize, Serialize };
use zstd;
use rayon::prelude::*;
fn main() -> eframe::Result {
    eframe::run_native(
        "SplatterIron",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(MyApp::default())))
    )
}
fn alpha_blend(dst: egui::Color32, src: egui::Color32) -> egui::Color32 {
    let sa = (src.a() as f32) / 255.0;
    let da = (dst.a() as f32) / 255.0;

    let out_a = sa + da * (1.0 - sa);

    if out_a <= 0.0 {
        return egui::Color32::TRANSPARENT;
    }

    let r = (((src.r() as f32) * sa + (dst.r() as f32) * da * (1.0 - sa)) / out_a) as u8;
    let g = (((src.g() as f32) * sa + (dst.g() as f32) * da * (1.0 - sa)) / out_a) as u8;
    let b = (((src.b() as f32) * sa + (dst.b() as f32) * da * (1.0 - sa)) / out_a) as u8;

    egui::Color32::from_rgba_unmultiplied(r, g, b, (out_a * 255.0) as u8)
}

fn blend_layers(bottom: &[Color32], top: &[Color32], output: &mut [Color32]) {
    for i in 0..bottom.len() {
        output[i] = alpha_blend(bottom[i], top[i]);
    }
}
fn composite_all_layers(layers: &[Vec<Color32>], output: &mut Vec<Color32>) {
    if layers.is_empty() {
        return;
    }

    output.copy_from_slice(&layers[0]);

    for layer in &layers[1..] {
        for i in 0..output.len() {
            output[i] = alpha_blend(output[i], layer[i]);
        }
    }
}

fn composite_layers_parallel(layers: &[Layer], output: &mut [egui::Color32]) {
    if layers.is_empty() {
        return;
    }

    output.copy_from_slice(&layers[0].pixels);

    output
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, out_px)| {
            let mut px = layers[0].pixels[i];

            for layer in &layers[1..] {
                px = alpha_blend(px, layer.pixels[i]);
            }

            *out_px = px;
        });
}

#[derive(Default, Clone, Serialize, Deserialize)]
struct Layer {
    pixels: Vec<egui::Color32>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Canvas {
    pixels: Vec<Layer>,
    height: u32,
    width: u32,
    #[serde(skip)]
    rendered_layers: Option<TextureHandle>,
    #[serde(skip)]
    placeholder_texture: Option<TextureHandle>,
    #[serde(skip)]
    output_pixels: Vec<Color32>,
    render_next_frame: bool,
}

struct MyApp {
    savefile_path: String,
    canvas: Canvas,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            savefile_path: String::new(),
            canvas: Canvas::default(),
        }
    }
}

impl Default for Canvas {
    fn default() -> Self {
        let mut rng = rand::thread_rng();

        let layers: Vec<Layer> = vec![Layer {
            pixels: (0..12 * 1_000_000)
                .map(|_| {
                    egui::Color32::from_rgba_unmultiplied(
                        rand::Rng::gen_range(&mut rng, 0..255),
                        rand::Rng::gen_range(&mut rng, 0..255),
                        rand::Rng::gen_range(&mut rng, 0..255),
                        255
                    )
                })
                .collect(),
        }];
        Self {
            pixels: layers,
            height: 3000,
            width: 4000,
            output_pixels: Vec::new(),
            rendered_layers: None,
            placeholder_texture: None,
            render_next_frame: true,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.canvas.render_next_frame || self.canvas.rendered_layers.is_none() {
            self.canvas.render_next_frame = false;

            let size = (self.canvas.width as usize) * (self.canvas.height as usize);

            if self.canvas.output_pixels.len() != size {
                self.canvas.output_pixels = vec![egui::Color32::TRANSPARENT; size];
            }

            composite_layers_parallel(&self.canvas.pixels, &mut self.canvas.output_pixels);
            let image_size = [self.canvas.width as usize, self.canvas.height as usize];
            let source_size_vec2 = egui::Vec2::new(
                self.canvas.width as f32,
                self.canvas.height as f32
            );
            let image = egui::ColorImage {
                pixels: self.canvas.output_pixels.clone(),
                size: image_size,
                source_size: source_size_vec2,
            };

            match &mut self.canvas.rendered_layers {
                Some(tex) => {
                    tex.set(image, egui::TextureOptions::LINEAR);
                }
                None => {
                    self.canvas.rendered_layers = Some(
                        ui
                            .ctx()
                            .load_texture("rendered_layers", image, egui::TextureOptions::LINEAR)
                    );
                }
            }
        }

        let mut is_quitting = false;
        Panel::top("top").show_inside(ui, |top_panel| {
            // ui.label("Toolbar");
            top_panel.horizontal(|top_panel_alignment| {
                let save_button = top_panel_alignment.button("Save");
                if save_button.clicked() {
                    todo!();
                }

                let load_button = top_panel_alignment.button("Load");
                if load_button.clicked() {
                    todo!();
                }
                let new_button = top_panel_alignment.button("New");
                if new_button.clicked() {
                    todo!();
                }
                let export_button = top_panel_alignment.button("Export").clicked();
                if export_button {
                    todo!();
                }
                let import_button = top_panel_alignment.button("Import");
                if import_button.clicked() {
                    todo!();
                }
                let close_button = top_panel_alignment.button("Close");
                if close_button.clicked() {
                    is_quitting = true;
                }
            });
        });

        Panel::left("side").show_inside(
            ui,
            |ui| {
                // if ui.button("Add").clicked() {
                //     self.counter += 1;
                // }
            }
        );

        egui::CentralPanel::default().show_inside(ui, |ui| {
            // ui.heading(format!("Counter: {}", self.counter));
            // ui .text_edit_singleline(&mut self.name);

            if let Some(tex) = &self.canvas.rendered_layers {
                egui::Frame
                    ::none()
                    .fill(egui::Color32::BLACK)
                    .show(ui, |ui| {
                        let size = egui::vec2(800.0, 600.0);

                        ui.allocate_ui(size, |ui| {
                            if let Some(tex) = &self.canvas.rendered_layers {
                                ui.image(tex);
                            }
                        });
                    });
            }
        });
        if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
