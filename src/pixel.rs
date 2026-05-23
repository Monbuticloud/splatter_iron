use eframe::egui::Color32;
use rayon::prelude::*;
use wide::u32x4;

/// Premultiply a straight-alpha Color32.
///
/// **Caller must supply straight (non-premultiplied) RGB.**
/// Calling this on an already-premultiplied pixel will darken colors again.
#[inline(always)]
pub fn premultiply(color: Color32) -> Color32 {
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

/// Alpha-blend premultiplied source over premultiplied destination.
/// Result is premultiplied.
#[inline(always)]
pub fn alpha_blend(destination: Color32, source: Color32) -> Color32 {
    // Extract source channels
    let source_red = source.r() as u32;
    let source_green = source.g() as u32;
    let source_blue = source.b() as u32;
    let source_alpha = source.a() as u32;

    // Extract destination channels
    let dest_red = destination.r() as u32;
    let dest_green = destination.g() as u32;
    let dest_blue = destination.b() as u32;
    let dest_alpha = destination.a() as u32;

    // Inverse alpha: how much of destination shows through
    let inverse_alpha = 255 - source_alpha;

    // Blend one channel: dest * inverse_alpha, rounded
    #[inline(always)]
    fn blend_channel(destination_channel: u32, inverse_alpha: u32) -> u32 {
        (destination_channel * inverse_alpha + 128) >> 8
    }

    Color32::from_rgba_premultiplied(
        (source_red + blend_channel(dest_red, inverse_alpha)) as u8,
        (source_green + blend_channel(dest_green, inverse_alpha)) as u8,
        (source_blue + blend_channel(dest_blue, inverse_alpha)) as u8,
        (source_alpha + blend_channel(dest_alpha, inverse_alpha)) as u8,
    )
}

// SIMD constants for the (value * alpha + 128) * 257 >> 16 fixed-point blend
const ROUNDING_BIAS_128: u32x4 = u32x4::splat(128);
const DIV_255_FACTOR_257: u32x4 = u32x4::splat(257);
const SHIFT_16: u32x4 = u32x4::splat(16);

/// Blend multiple premultiplied layers into an RGBA u8 output buffer.
///
/// Layers are composited bottom-to-top (index 0 = bottommost).
/// Uses SIMD (wide::u32x4) + rayon parallelism to process 4 pixels per task.
/// No heap allocation, no clones — only stack-local SIMD vectors.
///
/// # Panics
/// - If `layers` is empty
/// - If any layer has a different length from `layers[0]`
/// - If `output.len() != layers[0].len() * 4`
#[inline]
pub fn blend_layers(layers: &[&[Color32]], output: &mut [u8]) {
    // Validate inputs
    assert!(!layers.is_empty(), "blend_layers: at least one layer required");

    let pixel_count = layers[0].len();
    #[cfg(debug_assertions)]
    for (i, layer) in layers.iter().enumerate() {
        assert_eq!(
            layer.len(),
            pixel_count,
            "blend_layers: layer {i} length mismatch"
        );
    }
    assert_eq!(
        output.len(),
        pixel_count * 4,
        "blend_layers: output length mismatch"
    );

    // Fast path: single layer — just copy RGBA bytes directly
    if layers.len() == 1 {
        let source_layer = layers[0];
        for pixel_index in 0..pixel_count {
            let rgba_array = source_layer[pixel_index].to_array(); // [R, G, B, A]
            let output_index = pixel_index * 4;
            output[output_index] = rgba_array[0];
            output[output_index + 1] = rgba_array[1];
            output[output_index + 2] = rgba_array[2];
            output[output_index + 3] = rgba_array[3];
        }
        return;
    }

    // Split output into aligned 4-pixel SIMD chunks and scalar remainder
    let simd_chunks = pixel_count >> 2; // pixel_count / 4
    let aligned_byte_count = simd_chunks * 16; // 16 bytes per 4-pixel chunk
    let (aligned_buffer, remainder_buffer) = output.split_at_mut(aligned_byte_count);

    // --- Parallel SIMD for full 4-pixel chunks ---
    aligned_buffer
        .par_chunks_mut(16)
        .enumerate()
        .for_each(|(chunk_index, output_chunk)| {
            let pixel_base = chunk_index * 4;

            // Load bottom layer (index 0) pixels into SIMD accumulators
            let bottom_layer = layers[0];
            let bottom_pixel_0 = bottom_layer[pixel_base + 0].to_array();
            let bottom_pixel_1 = bottom_layer[pixel_base + 1].to_array();
            let bottom_pixel_2 = bottom_layer[pixel_base + 2].to_array();
            let bottom_pixel_3 = bottom_layer[pixel_base + 3].to_array();

            let mut accumulator_r =
                u32x4::new([bottom_pixel_0[0] as u32, bottom_pixel_1[0] as u32, bottom_pixel_2[0] as u32, bottom_pixel_3[0] as u32]);
            let mut accumulator_g =
                u32x4::new([bottom_pixel_0[1] as u32, bottom_pixel_1[1] as u32, bottom_pixel_2[1] as u32, bottom_pixel_3[1] as u32]);
            let mut accumulator_b =
                u32x4::new([bottom_pixel_0[2] as u32, bottom_pixel_1[2] as u32, bottom_pixel_2[2] as u32, bottom_pixel_3[2] as u32]);
            let mut accumulator_a =
                u32x4::new([bottom_pixel_0[3] as u32, bottom_pixel_1[3] as u32, bottom_pixel_2[3] as u32, bottom_pixel_3[3] as u32]);

            // Blend remaining layers (1..) on top of accumulators
            for &layer_slice in &layers[1..] {
                let top_pixel_0 = layer_slice[pixel_base + 0].to_array();
                let top_pixel_1 = layer_slice[pixel_base + 1].to_array();
                let top_pixel_2 = layer_slice[pixel_base + 2].to_array();
                let top_pixel_3 = layer_slice[pixel_base + 3].to_array();

                let top_r =
                    u32x4::new([top_pixel_0[0] as u32, top_pixel_1[0] as u32, top_pixel_2[0] as u32, top_pixel_3[0] as u32]);
                let top_g =
                    u32x4::new([top_pixel_0[1] as u32, top_pixel_1[1] as u32, top_pixel_2[1] as u32, top_pixel_3[1] as u32]);
                let top_b =
                    u32x4::new([top_pixel_0[2] as u32, top_pixel_1[2] as u32, top_pixel_2[2] as u32, top_pixel_3[2] as u32]);
                let top_a =
                    u32x4::new([top_pixel_0[3] as u32, top_pixel_1[3] as u32, top_pixel_2[3] as u32, top_pixel_3[3] as u32]);

                let inverse_alpha = u32x4::splat(255) - top_a;

                // Blend: accumulator = top + ((accumulator * inverse_alpha + 128) * 257) >> 16
                accumulator_r = top_r + (((accumulator_r * inverse_alpha + ROUNDING_BIAS_128) * DIV_255_FACTOR_257) >> SHIFT_16);
                accumulator_g = top_g + (((accumulator_g * inverse_alpha + ROUNDING_BIAS_128) * DIV_255_FACTOR_257) >> SHIFT_16);
                accumulator_b = top_b + (((accumulator_b * inverse_alpha + ROUNDING_BIAS_128) * DIV_255_FACTOR_257) >> SHIFT_16);
                accumulator_a = top_a + (((accumulator_a * inverse_alpha + ROUNDING_BIAS_128) * DIV_255_FACTOR_257) >> SHIFT_16);
            }

            // Write 4 blended pixels to output buffer as RGBA bytes
            let red_array = accumulator_r.to_array();
            let green_array = accumulator_g.to_array();
            let blue_array = accumulator_b.to_array();
            let alpha_array = accumulator_a.to_array();

            for pixel_offset in 0..4 {
                let output_index = pixel_offset * 4;
                output_chunk[output_index] = red_array[pixel_offset] as u8;
                output_chunk[output_index + 1] = green_array[pixel_offset] as u8;
                output_chunk[output_index + 2] = blue_array[pixel_offset] as u8;
                output_chunk[output_index + 3] = alpha_array[pixel_offset] as u8;
            }
        });

    // --- Scalar remainder for pixels not covered by full SIMD chunks ---
    let remainder_pixel_start = simd_chunks * 4;
    for (remainder_index, output_chunk) in remainder_buffer.chunks_mut(4).enumerate() {
        let pixel_index = remainder_pixel_start + remainder_index;
        let mut pixel = layers[0][pixel_index];

        // Blend remaining layers onto this pixel using scalar alpha_blend
        for &layer_slice in &layers[1..] {
            pixel = alpha_blend(pixel, layer_slice[pixel_index]);
        }

        let rgba_array = pixel.to_array();
        output_chunk[0] = rgba_array[0];
        output_chunk[1] = rgba_array[1];
        output_chunk[2] = rgba_array[2];
        output_chunk[3] = rgba_array[3];
    }
}