use eframe::egui::{ self, Panel, TextureHandle, Color32 };
use zstd;
fn main() -> eframe::Result {
    eframe::run_native(
        "SplatterIron",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(MyApp::default())))
    )
}
fn alpha_blend(dst: Color32, src: Color32) -> Color32 {
    let sa = (src.a() as f32) / 255.0;
    let da = (dst.a() as f32) / 255.0;

    let out_a = sa + da * (1.0 - sa);

    if out_a <= 0.0 {
        return Color32::TRANSPARENT;
    }

    let r = (((src.r() as f32) * sa + (dst.r() as f32) * da * (1.0 - sa)) / out_a) as u8;

    let g = (((src.g() as f32) * sa + (dst.g() as f32) * da * (1.0 - sa)) / out_a) as u8;

    let b = (((src.b() as f32) * sa + (dst.b() as f32) * da * (1.0 - sa)) / out_a) as u8;

    Color32::from_rgba_unmultiplied(r, g, b, (out_a * 255.0) as u8)
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

#[derive(Default, Clone)]
struct Layer {
    pixels: Vec<egui::Color32>,
}

#[derive(Clone)]
struct Canvas {
    pixels: Vec<Layer>,
    height: u32,
    width: u32,
    rendered_layers: Option<TextureHandle>,
    render_next_frame: bool,
}

impl serde::Serialize for Layer {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        todo!()
    }
}

impl<'de> serde::Deserialize<'de> for Layer {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
        where D: serde::Deserializer<'de>
    {
        todo!()
    }
}

impl serde::Serialize for Canvas {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        todo!()
    }
}

impl<'de> serde::Deserialize<'de> for Canvas {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
        where D: serde::Deserializer<'de>
    {
        todo!()
    }
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
        Self {
            pixels: vec![Layer { pixels: vec![egui::Color32::WHITE; 12 * 1000 * 1000] }],
            height: 3000,
            width: 4000,
            rendered_layers: TextureHandle::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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

        egui::CentralPanel::default().show_inside(
            ui,
            |ui| {
                // ui.heading(format!("Counter: {}", self.counter));
                // ui.text_edit_singleline(&mut self.name);
            }
        );
        if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
