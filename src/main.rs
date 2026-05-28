// Copyright (C) 2026 Nguyen Hoang Quoc Anh
// Alias: Mon, Monbuticloud
// SPDX-License-Identifier: AGPL-3.0-only

mod app;
mod canvas;
mod document;
mod file_io;
mod files;
mod pixel;
mod tool_config;
mod tools;
mod ui;
mod undo;
mod undo_history;

#[cfg(test)]
mod tests;
use mimalloc::MiMalloc;

use crate::app::{ APP_QUALIFIER, APP_ORG, APP_NAME };

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
fn main() -> eframe::Result {
    let project_dirs = directories::ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME).expect(
        "Couldn't resolve app dir"
    );
    let data_dir = project_dirs.data_local_dir().to_path_buf();
    std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

    eframe::run_native(
        APP_NAME,
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(app::MyApp::new(cc))))
    )
}
