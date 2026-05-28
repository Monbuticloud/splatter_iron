# main

Application entry point: installs the MiMalloc global allocator, resolves
the platform data directory, and launches the eframe GUI event loop.

## `fn main() -> eframe::Result`

`main` is the single entry point. It does three things:

1. **Resolve the platform data directory.** Uses `directories::ProjectDirs`
   to find the OS-appropriate location for application data (e.g.
   `~/.local/share/splatter_iron/` on Linux, `~/Library/Application
   Support/splatter_iron/` on macOS). Panics if the home directory cannot be
   found.
2. **Create the data directory.** Calls `create_dir_all` to ensure the path
   exists, creating any missing parents. Panics if the filesystem refuses
   (e.g. permissions).
3. **Launch the GUI.** Calls `eframe::run_native` with the app name, default
   native options, and a closure that constructs `app::MyApp`. Ownership of
   `MyApp` (and all application state — `Document`, `ToolConfig`,
   `UndoHistory`, `FileIO`, `UIState`) is transferred to the egui event
   loop. The call is blocking and returns only when the window is closed.

The function returns `eframe::Result`, which propagates any error from window
creation (missing display server, unsupported OpenGL version, etc.).
