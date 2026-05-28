//! Premultiplied-alpha pixel blending with SIMD (`wide::u32x4`) and
//! rayon parallelism.  Provides `blend_layers` (full canvas) and
//! `blend_region` (dirty-rect) compositing, plus `premultiply`,
//! `unpremultiply`, and `alpha_blend` primitives.

use bytemuck::cast_slice;
use eframe::egui::Color32;
use rayon::prelude::*;
use wide::u32x4;

pub const BYTES_PER_PIXEL: usize = 4;
pub const F32_COLOR_MAX: f32 = 255.0;

// SIMD constant for the (value * alpha + 128) >> 8 fixed-point blend
const ROUNDING_BIAS_128: u32x4 = u32x4::splat(128);

/// Convert a premultiplied-alpha color to straight alpha.
///
/// This is the inverse of [`premultiply`]. Fully opaque or fully transparent
/// pixels are returned unchanged (alpha == 0 never causes division by zero).
#[inline]
pub fn unpremultiply(color: Color32) -> Color32 {
    let alpha = color.a();
    if alpha == 0 || alpha == 255 {
        return color;
    }
    let alpha_u32 = alpha as u32;
    let red = (((color.r() as u32) * 255) / alpha_u32).min(255) as u8;
    let green = (((color.g() as u32) * 255) / alpha_u32).min(255) as u8;
    let blue = (((color.b() as u32) * 255) / alpha_u32).min(255) as u8;
    Color32::from_rgba_premultiplied(red, green, blue, alpha)
}

/// Convert a straight-alpha color to premultiplied alpha.
///
/// Uses fixed-point arithmetic `(value * alpha + 128) * 257 >> 16` for correct
/// rounding. Fully opaque colors pass through unchanged; fully transparent
/// colors become `Color32::TRANSPARENT`.
///
/// **Caller must supply straight (non-premultiplied) RGB.**
/// Calling this on an already-premultiplied pixel will darken colors again.
#[inline]
pub const fn premultiply(color: Color32) -> Color32 {
    let alpha = color.a();

    // Fully opaque — no change needed
    if alpha == 255 {
        return color;
    }

    // Fully transparent — return transparent
    if alpha == 0 {
        return Color32::TRANSPARENT;
    }

    // Multiply each channel by alpha with correct rounding
    // Uses (value * alpha + 128) * 257 >> 16  (fixed-point division by 255)
    let red = ((((color.r() as u32) * (alpha as u32) + 128) * 257) >> 16) as u8;
    let green = ((((color.g() as u32) * (alpha as u32) + 128) * 257) >> 16) as u8;
    let blue = ((((color.b() as u32) * (alpha as u32) + 128) * 257) >> 16) as u8;

    Color32::from_rgba_premultiplied(red, green, blue, alpha)
}

/// Alpha-blend a premultiplied source pixel over a premultiplied destination.
///
/// Uses fixed-point arithmetic `(dest_channel * inverse_alpha + 128) >> 8`
/// for each color channel. The result is in premultiplied-alpha format.
#[inline]
pub const fn alpha_blend(destination: Color32, source: Color32) -> Color32 {
    /// Blend a single color channel: `dest * inverse_alpha` with rounding.
    #[inline]
    const fn blend_channel(destination_channel: u32, inverse_alpha: u32) -> u32 {
        (destination_channel * inverse_alpha + 128) >> 8
    }

    let source_red = source.r() as u32;
    let source_green = source.g() as u32;
    let source_blue = source.b() as u32;
    let source_alpha = source.a() as u32;

    let dest_red = destination.r() as u32;
    let dest_green = destination.g() as u32;
    let dest_blue = destination.b() as u32;
    let dest_alpha = destination.a() as u32;

    let inverse_alpha = 255 - source_alpha;

    Color32::from_rgba_premultiplied(
        (source_red + blend_channel(dest_red, inverse_alpha)) as u8,
        (source_green + blend_channel(dest_green, inverse_alpha)) as u8,
        (source_blue + blend_channel(dest_blue, inverse_alpha)) as u8,
        (source_alpha + blend_channel(dest_alpha, inverse_alpha)) as u8
    )
}

/// Minimum SIMD chunk count before rayon parallelism kicks in.
const PARALLEL_BLEND_THRESHOLD: usize = 64;

/// SIMD blend of 4 pixels (one 16-byte chunk).
#[inline]
fn blend_simd_chunk(
    output_chunk: &mut [u8],
    layers: &[&[Color32]],
    pixel_base: usize,
) {
    let bottom_layer = layers[0];
    let bottom_pixel_0 = bottom_layer[pixel_base].to_array();
    let bottom_pixel_1 = bottom_layer[pixel_base + 1].to_array();
    let bottom_pixel_2 = bottom_layer[pixel_base + 2].to_array();
    let bottom_pixel_3 = bottom_layer[pixel_base + 3].to_array();

    let mut accumulator_r = u32x4::new([
        bottom_pixel_0[0] as u32,
        bottom_pixel_1[0] as u32,
        bottom_pixel_2[0] as u32,
        bottom_pixel_3[0] as u32,
    ]);
    let mut accumulator_g = u32x4::new([
        bottom_pixel_0[1] as u32,
        bottom_pixel_1[1] as u32,
        bottom_pixel_2[1] as u32,
        bottom_pixel_3[1] as u32,
    ]);
    let mut accumulator_b = u32x4::new([
        bottom_pixel_0[2] as u32,
        bottom_pixel_1[2] as u32,
        bottom_pixel_2[2] as u32,
        bottom_pixel_3[2] as u32,
    ]);
    let mut accumulator_a = u32x4::new([
        bottom_pixel_0[3] as u32,
        bottom_pixel_1[3] as u32,
        bottom_pixel_2[3] as u32,
        bottom_pixel_3[3] as u32,
    ]);

    for &layer_slice in &layers[1..] {
        let top_pixel_0 = layer_slice[pixel_base].to_array();
        let top_pixel_1 = layer_slice[pixel_base + 1].to_array();
        let top_pixel_2 = layer_slice[pixel_base + 2].to_array();
        let top_pixel_3 = layer_slice[pixel_base + 3].to_array();

        let top_r = u32x4::new([
            top_pixel_0[0] as u32,
            top_pixel_1[0] as u32,
            top_pixel_2[0] as u32,
            top_pixel_3[0] as u32,
        ]);
        let top_g = u32x4::new([
            top_pixel_0[1] as u32,
            top_pixel_1[1] as u32,
            top_pixel_2[1] as u32,
            top_pixel_3[1] as u32,
        ]);
        let top_b = u32x4::new([
            top_pixel_0[2] as u32,
            top_pixel_1[2] as u32,
            top_pixel_2[2] as u32,
            top_pixel_3[2] as u32,
        ]);
        let top_a = u32x4::new([
            top_pixel_0[3] as u32,
            top_pixel_1[3] as u32,
            top_pixel_2[3] as u32,
            top_pixel_3[3] as u32,
        ]);

        let inverse_alpha = u32x4::splat(255) - top_a;

        accumulator_r = top_r + ((accumulator_r * inverse_alpha + ROUNDING_BIAS_128) >> 8);
        accumulator_g = top_g + ((accumulator_g * inverse_alpha + ROUNDING_BIAS_128) >> 8);
        accumulator_b = top_b + ((accumulator_b * inverse_alpha + ROUNDING_BIAS_128) >> 8);
        accumulator_a = top_a + ((accumulator_a * inverse_alpha + ROUNDING_BIAS_128) >> 8);
    }

    let red_array = accumulator_r.to_array();
    let green_array = accumulator_g.to_array();
    let blue_array = accumulator_b.to_array();
    let alpha_array = accumulator_a.to_array();

    for pixel_offset in 0..4 {
        let output_index = pixel_offset * BYTES_PER_PIXEL;
        output_chunk[output_index] = red_array[pixel_offset] as u8;
        output_chunk[output_index + 1] = green_array[pixel_offset] as u8;
        output_chunk[output_index + 2] = blue_array[pixel_offset] as u8;
        output_chunk[output_index + 3] = alpha_array[pixel_offset] as u8;
    }
}

/// Blend a contiguous range of pixels across multiple layers.
///
/// Handles 4-pixel SIMD alignment, with optional rayon parallelism
/// when `parallel` is true and there are enough chunks.
///
/// # Panics
///
/// Panics if `layers` is empty, if any layer is shorter than
/// `pixel_start + pixel_count`, or if `output` is shorter than
/// `(pixel_start + pixel_count) * BYTES_PER_PIXEL`.
#[inline]
fn blend_pixel_range(
    layers: &[&[Color32]],
    output: &mut [u8],
    pixel_start: usize,
    pixel_count: usize,
    parallel: bool,
) {
    let pixel_end = pixel_start + pixel_count;

    // Fast path: single layer — memcpy RGBA bytes
    if layers.len() == 1 {
        let source: &[u8] = cast_slice(&layers[0][pixel_start..pixel_end]);
        let destination = &mut output[pixel_start * BYTES_PER_PIXEL..pixel_end * BYTES_PER_PIXEL];
        destination.copy_from_slice(source);
        return;
    }

    // Scalar head: pixels before the first 4-aligned boundary
    let aligned_start = (pixel_start + 3) & !3;
    let head_end = aligned_start.min(pixel_end);
    for pixel_index in pixel_start..head_end {
        let mut pixel = layers[0][pixel_index];
        for &layer_slice in &layers[1..] {
            pixel = alpha_blend(pixel, layer_slice[pixel_index]);
        }
        let rgba = pixel.to_array();
        let byte_index = pixel_index * BYTES_PER_PIXEL;
        output[byte_index..byte_index + BYTES_PER_PIXEL].copy_from_slice(&rgba);
    }

    // SIMD-aligned body
    let aligned_end = pixel_end & !3;
    if aligned_start < aligned_end {
        let simd_pixel_count = aligned_end - aligned_start;
        let simd_chunks = simd_pixel_count / 4;
        let byte_start = aligned_start * BYTES_PER_PIXEL;
        let aligned_output = &mut output[byte_start..byte_start + simd_pixel_count * BYTES_PER_PIXEL];

        if parallel && simd_chunks > PARALLEL_BLEND_THRESHOLD {
            aligned_output
                .par_chunks_mut(16)
                .enumerate()
                .for_each(|(chunk_index, chunk)| {
                    blend_simd_chunk(chunk, layers, aligned_start + chunk_index * 4);
                });
        } else {
            for (chunk_index, chunk) in aligned_output.chunks_mut(16).enumerate() {
                blend_simd_chunk(chunk, layers, aligned_start + chunk_index * 4);
            }
        }
    }

    // Scalar tail
    let tail_start = aligned_end.max(pixel_start);
    for pixel_index in tail_start..pixel_end {
        let mut pixel = layers[0][pixel_index];
        for &layer_slice in &layers[1..] {
            pixel = alpha_blend(pixel, layer_slice[pixel_index]);
        }
        let rgba = pixel.to_array();
        let byte_index = pixel_index * BYTES_PER_PIXEL;
        output[byte_index..byte_index + BYTES_PER_PIXEL].copy_from_slice(&rgba);
    }
}

/// Composite multiple premultiplied layers into an RGBA byte buffer.
///
/// Layers are blended bottom-to-top (index 0 = bottommost).
/// Uses SIMD (`wide::u32x4`) via rayon for parallel processing of 4-pixel chunks;
/// remaining pixels are handled with scalar `alpha_blend`.
///
/// # Panics
///
/// - If `layers` is empty.
/// - If any layer has a different length from `layers[0]`.
/// - If `output.len() != layers[0].len() * 4`.
#[inline]
pub fn blend_layers(layers: &[&[Color32]], output: &mut [u8]) {
    assert!(!layers.is_empty(), "blend_layers: at least one layer required");

    let pixel_count = layers[0].len();
    #[cfg(debug_assertions)]
    for (layer_index, layer) in layers.iter().enumerate() {
        assert_eq!(layer.len(), pixel_count, "blend_layers: layer {layer_index} length mismatch");
    }
    assert_eq!(output.len(), pixel_count * BYTES_PER_PIXEL, "blend_layers: output length mismatch");

    blend_pixel_range(layers, output, 0, pixel_count, true);
}

/// Blend only the pixels within a dirty rectangle.
///
/// Processes the region row-by-row, calling `blend_pixel_range` for each row
/// segment. Sequential iteration is used since dirty rects from brush strokes
/// are typically small enough that parallel overhead would dominate.
///
/// # Panics
///
/// Panics if any layer has fewer pixels than required by the region bounds,
/// or if `output` is too small for the required byte range.
#[inline]
pub fn blend_region(
    layers: &[&[Color32]],
    output: &mut [u8],
    canvas_width: u32,
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
) {
    if layers.is_empty() {
        return;
    }
    let width = canvas_width as usize;
    for y in min_y..=max_y {
        let pixel_start = (y as usize) * width + min_x as usize;
        let pixel_count = (max_x - min_x + 1) as usize;
        blend_pixel_range(layers, output, pixel_start, pixel_count, false);
    }
}
