//! Core domain types shared across all winrust crates.
//!
//! This crate intentionally has no I/O dependencies: it models the player
//! state, the play queue and the command bus so that every other crate can
//! depend on it without creating cycles.

use serde::{Deserialize, Serialize};

/// Where a track came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrackSource {
    /// A file on the local filesystem.
    Local {
        /// Absolute path to the media file.
        path: String,
    },
    /// A stream resolved from YouTube Music via yt-dlp.
    YouTubeMusic {
        /// YouTube video id.
        id: String,
        /// Direct (possibly expiring) media URL.
        url: String,
    },
}

impl TrackSource {
    /// Human readable label used by the UI.
    pub fn label(&self) -> String {
        match self {
            TrackSource::Local { path } => path.clone(),
            TrackSource::YouTubeMusic { id, .. } => format!("ytmusic:{id}"),
        }
    }
}

/// A single playable item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// Stable identifier (yt id or local path).
    pub id: String,
    /// Display title.
    pub title: String,
    /// Display artist.
    pub artist: String,
    /// Display album, optional.
    pub album: Option<String>,
    /// Duration in seconds, when known.
    pub duration: Option<f64>,
    /// Where the audio comes from.
    pub source: TrackSource,
    /// URL of a cover image, when known.
    pub thumbnail: Option<String>,
}

impl Track {
    /// Builds a local file track.
    pub fn local(
        path: impl Into<String>,
        title: impl Into<String>,
        artist: impl Into<String>,
    ) -> Self {
        let path = path.into();
        Self {
            id: path.clone(),
            title: title.into(),
            artist: artist.into(),
            album: None,
            duration: None,
            source: TrackSource::Local { path },
            thumbnail: None,
        }
    }

    /// Builds a YouTube Music track.
    pub fn ytmusic(
        id: impl Into<String>,
        title: impl Into<String>,
        artist: impl Into<String>,
        duration: Option<f64>,
        thumbnail: Option<String>,
    ) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            title: title.into(),
            artist: artist.into(),
            album: None,
            duration,
            source: TrackSource::YouTubeMusic {
                id: id.clone(),
                url: String::new(),
            },
            thumbnail,
        }
    }
}

/// Playback mode of the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RepeatMode {
    /// Do not repeat anything.
    #[default]
    Off,
    /// Repeat the whole queue.
    All,
    /// Repeat the current track only.
    One,
}

/// Playback status of the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlaybackState {
    /// Nothing loaded.
    #[default]
    Stopped,
    /// Currently playing.
    Playing,
    /// Paused.
    Paused,
}

/// Snapshot of everything the UI needs to draw itself.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// Index of the track being played within the queue.
    pub current_index: Option<usize>,
    /// Number of tracks in the queue.
    pub queue_len: usize,
    /// Current playback status.
    pub playback: PlaybackState,
    /// Current position in seconds.
    pub position: f64,
    /// Duration of the current track in seconds.
    pub duration: f64,
    /// Master volume in the `0.0..=1.0` range.
    pub volume: f32,
    /// Whether the track is muted.
    pub muted: bool,
    /// Repeat mode.
    pub repeat: RepeatMode,
    /// Whether shuffle is enabled.
    pub shuffle: bool,
    /// Title of the current track.
    pub current_title: String,
    /// Artist of the current track.
    pub current_artist: String,
    /// Title of the next track in the queue.
    pub next_title: String,
}

impl PlayerState {
    /// Returns `true` when the player is actively producing sound.
    pub fn is_playing(&self) -> bool {
        self.playback == PlaybackState::Playing
    }

    /// Progress through the current track in the `0.0..=1.0` range.
    pub fn progress(&self) -> f32 {
        if self.duration > 0.0 {
            (self.position / self.duration).clamp(0.0, 1.0) as f32
        } else {
            0.0
        }
    }
}

/// Commands sent from the UI towards the audio engine.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerCommand {
    /// Start (or replace) playback of the given queue index.
    Play(usize),
    /// Resume playback.
    Resume,
    /// Pause playback.
    Pause,
    /// Toggle between play and pause.
    Toggle,
    /// Stop playback and unload the current track.
    Stop,
    /// Jump to the next track.
    Next,
    /// Jump to the previous track.
    Previous,
    /// Seek to the given absolute position in seconds.
    Seek(f64),
    /// Set the master volume (`0.0..=1.0`).
    SetVolume(f32),
    /// Toggle the mute state.
    ToggleMute,
    /// Change the repeat mode.
    SetRepeat(RepeatMode),
    /// Enable or disable shuffle.
    SetShuffle(bool),
    /// Replace the entire queue.
    SetQueue(Vec<Track>),
    /// Insert a track at the given position.
    InsertAt(usize, Track),
    /// Remove the track at the given position.
    RemoveAt(usize),
    /// Move a track inside the queue.
    Move(usize, usize),
    /// Clear the queue.
    ClearQueue,
}

/// Events emitted by the audio engine back to the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerEvent {
    /// The engine started playing a new track.
    TrackStarted(Track),
    /// The engine reached the end of the current track.
    TrackEnded,
    /// The engine reached the end of the queue.
    QueueFinished,
    /// Playback state changed.
    StateChanged(PlaybackState),
    /// The current position changed (in seconds).
    Position(f64),
    /// The engine became ready to accept commands.
    Ready,
    /// An error occurred while playing the current track.
    Error(String),
}

/// The shared, thread safe player state.
#[derive(Debug, Default)]
pub struct Player {
    state: parking_lot::Mutex<PlayerState>,
}

impl Player {
    /// Creates a new player with sane defaults.
    pub fn new() -> Self {
        Self {
            state: parking_lot::Mutex::new(PlayerState {
                volume: 1.0,
                ..Default::default()
            }),
        }
    }

    /// Returns a snapshot of the current state.
    pub fn state(&self) -> PlayerState {
        self.state.lock().clone()
    }

    /// Mutates the state in place.
    pub fn update<R>(&self, f: impl FnOnce(&mut PlayerState) -> R) -> R {
        let mut guard = self.state.lock();
        f(&mut guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_clamped() {
        let mut state = PlayerState {
            duration: 10.0,
            ..Default::default()
        };
        assert_eq!(state.progress(), 0.0);
        state.position = 5.0;
        assert!((state.progress() - 0.5).abs() < f32::EPSILON);
        state.position = 20.0;
        assert_eq!(state.progress(), 1.0);
    }

    #[test]
    fn player_updates_state() {
        let player = Player::new();
        player.update(|s| s.volume = 0.5);
        assert!((player.state().volume - 0.5).abs() < f32::EPSILON);
    }
}
