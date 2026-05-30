// Copyright (C) 2026 Nguyen Hoang Quoc Anh
// Alias: Mon, Monbuticloud
// SPDX-License-Identifier: AGPL-3.0-only

//! Application entry point: installs the MiMalloc global allocator, resolves
//! the platform data directory, and launches the eframe GUI event loop.

mod debug;
mod app;
mod asset_library;
mod brush_library;
mod brush_params;
mod canvas;
mod document;
mod file_io;
mod files;
mod persistence;
mod pixel;
mod stamp_library;
mod tool_configuration;
mod tools;
mod ui;
mod undo;
mod undo_history;

#[cfg(test)]
mod tests;
use std::sync::Arc;

use eframe::egui_wgpu::wgpu;
use mimalloc::MiMalloc;

use crate::app::APP_NAME;
use crate::app::APP_ORGANIZATION;
use crate::app::APP_QUALIFIER;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/// Application entry point.
///
/// Initializes the local data directory, creates the autosave directory,
/// and runs the eframe GUI event loop.
///
/// # Errors
///
/// Returns an error if the data directory cannot be resolved or created,
/// or if the eframe window cannot be opened.
///
/// # Panics
///
/// Panics if the platform-specific data directory cannot be resolved
/// (no home directory) or if the operating system refuses to create the
/// data directory (e.g., file-system permissions).
fn main() -> eframe::Result {
    let project_dirs = directories::ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
        .expect("Couldn't resolve app dir");
    let data_dir = project_dirs.data_local_dir().to_path_buf();
    std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

    use eframe::egui_wgpu::WgpuConfiguration;

    eframe::run_native(
        APP_NAME,
        eframe::NativeOptions {
            wgpu_options: WgpuConfiguration {
                wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew(
                    eframe::egui_wgpu::WgpuSetupCreateNew {
                        device_descriptor: Arc::new(|_adapter: &wgpu::Adapter| {
                            wgpu::DeviceDescriptor {
                                label: Some("splatter_iron_device"),
                                required_limits: wgpu::Limits {
                                    max_texture_dimension_2d: 16384,
                                    ..wgpu::Limits::default()
                                },
                                ..Default::default()
                            }
                        }),
                        ..eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle()
                    },
                ),
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(app::MyApp::new(cc)))),
    )
}
