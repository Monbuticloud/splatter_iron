//! Dialog windows: error list, large-canvas warning, delete-layer
//! confirmation, new-canvas presets, unsaved-changes warning, stamp naming,
//! brush naming, toast notifications, and progress indicator.

use std::time::Instant;

use eframe::egui;

use crate::app::estimate_canvas_memory;
use crate::app::MEMORY_WARNING_THRESHOLD;
use crate::app::MyApp;
use crate::app::NEW_CANVAS_PRESETS;
use crate::app::PendingStamp;
use crate::app::UnsavedWarningAction;
use crate::canvas::Canvas;
use crate::file_io::PendingFileAction;

impl MyApp {
    /// Show the error-list window (dismiss, copy, dismiss-all).
    pub(crate) fn show_error_window(&mut self, ui: &mut egui::Ui) {
        if self.ui.errors.list.is_empty() {
            return;
        }
        let mut open = true;
        let mut to_dismiss: Vec<usize> = Vec::new();
        egui::Window
            ::new("Error")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ui, |ui| {
                for (index, msg) in self.ui.errors.list.iter().enumerate() {
                    ui.label(format!("Error: {msg}"));
                    ui.horizontal(|ui| {
                        if ui.button("Dismiss").clicked() {
                            to_dismiss.push(index);
                        }
                        if ui.button("Copy error").clicked() {
                            ui.ctx().copy_text(msg.clone());
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Dismiss All").clicked() {
                        to_dismiss.extend(0..self.ui.errors.list.len());
                    }
                });
            });

        to_dismiss.sort_unstable_by(|a, b| b.cmp(a));
        to_dismiss.dedup();
        for i in to_dismiss {
            self.ui.errors.list.remove(i);
        }
        if !open {
            self.ui.errors.list.clear();
        }
    }
}
