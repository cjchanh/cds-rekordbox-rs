# cds-rekordbox-rs

Read-only Rust tools for inspecting rekordbox device-export audio metadata without mutating a DJ USB.

## Crates

- `cds-audio-metadata-rs`: reads local audio tags and merges them with higher-trust rekordbox-derived fields.
- `cds-anlz-rs`: walks `PIONEER/USBANLZ/**/ANLZ0000.DAT` and returns typed per-track metadata such as BPM, duration, beat-grid presence, and hot-cue count.

## Install

```bash
cargo add cds-anlz-rs
```

For local workspace use:

```bash
cargo build --workspace --release
cargo test --workspace --release
```

## 30-Second Usage

```rust
use cds_anlz_rs::read_device_export;

let tracks = read_device_export("/Volumes/DJ_USB".as_ref())?;
for (path, metadata) in tracks {
    println!("{} bpm={:?}", path.display(), metadata.bpm);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## What It Does Not Do

- It does not write to rekordbox libraries.
- It does not move, rename, delete, or retag audio files.
- It does not parse `PIONEER/rekordbox/export.pdb` yet.
- It does not publish any sample DJ USB data.

## Upstream Work

The `upstream/rekordcrate/` directory contains a prepared accessor patch for `rekordcrate` 0.3.0. The upstream-ready patch is separate from the vendored-only warnings override.

## Origin

Extracted from Archivist Sort after a read-only validation run on a working DJ USB: 740 tracks sorted into additive copies, originals untouched, rekordbox library byte-identical.

## License

MIT for CDS code in this repository. `cds-anlz-rs` depends on `rekordcrate`, which is MPL-2.0.

