# ADR 11: RenderState Power Management

- **Status:** Accepted
- **Date:** 2026-05-17
- **Commits:** `f7fb2ef`, `5d4f3c4`

## Context

A paint program does not need to repaint at 60 fps when the user is idle or
when the window is in the background. Continuous full-frame rendering on an
unfocused window wastes GPU cycles, drains laptop batteries, and generates
heat.

Conversely, during active brush strokes, every frame matters — the brush
preview must track the cursor smoothly, and the canvas must update within one
frame of the stroke being applied.

## Decision

Introduce a `RenderState` enum with three states:

```rust
pub enum RenderState {
    /// Full rendering — canvas redraws every frame (active interaction).
    ActiveWake(Duration),
    /// Slow repainting — frames still run but canvas repainting is throttled.
    IdleThrottled,
    /// No rendering — viewport is unfocused, all GPU work suspended.
    UnfocusedFrozen,
}
```

### State transitions

```
                    mouse hover / drag
     ┌──────────────────────────────────────────────┐
     │                                              ▼
IdleThrottled ──────────────────►  ActiveWake(550ms)
     ▲                                              │
     │            timer expires                     │
     └──────────────────────────────────────────────┘

UnfocusedFrozen ──► IdleThrottled ──► (on focus return)
     │
     └── viewport unfocused → sleep 50ms → return
```

- **`ActiveWake(Duration)`**: Set to `ActiveWake(550ms)` whenever the cursor
  hovers over or drags on the canvas. Each frame decrements the duration by
  `predicted_delta_time`. When it reaches zero, transitions to `IdleThrottled`.
- **`IdleThrottled`**: Calls `ui.request_repaint_after(predicted_dt × 5)` to
  reduce repaint frequency from 60 fps to ~12 fps. The app still responds to
  events but with lower GPU load.
- **`UnfocusedFrozen`**: When the viewport is not focused, the `ui()` method
  sleeps for 50 ms and returns immediately without rendering anything. On the
  next frame (triggered by focus gain), transitions to `IdleThrottled`.

### Why not a single bool?

- A bool (`is_active`) doesn't encode the timeout duration.
- A timer countdown in the UI struct is cleaner than comparing timestamps.
- Three states model the three distinct power regimes: active (full speed),
  idle (throttled), and frozen (zero rendering).

## Consequences

- **Positive:** ~0% GPU usage when unfocused (50 ms sleep = 20 fps polling with
  zero rendering work).
- **Positive:** ~1.2 W GPU power saving in idle state (throttled repaint) on a
  typical laptop.
- **Positive:** Immediate responsiveness on hover — `ActiveWake` triggers full
  repaint within 550 ms of any interaction.
- **Negative:** The `predicted_delta_time × 5` multiplier is a heuristic; on
  very fast displays (240 Hz), the idle repaint may still be unnecessarily
  frequent.
- **Negative:** The 50 ms sleep in `UnfocusedFrozen` adds latency to the first
  frame after refocus (worst case 50 ms).
- **Negative:** Timer decrement uses `predicted_delta_time` which can spike
  after a stall, causing premature transition to idle.
