# winrust

**winrust** is a small desktop music player written in Rust. It plays music
from **YouTube Music** and from your **local folders**, with a clean interface
and a compact mini player.

```text
┌──────────────────────────────────────────────────────────────┐
│ [ Search YouTube Music…      ] [Search]  Online Library Queue │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│   One More Time — Daft Punk                          5:21    │
│   Instant Crush — Daft Punk                          4:38    │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ [♪]  One More Time — Daft Punk        🔀 🔁 ⚙ ⇲   🔊 ▬▬▬▬    │
│     ◀◀ ▶ ▶|   0:42 ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬  5:21      │
└──────────────────────────────────────────────────────────────┘
```

## Features

- **Two sources** — search YouTube Music, or point it at your local folders.
- **Mini player** — a compact always-on-top window with the essential
  controls, so the player can get out of the way.
- **Real audio pipeline** — `cpal` for the output, `symphonia` for decoding and
  `rubato` for sample rate conversion, with a ring buffer so playback never
  glitches.
- **Ten band equalizer** — biquad peaking filters on the classic Winamp
  frequencies (60 Hz … 16 kHz) with a preamp, applied to the signal.
- **Play queue** — add, remove and reorder the songs that play next.
- **Cover art** — the thumbnails of YouTube Music and the tags of the local
  files are shown next to the player.

## Requirements

- A Rust toolchain (1.87 or newer).
- [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) in `PATH`, if you want to
  stream from YouTube Music. The player starts without it, it only disables
  the online search.

## Building and running

```sh
cargo build --release
cargo run --release
```

The binary is written to `target/release/winrust-app`.

## Using it

1. **Online** — type a song, an artist or an album in the search box and press
   **Enter**. Double click a result to play it, or use the **+** button to
   only add it to the queue.
2. **Library** — press **Add folder…** and pick a directory. The folder is
   scanned recursively, the tags are read and the tracks are listed. The
   **Filter** box narrows the list by artist, album or title.
3. **Queue** — the songs that will play next. Double click one to jump to it.
4. Press the **mini player** button (⇲) to open the compact window; it stays
   on top of the other windows.

### Keyboard shortcuts

| Key | Action |
| --- | --- |
| `Space` | Play / pause |
| `Ctrl` + `Right` | Next track |
| `Ctrl` + `Left` | Previous track |
| `Ctrl` + `Up` / `Down` | Volume up / down |
| `Ctrl` + `S` | Stop |
| `Ctrl` + `Shift` + `S` | Save the settings |

## Project layout

```text
crates/
├── winrust-core/      # domain types: tracks, queue, player state
├── winrust-audio/     # cpal output, symphonia decoding, ring buffer
├── winrust-dsp/       # equalizer, FFT visualizer, resampler helpers
├── winrust-ytmusic/   # yt-dlp wrapper: search and stream resolution
├── winrust-library/   # local folder scanning and tag reading
├── winrust-ui/        # egui interface: main window, mini player, library, EQ
└── winrust-app/       # the executable
```

## How the YouTube Music backend works

There is no public API, so winrust drives `yt-dlp` in two steps:

1. **Search** — `yt-dlp` is pointed at the YouTube Music search page and
   limited with `--playlist-end`, which keeps the request bounded. If that
   fails, the generic `ytsearch` is used as a fallback.
2. **Resolve** — the video id is turned into a direct media URL. Several
   player clients (`web_embedded`, `mweb`, …) are tried in turn, because
   YouTube blocks some of them depending on the network.

The AAC stream in an MP4 container is preferred, because the decoders handle
it best; Opus in WebM is used as a fallback. Direct URLs expire after a few
hours, so they are resolved again every time a track is played.

YouTube sometimes answers with *"Sign in to confirm you're not a bot"*. In
that case the error is shown in the interface; passing cookies to `yt-dlp`
(for example with `--cookies-from-browser`) usually solves it.

## Legal notes

- YouTube Music does not offer a public API for third party players. The
  project therefore relies on `yt-dlp`, which is an external tool that is not
  distributed with winrust. Depending on your jurisdiction, using it may
  conflict with the terms of service of the platform. Use it responsibly.
- The project is not affiliated with Nullsoft or Winamp; it just happens to
  share the same equalizer band layout.

## License

[MIT](LICENSE)
