use std::{ default, time::Duration };

use eframe::egui::{ self, Panel, TextureHandle, Color32 };
use serde::{ self, Deserialize, Serialize };
use zstd;
use rayon::prelude::*;
use rand::{ rng, RngExt };

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum CurrentTool {
    SquareTool,
    CircleTool,
    SquareEraserTool,
    CircleEraserTool,
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
    #[serde(skip)]
    output_rgba: Vec<u8>,
    render_next_frame: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum RenderState {
    Warm(Duration),
    Cold,
    Frozen,
}

struct MyApp {
    savefile_path: String,
    current_tool: CurrentTool,
    current_color: Color32,
    current_layer: usize,
    past_tool: Option<CurrentTool>, //CurrentTool,
    past_position: Option<(u32, u32)>,
    radius: u32,
    canvas: Canvas,
    input_color_text: String,
    input_radius_text: String,
    render_state: RenderState,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            savefile_path: String::new(),
            canvas: Canvas::default(),
            render_state: RenderState::Cold,
            current_tool: CurrentTool::SquareTool,
            current_color: Color32::from_rgba_unmultiplied(255, 255, 255, 255),
            current_layer: 0,
            radius: 100,
            input_color_text: String::from("(255, 255, 255, 255)"),
            input_radius_text: String::from("100"),
            past_tool: None, //CurrentTool::SquareTool,
            past_position: None,
        }
    }
}

impl Default for Canvas {
    fn default() -> Self {
        // let mut rng = rand::thread_rng();
        let mut rng = rng();
        // let layers: Vec<Layer> = vec![Layer {
        //     pixels: (0..12 * 1_000_000)
        //         .map(|_| {
        //             egui::Color32::from_rgba_unmultiplied(
        //                 rng.random_range(0..255),
        //                 rng.random_range(0..255),
        //                 rng.random_range(0..255),
        //                 255
        //             )
        //         })
        //         .collect(),
        // }];
        let layers: Vec<Layer> = vec![Layer {
            pixels: vec![egui::Color32::TRANSPARENT; 12 * 1_000_000],
        }];
        Self {
            pixels: layers,

            height: 3000,
            width: 4000,
            output_rgba: Vec::new(),
            output_pixels: Vec::new(),

            rendered_layers: None,
            placeholder_texture: None,
            render_next_frame: true,
        }
    }
}

fn draw_square(
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
    canvas: &mut Canvas,
    color: Color32
) {
    for x in start_x..end_x {
        for y in start_y..end_y {
            canvas.pixels[0].pixels[(x + y * canvas.width) as usize] = color;
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
            for (i, c) in self.canvas.output_pixels.iter().enumerate() {
                let base = i * 4;
                self.canvas.output_rgba[base] = c.r();
                self.canvas.output_rgba[base + 1] = c.g();
                self.canvas.output_rgba[base + 2] = c.b();
                self.canvas.output_rgba[base + 3] = c.a();
            }
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [self.canvas.width as usize, self.canvas.height as usize],
                &self.canvas.output_rgba
            );
            // let image = egui::ColorImage {
            //     pixels: image,
            //     size: image_size,
            //     source_size: source_size_vec2,
            // };

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

        Panel::left("side").show_inside(ui, |ui| {
            // if ui.button("Add").clicked() {
            //     self.counter += 1;
            // }
            // Square paint tool
            let square_paint_tool_button = ui.button("Square Tool");
            if square_paint_tool_button.clicked() {
                self.current_tool = CurrentTool::SquareTool;
            }
        });

        let central_response = egui::CentralPanel
            ::default()

            .show_inside(ui, |ui| {
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
                    // let rect = ui.max_rect();
                    // let response = ui.allocate_rect(rect, egui::Sense::hover());

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

                            // println!("uv = {:?}, px = {}, {}", uv, pixel_x, pixel_y);

                            match self.current_tool {
                                CurrentTool::SquareTool => {
                                    // Handle square tool logic
                                    self.canvas.render_next_frame = true;
                                    // if selected at the edge of the canvas, it should only draw a partial square
                                    let half_radius = (self.radius as i32) / 2;
                                    let start_x = pixel_x.saturating_sub(half_radius as u32);
                                    let end_x = (pixel_x + (half_radius as u32)).min(
                                        self.canvas.width
                                    );
                                    let start_y = pixel_y.saturating_sub(half_radius as u32);
                                    let end_y = (pixel_y + (half_radius as u32)).min(
                                        self.canvas.height
                                    );
                                    if self.past_tool != Some(CurrentTool::SquareTool) {
                                        draw_square(
                                            start_x,
                                            start_y,
                                            end_x,
                                            end_y,
                                            &mut self.canvas,
                                            self.current_color
                                        );
                                    } else {
                                        // draw_square(
                                        //     start_x,
                                        //     start_y,
                                        //     end_x,
                                        //     end_y,
                                        //     &mut self.canvas,
                                        //     self.current_color
                                        // );
                                        //interpolate past pos
                                        // const AMOUNT_TO_INTERPOLATE: u32 = 10;
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
                                                draw_square(
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
                                    // Handle circle tool logic
                                    todo!();
                                }
                                CurrentTool::SquareEraserTool => {
                                    // Handle square eraser tool logic
                                    todo!();
                                }
                                CurrentTool::CircleEraserTool => {
                                    // Handle circle eraser tool logic
                                    todo!();
                                }
                            }
                            self.past_tool = Some(self.current_tool.clone());
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
