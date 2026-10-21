//! End to end tests against the real YouTube Music catalogue.
//!
//! These tests need network access and a working `yt-dlp`, so they are
//! ignored by default. Run them with:
//!
//! ```sh
//! cargo test -p winrust-ytmusic -- --ignored
//! ```

use winrust_ytmusic::YtMusicClient;

#[tokio::test]
#[ignore = "requires network access and yt-dlp"]
async fn searches_the_catalogue() {
    let client = YtMusicClient::new().expect("yt-dlp is installed");
    let results = client
        .search("daft punk", 5)
        .await
        .expect("the search works");

    assert!(!results.is_empty(), "the search returned nothing");
    assert!(results.len() <= 5);

    let first = &results[0];
    assert!(!first.id.is_empty());
    assert!(!first.title.is_empty());
    assert_ne!(first.artist, "Unknown artist");
    assert!(first.duration.unwrap_or(0.0) > 0.0);
}

#[tokio::test]
#[ignore = "requires network access and yt-dlp"]
async fn resolves_a_stream() {
    let client = YtMusicClient::new().expect("yt-dlp is installed");
    let results = client
        .search("daft punk", 1)
        .await
        .expect("the search works");

    let track = &results[0];
    let resolved = client
        .resolve(&track.id)
        .await
        .expect("the stream can be resolved");

    assert!(resolved.url.starts_with("http"), "the url looks wrong");
    assert!(resolved.duration.unwrap_or(0.0) > 0.0);
    assert!(resolved.extension.is_some(), "the extension is missing");
}
