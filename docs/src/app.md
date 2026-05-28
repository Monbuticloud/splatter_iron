# app

Top-level application constants, export-format registry, UI state, GPU texture
management, and the main `MyApp` struct that wires together the document, tool
configuration, undo history, and file-IO subsystems for eframe.

## Constants

### App Identity

Three reverse-domain constants identify the application to the OS for platform
data-directory resolution via `directories::ProjectDirs`:

| Constant | Value |
|---|---|
| `APP_QUALIFIER` | `"com"` |
| `APP_ORGANIZATION` | `"Monbuticloud"` |
| `APP_NAME` | `"SplatterIron"` |

`ProjectDirs::from("com", "Monbuticloud", "SplatterIron")` resolves to
a platform-specific path such as `~/.local/share/SplatterIron` on Linux or
`~/Library/Application Support/com.Monbuticloud.SplatterIron` on macOS.

### Canvas File-Format Constants

| Constant | Value | Purpose |
|---|---|---|
| `CANVAS_EXTENSION` | `".splattercanvas"` | Extension for native canvas files (zstd-compressed JSON) |
| `FILE_FILTER_NAME` | `"SplatterCanvas"` | File-dialog filter label displayed in OS pickers |
| `DEFAULT_CANVAS_NAME` | `"canvas.splattercanvas"` | Default save-file name when no path has been set |
