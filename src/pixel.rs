//! Premultiplied-alpha pixel blending with SIMD (`wide::u32x4`) and
//! rayon parallelism.  Provides `blend_layers` (full canvas) and
//! `blend_region` (dirty-rect) compositing, plus `premultiply`,
//! `unpremultiply`, and `alpha_blend` primitives.
//!
//! Supports three layer compositing modes:
//! - [`LayerMode::Normal`]: standard alpha-over blend.
//! - [`LayerMode::ClippedDown`]: layer alpha is clipped by the base layer's alpha.
//! - [`LayerMode::MaskedDown`]: layer alpha modulates the accumulator alpha
//!   (RGB content is not rendered).

use eframe::egui::Color32;
use rayon::prelude::*;
use wide::u32x4;

use crate::canvas::LayerMode;

/// Number of bytes per pixel in RGBA output buffers.
pub const BYTES_PER_PIXEL: usize = 4;
/// Maximum `f32` value used for normalising `u8` color channels.
pub const F32_COLOR_MAX: f32 = 255.0;

// SIMD constant for the (value * alpha + 128) >> 8 fixed-point blend
const ROUNDING_BIAS_128: u32x4 = u32x4::splat(128);

/// Per-layer information needed for compositing.
#[derive(Clone, Copy, Debug)]
pub struct LayerBlendInfo<'a> {
    /// Premultiplied-alpha RGBA pixels for this layer.
    pub pixels: &'a [Color32],
    /// Per-layer opacity (0–255).
    pub opacity: u8,
    /// Compositing mode relative to the layer below.
    pub mode: LayerMode,
}

/// Convert a premultiplied-alpha color to straight alpha.
///
/// This is the inverse of [`premultiply`]. Fully opaque or fully transparent
/// pixels are returned unchanged (alpha == 0 never causes division by zero).
///
/// # Parameters
///
/// * `color` — Premultiplied-alpha color to convert.
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
///
/// # Parameters
///
/// * `color` — Straight-alpha color to convert to premultiplied format.
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
///
/// # Parameters
///
/// * `destination` — Background pixel (premultiplied).
/// * `source` — Foreground pixel (premultiplied), composited over destination.
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
        (source_alpha + blend_channel(dest_alpha, inverse_alpha)) as u8,
    )
}

/// Minimum SIMD chunk count before rayon parallelism kicks in.
const PARALLEL_BLEND_THRESHOLD: usize = 64;

/// Scale a single premultiplied pixel by opacity (0–255) using fixed-point.
#[inline]
const fn scale_pixel_opacity(pixel: Color32, opacity: u8) -> Color32 {
    if opacity == 255 {
        return pixel;
    }
    let f = opacity as u32;
    Color32::from_rgba_premultiplied(
        ((pixel.r() as u32 * f + 128) * 257 >> 16) as u8,
        ((pixel.g() as u32 * f + 128) * 257 >> 16) as u8,
        ((pixel.b() as u32 * f + 128) * 257 >> 16) as u8,
        ((pixel.a() as u32 * f + 128) * 257 >> 16) as u8,
    )
}

/// Scale a single alpha value (0–255) by opacity (0–255) using fixed-point.
#[inline]
const fn scale_alpha(alpha: u8, opacity: u8) -> u8 {
    if opacity == 255 {
        return alpha;
    }
    ((alpha as u32 * opacity as u32 + 128) * 257 >> 16) as u8
}

/// Apply a MaskedDown layer: multiply each pixel's accumulated alpha by the
/// mask layer's alpha (after opacity scaling).  The mask layer's RGB content
/// is ignored — it only contributes its alpha channel.
#[inline]
fn apply_mask_simd_chunk(
    output_chunk: &mut [u8],
    mask_pixels: &[Color32],
    mask_opacity: u8,
    pixel_base: usize,
) {
    // Read 4 accumulator pixels from output buffer as u32 channels.
    let (acc_ra, acc_ga, acc_ba, mut acc_aa) = read_accumulator(output_chunk);

    // Read 4 mask alpha values.
    let m0 = scale_alpha(mask_pixels[pixel_base].a(), mask_opacity) as u32;
    let m1 = scale_alpha(mask_pixels[pixel_base + 1].a(), mask_opacity) as u32;
    let m2 = scale_alpha(mask_pixels[pixel_base + 2].a(), mask_opacity) as u32;
    let m3 = scale_alpha(mask_pixels[pixel_base + 3].a(), mask_opacity) as u32;
    let mask_a = u32x4::new([m0, m1, m2, m3]);

    // Accumulator alpha *= mask alpha  (RGB unchanged).
    acc_aa = (acc_aa * mask_a + ROUNDING_BIAS_128) >> u32x4::splat(8);

    write_accumulator(output_chunk, acc_ra, acc_ga, acc_ba, acc_aa);
}

/// Blend one Normal or ClippedDown layer over the existing output accumulator
/// with SIMD for 4 pixels.
#[inline]
fn blend_simd_chunk_over(
    output_chunk: &mut [u8],
    layer_pixels: &[Color32],
    layer_opacity: u8,
    pixel_base: usize,
    clip_base: Option<(&[Color32], u8)>,
) {
    // Read 4 accumulator pixels from output buffer.
    let (mut acc_ra, mut acc_ga, mut acc_ba, mut acc_aa) = read_accumulator(output_chunk);

    // Load 4 layer pixels.
    let top_pixel_0 = layer_pixels[pixel_base].to_array();
    let top_pixel_1 = layer_pixels[pixel_base + 1].to_array();
    let top_pixel_2 = layer_pixels[pixel_base + 2].to_array();
    let top_pixel_3 = layer_pixels[pixel_base + 3].to_array();

    let mut top_r = u32x4::new([
        top_pixel_0[0] as u32,
        top_pixel_1[0] as u32,
        top_pixel_2[0] as u32,
        top_pixel_3[0] as u32,
    ]);
    let mut top_g = u32x4::new([
        top_pixel_0[1] as u32,
        top_pixel_1[1] as u32,
        top_pixel_2[1] as u32,
        top_pixel_3[1] as u32,
    ]);
    let mut top_b = u32x4::new([
        top_pixel_0[2] as u32,
        top_pixel_1[2] as u32,
        top_pixel_2[2] as u32,
        top_pixel_3[2] as u32,
    ]);
    let mut top_a = u32x4::new([
        top_pixel_0[3] as u32,
        top_pixel_1[3] as u32,
        top_pixel_2[3] as u32,
        top_pixel_3[3] as u32,
    ]);

    // Apply per-layer opacity.
    if layer_opacity != 255 {
        let factor = u32x4::splat(layer_opacity as u32);
        top_r = (top_r * factor + ROUNDING_BIAS_128) >> u32x4::splat(8);
        top_g = (top_g * factor + ROUNDING_BIAS_128) >> u32x4::splat(8);
        top_b = (top_b * factor + ROUNDING_BIAS_128) >> u32x4::splat(8);
        top_a = (top_a * factor + ROUNDING_BIAS_128) >> u32x4::splat(8);
    }

    // Apply ClippedDown: clip all channels by base layer's alpha.
    if let Some((base_pixels, base_opacity)) = clip_base {
        let b0 = scale_alpha(base_pixels[pixel_base].a(), base_opacity) as u32;
        let b1 = scale_alpha(base_pixels[pixel_base + 1].a(), base_opacity) as u32;
        let b2 = scale_alpha(base_pixels[pixel_base + 2].a(), base_opacity) as u32;
        let b3 = scale_alpha(base_pixels[pixel_base + 3].a(), base_opacity) as u32;
        let base_a = u32x4::new([b0, b1, b2, b3]);

        top_r = (top_r * base_a + ROUNDING_BIAS_128) >> u32x4::splat(8);
        top_g = (top_g * base_a + ROUNDING_BIAS_128) >> u32x4::splat(8);
        top_b = (top_b * base_a + ROUNDING_BIAS_128) >> u32x4::splat(8);
        top_a = (top_a * base_a + ROUNDING_BIAS_128) >> u32x4::splat(8);
    }

    // Standard alpha-over blend into accumulator.
    let inverse_alpha = u32x4::splat(255) - top_a;

    acc_ra = top_r + ((acc_ra * inverse_alpha + ROUNDING_BIAS_128) >> 8);
    acc_ga = top_g + ((acc_ga * inverse_alpha + ROUNDING_BIAS_128) >> 8);
    acc_ba = top_b + ((acc_ba * inverse_alpha + ROUNDING_BIAS_128) >> 8);
    acc_aa = top_a + ((acc_aa * inverse_alpha + ROUNDING_BIAS_128) >> 8);

    write_accumulator(output_chunk, acc_ra, acc_ga, acc_ba, acc_aa);
}

/// Read 4 accumulator pixels from a 16-byte output chunk into u32x4 vectors.
#[inline]
fn read_accumulator(chunk: &[u8]) -> (u32x4, u32x4, u32x4, u32x4) {
    (
        u32x4::new([
            chunk[0] as u32,
            chunk[4] as u32,
            chunk[8] as u32,
            chunk[12] as u32,
        ]),
        u32x4::new([
            chunk[1] as u32,
            chunk[5] as u32,
            chunk[9] as u32,
            chunk[13] as u32,
        ]),
        u32x4::new([
            chunk[2] as u32,
            chunk[6] as u32,
            chunk[10] as u32,
            chunk[14] as u32,
        ]),
        u32x4::new([
            chunk[3] as u32,
            chunk[7] as u32,
            chunk[11] as u32,
            chunk[15] as u32,
        ]),
    )
}

/// Write 4 u32x4 vectors back into a 16-byte output chunk.
#[inline]
fn write_accumulator(chunk: &mut [u8], r: u32x4, g: u32x4, b: u32x4, a: u32x4) {
    let ra = r.to_array();
    let ga = g.to_array();
    let ba = b.to_array();
    let aa = a.to_array();
    for i in 0..4 {
        let o = i * 4;
        chunk[o] = ra[i] as u8;
        chunk[o + 1] = ga[i] as u8;
        chunk[o + 2] = ba[i] as u8;
        chunk[o + 3] = aa[i] as u8;
    }
}

/// Blend (or mask) a single layer over a pixel range of the output accumulator.
///
/// Handles [`LayerMode::Normal`] and [`LayerMode::ClippedDown`] via SIMD alpha-over
/// blend, and [`LayerMode::MaskedDown`] by modulating the accumulator alpha.
///
/// # Parameters
///
/// * `output` — RGBA accumulator buffer (read-write, full canvas size).
/// * `info` — The layer to apply (pixels, opacity, mode).
/// * `base` — For ClippedDown: the base layer whose alpha clips this layer.
/// * `pixel_start` — First pixel index in the range.
/// * `pixel_count` — Number of pixels to process.
/// * `parallel` — Whether to use rayon parallelism (full-canvas blends).
///
/// # Panics
///
/// Panics if `output` is too small for the requested pixel range.
#[inline]
fn apply_single_layer(
    output: &mut [u8],
    info: &LayerBlendInfo,
    base: Option<&LayerBlendInfo>,
    pixel_start: usize,
    pixel_count: usize,
    parallel: bool,
) {
    let pixel_end = pixel_start + pixel_count;

    let clip_base = match (info.mode, base) {
        (LayerMode::ClippedDown, Some(base_info)) => Some((base_info.pixels, base_info.opacity)),
        _ => None,
    };

    // Scalar head: pixels before the first 4-aligned boundary.
    let aligned_start = (pixel_start + 3) & !3;
    let head_end = aligned_start.min(pixel_end);
    for pixel_index in pixel_start..head_end {
        match info.mode {
            LayerMode::MaskedDown => {
                let mask_a = scale_alpha(info.pixels[pixel_index].a(), info.opacity);
                let byte_index = pixel_index * BYTES_PER_PIXEL;
                let acc_a = output[byte_index + 3] as u32;
                output[byte_index + 3] = ((acc_a * mask_a as u32 + 128) >> 8) as u8;
            }
            LayerMode::ClippedDown | LayerMode::Normal => {
                let mut pixel = info.pixels[pixel_index];
                pixel = scale_pixel_opacity(pixel, info.opacity);
                if let Some((base_pixels, base_opacity)) = clip_base {
                    let base_a = scale_alpha(base_pixels[pixel_index].a(), base_opacity);
                    let f = base_a as u32;
                    pixel = Color32::from_rgba_premultiplied(
                        ((pixel.r() as u32 * f + 128) >> 8) as u8,
                        ((pixel.g() as u32 * f + 128) >> 8) as u8,
                        ((pixel.b() as u32 * f + 128) >> 8) as u8,
                        ((pixel.a() as u32 * f + 128) >> 8) as u8,
                    );
                }
                let byte_index = pixel_index * BYTES_PER_PIXEL;
                let dst = Color32::from_rgba_premultiplied(
                    output[byte_index],
                    output[byte_index + 1],
                    output[byte_index + 2],
                    output[byte_index + 3],
                );
                let blended = alpha_blend(dst, pixel);
                let arr = blended.to_array();
                output[byte_index..byte_index + 4].copy_from_slice(&arr);
            }
        }
    }

    // SIMD-aligned body.
    let aligned_end = pixel_end & !3;
    if aligned_start < aligned_end {
        let simd_pixel_count = aligned_end - aligned_start;
        let simd_chunks = simd_pixel_count / 4;
        let byte_start = aligned_start * BYTES_PER_PIXEL;
        let aligned_output =
            &mut output[byte_start..byte_start + simd_pixel_count * BYTES_PER_PIXEL];

        match info.mode {
            LayerMode::MaskedDown => {
                if parallel && simd_chunks > PARALLEL_BLEND_THRESHOLD {
                    aligned_output
                        .par_chunks_mut(16)
                        .enumerate()
                        .for_each(|(ci, chunk)| {
                            apply_mask_simd_chunk(
                                chunk,
                                info.pixels,
                                info.opacity,
                                aligned_start + ci * 4,
                            );
                        });
                } else {
                    for (ci, chunk) in aligned_output.chunks_mut(16).enumerate() {
                        apply_mask_simd_chunk(
                            chunk,
                            info.pixels,
                            info.opacity,
                            aligned_start + ci * 4,
                        );
                    }
                }
            }
            LayerMode::ClippedDown | LayerMode::Normal => {
                if parallel && simd_chunks > PARALLEL_BLEND_THRESHOLD {
                    aligned_output
                        .par_chunks_mut(16)
                        .enumerate()
                        .for_each(|(ci, chunk)| {
                            blend_simd_chunk_over(
                                chunk,
                                info.pixels,
                                info.opacity,
                                aligned_start + ci * 4,
                                clip_base,
                            );
                        });
                } else {
                    for (ci, chunk) in aligned_output.chunks_mut(16).enumerate() {
                        blend_simd_chunk_over(
                            chunk,
                            info.pixels,
                            info.opacity,
                            aligned_start + ci * 4,
                            clip_base,
                        );
                    }
                }
            }
        }
    }

    // Scalar tail.
    let tail_start = aligned_end.max(pixel_start);
    for pixel_index in tail_start..pixel_end {
        match info.mode {
            LayerMode::MaskedDown => {
                let mask_a = scale_alpha(info.pixels[pixel_index].a(), info.opacity);
                let byte_index = pixel_index * BYTES_PER_PIXEL;
                let acc_a = output[byte_index + 3] as u32;
                output[byte_index + 3] = ((acc_a * mask_a as u32 + 128) >> 8) as u8;
            }
            LayerMode::ClippedDown | LayerMode::Normal => {
                let mut pixel = info.pixels[pixel_index];
                pixel = scale_pixel_opacity(pixel, info.opacity);
                if let Some((base_pixels, base_opacity)) = clip_base {
                    let base_a = scale_alpha(base_pixels[pixel_index].a(), base_opacity);
                    let f = base_a as u32;
                    pixel = Color32::from_rgba_premultiplied(
                        ((pixel.r() as u32 * f + 128) >> 8) as u8,
                        ((pixel.g() as u32 * f + 128) >> 8) as u8,
                        ((pixel.b() as u32 * f + 128) >> 8) as u8,
                        ((pixel.a() as u32 * f + 128) >> 8) as u8,
                    );
                }
                let byte_index = pixel_index * BYTES_PER_PIXEL;
                let dst = Color32::from_rgba_premultiplied(
                    output[byte_index],
                    output[byte_index + 1],
                    output[byte_index + 2],
                    output[byte_index + 3],
                );
                let blended = alpha_blend(dst, pixel);
                let arr = blended.to_array();
                output[byte_index..byte_index + 4].copy_from_slice(&arr);
            }
        }
    }
}

/// Compute the base layer index for each layer.
///
/// For a layer with [`LayerMode::ClippedDown`], the base is the nearest layer
/// below it whose mode is **not** `ClippedDown`.  Consecutive `ClippedDown` layers
/// all reference the same base (Photoshop-style clipping mask chain).
/// For `Normal` and `MaskedDown` layers the base index is set to the
/// layer's own index (unused).
fn compute_base_indices(layers: &[LayerBlendInfo]) -> Vec<usize> {
    let mut bases = Vec::with_capacity(layers.len());
    let mut current_base = 0;
    for (i, info) in layers.iter().enumerate() {
        if info.mode == LayerMode::ClippedDown {
            bases.push(current_base);
        } else {
            current_base = i;
            bases.push(i);
        }
    }
    bases
}

/// Composite multiple premultiplied layers into an RGBA byte buffer.
///
/// Layers are processed bottom-to-top (index 0 = bottommost).
/// Each layer has a per-layer opacity (0–255) and compositing mode
/// ([`LayerMode`]).
///
/// Uses SIMD (`wide::u32x4`) via rayon for parallel processing of 4-pixel
/// chunks; remaining pixels are handled with scalar `alpha_blend`.
///
/// # Parameters
///
/// * `layers` — Slice of layer blend info, ordered bottom-to-top.
/// * `output` — RGBA output buffer (`len = layers[0].pixels.len() * 4`).
///
/// # Panics
///
/// - If `layers` is empty.
/// - If any layer has a different number of pixels from `layers[0]`.
/// - If `output.len() != layers[0].pixels.len() * 4`.
#[inline]
pub fn blend_layers(layers: &[LayerBlendInfo], output: &mut [u8]) {
    assert!(
        !layers.is_empty(),
        "blend_layers: at least one layer required"
    );

    let pixel_count = layers[0].pixels.len();
    #[cfg(debug_assertions)]
    for (layer_index, li) in layers.iter().enumerate() {
        assert_eq!(
            li.pixels.len(),
            pixel_count,
            "blend_layers: layer {layer_index} length mismatch"
        );
    }
    assert_eq!(
        output.len(),
        pixel_count * BYTES_PER_PIXEL,
        "blend_layers: output length mismatch"
    );

    // Zero the output buffer — start from transparent.
    output.fill(0);

    // Pre-compute ClippedDown base indices.
    let base_indices = compute_base_indices(layers);

    for (i, info) in layers.iter().enumerate() {
        let base = match info.mode {
            LayerMode::ClippedDown => Some(&layers[base_indices[i]]),
            _ => None,
        };
        apply_single_layer(output, info, base, 0, pixel_count, true);
    }
}

/// Blend only the pixels within a dirty rectangle.
///
/// Processes the region row-by-row. Sequential iteration is used since
/// dirty rects from brush strokes are typically small enough that parallel
/// overhead would dominate.
///
/// # Parameters
///
/// * `layers` — Slice of layer blend info, ordered bottom-to-top.
/// * `output` — RGBA output buffer (full-canvas length).
/// * `canvas_width` — Width of the canvas in pixels (for row stride computation).
/// * `min_x` — Leftmost column of the region to blend (inclusive).
/// * `min_y` — Topmost row of the region to blend (inclusive).
/// * `max_x` — Rightmost column of the region to blend (inclusive).
/// * `max_y` — Bottommost row of the region to blend (inclusive).
///
/// # Panics
///
/// Panics if any layer has fewer pixels than required by the region bounds,
/// or if `output` is too small for the required byte range.
#[inline]
pub fn blend_region(
    layers: &[LayerBlendInfo],
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
    let base_indices = compute_base_indices(layers);

    for y in min_y..=max_y {
        let pixel_start = (y as usize) * width + min_x as usize;
        let pixel_count = (max_x - min_x + 1) as usize;

        for (i, info) in layers.iter().enumerate() {
            let base = match info.mode {
                LayerMode::ClippedDown => Some(&layers[base_indices[i]]),
                _ => None,
            };
            apply_single_layer(output, info, base, pixel_start, pixel_count, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── unpremultiply ─────────────────────────────────────────

    #[test]
    fn unpremultiply_opaque_unchanged() {
        let c = Color32::from_rgba_premultiplied(200, 100, 50, 255);
        assert_eq!(unpremultiply(c), c);
    }

    #[test]
    fn unpremultiply_transparent_unchanged() {
        let c = Color32::TRANSPARENT;
        assert_eq!(unpremultiply(c), c);
    }

    #[test]
    fn unpremultiply_semi_transparent_corrects_rgb() {
        // Straight: (200, 100, 50, 128) → premultiplied: (100, 50, 25, 128)
        let premul = Color32::from_rgba_premultiplied(100, 50, 25, 128);
        let straight = unpremultiply(premul);
        assert!(straight.r() >= 199 && straight.r() <= 201);
        assert!(straight.g() >= 99 && straight.g() <= 101);
        assert!(straight.b() >= 49 && straight.b() <= 51);
        assert_eq!(straight.a(), 128);
    }

    // ── premultiply ────────────────────────────────────────────

    #[test]
    fn premultiply_opaque_unchanged() {
        let c = Color32::from_rgba_unmultiplied(200, 100, 50, 255);
        assert_eq!(premultiply(c), c);
    }

    #[test]
    fn premultiply_transparent_returns_transparent() {
        let c = Color32::from_rgba_unmultiplied(200, 100, 50, 0);
        assert_eq!(premultiply(c), Color32::TRANSPARENT);
    }

    #[test]
    fn premultiply_semi_transparent_scales_rgb() {
        let c = Color32::from_rgba_unmultiplied(200, 100, 50, 128);
        let p = premultiply(c);
        assert!(p.r() <= 200);
        assert!(p.g() <= 100);
        assert!(p.b() <= 50);
        assert_eq!(p.a(), 128);
    }

    // ── alpha_blend ────────────────────────────────────────────

    #[test]
    fn alpha_blend_opaque_source_covers_dest() {
        let dest = Color32::from_rgba_premultiplied(10, 20, 30, 40);
        let src = Color32::from_rgba_premultiplied(200, 100, 50, 255);
        assert_eq!(alpha_blend(dest, src), src);
    }

    #[test]
    fn alpha_blend_transparent_source_leaves_dest_unchanged() {
        let dest = Color32::from_rgba_premultiplied(200, 100, 50, 255);
        let src = Color32::TRANSPARENT;
        let result = alpha_blend(dest, src);
        assert!(
            result.r() == 200 || result.r() == 199,
            "expected ~200, got {}",
            result.r()
        );
        assert!(
            result.g() == 100 || result.g() == 99,
            "expected ~100, got {}",
            result.g()
        );
        assert!(
            result.b() == 50 || result.b() == 49,
            "expected ~50, got {}",
            result.b()
        );
        assert!(
            result.a() == 255 || result.a() == 254,
            "expected ~255, got {}",
            result.a()
        );
    }

    #[test]
    fn alpha_blend_semi_transparent_combines() {
        let dest = Color32::from_rgba_premultiplied(50, 50, 50, 128);
        let src = Color32::from_rgba_premultiplied(200, 100, 50, 128);
        let result = alpha_blend(dest, src);
        assert_eq!(result.a(), 192);
    }

    // ── blend_layers ───────────────────────────────────────────

    /// Helper to build a `LayerBlendInfo` with Normal mode.
    fn normal(pixels: &[Color32], opacity: u8) -> LayerBlendInfo {
        LayerBlendInfo {
            pixels,
            opacity,
            mode: LayerMode::Normal,
        }
    }

    #[test]
    fn blend_layers_single_opaque_passthrough() {
        let pixels = vec![
            Color32::from_rgba_premultiplied(100, 200, 50, 255),
            Color32::from_rgba_premultiplied(50, 100, 200, 255),
        ];
        let layers = [normal(&pixels, 255)];
        let mut out = vec![0u8; pixels.len() * 4];
        blend_layers(&layers, &mut out);
        assert_eq!(out.len(), pixels.len() * 4);
        for (i, &color) in pixels.iter().enumerate() {
            let arr = color.to_array();
            assert_eq!(out[i * 4..i * 4 + 4], arr);
        }
    }

    #[test]
    fn blend_layers_two_opaque_top_wins() {
        let bottom = vec![Color32::from_rgba_premultiplied(10, 20, 30, 40)];
        let top = vec![Color32::from_rgba_premultiplied(200, 100, 50, 255)];
        let layers = [normal(&bottom, 255), normal(&top, 255)];
        let mut out = vec![0u8; 4];
        blend_layers(&layers, &mut out);
        assert_eq!(out.as_slice(), top[0].to_array());
    }

    #[test]
    fn blend_layers_with_opacity_scales() {
        let bottom = vec![Color32::from_rgba_premultiplied(100, 0, 0, 255)];
        let top = vec![Color32::from_rgba_premultiplied(0, 200, 0, 255)];
        let layers = [normal(&bottom, 255), normal(&top, 128)];
        let mut out = vec![0u8; 4];
        blend_layers(&layers, &mut out);
        assert_eq!(out[3], 255);
        assert!(out[1] >= 98 && out[1] <= 102);
    }

    #[test]
    fn blend_layers_clip_down_clips_to_base_alpha() {
        let base = vec![
            Color32::from_rgba_premultiplied(255, 0, 0, 255),
            Color32::from_rgba_premultiplied(0, 0, 0, 0),
        ];
        let clipped = vec![
            Color32::from_rgba_premultiplied(0, 255, 0, 255),
            Color32::from_rgba_premultiplied(0, 255, 0, 255),
        ];
        let layers = [
            normal(&base, 255),
            LayerBlendInfo {
                pixels: &clipped,
                opacity: 255,
                mode: LayerMode::ClippedDown,
            },
        ];
        let mut out = vec![0u8; 8];
        blend_layers(&layers, &mut out);
        assert!(
            out[0] <= 1,
            "pixel 0 red should be ~0, got {}",
            out[0]
        );
        assert!(
            out[1] >= 254,
            "pixel 0 green should be ~255, got {}",
            out[1]
        );
        assert_eq!(out[2], 0, "pixel 0 blue");
        assert_eq!(out[3], 255, "pixel 0 alpha");
        assert_eq!(
            out[4..8],
            [0, 0, 0, 0],
            "pixel 1 should be transparent"
        );
    }

    #[test]
    fn blend_layers_mask_down_modulates_accumulator_alpha() {
        let bottom = vec![Color32::from_rgba_premultiplied(255, 255, 255, 255)];
        let mask = vec![Color32::from_rgba_premultiplied(0, 0, 0, 128)];
        let layers = [
            normal(&bottom, 255),
            LayerBlendInfo {
                pixels: &mask,
                opacity: 255,
                mode: LayerMode::MaskedDown,
            },
        ];
        let mut out = vec![0u8; 4];
        blend_layers(&layers, &mut out);
        assert_eq!(
            out[0..4],
            [255, 255, 255, 128],
            "mask should halve alpha, keep RGB"
        );
    }

    #[test]
    #[should_panic(expected = "blend_layers: at least one layer required")]
    fn blend_layers_empty_panics() {
        blend_layers(&[], &mut []);
    }

    #[test]
    #[should_panic]
    fn blend_layers_wrong_output_length_panics() {
        let pixels = vec![Color32::from_rgba_premultiplied(255, 255, 255, 255)];
        blend_layers(&[normal(&pixels, 255)], &mut [0u8; 3]);
    }

    // ── blend_region ───────────────────────────────────────────

    #[test]
    fn blend_region_empty_layers_noop() {
        let mut out = [0u8; 16];
        blend_region(&[], &mut out, 4, 0, 0, 3, 3);
        assert_eq!(out, [0u8; 16]);
    }

    #[test]
    fn blend_region_single_layer_writes_subset() {
        let pixels = vec![
            Color32::from_rgba_premultiplied(1, 0, 0, 255),
            Color32::from_rgba_premultiplied(2, 0, 0, 255),
            Color32::from_rgba_premultiplied(3, 0, 0, 255),
            Color32::from_rgba_premultiplied(4, 0, 0, 255),
        ];
        let mut out = vec![0u8; 16];
        blend_region(&[normal(&pixels, 255)], &mut out, 2, 1, 0, 1, 0);
        assert_eq!(out[4..8], pixels[1].to_array());
        assert_eq!(out[0..4], [0u8; 4]);
    }
}
