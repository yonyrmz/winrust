//! Media source abstraction for the audio engine.
//!
//! Two source kinds are supported: local files and remote HTTP URLs (used
//! for YouTube Music streams resolved through `yt-dlp`).

use std::fs::File;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use thiserror::Error;

/// Errors produced while preparing a media source.
#[derive(Debug, Error)]
pub enum SourceError {
    /// The local file could not be opened.
    #[error("failed to open file {path}: {source}")]
    OpenFile {
        /// Path that failed to open.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The remote resource could not be downloaded.
    #[error("failed to download media: {0}")]
    Download(String),
    /// The response was empty.
    #[error("the media response was empty")]
    Empty,
}

/// Where the audio bytes come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaSource {
    /// A file on disk.
    File(PathBuf),
    /// A remote HTTP(S) URL.
    Http(String),
}

impl MediaSource {
    /// Builds a file source.
    pub fn file(path: impl Into<PathBuf>) -> Self {
        MediaSource::File(path.into())
    }

    /// Builds an HTTP source.
    pub fn http(url: impl Into<String>) -> Self {
        MediaSource::Http(url.into())
    }

    /// Returns the file extension of the source, if any.
    ///
    /// The extension is used to hint the demuxer, which matters for
    /// container formats that lack a reliable magic signature.
    pub fn extension(&self) -> Option<String> {
        match self {
            MediaSource::File(path) => path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_ascii_lowercase()),
            MediaSource::Http(url) => {
                let path = url.split(['?', '#']).next().unwrap_or(url);
                let name = path.rsplit('/').next().unwrap_or_default();
                match name.rsplit_once('.') {
                    Some((_, ext)) if !ext.is_empty() && ext.len() <= 5 => {
                        Some(ext.to_ascii_lowercase())
                    }
                    _ => None,
                }
            }
        }
    }

    /// Fetches the raw bytes of the source.
    ///
    /// Remote sources are downloaded fully into memory before decoding. This
    /// keeps the decoder simple and makes seeking cheap.
    pub fn read_all(&self) -> Result<Vec<u8>, SourceError> {
        match self {
            MediaSource::File(path) => {
                let mut file = File::open(path).map_err(|source| SourceError::OpenFile {
                    path: path.clone(),
                    source,
                })?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)
                    .map_err(|source| SourceError::OpenFile {
                        path: path.clone(),
                        source,
                    })?;
                Ok(buffer)
            }
            MediaSource::Http(url) => fetch_http(url),
        }
    }

    /// Opens the source as a readable byte stream.
    pub fn open_reader(&self) -> Result<Box<dyn Read + Send + Sync>, SourceError> {
        match self {
            MediaSource::File(path) => {
                let file = File::open(path).map_err(|source| SourceError::OpenFile {
                    path: path.clone(),
                    source,
                })?;
                Ok(Box::new(file))
            }
            MediaSource::Http(url) => {
                let bytes = fetch_http(url)?;
                Ok(Box::new(Cursor::new(bytes)))
            }
        }
    }
}

/// Downloads an HTTP(S) resource into memory.
fn fetch_http(url: &str) -> Result<Vec<u8>, SourceError> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) winrust/0.1")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|err| SourceError::Download(err.to_string()))?;

    let response = client
        .get(url)
        .send()
        .map_err(|err| SourceError::Download(err.to_string()))?
        .error_for_status()
        .map_err(|err| SourceError::Download(err.to_string()))?;

    let bytes = response
        .bytes()
        .map_err(|err| SourceError::Download(err.to_string()))?;

    if bytes.is_empty() {
        return Err(SourceError::Empty);
    }
    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_http_extension() {
        assert_eq!(
            MediaSource::http("https://x.test/stream.webm?sig=abc").extension(),
            Some("webm".to_string())
        );
        assert_eq!(MediaSource::http("https://x.test/stream").extension(), None);
    }

    #[test]
    fn extracts_file_extension() {
        assert_eq!(
            MediaSource::file("/music/song.MP3").extension(),
            Some("mp3".to_string())
        );
    }
}
