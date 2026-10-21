//! Loading of `.wsz` skin archives.
//!
//! A `.wsz` file is a ZIP archive containing the bitmaps and the plain text
//! configuration files of a skin. The loader is tolerant: unknown files are
//! ignored and a missing configuration file simply keeps the default colours.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use thiserror::Error;

use crate::config::{self, GeneralColors, PlaylistTheme, RegionSet, VisualizerColors};
use crate::image::Image;

/// Errors raised while loading a skin.
#[derive(Debug, Error)]
pub enum SkinError {
    /// The archive could not be opened.
    #[error("could not open the skin archive {path}: {source}")]
    Open {
        /// Path of the archive.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The archive is not a valid ZIP file.
    #[error("the skin archive is not a valid zip file: {0}")]
    InvalidArchive(String),
    /// A bitmap inside the archive could not be decoded.
    #[error("could not decode {name}: {source}")]
    Image {
        /// Name of the file inside the archive.
        name: String,
        /// Underlying decoding error.
        #[source]
        source: crate::image::ImageError,
    },
}

/// A loaded skin.
#[derive(Debug, Clone, Default)]
pub struct Skin {
    /// Display name of the skin.
    pub name: String,
    /// Decoded bitmaps, keyed by their lower case file name.
    pub images: HashMap<String, Image>,
    /// Colours of the playlist window.
    pub playlist: PlaylistTheme,
    /// The 24 general purpose colours.
    pub general: GeneralColors,
    /// The colours of the visualizer.
    pub visualizer: VisualizerColors,
    /// Window regions, used to implement transparency.
    pub regions: RegionSet,
}

impl Skin {
    /// Returns the bitmap with the given name, if present.
    pub fn image(&self, name: &str) -> Option<&Image> {
        self.images.get(&name.to_ascii_lowercase())
    }

    /// Whether the skin contains the given bitmap.
    pub fn has_image(&self, name: &str) -> bool {
        self.image(name).is_some()
    }
}

/// Loads a `.wsz` archive from disk.
pub fn load_wsz(path: &Path) -> Result<Skin, SkinError> {
    let file = std::fs::File::open(path).map_err(|source| SkinError::Open {
        path: path.display().to_string(),
        source,
    })?;

    let mut archive =
        zip::ZipArchive::new(file).map_err(|err| SkinError::InvalidArchive(err.to_string()))?;

    let mut skin = Skin {
        name: path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| "skin".to_string()),
        ..Default::default()
    };

    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| SkinError::InvalidArchive(err.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let file_name = name
            .rsplit('/')
            .next()
            .unwrap_or(name.as_str())
            .to_ascii_lowercase();
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .map_err(|err| SkinError::InvalidArchive(err.to_string()))?;
        entries.push((file_name, data));
    }

    apply_entries(&mut skin, entries)?;
    Ok(skin)
}

/// Applies the archive entries to a skin.
fn apply_entries(skin: &mut Skin, entries: Vec<(String, Vec<u8>)>) -> Result<(), SkinError> {
    for (name, data) in entries {
        if name.ends_with(".bmp") {
            let image = Image::decode(&data).map_err(|source| SkinError::Image {
                name: name.clone(),
                source,
            })?;
            skin.images.insert(name, image);
            continue;
        }

        let text = match String::from_utf8(data) {
            Ok(text) => text,
            Err(_) => continue,
        };

        match name.as_str() {
            "pledit.txt" => {
                if let Ok(theme) = config::parse_pledit(&text) {
                    skin.playlist = theme;
                }
            }
            "gen.txt" => {
                if let Ok(colors) = config::parse_general(&text) {
                    skin.general = colors;
                }
            }
            "viscolor.txt" => {
                if let Ok(colors) = config::parse_visualizer(&text) {
                    skin.visualizer = colors;
                }
            }
            "region.txt" => {
                skin.regions = config::parse_regions(&text);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Builds a skin from an in memory archive.
pub fn load_wsz_bytes(bytes: &[u8], name: &str) -> Result<Skin, SkinError> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|err| SkinError::InvalidArchive(err.to_string()))?;

    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| SkinError::InvalidArchive(err.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let file_name = entry
            .name()
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .map_err(|err| SkinError::InvalidArchive(err.to_string()))?;
        entries.push((file_name, data));
    }

    let mut skin = Skin {
        name: name.to_string(),
        ..Default::default()
    };
    apply_entries(&mut skin, entries)?;
    Ok(skin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn build_archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, data) in files {
                writer.start_file(*name, options).unwrap();
                writer.write_all(data).unwrap();
            }
            writer.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn loads_text_configuration() {
        let archive = build_archive(&[(
            "pledit.txt",
            b"[Text]\nNormal=#00FF00\nCurrent=#FFFFFF\nSelected=#0000FF\n",
        )]);
        let skin = load_wsz_bytes(&archive, "test").unwrap();
        assert_eq!(skin.playlist.foreground.r, 0);
        assert_eq!(skin.playlist.foreground.g, 255);
    }

    #[test]
    fn ignores_unknown_files() {
        let archive = build_archive(&[("readme.txt", b"hello")]);
        let skin = load_wsz_bytes(&archive, "test").unwrap();
        assert!(skin.images.is_empty());
    }
}
