use eframe::egui::Color32;

pub struct StrokePixel {
    pub index: u32,
    pub color_before: Color32,
    pub color_after: Color32,
}

pub struct Stroke {
    pub layer_index: usize,
    #[allow(dead_code)]
    pub width: u32,
    pub pixels: Vec<StrokePixel>,
}

#[inline]
pub fn undo_stroke(canvas: &mut crate::canvas::Canvas, stroke: &Stroke) {
    let layer = &mut canvas.pixels[stroke.layer_index];
    for pixel in &stroke.pixels {
        layer.pixels[pixel.index as usize] = pixel.color_before;
    }
}

#[inline]
pub fn redo_stroke(canvas: &mut crate::canvas::Canvas, stroke: &Stroke) {
    let layer = &mut canvas.pixels[stroke.layer_index];
    for pixel in &stroke.pixels {
        layer.pixels[pixel.index as usize] = pixel.color_after;
    }
}
