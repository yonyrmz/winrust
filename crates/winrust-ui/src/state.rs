//! Persistent settings of the player.
//!
//! The settings are stored as JSON in the user configuration directory so
//! that they survive a restart.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The state that is written to disk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedState {
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
    /// The folders of the local library.
    #[serde(default)]
    pub library_roots: Vec<PathBuf>,
}

/// The default master volume.
fn default_volume() -> f32 {
    1.0
}

/// Returns the path of the configuration file.
pub fn state_path() -> Option<PathBuf> {
    let directories = directories::ProjectDirs::from("dev", "winrust", "winrust")?;
    Some(directories.config_dir().join("settings.json"))
}

/// Loads the persisted settings, if there are any.
pub fn load() -> Option<PersistedState> {
    let path = state_path()?;
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Writes the settings to disk.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_volume_is_one() {
        assert_eq!(default_volume(), 1.0);
    }

    #[test]
    fn the_state_round_trips() {
        let state = PersistedState {
            volume: 0.5,
            balance: -0.25,
            equalizer: vec![1.0, 2.0],
            equalizer_preamp: 3.0,
            library_roots: vec![PathBuf::from("/music")],
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: PersistedState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.volume, 0.5);
        assert_eq!(parsed.equalizer, vec![1.0, 2.0]);
        assert_eq!(parsed.library_roots.len(), 1);
    }
}
