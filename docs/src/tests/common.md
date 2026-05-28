# tests::common

Shared test helpers that reduce boilerplate across every test module that constructs canvas state or uses non-black pixel colors.

## `fn small_canvas() -> Canvas`

Builds a minimal 10×10 single-layer canvas initialised entirely to `Color32::TRANSPARENT`. Used by almost every brush and fill test to avoid repeating struct-literal construction.

## `fn red() -> Color32`

Shorthand for `Color32::from_rgba_premultiplied(255, 0, 0, 255)`. Returns a fully opaque premultiplied red.

## `fn blue() -> Color32`

Shorthand for `Color32::from_rgba_premultiplied(0, 0, 255, 255)`. Returns a fully opaque premultiplied blue.
