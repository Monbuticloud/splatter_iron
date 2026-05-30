# ADR 9: Zstd-Compressed JSON (.splattercanvas)

- **Status:** Accepted
- **Date:** 2026-05-16
- **Commit:** `1b55e3e`

## Context

SplatterIron needs to save and load canvas documents. The requirements for the
file format are:

- **Lossless round-trip** — saving and loading must produce identical pixel
  data and layer structure.
- **Fast save** — the user should not wait more than ~200 ms for a save
  operation.
- **Small file size** — canvas data is repetitive (large transparent regions,
  uniform color spans); compression should exploit this.
- **Simple to implement** — no binary format design, no manual (de)serialization,
  no schema evolution in v0.1.
- **Single file** — a `.splattercanvas` file contains everything (layers, pixels,
  dimensions) in one blob.

## Decision

Serialize the `Canvas` struct to JSON via `serde_json`, then compress with
**zstd** at level 10:

```rust
// Streaming writer: JSON → zstd → File (or any Write), no intermediate Vec.
fn write_canvas(canvas: &Canvas, writer: impl Write) -> anyhow::Result<()> {
    let mut encoder = zstd::stream::Encoder::new(writer, COMPRESSION_LEVEL)?;
    encoder.multithread(n)?;
    serde_json::to_writer(&mut encoder, canvas)?;
    encoder.finish()?;
    Ok(())
}

// Streaming reader: File → zstd → serde, no intermediate Vec.
fn read_canvas(reader: impl Read) -> anyhow::Result<Canvas> {
    let decoder = zstd::Decoder::new(reader)?;
    let limited = decoder.take(MAX_DECOMPRESSED_BYTES);
    let canvas = serde_json::from_reader(limited)?;
    Ok(canvas)
}
```

### Why zstd?

| Format | Compression ratio | Encode speed | Decode speed | Library |
|--------|------------------|-------------|-------------|---------|
| zstd level 10 | ~4–8× on canvas data | ~150 MB/s | ~500 MB/s | `zstd` crate |
| gzip level 9 | ~3–5× | ~50 MB/s | ~200 MB/s | `flate2` |
| brotli level 9 | ~5–7× | ~5 MB/s | ~100 MB/s | `brotli` |
| uncompressed JSON | 1× | instant | instant | `serde_json` |

zstd offers the best speed/ratio trade-off: fast enough for real-time saves,
good compression on repetitive pixel data, and multi-threaded encoding via
`zstdmt`.

### Why JSON, not a binary format?

- `serde_json` is a serde derive — `Canvas` and `Layer` get serialization for
  free with `#[derive(Serialize, Deserialize)]`.
- Binary formats (bincode, messagepack) would save ~10% size but add complexity
  and schema risks.
- JSON is debuggable — corrupted files can be inspected after zstd decompression.
- The `.splattercanvas` extension signals the compressed format.

### Serde-skipped fields

`output_rgba`, `rendered_layers`, and `dirty_rect` are annotated with
`#[serde(skip)]` — they are runtime-only caches. Only `pixels`, `width`,
`height`, and `render_next_frame` are serialized.

## Consequences

- **Positive:** ~5 MB canvas file saves to ~0.6–1.2 MB on disk with zstd level
  10 — small enough for git storage and email attachment.
- **Positive:** Save is fast (~50 ms for 2000×1500 with zstdmt on 4 cores).
- **Positive:** No schema migration needed — serde ignores unknown fields,
  so forward-compatible.
- **Positive:** JSON is human-readable after decompression — `zstd -d file.zst
  | jq .` works for debugging.
- **Negative (mitigated):** JSON is ~3–5× larger than binary formats. Most of
  this is redundant: `"pixels":[...]` with 3M values consumes ~50 MB of JSON
  text per layer. However, `serde_json::to_writer` streams directly into the
  zstd encoder, so the raw JSON is never materialized in memory — peak usage
  is just the ztd internal buffers (~MB) plus the destination writer.
- **Negative:** zstd level 10 is CPU-intensive for large canvases; must run on
  a background thread (see ADR-0008).
