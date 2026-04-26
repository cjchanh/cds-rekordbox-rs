# cds-anlz-rs

## Table of Contents

- Description
- Install
- Example
- What It Does Not Do
- License
- Origin

## Description

Read-only parser for Pioneer rekordbox device-export ANLZ files under `PIONEER/USBANLZ`, returning per-track path, BPM, duration, beat-grid presence, and hot-cue count.

## Install

```bash
cargo add cds-anlz-rs
```

## Example

```rust
use cds_anlz_rs::read_device_export;

let tracks = read_device_export("/Volumes/DJ_USB".as_ref());
for (path, metadata) in tracks {
    println!("{} {:?}", path.display(), metadata.bpm);
}
```

## What It Does Not Do

- It does not write to rekordbox libraries.
- It does not move or rename audio files.
- It does not parse `PIONEER/rekordbox/export.pdb` yet.
- It does not recover title, artist, genre, rating, or play count from PDB.

## License

Planned license for CDS wrapper code: MIT OR Apache-2.0. Depends on `rekordcrate`, which is MPL-2.0.

## Origin

Extracted from Archivist Sort `crates/sort-core/src/rekordbox.rs`.
