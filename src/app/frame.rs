//! Frame-lifecycle methods called once per frame from `ui()`: poll async
//! I/O, manage render-state transitions, sync GPU texture, and autosave.

use std::path::PathBuf;

use eframe::egui;

use crate::app::MyApp;
use crate::app::PendingStamp;
use crate::app::ProgressState;
use crate::document::SaveState;

impl MyApp {
    /// Poll file-dialog and save-result channels and transfer loaded
    /// stamp/brush data into pending-dialog state.
    pub(crate) fn poll_file_results(&mut self, ctx: &egui::Context) {
        self.file_io.poll_dialog_results(
            &mut self.document,
            &mut self.undo,
            &mut self.ui.errors.list,
        );
        self.file_io.poll_save_results(&mut self.document, &mut self.ui.errors.list);

        // Execute deferred action after save completes.
        if self.document.save_state == SaveState::Idle {
            if let Some(action) = self.ui.dialogs.pending_after_save.take() {
                self.execute_unsaved_action(action);
            }
        }

        // Poll load/import results (applies `Canvas` to document).
        self.file_io.poll_load_import_results(
            &mut self.document,
            &mut self.undo,
            &mut self.ui.errors.list,
        );

        // Track async operation progress.
        if self.file_io.load_in_flight {
            self.ui.progress = ProgressState::Loading;
        } else if self.file_io.import_in_flight {
            self.ui.progress = ProgressState::Importing;
        } else if self.file_io.export_in_flight {
            self.ui.progress = ProgressState::Exporting;
        } else {
            self.ui.progress = ProgressState::Idle;
        }
        if self.file_io.poll_export_results(&mut self.ui.errors.list) {
            self.ui.progress = ProgressState::Idle;
        }

        if let Some((pixels, w, h, name)) = self.file_io.loaded_stamp_data.take() {
            self.ui.dialogs.pending_stamp_name = Some(PendingStamp {
                pixels,
                width: w,
                height: h,
                name,
                spacing: 25,
            });
        }

        if let Some(tips) = self.file_io.loaded_brush_data.take() {
            let pending: Vec<PendingStamp> = tips
                .into_iter()
                .map(|tip| PendingStamp {
                    pixels: tip.pixels,
                    width: tip.width,
                    height: tip.height,
                    name: tip.name,
                    spacing: tip.spacing,
                })
                .collect();
            self.ui.dialogs.pending_brushes = Some(pending);
        }

        // Track recently saved/loaded files.
        if !self.document.savefile_path.is_empty() {
            let path = PathBuf::from(&self.document.savefile_path);
            let is_already_tracked =
                self.ui.recent_files.first().is_some_and(|p| p == &path);
            if !is_already_tracked {
                self.push_recent_file(path);
                self.save_config();
            }
        }

        self.stamp_library.create_textures(ctx);
        self.brush_library.create_textures(ctx);
    }
}
