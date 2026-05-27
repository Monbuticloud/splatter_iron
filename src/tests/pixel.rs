use eframe::egui::Color32;

use crate::pixel;

/// Check that every channel of `a` and `b` differs by at most `max_diff`.
fn channels_close(a: Color32, b: Color32, max_diff: u8) -> bool {
    a.r().abs_diff(b.r()) <= max_diff &&
        a.g().abs_diff(b.g()) <= max_diff &&
        a.b().abs_diff(b.b()) <= max_diff &&
        a.a().abs_diff(b.a()) <= max_diff
}

/// Premultiplying an opaque color should return it unchanged.
#[test]
fn premultiply_opaque_is_identity() {
    let opaque = Color32::from_rgba_premultiplied(200, 100, 50, 255);
    assert_eq!(pixel::premultiply(opaque), opaque);
}

/// Premultiplying a fully transparent color should return `TRANSPARENT`.
#[test]
fn premultiply_transparent_is_unchanged() {
    assert_eq!(pixel::premultiply(Color32::TRANSPARENT), Color32::TRANSPARENT);
}

/// After premultiply, each RGB channel should be ≤ the original straight value.
#[test]
fn premultiply_non_opaque_approximation() {
    let straight = Color32::from_rgba_unmultiplied(128, 64, 32, 128);
    let premul = pixel::premultiply(straight);
    // Each premultiplied channel must be <= original (since it's multiplied by alpha/255)
    assert!(premul.r() <= straight.r());
    assert!(premul.g() <= straight.g());
    assert!(premul.b() <= straight.b());
    assert_eq!(premul.a(), 128);
}

/// Unpremultiply should recover the straight alpha values.
#[test]
fn unpremultiply_produces_straight_alpha() {
    let premul = Color32::from_rgba_premultiplied(64, 32, 16, 128);
    let straight = pixel::unpremultiply(premul);
    // straight.r() = 64 * 255 / 128 = 127
    assert_eq!(straight.r(), 127);
    assert_eq!(straight.g(), 63);
    assert_eq!(straight.b(), 31);
    assert_eq!(straight.a(), 128);
}

/// Round-tripping `premultiply(unpremultiply(c))` should stay within ±1 per channel.
#[test]
fn premultiply_unpremultiply_roundtrip_close() {
    let premul = Color32::from_rgba_premultiplied(64, 32, 16, 128);
    let back = pixel::premultiply(pixel::unpremultiply(premul));
    // Fixed-point (±1 per channel)
    assert!(channels_close(premul, back, 1), "premul={premul:?} back={back:?}");
}

/// Unpremultiplying an opaque color should return it unchanged.
#[test]
fn unpremultiply_opaque_is_identity() {
    let opaque = Color32::from_rgba_unmultiplied(200, 100, 50, 255);
    assert_eq!(pixel::unpremultiply(opaque), opaque);
}

/// Premultiplying a zero-alpha color should produce `TRANSPARENT`.
#[test]
fn premultiply_zero_alpha() {
    let c = Color32::from_rgba_unmultiplied(200, 100, 50, 0);
    assert_eq!(pixel::premultiply(c), Color32::TRANSPARENT);
}

/// Unpremultiplying `TRANSPARENT` should return `TRANSPARENT` unchanged.
#[test]
fn unpremultiply_zero_alpha_stays_transparent() {
    assert_eq!(pixel::unpremultiply(Color32::TRANSPARENT), Color32::TRANSPARENT);
}

/// Blending an opaque source over transparent should yield the source.
#[test]
fn alpha_blend_opaque_over_transparent() {
    let src = Color32::from_rgba_premultiplied(255, 0, 0, 255);
    assert_eq!(pixel::alpha_blend(Color32::TRANSPARENT, src), src);
}

/// Blending transparent source over destination should leave dest within ±1.
#[test]
fn alpha_blend_transparent_over_dest_is_close() {
    let dest = Color32::from_rgba_premultiplied(100, 150, 200, 255);
    let result = pixel::alpha_blend(dest, Color32::TRANSPARENT);
    // Fixed-point `(val * 255 + 128) >> 8` can be off by 1
    assert!(channels_close(dest, result, 1), "dest={dest:?} result={result:?}");
}

/// Blending a semi-transparent source over an opaque dest should produce an opaque result.
#[test]
fn alpha_blend_semi_transparent_over_opaque() {
    let dest = Color32::from_rgba_premultiplied(0, 255, 0, 255);
    let src = Color32::from_rgba_premultiplied(255, 0, 0, 128);
    let result = pixel::alpha_blend(dest, src);
    // Opaque result, red from src blended over green dest
    assert_eq!(result.a(), 255);
    assert!(result.r() > 0);
    assert!(result.g() > 0);
}

/// Blending a single layer should copy its premultiplied RGBA bytes directly.
#[test]
fn blend_layers_single_layer_copy() {
    let pixel_count = 12;
    let pixels = vec![Color32::from_rgba_premultiplied(50, 100, 150, 200); pixel_count];
    let mut output = vec![0u8; pixel_count * 4];
    pixel::blend_layers(&[&pixels], &mut output);
    for (i, p) in pixels.iter().enumerate() {
        let arr = p.to_array();
        assert_eq!(output[i * 4], arr[0]);
        assert_eq!(output[i * 4 + 1], arr[1]);
        assert_eq!(output[i * 4 + 2], arr[2]);
        assert_eq!(output[i * 4 + 3], arr[3]);
    }
}

/// With two opaque layers, the top layer should fully occlude the bottom.
#[test]
fn blend_layers_two_layers_opaque() {
    let pixel_count = 12;
    let bottom = vec![Color32::from_rgba_premultiplied(255, 0, 0, 255); pixel_count];
    let top = vec![Color32::from_rgba_premultiplied(0, 255, 0, 255); pixel_count];
    let mut output = vec![0u8; pixel_count * 4];
    pixel::blend_layers(&[&bottom, &top], &mut output);
    for i in 0..pixel_count {
        assert_eq!(output[i * 4], 0, "red channel at {i}");
        assert_eq!(output[i * 4 + 1], 255, "green channel at {i}");
        assert_eq!(output[i * 4 + 2], 0, "blue channel at {i}");
        assert_eq!(output[i * 4 + 3], 255, "alpha channel at {i}");
    }
}

// --- blend_region ---

/// `blend_region` on a single layer should match `blend_layers` for that region.
#[test]
fn blend_region_single_layer_matches_full() {
    let w = 8u32;
    let h = 8u32;
    let pixel_count = (w * h) as usize;
    let pixels = vec![Color32::from_rgba_premultiplied(100, 150, 200, 255); pixel_count];
    let mut full_out = vec![0u8; pixel_count * 4];
    let mut region_out = vec![0u8; pixel_count * 4];

    pixel::blend_layers(&[&pixels], &mut full_out);
    pixel::blend_region(&[&pixels], &mut region_out, w, 2, 2, 5, 5);

    // Pixels outside region should be untouched (zero)
    assert_eq!(region_out[0..2 * 8 * 4], vec![0u8; 2 * 8 * 4][..]);
    // Pixels inside region should match full blend
    for y in 2..=5 {
        for x in 2..=5 {
            let idx = (y * w + x) as usize * 4;
            assert_eq!(region_out[idx..idx + 4], full_out[idx..idx + 4],
                "mismatch at ({x},{y})");
        }
    }
}

/// `blend_region` with two layers should match `blend_layers` in the specified rect.
#[test]
fn blend_region_two_layers() {
    let w = 6u32;
    let h = 6u32;
    let pixel_count = (w * h) as usize;
    let bottom = vec![Color32::from_rgba_premultiplied(255, 0, 0, 255); pixel_count];
    let top = vec![Color32::from_rgba_premultiplied(0, 255, 0, 128); pixel_count];
    let mut full_out = vec![0u8; pixel_count * 4];
    let mut region_out = vec![0u8; pixel_count * 4];

    pixel::blend_layers(&[&bottom, &top], &mut full_out);
    pixel::blend_region(&[&bottom, &top], &mut region_out, w, 1, 1, 4, 4);

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize * 4;
            if x >= 1 && x <= 4 && y >= 1 && y <= 4 {
                assert_eq!(region_out[idx..idx + 4], full_out[idx..idx + 4],
                    "mismatch at ({x},{y})");
            } else {
                assert_eq!(region_out[idx..idx + 4], [0, 0, 0, 0],
                    "outside region should be zero at ({x},{y})");
            }
        }
    }
}

/// `blend_region` should be a no-op for an empty layer list.
#[test]
fn blend_region_empty_layers_no_panic() {
    let mut output = vec![0u8; 16];
    pixel::blend_region(&[], &mut output, 4, 0, 0, 3, 3);
    // Should not panic, output unchanged
}
