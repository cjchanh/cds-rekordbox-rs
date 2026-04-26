//! Per-track audio metadata extraction.
//!
//! Provides [`AudioMetadata`] (the unified struct) and [`read_id3`], which
//! parses ID3v2.3 / ID3v2.4 / Vorbis Comments / MP4 atoms from
//! `.mp3` / `.flac` / `.m4a` / `.aac` files via the `lofty` crate.
//!
//! Failure model: every parse error or malformed-tag condition returns
//! `AudioMetadata { source: None, .. }`. Panics are not allowed.

use std::path::Path;

use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::Accessor;
use lofty::tag::ItemKey;
use serde::{Deserialize, Serialize};

/// Source of the [`AudioMetadata`] record.
///
/// Ordered by trust priority (highest first) when records are merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSource {
    Rekordbox,
    Id3,
    Filename,
    None,
}

impl AudioSource {
    /// Higher number = more trusted.
    #[inline]
    pub fn priority(self) -> u8 {
        match self {
            AudioSource::Rekordbox => 3,
            AudioSource::Id3 => 2,
            AudioSource::Filename => 1,
            AudioSource::None => 0,
        }
    }
}

/// Unified per-track audio metadata.
///
/// All fields except `source` are optional. `bpm` is an `f32` because
/// rekordbox stores fractional BPMs. `initial_key` carries the key as
/// either a Camelot wheel code (`8A`) or a musical key (`Am`); we do not
/// normalize between the two.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub source: AudioSource,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub bpm: Option<f32>,
    pub initial_key: Option<String>,
    pub duration_ms: Option<u64>,
    pub bitrate_kbps: Option<u32>,
    pub sample_rate_hz: Option<u32>,
    /// True when at least one beat was parsed from a rekordbox ANLZ
    /// `BeatGrid`. `None` for non-rekordbox sources.
    pub beat_grid_present: Option<bool>,
    /// Count of hot cues recorded in a rekordbox ANLZ `CueList` /
    /// `ExtendedCueList`. `None` for non-rekordbox sources.
    pub hot_cue_count: Option<u32>,
}

impl AudioMetadata {
    /// Empty record with `source: None`.
    pub fn empty() -> Self {
        Self {
            source: AudioSource::None,
            title: None,
            artist: None,
            album: None,
            year: None,
            genre: None,
            bpm: None,
            initial_key: None,
            duration_ms: None,
            bitrate_kbps: None,
            sample_rate_hz: None,
            beat_grid_present: None,
            hot_cue_count: None,
        }
    }

    /// True when no field carries a non-null value (other than `source`).
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.year.is_none()
            && self.genre.is_none()
            && self.bpm.is_none()
            && self.initial_key.is_none()
            && self.duration_ms.is_none()
            && self.bitrate_kbps.is_none()
            && self.sample_rate_hz.is_none()
            && self.beat_grid_present.is_none()
            && self.hot_cue_count.is_none()
    }
}

/// Audio extensions that [`read_id3`] will probe.
const SUPPORTED_EXTS: &[&str] = &["mp3", "flac", "m4a", "aac"];

/// Merge a rekordbox-derived metadata record with an ID3-derived
/// record. Rekordbox is the canonical source for numeric fields it
/// carries (`bpm`, `duration_ms`, `beat_grid_present`, `hot_cue_count`).
/// ID3 fills in `title`, `artist`, `album`, `year`, `genre`,
/// `bitrate_kbps`, and `sample_rate_hz` — fields the ANLZ-only
/// rekordbox source does not provide.
///
/// When `rekordbox` is `None`, the ID3 record is returned verbatim.
/// When `id3.source == AudioSource::None`, only rekordbox values
/// populate the result.
pub fn merge(rekordbox: Option<AudioMetadata>, id3: AudioMetadata) -> AudioMetadata {
    let Some(rb) = rekordbox else {
        return id3;
    };
    let resolved_source = if rb.source == AudioSource::Rekordbox {
        AudioSource::Rekordbox
    } else {
        id3.source
    };
    AudioMetadata {
        source: resolved_source,
        title: id3.title,
        artist: id3.artist,
        album: id3.album,
        year: id3.year,
        genre: id3.genre,
        bpm: rb.bpm.or(id3.bpm),
        initial_key: rb.initial_key.or(id3.initial_key),
        duration_ms: rb.duration_ms.or(id3.duration_ms),
        bitrate_kbps: id3.bitrate_kbps,
        sample_rate_hz: id3.sample_rate_hz,
        beat_grid_present: rb.beat_grid_present,
        hot_cue_count: rb.hot_cue_count,
    }
}

/// Returns true when the path's extension is in [`SUPPORTED_EXTS`]
/// (case-insensitive). Caller is responsible for skipping AppleDouble
/// (`._*`) files; `read_id3` does not implement that filter.
pub fn is_supported_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            SUPPORTED_EXTS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

/// Read ID3 / Vorbis / MP4-atom tags from `path`.
///
/// Returns `AudioMetadata { source: Id3, .. }` when at least one tag is
/// present. Returns `AudioMetadata { source: None, .. }` when no tags
/// exist OR when the file is malformed. Never panics.
pub fn read_id3(path: &Path) -> AudioMetadata {
    if !is_supported_audio_path(path) {
        return AudioMetadata::empty();
    }
    let tagged_file = match lofty::read_from_path(path) {
        Ok(tf) => tf,
        Err(_) => return AudioMetadata::empty(),
    };

    let props = tagged_file.properties();
    let duration_ms = {
        let d = props.duration();
        if d.is_zero() {
            None
        } else {
            // .as_millis() returns u128; clamp to u64 (~5e8 years).
            Some(u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        }
    };
    let bitrate_kbps = props.audio_bitrate();
    let sample_rate_hz = props.sample_rate();

    let Some(tag) = tagged_file.primary_tag() else {
        // No tag block; still expose properties.
        return AudioMetadata {
            source: AudioSource::None,
            title: None,
            artist: None,
            album: None,
            year: None,
            genre: None,
            bpm: None,
            initial_key: None,
            duration_ms,
            bitrate_kbps,
            sample_rate_hz,
            beat_grid_present: None,
            hot_cue_count: None,
        };
    };

    let title = tag.title().map(|c| c.to_string());
    let artist = tag.artist().map(|c| c.to_string());
    let album = tag.album().map(|c| c.to_string());
    let year = tag.date().map(|ts| u32::from(ts.year)).or_else(|| {
        tag.get_string(ItemKey::Year)
            .and_then(|s| s.trim().get(..4).and_then(|y| y.parse::<u32>().ok()))
    });
    let genre = tag.genre().map(|c| c.to_string());

    // BPM: prefer fractional Bpm, fall back to IntegerBpm.
    let bpm = tag
        .get_string(ItemKey::Bpm)
        .and_then(|s| s.trim().parse::<f32>().ok())
        .or_else(|| {
            tag.get_string(ItemKey::IntegerBpm)
                .and_then(|s| s.trim().parse::<u32>().ok())
                .map(|n| n as f32)
        });
    let initial_key = tag.get_string(ItemKey::InitialKey).map(|s| s.to_string());

    // Determine whether any tag field carried a value; if not, treat as
    // unsourced (matches the spec's "no tags" branch).
    let has_tag_field = title.is_some()
        || artist.is_some()
        || album.is_some()
        || year.is_some()
        || genre.is_some()
        || bpm.is_some()
        || initial_key.is_some();

    AudioMetadata {
        source: if has_tag_field {
            AudioSource::Id3
        } else {
            AudioSource::None
        },
        title,
        artist,
        album,
        year,
        genre,
        bpm,
        initial_key,
        duration_ms,
        bitrate_kbps,
        sample_rate_hz,
        beat_grid_present: None,
        hot_cue_count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn audio_source_priority_orders_rekordbox_above_id3() {
        assert!(AudioSource::Rekordbox.priority() > AudioSource::Id3.priority());
        assert!(AudioSource::Id3.priority() > AudioSource::Filename.priority());
        assert!(AudioSource::Filename.priority() > AudioSource::None.priority());
    }

    #[test]
    fn audio_metadata_empty_round_trip_serde() {
        let m = AudioMetadata::empty();
        let s = serde_json::to_string(&m).expect("serialize empty");
        let back: AudioMetadata = serde_json::from_str(&s).expect("deserialize empty");
        assert_eq!(back.source, AudioSource::None);
        assert!(back.is_empty());
    }

    #[test]
    fn is_supported_audio_path_handles_case_and_unknowns() {
        assert!(is_supported_audio_path(&PathBuf::from("a.mp3")));
        assert!(is_supported_audio_path(&PathBuf::from("a.MP3")));
        assert!(is_supported_audio_path(&PathBuf::from("a.flac")));
        assert!(is_supported_audio_path(&PathBuf::from("a.m4a")));
        assert!(is_supported_audio_path(&PathBuf::from("a.AAC")));
        assert!(!is_supported_audio_path(&PathBuf::from("a.wav")));
        assert!(!is_supported_audio_path(&PathBuf::from("a")));
    }

    #[test]
    fn read_id3_unsupported_extension_returns_empty() {
        let path = PathBuf::from("/nonexistent/file.txt");
        let m = read_id3(&path);
        assert_eq!(m.source, AudioSource::None);
        assert!(m.is_empty());
    }

    #[test]
    fn read_id3_missing_path_returns_empty_no_panic() {
        let path = PathBuf::from("/nonexistent/file.mp3");
        let m = read_id3(&path);
        assert_eq!(m.source, AudioSource::None);
    }

    #[test]
    fn read_id3_malformed_data_returns_empty_no_panic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("garbage.mp3");
        std::fs::write(&path, b"not an mp3, just some bytes").expect("write");
        let m = read_id3(&path);
        assert_eq!(m.source, AudioSource::None);
    }

    #[test]
    fn merge_returns_id3_when_rekordbox_absent() {
        let id3 = AudioMetadata {
            source: AudioSource::Id3,
            title: Some("Track A".into()),
            artist: Some("Artist".into()),
            bpm: Some(128.0),
            ..AudioMetadata::empty()
        };
        let merged = merge(None, id3.clone());
        assert_eq!(merged.source, AudioSource::Id3);
        assert_eq!(merged.title.as_deref(), Some("Track A"));
        assert_eq!(merged.bpm, Some(128.0));
    }

    #[test]
    fn merge_prefers_rekordbox_for_bpm_and_duration_keeps_id3_strings() {
        let rb = AudioMetadata {
            source: AudioSource::Rekordbox,
            bpm: Some(140.0),
            duration_ms: Some(300_000),
            beat_grid_present: Some(true),
            hot_cue_count: Some(3),
            ..AudioMetadata::empty()
        };
        let id3 = AudioMetadata {
            source: AudioSource::Id3,
            title: Some("Track".into()),
            artist: Some("DJ".into()),
            album: Some("Album".into()),
            year: Some(2024),
            genre: Some("DnB".into()),
            bpm: Some(139.5),
            duration_ms: Some(299_500),
            bitrate_kbps: Some(320),
            sample_rate_hz: Some(44_100),
            ..AudioMetadata::empty()
        };
        let merged = merge(Some(rb), id3);
        assert_eq!(merged.source, AudioSource::Rekordbox);
        assert_eq!(merged.bpm, Some(140.0)); // rekordbox wins
        assert_eq!(merged.duration_ms, Some(300_000));
        assert_eq!(merged.beat_grid_present, Some(true));
        assert_eq!(merged.hot_cue_count, Some(3));
        // ID3-only fields preserved.
        assert_eq!(merged.title.as_deref(), Some("Track"));
        assert_eq!(merged.artist.as_deref(), Some("DJ"));
        assert_eq!(merged.album.as_deref(), Some("Album"));
        assert_eq!(merged.year, Some(2024));
        assert_eq!(merged.genre.as_deref(), Some("DnB"));
        assert_eq!(merged.bitrate_kbps, Some(320));
        assert_eq!(merged.sample_rate_hz, Some(44_100));
    }

    #[test]
    fn merge_handles_rekordbox_without_id3_tags() {
        let rb = AudioMetadata {
            source: AudioSource::Rekordbox,
            bpm: Some(175.0),
            beat_grid_present: Some(true),
            ..AudioMetadata::empty()
        };
        let empty_id3 = AudioMetadata::empty();
        let merged = merge(Some(rb), empty_id3);
        assert_eq!(merged.source, AudioSource::Rekordbox);
        assert_eq!(merged.bpm, Some(175.0));
        // No ID3 -> string fields stay None.
        assert!(merged.title.is_none());
        assert!(merged.artist.is_none());
    }
}
