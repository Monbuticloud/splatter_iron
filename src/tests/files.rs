//! Tests for serialization — canvas save/load round-trips, export, import.
//!
//! Validates that `save_canvas_to_path` / `load_canvas_from_path` produce
//! identical output, that image export produces valid headers, and that
//! zstd-compressed `.splattercanvas` files survive round-trip.

use eframe::egui::Color32;

use crate::canvas::Canvas;
use crate::canvas::DirtyRectList;
use crate::canvas::Layer;
use crate::files;

/// Build a 4×4 checkerboard canvas with alternating white/black opaque pixels.
fn checkerboard_4x4() -> Canvas {
    let mut pixels = Vec::with_capacity(16);
    for y in 0..4u8 {
        for x in 0..4u8 {
            if (x + y) % 2 == 0 {
                pixels.push(Color32::from_rgba_premultiplied(255, 255, 255, 255));
            } else {
                pixels.push(Color32::from_rgba_premultiplied(0, 0, 0, 255));
            }
        }
    }
    Canvas {
        pixels: vec![Layer { pixels, ..Default::default() }],
        height: 4,
        width: 4,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    }
}

/// Save and load a checkerboard canvas; all pixels should be identical.
#[test]
fn save_load_roundtrip_identical_pixels() {
    let original = checkerboard_4x4();
    let data = files::save_canvas_to_bytes(&original).expect("save to bytes");
    let loaded = files::load_canvas_from_bytes(&data).expect("load from bytes");
    assert_eq!(loaded.width, original.width);
    assert_eq!(loaded.height, original.height);
    assert_eq!(loaded.pixels.len(), original.pixels.len());
    for (i, (orig_p, load_p)) in original.pixels[0]
        .pixels
        .iter()
        .zip(loaded.pixels[0].pixels.iter())
        .enumerate()
    {
        assert_eq!(orig_p, load_p, "pixel {i} differs");
    }
}

/// Save and load a 2-layer canvas; both layers should survive the roundtrip.
#[test]
fn save_load_roundtrip_multi_layer() {
    let canvas = Canvas {
        pixels: vec![
            Layer {
                pixels: vec![Color32::from_rgba_premultiplied(255, 0, 0, 255); 9],
                ..Default::default()
            },
            Layer {
                pixels: vec![Color32::from_rgba_premultiplied(0, 0, 255, 255); 9],
                ..Default::default()
            },
        ],
        height: 3,
        width: 3,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    };
    let data = files::save_canvas_to_bytes(&canvas).expect("save");
    let loaded = files::load_canvas_from_bytes(&data).expect("load");
    assert_eq!(loaded.pixels.len(), 2);
    for (i, (a, b)) in canvas.pixels[0]
        .pixels
        .iter()
        .zip(loaded.pixels[0].pixels.iter())
        .enumerate()
    {
        assert_eq!(a, b, "layer 0 pixel {i}");
    }
    for (i, (a, b)) in canvas.pixels[1]
        .pixels
        .iter()
        .zip(loaded.pixels[1].pixels.iter())
        .enumerate()
    {
        assert_eq!(a, b, "layer 1 pixel {i}");
    }
}

/// Save and load a canvas with transparent and semi-transparent pixels.
#[test]
fn save_load_roundtrip_transparent() {
    let canvas = Canvas {
        pixels: vec![Layer {
            pixels: vec![
                Color32::TRANSPARENT,
                Color32::from_rgba_premultiplied(255, 0, 0, 128),
                Color32::TRANSPARENT,
            ],
            ..Default::default()
        }],
        height: 1,
        width: 3,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    };
    let data = files::save_canvas_to_bytes(&canvas).expect("save");
    let loaded = files::load_canvas_from_bytes(&data).expect("load");
    assert_eq!(loaded.pixels[0].pixels.len(), 3);
}

/// Export a checkerboard canvas to PNG and re-import; pixels should match.
#[test]
fn export_png_roundtrip() {
    let canvas = checkerboard_4x4();
    let mut rgba = Vec::with_capacity(16 * 4);
    for pixel in &canvas.pixels[0].pixels {
        rgba.extend_from_slice(&pixel.to_array());
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.png");
    files::export_as_image(&rgba, 4, 4, &path, image::ImageFormat::Png).expect("export PNG");
    let imported = files::import_image_as_canvas(&path).expect("import PNG");
    assert_eq!(imported.width, 4);
    assert_eq!(imported.height, 4);
    assert_eq!(imported.pixels.len(), 1);
    for (i, (orig, imp)) in canvas.pixels[0]
        .pixels
        .iter()
        .zip(imported.pixels[0].pixels.iter())
        .enumerate()
    {
        assert_eq!(orig, imp, "opaque pixel {i} mismatch after PNG roundtrip");
    }
}

/// Export a JPEG file and verify it exists and has content.
#[test]
fn export_jpeg_creates_file() {
    let mut rgba = Vec::with_capacity(4 * 4);
    for _ in 0..4 {
        rgba.extend_from_slice(&[255, 128, 64, 255]);
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.jpg");
    files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::Jpeg).expect("export JPEG");
    assert!(path.exists());
    let metadata = std::fs::metadata(&path).expect("metadata");
    assert!(metadata.len() > 0, "JPEG file should have content");
}

/// Export a semi-transparent PNG and verify it re-imports correctly.
#[test]
fn export_png_semi_transparent() {
    let mut rgba = Vec::with_capacity(4 * 4);
    rgba.extend_from_slice(&[255, 255, 255, 255]);
    rgba.extend_from_slice(&[128, 0, 0, 128]);
    rgba.extend_from_slice(&[0, 0, 0, 0]);
    rgba.extend_from_slice(&[0, 0, 255, 255]);
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.png");
    files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::Png).expect("export PNG");
    let imported = files::import_image_as_canvas(&path).expect("import PNG");
    assert_eq!(imported.width, 2);
    assert_eq!(imported.height, 2);
    assert_eq!(imported.pixels.len(), 1);
}

/// Loading non-zstd data should return an error.
#[test]
fn invalid_data_returns_error() {
    let bad = b"this is not zstd-compressed json";
    let result = files::load_canvas_from_bytes(bad);
    assert!(result.is_err());
}

/// Loading empty data should return an error.
#[test]
fn empty_data_returns_error() {
    let result = files::load_canvas_from_bytes(&[]);
    assert!(result.is_err());
}

// --- Export additional formats ---

/// Export a checkerboard canvas to WebP and verify it exists and has content.
#[test]
fn export_webp_creates_file() {
    let mut rgba = Vec::with_capacity(4 * 4);
    for _ in 0..4 {
        rgba.extend_from_slice(&[255, 128, 64, 255]);
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.webp");
    files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::WebP).expect("export WebP");
    assert!(path.exists());
    let metadata = std::fs::metadata(&path).expect("metadata");
    assert!(metadata.len() > 0, "WebP file should have content");
}

/// Export a canvas to GIF and verify it exists and has content.
#[test]
fn export_gif_creates_file() {
    let mut rgba = Vec::with_capacity(4 * 4);
    for _ in 0..4 {
        rgba.extend_from_slice(&[255, 128, 64, 255]);
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.gif");
    files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::Gif).expect("export GIF");
    assert!(path.exists());
    let metadata = std::fs::metadata(&path).expect("metadata");
    assert!(metadata.len() > 0, "GIF file should have content");
}

/// Export a canvas to TIFF and verify it exists and has content.
#[test]
fn export_tiff_creates_file() {
    let mut rgba = Vec::with_capacity(4 * 4);
    for _ in 0..4 {
        rgba.extend_from_slice(&[255, 128, 64, 255]);
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.tiff");
    files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::Tiff).expect("export TIFF");
    assert!(path.exists());
    let metadata = std::fs::metadata(&path).expect("metadata");
    assert!(metadata.len() > 0, "TIFF file should have content");
}

// --- save_bytes_to_file / load_bytes_from_file ---

/// `save_bytes_to_file` and `load_bytes_from_file` should round-trip correctly.
#[test]
fn save_bytes_to_file_roundtrip() {
    let data = b"hello, splatter canvas test data!";
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.bin");
    files::save_bytes_to_file(data, &path).expect("save bytes");
    let loaded = files::load_bytes_from_file(&path).expect("load bytes");
    assert_eq!(loaded, data, "bytes round-trip");
}

// --- Remaining export format tests (AVIF, TGA, ICO, PNM, QOI, EXR, HDR, Farbfeld) ---

/// Helper: export checkerboard data to a given format, verify file exists and has content.
fn check_export(format: image::ImageFormat, extension: &str) {
    let mut rgba = Vec::with_capacity(16 * 4);
    for y in 0..4u8 {
        for x in 0..4u8 {
            if (x + y) % 2 == 0 {
                rgba.extend_from_slice(&[255, 255, 255, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join(format!("test.{extension}"));
    files::export_as_image(&rgba, 4, 4, &path, format).expect(&format!("export {format:?}"));
    assert!(path.exists(), "file should exist for {format:?}");
    let metadata = std::fs::metadata(&path).expect("metadata");
    assert!(
        metadata.len() > 0,
        "file should have content for {format:?}"
    );
}

#[test]
fn export_avif_creates_file() {
    check_export(image::ImageFormat::Avif, "avif");
}

#[test]
fn export_tga_creates_file() {
    check_export(image::ImageFormat::Tga, "tga");
}

#[test]
fn export_ico_creates_file() {
    check_export(image::ImageFormat::Ico, "ico");
}

#[test]
fn export_pnm_creates_file() {
    check_export(image::ImageFormat::Pnm, "pnm");
}

#[test]
fn export_qoi_creates_file() {
    check_export(image::ImageFormat::Qoi, "qoi");
}

/// OpenEXR does not support Rgba8 directly, so export is expected to fail.
#[test]
fn export_exr_unsupported_color_type() {
    let rgba = vec![255u8; 16];
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.exr");
    let result = files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::OpenExr);
    assert!(result.is_err(), "OpenEXR over Rgba8 should error");
}

#[test]
fn export_hdr_creates_file() {
    check_export(image::ImageFormat::Hdr, "hdr");
}

#[test]
fn export_farbfeld_creates_file() {
    check_export(image::ImageFormat::Farbfeld, "ff");
}

/// Export with an unsupported format should return an error.
#[test]
fn export_unsupported_format_errors() {
    let rgba = vec![255u8; 16];
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.bmp");
    let result = files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::Bmp);
    assert!(result.is_err(), "unsupported format should error");
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("Unsupported"),
        "error should mention unsupported: {err}"
    );
}

/// Export with zero-width image should fail gracefully.
#[test]
fn export_zero_width_fails() {
    let rgba = vec![255u8; 0];
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.png");
    let result = files::export_as_image(&rgba, 0, 1, &path, image::ImageFormat::Png);
    assert!(result.is_err());
}

/// Import a JPEG image as a canvas roundtrip.
#[test]
fn import_jpeg_as_canvas() {
    // Create a 2x2 JPEG file, then import it
    let mut rgba = Vec::with_capacity(4 * 4);
    rgba.extend_from_slice(&[255, 0, 0, 255]);
    rgba.extend_from_slice(&[0, 255, 0, 255]);
    rgba.extend_from_slice(&[0, 0, 255, 255]);
    rgba.extend_from_slice(&[128, 128, 128, 255]);

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test_import.jpg");
    files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::Jpeg)
        .expect("export JPEG for import test");

    let imported = files::import_image_as_canvas(&path).expect("import JPEG");
    assert_eq!(imported.width, 2);
    assert_eq!(imported.height, 2);
    assert_eq!(imported.pixels.len(), 1);
    // JPEG is lossy, so we can't compare exact pixels — just check that it loaded
    assert!(imported.dirty_rect.needs_reblend());
}

/// Decompress corrupted zstd data should return an error.
#[test]
fn decompress_corrupted_data_errors() {
    let corrupted = b"this is not valid zstd compressed data at all!!!";
    let result = files::load_canvas_from_bytes(corrupted);
    assert!(result.is_err());
}

/// Decompress slightly corrupt zstd frame (valid header, bad content).
#[test]
fn decompress_partially_corrupted_zstd_errors() {
    // Build a minimal but invalid zstd frame
    let mut buf = Vec::new();
    // Magic number for zstd
    buf.extend_from_slice(&[0x28, 0xB5, 0x2F, 0xFD]);
    // Frame header (1 byte: content_size_flag=0, single_segment=1,
    //               reserved=0, window_log=0)
    buf.push(0x00);
    // Add some garbage as body
    buf.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
    let result = files::load_canvas_from_bytes(&buf);
    assert!(result.is_err());
}

/// Export and re-import TIFF roundtrip.
#[test]
fn export_tiff_roundtrip() {
    let mut rgba = Vec::with_capacity(4 * 4);
    for _ in 0..4 {
        rgba.extend_from_slice(&[128, 64, 32, 255]);
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("roundtrip.tiff");
    files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::Tiff)
        .expect("export TIFF for roundtrip");
    let imported = files::import_image_as_canvas(&path).expect("import TIFF");
    assert_eq!(imported.width, 2);
    assert_eq!(imported.height, 2);
}

/// Loading a canvas with zero width should be rejected.
#[test]
fn load_zero_width_canvas_rejected() {
    let canvas = Canvas {
        pixels: vec![Layer { pixels: vec![Color32::TRANSPARENT; 1], ..Default::default() }],
        height: 1,
        width: 0,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    };
    let data = files::save_canvas_to_bytes(&canvas).expect("save");
    assert!(
        files::load_canvas_from_bytes(&data).is_err(),
        "zero-width canvas should be rejected"
    );
}

/// Loading a canvas with zero height should be rejected.
#[test]
fn load_zero_height_canvas_rejected() {
    let canvas = Canvas {
        pixels: vec![Layer { pixels: vec![], ..Default::default() }],
        height: 0,
        width: 1,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    };
    let data = files::save_canvas_to_bytes(&canvas).expect("save");
    assert!(
        files::load_canvas_from_bytes(&data).is_err(),
        "zero-height canvas should be rejected"
    );
}

/// Loading a canvas with a layer that has the wrong pixel count should be rejected.
#[test]
fn load_wrong_layer_size_rejected() {
    let canvas = Canvas {
        pixels: vec![
            Layer {
                pixels: vec![Color32::TRANSPARENT; 4],
                ..Default::default()
            },
            Layer {
                // Second layer has wrong size (2 instead of 4)
                pixels: vec![Color32::TRANSPARENT; 2],
                ..Default::default()
            },
        ],
        height: 2,
        width: 2,
        output_rgba: Vec::new(),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    };
    let data = files::save_canvas_to_bytes(&canvas).expect("save");
    assert!(
        files::load_canvas_from_bytes(&data).is_err(),
        "mismatched layer pixel count should be rejected"
    );
}
