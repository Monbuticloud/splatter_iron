use eframe::egui::Color32;
use wide::u32x4;

/// Premultiply a straight-alpha Color32.
///
/// **Caller must supply straight (non-premultiplied) RGB.**
/// Calling this on an already-premultiplied pixel will darken colors again.
#[inline(always)]
pub fn premultiply(color: Color32) -> Color32 {
    let alpha = color.a();
    if alpha == 255 {
        return color;
    }
    if alpha == 0 {
        return Color32::TRANSPARENT;
    }

    let red = ((((color.r() as u32) * (alpha as u32) + 128) * 257) >> 16) as u8;
    let green = ((((color.g() as u32) * (alpha as u32) + 128) * 257) >> 16) as u8;
    let blue = ((((color.b() as u32) * (alpha as u32) + 128) * 257) >> 16) as u8;
    Color32::from_rgba_premultiplied(red, green, blue, alpha)
}

/// Alpha-blend premultiplied src over premultiplied dst.
/// Result is premultiplied.
#[inline(always)]
pub fn alpha_blend(dst: Color32, src: Color32) -> Color32 {
    let sr = src.r() as u32;
    let sg = src.g() as u32;
    let sb = src.b() as u32;
    let sa = src.a() as u32;

    let dr = dst.r() as u32;
    let dg = dst.g() as u32;
    let db = dst.b() as u32;
    let da = dst.a() as u32;

    let inv = 255 - sa;

    #[inline(always)]
    fn blend(d: u32, inv: u32) -> u32 {
        (d * inv + 128) >> 8
    }

    Color32::from_rgba_premultiplied(
        (sr + blend(dr, inv)) as u8,
        (sg + blend(dg, inv)) as u8,
        (sb + blend(db, inv)) as u8,
        (sa + blend(da, inv)) as u8
    )
}
const ONE28: u32x4 = u32x4::splat(128);
const TWO57: u32x4 = u32x4::splat(257);
const SIXTEEN: u32x4 = u32x4::splat(16);

/// Blend multiple premultiplied layers into an RGBA u8 output buffer.
///
/// Layers are composited bottom-to-top (index 0 = bottommost).
/// Uses SIMD (wide::u32x4) to process 4 pixels per iteration.
/// No heap allocation, no clones — only stack-local SIMD vectors.
///
/// # Panics
/// - If `layers` is empty
/// - If any layer has a different length from `layers[0]`
/// - If `output.len() != layers[0].len() * 4`
#[inline]
pub fn blend_layers(layers: &[&[Color32]], output: &mut [u8]) {
    assert!(!layers.is_empty(), "blend_layers: at least one layer required");

    let len = layers[0].len();
    #[cfg(debug_assertions)]
    for (i, layer) in layers.iter().enumerate() {
        assert_eq!(layer.len(), len, "blend_layers: layer {i} length mismatch");
    }
    assert_eq!(output.len(), len * 4, "blend_layers: output length mismatch");

    // Fast path: single layer — just copy bytes
    if layers.len() == 1 {
        let src = layers[0];
        for i in 0..len {
            let arr = src[i].to_array(); // [R, G, B, A]
            let out_idx = i * 4;
            output[out_idx]     = arr[0];
            output[out_idx + 1] = arr[1];
            output[out_idx + 2] = arr[2];
            output[out_idx + 3] = arr[3];
        }
        return;
    }

    let chunks = len / 4;

    for chunk in 0..chunks {
        let base = chunk * 4;

        // --- Load bottom layer (index 0) into SIMD accumulators ---
        let mut acc_r_arr = [0u32; 4];
        let mut acc_g_arr = [0u32; 4];
        let mut acc_b_arr = [0u32; 4];
        let mut acc_a_arr = [0u32; 4];
        {
            let btm = &layers[0];
            for j in 0..4 {
                let c = btm[base + j].to_array();
                acc_r_arr[j] = c[0] as u32;
                acc_g_arr[j] = c[1] as u32;
                acc_b_arr[j] = c[2] as u32;
                acc_a_arr[j] = c[3] as u32;
            }
        }

        let mut acc_r = u32x4::new(acc_r_arr);
        let mut acc_g = u32x4::new(acc_g_arr);
        let mut acc_b = u32x4::new(acc_b_arr);
        let mut acc_a = u32x4::new(acc_a_arr);

        // --- Blend remaining layers into accumulators ---
        for &layer_slice in &layers[1..] {
            let mut tr = [0u32; 4];
            let mut tg = [0u32; 4];
            let mut tb = [0u32; 4];
            let mut ta = [0u32; 4];
            for j in 0..4 {
                let c = layer_slice[base + j].to_array();
                tr[j] = c[0] as u32;
                tg[j] = c[1] as u32;
                tb[j] = c[2] as u32;
                ta[j] = c[3] as u32;
            }

            let top_r = u32x4::new(tr);
            let top_g = u32x4::new(tg);
            let top_b = u32x4::new(tb);
            let top_a = u32x4::new(ta);

            let inv_a = u32x4::splat(255) - top_a;

            // acc = top + ((acc * inv_a + 128) * 257) >> 16
            acc_r = top_r + (((acc_r * inv_a + ONE28) * TWO57) >> SIXTEEN);
            acc_g = top_g + (((acc_g * inv_a + ONE28) * TWO57) >> SIXTEEN);
            acc_b = top_b + (((acc_b * inv_a + ONE28) * TWO57) >> SIXTEEN);
            acc_a = top_a + (((acc_a * inv_a + ONE28) * TWO57) >> SIXTEEN);
        }

        // --- Write 4 pixels to output as bytes ---
        let rr = acc_r.to_array();
        let rg = acc_g.to_array();
        let rb = acc_b.to_array();
        let ra = acc_a.to_array();

        for j in 0..4 {
            let out_idx = (base + j) * 4;
            output[out_idx]     = rr[j] as u8;
            output[out_idx + 1] = rg[j] as u8;
            output[out_idx + 2] = rb[j] as u8;
            output[out_idx + 3] = ra[j] as u8;
        }
    }

    // Scalar remainder for pixels not covered by full chunks
    for i in chunks * 4..len {
        let mut pixel = layers[0][i];
        for &layer_slice in &layers[1..] {
            pixel = alpha_blend(pixel, layer_slice[i]);
        }
        let arr = pixel.to_array();
        let out_idx = i * 4;
        output[out_idx]     = arr[0];
        output[out_idx + 1] = arr[1];
        output[out_idx + 2] = arr[2];
        output[out_idx + 3] = arr[3];
    }
}