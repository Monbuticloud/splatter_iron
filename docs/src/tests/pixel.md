# tests::pixel

Tests for the premultiplied-alpha pixel pipeline: `premultiply`, `unpremultiply`, `alpha_blend`, `blend_layers`, and `blend_region`.

## Test strategy

Each compositing primitive is exercised with opaque, transparent, and semi-transparent inputs to verify correct fixed-point arithmetic. Round-trip tests confirm that `premultiply ∘ unpremultiply` stays within ±1 per channel (the tolerances of the `(val * 255 + 128) >> 8` approximation).

## `premultiply_opaque_is_identity`

An already-premultiplied opaque color passes through unchanged.

## `premultiply_transparent_is_unchanged`

`Color32::TRANSPARENT` is invariant under premultiply.

## `premultiply_non_opaque_approximation`

After premultiplying a straight-alpha color, each RGB channel is ≤ its original value (since it is scaled by α/255).

## `unpremultiply_produces_straight_alpha`

Inverse of premultiply: recovers the straight channel value via `channel * 255 / α`.

## `premultiply_unpremultiply_roundtrip_close`

Fixed-point round-trip stays within ±1 per channel.

## `unpremultiply_opaque_is_identity`

An opaque straight-alpha color is unchanged by unpremultiply.

## `premultiply_zero_alpha`

A zero-alpha straight color becomes `TRANSPARENT` after premultiply.

## `unpremultiply_zero_alpha_stays_transparent`

`TRANSPARENT` is invariant under unpremultiply.

## `alpha_blend_opaque_over_transparent`

Blending opaque source over transparent destination yields the source unchanged.

## `alpha_blend_transparent_over_dest_is_close`

Blending transparent source over opaque destination leaves the destination within ±1 per channel.

## `alpha_blend_semi_transparent_over_opaque`

A 50%-opaque red source blended over an opaque green destination yields an opaque mixed result.

## `blend_layers_single_layer_copy`

With one layer, the output RGBA bytes are a direct copy of the layer's premultiplied pixel array.

## `blend_layers_two_layers_opaque`

With two opaque layers, the top layer fully occludes the bottom across every pixel.

## `blend_region_single_layer_matches_full`

`blend_region` on a sub-rectangle produces the same output as `blend_layers` inside the rect and leaves pixels outside untouched (zero-initialised).

## `blend_region_two_layers`

`blend_region` with two layers matches `blend_layers` inside the rect; pixels outside remain zero.

## `blend_region_empty_layers_no_panic`

Calling `blend_region` with an empty layer list does not panic and leaves the output buffer unchanged.

## `blend_layers_three_layers`

With three opaque layers (red → green → blue), the topmost (blue) fully occludes the rest at every pixel.

## `blend_layers_semi_transparent_top`

A semi-transparent red top layer blended over an opaque green bottom layer produces an opaque red-tinted result; both red and green channels are positive, with red dominating.

## Regression: `premultiply_of_premultiplied_darkens_again`

Documents the bug pattern: calling `premultiply` on an already-premultiplied `Color32` double-scales the RGB channels, producing a darker color (e.g. 50% transparent red drops from r=128 to r=64).

## Regression: `premultiply_on_premul_storage_darkens`

`Color32` always stores premultiplied bytes internally. Calling `premultiply` on a value from `from_rgba_unmultiplied` (which already performed the conversion) darkens it further — the same class of bug that affected the original brush code.

## `blend_layers_empty_layers_panics`

Calling blend_layers with an empty layer vec panics (invariant violation).

## `blend_layers_mismatched_lengths_panics`

blend_layers with layers of different pixel counts panics.
