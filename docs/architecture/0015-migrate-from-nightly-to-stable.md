# ADR 15: Migrate from Nightly to Stable Rust

- **Status:** Accepted
- **Date:** 2026-05-29

## Context

The project was initially configured for nightly Rust for two reasons:

1. **`edition = "2024"`** — When the project started, the 2024 edition was
   nightly-only. Rust 1.85.0 (February 2025) stabilized the 2024 edition,
   making it available on stable.
2. **`build-std = ["std"]`** — Building the standard library from source,
   controlled via the `[unstable]` table in `.cargo/config.toml`. This
   feature has never left nightly and shows no signs of stabilization.

The nightly requirement added friction:
- Every contributor must run `rustup override set nightly` manually.
- Editors, CI, and tooling (rust-analyzer, etc.) must target nightly.
- The `[unstable]` table and `build-std` key are silently ignored by stable
   cargo, causing confusing build failures for anyone who skipped the
   override step.
- No `rust-toolchain.toml` existed, so the nightly toolchain was only
   documented in `AGENTS.md` — easy to miss.

### Audit of nightly usage

A thorough audit of all source files and configuration confirmed that the
project uses **zero** `#![feature(...)]` attribute gates, zero nightly-only
library features, and zero nightly crate dependencies. All nightly reliance
was purely in the build configuration layer.

The `lib/` directory (Zig `compiler_rt`, gitignored) was present only to
support `build-std` cross-compilation. Without `build-std`, it is dead
weight.

## Decision

Drop all dependence on nightly Rust:

1. **Remove `build-std`** — Delete the `[unstable]` section and
   `build-std = ["std"]` from `.cargo/config.toml`.
2. **Remove global rustflags** — The unconditional `-C opt-level=3`,
   `-C debuginfo=0`, and `-C target-feature=+crt-static` flags in
   `.cargo/config.toml` overrode per-profile settings in `Cargo.toml`.
   These are removed; `Cargo.toml`'s `[profile.*]` sections now control
   optimization and debug info exclusively.
3. **Pin stable toolchain** — A `rust-toolchain.toml` file pins the
   channel to `1.96.0` (the latest stable at time of migration). This
   replaces the undocumented `rustup override set nightly`.
4. **Update MSRV** — `clippy.toml` MSRV raised from `1.85.0` to `1.96.0`
   to match the pinned toolchain.

`edition = "2024"` in `Cargo.toml` and `rustfmt.toml` is retained — it has
been stable since Rust 1.85.0.

## Consequences

- **Positive:** Build requires zero manual toolchain setup.
  `rustup run` / `cargo build` works out of the box.
- **Positive:** Editors and CI pick up `rust-toolchain.toml` automatically.
- **Positive:** Simpler `.cargo/config.toml` — no more confusing
  `[unstable]` table or global overrides.
- **Positive:** Faster clean builds — compiling `std` from source is no
  longer required.
- **Positive:** Cross-compilation becomes simpler — use `rustup target add`
  instead of providing Zig `compiler_rt`.
- **Negative:** Slightly less control over standard library optimization.
  The pre-built `std` from the toolchain distribution is already compiled
  with optimizations, so the practical impact is negligible.
- **Negative:** `+crt-static` is no longer available. On macOS this had no
  effect; cross-compilation to Windows MSVC targets may need alternative
  CRT linkage strategies (typically handled by the target's default linker
  configuration).
