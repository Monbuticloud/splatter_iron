//! Async file-IO subsystem: dialog, save, export, and load/import managers.
//!
//! Each manager owns its own mpsc channel pair and state flags, and is
//! constructed independently. The frame loop orchestrates them explicitly.

mod dialog_manager;
mod export_manager;
mod load_import_manager;
mod save_manager;

pub use dialog_manager::DialogManager;
#[cfg(test)]
pub use dialog_manager::DialogResult;
pub use dialog_manager::DispatchedAction;
pub use dialog_manager::PendingFileAction;
pub use export_manager::ExportManager;
pub use load_import_manager::LoadImportManager;
#[cfg(test)]
pub use load_import_manager::LoadImportResult;
pub use save_manager::SaveKind;
pub use save_manager::SaveManager;
#[cfg(test)]
pub use save_manager::SaveResult;
