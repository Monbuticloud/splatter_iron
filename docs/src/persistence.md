# persistence

Config persistence: save/load tool configuration and recent files.

## Methods

| Method                | Purpose                                                       |
| --------------------- | ------------------------------------------------------------- |
| `push_recent_file()`  | Add a file path to the recent-files list (dedup, max 10)      |
| `config_path()`       | Path to the user-config JSON file (`{data_dir}/config.json`)  |
| `save_config()`       | Persist `ToolConfiguration` + recent files to disk as JSON    |
| `handle_config_save()`| Timer-driven config save (runs on same 2-minute cadence as autosave) |

All methods are `pub(crate)` and used from `app/frame.rs` and `ui/dialogs.rs`.
