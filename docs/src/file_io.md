# file_io

Async file I/O orchestration split into four sub-modules, each responsible for
a distinct subsystem. File dialogs run on background threads via `rfd` to avoid
macOS winit re-entrancy panics. Save/load/export operations clone the canvas and
serialise on background threads, sending results back to the UI thread through
mpsc channels.

## Submodules

| Module                | Purpose                                                                                                                                              |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `dialog_manager`      | Queues native file dialogs (`rfd`) on background threads and dispatches results (`PendingFileAction` → `DialogResult` → `DispatchedAction`).         |
| `save_manager`        | Async save orchestration: manual saves and periodic autosaves (`SaveKind` → `SaveResult`).                                                           |
| `load_import_manager` | Async load/import orchestration: deserialises `.splattercanvas` files, imports images, and imports `.splatterarchive` archives (`LoadImportResult`). |
| `export_manager`      | Async export orchestration: image encoding and archive serialisation via `ExportStrategy`.                                                           |

## Channel Topology

```
UI Thread                          Background Thread(s)
─────────                          ────────────────────
dialog_manager                          ┌─ rfd dialog thread
  ├─ pending_file_action ───────►       │   (opens native dialog,
  │                              │      │    sends DialogResult)
  │                              │      └────────────────►
  ◄──── dialog_receiver ─────────┘
       (poll_dialog_results)

save_manager
  ├─ trigger_async_save ──────────────► save thread
  │                                    (clones canvas, serialises)
  ◄──── save_result_receiver ──────────┘
       (poll_save_results)

load_import_manager
  ├─ trigger_async_load ──────────────► load thread
  ├─ trigger_async_import ────────────► import thread
  │                                    (deserialises / decodes)
  ◄──── load_import_receiver ──────────┘
       (poll_load_import_results)

export_manager
  ├─ trigger_async_export ────────────► export thread
  │                                    (encodes image)
  ◄──── export_result_receiver ────────┘
       (poll_export_results)
```

## Recent Files

The `file_io` module also manages recent-file tracking via `push_recent_file`
(mutates `persistence.rs`'s config store), exposed through the top-right
"Recent" context menu in the UI.
