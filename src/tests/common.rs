//! Shared test helpers: small canvases, premultiplied-color shorthands.
//!
//! Used by all other test modules to reduce boilerplate when constructing
//! predictable input state.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use eframe::egui::Color32;

use crate::app::MyApp;
use crate::app::UIState;
use crate::asset_library::Library;
use crate::canvas::Canvas;
use crate::canvas::DirtyRectList;
use crate::canvas::Layer;
use crate::document::Document;
use crate::file_io::DialogManager;
use crate::file_io::ExportManager;
use crate::file_io::LoadImportManager;
use crate::file_io::SaveManager;
use crate::files::DefaultExportStrategy;
use crate::tool_configuration::ToolConfiguration;
use crate::undo_history::UndoHistory;

/// Build a `MyApp` rooted at `data_dir` so persistence methods write
/// into the temporary directory. The returned `TempDir` must stay alive
/// for the lifetime of the app.
pub fn create_test_app(data_dir: PathBuf) -> MyApp {
    let canvas = Canvas::new(10, 10);
    let pixel_count = (canvas.width * canvas.height) as usize;
    let (dialog_tx, dialog_rx) = mpsc::channel();
    let (save_tx, save_rx) = mpsc::channel();

    MyApp {
        document: Document::new(canvas),
        tool_configuration: ToolConfiguration::default(),
        undo: UndoHistory::new(pixel_count),
        dialog_manager: DialogManager::new(dialog_tx, dialog_rx),
        save_manager: SaveManager::new(save_tx, save_rx, data_dir.clone()),
        export_manager: ExportManager::new(Arc::new(DefaultExportStrategy)),
        load_import_manager: LoadImportManager::new(),
        ui: UIState::default(),
        gpu_texture: None,
        stamp_library: Library::load_from_disk(&data_dir),
        brush_library: Library::load_from_disk(&data_dir),
    }
}

/// Build a 10×10 single-layer transparent canvas for use in tests.
///
/// # Returns
///
/// A pre-built `Canvas` with one fully transparent layer at 10×10 resolution.
pub fn small_canvas() -> Canvas {
    Canvas {
        pixels: vec![Layer {
            pixels: vec![Color32::TRANSPARENT; 100],
            ..Default::default()
        }],
        height: 10,
        width: 10,
        output_rgba: Arc::new(Vec::new()),
        rendered_layers: None,
        dirty_rect: DirtyRectList::new(),
    }
}

/// Shortcut for a fully opaque red color in premultiplied format.
///
/// # Returns
///
/// `Color32::from_rgba_premultiplied(255, 0, 0, 255)`.
pub fn red() -> Color32 {
    Color32::from_rgba_premultiplied(255, 0, 0, 255)
}

/// Shortcut for a fully opaque blue color in premultiplied format.
///
/// # Returns
///
/// `Color32::from_rgba_premultiplied(0, 0, 255, 255)`.
pub fn blue() -> Color32 {
    Color32::from_rgba_premultiplied(0, 0, 255, 255)
}
