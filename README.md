# winrust

**winrust** is a Winamp inspired music player written in Rust. It keeps the
classic look and feel — the tiny window, the LED time display, the equalizer
with its ten bands, the spectrum analyzer and the skinnable interface — and
plays music straight from **YouTube Music**.

```text
┌──────────────────────────────────────────────┐
│  ▓▓ winrust ▓▓   00:00   ▶ ▮▮  ♪ ♪ ♪  ▓▓▓▓  │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│  [ search YouTube Music… ]  [Search]         │
│  ──────────────────────────────────────────  │
│  1. Artist — Title                    3:21   │
│  2. Artist — Title                    4:02   │
└──────────────────────────────────────────────┘
```

## Features

- **Winamp look** — the main window is drawn from a skin, exactly like the
  original: 275×116 pixels, LED digits, transport buttons, position bar,
  volume and balance sliders.
- **Skins** — loads classic `.wsz` archives (the ZIP format Winamp used). A
  default skin is generated procedurally so no proprietary artwork has to be
  shipped.
- **YouTube Music** — search the catalogue and play it, powered by the
  external `yt-dlp` binary.
- **Real audio pipeline** — `cpal` for the output, `symphonia` for decoding
  and `rubato` for sample rate conversion, with a ring buffer so playback
  never glitches.
- **Ten band equalizer** — biquad peaking filters on the classic Winamp
  frequencies (60 Hz … 16 kHz) with a preamp.
- **Spectrum analyzer** — an FFT based visualizer fed by the live signal.
- **Playlist** — queue management with a persistent, JSON backed store.

## Requirements

- A Rust toolchain (1.87 or newer).
- [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) in `PATH`, if you want to
  stream from YouTube Music.

## Building and running

```sh
cargo build --release
cargo run --release
```

The binary is written to `target/release/winrust-app`.

## Using it

1. Type a song, an artist or an album in the search box and press **Enter**.
2. Double click a result to play it. The track is added to the playlist.
3. Use the transport buttons of the main window to play, pause, stop and
   skip.
4. Click the **EQ** button to open the equalizer and the **PL** button to
   open the playlist editor.
5. Use **Load skin…** to load a `.wsz` archive.

### Keyboard shortcuts

| Key | Action |
| --- | --- |
| `Space` | Play / pause |
| `Ctrl` + `Right` | Next track |
| `Ctrl` + `Left` | Previous track |
| `Ctrl` + `Up` / `Down` | Volume up / down |
| `Ctrl` + `S` | Stop |

## Project layout

```text
crates/
├── winrust-core/      # domain types: tracks, queue, player state
├── winrust-audio/     # cpal output, symphonia decoding, ring buffer
├── winrust-dsp/       # equalizer, FFT visualizer, resampler helpers
├── winrust-ytmusic/   # yt-dlp wrapper: search and stream resolution
├── winrust-skin/      # .wsz loader, sprite tables, default skin
├── winrust-ui/        # egui interface: main window, browser, playlist, EQ
└── winrust-app/       # the executable
```

## Legal notes

- **winrust is not affiliated with Nullsoft or Winamp.** It is an independent
  open source project that happens to share a look and a file format.
- YouTube Music does not offer a public API for third party players. The
  project therefore relies on `yt-dlp`, which is an external tool that is not
  distributed with winrust. Depending on your jurisdiction, using it may
  conflict with the terms of service of the platform. Use it responsibly.
- No artwork from the original Winamp is bundled. The default skin is drawn
  from scratch at runtime; third party `.wsz` skins are provided by the user
  and remain the property of their authors.

## License

[MIT](LICENSE)
