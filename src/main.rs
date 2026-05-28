// Copyright (C) 2026 Nguyen Hoang Quoc Anh
// Alias: Mon, Monbuticloud
// SPDX-License-Identifier: AGPL-3.0-only

//! Application entry point: installs the MiMalloc global allocator, resolves
//! the platform data directory, and launches the eframe GUI event loop.

mod app;
mod canvas;
mod document;
mod file_io;
mod files;
mod pixel;
mod tool_configuration;
mod tools;
mod ui;
mod undo;
mod undo_history;

#[cfg(test)]
mod tests;
use mimalloc::MiMalloc;

use crate::app::{ APP_QUALIFIER, APP_ORGANIZATION, APP_NAME };

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
    let project_dirs = directories::ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME).expect(
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
