use eframe::egui::Color32;

pub struct StrokePixel {
    pub index: u32,        // y * width + x
    pub color_before: u32, // Color32 as u32 (rgba premultiplied)
    pub color_after: u32,  // Color32 as u32 (rgba premultiplied)
}

pub struct Stroke {
    pub layer_index: usize,
    pub width: u32,        // canvas width at time of stroke, for reconstructing (x,y) if needed
    pub pixels: Vec<StrokePixel>,
}

pub fn undo_stroke(canvas: &mut crate::canvas::Canvas, stroke: &Stroke) {
    let layer = &mut canvas.pixels[stroke.layer_index];
    for pixel in &stroke.pixels {
        layer.pixels[pixel.index as usize] = Color32::from_rgba_premultiplied(
            (pixel.color_before >> 24) as u8,
            (pixel.color_before >> 16) as u8,
            (pixel.color_before >> 8) as u8,
            pixel.color_before as u8,
        );
    }
}

pub fn redo_stroke(canvas: &mut crate::canvas::Canvas, stroke: &Stroke) {
    let layer = &mut canvas.pixels[stroke.layer_index];
    for pixel in &stroke.pixels {
        layer.pixels[pixel.index as usize] = Color32::from_rgba_premultiplied(
            (pixel.color_after >> 24) as u8,
            (pixel.color_after >> 16) as u8,
            (pixel.color_after >> 8) as u8,
            pixel.color_after as u8,
        );
    }
}

#[inline(always)]
pub fn color32_to_u32(c: Color32) -> u32 {
    ((c.r() as u32) << 24) | ((c.g() as u32) << 16) | ((c.b() as u32) << 8) | (c.a() as u32)
}