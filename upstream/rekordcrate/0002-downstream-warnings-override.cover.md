
# 0002 - Downstream Warning Overrides

## NOT For Upstream

This patch is vendored-only. It exists so the staged `rekordcrate-0.3.0` copy can pass `cargo build --release` and `cargo test --release` under the current Rust toolchain while preserving upstream's crate-level `deny(warnings)` policy.

## Motivation

`modular-bitfield` macro expansion produces `unused_parens` warnings that fire outside the source-level struct attribute scope. A module-level allow is the narrow practical downstream override. The `StarRating` enum is an existing unused private type in `src/xml.rs`; the local allow keeps release-mode verification deterministic.

## Exact Changes

- `src/anlz.rs`: add module-level `#![allow(unused_parens)]` with a downstream-only explanation.
- `src/anlz.rs`: keep per-struct TODO comments on the three affected bitfield structs as a future upstream roadmap.
- `src/xml.rs`: keep `#[allow(dead_code)]` on `StarRating` with a downstream-only TODO comment.

## Patch Hash

- BLAKE3: `4b19c4b35115e43612e07648da2687d123c60db0ec3b5f1f467178da3fff9bcb`

## Verification

| Command | Result | Elapsed | Notes |
|---|---|---:|---|
| `cargo build --release` | PASS | 1s | No warnings emitted in release mode. |
| `cargo test --release` | PASS | 3s | 123 tests passed including doc test. |

Key output:

```text
    Finished `release` profile [optimized] target(s) in 1.80s
    Finished `release` profile [optimized] target(s) in 1.78s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
```
