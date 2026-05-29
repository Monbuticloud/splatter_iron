//! Serialization and deserialization of canvas data (zstd-compressed JSON,
//! `.splattercanvas` format), plus image export to 13 formats.

use std::io::BufWriter;
use std::io::Read;
use std::path::Path;

use bytemuck::cast_slice;
use eframe::egui::Color32;
use image::ImageEncoder;

use crate::canvas::Canvas;
use crate::canvas::DirtyRectList;
use crate::canvas::Layer;
use crate::pixel::F32_COLOR_MAX;
use crate::pixel::premultiply;
use crate::pixel::unpremultiply;

const COMPRESSION_LEVEL: i32 = 10;
const MAX_DECOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;
const JPEG_QUALITY: u8 = 100;

/// Read the raw bytes of a file from disk.
///
/// # Parameters
///
/// * `path` — Path to the file to read.
///
/// # Errors
///
/// Returns `std::io::Error` if the file cannot be read (e.g. not found, permission denied).
pub fn load_bytes_from_file(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(path)
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
/// or if the decompressed data exceeds [`MAX_DECOMPRESSED_BYTES`].
pub fn load_canvas_from_bytes(data: &[u8]) -> anyhow::Result<Canvas> {
    let mut decompressed = Vec::new();
    let mut decoder = zstd::Decoder::new(data)?;
    let mut buf = [0u8; 8192];
    loop {
        let n = decoder.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if decompressed.len() + n > MAX_DECOMPRESSED_BYTES as usize {
            anyhow::bail!(
                "decompressed data exceeds {} bytes",
                MAX_DECOMPRESSED_BYTES,
            );
        }
        decompressed.extend_from_slice(&buf[..n]);
    }
    let mut canvas: Canvas = serde_json::from_slice(&decompressed)?;

    // Validate dimensions
    if canvas.width == 0 || canvas.height == 0 {
        anyhow::bail!(
            "invalid canvas dimensions: {}x{}",
            canvas.width,
            canvas.height,
        );
    }

    // Validate each layer has the correct number of pixels
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

/// Serialize a `Canvas` to zstd-compressed JSON bytes without writing to disk.
///
/// Uses multi-threaded zstd compression. This is the CPU-heavy part of saving
/// and should be called on a background thread.
///
/// # Parameters
///
/// * `canvas` — The canvas to serialize.
///
/// # Errors
///
/// Returns an error if JSON serialization or zstd compression fails.
pub fn save_canvas_to_bytes(canvas: &Canvas) -> anyhow::Result<Vec<u8>> {
    use std::io::Write;
    let json = serde_json::to_vec(canvas)?;
    let thread_count = std::thread::available_parallelism()
        .map(|count| count.get() as u32)
        .unwrap_or(1);
    let mut encoder = zstd::stream::Encoder::new(Vec::new(), COMPRESSION_LEVEL)?;
    encoder.multithread(thread_count)?;
    encoder.write_all(&json)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}

/// Write pre-serialized bytes to a file.
///
/// This is a pure I/O operation — serialization should be done beforehand
/// with [`save_canvas_to_bytes`].
///
/// # Parameters
///
/// * `data` — Pre-serialized bytes to write.
/// * `path` — Destination file path.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn save_bytes_to_file(data: &[u8], path: &Path) -> anyhow::Result<()> {
    std::fs::write(path, data)?;
    Ok(())
}

/// Strategy for exporting a premultiplied RGBA buffer to an image file.
///
/// This trait decouples image encoding from [`FileIO`](crate::file_io::FileIO),
/// allowing the export implementation to be injected from the application
/// layer. The default implementation [`DefaultExportStrategy`] handles all
/// 13 supported image formats.
pub trait ExportStrategy {
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
    let mut img = image::RgbaImage::new(width, height);
    let is_jpeg = format == image::ImageFormat::Jpeg;

    for y in 0..height {
        for x in 0..width {
            let pixel_index = ((y * width + x) as usize) * 4;
            let red = premultiplied_rgba[pixel_index];
            let green = premultiplied_rgba[pixel_index + 1];
            let blue = premultiplied_rgba[pixel_index + 2];
            let alpha = premultiplied_rgba[pixel_index + 3];

            let (final_red, final_green, final_blue, final_alpha) = if is_jpeg {
                // Blend premultiplied RGBA against white background:
                // fully transparent (a=0,r=0) -> white (255,255,255)
                // For premultiplied over white: r' = r + (255 - a) (clamped)
                let inverse_alpha = (255u8).wrapping_sub(alpha); // 255 - a
                (
                    red.saturating_add(inverse_alpha),
                    green.saturating_add(inverse_alpha),
                    blue.saturating_add(inverse_alpha),
                    255u8,
                )
            } else {
                let premultiplied = Color32::from_rgba_premultiplied(red, green, blue, alpha);
                let straight = unpremultiply(premultiplied);
                (straight.r(), straight.g(), straight.b(), straight.a())
            };

            let rgba = image::Rgba([final_red, final_green, final_blue, final_alpha]);
            img.put_pixel(x, y, rgba);
        }
    }

    let file = std::fs::File::create(path)?;
    let writer = BufWriter::new(file);

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

    // JPEG requires RGB8 (3 bytes/pixel). Alpha was already blended
    // against white in the loop above, so strip the alpha channel.
    if format == image::ImageFormat::Jpeg {
        let rgb: Vec<u8> = img.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
        image::codecs::jpeg::JpegEncoder::new_with_quality(writer, JPEG_QUALITY).write_image(
            &rgb,
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )?;
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
        image::ImageFormat::Hdr => {
            // Build Rgb32F image from the straight RGBA buffer.
            // HDR stores linear float RGB (alpha is ignored).
            // Build u8 buffer directly from f32 values (no unsafe needed).
            let pixel_count = (width * height) as usize;
            let mut float_bytes = Vec::with_capacity(pixel_count * 3 * 4);
            for chunk in raw.chunks_exact(4) {
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
        }
        image::ImageFormat::Farbfeld => {
            // Farbfeld requires u16 RGBA (8 bytes/pixel), native endian.
            let pixel_count = (width * height) as usize;
            let mut rgba16 = Vec::with_capacity(pixel_count * 4);
            for chunk in raw.chunks_exact(4) {
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
        pixels: vec![Layer { pixels }],
        height: height_u32,
        width: width_u32,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect,
    })
}
