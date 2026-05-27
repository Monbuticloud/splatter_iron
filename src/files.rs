use std::io::BufWriter;
use std::path::Path;
use bytemuck::cast_slice;
use eframe::egui::Color32;
use image::ImageEncoder;

use crate::canvas::{ Canvas, Layer };
use crate::pixel::{ premultiply, unpremultiply, F32_COLOR_MAX };

const COMPRESSION_LEVEL: i32 = 10;
const JPEG_QUALITY: u8 = 95;

/// Read the raw bytes of a file from disk.
///
/// # Errors
///
/// Returns `std::io::Error` if the file cannot be read (e.g. not found, permission denied).
pub fn load_data_from_file(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(path)
}

/// Deserialize a `Canvas` from zstd-compressed JSON bytes.
///
/// # Errors
///
/// Returns an error if zstd decompression or JSON deserialization fails.
pub fn load_app_from_data(data: &[u8]) -> anyhow::Result<Canvas> {
    let decompressed = zstd::decode_all(data)?;
    let canvas = serde_json::from_slice(&decompressed)?;
    Ok(canvas)
}

/// Serialize a `Canvas` to zstd-compressed JSON bytes without writing to disk.
///
/// Uses multi-threaded zstd compression. This is the CPU-heavy part of saving
/// and should be called on a background thread.
///
/// # Errors
///
/// Returns an error if JSON serialisation or zstd compression fails.
pub fn save_canvas_to_bytes(canvas: &Canvas) -> anyhow::Result<Vec<u8>> {
    use std::io::Write;
    let json = serde_json::to_vec(canvas)?;
    let n_threads = std::thread
        ::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);
    let mut encoder = zstd::stream::Encoder::new(Vec::new(), COMPRESSION_LEVEL)?;
    encoder.multithread(n_threads)?;
    encoder.write_all(&json)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}

/// Write pre-serialized bytes to a file.
///
/// This is a pure I/O operation — serialization should be done beforehand
/// with [`save_canvas_to_bytes`].
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn save_bytes_to_file(data: &[u8], path: &Path) -> anyhow::Result<()> {
    std::fs::write(path, data)?;
    Ok(())
}

/// Export a flattened premultiplied RGBA buffer to an image file.
///
/// `premultiplied_rgba` is the already-blended premultiplied buffer
/// (e.g. from `Canvas::output_rgba`). For JPEG the alpha channel is blended
/// against a white background; for other formats straight alpha is preserved.
///
/// Supports 17 formats: `AVIF`, `PNG`, `JPEG`, `WebP`, `GIF`, `TIFF`, `TGA`, `ICO`, `PNM`,
/// `QOI`, `OpenEXR`, `HDR`, and `Farbfeld`.
///
/// # Errors
///
/// Returns an error if the file cannot be created or the image encoder fails.
pub fn export_as_image(
    premultiplied_rgba: &[u8],
    width: u32,
    height: u32,
    path: &Path,
    format: image::ImageFormat
) -> anyhow::Result<()> {
    let mut img = image::RgbaImage::new(width, height);
    let is_jpeg = format == image::ImageFormat::Jpeg;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) as usize) * 4;
            let r = premultiplied_rgba[idx];
            let g = premultiplied_rgba[idx + 1];
            let b = premultiplied_rgba[idx + 2];
            let a = premultiplied_rgba[idx + 3];

            let (fr, fg, fb, fa) = if is_jpeg {
                // Blend premultiplied RGBA against white background:
                // fully transparent (a=0,r=0) -> white (255,255,255)
                // For premultiplied over white: r' = r + (255 - a) (clamped)
                let inv = (255u8).wrapping_sub(a); // 255 - a
                (r.saturating_add(inv), g.saturating_add(inv), b.saturating_add(inv), 255u8)
            } else {
                let pm = Color32::from_rgba_premultiplied(r, g, b, a);
                let straight = unpremultiply(pm);
                (straight.r(), straight.g(), straight.b(), straight.a())
            };

            let rgba = image::Rgba([fr, fg, fb, fa]);
            img.put_pixel(x, y, rgba);
        }
    }

    let file = std::fs::File::create(path)?;
    let writer = BufWriter::new(file);

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
        image::codecs::jpeg::JpegEncoder::new_with_quality(writer, JPEG_QUALITY)
            .write_image(&rgb, width, height, image::ExtendedColorType::Rgb8)?;
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

        image::ImageFormat::WebP => export_via!(image::codecs::webp::WebPEncoder::new_lossless(writer))?,
        image::ImageFormat::Tiff => export_via!(image::codecs::tiff::TiffEncoder::new(writer))?,
        image::ImageFormat::Tga => export_via!(image::codecs::tga::TgaEncoder::new(writer))?,
        image::ImageFormat::Ico => export_via!(image::codecs::ico::IcoEncoder::new(writer))?,
        image::ImageFormat::Pnm => export_via!(image::codecs::pnm::PnmEncoder::new(writer))?,
        image::ImageFormat::Qoi => export_via!(image::codecs::qoi::QoiEncoder::new(writer))?,
        image::ImageFormat::OpenExr => export_via!(image::codecs::openexr::OpenExrEncoder::new(writer))?,
        image::ImageFormat::Hdr => {
            // Build Rgb32F image from the straight RGBA buffer.
            // HDR stores linear float RGB (alpha is ignored).
            // Build u8 buffer directly from f32 values (no unsafe needed).
            let pixel_count = (width * height) as usize;
            let mut float_bytes = Vec::with_capacity(pixel_count * 3 * 4);
            for chunk in raw.chunks_exact(4) {
                let r = f32::from(chunk[0]) / F32_COLOR_MAX;
                let g = f32::from(chunk[1]) / F32_COLOR_MAX;
                let b = f32::from(chunk[2]) / F32_COLOR_MAX;
                float_bytes.extend_from_slice(&r.to_ne_bytes());
                float_bytes.extend_from_slice(&g.to_ne_bytes());
                float_bytes.extend_from_slice(&b.to_ne_bytes());
            }
            let encoder = image::codecs::hdr::HdrEncoder::new(writer);
            encoder.write_image(&float_bytes, width, height, image::ExtendedColorType::Rgb32F)?;
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
            encoder.write_image(rgba16_bytes, width, height, image::ExtendedColorType::Rgba16)?;
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
/// # Errors
///
/// Returns an error if the file cannot be read or the image format is
/// not recognised by the `image` crate.
pub fn import_image_as_canvas(path: &Path) -> anyhow::Result<Canvas> {
    let dyn_img = image::open(path)?;
    let rgba = dyn_img.to_rgba8();
    let (width_u32, height_u32) = rgba.dimensions();
    let pixel_count = (width_u32 as usize) * (height_u32 as usize);

    let mut pixels = Vec::with_capacity(pixel_count);
    for pixel in rgba.pixels() {
        let straight = Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]);
        pixels.push(premultiply(straight));
    }

    Ok(Canvas {
        pixels: vec![Layer { pixels }],
        height: height_u32,
        width: width_u32,
        output_rgba: Vec::new(),
        rendered_layers: None,
        render_next_frame: true,
    })
}
