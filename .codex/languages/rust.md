# Rust guidance

Run the Rust quality gate from the repository root:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo build --release --locked
```

Use `cargo fmt --all` separately to apply formatting. Keep integration tests under `tests/` with observable contracts, local HTTP mocks, and virtual-time checks for TTL/delay; do not use real SwitchBot calls or performance measurements for the Issue #60 migration.

Runtime configuration, metrics, API access, HTTP handling, and CLI are separate modules. Update the migration guide when public behavior changes.
