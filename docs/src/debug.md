# debug

Debug utilities gated behind the `debug-snapshot` feature.

Provides `debug_snapshot` which dumps application state via `dbg!`.
Only compiled when `feature = "debug-snapshot"` is enabled.

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

- Feature flag: `debug-snapshot`.
- Panics only via the inner `dbg!` macro (no additional panics).
