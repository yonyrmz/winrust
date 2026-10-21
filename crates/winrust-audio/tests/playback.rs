//! End to end test of the YouTube Music playback path.
//!
//! It resolves a real track, downloads the stream and decodes it, which
//! exercises the whole pipeline. The test needs network access and a
//! working `yt-dlp`, so it is ignored by default.
//!
//! ```sh
//! cargo test -p winrust-audio --test playback -- --ignored
//! ```

use winrust_audio::{DecoderStream, MediaSource};

#[tokio::test]
#[ignore = "requires network access and yt-dlp"]
async fn downloads_and_decodes_a_youtube_music_track() {
    let client = winrust_ytmusic::YtMusicClient::new().expect("yt-dlp is installed");

    let results = client
        .search("daft punk one more time", 3)
        .await
        .expect("the search works");
    assert!(!results.is_empty(), "the search returned nothing");

    let track = &results[0];
    let resolved = client
        .resolve(&track.id)
        .await
        .expect("the stream can be resolved");

    let source = MediaSource::http_with_extension(resolved.url.clone(), resolved.extension.clone());
    let mut decoder = DecoderStream::open(&source).expect("the stream can be decoded");

    println!(
        "decoding '{}' at {} Hz with {} channels",
        track.title, decoder.stream_format.sample_rate, decoder.stream_format.channels
    );

    let mut frames = 0usize;
    while let Some(samples) = decoder.decode_next().expect("decoding works") {
        assert!(samples.iter().all(|sample| sample.is_finite()));
        frames += samples.len() / decoder.stream_format.channels as usize;
    }

    let expected = resolved.duration.unwrap_or(0.0) * decoder.stream_format.sample_rate as f64;
    assert!(
        frames as f64 > expected * 0.8,
        "decoded {frames} frames, expected about {expected}"
    );
}
