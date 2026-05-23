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
/// Blend two premultiplied pixel buffers element-wise, using SIMD (wide::u32x4)
/// to process **4 pixels per iteration** — each u32x4 lane holds the same channel
/// from 4 different pixels, achieving genuine pixel-level parallelism.
#[inline]
pub fn blend_layers(bottom: &[Color32], top: &[Color32], output: &mut [Color32]) {
    assert_eq!(bottom.len(), top.len());
    assert_eq!(bottom.len(), output.len());

    let len = bottom.len();
    let chunks = len / 4;

    for chunk in 0..chunks {
        let base = chunk * 4;

        // Gather R/G/B/A from 4 top pixels into four u32x4
        let mut tr = [0u32; 4];
        let mut tg = [0u32; 4];
        let mut tb = [0u32; 4];
        let mut ta = [0u32; 4];
        // Same for bottom pixels
        let mut dr = [0u32; 4];
        let mut dg = [0u32; 4];
        let mut db = [0u32; 4];
        let mut da = [0u32; 4];

        for j in 0..4 {
            let t = top[base + j].to_array();
            let d = bottom[base + j].to_array();
            tr[j] = t[0] as u32;
            tg[j] = t[1] as u32;
            tb[j] = t[2] as u32;
            ta[j] = t[3] as u32;
            dr[j] = d[0] as u32;
            dg[j] = d[1] as u32;
            db[j] = d[2] as u32;
            da[j] = d[3] as u32;
        }

        let top_r = u32x4::new(tr);
        let top_g = u32x4::new(tg);
        let top_b = u32x4::new(tb);
        let top_a = u32x4::new(ta);
        let dst_r = u32x4::new(dr);
        let dst_g = u32x4::new(dg);
        let dst_b = u32x4::new(db);
        let dst_a = u32x4::new(da);

        let inv_a = u32x4::splat(255) - top_a;

        // src + ((dst * inv_src_a + 128) * 257) >> 16  (per channel, 4 pixels in parallel)

        let result_r = top_r + (((dst_r * inv_a + ONE28) * TWO57) >> SIXTEEN);
        let result_g = top_g + (((dst_g * inv_a + ONE28) * TWO57) >> SIXTEEN);
        let result_b = top_b + (((dst_b * inv_a + ONE28) * TWO57) >> SIXTEEN);
        let result_a = top_a + (((dst_a * inv_a + ONE28) * TWO57) >> SIXTEEN);

        let rr = result_r.to_array();
        let rg = result_g.to_array();
        let rb = result_b.to_array();
        let ra = result_a.to_array();

        for j in 0..4 {
            output[base + j] = Color32::from_rgba_premultiplied(
                rr[j] as u8,
                rg[j] as u8,
                rb[j] as u8,
                ra[j] as u8
            );
        }
    }

    // Scalar remainder for pixels not covered by full chunks
    for i in chunks * 4..len {
        output[i] = alpha_blend(bottom[i], top[i]);
    }
}
