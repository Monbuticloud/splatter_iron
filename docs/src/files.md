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
