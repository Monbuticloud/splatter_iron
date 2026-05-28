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

### Import Extensions (`IMPORT_EXTENSIONS`)

A flat list of 19 file extensions accepted by the image-import dialog:

`avif`, `png`, `jpg`, `jpeg`, `webp`, `gif`, `tiff`, `tif`, `tga`, `ico`,
`pnm`, `pgm`, `ppm`, `pbm`, `pam`, `qoi`, `exr`, `hdr`, `ff`

These cover all raster image formats supported by the `image` crate, including
legacy formats (TGA, ICO, PNM variants) and HDR/EXR for high-dynamic-range
workflows. The list is used to build the file-type filter shown in native OS
file-open dialogs.
