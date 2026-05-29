//! Left tool palette: tool selection buttons for square, circle, square
//! eraser, circle eraser, bucket-fill, stamp tool, and custom brush tool
//! with scrollable galleries, tint mode, and sampling-precision dropdowns.

use eframe::egui;

use crate::app::MyApp;
use crate::canvas::CurrentTool;
use crate::file_io::PendingFileAction;
use crate::stamp_library::{ StampSampling, StampTintMode };

/// Selection highlight color for active tool buttons.
const SELECTED_TOOL_COLOR: egui::Color32 = egui::Color32::from_rgb(128, 0, 128);

/// Thumbnail size for stamp and brush gallery entries.
const THUMBNAIL_SIZE: f32 = 64.0;

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
        ui.selectable_value(
            &mut self.tool_configuration.current_tool,
            CurrentTool::CustomBrush,
            "Custom Brush",
        );

        ui.visuals_mut().selection.bg_fill = old_selection_color;

        // Stamp-specific controls
        if self.tool_configuration.current_tool == CurrentTool::Stamp {
            ui.separator();

            if ui.button("Load Stamp Image...").clicked() {
                self.file_io.queue_file_action(PendingFileAction::LoadStamp);
                ui.ctx().request_repaint();
            }

            let mut cmd_select: Option<usize> = None;
            let mut cmd_delete: Option<usize> = None;

            if !self.stamp_library.is_empty() {
                Self::render_gallery(
                    ui,
                    self.stamp_library.len(),
                    self.stamp_library.selected_index(),
                    |index| {
                        let entry = &self.stamp_library.entries()[index];
                        (entry.name.clone(), entry.texture_id(), entry.width, entry.height)
                    },
                    &mut cmd_select,
                    &mut cmd_delete,
                );

                ui.separator();

                ui.label("Tint mode:");
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.tool_configuration.stamp_tint_mode,
                        StampTintMode::Original,
                        "Original",
                    );
                    ui.selectable_value(
                        &mut self.tool_configuration.stamp_tint_mode,
                        StampTintMode::Tinted,
                        "Tinted",
                    );
                });

                ui.label("Sampling:");
                Self::sampling_combo(ui, &mut self.tool_configuration.stamp_sampling);
            } else {
                ui.label("No stamps loaded.");
            }

            if let Some(index) = cmd_select {
                self.stamp_library.select(index);
            }
            if let Some(index) = cmd_delete {
                self.stamp_library.remove(index);
            }
        }

        // Custom brush-specific controls
        if self.tool_configuration.current_tool == CurrentTool::CustomBrush {
            ui.separator();

            if ui.button("Import Brush...").clicked() {
                self.file_io.queue_file_action(PendingFileAction::LoadBrush);
                ui.ctx().request_repaint();
            }

            let mut cmd_select: Option<usize> = None;
            let mut cmd_delete: Option<usize> = None;

            if !self.brush_library.is_empty() {
                Self::render_gallery(
                    ui,
                    self.brush_library.len(),
                    self.brush_library.selected_index(),
                    |index| {
                        let entry = &self.brush_library.entries()[index];
                        (entry.name.clone(), entry.texture_id(), entry.width, entry.height)
                    },
                    &mut cmd_select,
                    &mut cmd_delete,
                );

                ui.separator();

                ui.label("Tint mode:");
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.tool_configuration.brush_tint_mode,
                        StampTintMode::Original,
                        "Original",
                    );
                    ui.selectable_value(
                        &mut self.tool_configuration.brush_tint_mode,
                        StampTintMode::Tinted,
                        "Tinted",
                    );
                });

                ui.label("Sampling:");
                Self::sampling_combo(ui, &mut self.tool_configuration.brush_sampling);
            } else {
                ui.label("No brushes imported.");
            }

            if let Some(index) = cmd_select {
                self.brush_library.select(index);
            }
            if let Some(index) = cmd_delete {
                self.brush_library.remove(index);
            }
        }
    }

    /// Render a scrollable gallery of library entries (stamps or brushes).
    fn render_gallery(
        ui: &mut egui::Ui,
        len: usize,
        selected: Option<usize>,
        entry_fn: impl Fn(usize) -> (String, Option<egui::TextureId>, u32, u32),
        cmd_select: &mut Option<usize>,
        cmd_delete: &mut Option<usize>,
    ) {
        let thumbnail_size = egui::vec2(THUMBNAIL_SIZE, THUMBNAIL_SIZE);

        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 80.0)
            .show(ui, |ui| {
                for index in 0..len {
                    let (name, tex_id, w, h) = entry_fn(index);
                    let is_selected = Some(index) == selected;

                    egui::Frame::NONE
                        .fill(if is_selected {
                            SELECTED_TOOL_COLOR
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if let Some(tid) = tex_id {
                                    let response = ui.add(
                                        egui::Image::new((
                                            tid,
                                            egui::vec2(w as f32, h as f32),
                                        ))
                                        .fit_to_exact_size(thumbnail_size)
                                        .sense(egui::Sense::click()),
                                    );
                                    if response.clicked() {
                                        *cmd_select = Some(index);
                                    }
                                    if response.double_clicked() {
                                        *cmd_delete = Some(index);
                                    }
                                } else {
                                    ui.allocate_space(thumbnail_size);
                                }

                                ui.vertical(|ui| {
                                    ui.label(&name);
                                    ui.label(format!("{w}×{h}"));
                                });
                            });
                        });
                }
            });
    }

    /// Render a Nearest/Bilinear sampling combobox.
    fn sampling_combo(ui: &mut egui::Ui, sampling: &mut StampSampling) {
        egui::ComboBox::from_label("")
            .selected_text(match sampling {
                StampSampling::Nearest => "Nearest",
                StampSampling::Bilinear => "Bilinear",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    sampling,
                    StampSampling::Nearest,
                    "Nearest",
                );
                ui.selectable_value(
                    sampling,
                    StampSampling::Bilinear,
                    "Bilinear",
                );
            });
    }
}
