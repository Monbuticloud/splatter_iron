use eframe::egui::Color32;

use crate::canvas::{Canvas, Layer};
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
        pixels: vec![Layer { pixels }],
        height: 4,
        width: 4,
        output_rgba: Vec::new(),
        rendered_layers: None,
        render_next_frame: false,
    }
}

/// Save and load a checkerboard canvas; all pixels should be identical.
#[test]
fn save_load_roundtrip_identical_pixels() {
    let original = checkerboard_4x4();
    let data = files::save_canvas_to_bytes(&original).expect("save to bytes");
    let loaded = files::load_app_from_data(&data).expect("load from bytes");
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
    let mut canvas = Canvas {
        pixels: vec![
            Layer { pixels: vec![Color32::from_rgba_premultiplied(255, 0, 0, 255); 9] },
            Layer { pixels: vec![Color32::from_rgba_premultiplied(0, 0, 255, 255); 9] },
        ],
        height: 3,
        width: 3,
        output_rgba: Vec::new(),
        rendered_layers: None,
        render_next_frame: false,
    };
    let data = files::save_canvas_to_bytes(&canvas).expect("save");
    let loaded = files::load_app_from_data(&data).expect("load");
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
        }],
        height: 1,
        width: 3,
        output_rgba: Vec::new(),
        rendered_layers: None,
        render_next_frame: false,
    };
    let data = files::save_canvas_to_bytes(&canvas).expect("save");
    let loaded = files::load_app_from_data(&data).expect("load");
    assert_eq!(loaded.pixels[0].pixels.len(), 3);
}

/// Export a checkerboard canvas to PNG and re-import; pixels should match.
#[test]
fn export_png_roundtrip() {
    let canvas = checkerboard_4x4();
    let mut rgba = Vec::with_capacity(16 * 4);
    for p in &canvas.pixels[0].pixels {
        rgba.extend_from_slice(&p.to_array());
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.png");
    files::export_as_image(&rgba, 4, 4, &path, image::ImageFormat::Png)
        .expect("export PNG");
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
    files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::Jpeg)
        .expect("export JPEG");
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
    files::export_as_image(&rgba, 2, 2, &path, image::ImageFormat::Png)
        .expect("export PNG");
    let imported = files::import_image_as_canvas(&path).expect("import PNG");
    assert_eq!(imported.width, 2);
    assert_eq!(imported.height, 2);
    assert_eq!(imported.pixels.len(), 1);
}

/// Loading non-zstd data should return an error.
#[test]
fn invalid_data_returns_error() {
    let bad = b"this is not zstd-compressed json";
    let result = files::load_app_from_data(bad);
    assert!(result.is_err());
}

#[test]
fn empty_data_returns_error() {
    let result = files::load_app_from_data(&[]);
    assert!(result.is_err());
}
