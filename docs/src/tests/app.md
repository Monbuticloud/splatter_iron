# tests/app

Tests for application-level constants, UI state defaults, and export format
metadata.

## `ui_state_default_values`

Default `UIState` should have `IdleThrottled` render state, zero elapsed time,
no autosaves, no pending layer deletion, no pending stamp name, no toast
message, and default canvas size 2000×1500.

## `pending_stamp_construction`

PendingStamp can be constructed with pixel data and dimensions; verifies width, height, name, and pixel count.

## `export_formats_all_have_extensions`

All EXPORT_FORMATS entries should have at least one extension.

## `export_formats_formats_are_distinct`

EXPORT_FORMATS should reference distinct image::ImageFormat variants.

## `export_information_extensions`

Each ExportInformation should have a valid image::ImageFormat with at least one non-empty extension string.

## `import_extensions_non_empty`

IMPORT_EXTENSIONS should contain common image file extensions (png, jpg, jpeg, webg, gif).

## `export_formats_entries_accessible`

ExportInformation struct can be read from EXPORT_FORMATS list entries.
