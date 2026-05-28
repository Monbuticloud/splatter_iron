# ADR 8: Async File IO via mpsc Channels

- **Status:** Accepted
- **Date:** 2026-05-18
- **Commits:** `2efcd21`, `c3597f1`

## Context

File operations (load, save, export) involve native OS dialogs and disk I/O
that can block the UI thread for hundreds of milliseconds to seconds. In
egui's immediate-mode loop, blocking the `ui()` method freezes the entire
application — no repaint, no input processing, no animation.

Additionally, macOS winit panics if native file dialogs (`rfd`) are opened
inside a window event handler. The dialog must be spawned between frames, not
during UI layout.

Initial attempts ran dialogs and saves inline with `todo!()` or synchronous
`rfd` calls, causing both the freeze and the macOS panic.

## Decision

Implement an asynchronous file-IO system using **two mpsc channels**:

```
┌─────────────────────────────────────────────────────┐
│ UI Thread (eframe event loop)                        │
│                                                      │
│  1. queue_file_action() → spawns background thread   │
│     that opens rfd dialog                            │
│  2. poll_dialog_results() → try_recv() at frame start│
│     - on path received: trigger async save or load   │
│  3. poll_save_results() → try_recv() at frame start  │
│     - on result: update savefile_path / error_list    │
│                                                      │
│  Channels:                                           │
│    dialog_sender     → background → dialog_receiver   │
│    save_result_sender ← background ← save_result_recv  │
└─────────────────────────────────────────────────────┘
```

### `FileIO` struct

```rust
pub struct FileIO {
    pending_file_action: Option<PendingFileAction>,
    dialog_sender: mpsc::Sender<DialogResult>,
    dialog_receiver: mpsc::Receiver<DialogResult>,
    save_result_sender: mpsc::Sender<SaveResult>,
    save_result_receiver: mpsc::Receiver<SaveResult>,
    app_local_data_directory: PathBuf,
}
```

### Flow

1. User clicks "Save" → `queue_file_action(Save)` spawns thread that runs
   `rfd::FileDialog::new().save_file()` and sends result via `dialog_sender`.
2. Next frame, `poll_dialog_results()` receives the path, spawns a second
   thread that serializes the canvas (zstd-compressed JSON) and writes to disk.
3. Following frame, `poll_save_results()` receives `SaveResult::ManualSave(path)`
   and updates the document's `savefile_path`.

### Why mpsc channels instead of async/await?

- Egui/eframe doesn't natively integrate with `tokio` or `async` runtimes.
- `mpsc::try_recv()` is non-blocking and cheap — called once per frame.
- Background threads are short-lived and spawned per operation; no thread pool
  management needed.
- Zero dependency overhead beyond `std::sync::mpsc`.

## Consequences

- **Positive:** UI never blocks on file operations — the frame loop continues
  repainting while saves happen in the background.
- **Positive:** macOS winit re-entrancy panic avoided — dialogs open between
  frames, not during `ui()`.
- **Positive:** Autosave integrated naturally — `trigger_async_save()` on a
  2-minute timer, same channel mechanism as manual saves.
- **Positive:** Error handling is explicit — `SaveResult::Failed(String)` is
  displayed in the error overlay window, not lost to stderr.
- **Negative:** Two channels + two polling calls per frame adds ~1–2 μs overhead.
- **Negative:** The canvas is cloned for each async save (`document.canvas.clone()`)
  — a full 12 MB copy for 2000×1500 with layers before background work begins.
- **Negative:** No way to cancel an in-flight save operation.
