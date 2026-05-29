use eframe::egui::Color32;

use crate::canvas::DirtyRect;
use crate::tools::brush_common::apply_visited_runs;

/// `apply_visited_runs` with no visited pixels returns an empty run list.
#[test]
fn no_visited_pixels_returns_empty() {
    let mut pixels = vec![Color32::TRANSPARENT; 100];
    let dirty = DirtyRect::new(0, 0, 9, 9);
    let mut visited = vec![0u32; 100];
    let mut drag = vec![0u32; 100];

    let runs = apply_visited_runs(
        &mut pixels,
        &dirty,
        10,
        &visited,
        1,
        Color32::RED,
        false,
        &mut drag,
        0,
    );
    assert!(runs.is_empty());
}

/// `apply_visited_runs` with all pixels visited captures the correct runs.
#[test]
fn all_visited_produces_runs() {
    let mut pixels = vec![Color32::TRANSPARENT; 4];
    let dirty = DirtyRect::new(0, 0, 1, 1);
    let mut visited = vec![1u32; 4];
    let mut drag = vec![0u32; 4];

    let runs = apply_visited_runs(
        &mut pixels,
        &dirty,
        2,
        &visited,
        1,
        Color32::RED,
        false,
        &mut drag,
        0,
    );
    assert_eq!(runs.len(), 2, "one run per row");
    assert_eq!(pixels.iter().filter(|p| **p == Color32::RED).count(), 4);
}

/// `apply_visited_runs` with alpha-overlay blends correctly.
#[test]
fn alpha_overlay_blends() {
    let mut pixels = vec![Color32::from_rgba_premultiplied(255, 0, 0, 255); 4];
    let dirty = DirtyRect::new(0, 0, 1, 1);
    let mut visited = vec![1u32; 4];
    let mut drag = vec![0u32; 4];
    let overlay = Color32::from_rgba_premultiplied(0, 0, 255, 128);

    let runs = apply_visited_runs(
        &mut pixels,
        &dirty,
        2,
        &visited,
        1,
        overlay,
        true,
        &mut drag,
        1,
    );
    assert!(!runs.is_empty());
    // Every pixel should have been modified (blended)
    assert!(pixels.iter().all(|p| *p != Color32::from_rgba_premultiplied(255, 0, 0, 255)));
}

/// `apply_visited_runs` skips already processed pixels in alpha-overlay mode.
#[test]
fn alpha_overlay_skips_processed() {
    let mut pixels = vec![Color32::TRANSPARENT; 4];
    let dirty = DirtyRect::new(0, 0, 1, 1);
    let mut visited = vec![1u32; 4];
    let mut drag = vec![1u32; 4]; // all already processed
    let overlay = Color32::RED;

    let runs = apply_visited_runs(
        &mut pixels,
        &dirty,
        2,
        &visited,
        1,
        overlay,
        true,
        &mut drag,
        1,
    );
    assert!(runs.is_empty(), "all pixels already processed");
}
