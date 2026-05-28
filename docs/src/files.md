# files

## `load_data_from_file(path)`

Reads the raw bytes of a file from disk. A thin wrapper around
`std::fs::read(path)` that serves as the I/O entry point for both loading
canvas files and importing images.

### Signature

```rust
pub fn load_data_from_file(path: &Path) -> Result<Vec<u8>, std::io::Error>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `path` | `&Path` | Filesystem path to read |

### Errors

Returns `std::io::Error` if:
- The file does not exist at `path`.
- The calling process lacks read permission.
- The path points to a directory or a non-regular file.
- An I/O error occurs during the read.

---

## `load_app_from_data(data)`

Deserialises a `Canvas` from zstd-compressed JSON bytes. This is the
deserialisation counterpart to `save_canvas_to_bytes` and handles both
decompression and JSON parsing.

### Signature

```rust
pub fn load_app_from_data(data: &[u8]) -> anyhow::Result<Canvas>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `data` | `&[u8]` | Zstd-compressed JSON bytes produced by `save_canvas_to_bytes` |

### Errors

Returns an error if:
- The input is not valid zstd-compressed data.
- The decompressed bytes are not valid UTF-8 JSON.
- The JSON structure does not match the `Canvas` type (e.g. missing fields,
  wrong field types).
