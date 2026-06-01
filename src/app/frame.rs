//! Frame-lifecycle methods called once per frame from `ui()`: poll async
//! I/O, manage render-state transitions, sync GPU texture, and autosave.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui;
use eframe::egui_wgpu::wgpu;

use crate::app::AUTOSAVE_INTERVAL_MINUTES;
use crate::app::MyApp;
use crate::app::PendingStamp;
use crate::app::ProgressState;
use crate::app::REPAINT_DELAY_MULTIPLIER;
use crate::app::UNFOCUSED_SLEEP_MILLISECONDS;
use crate::canvas::RenderState;
use crate::document::SaveState;
use crate::file_io::DispatchedAction;
use crate::file_io::SaveKind;

impl MyApp {
    /// Poll file-dialog and save-result channels and transfer loaded
    /// stamp/brush data into pending-dialog state.
    pub(crate) fn poll_file_results(&mut self, ctx: &egui::Context) {
        // Capture save state before polling, to detect completion.
        let was_saving = matches!(
            self.ui.progress,
            ProgressState::Saving | ProgressState::Autosaving
        );
        let was_autosave = self.ui.progress == ProgressState::Autosaving;

        // 1. Poll dialog results → dispatch to the appropriate subsystem.
        let actions = self
            .dialog_manager
            .poll_dialog_results(&mut self.ui.errors.list);
        for action in actions {
            match action {
                DispatchedAction::Save(path) => {
                    self.save_manager
                        .trigger_async_save(&mut self.document, SaveKind::ManualSave(path));
                }
                DispatchedAction::Load(path) => {
                    self.load_import_manager.trigger_async_load(path);
                }
                DispatchedAction::Import(path) => {
                    self.load_import_manager.trigger_async_import(path);
                }
                DispatchedAction::Export(_index, path) => {
                    if self.document.canvas.output_rgba.is_empty() {
                        continue;
                    }
                    let rgba = Arc::clone(&self.document.canvas.output_rgba);
                    let w = self.document.canvas.width;
                    let h = self.document.canvas.height;
                    self.export_manager.trigger_async_export(rgba, w, h, path);
                }
                DispatchedAction::ExportArchive(path) => {
                    let canvas = (*self.document.canvas).clone();
                    self.export_manager
                        .trigger_async_export_archive(canvas, path);
                }
                DispatchedAction::ImportArchive(path) => {
                    self.load_import_manager.trigger_async_import_archive(path);
                }
            }
        }

        // 2. Poll save results.
        self.save_manager
            .poll_save_results(&mut self.document, &mut self.ui.errors.list);

        // 3. Execute deferred action after save completes.
        if self.document.save_state == SaveState::Idle {
            if let Some(action) = self.ui.dialogs.pending_after_save.take() {
                self.execute_unsaved_action(action);
            }
        }

        // 4. Poll load/import results (applies `Canvas` to document).
        self.load_import_manager.poll_load_import_results(
            &mut self.document,
            &mut self.undo,
            &mut self.ui.errors.list,
        );

        // 5. Track async operation progress.
        // Priority: load > import > export > save > idle.
        if self.load_import_manager.load_in_flight {
            self.ui.progress = ProgressState::Loading;
        } else if self.load_import_manager.import_in_flight {
            self.ui.progress = ProgressState::Importing;
        } else if self.export_manager.export_in_flight {
            self.ui.progress = ProgressState::Exporting;
        } else if self.document.save_state == SaveState::InFlight {
            self.ui.progress = if self.save_manager.autosave_in_flight {
                ProgressState::Autosaving
            } else {
                ProgressState::Saving
            };
        } else if was_saving {
            // Save just completed this frame — set brief flash message.
            self.ui.last_status_message = Some((
                if was_autosave { "Autosaved" } else { "Saved" },
                std::time::Instant::now(),
            ));
            self.ui.progress = ProgressState::Idle;
        } else {
            self.ui.progress = ProgressState::Idle;
        }
        if self.export_manager.poll_export_results(&mut self.ui.errors.list) {
            self.ui.progress = ProgressState::Idle;
        }

        // 6. Transfer stamp/brush data from dialog manager to UI dialogs.
        if let Some((pixels, w, h, name)) = self.dialog_manager.loaded_stamp_data.take() {
            self.ui.dialogs.pending_stamp_name = Some(PendingStamp {
                pixels,
                width: w,
                height: h,
                name,
                spacing: 25,
            });
        }

        if let Some(tips) = self.dialog_manager.loaded_brush_data.take() {
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

        // 7. Track recently saved/loaded files.
        if !self.document.savefile_path.is_empty() {
            let path = PathBuf::from(&self.document.savefile_path);
            let is_already_tracked = self.ui.recent_files.first().is_some_and(|p| p == &path);
            if !is_already_tracked {
                self.push_recent_file(path);
                self.save_config();
            }
        }

        self.stamp_library.create_textures(ctx);
        self.brush_library.create_textures(ctx);
    }

    /// Advance the render-state machine and return `true` if the frame should
    /// be skipped (viewport unfocused or frozen).
    pub(crate) fn update_render_state(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.ctx().input(|i| i.viewport().focused.unwrap_or(true)) {
            std::thread::sleep(std::time::Duration::from_millis(
                UNFOCUSED_SLEEP_MILLISECONDS,
            ));
            self.ui.render_state = RenderState::UnfocusedFrozen;
            return true;
        }
        let predicted_delta_time =
            Duration::from_secs_f32(ui.ctx().input(|i| i.predicted_dt).max(0.0));
        let real_delta_time = Duration::from_secs_f32(ui.ctx().input(|i| i.stable_dt).max(0.0));

        self.ui.time_elapsed += real_delta_time;

        match self.ui.render_state {
            RenderState::ActiveWake(duration) => {
                let remaining = duration.saturating_sub(predicted_delta_time);
                if remaining.is_zero() {
                    self.ui.render_state = RenderState::IdleThrottled;
                    ui.request_repaint_after(predicted_delta_time * REPAINT_DELAY_MULTIPLIER);
                } else {
                    self.ui.render_state = RenderState::ActiveWake(remaining);
                }
            }
            RenderState::IdleThrottled => {
                ui.request_repaint_after(predicted_delta_time * REPAINT_DELAY_MULTIPLIER);
            }
            RenderState::UnfocusedFrozen => {
                self.ui.render_state = RenderState::IdleThrottled;
                return true;
            }
        }
        false
    }

    /// Recreate the GPU texture if dimensions changed, then blend and upload.
    pub(crate) fn sync_gpu_texture(&mut self, frame: &mut eframe::Frame, ui: &mut egui::Ui) {
        if let Some(gpu) = &self.gpu_texture {
            let texture_size = gpu.texture.size();
            if texture_size.width != self.document.canvas.width
                || texture_size.height != self.document.canvas.height
            {
                self.recreate_gpu_texture(frame);
            }
        }

        let needs_blend = self.document.canvas.dirty_rect.needs_reblend();

        if self.gpu_texture.is_some() {
            if needs_blend {
                let dirty = self.document.blend_to_output();
                if let Some(ref gpu) = self.gpu_texture {
                    self.document
                        .upload_to_gpu(&gpu.queue, &gpu.texture, &dirty);
                }
            }
        } else if needs_blend || self.document.canvas.rendered_layers.is_none() {
            self.document.render_to_texture(ui);
        }
    }

    /// Recreate the wgpu GPU texture after a canvas resize.
    ///
    /// Uses `update_egui_texture_from_wgpu_texture` to keep the same
    /// `egui::TextureId`, avoiding stale entries in the renderer's map.
    ///
    /// If the canvas dimensions exceed the device's `max_texture_dimension_2d`,
    /// an error is pushed to `displayed_error_list` and the texture is not
    /// recreated (the old texture remains, now stale).
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the renderer lock cannot be acquired within
    /// 10 seconds (parking_lot deadlock detection). Panics if the wgpu device
    /// has been lost.
    pub(crate) fn recreate_gpu_texture(&mut self, frame: &mut eframe::Frame) {
        let Some(render_state) = frame.wgpu_render_state() else {
            return;
        };
        let Some(gpu) = &mut self.gpu_texture else {
            return;
        };
        let width = self.document.canvas.width;
        let height = self.document.canvas.height;
        let max_dim = render_state.device.limits().max_texture_dimension_2d;
        if width > max_dim || height > max_dim {
            self.ui.errors.list.push(format!(
                "Canvas too large for GPU: {width}×{height} exceeds device max \
                 texture dimension of {max_dim}. The display may be incomplete."
            ));
            return;
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        gpu.texture = render_state.device.create_texture(
            &(wgpu::TextureDescriptor {
                label: Some("splatter_iron_canvas"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }),
        );
        let view = gpu
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut renderer = render_state.renderer.write();
        renderer.update_egui_texture_from_wgpu_texture(
            &render_state.device,
            &view,
            wgpu::FilterMode::Linear,
            gpu.texture_id,
        );
    }

    /// Trigger an autosave if the canvas is dirty and enough time has elapsed.
    pub(crate) fn handle_autosave(&mut self) {
        if self.document.dirty_since_last_autosave
            && self
                .ui
                .time_elapsed
                .saturating_sub(self.ui.last_autosave_time)
                >= Duration::from_mins(AUTOSAVE_INTERVAL_MINUTES)
        {
            self.ui.last_autosave_time = self.ui.time_elapsed;
            self.ui.times_autosaved += 1;
            self.ui.progress = ProgressState::Autosaving;
            self.save_manager
                .trigger_async_save(&mut self.document, SaveKind::Autosave);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::AUTOSAVE_INTERVAL_MINUTES;
    use std::time::Duration;

    /// [`handle_autosave`] does nothing when canvas is not dirty.
    #[test]
    fn handle_autosave_skipped_when_not_dirty() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.document.dirty_since_last_autosave = false;
        app.handle_autosave();
        // No panic — autosave skipped.
    }

    /// [`handle_autosave`] triggers autosave when dirty and interval has elapsed.
    #[test]
    fn handle_autosave_triggers_when_dirty_and_elapsed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.document.dirty_since_last_autosave = true;
        app.ui.time_elapsed =
            Duration::from_mins(AUTOSAVE_INTERVAL_MINUTES) + Duration::from_secs(1);
        app.handle_autosave();
        assert!(app.ui.times_autosaved >= 1);
    }

    /// [`handle_autosave`] does not trigger when dirty but interval not elapsed.
    #[test]
    fn handle_autosave_skipped_when_not_enough_time() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.document.dirty_since_last_autosave = true;
        app.ui.time_elapsed = Duration::from_secs(1);
        app.handle_autosave();
        assert_eq!(app.ui.times_autosaved, 0);
    }

    /// [`handle_autosave`] does not panic when channels disconnected.
    #[test]
    fn handle_autosave_no_panic_with_unconnected_channels() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = crate::tests::common::create_test_app(dir.path().to_path_buf());
        app.document.dirty_since_last_autosave = true;
        app.ui.time_elapsed =
            Duration::from_mins(AUTOSAVE_INTERVAL_MINUTES) + Duration::from_secs(1);
        // trigger_async_save still works with no receiver.
        app.handle_autosave();
    }
}
