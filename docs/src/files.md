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

---

## `save_canvas_to_bytes(canvas)`

Serialises a `Canvas` to zstd-compressed JSON bytes without writing to disk.
Uses multi-threaded (zstdmt) compression at level 10. This is intentionally
split from `save_bytes_to_file` so that the CPU-heavy compression can be
offloaded to a background thread.

### Signature

```rust
pub fn save_canvas_to_bytes(canvas: &Canvas) -> anyhow::Result<Vec<u8>>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `canvas` | `&Canvas` | The canvas to serialise |

### Errors

Returns an error if:
- JSON serialisation of the `Canvas` fails (should not happen in practice since
  `Canvas` derives `Serialize` and contains only plain data).
- Zstd compression fails (e.g. out of memory).

### Performance notes

- Compression level `10` provides a good trade-off between file size and speed
  for `.splattercanvas` files.
- The thread count is set to `available_parallelism()`, falling back to 1 if
  the system cannot report the number of hardware threads.
- The encoder uses zstdmt for multi-threaded compression.

---

## `save_bytes_to_file(data, path)`

Writes pre-serialised bytes to a file. A pure I/O operation with no
computation. Designed to be called on the main thread (or any thread) after
`save_canvas_to_bytes` has completed compression on a background thread.

### Signature

```rust
pub fn save_bytes_to_file(data: &[u8], path: &Path) -> anyhow::Result<()>
```

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `data` | `&[u8]` | Pre-serialised bytes (e.g. from `save_canvas_to_bytes`) |
| `path` | `&Path` | Destination file path |

### Errors

Returns an error if:
- The parent directory does not exist.
- The calling process lacks write permission.
- An I/O error occurs during the write (disk full, etc.).
