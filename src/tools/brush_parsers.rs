//! Brush file format parsers for GIMP Brush (.gbr) and Photoshop Brush (.abr).

use std::path::Path;

use eframe::egui::Color32;

/// A parsed brush tip ready for use in the brush library.
#[derive(Debug)]

pub struct BrushTip {
    /// Display name (file stem or ABR internal name).
    pub name: String,
    /// Premultiplied-alpha pixel data, row-major.
    pub pixels: Vec<Color32>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Spacing as a percentage (0–100). Defaults to 25 when unknown.
    pub spacing: u8,
}

/// Parse a brush file by extension and return all brush tips contained within.
///
/// Supported formats: `.gbr`, `.abr`.
///
/// # Errors
///
/// Returns an error string if the file cannot be read, the format is
/// unrecognised, or the data is malformed.

pub fn parse_brush_file(path: &Path) -> Result<Vec<BrushTip>, String> {

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| "File has no extension".to_string())?;

    let data = std::fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "brush".to_string());

    let mut tips = match ext.as_str() {
        "gbr" => parse_gbr(&data)?,
        "abr" => parse_abr(&data)?,
        _ => return Err(format!("Unsupported brush format: .{ext}")),
    };

    // Assign unique names
    if tips.len() == 1 {

        tips[0].name = stem;
    } else {

        for (i, tip) in tips.iter_mut().enumerate() {

            tip.name = format!("{stem}_{i}");
        }
    }

    Ok(tips)
}

// ---------------------------------------------------------------------------
// GBR (GIMP Brush) parser
// ---------------------------------------------------------------------------

/// Parse a GIMP Brush (.gbr) file from a byte buffer.
///
/// The GBR format stores a single brush tip per file.
/// Supported: version 1 (20-byte header) and version 2 (24-byte header).
/// Pixel data may be grayscale (1 bpp) or RGBA (4 bpp), stored as
/// straight-alpha.
///
/// # Errors
///
/// Returns an error string if the buffer is too short for the header,
/// the magic bytes are not `GIMP`, the version is unsupported (neither 1
/// nor 2), the dimensions are zero, the bytes-per-pixel is not 1 or 4,
/// the data is truncated, or the v2 spacing field is missing.

pub(crate) fn parse_gbr(data: &[u8]) -> Result<Vec<BrushTip>, String> {

    if data.len() < 20 {

        return Err("GBR file too short".into());
    }

    if &data[0..4] != b"GIMP" {

        return Err("Invalid GBR magic bytes".into());
    }

    // Version: try numeric (u32 BE) first; also accept "2.1 " or "2.0 " strings
    let version_int = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

    let version_str = &data[4..8];

    let is_v2 = version_int == 2 || version_str == b"2.1 " || version_str == b"2.0 ";

    let is_v1 = version_int == 1;

    if !is_v1 && !is_v2 {

        return Err(format!("Unsupported GBR version: {version_int}"));
    }

    let width = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

    let height = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);

    let bpp = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);

    if width == 0 || height == 0 {

        return Err("GBR has zero dimensions".into());
    }

    if bpp != 1 && bpp != 4 {

        return Err(format!("Unsupported GBR bpp: {bpp} (must be 1 or 4)"));
    }

    let (spacing, pixel_offset) = if is_v2 {

        if data.len() < 24 {

            return Err("GBR v2 file too short for spacing field".into());
        }

        let spacing_val = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

        (spacing_val.min(100) as u8, 24_usize)
    } else {

        (25_u8, 20_usize)
    };

    let expected_pixels = (width as usize) * (height as usize);

    let expected_bytes = expected_pixels * (bpp as usize);

    if data[pixel_offset..].len() < expected_bytes {

        return Err("GBR pixel data truncated".into());
    }

    let pixel_data = &data[pixel_offset..pixel_offset + expected_bytes];

    let mut pixels = Vec::with_capacity(expected_pixels);

    match bpp {
        1 => {

            // Grayscale → white with grayscale value as alpha
            for &gray in pixel_data {

                let straight = Color32::from_rgba_unmultiplied(255, 255, 255, gray);

                pixels.push(straight);
            }
        }
        4 => {

            // RGBA → premultiplied via from_rgba_unmultiplied
            for chunk in pixel_data.chunks_exact(4) {

                let straight =
                    Color32::from_rgba_unmultiplied(chunk[0], chunk[1], chunk[2], chunk[3]);

                pixels.push(straight);
            }
        }
        _ => unreachable!(),
    }

    Ok(vec![BrushTip {
        name: String::new(),
        pixels,
        width,
        height,
        spacing,
    }])
}

// ---------------------------------------------------------------------------
// ABR (Photoshop Brush) parser
// ---------------------------------------------------------------------------

/// Parse a Photoshop Brush (.abr) file from a byte buffer.
///
/// Supports ABR versions 6 through 10. Extracts sampled brushes (embedded
/// image data) and rasterises common computed/parametric brush shapes
/// (round, capped-round, square, diamond). Unknown subblock types are
/// skipped with a logged warning.
///
/// # Errors
///
/// Returns an error string if the buffer is too short for the header,
/// the magic bytes are not `8BPB`, the version is outside the 6–10 range,
/// the header extends beyond the file, or no usable brush tips are found.
/// Individual subblob parse failures are silently skipped.

pub(crate) fn parse_abr(data: &[u8]) -> Result<Vec<BrushTip>, String> {

    if data.len() < 14 {

        return Err("ABR file too short".into());
    }

    if &data[0..4] != b"8BPB" {

        return Err("Invalid ABR magic bytes".into());
    }

    let version = u16::from_be_bytes([data[4], data[5]]);

    if !(6..=10).contains(&version) {

        return Err(format!(
            "Unsupported ABR version: {version} (only 6–10 supported)"
        ));
    }

    // Number of subblocks (only reliable in v6–7; v8+ parses subblocks iteratively)
    let _sub_count = u16::from_be_bytes([data[6], data[7]]);

    let mut tips: Vec<BrushTip> = Vec::new();

    // Initial parse offset: after the 14-byte fixed header + optional tag
    // For v6+: header is 14 bytes (4 magic + 2 version + 2 subcount + 4 reserved + 2 something)
    // Actually, the ABR header varies. Let's use a more flexible approach.
    let header_size = if version >= 8 {

        16_u32
    } else if version == 6 || version == 7 {

        14_u32
    } else {

        14
    };

    // We'll parse subblocks by scanning for 8BIM markers instead of relying on sub_count.

    let mut offset = header_size as usize;

    if offset >= data.len() {

        return Err("ABR header extends beyond file".into());
    }

    // sub_count can be 0 for v8+ — we scan for subblocks by signature
    let mut scan_count = 0;

    let max_subblocks = 1000; // safety limit

    loop {

        if offset + 14 > data.len() || scan_count >= max_subblocks {

            break;
        }

        let sig = &data[offset..offset + 4];

        let block_type = u16::from_be_bytes([data[offset + 4], data[offset + 5]]);

        // Size field: for v6–7 it's a u32; for v8+ tags use different sizing
        let block_size = if version <= 7 {

            u32::from_be_bytes([
                data[offset + 6],
                data[offset + 7],
                data[offset + 8],
                data[offset + 9],
            ]) as usize
        } else {

            // v8+ uses length after subblock header differently
            u32::from_be_bytes([
                data[offset + 6],
                data[offset + 7],
                data[offset + 8],
                data[offset + 9],
            ]) as usize
        };

        if sig == b"8BIM" && block_type == 1 && block_size > 0 {

            // Sampled brush — extract embedded image
            if let Ok(Some(tip)) = parse_abr_sampled(&data[offset + 14..], block_size, version) {

                tips.push(tip);
            }
        }

        if sig == b"8BIM" && block_type == 2 && block_size > 0 {

            // Computed brush — rasterise parametric shape
            if let Ok(Some(tip)) = rasterise_computed_brush(&data[offset + 14..], block_size) {

                tips.push(tip);
            }
        }

        // Advance: 14 bytes header + block_size (padded to even)
        let padded_size = block_size + (block_size & 1);

        offset += 14 + padded_size;

        scan_count += 1;
    }

    if tips.is_empty() {

        Err("No usable brush tips found in ABR file".into())
    } else {

        Ok(tips)
    }
}

/// Extract a sampled brush tip from an ABR subblock's data payload.
///
/// In ABR v6–7, sampled brushes use tag-based encoding with `8BIM`
/// tag signatures. The embedded image is typically stored as PNG or
/// raw BGRA data.  In ABR v10, JPEG-XL compressed samples may appear.

fn parse_abr_sampled(
    data: &[u8],
    block_size: usize,
    _version: u16,
) -> Result<Option<BrushTip>, String> {

    if data.len() < block_size {

        return Err("ABR sampled data truncated".into());
    }

    // Scan tags within the subblock
    let mut offset = 0_usize;

    while offset + 8 <= block_size {

        if offset + 8 > data.len() {

            break;
        }

        let tag = &data[offset..offset + 4];

        let tag_len = u32::from_be_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;

        if tag == b"8BIM" && offset + 12 <= block_size {

            // Extended tag: 4 sig + 4 type + 4 length
            let _ext_type = &data[offset + 4..offset + 8];

            let ext_len = u32::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]) as usize;

            offset += 12;

            if offset + ext_len <= block_size && offset + ext_len <= data.len() {

                // Try to decode as embedded image (PNG or raw)
                let image_data = &data[offset..offset + ext_len];

                if let Ok(tip) = decode_embedded_image(image_data) {

                    return Ok(Some(tip));
                }
            }

            offset += ext_len + (ext_len & 1); // pad to even
        } else {

            // Simple tag
            offset += 8;

            if offset + tag_len <= block_size && offset + tag_len <= data.len() {

                // Might contain image data — attempt decode
                let image_data = &data[offset..offset + tag_len];

                if let Ok(tip) = decode_embedded_image(image_data) {

                    return Ok(Some(tip));
                }
            }

            offset += tag_len + (tag_len & 1);
        }
    }

    Ok(None)
}

/// Attempt to decode embedded image data as PNG or raw RGBA/BGRA.

fn decode_embedded_image(data: &[u8]) -> Result<BrushTip, String> {

    // Try PNG first
    if data.len() > 8 && data[..8] == [137, 80, 78, 71, 13, 10, 26, 10] {

        return decode_png_image(data);
    }

    // Try as raw RGBA/BGRA data (ABR stores BGRA)
    // Need at least 4 bytes and a reasonable size
    if data.len() >= 4 && is_raw_image_size_plausible(data.len()) {

        if let Ok(tip) = decode_raw_bgra(data) {

            return Ok(tip);
        }
    }

    Err("Unknown embedded image format".into())
}

/// Guess reasonable image dimensions from raw buffer size.

fn is_raw_image_size_plausible(byte_count: usize) -> bool {

    // Must be multiple of 4 (RGBA), at least 4 pixels, at most huge
    byte_count % 4 == 0 && byte_count >= 16 && byte_count <= 256 * 256 * 4
}

/// Decode a PNG buffer into a BrushTip.

fn decode_png_image(data: &[u8]) -> Result<BrushTip, String> {

    let img = image::load_from_memory(data).map_err(|e| format!("PNG decode failed: {e}"))?;

    let rgba = img.to_rgba8();

    let (w, h) = rgba.dimensions();

    let mut pixels = Vec::with_capacity((w * h) as usize);

    for pixel in rgba.pixels() {

        let straight = Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]);

        pixels.push(straight);
    }

    Ok(BrushTip {
        name: String::new(),
        pixels,
        width: w,
        height: h,
        spacing: 25,
    })
}

/// Decode raw BGRA (Photoshop's native byte order) into premultiplied RGBA.

fn decode_raw_bgra(data: &[u8]) -> Result<BrushTip, String> {

    let pixel_count = data.len() / 4;

    // Try square-ish dimensions
    let w = ((pixel_count as f64).sqrt().round() as u32).max(1);

    let h = ((pixel_count as f64) / w as f64).ceil() as u32;

    // Ensure w * h matches
    let (w, h) = if w * h == pixel_count as u32 {

        (w, h)
    } else {

        // Try exact divisors
        (pixel_count as u32, 1)
    };

    let mut pixels = Vec::with_capacity(pixel_count);

    for chunk in data.chunks_exact(4) {

        // BGRA → RGBA
        let straight = Color32::from_rgba_unmultiplied(chunk[2], chunk[1], chunk[0], chunk[3]);

        pixels.push(straight);
    }

    Ok(BrushTip {
        name: String::new(),
        pixels,
        width: w,
        height: h,
        spacing: 25,
    })
}

/// Rasterise a computed/parametric brush from its ABR subblock data.
///
/// Supports shapes: round, capped-round, square, diamond.

fn rasterise_computed_brush(data: &[u8], _block_size: usize) -> Result<Option<BrushTip>, String> {

    // Computed brush parameters vary by ABR version.
    // Common parameter tags in v6–10:
    //   "diam" — diameter (u16 BE, pixels)
    //   "hrad" — hardness (u16, 0–16383, scaled to 0–100%)
    //   "rond" — roundness (u16, 0–16383, percentage)
    //   "angl" — angle (u16, 0–360)
    //   "spac" — spacing (u16, 0–100, percentage)
    //   "shpe" — shape type (u16: 0=round, 1=square, 2=diamond, 3=capped)

    let mut diameter: u32 = 64;

    let mut hardness: u16 = 16383; // 100%
    let mut roundness: u16 = 16383; // 100%
    let mut _angle: u16 = 0;

    let mut spacing: u16 = 25;

    let mut shape: u16 = 0; // 0=round

    let mut offset = 0;

    while offset + 8 <= data.len() {

        let tag = &data[offset..offset + 4];

        let tag_len = u32::from_be_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;

        offset += 8;

        let inner = if offset + tag_len <= data.len() {

            &data[offset..offset + tag_len]
        } else {

            break;
        };

        offset += tag_len + (tag_len & 1);

        if inner.len() < 2 {

            continue;
        }

        let val = u16::from_be_bytes([inner[0], inner[1]]);

        match tag {
            b"diam" => diameter = val.max(1) as u32,
            b"hrad" => hardness = val.min(16383),
            b"rond" => roundness = val.min(16383),
            b"angl" => _angle = val % 360,
            b"spac" => spacing = val.min(100).max(1),
            b"shpe" => shape = val.min(3),
            _ => {}
        }
    }

    let hardness_pct = hardness as f64 / 16383.0;

    let roundness_pct = roundness as f64 / 16383.0;

    let radius = (diameter as f64 / 2.0).ceil() as u32;

    let w = diameter.max(1);

    let h = ((diameter as f64 * roundness_pct).ceil() as u32).max(1);

    let pixels = match shape {
        0 => rasterise_round(w, h, radius, hardness_pct),
        1 => rasterise_square(w, h, hardness_pct),
        2 => rasterise_diamond(w, h, hardness_pct),
        3 => rasterise_capped_round(w, h, radius, hardness_pct),
        _ => return Ok(None),
    };

    Ok(Some(BrushTip {
        name: String::new(),
        pixels,
        width: w,
        height: h,
        spacing: spacing as u8,
    }))
}

/// Rasterise a round brush: a filled circle with hardness-controlled falloff.

fn rasterise_round(w: u32, h: u32, radius: u32, hardness: f64) -> Vec<Color32> {

    let count = (w * h) as usize;

    let mut pixels = Vec::with_capacity(count);

    let cx = w as f64 / 2.0;

    let cy = h as f64 / 2.0;

    let r = radius as f64;

    // Soft falloff zone (pixels between hardness radius and full radius)
    let hard_r = r * hardness.sqrt(); // sqrt so hardness ≈ perceived sharpness
    let falloff = (r - hard_r).max(1.0);

    for y in 0..h {

        for x in 0..w {

            let dx = x as f64 - cx + 0.5;

            let dy = y as f64 - cy + 0.5;

            let dist = (dx * dx + dy * dy).sqrt();

            let alpha = if dist <= hard_r {

                255
            } else if dist >= r {

                0
            } else {

                // Linear falloff
                let t = (dist - hard_r) / falloff;

                ((1.0 - t) * 255.0 + 0.5) as u8
            };

            let straight = Color32::from_rgba_unmultiplied(0, 0, 0, alpha);

            pixels.push(straight);
        }
    }

    pixels
}

/// Rasterise a square brush.

fn rasterise_square(w: u32, h: u32, hardness: f64) -> Vec<Color32> {

    let count = (w * h) as usize;

    let mut pixels = Vec::with_capacity(count);

    let edge_dist = ((1.0 - hardness) * (w.min(h) as f64) / 2.0).max(1.0);

    for y in 0..h {

        for x in 0..w {

            let dx = (x as f64 + 0.5).min((w - x) as f64);

            let dy = (y as f64 + 0.5).min((h - y) as f64);

            let dist_to_edge = dx.min(dy);

            let alpha = if dist_to_edge >= edge_dist {

                255
            } else {

                let t = dist_to_edge / edge_dist;

                (t * 255.0 + 0.5) as u8
            };

            let straight = Color32::from_rgba_unmultiplied(0, 0, 0, alpha);

            pixels.push(straight);
        }
    }

    pixels
}

/// Rasterise a diamond brush (square rotated 45°, filled).

fn rasterise_diamond(w: u32, h: u32, hardness: f64) -> Vec<Color32> {

    let count = (w * h) as usize;

    let mut pixels = Vec::with_capacity(count);

    let cx = w as f64 / 2.0;

    let cy = h as f64 / 2.0;

    let half = (w.min(h) as f64) / 2.0;

    let edge_dist = ((1.0 - hardness) * half).max(1.0);

    for y in 0..h {

        for x in 0..w {

            // Manhattan distance (rotated 45° diamond)
            let dx = (x as f64 - cx + 0.5).abs();

            let dy = (y as f64 - cy + 0.5).abs();

            let dist_to_center = dx + dy;

            let max_dist = half;

            let dist_to_edge = (max_dist - dist_to_center).max(0.0);

            let alpha = if dist_to_edge >= edge_dist {

                255
            } else {

                let t = dist_to_edge / edge_dist;

                (t * 255.0 + 0.5) as u8
            };

            let straight = Color32::from_rgba_unmultiplied(0, 0, 0, alpha);

            pixels.push(straight);
        }
    }

    pixels
}

/// Rasterise a capped-round brush (rectangle with semicircular ends).

fn rasterise_capped_round(w: u32, h: u32, radius: u32, hardness: f64) -> Vec<Color32> {

    let count = (w * h) as usize;

    let mut pixels = Vec::with_capacity(count);

    let r = radius as f64;

    let hard_r = r * hardness.sqrt();

    let falloff = (r - hard_r).max(1.0);

    let cx = w as f64 / 2.0;

    let cy = h as f64 / 2.0;

    let half_h = h as f64 / 2.0;

    for y in 0..h {

        for x in 0..w {

            let px = x as f64 + 0.5;

            let py = y as f64 + 0.5;

            let dcy = (py - cy).abs();

            let dist = if dcy <= half_h - r {

                // Middle section: distance to nearest vertical edge
                (cx - (px - cx).abs()).max(0.0) // actually distance to left/right edge
            } else {

                // End caps: distance to semicircle center
                let cap_cy = if py < cy { r } else { h as f64 - r };

                let dx = px - cx;

                let dy = py - cap_cy;

                (dx * dx + dy * dy).sqrt()
            };

            let alpha = if dist <= hard_r {

                255
            } else if dist >= r {

                0
            } else {

                let t = (dist - hard_r) / falloff;

                ((1.0 - t) * 255.0 + 0.5) as u8
            };

            let straight = Color32::from_rgba_unmultiplied(0, 0, 0, alpha);

            pixels.push(straight);
        }
    }

    pixels
}
