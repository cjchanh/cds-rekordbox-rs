
# 0001 - Add PDB Public Accessors And Docs

## Motivation

Spec 52 AC-03 deferred PDB-derived metadata until `rekordcrate` exposes a read-only public surface:

> PDB-derived fields (title, artist, album, genre, year, rating, play_count) are **explicitly out of scope** for this spec. `rekordcrate-0.3.0::pdb::Track` exposes zero `pub` fields; the only license-compatible alternative `rbox-0.1.7` is GPL-3.0-only; `PIONEER/rekordbox/exportLibrary.db` is SQLCipher-encrypted. PDB scope moves to a follow-on spec gated on (a) upstream rekordcrate PR adding `pub fn` accessors, (b) local vendored fork, or (c) custom `binrw` PDB parser.

This patch adds read-only accessors without changing binary layout, parser behavior, or write behavior.

## Exact Changes

- `Album`: `id`, `artist_id`, `name`.
- `Artist`: `id`, `name`.
- `Artwork`: `id`, `path`.
- `Genre`: `id`, `name`.
- `Key`: `id`, `name`.
- `Label`: `id`, `name`.
- `Track`: `id`, `artist_id`, `album_id`, `genre_id`, `key_id`, `artwork_id`, `sample_rate`, `bitrate`, `tempo_centi_bpm`, `play_count`, `year`, `duration_seconds`, `rating`, `title`, `filename`, `file_path`, `comment`, `analyze_path`.
- Every public accessor has a one-line `///` doc comment matching the existing upstream documentation style.

## Patch Hash

- BLAKE3: `c944a4891540395871a72c8e3242ffe44f1a5e56ac1e067fa751abcacb4e3458`

## Verification

Upstream-ready copy excluded downstream-only warning overrides and was validated in debug mode because upstream's `deny(warnings)` gate is release-only.

| Command | Result | Elapsed | Notes |
|---|---|---:|---|
| `cargo build` | PASS | 5s | Succeeded with existing upstream warnings only. |
| `cargo test` | PASS | 3s | 123 tests passed including doc test. |

Key output:

```text
warning: unnecessary parentheses around type
warning: enum `StarRating` is never used
warning: `rekordcrate` (lib) generated 10 warnings (run `cargo fix --lib -p rekordcrate` to apply 9 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.75s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
warning: unnecessary parentheses around type
warning: unnecessary parentheses around type
warning: unnecessary parentheses around type
warning: unnecessary parentheses around type
warning: unnecessary parentheses around type
warning: unnecessary parentheses around type
warning: unnecessary parentheses around type
warning: unnecessary parentheses around type
warning: unnecessary parentheses around type
warning: 9 warnings emitted
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
```
