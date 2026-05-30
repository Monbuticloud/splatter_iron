# debug

Debug utilities for development builds.

Provides `debug_snapshot` which dumps application state via `dbg!`
when compiled in debug mode.  No-ops in release builds.

## `debug_snapshot`

```rust
pub fn debug_snapshot(app: &MyApp)
```

Dump a snapshot of the application state using `dbg!`.

### Parameters

| Parameter | Type    | Purpose                      |
| --------- | ------- | ---------------------------- |
| `app`     | `&MyApp`| The application state to inspect |

### Notes

- Only active in debug builds (`#[cfg(debug_assertions)]`).
- In release builds the function body compiles to a no-op.
- Panics only via the inner `dbg!` macro (no additional panics).
