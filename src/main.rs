// Copyright (C) 2026 Nguyen Hoang Quoc Anh
// Alias: Mon, Monbuticloud
// SPDX-License-Identifier: AGPL-3.0-only

mod app;
mod canvas;
mod pixel;
mod ui;
mod undo;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
// Unstable MiMalloc features can cause build issues, so we'll stick to the default allocator for now.
// Never mind its just windows that has issues, linux and mac are fine. I'll just add a note about it in the readme (maybe) and leave it as is for now.

fn main() -> eframe::Result {
    eframe::run_native(
        "SplatterIron",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(app::MyApp::default())))
    )
}
