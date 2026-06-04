//! Async export orchestration: background-thread image encoding and archive
//! serialisation.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use crate::canvas::Canvas;
use crate::files::ExportStrategy;

/// Spawns background threads to encode and write exported images or archives.
///
/// Owns the export channel pair, the pluggable [`ExportStrategy`], and
/// the in-flight flag. Used by the frame loop to trigger exports and
/// poll for completions.

pub struct ExportManager {
    /// Injected export strategy for writing image files.
    pub export_strategy: Arc<dyn ExportStrategy + Send + Sync>,
    /// Channel sender for export results from background thread.
    pub export_result_sender: mpsc::Sender<anyhow::Result<()>>,
    /// Channel receiver for export results on the UI thread.
    pub export_result_receiver: mpsc::Receiver<anyhow::Result<()>>,
    /// `true` while an async export thread is running.
    pub export_in_flight: bool,
}

impl std::fmt::Debug for ExportManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        f.debug_struct("ExportManager")
            .field("export_in_flight", &self.export_in_flight)
            .finish()
    }
}

impl ExportManager {
    /// Create a new `ExportManager` with an internal channel pair.
    ///
    /// # Parameters
    ///
    /// * `export_strategy` — Strategy for writing file exports, shared via
    ///   `Arc` for cross-thread access.

    pub fn new(export_strategy: Arc<dyn ExportStrategy + Send + Sync>) -> Self {

        let (export_result_sender, export_result_receiver) = mpsc::channel();

        Self {
            export_strategy,
            export_result_sender,
            export_result_receiver,
            export_in_flight: false,
        }
    }

    /// Spawn a background thread to encode and write the exported image.
    ///
    /// # Parameters
    ///
    /// * `premultiplied_rgba` — The already-blended premultiplied RGBA buffer.
    /// * `width` — Image width in pixels.
    /// * `height` — Image height in pixels.
    /// * `path` — Destination file path.

    pub fn trigger_async_export(
        &mut self,
        premultiplied_rgba: Arc<Vec<u8>>,
        width: u32,
        height: u32,
        path: PathBuf,
    ) {

        self.export_in_flight = true;

        let strategy = Arc::clone(&self.export_strategy);

        let sender = self.export_result_sender.clone();

        std::thread::spawn(move || {

            let result = strategy.export(&premultiplied_rgba, width, height, &path);

            let _ = sender.send(result);
        });
    }

    /// Spawn a background thread to serialise and write an xz-compressed
    /// `.splatterarchive` file.
    ///
    /// # Parameters
    ///
    /// * `canvas` — The canvas to serialise.
    /// * `path` — Destination file path.

    pub fn trigger_async_export_archive(&mut self, canvas: Canvas, path: PathBuf) {

        self.export_in_flight = true;

        let sender = self.export_result_sender.clone();

        std::thread::spawn(move || {

            let result = crate::files::save_canvas_to_path_xz(&canvas, &path);

            let _ = sender.send(result);
        });
    }

    /// Poll for completed async export results.
    ///
    /// Returns `true` if an export result was processed (success or failure).
    ///
    /// # Parameters
    ///
    /// * `error_list` — Error list to push failure messages into.

    pub fn poll_export_results(&mut self, error_list: &mut Vec<String>) -> bool {

        if let Ok(result) = self.export_result_receiver.try_recv() {

            self.export_in_flight = false;

            if let Err(error) = result {

                error_list.push(format!("Export failed: {error}"));
            }

            true
        } else {

            false
        }
    }
}
