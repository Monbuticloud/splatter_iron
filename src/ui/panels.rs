//! Panel layout coordinator: renders top, left, right, and centre panels
//! in the correct egui layout order.

use eframe::egui::{self, Panel};

use crate::app::MyApp;

impl MyApp {
    /// Render all four panels (top, left, right, centre) and return whether
    /// the user requested to quit (via the top panel).
    pub(crate) fn show_panels(&mut self, ui: &mut egui::Ui) -> bool {
        let is_quitting = Panel::top("top").show_inside(ui, |ui| self.show_top_panel(ui)).inner;

        Panel::left("side").show_inside(ui, |ui| self.show_left_panel(ui));
        Panel::right("right").show_inside(ui, |ui| self.show_right_panel(ui));
        egui::CentralPanel::default().show_inside(ui, |ui| self.show_central_panel(ui));

        is_quitting
    }
}
