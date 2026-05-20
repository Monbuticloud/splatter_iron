use eframe::egui::Color32;

/// Premultiply a straight-alpha Color32.
pub fn premultiply(color: Color32) -> Color32 {
    let alpha = color.a();
    if alpha == 255 {
        return color;
    }
    if alpha == 0 {
        return Color32::TRANSPARENT;
    }

    let red = (((color.r() as u32) * (alpha as u32) + 127) / 255) as u8;
    let green = (((color.g() as u32) * (alpha as u32) + 127) / 255) as u8;
    let blue = (((color.b() as u32) * (alpha as u32) + 127) / 255) as u8;
    Color32::from_rgba_premultiplied(red, green, blue, alpha)
}

/// Alpha-blend premultiplied src over premultiplied dst.
/// Result is premultiplied.
pub fn alpha_blend(destination: Color32, source: Color32) -> Color32 {
    let source_alpha = source.a() as u32;
    let destination_alpha = destination.a() as u32;
    let inverse_source_alpha = 255 - source_alpha;

    let red = (source.r() as u32) + ((destination.r() as u32) * inverse_source_alpha) / 255;
    let green = (source.g() as u32) + ((destination.g() as u32) * inverse_source_alpha) / 255;
    let blue = (source.b() as u32) + ((destination.b() as u32) * inverse_source_alpha) / 255;

    let alpha = source_alpha + (destination_alpha * inverse_source_alpha) / 255;

    Color32::from_rgba_premultiplied(red as u8, green as u8, blue as u8, alpha as u8)
}

/// Blend two premultiplied pixel buffers element-wise.
pub fn blend_layers(bottom: &[Color32], top: &[Color32], output: &mut [Color32]) {
    for index in 0..bottom.len() {
        output[index] = alpha_blend(bottom[index], top[index]);
    }
}
