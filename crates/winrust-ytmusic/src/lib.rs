//! YouTube Music support for winrust, powered by the external `yt-dlp` binary.
//!
//! YouTube Music does not expose a public API for third party players, so
//! winrust shells out to `yt-dlp` for two operations:
//!
//! * **search** – queries the YouTube Music search page and returns the
//!   matching entries.
//! * **resolve** – turns a video id into a direct media URL that the audio
//!   engine can download.
//!
//! The direct URLs are time limited (usually a few hours), which is fine for
//! on demand playback.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tokio::process::Command;
use winrust_core::Track;

/// Errors produced while talking to `yt-dlp`.
#[derive(Debug, thiserror::Error)]
pub enum YtMusicError {
    /// The `yt-dlp` executable could not be found in `PATH`.
    #[error("yt-dlp was not found in PATH; install it to stream from YouTube Music")]
    MissingBinary,
    /// The process could not be spawned.
    #[error("could not run yt-dlp: {0}")]
    Spawn(String),
    /// The command exited with a failure.
    #[error("yt-dlp failed ({code:?}): {stderr}")]
    CommandFailed {
        /// Process exit code, when available.
        code: Option<i32>,
        /// Standard error output.
        stderr: String,
    },
    /// The command timed out.
    #[error("yt-dlp timed out after {0:?}")]
    Timeout(Duration),
    /// The output could not be parsed.
    #[error("could not parse the yt-dlp output: {0}")]
    Parse(String),
    /// The search returned no results.
    #[error("no results found")]
    Empty,
}

/// A resolved, directly playable media stream.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedStream {
    /// Direct media URL. It expires after a few hours.
    pub url: String,
    /// Title reported by YouTube Music.
    pub title: String,
    /// Artist reported by YouTube Music.
    pub artist: String,
    /// Duration in seconds, when known.
    pub duration: Option<f64>,
    /// Thumbnail URL, when known.
    pub thumbnail: Option<String>,
}

/// A client for the YouTube Music catalogue.
#[derive(Debug, Clone)]
pub struct YtMusicClient {
    binary: PathBuf,
    timeout: Duration,
}

impl Default for YtMusicClient {
    fn default() -> Self {
        Self {
            binary: PathBuf::from("yt-dlp"),
            timeout: Duration::from_secs(45),
        }
    }
}

impl YtMusicClient {
    /// Creates a client using the `yt-dlp` binary found in `PATH`.
    ///
    /// The binary is looked up eagerly so the UI can report a helpful error
    /// instead of failing on the first search.
    pub fn new() -> Result<Self, YtMusicError> {
        let binary = find_executable("yt-dlp").ok_or(YtMusicError::MissingBinary)?;
        Ok(Self {
            binary,
            timeout: Duration::from_secs(45),
        })
    }

    /// Creates a client pointing at an explicit `yt-dlp` binary.
    pub fn with_binary(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            timeout: Duration::from_secs(45),
        }
    }

    /// Whether the `yt-dlp` binary can be located.
    pub fn is_available() -> bool {
        find_executable("yt-dlp").is_some()
    }

    /// Path of the binary in use.
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// Searches YouTube Music for the given query.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>, YtMusicError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!("https://music.youtube.com/search?q={}", urlencode(query));

        let mut args: Vec<String> = vec![
            "--flat-playlist".into(),
            "--dump-json".into(),
            "--no-warnings".into(),
            "--no-playlist".into(),
            "--playlist-end".into(),
            limit.clamp(1, 200).to_string(),
        ];
        args.push(url);

        let stdout = self.run(&args).await?;
        let tracks = parse_search_results(&stdout);

        if tracks.is_empty() {
            // Fall back to the generic YouTube search in case the music
            // search page layout changed.
            let fallback_args: Vec<String> = vec![
                format!("ytsearch{limit}:{query}"),
                "--flat-playlist".into(),
                "--dump-json".into(),
                "--no-warnings".into(),
            ];
            let stdout = self.run(&fallback_args).await?;
            let tracks = parse_search_results(&stdout);
            if tracks.is_empty() {
                return Err(YtMusicError::Empty);
            }
            return Ok(tracks);
        }

        Ok(tracks)
    }

    /// Resolves a YouTube Music video id into a directly playable stream.
    pub async fn resolve(&self, id: &str) -> Result<ResolvedStream, YtMusicError> {
        let id = id.trim();
        if id.is_empty() {
            return Err(YtMusicError::Parse("empty video id".into()));
        }

        let url = format!("https://music.youtube.com/watch?v={id}");
        let args = vec![
            "--dump-json".into(),
            "--no-warnings".into(),
            "-f".into(),
            "bestaudio/best".into(),
            url,
        ];

        let stdout = self.run(&args).await?;
        parse_resolved(&stdout)
    }

    /// Runs `yt-dlp` with the given arguments and returns its standard output.
    async fn run(&self, args: &[String]) -> Result<String, YtMusicError> {
        let mut command = Command::new(&self.binary);
        command
            .args(args.iter().map(String::as_str))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let output = tokio::time::timeout(self.timeout, command.output())
            .await
            .map_err(|_| YtMusicError::Timeout(self.timeout))?
            .map_err(|err| YtMusicError::Spawn(err.to_string()))?;

        if !output.status.success() {
            return Err(YtMusicError::CommandFailed {
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// Minimal shape of a `yt-dlp --dump-json` entry.
#[derive(Debug, Deserialize)]
struct DumpEntry {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    uploader: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    artist: Option<String>,
    #[serde(default)]
    thumbnails: Option<Vec<Thumbnail>>,
}

/// A thumbnail entry as reported by `yt-dlp`.
#[derive(Debug, Deserialize)]
struct Thumbnail {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    width: Option<u64>,
    #[serde(default)]
    height: Option<u64>,
}

impl Thumbnail {
    /// Area of the thumbnail, used to pick the largest one.
    fn area(&self) -> u64 {
        self.width.unwrap_or(0) * self.height.unwrap_or(0)
    }
}

/// Picks the largest thumbnail of an entry.
fn best_thumbnail(thumbnails: Option<&Vec<Thumbnail>>) -> Option<String> {
    let thumbnails = thumbnails?;
    thumbnails
        .iter()
        .filter(|thumb| thumb.url.is_some())
        .max_by_key(|thumb| thumb.area())
        .and_then(|thumb| thumb.url.clone())
}

/// Parses the `--dump-json` output of a search run.
fn parse_search_results(stdout: &str) -> Vec<Track> {
    let mut tracks = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<DumpEntry>(line) else {
            continue;
        };
        let Some(id) = entry.id.clone().filter(|id| !id.is_empty()) else {
            continue;
        };

        let title = entry.title.unwrap_or_else(|| id.clone());
        let artist = entry
            .artist
            .or(entry.channel)
            .or(entry.uploader)
            .unwrap_or_else(|| "Unknown artist".to_string());

        tracks.push(Track::ytmusic(
            id,
            title,
            artist,
            entry.duration,
            best_thumbnail(entry.thumbnails.as_ref()),
        ));
    }

    tracks
}

/// Parses the `--dump-json` output of a resolve run.
fn parse_resolved(stdout: &str) -> Result<ResolvedStream, YtMusicError> {
    let mut last_error = YtMusicError::Empty;

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<DumpEntry>(line) else {
            continue;
        };

        let Some(media_url) = entry.url.clone().filter(|url| !url.is_empty()) else {
            last_error = YtMusicError::Parse("the entry did not contain a media url".into());
            continue;
        };

        let title = entry.title.clone().unwrap_or_default();
        let artist = entry
            .artist
            .clone()
            .or(entry.channel.clone())
            .or(entry.uploader.clone())
            .unwrap_or_default();

        return Ok(ResolvedStream {
            url: media_url,
            title,
            artist,
            duration: entry.duration,
            thumbnail: best_thumbnail(entry.thumbnails.as_ref()),
        });
    }

    Err(last_error)
}

/// Looks for an executable in `PATH`.
fn find_executable(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string())
            .split(';')
            .filter(|ext| !ext.is_empty())
            .map(|ext| ext.to_ascii_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };

    for dir in std::env::split_paths(&path) {
        for extension in &extensions {
            let candidate = if extension.is_empty() {
                dir.join(name)
            } else {
                dir.join(format!("{name}{extension}"))
            };
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Percent encodes a query string.
fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Extracts a YouTube video id from a URL or returns the id unchanged.
pub fn extract_video_id(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Some(rest) = input.split("v=").nth(1) {
        let id: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !id.is_empty() {
            return Some(id);
        }
    }
    if input.len() == 11
        && input
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Some(input.to_string());
    }
    None
}

/// Builds the YouTube Music watch URL for a video id.
pub fn watch_url(id: &str) -> String {
    format!("https://music.youtube.com/watch?v={id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_output() {
        let output = concat!(
            r#"{"id":"abc123XYZ_1","title":"Song","duration":210.5,"channel":"Artist","thumbnails":[{"url":"http://x/1.jpg","width":60,"height":60},{"url":"http://x/2.jpg","width":500,"height":500}]}"#,
            "\n",
            r#"{"id":"def","title":"Other"}"#
        );
        let tracks = parse_search_results(output);
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "Song");
        assert_eq!(tracks[0].artist, "Artist");
        assert_eq!(tracks[0].duration, Some(210.5));
        assert_eq!(
            tracks[0].thumbnail.as_deref(),
            Some("http://x/2.jpg"),
            "the largest thumbnail must win"
        );
        assert_eq!(tracks[1].artist, "Unknown artist");
    }

    #[test]
    fn parses_resolved_output() {
        let output =
            r#"{"id":"abc","title":"Song","url":"https://media.test/a.webm","duration":120}"#;
        let resolved = parse_resolved(output).unwrap();
        assert_eq!(resolved.url, "https://media.test/a.webm");
        assert_eq!(resolved.title, "Song");
        assert_eq!(resolved.duration, Some(120.0));
    }

    #[test]
    fn extracts_video_ids() {
        assert_eq!(
            extract_video_id("https://music.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_video_id("dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(extract_video_id(""), None);
    }

    #[test]
    fn urlencodes_queries() {
        assert_eq!(urlencode("daft punk"), "daft+punk");
        assert_eq!(urlencode("a&b"), "a%26b");
    }
}
