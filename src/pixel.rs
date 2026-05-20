use eframe::egui::Color32;

/// Alpha-blend src over dst.
pub fn alpha_blend(dst: Color32, src: Color32) -> Color32 {
    let sa = (src.a() as f32) / 255.0;
    let da = (dst.a() as f32) / 255.0;

    let out_a = sa + da * (1.0 - sa);

    if out_a <= 0.0 {
        return Color32::TRANSPARENT;
    }

    let r = (((src.r() as f32) * sa + (dst.r() as f32) * da * (1.0 - sa)) / out_a) as u8;
    let g = (((src.g() as f32) * sa + (dst.g() as f32) * da * (1.0 - sa)) / out_a) as u8;
    let b = (((src.b() as f32) * sa + (dst.b() as f32) * da * (1.0 - sa)) / out_a) as u8;

    Color32::from_rgba_unmultiplied(r, g, b, (out_a * 255.0) as u8)
}

/// Blend two pixel buffers element-wise.
pub fn blend_layers(bottom: &[Color32], top: &[Color32], output: &mut [Color32]) {
    for i in 0..bottom.len() {
        output[i] = alpha_blend(bottom[i], top[i]);
    }
}
