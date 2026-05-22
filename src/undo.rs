use eframe::egui::Color32;

pub struct StrokePixels {
    pub x: usize,
    pub y: usize,
    pub color_after: Color32,
    pub color_before: Color32,
}

pub struct Stroke {
    pub layer_index: usize,
    pub pixels: Vec<StrokePixels>,
}

pub fn undo_stroke(canvas: &mut crate::canvas::Canvas, stroke: Stroke) {
    let layer = &mut canvas.pixels[stroke.layer_index];
    for pixel in stroke.pixels {
        let index = pixel.y * (canvas.width as usize) + pixel.x;
        layer.pixels[index] = pixel.color_before;
    }
}
pub fn redo_stroke(canvas: &mut crate::canvas::Canvas, stroke: Stroke) {
    let layer = &mut canvas.pixels[stroke.layer_index];
    for pixel in stroke.pixels {
        let index = pixel.y * (canvas.width as usize) + pixel.x;
        layer.pixels[index] = pixel.color_after;
    }
}
