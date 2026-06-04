# Check formatting
fmt:
    cargo fmt --check

# Run clippy (warn-only, consistent with Cargo.toml lint config)
clippy:
    cargo clippy --all-targets

# Run tests (uses nextest for JUnit output in CI)
test:
    cargo nextest run

# Test with debug-snapshot feature
test-debug:
    cargo test --features debug-snapshot

# Build release
build-release:
    cargo build --release

# Build docs (deny warnings on pub items without docs)
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Generate HTML coverage report using cargo-llvm-cov
coverage:
    cargo llvm-cov --all-features --workspace --html

# Check cargo-deny advisories, bans, licenses
deny:
    cargo deny check

# Check cargo-audit for security advisories
audit:
    cargo audit

# Full CI pipeline (runs sequentially; stops at first failure)
ci: fmt clippy test test-debug build-release doc deny audit
