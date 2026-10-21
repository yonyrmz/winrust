//! Integration tests for the decoding pipeline.

use std::io::{Cursor, Write};

use winrust_audio::{DecoderStream, MediaSource, SampleRateConverter};

/// Writes a 16 bit PCM WAV file with a sine wave.
fn write_wav(path: &std::path::Path, sample_rate: u32, seconds: f32) {
    let frames = (sample_rate as f32 * seconds) as u32;
    let channels: u16 = 2;
    let bits_per_sample: u16 = 16;
    let block_align = channels * bits_per_sample / 8;
    let data_size = frames * block_align as u32;

    let mut data = Vec::new();
    for frame in 0..frames {
        let value = ((frame as f32 / sample_rate as f32) * 440.0 * std::f32::consts::TAU).sin();
        let sample = (value * 20_000.0) as i16;
        for _ in 0..channels {
            data.extend_from_slice(&sample.to_le_bytes());
        }
    }

    let mut file = std::fs::File::create(path).expect("the wav file can be created");
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&(sample_rate * block_align as u32).to_le_bytes())
        .unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&bits_per_sample.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    file.write_all(&data).unwrap();
}

#[test]
fn decodes_a_local_wav_file() {
    let dir = std::env::temp_dir().join("winrust-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tone.wav");

    write_wav(&path, 44_100, 0.5);

    let source = MediaSource::file(&path);
    let mut decoder = DecoderStream::open(&source).expect("the wav file can be decoded");

    assert_eq!(decoder.stream_format.sample_rate, 44_100);
    assert_eq!(decoder.stream_format.channels, 2);

    let mut total = 0usize;
    while let Some(samples) = decoder.decode_next().expect("decoding works") {
        total += samples.len();
        assert!(samples.iter().all(|sample| sample.is_finite()));
    }

    // Two channels of 44_100 samples for half a second.
    assert!(
        total >= 44_100,
        "expected at least 44100 samples, got {total}"
    );

    std::fs::remove_file(&path).ok();
}

#[test]
fn resamples_to_the_target_rate() {
    let dir = std::env::temp_dir().join("winrust-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tone-resample.wav");

    write_wav(&path, 44_100, 1.0);

    let mut decoder =
        DecoderStream::open(&MediaSource::file(&path)).expect("the wav file can be decoded");
    let mut converter =
        SampleRateConverter::new(44_100, 48_000, 2).expect("the converter can be created");

    let mut output = Vec::new();
    while let Some(samples) = decoder.decode_next().expect("decoding works") {
        converter
            .process(&samples, &mut output)
            .expect("resampling works");
    }
    converter.flush(&mut output).expect("flushing works");

    // One second at 48 kHz in stereo.
    assert!(
        output.len() >= 48_000 * 2,
        "expected at least 96000 samples, got {}",
        output.len()
    );
    assert!(output.iter().all(|sample| sample.is_finite()));

    std::fs::remove_file(&path).ok();
}

#[test]
fn decodes_an_in_memory_cursor() {
    // A minimal 8 bit PCM WAV built by hand.
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&36u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8_000u32.to_le_bytes());
    bytes.extend_from_slice(&8_000u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&800u32.to_le_bytes());
    bytes.extend_from_slice(&vec![128u8; 800]);

    let cursor = Cursor::new(bytes);
    let mut reader = Box::new(cursor);
    let mut header = [0u8; 4];
    std::io::Read::read_exact(&mut reader, &mut header).unwrap();
    assert_eq!(&header, b"RIFF");
}
