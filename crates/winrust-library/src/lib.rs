//! Local music library for winrust.
//!
//! The library scans one or more folders, reads the tags of every audio file
//! it finds and keeps the result in a flat list that the interface can
//! display and filter.

use std::path::{Path, PathBuf};

use lofty::prelude::*;
use lofty::probe::Probe;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use winrust_core::Track;

/// The file extensions that are considered audio.
pub const AUDIO_EXTENSIONS: [&str; 20] = [
    "mp3", "flac", "ogg", "oga", "opus", "m4a", "mp4", "aac", "wav", "wave", "wma", "aiff", "aif",
    "webm", "mka", "wv", "ape", "mpc", "m3u", "m3u8",
];

/// Errors raised while scanning a folder.
#[derive(Debug, Error)]
pub enum LibraryError {
    /// The folder could not be read.
    #[error("could not read {path}: {source}")]
    Read {
        /// The folder that failed.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// A track of the local library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalTrack {
    /// The absolute path of the file.
    pub path: PathBuf,
    /// The title of the track.
    pub title: String,
    /// The artist of the track.
    pub artist: String,
    /// The album of the track.
    pub album: String,
    /// The duration in seconds, when it could be read.
    pub duration: Option<f64>,
    /// The track number inside the album, when it is known.
    pub track_number: Option<u32>,
}

impl LocalTrack {
    /// Builds a player track from a library entry.
    pub fn to_track(&self) -> Track {
        let mut track = Track::local(
            self.path.display().to_string(),
            self.title.clone(),
            self.artist.clone(),
        );
        track.album = Some(self.album.clone());
        track.duration = self.duration;
        track
    }
}

/// The local music library.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Library {
    /// The folders that are scanned.
    pub roots: Vec<PathBuf>,
    /// The indexed tracks.
    pub tracks: Vec<LocalTrack>,
}

impl Library {
    /// Creates an empty library.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a folder to the library and scans it.
    pub fn add_root(&mut self, root: impl Into<PathBuf>) -> Result<(), LibraryError> {
        let root = root.into();
        if !root.is_dir() {
            return Err(LibraryError::Read {
                path: root,
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "the folder does not exist",
                ),
            });
        }
        if !self.roots.iter().any(|existing| existing == &root) {
            self.roots.push(root);
        }
        self.rescan();
        Ok(())
    }

    /// Removes a folder from the library.
    pub fn remove_root(&mut self, root: &Path) {
        self.roots.retain(|existing| existing != root);
        self.rescan();
    }

    /// Scans every folder again and rebuilds the index.
    pub fn rescan(&mut self) {
        let mut tracks = Vec::new();

        for root in &self.roots {
            match scan_folder(root, &mut tracks) {
                Ok(()) => {}
                Err(err) => tracing::warn!("{err}"),
            }
        }

        tracks.sort_by(|left, right| {
            left.artist
                .to_lowercase()
                .cmp(&right.artist.to_lowercase())
                .then_with(|| left.album.to_lowercase().cmp(&right.album.to_lowercase()))
                .then_with(|| {
                    left.track_number
                        .unwrap_or(0)
                        .cmp(&right.track_number.unwrap_or(0))
                })
                .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        });

        self.tracks = tracks;
    }

    /// The number of indexed tracks.
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// Whether the library is empty.
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
}

/// Walks a folder and collects the audio files it contains.
fn scan_folder(root: &Path, tracks: &mut Vec<LocalTrack>) -> Result<(), LibraryError> {
    let entries = std::fs::read_dir(root).map_err(|source| LibraryError::Read {
        path: root.to_path_buf(),
        source,
    })?;

    let mut folders = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                tracing::debug!("skipping an unreadable entry of {root:?}: {err}");
                continue;
            }
        };

        let path = entry.path();
        if path.is_dir() {
            folders.push(path);
            continue;
        }

        if !is_audio_file(&path) {
            continue;
        }

        match read_track(&path) {
            Ok(track) => tracks.push(track),
            Err(err) => tracing::debug!("could not read the tags of {path:?}: {err}"),
        }
    }

    // The sub folders are scanned as well.
    for folder in folders {
        let _ = scan_folder(&folder, tracks);
    }

    Ok(())
}

/// Whether the path looks like an audio file.
fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            let extension = extension.to_ascii_lowercase();
            AUDIO_EXTENSIONS.contains(&extension.as_str())
        })
        .unwrap_or(false)
}

/// Reads the tags and the properties of a file.
fn read_track(path: &Path) -> Result<LocalTrack, Box<dyn std::error::Error>> {
    let tagged = Probe::open(path)?.guess_file_type()?.read()?;

    let duration = {
        let properties = tagged.properties();
        let seconds = properties.duration().as_secs_f64();
        if seconds > 0.0 {
            Some(seconds)
        } else {
            None
        }
    };

    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let title = tag
        .and_then(|tag| tag.title())
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty());
    let artist = tag
        .and_then(|tag| tag.artist())
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty());
    let album = tag
        .and_then(|tag| tag.album())
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty());
    let track_number = tag.and_then(|tag| tag.track()).filter(|number| *number > 0);

    let file_name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    Ok(LocalTrack {
        path: path.to_path_buf(),
        title: title.unwrap_or(file_name),
        artist: artist.unwrap_or_else(|| "Unknown artist".to_string()),
        album: album.unwrap_or_default(),
        duration,
        track_number,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_audio_files() {
        assert!(is_audio_file(Path::new("song.MP3")));
        assert!(is_audio_file(Path::new("/music/song.flac")));
        assert!(!is_audio_file(Path::new("/music/cover.jpg")));
        assert!(!is_audio_file(Path::new("/music/song")));
    }

    #[test]
    fn an_empty_library_has_no_tracks() {
        let library = Library::new();
        assert!(library.is_empty());
        assert_eq!(library.len(), 0);
    }

    #[test]
    fn scanning_a_missing_folder_is_reported() {
        let mut library = Library::new();
        let result = library.add_root("this folder does not exist");
        assert!(result.is_err());
        assert!(library.is_empty());
    }
}
