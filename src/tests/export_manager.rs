//! Tests for [`ExportManager`] — async export orchestration, result polling.

use std::sync::Arc;

use crate::file_io::ExportManager;
use crate::files::DefaultExportStrategy;

/// Create an [`ExportManager`] with a default strategy.
fn create_export_manager() -> ExportManager {
    ExportManager::new(Arc::new(DefaultExportStrategy))
}

// --- poll_export_results ---

#[test]
fn poll_export_results_success_returns_true() {
    let mut em = create_export_manager();
    let mut errors = Vec::new();

    em.export_result_sender
        .send(Ok(()))
        .expect("send success");
    assert!(em.poll_export_results(&mut errors));
    assert!(errors.is_empty());
    assert!(!em.export_in_flight);
}

#[test]
fn poll_export_results_error_appends() {
    let mut em = create_export_manager();
    let mut errors = Vec::new();

    let error: anyhow::Result<()> = Err(anyhow::anyhow!("export failed"));
    em.export_result_sender
        .send(error)
        .expect("send error");
    assert!(em.poll_export_results(&mut errors));
    assert!(errors.iter().any(|e| e.contains("export failed")));
}

#[test]
fn poll_export_results_no_message_returns_false() {
    let mut em = create_export_manager();
    let mut errors = Vec::new();

    assert!(!em.poll_export_results(&mut errors));
    assert!(errors.is_empty());
}
