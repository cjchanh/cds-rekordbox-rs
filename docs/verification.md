# Verification

Run from the repository root:

```bash
scripts/green_check.sh
```

Current local gate:

- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- `cargo build --workspace --release`: PASS
- `cargo test --workspace --release`: PASS, 25 tests including doc-test gates

The package is read-only tooling. Do not commit private DJ USB paths, copied media, rekordbox database exports, or generated build artifacts.

