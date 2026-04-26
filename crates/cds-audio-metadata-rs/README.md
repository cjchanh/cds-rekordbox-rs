# cds-audio-metadata-rs

## Table of Contents

- Description
- Install
- Example
- What It Does Not Do
- License
- Origin

## Description

Small Rust library for reading local audio metadata and merging ID3-style tags with higher-trust rekordbox-derived fields.

## Install

```bash
cargo add cds-audio-metadata-rs
```

## Example

```rust
use cds_audio_metadata_rs::{read_id3, AudioMetadata};

let meta: AudioMetadata = read_id3("/music/track.mp3".as_ref());
println!("source={:?} bpm={:?}", meta.source, meta.bpm);
```

## What It Does Not Do

- It does not write tags.
- It does not call network services.
- It does not infer missing title/artist fields from filenames.
- It does not parse Pioneer PDB databases.

## License

Planned license: MIT OR Apache-2.0.

## Origin

Extracted from Archivist Sort `crates/sort-core/src/audio_metadata.rs`.
