//! Undo-record types ([`UndoRecord`], [`RunSegment`], [`BeforePixels`])
//! and their application to the canvas ([`undo_apply`], [`redo_apply`]).
//! Run-length compression ([`compress_and_store`]) reduces storage for uniform spans.

use std::io::Cursor;
use std::io::Read;

use eframe::egui::Color32;

use crate::canvas::Canvas;
use crate::canvas::Layer;
use crate::canvas::LayerMode;
use crate::pixel::alpha_blend;
use crate::pixel::alpha_blend_simd_four;

/// Maximum allowed bytes for decompressed undo data (512 MB).
/// Matches the constant in `files.rs` for consistency.

const MAX_DECOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;

/// Compressed storage for a run of before-pixels: either all the same color
/// (`All`) or an offset+length into a flat `before_pixels` buffer (`Many`).
#[derive(Clone)]

pub enum BeforePixels {
    /// Every pixel in the run had the same original color.
    All(Color32),
    /// Pixels had distinct colors (run refers into `UndoRecord::Run::before_pixels`).
    Many {
        /// Starting index in the flat `before_pixels` buffer.
        offset: u32,
        /// Number of contiguous pixels in this run.
        length: u32,
    },
}

impl std::fmt::Debug for BeforePixels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        match self {
            Self::All(color) => f.debug_tuple("All").field(color).finish(),
            Self::Many { offset, length } => f
                .debug_struct("Many")
                .field("offset", offset)
                .field("length", length)
                .finish(),
        }
    }
}

/// A contiguous range of pixels in an undo `Run` record.
#[derive(Debug)]

pub struct RunSegment {
    /// Starting pixel index within the layer's flat pixel array.
    pub start: u32,
    /// Number of contiguous pixels in this run.
    pub length: u32,
    /// Original pixel values before the stroke (compressed storage).
    pub before: BeforePixels,
}

pub(crate) const RLE_SHORT_RUN_THRESHOLD: u32 = 8;

/// Compress a contiguous run of pixel data into a flat `before_pixels` buffer.
///
/// If the run is shorter than 8 pixels or not all identical, appends the
/// pixel data to `buf` and returns `BeforePixels::Many { offset, length }`.
/// Uniform runs of 8+ pixels return `BeforePixels::All` without touching `buf`.
///
/// # Parameters
///
/// * `slice` — Contiguous run of before-pixels to compress.
/// * `buf` — Flat buffer (ownership held by the enclosing `UndoRecord::Run`).

pub fn compress_and_store(slice: &[Color32], buf: &mut Vec<Color32>) -> (BeforePixels, u32) {

    let length = slice.len() as u32;

    if length >= RLE_SHORT_RUN_THRESHOLD && slice.iter().all(|&p| p == slice[0]) {

        (BeforePixels::All(slice[0]), length)
    } else {

        let offset = buf.len() as u32;

        buf.extend_from_slice(slice);

        (BeforePixels::Many { offset, length }, length)
    }
}

/// A record of a single change in the undo/redo stack.
///
/// Variants cover both drawing strokes and layer-structural operations.
#[derive(Debug)]

pub enum UndoRecord {
    /// Per-pixel before/after state for a brush stroke.
    Run {
        /// Index of the layer that was modified.
        layer_index: usize,
        /// Color applied by the stroke (after-state).
        color_after: Color32,
        /// Compressed run-length segments preserving before-pixel data.
        runs: Vec<RunSegment>,
        /// Flat buffer of all non-uniform before-pixels referenced by
        /// `BeforePixels::Many { offset, length }` in each run.
        before_pixels: Vec<Color32>,
        /// zstd-compressed version of `before_pixels` stored in the undo
        /// history stack to reduce memory; decompressed on undo access.
        compressed_before_pixels: Option<Vec<u8>>,
        /// zstd-compressed full-layer snapshot for strokes covering >50% of
        /// the layer's pixels. When set, `undo_apply` restores the entire
        /// layer from this snapshot instead of iterating per-pixel runs.
        /// `runs` and `color_after` are still used for `redo_apply`.
        full_layer_before: Option<Vec<u8>>,
        /// Whether this stroke was drawn as an alpha overlay (vs. opaque).
        is_alpha_overlay: bool,
    },
    /// A new layer was created.
    AddLayer {
        /// Index at which the layer was inserted.
        index: usize,
        /// Canvas width at the time of creation (for transparent pixel buffer).
        width: u32,
        /// Canvas height at the time of creation.
        height: u32,
        /// Layer name assigned at creation.
        name: String,
        /// Layer visibility at creation.
        visible: bool,
        /// Layer opacity at creation.
        opacity: u8,
        /// Layer compositing mode at creation.
        mode: LayerMode,
    },
    /// An existing layer was deleted.
    DeleteLayer {
        /// Former index of the deleted layer.
        index: usize,
        /// The deleted layer's full state (for restoration).
        layer: Box<Layer>,
    },
    /// A layer was moved up or down in the stack.
    MoveLayer {
        /// Original index before the move.
        from_index: usize,
        /// Target index after the move.
        to_index: usize,
    },
    /// A layer's non-pixel properties (visibility, opacity, name, mode) changed.
    ModifyLayer {
        /// Index of the modified layer.
        index: usize,
        /// Layer properties before the change.
        old_visible: bool,
        old_opacity: u8,
        old_name: String,
        old_mode: LayerMode,
        /// Layer properties after the change.
        new_visible: bool,
        new_opacity: u8,
        new_name: String,
        new_mode: LayerMode,
    },
}

impl UndoRecord {
    /// If the stroke covers more than 50% of the layer's pixels, replace the
    /// per-pixel `before_pixels` with a zstd-compressed full-layer snapshot.
    ///
    /// The snapshot is reconstructed by cloning the current (post-stroke)
    /// `layer` and restoring the original pixel values from `runs` and
    /// `before_pixels`. This trades per-pixel decompress overhead for a
    /// simple full-layer swap on undo, which is significantly faster for
    /// large strokes.
    ///
    /// Runs and `color_after` are preserved for `redo_apply`.
    ///
    /// Call before `compress_before`, and only when the current layer state
    /// is available (i.e. the stroke has already been applied).
    ///
    /// # Parameters
    ///
    /// * `layer` — The current (post-stroke) layer, used to reconstruct the
    ///   before-state from runs and before_pixels.
    /// * `level` — Zstd compression level for the snapshot.
    ///
    /// If the stroke covers more than 50% of the layer's pixels, replace the
    /// per-pixel `before_pixels` with a zstd-compressed full-layer snapshot.
    ///
    /// The snapshot is reconstructed by cloning the current (post-stroke)
    /// `layer` and restoring the original pixel values from `runs` and
    /// `before_pixels`. This trades per-pixel decompress overhead for a
    /// simple full-layer swap on undo, which is significantly faster for
    /// large strokes.
    ///
    /// Runs and `color_after` are preserved for `redo_apply`.
    ///
    /// Call before `compress_before`, and only when the current layer state
    /// is available (i.e. the stroke has already been applied).
    ///
    /// # Parameters
    ///
    /// * `layer` — The current (post-stroke) layer, used to reconstruct the
    ///   before-state from runs and before_pixels.
    /// * `level` — Zstd compression level for the snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error if zstd compression or JSON serialization fails.

    pub fn maybe_snapshot(&mut self, layer: &Layer, level: i32) -> anyhow::Result<()> {

        let UndoRecord::Run {
            runs,
            before_pixels,
            full_layer_before,
            ..
        } = self
        else {

            return Ok(());
        };

        if before_pixels.is_empty() || full_layer_before.is_some() {

            return Ok(());
        }

        let total_pixels = layer.pixels.len() as f64;

        let covered: f64 = runs.iter().map(|r| u64::from(r.length)).sum::<u64>() as f64;

        if covered <= total_pixels * 0.5 {

            return Ok(());
        }

        // Reconstruct the before-layer by restoring original pixels into a
        // clone of the current (post-stroke) layer.
        let mut before_layer = layer.clone();

        for run in runs {

            let start = run.start as usize;

            let end = start + run.length as usize;

            match &run.before {
                BeforePixels::All(color) => {

                    before_layer.pixels[start..end].fill(*color);
                }
                BeforePixels::Many { offset, length } => {

                    let off = *offset as usize;

                    let len = *length as usize;

                    before_layer.pixels[start..end].copy_from_slice(&before_pixels[off..off + len]);
                }
            }
        }

        let json = serde_json::to_vec(&before_layer)?;

        *full_layer_before = Some(zstd::encode_all(std::io::Cursor::new(json), level)?);

        before_pixels.clear();

        Ok(())
    }

    /// Compress `before_pixels` using zstd and store the result in
    /// `compressed_before_pixels`, then clear the uncompressed buffer.
    /// No-op if `before_pixels` is already empty or already compressed.
    ///
    /// Call before pushing this record into the undo history stack.
    ///
    /// # Errors
    ///
    /// Returns an error if zstd compression fails.

    pub fn compress_before(&mut self, level: i32) -> anyhow::Result<()> {

        match self {
            UndoRecord::Run {
                before_pixels,
                compressed_before_pixels,
                ..
            } if !before_pixels.is_empty() => {

                let bytes = bytemuck::cast_slice(before_pixels.as_slice());

                *compressed_before_pixels = Some(zstd::encode_all(Cursor::new(bytes), level)?);

                before_pixels.clear();
            }
            _ => {}
        }

        Ok(())
    }

    /// Decompress `compressed_before_pixels` back into `before_pixels`
    /// and clear the compressed buffer.
    /// No-op if `before_pixels` is already populated or no compressed data.
    ///
    /// Call before passing this record to `undo_apply`.
    ///
    /// # Panics
    ///
    /// Panics if zstd decompression fails or if decompressed data exceeds
    /// the size limit, indicating corrupt data or a malicious save.

    pub fn decompress_before(&mut self) {

        match self {
            UndoRecord::Run {
                before_pixels,
                compressed_before_pixels,
                ..
            } if compressed_before_pixels.is_some() => {

                if let Some(compressed) = compressed_before_pixels.take() {

                    let cursor = Cursor::new(compressed);

                    let mut limited = zstd::Decoder::new(cursor)
                        .expect("zstd decoder creation")
                        .take(MAX_DECOMPRESSED_BYTES);

                    let mut bytes = Vec::new();

                    limited
                        .read_to_end(&mut bytes)
                        .expect("zstd decompression of before_pixels");

                    let pixels: &[Color32] = bytemuck::cast_slice(&bytes);

                    *before_pixels = pixels.to_vec();
                }
            }
            _ => {}
        }
    }
}

/// Restore canvas state to before the operation recorded by `record`.
///
/// When the `Run` variant has a `full_layer_before` snapshot, the entire
/// layer is replaced from the decompressed snapshot (faster for large strokes).
/// Otherwise, per-pixel restoration from `runs` and `before_pixels` is used.
///
/// # Parameters
///
/// * `canvas` — The canvas to modify.
/// * `record` — The undo record describing the operation to reverse.
///
/// # Panics
///
/// Panics if a layer index in the record is out of bounds, indicating a
/// corrupt or mismatched undo record.
#[inline]

pub fn undo_apply(canvas: &mut Canvas, record: &UndoRecord) {

    match record {
        UndoRecord::Run {
            layer_index,
            runs,
            before_pixels,
            full_layer_before,
            ..
        } => {

            if let Some(compressed) = full_layer_before {

                // Full-layer restore: stream-decompress with size limit.
                let cursor = std::io::Cursor::new(compressed);

                let mut limited = zstd::Decoder::new(cursor)
                    .expect("zstd decoder creation")
                    .take(MAX_DECOMPRESSED_BYTES);

                let before_layer: Layer = serde_json::from_reader(&mut limited)
                    .expect("deserialization of layer snapshot");

                canvas.pixels[*layer_index] = before_layer;
            } else {

                // Per-pixel restore from runs and before_pixels.
                let layer = &mut canvas.pixels[*layer_index];

                for run in runs {

                    let end = (run.start as usize) + run.length as usize;

                    match &run.before {
                        BeforePixels::All(color) => {

                            layer.pixels[run.start as usize..end].fill(*color);
                        }
                        BeforePixels::Many { offset, length } => {

                            layer.pixels[run.start as usize..end].copy_from_slice(
                                &before_pixels
                                    [*offset as usize..*offset as usize + *length as usize],
                            );
                        }
                    }
                }
            }
        }
        UndoRecord::AddLayer { index, .. } => {

            canvas.pixels.remove(*index);
        }
        UndoRecord::DeleteLayer { index, layer } => {

            canvas.pixels.insert(*index, *layer.clone());
        }
        UndoRecord::MoveLayer {
            from_index,
            to_index,
        } => {

            canvas.pixels.swap(*to_index, *from_index);
        }
        UndoRecord::ModifyLayer {
            index,
            old_visible,
            old_opacity,
            old_name,
            old_mode,
            new_visible: _,
            new_opacity: _,
            new_name: _,
            new_mode: _,
        } => {

            let layer = &mut canvas.pixels[*index];

            layer.visible = *old_visible;

            layer.opacity = *old_opacity;

            layer.name.clone_from(old_name);

            layer.mode = *old_mode;
        }
    }
}

/// Reapply a previously undone operation from its undo record.
///
/// # Parameters
///
/// * `canvas` — The canvas to modify.
/// * `record` — The undo record containing the operation to reapply.
///
/// # Panics
///
/// Panics if a layer index in the record is out of bounds, indicating a
/// corrupt or mismatched undo record.
#[inline]

pub fn redo_apply(canvas: &mut Canvas, record: &UndoRecord) {

    match record {
        UndoRecord::Run {
            layer_index,
            color_after,
            runs,
            is_alpha_overlay,
            ..
        } => {

            let layer = &mut canvas.pixels[*layer_index];

            if *is_alpha_overlay {

                for run in runs {

                    let end = (run.start as usize) + run.length as usize;

                    let pixels = &mut layer.pixels[run.start as usize..end];

                    // SIMD bulk blend (4 pixels at a time)
                    let (simd, tail) = pixels.split_at_mut(pixels.len() - (pixels.len() % 4));

                    for chunk in simd.chunks_exact_mut(4) {

                        let arr: &mut [Color32; 4] = chunk
                            .try_into()
                            .expect("chunks_exact_mut yields exactly 4 elements");

                        alpha_blend_simd_four(arr, *color_after);
                    }

                    // Scalar tail (<4 pixels)
                    for pixel in tail.iter_mut() {

                        *pixel = alpha_blend(*pixel, *color_after);
                    }
                }
            } else {

                for run in runs {

                    let end = (run.start as usize) + run.length as usize;

                    layer.pixels[run.start as usize..end].fill(*color_after);
                }
            }
        }
        UndoRecord::AddLayer {
            index,
            width,
            height,
            name,
            visible,
            opacity,
            mode,
        } => {

            let pixel_count = (*width as usize) * (*height as usize);

            canvas.pixels.insert(
                *index,
                Layer {
                    pixels: vec![Color32::TRANSPARENT; pixel_count],
                    name: name.clone(),
                    visible: *visible,
                    opacity: *opacity,
                    mode: *mode,
                },
            );
        }
        UndoRecord::DeleteLayer { index, layer: _ } => {

            canvas.pixels.remove(*index);
        }
        UndoRecord::MoveLayer {
            from_index,
            to_index,
        } => {

            canvas.pixels.swap(*from_index, *to_index);
        }
        UndoRecord::ModifyLayer {
            index,
            old_visible: _,
            old_opacity: _,
            old_name: _,
            old_mode: _,
            new_visible,
            new_opacity,
            new_name,
            new_mode,
        } => {

            let layer = &mut canvas.pixels[*index];

            layer.visible = *new_visible;

            layer.opacity = *new_opacity;

            layer.name.clone_from(new_name);

            layer.mode = *new_mode;
        }
    }
}
