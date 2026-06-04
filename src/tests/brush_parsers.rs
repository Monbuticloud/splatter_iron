//! Tests for `brush_parsers` — GBR and ABR brush format parsing.
//!
//! Constructs synthetic GBR v1/v2 and ABR v6–10 byte buffers and verifies
//! correct pixel decoding, spacing, error handling, and computed brush
//! rasterisation.

use crate::tools::brush_parsers::parse_abr;
use crate::tools::brush_parsers::parse_gbr;

/// Build a valid GBR v2 byte buffer for a 2×2 RGBA brush.

fn make_gbr_v2_rgba() -> Vec<u8> {

    let mut buf = Vec::new();

    buf.extend_from_slice(b"GIMP");

    // Version "2.1 " (4-byte string) — common in GIMP 2.10+
    buf.extend_from_slice(b"2.1 ");

    // Width = 2, Height = 2 (u32 BE)
    buf.extend_from_slice(&2u32.to_be_bytes());

    buf.extend_from_slice(&2u32.to_be_bytes());

    // Bytes per pixel = 4 (RGBA)
    buf.extend_from_slice(&4u32.to_be_bytes());

    // Spacing = 20 (u32 BE)
    buf.extend_from_slice(&20u32.to_be_bytes());

    // Pixel data: 4 pixels, RGBA straight-alpha
    // Red   (255, 0, 0, 255)
    // Green (0, 255, 0, 255)
    // Blue  (0, 0, 255, 255)
    // White (255, 255, 255, 128)
    buf.extend_from_slice(&[255, 0, 0, 255]);

    buf.extend_from_slice(&[0, 255, 0, 255]);

    buf.extend_from_slice(&[0, 0, 255, 255]);

    buf.extend_from_slice(&[255, 255, 255, 128]);

    buf
}

/// Build a valid GBR v2 grayscale byte buffer.

fn make_gbr_v2_gray() -> Vec<u8> {

    let mut buf = Vec::new();

    buf.extend_from_slice(b"GIMP");

    buf.extend_from_slice(b"2.1 ");

    buf.extend_from_slice(&2u32.to_be_bytes());

    buf.extend_from_slice(&2u32.to_be_bytes());

    buf.extend_from_slice(&1u32.to_be_bytes()); // 1 byte per pixel
    buf.extend_from_slice(&15u32.to_be_bytes()); // spacing 15
    // Grayscale values (opacity): 0, 128, 255, 64
    buf.extend_from_slice(&[0, 128, 255, 64]);

    buf
}

/// Build a GBR v1 byte buffer (no spacing field).

fn make_gbr_v1() -> Vec<u8> {

    let mut buf = Vec::new();

    buf.extend_from_slice(b"GIMP");

    buf.extend_from_slice(&1u32.to_be_bytes()); // version 1
    buf.extend_from_slice(&1u32.to_be_bytes()); // width = 1
    buf.extend_from_slice(&1u32.to_be_bytes()); // height = 1
    buf.extend_from_slice(&4u32.to_be_bytes()); // bpp = 4
    // No spacing field in v1
    buf.extend_from_slice(&[64, 128, 192, 255]); // single pixel
    buf
}

/// Parse a valid GBR v2 RGBA brush and verify dimensions + pixel count.
#[test]

fn parse_gbr_v2_rgba_basic() {

    let buf = make_gbr_v2_rgba();

    let tips = parse_gbr(&buf).expect("should parse valid GBR v2 RGBA");

    assert_eq!(tips.len(), 1);

    let tip = &tips[0];

    assert_eq!(tip.width, 2);

    assert_eq!(tip.height, 2);

    assert_eq!(tip.pixels.len(), 4);

    assert_eq!(tip.spacing, 20);
}

/// Verify RGBA pixel values: count, ordering, and premultiplied format.
#[test]

fn parse_gbr_v2_rgba_pixels() {

    let buf = make_gbr_v2_rgba();

    let tips = parse_gbr(&buf).expect("should parse");

    let tip = &tips[0];

    assert_eq!(tip.pixels.len(), 4);

    // Each pixel must be premultiplied: R ≤ A, G ≤ A, B ≤ A
    for (i, p) in tip.pixels.iter().enumerate() {

        assert!(p.r() <= p.a(), "pixel {i}: R must be ≤ A (premultiplied)");

        assert!(p.g() <= p.a(), "pixel {i}: G must be ≤ A (premultiplied)");

        assert!(p.b() <= p.a(), "pixel {i}: B must be ≤ A (premultiplied)");
    }

    // Pixel 0 is fully opaque red → (255,0,0,255)
    assert_eq!(tip.pixels[0].to_array(), [255, 0, 0, 255]);

    // Pixel 2 is fully opaque blue → (0,0,255,255)
    assert_eq!(tip.pixels[2].to_array(), [0, 0, 255, 255]);

    // Pixel 3: white at alpha 128 → premultiplied (128,128,128,128)
    assert_eq!(tip.pixels[3].to_array(), [128, 128, 128, 128]);
}

/// Parse a grayscale GBR brush.
#[test]

fn parse_gbr_v2_grayscale() {

    let buf = make_gbr_v2_gray();

    let tips = parse_gbr(&buf).expect("should parse grayscale GBR");

    let tip = &tips[0];

    assert_eq!(tip.width, 2);

    assert_eq!(tip.height, 2);

    assert_eq!(tip.spacing, 15);

    // Grayscale value 128 → white at alpha 128 → premultiplied (128,128,128,128)
    let p1 = tip.pixels[1];

    assert!(p1.r() == p1.g() && p1.g() == p1.b());

    assert_eq!(p1.a(), 128);

    assert_ne!(p1.r(), 0);

    // Grayscale value 0 → fully transparent
    let p0 = tip.pixels[0];

    assert_eq!(p0.a(), 0);
}

/// Parse a GBR v1 file (no spacing).
#[test]

fn parse_gbr_v1() {

    let buf = make_gbr_v1();

    let tips = parse_gbr(&buf).expect("should parse GBR v1");

    let tip = &tips[0];

    assert_eq!(tip.width, 1);

    assert_eq!(tip.height, 1);

    // v1 defaults to spacing 25
    assert_eq!(tip.spacing, 25);
}

/// Invalid magic should return an error.
#[test]

fn parse_gbr_invalid_magic() {

    let buf = b"NOTGIMP...".to_vec();

    assert!(parse_gbr(&buf).is_err());
}

/// Truncated file should return an error.
#[test]

fn parse_gbr_truncated() {

    let buf = b"GIMP".to_vec();

    assert!(parse_gbr(&buf).is_err());
}

/// Zero dimensions should return an error.
#[test]

fn parse_gbr_zero_dimensions() {

    let mut buf = Vec::new();

    buf.extend_from_slice(b"GIMP");

    buf.extend_from_slice(&2u32.to_be_bytes());

    buf.extend_from_slice(&0u32.to_be_bytes()); // width = 0
    buf.extend_from_slice(&0u32.to_be_bytes()); // height = 0
    buf.extend_from_slice(&4u32.to_be_bytes());

    buf.extend_from_slice(&25u32.to_be_bytes());

    assert!(parse_gbr(&buf).is_err());
}

/// Unsupported bpp should return an error.
#[test]

fn parse_gbr_unsupported_bpp() {

    let mut buf = Vec::new();

    buf.extend_from_slice(b"GIMP");

    buf.extend_from_slice(&2u32.to_be_bytes());

    buf.extend_from_slice(&2u32.to_be_bytes());

    buf.extend_from_slice(&2u32.to_be_bytes());

    buf.extend_from_slice(&3u32.to_be_bytes()); // bpp = 3 (unsupported)
    buf.extend_from_slice(&25u32.to_be_bytes());

    assert!(parse_gbr(&buf).is_err());
}

// ---------------------------------------------------------------------------
// ABR parser tests (Photoshop Brush, versions 6–10)
// ---------------------------------------------------------------------------

/// Build a minimal ABR v6 file with a single raw-BGRA sampled brush (2x2).

fn make_abr_v6_bgra() -> Vec<u8> {

    let bgra: [u8; 16] = [
        0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 255, 255, 255, 128,
    ];

    let mut block_data = Vec::new();

    block_data.extend_from_slice(b"data");

    block_data.extend_from_slice(&16u32.to_be_bytes());

    block_data.extend_from_slice(&bgra);

    let block_size = block_data.len() as u32;

    let mut buf = Vec::new();

    buf.extend_from_slice(b"8BPB");

    buf.extend_from_slice(&6u16.to_be_bytes());

    buf.extend_from_slice(&1u16.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&[0u8; 2]);

    buf.extend_from_slice(b"8BIM");

    buf.extend_from_slice(&1u16.to_be_bytes());

    buf.extend_from_slice(&block_size.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&block_data);

    buf
}

/// Build a minimal ABR v6 file with one computed round brush (diam=6).

fn make_abr_v6_computed_round() -> Vec<u8> {

    let mut block_data = Vec::new();

    block_data.extend_from_slice(b"diam");

    block_data.extend_from_slice(&2u32.to_be_bytes());

    block_data.extend_from_slice(&6u16.to_be_bytes());

    block_data.extend_from_slice(b"shpe");

    block_data.extend_from_slice(&2u32.to_be_bytes());

    block_data.extend_from_slice(&0u16.to_be_bytes());

    let block_size = block_data.len() as u32;

    let mut buf = Vec::new();

    buf.extend_from_slice(b"8BPB");

    buf.extend_from_slice(&6u16.to_be_bytes());

    buf.extend_from_slice(&1u16.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&[0u8; 2]);

    buf.extend_from_slice(b"8BIM");

    buf.extend_from_slice(&2u16.to_be_bytes());

    buf.extend_from_slice(&block_size.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&block_data);

    buf
}

/// Build an ABR v6 file with two computed brush subblocks (round + square).

fn make_abr_v6_multi() -> Vec<u8> {

    let mut block1 = Vec::new();

    block1.extend_from_slice(b"diam");

    block1.extend_from_slice(&2u32.to_be_bytes());

    block1.extend_from_slice(&6u16.to_be_bytes());

    block1.extend_from_slice(b"shpe");

    block1.extend_from_slice(&2u32.to_be_bytes());

    block1.extend_from_slice(&0u16.to_be_bytes());

    let size1 = block1.len() as u32;

    let mut block2 = Vec::new();

    block2.extend_from_slice(b"diam");

    block2.extend_from_slice(&2u32.to_be_bytes());

    block2.extend_from_slice(&4u16.to_be_bytes());

    block2.extend_from_slice(b"shpe");

    block2.extend_from_slice(&2u32.to_be_bytes());

    block2.extend_from_slice(&1u16.to_be_bytes());

    let size2 = block2.len() as u32;

    let mut buf = Vec::new();

    buf.extend_from_slice(b"8BPB");

    buf.extend_from_slice(&6u16.to_be_bytes());

    buf.extend_from_slice(&2u16.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&[0u8; 2]);

    buf.extend_from_slice(b"8BIM");

    buf.extend_from_slice(&2u16.to_be_bytes());

    buf.extend_from_slice(&size1.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&block1);

    buf.extend_from_slice(b"8BIM");

    buf.extend_from_slice(&2u16.to_be_bytes());

    buf.extend_from_slice(&size2.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&block2);

    buf
}

#[test]

fn parse_abr_invalid_magic() {

    assert!(parse_abr(b"XXXX").is_err());
}

#[test]

fn parse_abr_truncated() {

    assert!(parse_abr(b"8BPB").is_err());
}

#[test]

fn parse_abr_unsupported_version() {

    let mut buf = Vec::new();

    buf.extend_from_slice(b"8BPB");

    buf.extend_from_slice(&5u16.to_be_bytes());

    buf.extend_from_slice(&1u16.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&[0u8; 2]);

    assert!(parse_abr(&buf).is_err());
}

#[test]

fn parse_abr_sampled_bgra() {

    let buf = make_abr_v6_bgra();

    let tips = parse_abr(&buf).expect("should parse ABR v6 BGRA sampled");

    assert_eq!(tips.len(), 1);

    let tip = &tips[0];

    assert_eq!(tip.pixels.len(), 4);

    assert_eq!(tip.spacing, 25);

    for (i, p) in tip.pixels.iter().enumerate() {

        assert!(p.r() <= p.a(), "sampled pixel {i}: R must be ≤ A");
    }

    assert_eq!(tip.pixels[0].to_array(), [255, 0, 0, 255]);

    assert_eq!(tip.pixels[1].to_array(), [0, 255, 0, 255]);

    assert_eq!(tip.pixels[2].to_array(), [0, 0, 255, 255]);

    assert_eq!(tip.pixels[3].to_array(), [128, 128, 128, 128]);
}

#[test]

fn parse_abr_computed_round() {

    let buf = make_abr_v6_computed_round();

    let tips = parse_abr(&buf).expect("should parse computed round brush");

    assert_eq!(tips.len(), 1);

    let tip = &tips[0];

    assert_eq!(tip.width, 6);

    assert_eq!(tip.height, 6);

    assert_eq!(tip.pixels.len(), 36);

    let center = tip.pixels[(3 * tip.width + 3) as usize];

    assert_eq!(center.a(), 255, "round brush center should be opaque");
}

#[test]

fn parse_abr_computed_square() {

    let buf = make_abr_v6_multi();

    let tips = parse_abr(&buf).expect("should parse multi-subblock ABR");

    let square = &tips[1];

    assert_eq!(square.width, 4);

    assert_eq!(square.height, 4);

    assert_eq!(square.pixels.len(), 16);
}

#[test]

fn parse_abr_multi_subblock() {

    let buf = make_abr_v6_multi();

    let tips = parse_abr(&buf).expect("should parse multi-subblock ABR");

    assert_eq!(tips.len(), 2);

    assert_eq!(tips[0].width, 6);

    assert_eq!(tips[1].width, 4);
}

#[test]

fn parse_abr_empty_no_tips() {

    let mut buf = Vec::new();

    buf.extend_from_slice(b"8BPB");

    buf.extend_from_slice(&6u16.to_be_bytes());

    buf.extend_from_slice(&0u16.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&[0u8; 2]);

    assert!(parse_abr(&buf).is_err(), "no subblocks should error");
}

#[test]

fn parse_abr_computed_default_spacing() {

    let buf = make_abr_v6_computed_round();

    let tips = parse_abr(&buf).expect("should parse");

    assert_eq!(tips[0].spacing, 25);
}

#[test]

fn parse_abr_computed_explicit_spacing() {

    let mut block_data = Vec::new();

    block_data.extend_from_slice(b"diam");

    block_data.extend_from_slice(&2u32.to_be_bytes());

    block_data.extend_from_slice(&6u16.to_be_bytes());

    block_data.extend_from_slice(b"spac");

    block_data.extend_from_slice(&2u32.to_be_bytes());

    block_data.extend_from_slice(&10u16.to_be_bytes());

    block_data.extend_from_slice(b"shpe");

    block_data.extend_from_slice(&2u32.to_be_bytes());

    block_data.extend_from_slice(&0u16.to_be_bytes());

    let block_size = block_data.len() as u32;

    let mut buf = Vec::new();

    buf.extend_from_slice(b"8BPB");

    buf.extend_from_slice(&6u16.to_be_bytes());

    buf.extend_from_slice(&1u16.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&[0u8; 2]);

    buf.extend_from_slice(b"8BIM");

    buf.extend_from_slice(&2u16.to_be_bytes());

    buf.extend_from_slice(&block_size.to_be_bytes());

    buf.extend_from_slice(&[0u8; 4]);

    buf.extend_from_slice(&block_data);

    let tips = parse_abr(&buf).expect("should parse with explicit spacing");

    assert_eq!(tips[0].spacing, 10);
}

/// `parse_brush_file` with no extension returns an error.
#[test]

fn parse_brush_file_no_extension() {

    let path = std::path::Path::new("no_ext");

    let result = crate::tools::brush_parsers::parse_brush_file(path);

    assert!(result.is_err());

    assert!(result.unwrap_err().contains("no extension"));
}

/// `parse_brush_file` with an unsupported extension returns an error.
#[test]

fn parse_brush_file_unsupported_format() {

    let dir = tempfile::tempdir().expect("temp dir");

    let path = dir.path().join("test.xyz");

    std::fs::write(&path, b"dummy").expect("write dummy file");

    let result = crate::tools::brush_parsers::parse_brush_file(&path);

    assert!(result.is_err());

    assert!(result.unwrap_err().contains("Unsupported brush format"));
}

/// `parse_brush_file` with a non-existent file returns a read error.
#[test]

fn parse_brush_file_read_failure() {

    let path = std::path::Path::new("/nonexistent/path/test.gbr");

    let result = crate::tools::brush_parsers::parse_brush_file(path);

    assert!(result.is_err());

    assert!(result.unwrap_err().contains("Failed to read file"));
}

/// `parse_brush_file` with a valid GBR v2 file returns a single tip with the file stem name.
#[test]

fn parse_brush_file_gbr_single_tip_named_by_stem() {

    let dir = tempfile::tempdir().expect("temp dir");

    let path = dir.path().join("my_brush.gbr");

    let gbr_data = make_gbr_v2_rgba();

    std::fs::write(&path, &gbr_data).expect("write gbr");

    let tips = crate::tools::brush_parsers::parse_brush_file(&path).expect("parse gbr");

    assert_eq!(tips.len(), 1);

    assert_eq!(tips[0].name, "my_brush");

    assert_eq!(tips[0].width, 2);

    assert_eq!(tips[0].height, 2);
}
