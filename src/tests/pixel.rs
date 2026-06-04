//! Tests for pixel-blending primitives: `premultiply`, `unpremultiply`,
//! `alpha_blend`, `blend_layers`, and `blend_region`.
//!
//! Confirms correct premultiplied-alpha math, SIMD-accelerated layer
//! compositing, and partial-region blending.

use eframe::egui::Color32;

use crate::canvas::LayerMode;
use crate::pixel;
use crate::pixel::LayerBlendInfo;

/// Check that every channel of `a` and `b` differs by at most `max_diff`.

fn channels_close(left: Color32, right: Color32, max_diff: u8) -> bool {

    left.r().abs_diff(right.r()) <= max_diff
        && left.g().abs_diff(right.g()) <= max_diff
        && left.b().abs_diff(right.b()) <= max_diff
        && left.a().abs_diff(right.a()) <= max_diff
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

    assert_eq!(
        pixel::premultiply(Color32::TRANSPARENT),
        Color32::TRANSPARENT
    );
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
    assert!(
        channels_close(premul, back, 1),
        "premul={premul:?} back={back:?}"
    );
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

    let color = Color32::from_rgba_unmultiplied(200, 100, 50, 0);

    assert_eq!(pixel::premultiply(color), Color32::TRANSPARENT);
}

/// Unpremultiplying `TRANSPARENT` should return `TRANSPARENT` unchanged.
#[test]

fn unpremultiply_zero_alpha_stays_transparent() {

    assert_eq!(
        pixel::unpremultiply(Color32::TRANSPARENT),
        Color32::TRANSPARENT
    );
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
    assert!(
        channels_close(dest, result, 1),
        "dest={dest:?} result={result:?}"
    );
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

    pixel::blend_layers(
        &[LayerBlendInfo {
            pixels: &pixels[..],
            opacity: 255,
            mode: LayerMode::Normal,
        }],
        &mut output,
    );

    for (pixel_index, pixel) in pixels.iter().enumerate() {

        let arr = pixel.to_array();

        assert_eq!(output[pixel_index * 4], arr[0]);

        assert_eq!(output[pixel_index * 4 + 1], arr[1]);

        assert_eq!(output[pixel_index * 4 + 2], arr[2]);

        assert_eq!(output[pixel_index * 4 + 3], arr[3]);
    }
}

/// With two opaque layers, the top layer should fully occlude the bottom.
#[test]

fn blend_layers_two_layers_opaque() {

    let pixel_count = 12;

    let bottom = vec![Color32::from_rgba_premultiplied(255, 0, 0, 255); pixel_count];

    let top = vec![Color32::from_rgba_premultiplied(0, 255, 0, 255); pixel_count];

    let mut output = vec![0u8; pixel_count * 4];

    pixel::blend_layers(
        &[
            LayerBlendInfo {
                pixels: &bottom[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
            LayerBlendInfo {
                pixels: &top[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
        ],
        &mut output,
    );

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

    let width = 8u32;

    let height = 8u32;

    let pixel_count = (width * height) as usize;

    let pixels = vec![Color32::from_rgba_premultiplied(100, 150, 200, 255); pixel_count];

    let mut full_out = vec![0u8; pixel_count * 4];

    let mut region_out = vec![0u8; pixel_count * 4];

    pixel::blend_layers(
        &[LayerBlendInfo {
            pixels: &pixels[..],
            opacity: 255,
            mode: LayerMode::Normal,
        }],
        &mut full_out,
    );

    pixel::blend_region(
        &[LayerBlendInfo {
            pixels: &pixels[..],
            opacity: 255,
            mode: LayerMode::Normal,
        }],
        &mut region_out,
        width,
        2,
        2,
        5,
        5,
    );

    // Pixels outside region should be untouched (zero)
    assert_eq!(region_out[0..2 * 8 * 4], vec![0u8; 2 * 8 * 4][..]);

    // Pixels inside region should match full blend
    for y in 2..=5 {

        for x in 2..=5 {

            let pixel_index = (y * width + x) as usize * 4;

            assert_eq!(
                region_out[pixel_index..pixel_index + 4],
                full_out[pixel_index..pixel_index + 4],
                "mismatch at ({x},{y})"
            );
        }
    }
}

/// `blend_region` with two layers should match `blend_layers` in the specified rect.
#[test]

fn blend_region_two_layers() {

    let width = 6u32;

    let height = 6u32;

    let pixel_count = (width * height) as usize;

    let bottom = vec![Color32::from_rgba_premultiplied(255, 0, 0, 255); pixel_count];

    let top = vec![Color32::from_rgba_premultiplied(0, 255, 0, 128); pixel_count];

    let mut full_out = vec![0u8; pixel_count * 4];

    let mut region_out = vec![0u8; pixel_count * 4];

    pixel::blend_layers(
        &[
            LayerBlendInfo {
                pixels: &bottom[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
            LayerBlendInfo {
                pixels: &top[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
        ],
        &mut full_out,
    );

    pixel::blend_region(
        &[
            LayerBlendInfo {
                pixels: &bottom[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
            LayerBlendInfo {
                pixels: &top[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
        ],
        &mut region_out,
        width,
        1,
        1,
        4,
        4,
    );

    for y in 0..height {

        for x in 0..width {

            let pixel_index = (y * width + x) as usize * 4;

            if x >= 1 && x <= 4 && y >= 1 && y <= 4 {

                assert_eq!(
                    region_out[pixel_index..pixel_index + 4],
                    full_out[pixel_index..pixel_index + 4],
                    "mismatch at ({x},{y})"
                );
            } else {

                assert_eq!(
                    region_out[pixel_index..pixel_index + 4],
                    [0, 0, 0, 0],
                    "outside region should be zero at ({x},{y})"
                );
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

// --- Three layers ---

/// With three opaque layers, the topmost should fully occlude the rest.
#[test]

fn blend_layers_three_layers() {

    let pixel_count = 12;

    let bottom = vec![Color32::from_rgba_premultiplied(255, 0, 0, 255); pixel_count];

    let middle = vec![Color32::from_rgba_premultiplied(0, 255, 0, 255); pixel_count];

    let top = vec![Color32::from_rgba_premultiplied(0, 0, 255, 255); pixel_count];

    let mut output = vec![0u8; pixel_count * 4];

    pixel::blend_layers(
        &[
            LayerBlendInfo {
                pixels: &bottom[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
            LayerBlendInfo {
                pixels: &middle[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
            LayerBlendInfo {
                pixels: &top[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
        ],
        &mut output,
    );

    for i in 0..pixel_count {

        assert_eq!(output[i * 4], 0, "red channel at {i}");

        assert_eq!(output[i * 4 + 1], 0, "green channel at {i}");

        assert_eq!(output[i * 4 + 2], 255, "blue channel at {i}");

        assert_eq!(output[i * 4 + 3], 255, "alpha channel at {i}");
    }
}

/// Semi-transparent top layer should blend through to the opaque bottom layer.
#[test]

fn blend_layers_semi_transparent_top() {

    let pixel_count = 12;

    let bottom = vec![Color32::from_rgba_premultiplied(0, 255, 0, 255); pixel_count];

    let top = vec![Color32::from_rgba_premultiplied(255, 0, 0, 128); pixel_count];

    let mut output = vec![0u8; pixel_count * 4];

    pixel::blend_layers(
        &[
            LayerBlendInfo {
                pixels: &bottom[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
            LayerBlendInfo {
                pixels: &top[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
        ],
        &mut output,
    );

    for i in 0..pixel_count {

        // Opaque result, red from top blended over green bottom
        assert_eq!(output[i * 4 + 3], 255, "opaque at {i}");

        // Red channel should be > 0 (source contributes)
        assert!(output[i * 4] > 0, "red at {i}");

        // Green channel should be > 0 (dest still visible through src)
        assert!(output[i * 4 + 1] > 0, "green at {i}");

        // Result should be visually redder than green
        assert!(
            output[i * 4] > output[i * 4 + 1],
            "more red than green at {i}"
        );
    }
}

// --------------------------------------------------
//  Regression: double-premultiply guard
// --------------------------------------------------

/// Calling `premultiply` on an already-premultiplied color darkens it further.
///
/// This documents the semantics: `premultiply` assumes straight-alpha input.
/// Passing a premultiplied `Color32` (as returned by egui's color picker) into
/// `premultiply` is a bug — the color becomes ~50% darker at 50% alpha.
#[test]

fn premultiply_of_premultiplied_darkens_again() {

    // Simulate a 50% transparent red from the color picker (already premultiplied).
    let already_premul = Color32::from_rgba_premultiplied(128, 0, 0, 128);

    // Buggy double-premultiply.
    let double_premul = pixel::premultiply(already_premul);

    assert_eq!(already_premul.r(), 128, "correct premul: r=128");

    assert_eq!(double_premul.r(), 64, "double-premul darkens to r=64");

    assert_eq!(double_premul.g(), 0);

    assert_eq!(double_premul.b(), 0);

    assert_eq!(double_premul.a(), 128);
}

/// `premultiply` on an already-premultiplied `Color32` darkens it further.
///
/// `Color32` always stores premultiplied bytes internally (per egui's docs),
/// so `Color32::from_rgba_unmultiplied` converts straight→premultiplied at
/// construction. Calling `premultiply` on the result is a double application
/// of alpha scaling — the same bug pattern as the original brush code.
#[test]

fn premultiply_on_premul_storage_darkens() {

    // From straight alpha: egui stores as premultiplied internally.
    let color = Color32::from_rgba_unmultiplied(255, 0, 0, 128);

    // Color32 stores the premultiplied result, so .r() returns 128*128/255 ≈ 64.
    let stored_r = color.r();

    // Calling premultiply on the already-premultiplied storage darkens it.
    let doubled = pixel::premultiply(color);

    assert!(
        doubled.r() < stored_r,
        "premultiply on premultiplied Color32 darkens: {} -> {}",
        stored_r,
        doubled.r()
    );
}

// --- blend_layers edge & panic cases ---

/// `blend_layers` with an empty layer vec should panic.
#[test]
#[should_panic(expected = "at least one layer")]

fn blend_layers_empty_layers_panics() {

    let mut output = vec![0u8; 16];

    pixel::blend_layers(&[], &mut output);
}

/// `blend_layers` with mismatched layer lengths should panic.
#[test]
#[should_panic(expected = "length mismatch")]

fn blend_layers_mismatched_lengths_panics() {

    let pixel_count = 4;

    let bottom = vec![Color32::RED; pixel_count];

    let top = vec![Color32::BLUE; pixel_count + 2]; // longer layer
    let mut output = vec![0u8; pixel_count * 4];

    pixel::blend_layers(
        &[
            LayerBlendInfo {
                pixels: &bottom[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
            LayerBlendInfo {
                pixels: &top[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
        ],
        &mut output,
    );
}

/// `blend_layers` with output buffer too small should panic.
#[test]
#[should_panic(expected = "output length mismatch")]

fn blend_layers_output_too_small_panics() {

    let pixels = vec![Color32::RED; 4];

    let mut output = vec![0u8; 4]; // should be 16 bytes for 4 pixels
    pixel::blend_layers(
        &[LayerBlendInfo {
            pixels: &pixels[..],
            opacity: 255,
            mode: LayerMode::Normal,
        }],
        &mut output,
    );
}

// --- blend_region edge cases ---

/// `blend_region` with zero-width rect (min > max) is a no-op.
#[test]

fn blend_region_zero_width_noop() {

    let mut output = vec![0u8; 64];

    let pixels = vec![Color32::RED; 16];

    pixel::blend_region(
        &[LayerBlendInfo {
            pixels: &pixels[..],
            opacity: 255,
            mode: LayerMode::Normal,
        }],
        &mut output,
        4,
        5,
        0,
        4,
        3,
    );
}

/// `blend_region` with zero-height rect should be a no-op.
#[test]

fn blend_region_zero_height_noop() {

    let mut output = vec![0u8; 64];

    let pixels = vec![Color32::RED; 16];

    // min_y=5, max_y=3 -> zero height, loop doesn't execute
    pixel::blend_region(
        &[LayerBlendInfo {
            pixels: &pixels[..],
            opacity: 255,
            mode: LayerMode::Normal,
        }],
        &mut output,
        4,
        0,
        5,
        3,
        3,
    );

    assert_eq!(output, vec![0u8; 64]);
}

/// `blend_region` with rect extending beyond canvas bounds should panic.
#[test]
#[should_panic]

fn blend_region_out_of_bounds_panics() {

    let mut output = vec![0u8; 64];

    let pixels = vec![Color32::RED; 16];

    // Canvas is 4x4 (16 pixels), rect (0, 0, 10, 3) extends beyond width
    pixel::blend_region(
        &[LayerBlendInfo {
            pixels: &pixels[..],
            opacity: 255,
            mode: LayerMode::Normal,
        }],
        &mut output,
        4,
        0,
        0,
        10,
        3,
    );
}

// --- Additional edge cases for primitives ---

/// `premultiply` on a color with alpha=1 should zero out all RGB channels
/// (effectively invisible, but alpha=1 remains).
#[test]

fn premultiply_alpha_one_zeros_rgb() {

    let color = Color32::from_rgba_unmultiplied(100, 200, 50, 1);

    let premul = pixel::premultiply(color);

    assert_eq!(premul.r(), 0, "red channel zeroed");

    assert_eq!(premul.g(), 0, "green channel zeroed");

    assert_eq!(premul.b(), 0, "blue channel zeroed");

    assert_eq!(premul.a(), 1, "alpha preserved");
}

/// `unpremultiply` on fully opaque with max channels should be identity.
#[test]

fn unpremultiply_opaque_max_values() {

    let color = Color32::from_rgba_premultiplied(255, 255, 255, 255);

    assert_eq!(pixel::unpremultiply(color), color);
}

/// `alpha_blend` with fully transparent source and opaque destination
/// should leave destination within ±1 (alpha may be 254 due to fixed-point).
#[test]

fn alpha_blend_transparent_over_opaque_no_change() {

    let dest = Color32::from_rgba_premultiplied(128, 64, 32, 255);

    let result = pixel::alpha_blend(dest, Color32::TRANSPARENT);

    assert!(result.r().abs_diff(128) <= 1, "red: {}", result.r());

    assert!(result.g().abs_diff(64) <= 1, "green: {}", result.g());

    assert!(result.b().abs_diff(32) <= 1, "blue: {}", result.b());

    // Fixed-point: (255*255 + 128) >> 8 = 254, so alpha may be 254 not 255
    assert!(result.a() >= 254, "alpha: {}", result.a());
}

/// `blend_layers` with exactly 4 pixels (one SIMD chunk) should work.
#[test]

fn blend_layers_exactly_one_simd_chunk() {

    let pixel_count = 4;

    let bottom = vec![Color32::from_rgba_premultiplied(255, 0, 0, 255); pixel_count];

    let top = vec![Color32::from_rgba_premultiplied(0, 255, 0, 128); pixel_count];

    let mut output = vec![0u8; pixel_count * 4];

    pixel::blend_layers(
        &[
            LayerBlendInfo {
                pixels: &bottom[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
            LayerBlendInfo {
                pixels: &top[..],
                opacity: 255,
                mode: LayerMode::Normal,
            },
        ],
        &mut output,
    );

    // Result should be alpha-blended green over red
    for i in 0..pixel_count {

        assert_eq!(output[i * 4 + 3], 255, "alpha at {i}");

        assert!(output[i * 4] > 0, "red at {i}"); // red still visible
        assert!(output[i * 4 + 1] > 0, "green at {i}"); // green blended
    }
}

/// `blend_layers` with 5 pixels (one SIMD chunk + scalar tail).
#[test]

fn blend_layers_simd_plus_tail() {

    let pixel_count = 5;

    let pixels = vec![Color32::from_rgba_premultiplied(100, 150, 200, 255); pixel_count];

    let mut output = vec![0u8; pixel_count * 4];

    pixel::blend_layers(
        &[LayerBlendInfo {
            pixels: &pixels[..],
            opacity: 255,
            mode: LayerMode::Normal,
        }],
        &mut output,
    );

    for i in 0..pixel_count {

        assert_eq!(output[i * 4], 100, "pixel {i} red");

        assert_eq!(output[i * 4 + 1], 150, "pixel {i} green");

        assert_eq!(output[i * 4 + 2], 200, "pixel {i} blue");

        assert_eq!(output[i * 4 + 3], 255, "pixel {i} alpha");
    }
}
