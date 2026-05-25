use std::io::{BufWriter, Write};
use std::path::Path;
use zstd;

use eframe::egui::Color32;
use image::ImageEncoder;

use crate::canvas::{Canvas, Layer};
use crate::pixel::{premultiply, unpremultiply};

const COMPRESSION_LEVEL: i32 = 10;

pub fn get_save_data(canvas: &Canvas) -> anyhow::Result<Vec<u8>> {
    let json = serde_json::to_vec(canvas)?;
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);
    let mut encoder = zstd::stream::Encoder::new(Vec::new(), COMPRESSION_LEVEL)?;
    encoder.multithread(n_threads)?;
    encoder.write_all(&json)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}

pub fn save_data_to_file(data: &[u8], path: &Path) -> Result<(), std::io::Error> {
    std::fs::write(path, data)?;
    Ok(())
}

pub fn load_data_from_file(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(path)
}

pub fn load_app_from_data(data: &[u8]) -> anyhow::Result<Canvas> {
    let decompressed = zstd::decode_all(data)?;
    let canvas = serde_json::from_slice(&decompressed)?;
    Ok(canvas)
}

pub fn save_canvas(app: &crate::app::MyApp) -> anyhow::Result<()> {
    let data = get_save_data(&app.canvas)?;
    save_data_to_file(&data, Path::new(&app.savefile_path))?;
    Ok(())
}

/// Export the flattened premultiplied RGBA buffer to an image file.
///
/// `premultiplied_rgba` is the already-blended premultiplied RGBA u8 buffer
/// (e.g. from `Canvas::output_rgba`). For JPEG the image is blended against
/// white; for other formats the alpha channel is preserved.
pub fn export_as_image(
    premultiplied_rgba: &[u8],
    width: u32,
    height: u32,
    path: &Path,
    format: image::ImageFormat,
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
                (
                    r.saturating_add(inv),
                    g.saturating_add(inv),
                    b.saturating_add(inv),
                    255u8,
                )
            } else {
                let pm = Color32::from_rgba_premultiplied(r, g, b, a);
                let straight = unpremultiply(pm);
                (straight.r(), straight.g(), straight.b(), straight.a())
            };

            #[allow(clippy::tuple_array_conversions)]
            img.put_pixel(x, y, image::Rgba([fr, fg, fb, fa]));
        }
    }

    let file = std::fs::File::create(path)?;
    let writer = BufWriter::new(file);

    // For formats that need float conversion (EXR, HDR), build a separate buffer.
    // `img` is moved into the GIF branch; for others we clone raw before moving.
    let raw = img.clone().into_raw();

    match format {
        image::ImageFormat::Avif => {
            let encoder = image::codecs::avif::AvifEncoder::new(writer);
            encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)?;
        }
        image::ImageFormat::Png => {
            let encoder = image::codecs::png::PngEncoder::new(writer);
            encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)?;
        }
        image::ImageFormat::Jpeg => {
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, 95);
            encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgb8)?;
        }
        image::ImageFormat::WebP => {
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(writer);
            encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)?;
        }
        image::ImageFormat::Gif => {
            let frame = image::Frame::new(img);
            let mut encoder = image::codecs::gif::GifEncoder::new(writer);
            encoder.encode_frame(frame)?;
        }
        image::ImageFormat::Tiff => {
            let encoder = image::codecs::tiff::TiffEncoder::new(writer);
            encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)?;
        }
        image::ImageFormat::Tga => {
            let encoder = image::codecs::tga::TgaEncoder::new(writer);
            encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)?;
        }
        image::ImageFormat::Ico => {
            let encoder = image::codecs::ico::IcoEncoder::new(writer);
            encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)?;
        }
        image::ImageFormat::Pnm => {
            let encoder = image::codecs::pnm::PnmEncoder::new(writer);
            encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)?;
        }
        image::ImageFormat::Qoi => {
            let encoder = image::codecs::qoi::QoiEncoder::new(writer);
            encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)?;
        }
        image::ImageFormat::OpenExr => {
            let encoder = image::codecs::openexr::OpenExrEncoder::new(writer);
            encoder.write_image(&raw, width, height, image::ExtendedColorType::Rgba8)?;
        }
        image::ImageFormat::Hdr => {
            // Build Rgb32F image from the straight RGBA buffer.
            // HDR stores linear float RGB (alpha is ignored).
            let pixel_count = (width * height) as usize;
            let mut float_pixels = Vec::with_capacity(pixel_count * 3);
            for chunk in raw.chunks_exact(4) {
                let r = f32::from(chunk[0]) / 255.0;
                let g = f32::from(chunk[1]) / 255.0;
                let b = f32::from(chunk[2]) / 255.0;
                float_pixels.push(r);
                float_pixels.push(g);
                float_pixels.push(b);
            }
            let encoder = image::codecs::hdr::HdrEncoder::new(writer);
            // Convert Vec<f32> to &[u8] by transmuting (safe because f32 is 4 bytes)
            let float_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    float_pixels.as_ptr() as *const u8,
                    float_pixels.len() * std::mem::size_of::<f32>()
                )
            };
            encoder.write_image(
                float_bytes,
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
            // Use the native-endian encode() method; it converts to BE internally.
            let rgba16_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    rgba16.as_ptr() as *const u8,
                    rgba16.len() * std::mem::size_of::<u16>()
                )
            };
            let encoder = image::codecs::farbfeld::FarbfeldEncoder::new(writer);
            encoder.write_image(rgba16_bytes, width, height, image::ExtendedColorType::Rgba16)?;
        }
        _ => {
            anyhow::bail!("Unsupported export format: {format:?}");
        }
    }

    Ok(())
}

/// Import an image file as a single-layer Canvas.
///
/// Decodes any supported image format into premultiplied RGBA pixels
/// and returns a new Canvas with one layer at image resolution.
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