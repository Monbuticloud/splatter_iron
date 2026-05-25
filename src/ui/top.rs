use eframe::egui;

use crate::{ app::MyApp, files::get_save_data };

impl MyApp {
    #[inline(always)]
    pub fn show_top_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let mut is_quitting = false;
        ui.horizontal(|ui| {
            let save_button = ui.button("Save");
            if save_button.clicked() {
                get_save_data(app);
            }

            let load_button = ui.button("Load");
            if load_button.clicked() {
                todo!();
            }
            let new_button = ui.button("New");
            if new_button.clicked() {
                todo!();
            }
            let export_button = ui.button("Export").clicked();
            if export_button {
                todo!();
            }
            let import_button = ui.button("Import");
            if import_button.clicked() {
                todo!();
            }
            let close_button = ui.button("Close");
            if close_button.clicked() {
                is_quitting = true;
            }
        });
        is_quitting
    }
}
