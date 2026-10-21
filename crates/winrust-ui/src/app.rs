//! The main application: the Winamp style window plus the browser panels.

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use egui::{Color32, RichText};
use winrust_audio::{AudioEngine, MediaSource};
use winrust_core::{PlaybackState, PlayerEvent, RepeatMode, Track, TrackSource};
use winrust_dsp::Visualizer;
use winrust_skin::sprites::{ToggleButton, TransportButton};
use winrust_skin::Skin;
use winrust_ytmusic::YtMusicClient;

use crate::main_window::{MainWindow, MainWindowState};
use crate::renderer::TextureCache;
use crate::theme::Theme;

/// A message sent from a background task to the UI thread.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum AppMessage {
    /// The search finished successfully.
    SearchResults(Vec<Track>),
    /// The search failed.
    SearchError(String),
    /// A track was resolved into a playable URL.
    Resolved(Track, String, Option<String>),
    /// Resolving a track failed.
    ResolveError(String),
}

/// The winrust application.
pub struct WinrustApp {
    /// The audio engine, when an output device was available.
    pub engine: Option<AudioEngine>,
    /// The player events produced by the audio engine.
    events: Receiver<PlayerEvent>,
    /// The YouTube Music client, when `yt-dlp` is installed.
    ytmusic: Option<YtMusicClient>,
    /// The tokio runtime used for the background tasks.
    runtime: Option<tokio::runtime::Runtime>,
    /// The channel used to receive background messages.
    messages: Receiver<AppMessage>,
    /// The sender used by the background tasks.
    message_tx: Sender<AppMessage>,

    /// The loaded skin.
    skin: Skin,
    /// The uploaded textures of the skin.
    textures: TextureCache,
    /// The colour theme.
    theme: Theme,
    /// The path of the loaded skin file.
    skin_path: Option<String>,

    /// The current search query.
    query: String,
    /// The results of the last search.
    results: Vec<Track>,
    /// Whether a search is running.
    searching: bool,
    /// The last error message.
    error: Option<String>,

    /// The play queue.
    queue: Vec<Track>,
    /// The index of the track that is playing.
    current_index: Option<usize>,
    /// The title of the current track.
    current_title: String,
    /// The artist of the current track.
    current_artist: String,

    /// The master volume.
    volume: f32,
    /// The stereo balance.
    balance: f32,
    /// Whether shuffle is enabled.
    shuffle: bool,
    /// The repeat mode.
    repeat: RepeatMode,
    /// Whether the equalizer window is open.
    equalizer_open: bool,
    /// Whether the playlist window is open.
    playlist_open: bool,
    /// The zoom factor of the skin.
    scale: f32,

    /// The spectrum analyzer.
    visualizer: Visualizer,
    /// The equalizer band gains in dB.
    eq_bands: [f32; winrust_dsp::BAND_COUNT],
    /// The equalizer preamp in dB.
    eq_preamp: f32,
    /// Whether the equalizer is enabled.
    eq_enabled: bool,
    /// The track that is currently being resolved.
    pending_track: Option<Track>,
    /// The elapsed time, used by the scrolling ticker.
    time: f64,
}

impl WinrustApp {
    /// Creates the application.
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (event_tx, events) = mpsc::channel();
        let (message_tx, messages) = mpsc::channel();

        let engine = match AudioEngine::new(event_tx) {
            Ok(engine) => Some(engine),
            Err(err) => {
                tracing::error!("could not start the audio engine: {err}");
                None
            }
        };

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .ok();

        let ytmusic = match YtMusicClient::new() {
            Ok(client) => Some(client),
            Err(err) => {
                tracing::warn!("yt-dlp is not available: {err}");
                None
            }
        };

        let skin = winrust_skin::default_skin();
        let theme = Theme::from_skin(&skin);

        // Restore the previous session, if there is one.
        let mut app = Self {
            engine,
            events,
            ytmusic,
            runtime,
            messages,
            message_tx,
            skin,
            textures: TextureCache::new(),
            theme,
            skin_path: None,
            query: String::new(),
            results: Vec::new(),
            searching: false,
            error: None,
            queue: Vec::new(),
            current_index: None,
            current_title: String::new(),
            current_artist: String::new(),
            volume: 1.0,
            balance: 0.0,
            shuffle: false,
            repeat: RepeatMode::Off,
            equalizer_open: false,
            playlist_open: false,
            scale: 2.0,
            visualizer: Visualizer::new(19, 44_100.0),
            eq_bands: [0.0; winrust_dsp::BAND_COUNT],
            eq_preamp: 0.0,
            eq_enabled: true,
            pending_track: None,
            time: 0.0,
        };

        app.restore();
        Ok(app)
    }

    /// Restores the previous session from disk.
    fn restore(&mut self) {
        let Some(state) = crate::state::load() else {
            return;
        };

        self.queue = state.queue;
        self.volume = state.volume.clamp(0.0, 1.0);
        self.balance = state.balance.clamp(-1.0, 1.0);
        if state.equalizer.len() == winrust_dsp::BAND_COUNT {
            self.eq_bands.copy_from_slice(&state.equalizer);
        }
        self.eq_preamp = state.equalizer_preamp;

        if let Some(path) = state.skin.as_ref() {
            match winrust_skin::load_wsz(std::path::Path::new(path)) {
                Ok(skin) => {
                    self.skin = skin;
                    self.theme = Theme::from_skin(&self.skin);
                    self.skin_path = Some(path.clone());
                }
                Err(err) => {
                    tracing::warn!("could not restore the skin {path}: {err}");
                }
            }
        }

        if let Some(engine) = self.engine.as_ref() {
            engine.set_volume(self.volume);
            engine.set_balance(self.balance);
            engine.set_equalizer_bands(self.eq_bands);
            engine.set_equalizer_preamp(self.eq_preamp);
        }
    }

    /// Whether the YouTube Music backend is available.
    pub fn has_youtube_music(&self) -> bool {
        self.ytmusic.is_some()
    }

    /// Starts a search for the current query.
    fn start_search(&mut self) {
        let Some(client) = self.ytmusic.clone() else {
            self.error =
                Some("yt-dlp was not found. Install it to stream from YouTube Music.".to_string());
            return;
        };
        let Some(runtime) = self.runtime.as_ref() else {
            self.error = Some("the background runtime is not available".to_string());
            return;
        };

        let query = self.query.trim().to_string();
        if query.is_empty() {
            return;
        }

        self.searching = true;
        self.error = None;

        let tx = self.message_tx.clone();
        runtime.spawn(async move {
            match client.search(&query, 25).await {
                Ok(results) => {
                    let _ = tx.send(AppMessage::SearchResults(results));
                }
                Err(err) => {
                    let _ = tx.send(AppMessage::SearchError(err.to_string()));
                }
            }
        });
    }

    /// Resolves a track into a playable URL and starts playing it.
    fn play_track(&mut self, track: Track) {
        let Some(client) = self.ytmusic.clone() else {
            self.error = Some("yt-dlp was not found in PATH".to_string());
            return;
        };
        let Some(runtime) = self.runtime.as_ref() else {
            self.error = Some("the background runtime is not available".to_string());
            return;
        };

        self.pending_track = Some(track.clone());
        self.error = None;

        let tx = self.message_tx.clone();
        runtime.spawn(async move {
            match client.resolve(&track.id).await {
                Ok(resolved) => {
                    let _ = tx.send(AppMessage::Resolved(
                        track,
                        resolved.url,
                        resolved.extension,
                    ));
                }
                Err(err) => {
                    let _ = tx.send(AppMessage::ResolveError(err.to_string()));
                }
            }
        });
    }

    /// Adds a track to the queue and starts playing it.
    fn enqueue_and_play(&mut self, track: Track) {
        if let Some(index) = self.queue.iter().position(|item| item.id == track.id) {
            self.current_index = Some(index);
        } else {
            self.queue.push(track.clone());
            self.current_index = Some(self.queue.len() - 1);
        }
        self.play_track(track);
    }

    /// Plays a track of the queue by index.
    fn play_queue_index(&mut self, index: usize) {
        if index >= self.queue.len() {
            return;
        }
        self.current_index = Some(index);
        let track = self.queue[index].clone();
        self.play_track(track);
    }

    /// Advances to the next track of the queue.
    fn play_next(&mut self) {
        if self.queue.is_empty() {
            return;
        }

        let next = if self.shuffle && self.queue.len() > 1 {
            // Pick another track at random, never the current one.
            let mut index = self.current_index.unwrap_or(0);
            while index == self.current_index.unwrap_or(usize::MAX) {
                index = rand_index(self.queue.len());
            }
            Some(index)
        } else {
            match self.repeat {
                RepeatMode::One => self.current_index,
                _ => {
                    if self.current_index.map(|index| index + 1) == Some(self.queue.len()) {
                        match self.repeat {
                            RepeatMode::All => Some(0),
                            _ => None,
                        }
                    } else {
                        self.current_index.map(|index| index + 1)
                    }
                }
            }
        };

        match next {
            Some(index) => self.play_queue_index(index),
            None => {
                if let Some(engine) = self.engine.as_ref() {
                    engine.stop();
                }
                self.current_index = None;
            }
        }
    }

    /// Goes back to the previous track of the queue.
    fn play_previous(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        match self.current_index {
            Some(index) if index > 0 => self.play_queue_index(index - 1),
            Some(_) => self.play_queue_index(0),
            None => self.play_queue_index(0),
        }
    }

    /// Toggles between playing and pausing.
    fn toggle_playback(&mut self) {
        let Some(engine) = self.engine.as_ref() else {
            return;
        };
        match engine.state() {
            PlaybackState::Playing => engine.pause(),
            PlaybackState::Paused => engine.resume(),
            PlaybackState::Stopped => {
                if let Some(index) = self.current_index {
                    self.play_queue_index(index);
                } else if !self.queue.is_empty() {
                    self.play_queue_index(0);
                } else if !self.results.is_empty() {
                    // Nothing is queued yet: start with the first result.
                    let track = self.results[0].clone();
                    self.enqueue_and_play(track);
                }
            }
        }
    }

    /// Handles the messages coming from the background tasks.
    fn drain_messages(&mut self) {
        while let Ok(message) = self.messages.try_recv() {
            match message {
                AppMessage::SearchResults(results) => {
                    self.searching = false;
                    self.results = results;
                }
                AppMessage::SearchError(err) => {
                    self.searching = false;
                    self.error = Some(err);
                }
                AppMessage::Resolved(track, url, extension) => {
                    self.pending_track = None;
                    self.current_title = track.title.clone();
                    self.current_artist = track.artist.clone();

                    if let Some(engine) = self.engine.as_ref() {
                        let mut track = track;
                        track.source = TrackSource::YouTubeMusic {
                            id: track.id.clone(),
                            url: url.clone(),
                        };
                        let source = MediaSource::http_with_extension(url, extension);
                        engine.play(track, source);
                    } else {
                        self.error = Some("no audio output device is available".to_string());
                    }
                }
                AppMessage::ResolveError(err) => {
                    self.pending_track = None;
                    self.error = Some(err);
                }
            }
        }
    }

    /// Handles the events coming from the audio engine.
    fn drain_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                PlayerEvent::TrackEnded => self.play_next(),
                PlayerEvent::StateChanged(state) => {
                    if state == PlaybackState::Stopped {
                        self.current_title.clear();
                        self.current_artist.clear();
                    }
                }
                PlayerEvent::Error(err) => self.error = Some(err),
                _ => {}
            }
        }
    }

    /// The state shown by the main window.
    fn main_window_state(&self) -> MainWindowState {
        let (position, duration, playing, paused) = match self.engine.as_ref() {
            Some(engine) => (
                engine.position(),
                engine.duration(),
                engine.state() == PlaybackState::Playing,
                engine.state() == PlaybackState::Paused,
            ),
            None => (0.0, 0.0, false, false),
        };

        MainWindowState {
            playing,
            paused,
            position,
            duration,
            volume: self.volume,
            balance: self.balance,
            shuffle: self.shuffle,
            repeat: self.repeat != RepeatMode::Off,
            title: self.current_title.clone(),
            artist: self.current_artist.clone(),
            visualizer: self.visualizer.bars.clone(),
            equalizer_open: self.equalizer_open,
            playlist_open: self.playlist_open,
            time: self.time,
        }
    }

    /// Handles the interactions of the main window.
    fn handle_main_window(&mut self, actions: crate::main_window::MainWindowActions) {
        if let Some(button) = actions.transport {
            match button {
                TransportButton::Play => {
                    if let Some(engine) = self.engine.as_ref() {
                        if engine.state() == PlaybackState::Stopped {
                            self.toggle_playback();
                        }
                    }
                }
                TransportButton::Pause => {
                    if let Some(engine) = self.engine.as_ref() {
                        if engine.state() == PlaybackState::Playing {
                            engine.pause();
                        }
                    }
                }
                TransportButton::Stop => {
                    if let Some(engine) = self.engine.as_ref() {
                        engine.stop();
                    }
                }
                TransportButton::Next => self.play_next(),
                TransportButton::Previous => self.play_previous(),
                TransportButton::Eject => {}
            }
        }

        if let Some(toggle) = actions.toggle {
            match toggle {
                ToggleButton::Shuffle => self.shuffle = !self.shuffle,
                ToggleButton::Repeat => {
                    self.repeat = match self.repeat {
                        RepeatMode::Off => RepeatMode::All,
                        RepeatMode::All => RepeatMode::One,
                        RepeatMode::One => RepeatMode::Off,
                    }
                }
                ToggleButton::Equalizer => self.equalizer_open = !self.equalizer_open,
                ToggleButton::Playlist => self.playlist_open = !self.playlist_open,
            }
        }

        if let Some(position) = actions.seek {
            if let Some(engine) = self.engine.as_ref() {
                let duration = engine.duration();
                if duration > 0.0 {
                    engine.seek(position as f64 * duration);
                }
            }
        }

        if let Some(volume) = actions.volume {
            self.volume = volume.clamp(0.0, 1.0);
            if let Some(engine) = self.engine.as_ref() {
                engine.set_volume(self.volume);
            }
        }

        if let Some(balance) = actions.balance {
            self.balance = balance;
            if let Some(engine) = self.engine.as_ref() {
                engine.set_balance(self.balance);
            }
        }
    }

    /// Draws the browser panel with the search field and the results.
    fn show_browser(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style())
            .fill(self.theme.panel)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text("Search YouTube Music…")
                            .desired_width(240.0),
                    );
                    let submit = response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    if ui.button("Search").clicked() || submit {
                        self.start_search();
                    }
                    if self.searching {
                        ui.spinner();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Load skin…").clicked() {
                            self.open_skin_dialog();
                        }
                        if ui.button("Save").clicked() {
                            self.save_state();
                        }
                    });
                });

                if let Some(error) = &self.error {
                    ui.colored_label(Color32::from_rgb(0xff, 0x60, 0x60), error);
                }

                if self.results.is_empty() {
                    ui.weak(if self.has_youtube_music() {
                        "Search for a song to fill the playlist."
                    } else {
                        "yt-dlp is not installed: install it to search YouTube Music."
                    });
                    return;
                }

                let mut to_play: Option<usize> = None;
                let mut to_add: Option<usize> = None;
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        for (index, track) in self.results.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let title = ui.add(
                                    egui::Label::new(RichText::new(&track.title).strong())
                                        .truncate(),
                                );
                                let artist = ui.add(egui::Label::new(&track.artist).truncate());
                                if title.double_clicked() || artist.double_clicked() {
                                    to_play = Some(index);
                                }
                                if ui
                                    .small_button("+")
                                    .on_hover_text("Add to the playlist")
                                    .clicked()
                                {
                                    to_add = Some(index);
                                }
                            });
                        }
                    });

                if let Some(index) = to_add {
                    let track = self.results[index].clone();
                    if !self.queue.iter().any(|item| item.id == track.id) {
                        self.queue.push(track);
                    }
                }

                if let Some(index) = to_play {
                    let track = self.results[index].clone();
                    self.enqueue_and_play(track);
                }
            });
    }

    /// Draws the playlist window.
    fn show_playlist_window(&mut self, ctx: &egui::Context) {
        let mut open = self.playlist_open;
        egui::Window::new("Playlist")
            .open(&mut open)
            .resizable(true)
            .default_width(420.0)
            .default_height(320.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Clear").clicked() {
                        self.queue.clear();
                        self.current_index = None;
                        if let Some(engine) = self.engine.as_ref() {
                            engine.stop();
                        }
                    }
                    if ui.button("Remove current").clicked() {
                        if let Some(index) = self.current_index {
                            self.queue.remove(index);
                            if self.queue.is_empty() {
                                self.current_index = None;
                            } else {
                                self.current_index = Some(index.min(self.queue.len() - 1));
                            }
                        }
                    }
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut to_play = None;
                    let mut to_remove = None;
                    for (index, track) in self.queue.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let is_current = self.current_index == Some(index);
                            let text = if is_current {
                                RichText::new(&track.title).color(self.theme.playlist_current)
                            } else {
                                RichText::new(&track.title).color(self.theme.playlist_text)
                            };
                            let label = ui.add(egui::Label::new(text).truncate());
                            if label.double_clicked() {
                                to_play = Some(index);
                            }
                            ui.add(egui::Label::new(&track.artist).truncate());
                            if ui.small_button("x").clicked() {
                                to_remove = Some(index);
                            }
                        });
                    }

                    if let Some(index) = to_play {
                        self.play_queue_index(index);
                    }
                    if let Some(index) = to_remove {
                        self.queue.remove(index);
                        if self.current_index == Some(index) {
                            self.current_index = None;
                        } else if self.current_index.map(|current| current > index) == Some(true) {
                            self.current_index = self.current_index.map(|current| current - 1);
                        }
                    }
                });
            });

        self.playlist_open = open;
    }

    /// Draws the equalizer window.
    fn show_equalizer_window(&mut self, ctx: &egui::Context) {
        let mut open = self.equalizer_open;
        egui::Window::new("Equalizer")
            .open(&mut open)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let mut enabled = self.eq_enabled;
                    if ui.checkbox(&mut enabled, "Enabled").changed() {
                        self.eq_enabled = enabled;
                        if let Some(engine) = self.engine.as_ref() {
                            engine.set_equalizer_enabled(enabled);
                        }
                    }
                    if ui.button("Reset").clicked() {
                        self.eq_bands = [0.0; winrust_dsp::BAND_COUNT];
                        self.eq_preamp = 0.0;
                        self.push_equalizer();
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Preamp");
                    let mut preamp = self.eq_preamp;
                    if ui
                        .add_sized(
                            [160.0, 20.0],
                            egui::Slider::new(&mut preamp, -12.0..=12.0).suffix(" dB"),
                        )
                        .changed()
                    {
                        self.eq_preamp = preamp;
                        self.push_equalizer();
                    }
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    for band in 0..winrust_dsp::BAND_COUNT {
                        ui.vertical(|ui| {
                            let mut gain = self.eq_bands[band];
                            let response = ui.add_sized(
                                [22.0, 120.0],
                                egui::Slider::new(&mut gain, -12.0..=12.0).vertical(),
                            );
                            if response.changed() {
                                self.eq_bands[band] = gain;
                                self.push_equalizer();
                            }
                            ui.small(format!("{}", winrust_dsp::WINAMP_BANDS[band]));
                        });
                    }
                });
            });

        self.equalizer_open = open;
    }

    /// Handles the global keyboard shortcuts.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (space, next, previous, volume_up, volume_down, stop) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::Space),
                input.key_pressed(egui::Key::ArrowRight) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::ArrowLeft) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::ArrowUp) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::ArrowDown) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::S) && input.modifiers.ctrl,
            )
        });

        if space {
            self.toggle_playback();
        }
        if next {
            self.play_next();
        }
        if previous {
            self.play_previous();
        }
        if stop {
            if let Some(engine) = self.engine.as_ref() {
                engine.stop();
            }
        }
        if volume_up || volume_down {
            let delta = if volume_up { 0.05 } else { -0.05 };
            self.volume = (self.volume + delta).clamp(0.0, 1.0);
            if let Some(engine) = self.engine.as_ref() {
                engine.set_volume(self.volume);
            }
        }
    }

    /// Sends the equalizer settings to the engine.
    fn push_equalizer(&mut self) {
        if let Some(engine) = self.engine.as_ref() {
            engine.set_equalizer_bands(self.eq_bands);
            engine.set_equalizer_preamp(self.eq_preamp);
            engine.set_equalizer_enabled(self.eq_enabled);
        }
    }

    /// Opens the native file dialog and loads the selected skin.
    fn open_skin_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Winamp skin", &["wsz", "zip"])
            .set_title("Load a skin")
            .pick_file()
        else {
            return;
        };

        match winrust_skin::load_wsz(&path) {
            Ok(skin) => {
                self.skin = skin;
                self.theme = Theme::from_skin(&self.skin);
                self.textures.clear();
                self.skin_path = Some(path.display().to_string());
                self.error = None;
            }
            Err(err) => {
                self.error = Some(format!("could not load the skin: {err}"));
            }
        }
    }

    /// Writes the current state to disk.
    fn save_state(&mut self) {
        let state = crate::state::PersistedState {
            queue: self.queue.clone(),
            volume: self.volume,
            balance: self.balance,
            equalizer: self.eq_bands.to_vec(),
            equalizer_preamp: self.eq_preamp,
            skin: self.skin_path.clone(),
        };

        match crate::state::save(&state) {
            Ok(()) => self.error = None,
            Err(err) => self.error = Some(format!("could not save the state: {err}")),
        }
    }
}

/// Picks a pseudo random index below `len`.
///
/// A small xorshift generator is used on purpose: it keeps the player free
/// of a dependency on a random number generator for such a small need.
fn rand_index(len: usize) -> usize {
    if len <= 1 {
        return 0;
    }
    let mut state = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.subsec_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    (state % len as u64) as usize
}

/// eframe glue.
impl eframe::App for WinrustApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_messages();
        self.drain_events();
        self.handle_shortcuts(ui.ctx());

        // Feed the visualizer with the samples coming out of the engine.
        if let Some(engine) = self.engine.as_ref() {
            let samples = engine.drain_tap();
            if !samples.is_empty() {
                self.visualizer.push_mono(&samples);
            } else {
                self.visualizer.decay();
            }
        }

        self.textures.upload(ui.ctx(), &self.skin);
        self.time = ui.ctx().input(|input| input.time);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.add_space(6.0);
                        let actions = {
                            let main = MainWindow::new(
                                &self.skin,
                                &self.textures,
                                &self.theme,
                                self.scale,
                            );
                            main.show(ui, &self.main_window_state())
                        };
                        self.handle_main_window(actions);
                    });

                    ui.add_space(6.0);
                    self.show_browser(ui);
                });
            });

        self.show_playlist_window(ui.ctx());
        self.show_equalizer_window(ui.ctx());

        // Keep the visualizer and the time display up to date.
        ui.ctx().request_repaint_after(Duration::from_millis(50));
    }
}
