//! Serialization and deserialization of canvas data (zstd-compressed JSON,
//! `.splattercanvas` format), plus image export to 13 formats.

use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use bytemuck::cast_slice;
use eframe::egui::Color32;
use image::ImageEncoder;
use rayon::prelude::*;

use crate::canvas::Canvas;
use crate::canvas::DirtyRectList;
use crate::canvas::Layer;
use crate::pixel::F32_COLOR_MAX;
use crate::pixel::premultiply;
use crate::pixel::unpremultiply;

const COMPRESSION_LEVEL: i32 = 10;
const XZ_COMPRESSION_PRESET: u32 = 9;
const MAX_DECOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;
const JPEG_QUALITY: u8 = 100;

/// Decompress and deserialize a `Canvas` from any `std::io::Read` by streaming
/// the zstd decoder directly into `serde_json::from_reader`.
///
/// Eliminates the intermediate decompression `Vec<u8>` allocation — compressed
/// data is decompressed incrementally and parsed as JSON on the fly.
///
/// # Parameters
///
/// * `reader` — Source of zstd-compressed JSON bytes (e.g. `File`, `&[u8]`).
///
/// # Errors
///
/// Returns an error if zstd decompression or JSON deserialization fails,
/// if the decompressed data exceeds [`MAX_DECOMPRESSED_BYTES`],
/// or if the canvas has invalid dimensions or mismatched layer sizes.
fn read_canvas(reader: impl Read) -> anyhow::Result<Canvas> {
    let decoder = zstd::Decoder::new(reader)?;
    let limited = decoder.take(MAX_DECOMPRESSED_BYTES);
    let mut canvas: Canvas = serde_json::from_reader(limited).map_err(|e| {
        if e.classify() == serde_json::error::Category::Eof {
            anyhow::anyhow!(
                "decompressed data exceeds {} bytes",
                MAX_DECOMPRESSED_BYTES,
            )
        } else {
            e.into()
        }
    })?;

    if canvas.width == 0 || canvas.height == 0 {
        anyhow::bail!(
            "invalid canvas dimensions: {}x{}",
            canvas.width,
            canvas.height,
        );
    }

    let expected = (canvas.width as usize).saturating_mul(canvas.height as usize);
    for (i, layer) in canvas.pixels.iter().enumerate() {
        if layer.pixels.len() != expected {
            anyhow::bail!(
                "layer {i}: expected {expected} pixels, got {}",
                layer.pixels.len(),
            );
        }
    }

    canvas.dirty_rect.request_full_blend();
    Ok(canvas)
}

/// Deserialize a `Canvas` from zstd-compressed JSON bytes.
///
/// # Parameters
///
/// * `data` — Zstd-compressed JSON bytes produced by [`save_canvas_to_bytes`].
///
/// # Errors
///
/// Returns an error if zstd decompression or JSON deserialization fails,
/// if the decompressed data exceeds [`MAX_DECOMPRESSED_BYTES`],
/// or if the canvas has invalid dimensions or mismatched layer sizes.
pub fn load_canvas_from_bytes(data: &[u8]) -> anyhow::Result<Canvas> {
    read_canvas(data)
}

/// Serialize a `Canvas` into any `std::io::Write` by streaming JSON directly
/// into a multi-threaded zstd encoder.
///
/// Eliminates the intermediate `serde_json::to_vec` allocation — JSON is
/// serialized incrementally into the zstd compressor, which in turn writes
/// compressed frames into `writer`.
///
/// Uses multi-threaded zstd compression. This is the CPU-heavy part of saving
/// and should be called on a background thread.
///
/// # Parameters
///
/// * `canvas` — The canvas to serialize.
/// * `writer` — Destination writer (e.g. `File`, `Vec<u8>`).
///
/// # Errors
///
/// Returns an error if JSON serialization or zstd compression fails.
fn write_canvas(canvas: &Canvas, writer: impl Write) -> anyhow::Result<()> {
    let thread_count = std::thread::available_parallelism()
        .map(|count| count.get() as u32)
        .unwrap_or(1);
    let mut encoder = zstd::stream::Encoder::new(writer, COMPRESSION_LEVEL)?;
    encoder.multithread(thread_count)?;
    serde_json::to_writer(&mut encoder, canvas)?;
    encoder.finish()?;
    Ok(())
}

/// Serialize a `Canvas` to zstd-compressed JSON bytes without writing to disk.
///
/// Uses multi-threaded zstd compression. This is the CPU-heavy part of saving
/// and should be called on a background thread. Internally streams JSON
/// directly into the zstd encoder — no intermediate JSON `Vec<u8>`.
///
/// # Parameters
///
/// * `canvas` — The canvas to serialize.
///
/// # Errors
///
/// Returns an error if JSON serialization or zstd compression fails.
pub fn save_canvas_to_bytes(canvas: &Canvas) -> anyhow::Result<Vec<u8>> {
    let mut compressed = Vec::new();
    write_canvas(canvas, &mut compressed)?;
    Ok(compressed)
}

/// Serialize a `Canvas` directly to a file by streaming JSON through zstd
/// compression into a `File` — zero intermediate `Vec<u8>` allocations.
///
/// The file is created at `path`; the parent directory must exist.
///
/// # Parameters
///
/// * `canvas` — The canvas to serialize.
/// * `path` — Destination file path.
///
/// # Errors
///
/// Returns an error if the file cannot be created, or if JSON serialization
/// or zstd compression fails.
pub fn save_canvas_to_path(canvas: &Canvas, path: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    write_canvas(canvas, file)
}

/// Decompress and deserialize a `Canvas` from a file by streaming the file
/// through zstd decompression into `serde_json::from_reader` — no intermediate
/// `Vec<u8>` for the compressed file or the decompressed JSON.
///
/// # Parameters
///
/// * `path` — Path to a `.splattercanvas` file.
///
/// # Errors
///
/// Returns an error if the file cannot be read, or if zstd decompression or
/// JSON deserialization fails, or if the canvas has invalid dimensions.
pub fn load_canvas_from_path(path: &Path) -> anyhow::Result<Canvas> {
    let file = std::fs::File::open(path)?;
    read_canvas(file)
}

// ---------------------------------------------------------------------------
// XZ-compressed `.splatterarchive` format (export/import only, max compression)
// ---------------------------------------------------------------------------

/// Decompress and deserialize a `Canvas` from any `std::io::Read` by streaming
/// the xz decoder directly into `serde_json::from_reader`.
///
/// No intermediate decompression `Vec<u8>` is allocated — compressed data is
/// decompressed incrementally and parsed as JSON on the fly.
///
/// # Parameters
///
/// * `reader` — Source of xz-compressed JSON bytes (e.g. `File`, `&[u8]`).
///
/// # Errors
///
/// Returns an error if xz decompression or JSON deserialization fails,
/// if the decompressed data exceeds [`MAX_DECOMPRESSED_BYTES`],
/// or if the canvas has invalid dimensions or mismatched layer sizes.
fn read_canvas_xz(reader: impl Read) -> anyhow::Result<Canvas> {
    let decoder = xz2::read::XzDecoder::new(reader);
    let limited = decoder.take(MAX_DECOMPRESSED_BYTES);
    let mut canvas: Canvas = serde_json::from_reader(limited).map_err(|e| {
        if e.classify() == serde_json::error::Category::Eof {
            anyhow::anyhow!(
                "decompressed data exceeds {} bytes",
                MAX_DECOMPRESSED_BYTES,
            )
        } else {
            e.into()
        }
    })?;

    if canvas.width == 0 || canvas.height == 0 {
        anyhow::bail!(
            "invalid canvas dimensions: {}x{}",
            canvas.width,
            canvas.height,
        );
    }

    let expected = (canvas.width as usize).saturating_mul(canvas.height as usize);
    for (i, layer) in canvas.pixels.iter().enumerate() {
        if layer.pixels.len() != expected {
            anyhow::bail!(
                "layer {i}: expected {expected} pixels, got {}",
                layer.pixels.len(),
            );
        }
    }

    canvas.dirty_rect.request_full_blend();
    Ok(canvas)
}

/// Deserialize a `Canvas` from xz-compressed JSON bytes.
///
/// # Parameters
///
/// * `data` — Xz-compressed JSON bytes produced by [`save_canvas_to_bytes_xz`].
///
/// # Errors
///
/// Returns an error if xz decompression or JSON deserialization fails,
/// if the decompressed data exceeds [`MAX_DECOMPRESSED_BYTES`],
/// or if the canvas has invalid dimensions or mismatched layer sizes.
pub fn load_canvas_from_bytes_xz(data: &[u8]) -> anyhow::Result<Canvas> {
    read_canvas_xz(data)
}

/// Decompress and deserialize a `Canvas` from a file by streaming the file
/// through xz decompression into `serde_json::from_reader` — no intermediate
/// `Vec<u8>` for the compressed file or the decompressed JSON.
///
/// # Parameters
///
/// * `path` — Path to a `.splatterarchive` file.
///
/// # Errors
///
/// Returns an error if the file cannot be read, or if xz decompression or
/// JSON deserialization fails, or if the canvas has invalid dimensions.
pub fn load_canvas_from_path_xz(path: &Path) -> anyhow::Result<Canvas> {
    let file = std::fs::File::open(path)?;
    read_canvas_xz(file)
}

/// Serialize a `Canvas` into any `std::io::Write` by streaming JSON directly
/// into an xz encoder at maximum compression (preset 9).
///
/// No intermediate JSON `Vec<u8>` is allocated — JSON is serialized
/// incrementally into the xz compressor, which writes compressed frames
/// into `writer`.
///
/// # Parameters
///
/// * `canvas` — The canvas to serialize.
/// * `writer` — Destination writer (e.g. `File`, `Vec<u8>`).
///
/// # Errors
///
/// Returns an error if JSON serialization or xz compression fails.
fn write_canvas_xz(canvas: &Canvas, writer: impl Write) -> anyhow::Result<()> {
    let mut encoder = xz2::write::XzEncoder::new(writer, XZ_COMPRESSION_PRESET);
    serde_json::to_writer(&mut encoder, canvas)?;
    encoder.finish()?;
    Ok(())
}

/// Serialize a `Canvas` to xz-compressed JSON bytes without writing to disk.
///
/// This is the CPU-heavy part of exporting and should be called on a
/// background thread. Internally streams JSON directly into the xz encoder
/// — no intermediate JSON `Vec<u8>`.
///
/// # Parameters
///
/// * `canvas` — The canvas to serialize.
///
/// # Errors
///
/// Returns an error if JSON serialization or xz compression fails.
pub fn save_canvas_to_bytes_xz(canvas: &Canvas) -> anyhow::Result<Vec<u8>> {
    let mut compressed = Vec::new();
    write_canvas_xz(canvas, &mut compressed)?;
    Ok(compressed)
}

/// Serialize a `Canvas` directly to a file by streaming JSON through xz
/// compression into a `File` — zero intermediate `Vec<u8>` allocations.
///
/// The file is created at `path`; the parent directory must exist.
///
/// # Parameters
///
/// * `canvas` — The canvas to serialize.
/// * `path` — Destination file path.
///
/// # Errors
///
/// Returns an error if the file cannot be created, or if JSON serialization
/// or xz compression fails.
pub fn save_canvas_to_path_xz(canvas: &Canvas, path: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    write_canvas_xz(canvas, file)
}

/// Strategy for exporting a premultiplied RGBA buffer to an image file.
///
/// This trait decouples image encoding from [`FileIO`](crate::file_io::FileIO),
/// allowing the export implementation to be injected from the application
/// layer. The default implementation [`DefaultExportStrategy`] handles all
/// 13 supported image formats.
pub trait ExportStrategy: Send + Sync {
    /// Write `premultiplied_rgba` to the file at `path`.
    ///
    /// `premultiplied_rgba` is the already-blended premultiplied buffer
    /// (e.g. from `Canvas::output_rgba`).
    ///
    /// # Parameters
    ///
    /// * `premultiplied_rgba` — Flattened premultiplied RGBA pixel data.
    /// * `width` — Image width in pixels.
    /// * `height` — Image height in pixels.
    /// * `path` — Destination file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or the image encoder fails.
    fn export(
        &self,
        premultiplied_rgba: &[u8],
        width: u32,
        height: u32,
        path: &Path,
    ) -> anyhow::Result<()>;
}

/// Default export strategy supporting all 13 formats by detecting the
/// target format from the file path extension.
#[derive(Debug)]
pub struct DefaultExportStrategy;

impl ExportStrategy for DefaultExportStrategy {
    fn export(
        &self,
        premultiplied_rgba: &[u8],
        width: u32,
        height: u32,
        path: &Path,
    ) -> anyhow::Result<()> {
        let Some(format) = image::ImageFormat::from_extension(
            path.extension().and_then(|ext| ext.to_str()).unwrap_or(""),
        ) else {
            anyhow::bail!("Cannot determine image format from path: {}", path.display());
        };
        export_as_image(premultiplied_rgba, width, height, path, format)
    }
}

/// Export a flattened premultiplied RGBA buffer to an image file.
///
/// `premultiplied_rgba` is the already-blended premultiplied buffer
/// (e.g. from `Canvas::output_rgba`). For JPEG the alpha channel is blended
/// against a white background; for other formats straight alpha is preserved.
///
/// Supports 13 formats: `AVIF`, `PNG`, `JPEG`, `WebP`, `GIF`, `TIFF`, `TGA`, `ICO`, `PNM`,
/// `QOI`, `OpenEXR`, `HDR`, and `Farbfeld`.
///
/// # Parameters
///
/// * `premultiplied_rgba` — Flattened premultiplied RGBA pixel data.
/// * `width` — Image width in pixels.
/// * `height` — Image height in pixels.
/// * `path` — Destination file path (extension determines format).
/// * `format` — Target image format from the `image` crate.
///
/// # Errors
///
/// Returns an error if the file cannot be created or the image encoder fails.
pub fn export_as_image(
    premultiplied_rgba: &[u8],
    mut width: u32,
    mut height: u32,
    path: &Path,
    format: image::ImageFormat,
) -> anyhow::Result<()> {
    let total_pixels = (width * height) as usize;
    if total_pixels == 0 {
        anyhow::bail!("cannot export an image with zero pixels");
    }
    let premultiplied: &[Color32] = bytemuck::cast_slice(premultiplied_rgba);
    let is_jpeg = format == image::ImageFormat::Jpeg;

    let mut raw_output = vec![0u8; total_pixels * 4];

    raw_output
        .par_chunks_mut(4)
        .enumerate()
        .for_each(|(i, pixel)| {
            let c = premultiplied[i];
            let (fr, fg, fb, fa) = if is_jpeg {
                // Blend premultiplied RGBA against white background:
                // fully transparent (a=0,r=0) -> white (255,255,255)
                // For premultiplied over white: r' = r + (255 - a) (clamped)
                let inv_a = 255u8.wrapping_sub(c.a());
                (
                    c.r().saturating_add(inv_a),
                    c.g().saturating_add(inv_a),
                    c.b().saturating_add(inv_a),
                    255,
                )
            } else {
                let straight = unpremultiply(c);
                (straight.r(), straight.g(), straight.b(), straight.a())
            };
            pixel[0] = fr;
            pixel[1] = fg;
            pixel[2] = fb;
            pixel[3] = fa;
        });

    let file = std::fs::File::create(path)?;
    let writer = BufWriter::new(file);

    // JPEG: alpha was blended against white above. Strip alpha channel
    // directly from raw_output, skipping the RgbaImage intermediate.
    if format == image::ImageFormat::Jpeg {
        let rgb: Vec<u8> = raw_output
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        image::codecs::jpeg::JpegEncoder::new_with_quality(writer, JPEG_QUALITY).write_image(
            &rgb,
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )?;
        return Ok(());
    }

    // Build Rgb32F for HDR directly from raw_output, skipping RgbaImage.
    if format == image::ImageFormat::Hdr {
        let pixel_count = (width * height) as usize;
        let mut float_bytes = Vec::with_capacity(pixel_count * 3 * 4);
        for chunk in raw_output.chunks_exact(4) {
            let red = f32::from(chunk[0]) / F32_COLOR_MAX;
            let green = f32::from(chunk[1]) / F32_COLOR_MAX;
            let blue = f32::from(chunk[2]) / F32_COLOR_MAX;
            float_bytes.extend_from_slice(&red.to_ne_bytes());
            float_bytes.extend_from_slice(&green.to_ne_bytes());
            float_bytes.extend_from_slice(&blue.to_ne_bytes());
        }
        let encoder = image::codecs::hdr::HdrEncoder::new(writer);
        encoder.write_image(
            &float_bytes,
            width,
            height,
            image::ExtendedColorType::Rgb32F,
        )?;
        return Ok(());
    }

    // Build u16 RGBA for Farbfeld directly from raw_output, skipping RgbaImage.
    if format == image::ImageFormat::Farbfeld {
        let pixel_count = (width * height) as usize;
        let mut rgba16 = Vec::with_capacity(pixel_count * 4);
        for chunk in raw_output.chunks_exact(4) {
            rgba16.push(u16::from(chunk[0]));
            rgba16.push(u16::from(chunk[1]));
            rgba16.push(u16::from(chunk[2]));
            rgba16.push(u16::from(chunk[3]));
        }
        let rgba16_bytes: &[u8] = cast_slice(&rgba16);
        let encoder = image::codecs::farbfeld::FarbfeldEncoder::new(writer);
        encoder.write_image(
            rgba16_bytes,
            width,
            height,
            image::ExtendedColorType::Rgba16,
        )?;
        return Ok(());
    }

    // For non-RgbaImage formats (ICO, GIF) and the rest, create RgbaImage.
    let mut img = image::RgbaImage::from_raw(width, height, raw_output)
        .expect("dimensions match allocated pixel count");

    // ICO format has a 256×256 pixel limit — scale down if needed.
    if format == image::ImageFormat::Ico && (width > 256 || height > 256) {
        let scale = (256.0f64 / width.max(height) as f64).min(1.0);
        let new_width = (width as f64 * scale).round() as u32;
        let new_height = (height as f64 * scale).round() as u32;
        let new_width = new_width.max(1);
        let new_height = new_height.max(1);
        img = image::imageops::resize(
            &img,
            new_width,
            new_height,
            image::imageops::FilterType::Lanczos3,
        );
        let (w, h) = img.dimensions();
        width = w;
        height = h;
    }

    // GIF needs the `RgbaImage` directly, not raw bytes — handle it first.
    if format == image::ImageFormat::Gif {
        let frame = image::Frame::new(img);
        let mut encoder = image::codecs::gif::GifEncoder::new(writer);
        encoder.encode_frame(frame)?;
        return Ok(());
    }

    // Consume img into raw byte buffer for all other formats.
    let raw = img.into_raw();

    macro_rules! export_via {
        ($encoder:expr) => {
            $encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)
        };
        ($encoder:expr, $color:expr) => {
            $encoder.write_image(&raw, width, height, $color)
        };
    }

    match format {
        image::ImageFormat::Avif => export_via!(image::codecs::avif::AvifEncoder::new(writer))?,
        image::ImageFormat::Png => export_via!(image::codecs::png::PngEncoder::new(writer))?,

        image::ImageFormat::WebP => {
            export_via!(image::codecs::webp::WebPEncoder::new_lossless(writer))?
        }
        image::ImageFormat::Tiff => export_via!(image::codecs::tiff::TiffEncoder::new(writer))?,
        image::ImageFormat::Tga => export_via!(image::codecs::tga::TgaEncoder::new(writer))?,
        image::ImageFormat::Ico => export_via!(image::codecs::ico::IcoEncoder::new(writer))?,
        image::ImageFormat::Pnm => export_via!(image::codecs::pnm::PnmEncoder::new(writer))?,
        image::ImageFormat::Qoi => export_via!(image::codecs::qoi::QoiEncoder::new(writer))?,
        image::ImageFormat::OpenExr => {
            export_via!(image::codecs::openexr::OpenExrEncoder::new(writer))?
        }
        _ => {
            anyhow::bail!("Unsupported export format: {format:?}");
        }
    }

    Ok(())
}

/// Decode an image file into a single-layer Canvas.
///
/// Supports any format that the `image` crate can decode. The resulting
/// canvas has one layer with premultiplied-alpha RGBA pixels at the
/// image's native resolution.
///
/// # Parameters
///
/// * `path` — Path to the image file to import.
///
/// # Errors
///
/// Returns an error if the file cannot be read or the image format is
/// not recognized by the `image` crate.
pub fn import_image_as_canvas(path: &Path) -> anyhow::Result<Canvas> {
    let dynamic_image = image::open(path)?;
    let rgba = dynamic_image.to_rgba8();
    let (width_u32, height_u32) = rgba.dimensions();
    let pixel_count = (width_u32 as usize) * (height_u32 as usize);

    let mut pixels = Vec::with_capacity(pixel_count);
    for pixel in rgba.pixels() {
        let straight = Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]);
        pixels.push(premultiply(straight));
    }

    let mut dirty_rect = DirtyRectList::new();
    dirty_rect.request_full_blend();
    Ok(Canvas {
        pixels: vec![Layer { pixels, ..Default::default() }],
        height: height_u32,
        width: width_u32,
        output_rgba: Arc::new(Vec::new()),
        rendered_layers: None,
        dirty_rect,
    })
}
