//! Integration smoke test for [`sort_core::rekordbox`] against a real
//! Pioneer ANLZ0000.DAT fixture.
//!
//! The fixture was extracted from a working DJ USB and contains the
//! standard tag layout (PPTH, PVBR, PQTZ, PWAV, PWV2, PCOB, PCOB).
//! Reference values were confirmed via pyrekordbox before commit:
//!
//! - PPTH path: `/Contents/UnknownArtist/UnknownAlbum/coat check.mp3`
//! - PQTZ beat count: 126
//! - PQTZ median tempo: 77.40 BPM
//! - PQTZ last beat time: 97_273 ms
//! - PCOB hot cues: 0
//!
//! Run with `cargo test --package sort-core --test rekordbox_smoke`.

use std::path::{Path, PathBuf};

use cds_anlz_rs as rekordbox;
use cds_audio_metadata_rs::AudioSource;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("anlz")
        .join("sample.dat")
}

#[test]
fn parses_real_anlz_fixture_and_extracts_path_and_bpm() {
    let path = fixture_path();
    assert!(path.is_file(), "fixture missing: {}", path.display());

    let parsed =
        rekordbox::read_anlz_file(&path).expect("real anlz fixture should parse without error");
    let (track_path, metadata) = parsed.expect("fixture has a Path content section");

    // Track path comes from the PPTH section verbatim.
    assert_eq!(
        track_path,
        PathBuf::from("/Contents/UnknownArtist/UnknownAlbum/coat check.mp3"),
        "track path should match PPTH content"
    );

    assert_eq!(metadata.source, AudioSource::Rekordbox);

    // BPM is the median of 126 beat tempos divided by 100.
    let bpm = metadata
        .bpm
        .expect("BPM should be set when BeatGrid present");
    assert!(
        (76.0..=79.0).contains(&bpm),
        "expected BPM near 77.40, got {bpm}"
    );

    // Duration comes from the last Beat.time field; sample is ~97 s.
    let duration = metadata
        .duration_ms
        .expect("duration_ms should be set when BeatGrid present");
    assert_eq!(duration, 97_273);

    assert_eq!(metadata.beat_grid_present, Some(true));
    // No hot cues recorded in this sample → field stays None per the
    // module's "0 → None" encoding.
    assert!(metadata.hot_cue_count.is_none());

    // Rekordbox source carries no title/artist/album/bitrate/sample_rate
    // — those fields require PDB scope (deferred).
    assert!(metadata.title.is_none());
    assert!(metadata.artist.is_none());
    assert!(metadata.album.is_none());
    assert!(metadata.bitrate_kbps.is_none());
    assert!(metadata.sample_rate_hz.is_none());
}

#[test]
fn read_device_export_walks_synthetic_pioneer_layout() {
    // Build a temp directory mimicking the Pioneer device tree:
    //   <root>/PIONEER/USBANLZ/P000/00000001/ANLZ0000.DAT
    let dir = tempfile::tempdir().expect("tempdir");
    let target_dir = dir
        .path()
        .join("PIONEER")
        .join("USBANLZ")
        .join("P000")
        .join("00000001");
    std::fs::create_dir_all(&target_dir).expect("create dirs");
    std::fs::copy(fixture_path(), target_dir.join("ANLZ0000.DAT"))
        .expect("copy fixture into synthetic tree");

    let map = rekordbox::read_device_export(dir.path());
    assert_eq!(map.len(), 1, "should find exactly the one ANLZ");

    let expected_key = dir
        .path()
        .join("Contents")
        .join("UnknownArtist")
        .join("UnknownAlbum")
        .join("coat check.mp3");
    let metadata = map
        .get(&expected_key)
        .unwrap_or_else(|| panic!("expected key {} missing", expected_key.display()));

    assert_eq!(metadata.source, AudioSource::Rekordbox);
    assert!(metadata.bpm.is_some());
    assert_eq!(metadata.beat_grid_present, Some(true));
}

#[test]
fn read_device_export_skips_non_anlz_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target_dir = dir
        .path()
        .join("PIONEER")
        .join("USBANLZ")
        .join("P000")
        .join("00000001");
    std::fs::create_dir_all(&target_dir).expect("create dirs");
    // Drop the real ANLZ alongside an EXT/2EX which the walker should ignore.
    std::fs::copy(fixture_path(), target_dir.join("ANLZ0000.DAT")).unwrap();
    std::fs::write(target_dir.join("ANLZ0000.EXT"), b"unrelated").unwrap();
    std::fs::write(target_dir.join("ANLZ0000.2EX"), b"unrelated").unwrap();

    let map = rekordbox::read_device_export(dir.path());
    assert_eq!(map.len(), 1, "only ANLZ0000.DAT should be parsed");
}

#[test]
fn read_device_export_skips_corrupt_anlz_silently() {
    let dir = tempfile::tempdir().expect("tempdir");
    let good = dir
        .path()
        .join("PIONEER")
        .join("USBANLZ")
        .join("P000")
        .join("00000001");
    let bad = dir
        .path()
        .join("PIONEER")
        .join("USBANLZ")
        .join("P000")
        .join("00000002");
    std::fs::create_dir_all(&good).unwrap();
    std::fs::create_dir_all(&bad).unwrap();

    std::fs::copy(fixture_path(), good.join("ANLZ0000.DAT")).unwrap();
    std::fs::write(bad.join("ANLZ0000.DAT"), b"definitely not an anlz file").unwrap();

    let map = rekordbox::read_device_export(dir.path());
    // The good one must land; the bad one must not panic the walker.
    assert_eq!(map.len(), 1, "corrupt sibling should be skipped silently");
}

#[test]
fn read_device_export_returns_empty_for_non_pioneer_root() {
    let dir = tempfile::tempdir().expect("tempdir");
    let map = rekordbox::read_device_export(dir.path());
    assert!(map.is_empty());
}

fn _ensure_path_helpers_compile(_: &Path) {
    // Static check that the module's public API surface stays stable;
    // forces a compile-time reference to the symbols we depend on.
    let _ = rekordbox::AnlzReadError::Io as fn(_) -> _;
}
