//! Per-track metadata extraction from Pioneer rekordbox device exports.
//!
//! Walks `PIONEER/USBANLZ/<P*>/<id>/ANLZ0000.DAT` files under a given
//! device-export root and parses each via [`rekordcrate::anlz::ANLZ`].
//! For every successfully-parsed file, extracts:
//!
//! - the embedded track path (PPTH section, `Content::Path`)
//! - BPM from the BeatGrid (median of `Beat.tempo` entries, divided by
//!   100.0 to undo rekordbox's centi-BPM encoding)
//! - duration in milliseconds from the last `Beat.time`
//! - `beat_grid_present` true when at least one beat parsed
//! - `hot_cue_count` from the `CueList` / `ExtendedCueList`
//!
//! ANLZ-only scope: title, artist, album, genre, year, rating, and
//! play_count live in `PIONEER/rekordbox/export.pdb` whose `Track`
//! struct exposes zero `pub` fields in `rekordcrate-0.3`. Those
//! fields are explicitly out of scope for spec 52 — see the spec's
//! AC-03 deferral note.
//!
//! Read-only and deterministic. A parse failure on a single ANLZ file
//! is silently skipped; the function never panics.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use binrw::BinRead;
use rekordcrate::anlz::{Content, ANLZ};
use walkdir::WalkDir;

use cds_audio_metadata_rs::{AudioMetadata, AudioSource};

const ANLZ_FILENAME: &str = "ANLZ0000.DAT";

/// Errors returned by [`read_anlz_file`].
#[derive(Debug, thiserror::Error)]
pub enum AnlzReadError {
    /// Failed to open the file.
    #[error("io error reading anlz: {0}")]
    Io(#[from] std::io::Error),
    /// `binrw` failed to parse the ANLZ structure.
    #[error("anlz parse error: {0}")]
    Parse(#[from] binrw::Error),
}

/// Walk `<root>/PIONEER/USBANLZ/` recursively and parse every
/// `ANLZ0000.DAT` file found.
///
/// Returns a map keyed by the absolute path of the audio file the
/// ANLZ refers to (anchored against `root`). Files that fail to parse,
/// or that contain no `Path` section, are silently skipped.
///
/// Returns an empty map when `<root>/PIONEER/USBANLZ` does not exist
/// (the input is not a Pioneer device export).
pub fn read_device_export(root: &Path) -> HashMap<PathBuf, AudioMetadata> {
    let mut out: HashMap<PathBuf, AudioMetadata> = HashMap::new();
    let usbanlz_root = root.join("PIONEER").join("USBANLZ");
    if !usbanlz_root.is_dir() {
        return out;
    }

    for entry in WalkDir::new(&usbanlz_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.file_name().and_then(|s| s.to_str()) != Some(ANLZ_FILENAME) {
            continue;
        }
        let Ok(Some((rel_path, metadata))) = read_anlz_file(p) else {
            continue;
        };
        let absolute = anchor_track_path(root, &rel_path);
        out.insert(absolute, metadata);
    }
    out
}

/// Parse a single ANLZ0000.DAT file.
///
/// Returns `Ok(Some((rel_path, metadata)))` when the file contains a
/// `Content::Path` section.  Returns `Ok(None)` when parsing succeeded
/// but no path section was found (we have nothing to anchor the
/// metadata to).  Returns `Err` on I/O or `binrw` parse failure.
pub fn read_anlz_file(path: &Path) -> Result<Option<(PathBuf, AudioMetadata)>, AnlzReadError> {
    let mut reader = File::open(path)?;
    let anlz = ANLZ::read(&mut reader)?;

    let mut track_path: Option<PathBuf> = None;
    let mut tempos: Vec<u16> = Vec::new();
    let mut last_beat_time: Option<u32> = None;
    let mut hot_cue_count: u32 = 0;
    let mut beat_grid_present = false;

    for section in &anlz.sections {
        match &section.content {
            Content::Path(p) => {
                track_path = Some(PathBuf::from(p.path.to_string()));
            }
            Content::BeatGrid(bg) => {
                if !bg.beats.is_empty() {
                    beat_grid_present = true;
                }
                for b in &bg.beats {
                    tempos.push(b.tempo);
                    last_beat_time = Some(b.time);
                }
            }
            Content::CueList(cl) => {
                for c in &cl.cues {
                    if c.hot_cue > 0 {
                        hot_cue_count = hot_cue_count.saturating_add(1);
                    }
                }
            }
            Content::ExtendedCueList(ecl) => {
                for c in &ecl.cues {
                    if c.hot_cue > 0 {
                        hot_cue_count = hot_cue_count.saturating_add(1);
                    }
                }
            }
            _ => {}
        }
    }

    let Some(rel_path) = track_path else {
        return Ok(None);
    };

    let bpm = median_bpm(&tempos);
    let duration_ms = last_beat_time.map(u64::from);

    let metadata = AudioMetadata {
        source: AudioSource::Rekordbox,
        title: None,
        artist: None,
        album: None,
        year: None,
        genre: None,
        bpm,
        initial_key: None,
        duration_ms,
        bitrate_kbps: None,
        sample_rate_hz: None,
        beat_grid_present: if beat_grid_present { Some(true) } else { None },
        hot_cue_count: if hot_cue_count == 0 {
            None
        } else {
            Some(hot_cue_count)
        },
    };

    Ok(Some((rel_path, metadata)))
}

/// Median of centi-BPM tempos divided by 100, rounded to two decimals.
/// Returns `None` for an empty slice.
fn median_bpm(tempos: &[u16]) -> Option<f32> {
    if tempos.is_empty() {
        return None;
    }
    let mut sorted: Vec<u16> = tempos.to_vec();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    let median_centi = if sorted.len().is_multiple_of(2) {
        (u32::from(sorted[mid - 1]) + u32::from(sorted[mid])) / 2
    } else {
        u32::from(sorted[mid])
    };
    let raw = (median_centi as f32) / 100.0;
    // round to 2 decimals so reports / TSVs match the human-readable
    // pyrekordbox output (e.g. 175.96, 177.86) rather than emitting
    // f32 rounding noise.
    Some((raw * 100.0).round() / 100.0)
}

/// Resolve a rekordbox-relative track path (typically `/Contents/...`)
/// against the device-export `root`.
///
/// Pioneer encodes the path as a UTF-16 string starting with a leading
/// slash ("/Contents/UnknownArtist/..."). We treat that leading slash
/// as a separator anchor, not a filesystem absolute path.
fn anchor_track_path(root: &Path, rel: &Path) -> PathBuf {
    let s = rel.to_string_lossy();
    let trimmed = s.strip_prefix('/').unwrap_or(&s);
    root.join(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn read_device_export_empty_when_no_pioneer_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let map = read_device_export(dir.path());
        assert!(map.is_empty());
    }

    #[test]
    fn read_device_export_empty_when_no_anlz_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("PIONEER").join("USBANLZ")).unwrap();
        let map = read_device_export(dir.path());
        assert!(map.is_empty());
    }

    #[test]
    fn read_anlz_file_returns_err_on_garbage_input() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("ANLZ0000.DAT");
        std::fs::write(&p, b"not an anlz file at all").unwrap();
        let res = read_anlz_file(&p);
        assert!(
            res.is_err(),
            "expected parse error on garbage; got {:?}",
            res
        );
    }

    #[test]
    fn read_anlz_file_returns_io_err_on_missing() {
        let p = PathBuf::from("/nonexistent/ANLZ0000.DAT");
        let res = read_anlz_file(&p);
        assert!(matches!(res, Err(AnlzReadError::Io(_))));
    }

    #[test]
    fn anchor_track_path_strips_leading_slash() {
        let root = PathBuf::from("/Volumes/USB");
        let rel = PathBuf::from("/Contents/Artist/track.mp3");
        let anchored = anchor_track_path(&root, &rel);
        assert_eq!(
            anchored,
            PathBuf::from("/Volumes/USB/Contents/Artist/track.mp3")
        );
    }

    #[test]
    fn anchor_track_path_handles_no_leading_slash() {
        let root = PathBuf::from("/Volumes/USB");
        let rel = PathBuf::from("Contents/track.mp3");
        let anchored = anchor_track_path(&root, &rel);
        assert_eq!(anchored, PathBuf::from("/Volumes/USB/Contents/track.mp3"));
    }

    #[test]
    fn median_bpm_empty_slice_returns_none() {
        assert_eq!(median_bpm(&[]), None);
    }

    #[test]
    fn median_bpm_single_tempo_undoes_centi_encoding() {
        // 12999 centi-BPM = 129.99 BPM
        assert_eq!(median_bpm(&[12_999]), Some(129.99));
    }

    #[test]
    fn median_bpm_odd_count_returns_middle_value() {
        // sorted: 12000, 12500, 13000 → middle 12500 → 125.00
        assert_eq!(median_bpm(&[13_000, 12_000, 12_500]), Some(125.00));
    }

    #[test]
    fn median_bpm_even_count_averages_two_middle_values() {
        // sorted: 12000, 12500, 13000, 13500 → middle pair (12500, 13000) → 12750 → 127.50
        assert_eq!(median_bpm(&[13_000, 12_000, 12_500, 13_500]), Some(127.50));
    }

    #[test]
    fn median_bpm_steady_track_returns_exact_value() {
        let tempos = vec![14_000_u16; 100];
        assert_eq!(median_bpm(&tempos), Some(140.00));
    }
}
