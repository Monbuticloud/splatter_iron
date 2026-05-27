use eframe::egui::Color32;

use crate::pixel;

/// Check that every channel of `a` and `b` differs by at most `max_diff`.
fn channels_close(a: Color32, b: Color32, max_diff: u8) -> bool {
    a.r().abs_diff(b.r()) <= max_diff &&
        a.g().abs_diff(b.g()) <= max_diff &&
        a.b().abs_diff(b.b()) <= max_diff &&
        a.a().abs_diff(b.a()) <= max_diff
}

/// Premultiplying an opaque colour should return it unchanged.
#[test]
fn premultiply_opaque_is_identity() {
    let opaque = Color32::from_rgba_premultiplied(200, 100, 50, 255);
    assert_eq!(pixel::premultiply(opaque), opaque);
}

/// Premultiplying a fully transparent colour should return `TRANSPARENT`.
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

/// Unpremultiplying an opaque colour should return it unchanged.
#[test]
fn unpremultiply_opaque_is_identity() {
    let opaque = Color32::from_rgba_unmultiplied(200, 100, 50, 255);
    assert_eq!(pixel::unpremultiply(opaque), opaque);
}

/// Premultiplying a zero-alpha colour should produce `TRANSPARENT`.
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
