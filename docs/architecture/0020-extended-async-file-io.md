# ADR 20: Extended Async File IO

- **Status:** Accepted
- **Date:** 2026-06-01

## Context

ADR 8 introduced a `FileIO` struct with 6 fields and 2 mpsc channel pairs
(dialog + save-result) to handle load, save, autosave, and export
operations asynchronously. Since then, the file-IO subsystem grew to
support:

- Stamp image loading (`LoadStamp`)
- Brush file loading (`LoadBrush`, `.abr`/`.gbr`)
- Archive export/import (`.splatterarchive`, xz-compressed)
- Dedicated load/import result channels
- Export strategy abstraction for 13 image formats
- In-flight tracking for concurrent operations

The original 2-channel design could not cleanly handle the divergence
between save results, export results, and load/import results.

## Decision

The `FileIO` struct now has 18 fields (up from 6) and 4 channel pairs:

### Fields

| # | Field | Purpose |
|---|---|---|
| 1 | `pending_file_action` | Currently queued dialog action |
| 2–3 | `dialog_sender` / `dialog_receiver` | File-dialog results |
| 4–5 | `save_result_sender` / `save_result_receiver` | Async save outcomes |
| 6 | `app_local_data_directory` | Base path for autosaves |
| 7 | `loaded_stamp_data` | Decoded stamp image (UI-consumed) |
| 8 | `loaded_brush_data` | Parsed brush tips (UI-consumed) |
| 9 | `export_strategy` | Pluggable `ExportStrategy` impl |
| 10–11 | `export_result_sender` / `export_result_receiver` | Export outcomes |
| 12 | `export_in_flight` | Export thread running flag |
| 13–14 | `load_import_sender` / `load_import_receiver` | Load/import data |
| 15 | `load_in_flight` | Load thread running flag |
| 16 | `import_in_flight` | Import thread running flag |
| 17 | `autosave_in_flight` | Autosave-in-progress flag (UI status) |
| 18 | `archive_autosave_in_flight` | Archive autosave flag |

### Channel topology

```
dialog     → [background thread] → dialog    (file paths / cancellations)
save       → [background thread] → save      (SaveResult enum)
export     → [background thread] → export    (anyhow::Result<()>)
load_import → [background thread] → load_import (LoadImportResult enum)
```

### Public API (16 methods)

| Method | Role |
|---|---|
| `new()` | Construct with channel pairs, data dir, export strategy |
| `autosave_directory()` | Return the `{data_dir}/autosaves/` path |
| `queue_file_action()` | Spawn background dialog thread for any action |
| `queue_load_direct()` | Load a path without showing a dialog |
| `poll_dialog_results()` | Process completed dialogs (per frame) |
| `trigger_async_save()` | Serialize and write canvas on background thread |
| `save_to_current_path()` | Save to current `savefile_path` if set |
| `trigger_async_autosave_archive()` | Archive autosave (xz) on background thread |
| `trigger_async_export()` | Export image via strategy on background thread |
| `trigger_async_load()` | Read and deserialize `.splattercanvas` |
| `trigger_async_import()` | Decode image file as new canvas |
| `trigger_async_export_archive()` | One-shot xz archive export |
| `trigger_async_import_archive()` | Read and deserialize `.splatterarchive` |
| `poll_load_import_results()` | Apply completed load/import results |
| `poll_export_results()` | Check for completed export |
| `poll_save_results()` | Update document state from save results |

### PendingFileAction (8 variants)

Load, Save, Import, Export(usize), LoadStamp, LoadBrush, ExportArchive,
ImportArchive — up from ADR 8's 3 (Load, Save, Import).

### SaveResult (4 variants)

Autosave, ManualSave(PathBuf), ArchiveAutosave, Failed(String).

### LoadImportResult (4 variants)

Loaded(Canvas, String), Imported(Vec\<Layer\>, u32, u32),
ArchiveImported(Canvas), Failed(String).

## Consequences

- **Positive:** Separate channels prevent head-of-line blocking — an export
  result arriving late doesn't delay load/import processing.
- **Positive:** Stamp and brush loading are integrated into the same
  dialog/result pipeline without special-casing in the app frame.
- **Positive:** The `ExportStrategy` trait (injected via constructor) makes
  the system testable without real image encoding — mock strategies can
  verify the flow.
- **Positive:** Archive autosave runs on its own timer alongside regular
  autosave, giving two independent save streams for redundancy.
- **Negative:** 18 fields and 16 methods make `FileIO` a large struct —
  testing requires constructing many channel pairs and flags.
- **Negative:** Four separate `poll_*` calls must be invoked each frame
  in the correct order (dialog → load_import → export → save), a
  caller-ordering invariant not captured in the type system.
- **Negative:** The `LoadImportResult` enum conflates three operation types
  (load, import, archive-import) into one channel, requiring runtime
  discrimination at the poll site.
