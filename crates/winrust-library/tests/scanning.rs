//! Tests of the local library.

use std::io::Write;

use winrust_library::Library;

/// Writes a tiny WAV file with a RIFF INFO title tag.
fn write_wav_with_title(path: &std::path::Path, title: &str) {
    let samples: Vec<i16> = (0..8000).map(|index| (index % 64) as i16).collect();
    let data_size = (samples.len() * 2) as u32;

    let mut file = std::fs::File::create(path).expect("the wav file can be created");
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&8_000u32.to_le_bytes()).unwrap();
    file.write_all(&16_000u32.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    for sample in &samples {
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
    file.flush().unwrap();

    // A RIFF INFO chunk with the title, appended after the data chunk.
    let mut info = Vec::new();
    info.extend_from_slice(b"INFO");
    info.extend_from_slice(b"INAM");
    info.extend_from_slice(&(title.len() as u32 + 1).to_le_bytes());
    info.extend_from_slice(title.as_bytes());
    info.push(0);
    if info.len() % 2 == 1 {
        info.push(0);
    }

    let mut tag = Vec::new();
    tag.extend_from_slice(b"LIST");
    tag.extend_from_slice(&(info.len() as u32).to_le_bytes());
    tag.extend_from_slice(&info);

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("the wav file can be opened");
    file.write_all(&tag).unwrap();
}

#[test]
fn scans_a_folder() {
    let dir = tempfile::tempdir().expect("the temp dir can be created");
    write_wav_with_title(&dir.path().join("first.wav"), "First Song");
    std::fs::write(dir.path().join("notes.txt"), b"not audio").unwrap();
    std::fs::create_dir_all(dir.path().join("album")).unwrap();
    write_wav_with_title(&dir.path().join("album").join("second.wav"), "Second Song");

    let mut library = Library::new();
    library
        .add_root(dir.path())
        .expect("the folder can be added");

    assert_eq!(library.len(), 2, "only the audio files are indexed");

    let titles: Vec<String> = library
        .tracks
        .iter()
        .map(|track| track.title.clone())
        .collect();
    assert!(titles.contains(&"First Song".to_string()), "{titles:?}");
    assert!(titles.contains(&"Second Song".to_string()), "{titles:?}");

    // The duration has to be read from the file.
    let track = &library.tracks[0];
    assert!(track.duration.unwrap_or(0.0) > 0.0);
}

#[test]
fn removes_a_folder() {
    let dir = tempfile::tempdir().expect("the temp dir can be created");
    write_wav_with_title(&dir.path().join("song.wav"), "Song");

    let mut library = Library::new();
    library.add_root(dir.path()).unwrap();
    assert_eq!(library.len(), 1);

    library.remove_root(dir.path());
    assert!(library.is_empty());
    assert!(library.roots.is_empty());
}
