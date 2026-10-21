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
    Http {
        /// The URL of the media.
        url: String,
        /// An optional container hint, such as `webm` or `mp4`.
        extension: Option<String>,
    },
}

impl MediaSource {
    /// Builds a file source.
    pub fn file(path: impl Into<PathBuf>) -> Self {
        MediaSource::File(path.into())
    }

    /// Builds an HTTP source.
    pub fn http(url: impl Into<String>) -> Self {
        MediaSource::Http {
            url: url.into(),
            extension: None,
        }
    }

    /// Builds an HTTP source with a container hint.
    pub fn http_with_extension(url: impl Into<String>, extension: Option<String>) -> Self {
        MediaSource::Http {
            url: url.into(),
            extension: extension.map(|ext| ext.trim_start_matches('.').to_ascii_lowercase()),
        }
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
            MediaSource::Http { url, extension } => {
                if let Some(extension) = extension {
                    if !extension.is_empty() {
                        return Some(extension.clone());
                    }
                }
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
            MediaSource::Http { url, .. } => fetch_http(url),
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
            MediaSource::Http { url, .. } => {
                let bytes = fetch_http(url)?;
                Ok(Box::new(Cursor::new(bytes)))
            }
        }
    }
}

/// Downloads an HTTP(S) resource into memory.
///
/// The client is deliberately blocking: the decoder runs on its own thread,
/// so there is no need for an asynchronous stack here.
fn fetch_http(url: &str) -> Result<Vec<u8>, SourceError> {
    let mut response = ureq::get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
        )
        .call()
        .map_err(|err| SourceError::Download(err.to_string()))?;

    let body = response
        .body_mut()
        .read_to_vec()
        .map_err(|err| SourceError::Download(err.to_string()))?;

    if body.is_empty() {
        return Err(SourceError::Empty);
    }
    Ok(body)
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
