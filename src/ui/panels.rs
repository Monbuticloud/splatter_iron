//! Panel layout coordinator: renders top, left, right, and centre panels
//! in the correct egui layout order.

use eframe::egui::Panel;
use eframe::egui::{self};

use crate::app::MyApp;

impl MyApp {
    /// Render all four panels (top, left, right, centre) and return whether
    /// the user requested to quit (via the top panel).

    pub(crate) fn show_panels(&mut self, ui: &mut egui::Ui) -> bool {

        let is_quitting = Panel::top("top")
            .show_inside(ui, |ui| self.show_top_panel(ui))
            .inner;

        Panel::left("side").show_inside(ui, |ui| self.show_left_panel(ui));

        Panel::right("right").show_inside(ui, |ui| self.show_right_panel(ui));

        egui::CentralPanel::default().show_inside(ui, |ui| self.show_central_panel(ui));

        is_quitting
    }
}

#[cfg(test)]

mod tests {

    use egui_kittest::kittest::Queryable;

    #[test]

    fn show_panels_renders_all_panels() {

        let dir = tempfile::tempdir().expect("temp dir");

        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());

        let mut harness = egui_kittest::Harness::new_ui(|ui| {

            app.show_panels(ui);
        });

        harness.step();

        // Top panel buttons
        harness.get_by_label("Save");

        // Left panel buttons
        harness.get_by_label("Square Tool");

        // Right panel labels
        harness.get_by_label("Settings");

        // Status line in centre panel
        harness.get_by_label("10×10");
    }
}
