//! Left tool palette: tool selection buttons for square, circle, square
//! eraser, circle eraser, bucket-fill, and stamp tool with tint mode,
//! scrollable stamp gallery, and sampling-precision dropdown.

use eframe::egui;

use crate::app::MyApp;
use crate::canvas::CurrentTool;
use crate::file_io::PendingFileAction;
use crate::stamp_library::StampSampling;

/// Selection highlight color for active tool buttons.
const SELECTED_TOOL_COLOR: egui::Color32 = egui::Color32::from_rgb(128, 0, 128);

/// Thumbnail size for stamp gallery entries.
const STAMP_THUMBNAIL_SIZE: f32 = 64.0;

impl MyApp {
    /// Render the left tool panel with selectable tool buttons and stamp
    /// gallery when the stamp tool is active.
    pub fn show_left_panel(&mut self, ui: &mut egui::Ui) {
        let old_selection_color = ui.visuals().selection.bg_fill;
        ui.visuals_mut().selection.bg_fill = SELECTED_TOOL_COLOR;

        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::Square,
            "Square Tool",
        );
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::Circle,
            "Circle Tool",
        );
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::SquareEraser,
            "Square Eraser",
        );
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::CircleEraser,
            "Circle Eraser",
        );
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::BucketFill,
            "Bucket Fill",
        );
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::Stamp,
            "Stamp Tool",
        );

        ui.visuals_mut().selection.bg_fill = old_selection_color;

        // Stamp-specific controls
        if self.tool_configuration.current_tool == CurrentTool::Stamp {
            ui.separator();

            if ui.button("Load Stamp Image...").clicked() {
                self.file_io.queue_file_action(PendingFileAction::LoadStamp);
                ui.ctx().request_repaint();
            }

            // Collect deferred commands so we don't borrow self.stamp_library
            // both immutably and mutably in the same scope.
            let mut cmd_select: Option<usize> = None;
            let mut cmd_delete: Option<usize> = None;

            if !self.stamp_library.is_empty() {
                let thumbnail_size = egui::vec2(STAMP_THUMBNAIL_SIZE, STAMP_THUMBNAIL_SIZE);
                let selected = self.stamp_library.selected_index();

                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() - 80.0)
                    .show(ui, |ui| {
                        for index in 0..self.stamp_library.len() {
                            let entry = &self.stamp_library.entries()[index];
                            let is_selected = Some(index) == selected;

                            egui::Frame::NONE
                                .fill(if is_selected {
                                    SELECTED_TOOL_COLOR
                                } else {
                                    egui::Color32::TRANSPARENT
                                })
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        // Thumbnail
                                        if let Some(tex_id) = entry.texture_id() {
                                            let response = ui.add(
                                                egui::Image::new((
                                                    tex_id,
                                                    egui::vec2(
                                                        entry.width as f32,
                                                        entry.height as f32,
                                                    ),
                                                ))
                                                .fit_to_exact_size(thumbnail_size)
                                                .sense(egui::Sense::click()),
                                            );
                                            if response.clicked() {
                                                cmd_select = Some(index);
                                            }
                                            if response.double_clicked() {
                                                cmd_delete = Some(index);
                                            }
                                        } else {
                                            ui.allocate_space(thumbnail_size);
                                        }

                                        // Name + size
                                        ui.vertical(|ui| {
                                            ui.label(&entry.name);
                                            ui.label(format!("{}×{}", entry.width, entry.height));
                                        });
                                    });
                                });
                        }
                    });

                ui.separator();

                // Tint mode
                ui.label("Tint mode:");
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.tool_configuration.stamp_tinted,
                        false,
                        "Original",
                    );
                    ui.selectable_value(
                        &mut self.tool_configuration.stamp_tinted,
                        true,
                        "Tinted",
                    );
                });

                // Sampling dropdown
                ui.label("Sampling:");
                egui::ComboBox::from_label("")
                    .selected_text(match self.tool_configuration.stamp_sampling {
                        StampSampling::Nearest => "Nearest",
                        StampSampling::Bilinear => "Bilinear",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.tool_configuration.stamp_sampling,
                            StampSampling::Nearest,
                            "Nearest",
                        );
                        ui.selectable_value(
                            &mut self.tool_configuration.stamp_sampling,
                            StampSampling::Bilinear,
                            "Bilinear",
                        );
                    });
            } else {
                ui.label("No stamps loaded.");
            }

            // Apply deferred commands
            if let Some(index) = cmd_select {
                self.stamp_library.select(index);
            }
            if let Some(index) = cmd_delete {
                self.stamp_library.remove(index);
            }
        }
    }
}
