//! Persistent state of the player.
//!
//! The queue, the volume and the equalizer settings are stored as JSON in
//! the user configuration directory so that they survive a restart.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use winrust_core::Track;

/// The state that is written to disk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedState {
    /// The play queue.
    #[serde(default)]
    pub queue: Vec<Track>,
    /// The master volume.
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// The stereo balance.
    #[serde(default)]
    pub balance: f32,
    /// The equalizer band gains in dB.
    #[serde(default)]
    pub equalizer: Vec<f32>,
    /// The equalizer preamp in dB.
    #[serde(default)]
    pub equalizer_preamp: f32,
    /// The path of the loaded skin.
    #[serde(default)]
    pub skin: Option<String>,
}

/// The default master volume.
fn default_volume() -> f32 {
    1.0
}

/// Returns the path of the configuration file.
pub fn state_path() -> Option<PathBuf> {
    let directories = directories::ProjectDirs::from("dev", "winrust", "winrust")?;
    Some(directories.config_dir().join("state.json"))
}

/// Loads the persisted state, if there is any.
pub fn load() -> Option<PersistedState> {
    let path = state_path()?;
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Writes the state to disk.
pub fn save(state: &PersistedState) -> std::io::Result<()> {
    let path = state_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "the configuration directory could not be determined",
        )
    })?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let contents = serde_json::to_string_pretty(state)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(path, contents)
}
