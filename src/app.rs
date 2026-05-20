use std::time::Duration;

use eframe::egui::{ self, Color32, Panel };
use serde::{ Deserialize, Serialize };

use crate::canvas::{ self, Canvas, CurrentTool, RenderState };

pub struct MyApp {
    pub savefile_path: String,
    pub current_tool: CurrentTool,
    pub current_color: Color32,
    pub current_layer: usize,
    pub past_tool: Option<CurrentTool>,
    pub past_position: Option<(u32, u32)>,
    pub radius: u32,
    pub canvas: Canvas,
    pub input_color_text: String,
    pub input_radius_text: String,
    pub render_state: RenderState,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            savefile_path: String::new(),
            canvas: Canvas::default(),
            render_state: RenderState::Cold,
            current_tool: CurrentTool::SquareTool,
            current_color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
            current_layer: 0,
            radius: 100,
            input_color_text: String::from("(255, 255, 255, 255)"),
            input_radius_text: String::from("100"),
            past_tool: None,
            past_position: None,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if !ui.ctx().input(|i| i.viewport().focused.unwrap_or(true)) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            self.render_state = RenderState::Frozen;
            return;
        }
        let dt = Duration::from_millis((ui.ctx().input(|i| i.predicted_dt) * 1000.0) as u64);

        match self.render_state {
            RenderState::Warm(duration) => {
                self.render_state = RenderState::Warm(duration.saturating_sub(dt));
            }
            RenderState::Cold => {
                ui.request_repaint_after(dt * 2);
            }
            RenderState::Frozen => {
                self.render_state = RenderState::Cold;
                return;
            }
        }

        if
            (self.canvas.render_next_frame || self.canvas.rendered_layers.is_none()) &&
            self.render_state != RenderState::Frozen
        {
            let size = (self.canvas.width as usize) * (self.canvas.height as usize);

            if self.canvas.output_rgba.len() != size * 4 {
                self.canvas.output_rgba = vec![0; size * 4];
            }
            self.canvas.render_next_frame = false;

            if self.canvas.output_rgba.len() != size * 4 {
                self.canvas.output_rgba = vec![0; size * 4];
            }

            canvas::composite_layers_parallel_rgba(
                &self.canvas.pixels,
                &mut self.canvas.output_rgba
            );
            let image = egui::ColorImage::from_rgba_premultiplied(
                [self.canvas.width as usize, self.canvas.height as usize],
                &self.canvas.output_rgba
            );

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

        Panel::left("side").show_inside(ui, |ui| {
            let square_paint_tool_button = ui.button("Square Tool");
            if square_paint_tool_button.clicked() {
                self.current_tool = CurrentTool::SquareTool;
            }
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(tex) = &self.canvas.rendered_layers {
                let avail = ui.available_size();
                let tex_size = tex.size_vec2();

                let scale = (avail.x / tex_size.x).min(avail.y / tex_size.y);
                let draw_size = tex_size * scale;

                let response = ui.add(
                    egui::Image
                        ::new(tex)
                        .fit_to_exact_size(draw_size)
                        .sense(egui::Sense::click_and_drag())
                );

                if response.hovered() {
                    self.render_state = RenderState::Warm(Duration::from_millis(550));
                }

                if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let local = pos - response.rect.min;
                        let uv = egui::vec2(
                            local.x / response.rect.width(),
                            local.y / response.rect.height()
                        );

                        let pixel_x = (uv.x * (self.canvas.width as f32)).floor() as u32;
                        let pixel_y = (uv.y * (self.canvas.height as f32)).floor() as u32;

                        match self.current_tool {
                            CurrentTool::SquareTool => {
                                self.canvas.render_next_frame = true;
                                let half_radius = (self.radius as i32) / 2;
                                let start_x = pixel_x.saturating_sub(half_radius as u32);
                                let end_x = (pixel_x + (half_radius as u32)).min(self.canvas.width);
                                let start_y = pixel_y.saturating_sub(half_radius as u32);
                                let end_y = (pixel_y + (half_radius as u32)).min(
                                    self.canvas.height
                                );
                                if self.past_tool != Some(CurrentTool::SquareTool) {
                                    canvas::draw_square(
                                        start_x,
                                        start_y,
                                        end_x,
                                        end_y,
                                        &mut self.canvas,
                                        self.current_color
                                    );
                                } else {
                                    let amount_to_interpolate: u32 = (
                                        (self.past_position.unwrap_or((0, 0)).0 as i32) -
                                        (pixel_x as i32)
                                    )
                                        .abs()
                                        .min(48)
                                        .max(12) as u32;
                                    if let Some((past_x, past_y)) = self.past_position {
                                        for i in 1..=amount_to_interpolate {
                                            let interp_x =
                                                past_x +
                                                (
                                                    ((((pixel_x as i32) - (past_x as i32)) *
                                                        (i as i32)) /
                                                        (amount_to_interpolate as i32)) as u32
                                                );
                                            let interp_y =
                                                past_y +
                                                (
                                                    ((((pixel_y as i32) - (past_y as i32)) *
                                                        (i as i32)) /
                                                        (amount_to_interpolate as i32)) as u32
                                                );
                                            let interp_start_x = interp_x.saturating_sub(
                                                half_radius as u32
                                            );
                                            let interp_end_x = (
                                                interp_x + (half_radius as u32)
                                            ).min(self.canvas.width - 1);
                                            let interp_start_y = interp_y.saturating_sub(
                                                half_radius as u32
                                            );
                                            let interp_end_y = (
                                                interp_y + (half_radius as u32)
                                            ).min(self.canvas.height - 1);
                                            canvas::draw_square(
                                                interp_start_x,
                                                interp_start_y,
                                                interp_end_x,
                                                interp_end_y,
                                                &mut self.canvas,
                                                self.current_color
                                            );
                                        }
                                    }
                                }
                            }
                            CurrentTool::CircleTool => {
                                todo!();
                            }
                            CurrentTool::SquareEraserTool => {
                                todo!();
                            }
                            CurrentTool::CircleEraserTool => {
                                todo!();
                            }
                        }
                        self.past_tool = Some(self.current_tool);
                        self.past_position = Some((pixel_x, pixel_y));
                    }
                } else {
                    self.past_tool = None;
                    self.past_position = None;
                }
            }
        });
        egui::SidePanel::right("right").show(ui, |ui| {
            ui.label("Settings");

            ui.color_edit_button_srgba(&mut self.current_color);
            ui.add(egui::DragValue::new(&mut self.radius).clamp_range(0..=200));
        });
        if is_quitting {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
