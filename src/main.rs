use eframe::egui::{ self, Panel };

fn main() -> eframe::Result {
    eframe::run_native(
        "SplatterIron",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(MyApp::default())))
    )
}

#[derive(Default)]
struct Layer {
    pixels: Vec<egui::Color32>,
}

struct Canvas {
    pixels: Vec<Layer>,
    hight: u32,
    width: u32,
    past_state: [Vec<Layer>; 10],
}

impl serde::Serialize for Layer {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        todo!()
    }
}

impl serde::Deserialize for Layer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        todo!()
    }
}

impl serde::Serialize for Canvas {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        todo!()
    }
}

impl serde::Deserialize for Canvas {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        todo!()
    }
}

struct MyApp {
    savefile_path: String,
    canvas: Canvas,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut is_quitting = false;
        Panel::top("top").show_inside(ui, |top_panel| {
            // ui.label("Toolbar");
            let save_button = top_panel.button("Save");
            if save_button.clicked() {
                todo!();
            }

            let load_button = top_panel.button("Load");
            if load_buttonbutton.clicked() {
                todo!();
            }
            let new_button = top_panel.button("New");
            if new_button.clicked() {
                todo!();
            }
            let export_button = top_panel.button("Export").clicked();
            if export_button {
                todo!();
            }
            let import_button = top_panel.button("Import");
            if button.clicked() {
                todo!();
            }
            let close_button = top_panel.button("Close");
            if close_button.clicked() {
                is_quitting = true;
            }
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
