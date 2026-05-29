//! Tests for `brush_parsers::parse_gbr` — GIMP Brush format parsing.
//!
//! Constructs synthetic GBR v1 and v2 byte buffers and verifies correct
//! pixel decoding, spacing, and error handling.

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
