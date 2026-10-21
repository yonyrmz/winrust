//! The winrust application.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use egui::{Align, Layout, RichText, Vec2};
use winrust_audio::{AudioEngine, MediaSource};
use winrust_core::{PlaybackState, PlayerEvent, RepeatMode, Track, TrackSource};
use winrust_dsp::Visualizer;
use winrust_library::Library;
use winrust_ytmusic::YtMusicClient;

use crate::equalizer::EqualizerState;
use crate::mini_player::{self, ActionSink, MiniPlayerState};
use crate::player::{PlayerActions, PlayerBar};
use crate::state::PersistedState;
use crate::theme::Theme;

/// The tab that is being shown in the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// The results of a YouTube Music search.
    #[default]
    Online,
    /// The local library.
    Library,
    /// The play queue.
    Queue,
}

impl View {
    /// The label of the tab.
    pub fn label(&self) -> &'static str {
        match self {
            View::Online => "Online",
            View::Library => "Library",
            View::Queue => "Queue",
        }
    }
}

/// A message sent from a background task to the interface.
#[derive(Debug)]
enum AppMessage {
    /// The search finished successfully.
    SearchResults(Vec<Track>),
    /// The search failed.
    SearchError(String),
    /// A track was resolved into a playable URL.
    Resolved(Track, String, Option<String>),
    /// Resolving a track failed.
    ResolveError(String),
    /// The cover art of a track was downloaded.
    Thumbnail(String, Vec<u8>),
}

/// The winrust application.
pub struct WinrustApp {
    /// The audio engine, when an output device was available.
    engine: Option<AudioEngine>,
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

    /// The local library.
    library: Library,
    /// The tab that is being shown.
    view: View,
    /// The current search query.
    query: String,
    /// The results of the last search.
    results: Vec<Track>,
    /// Whether a search is running.
    searching: bool,
    /// The last error message.
    error: Option<String>,
    /// The filter of the library list.
    filter: String,

    /// The play queue.
    queue: Vec<Track>,
    /// The index of the track that is playing.
    current_index: Option<usize>,
    /// The title of the current track.
    title: String,
    /// The artist of the current track.
    artist: String,
    /// The album of the current track.
    album: String,

    /// The master volume.
    volume: f32,
    /// The stereo balance.
    balance: f32,
    /// Whether shuffle is enabled.
    shuffle: bool,
    /// The repeat mode.
    repeat: RepeatMode,

    /// The equalizer.
    equalizer: EqualizerState,
    /// The spectrum analyzer.
    visualizer: Visualizer,
    /// Whether the equalizer window is open.
    show_equalizer: bool,
    /// Whether the mini player is open.
    show_mini_player: bool,
    /// The channel that carries the actions of the mini player.
    mini_sink: ActionSink,

    /// The cover art of the current track.
    cover: Option<egui::TextureHandle>,
    /// The cached cover art.
    covers: HashMap<String, egui::TextureHandle>,
    /// The egui context, kept so that the mini player can be closed.
    ctx: Option<egui::Context>,
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

        let mut app = Self {
            engine,
            events,
            ytmusic,
            runtime,
            messages,
            message_tx,
            library: Library::new(),
            view: View::default(),
            query: String::new(),
            results: Vec::new(),
            searching: false,
            error: None,
            filter: String::new(),
            queue: Vec::new(),
            current_index: None,
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            volume: 1.0,
            balance: 0.0,
            shuffle: false,
            repeat: RepeatMode::Off,
            equalizer: EqualizerState {
                bands: [0.0; winrust_dsp::BAND_COUNT],
                preamp: 0.0,
                enabled: true,
            },
            visualizer: Visualizer::new(32, 44_100.0),
            show_equalizer: false,
            show_mini_player: false,
            mini_sink: mini_player::action_sink(),
            cover: None,
            covers: HashMap::new(),
            ctx: None,
        };

        app.restore();
        Ok(app)
    }

    /// Whether the YouTube Music backend is available.
    pub fn has_youtube_music(&self) -> bool {
        self.ytmusic.is_some()
    }

    /// Restores the previous session from disk.
    fn restore(&mut self) {
        let Some(state) = crate::state::load() else {
            return;
        };

        self.volume = state.volume.clamp(0.0, 1.0);
        self.balance = state.balance.clamp(-1.0, 1.0);
        if state.equalizer.len() == winrust_dsp::BAND_COUNT {
            self.equalizer.bands.copy_from_slice(&state.equalizer);
        }
        self.equalizer.preamp = state.equalizer_preamp;

        for root in &state.library_roots {
            if self.library.add_root(root).is_ok() {
                continue;
            }
        }

        if let Some(view) = state.last_view.as_deref() {
            self.view = match view {
                "Library" => View::Library,
                "Queue" => View::Queue,
                _ => View::Online,
            };
        }

        if let Some(engine) = self.engine.as_ref() {
            engine.set_volume(self.volume);
            engine.set_balance(self.balance);
            engine.set_equalizer_bands(self.equalizer.bands);
            engine.set_equalizer_preamp(self.equalizer.preamp);
        }
    }

    /// Writes the current settings to disk.
    pub fn save(&mut self) {
        let state = PersistedState {
            volume: self.volume,
            balance: self.balance,
            equalizer: self.equalizer.bands.to_vec(),
            equalizer_preamp: self.equalizer.preamp,
            library_roots: self.library.roots.clone(),
            last_view: Some(self.view.label().to_string()),
        };

        if let Err(err) = crate::state::save(&state) {
            self.error = Some(format!("could not save the settings: {err}"));
        }
    }

    // ---------------------------------------------------------------- search

    /// Starts a search for the current query.
    fn start_search(&mut self) {
        let Some(client) = self.ytmusic.clone() else {
            self.error =
                Some("yt-dlp was not found. Install it to stream from YouTube Music.".into());
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
        self.view = View::Online;

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
    fn resolve_and_play(&mut self, track: Track) {
        let Some(client) = self.ytmusic.clone() else {
            self.error = Some("yt-dlp was not found in PATH".to_string());
            return;
        };
        let Some(runtime) = self.runtime.as_ref() else {
            self.error = Some("the background runtime is not available".to_string());
            return;
        };

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

    /// Downloads the cover art of a track in the background.
    fn fetch_cover(&mut self, url: &str) {
        if self.covers.contains_key(url) {
            self.cover = self.covers.get(url).cloned();
            return;
        }

        let Some(runtime) = self.runtime.as_ref() else {
            return;
        };

        let url = url.to_string();
        let key = url.clone();
        let tx = self.message_tx.clone();
        runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                ureq::get(&url).call().ok()?.body_mut().read_to_vec().ok()
            })
            .await;

            if let Ok(Some(bytes)) = result {
                let _ = tx.send(AppMessage::Thumbnail(key, bytes));
            }
        });
    }

    // -------------------------------------------------------------- playback

    /// Plays a track, adding it to the queue when needed.
    fn play_track(&mut self, track: Track) {
        match &track.source {
            TrackSource::Local { path } => {
                let path = path.clone();
                self.enqueue(&track);
                self.current_index = self.queue.iter().position(|item| item.id == track.id);
                self.title = track.title.clone();
                self.artist = track.artist.clone();
                self.album = track.album.clone().unwrap_or_default();

                if let Some(engine) = self.engine.as_ref() {
                    engine.play(track, MediaSource::file(path));
                } else {
                    self.error = Some("no audio output device is available".to_string());
                }
            }
            TrackSource::YouTubeMusic { .. } => {
                self.enqueue(&track);
                self.current_index = self.queue.iter().position(|item| item.id == track.id);
                self.resolve_and_play(track);
            }
        }
    }

    /// Adds a track to the queue, if it is not there already.
    fn enqueue(&mut self, track: &Track) {
        if !self.queue.iter().any(|item| item.id == track.id) {
            self.queue.push(track.clone());
        }
    }

    /// Plays the track of the queue at the given index.
    fn play_queue_index(&mut self, index: usize) {
        let Some(track) = self.queue.get(index).cloned() else {
            return;
        };
        self.current_index = Some(index);
        self.play_track(track);
    }

    /// Advances to the next track of the queue.
    fn play_next(&mut self) {
        if self.queue.is_empty() {
            return;
        }

        let next = if self.shuffle && self.queue.len() > 1 {
            let mut index = self.current_index.unwrap_or(0);
            while index == self.current_index.unwrap_or(usize::MAX) {
                index = rand_index(self.queue.len());
            }
            Some(index)
        } else {
            match self.repeat {
                RepeatMode::One => self.current_index,
                _ => match self.current_index.map(|index| index + 1) {
                    Some(index) if index < self.queue.len() => Some(index),
                    Some(_) => match self.repeat {
                        RepeatMode::All => Some(0),
                        _ => None,
                    },
                    None => Some(0),
                },
            }
        };

        match next {
            Some(index) => self.play_queue_index(index),
            None => {
                if let Some(engine) = self.engine.as_ref() {
                    engine.stop();
                }
                self.current_index = None;
                self.title.clear();
                self.artist.clear();
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
                    let track = self.results[0].clone();
                    self.play_track(track);
                } else if !self.library.is_empty() {
                    let track = self.library.tracks[0].to_track();
                    self.play_track(track);
                }
            }
        }
    }

    // --------------------------------------------------------------- helpers

    /// Handles the messages coming from the background tasks.
    fn drain_messages(&mut self, ctx: &egui::Context) {
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
                    self.title = track.title.clone();
                    self.artist = track.artist.clone();

                    if let Some(thumbnail) = track.thumbnail.clone() {
                        self.fetch_cover(&thumbnail);
                    }

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
                    self.error = Some(err);
                }
                AppMessage::Thumbnail(url, bytes) => {
                    if let Some(texture) = decode_cover(ctx, &bytes) {
                        self.cover = Some(texture.clone());
                        self.covers.insert(url, texture);
                    }
                }
            }
        }
    }

    /// Handles the events coming from the audio engine.
    fn drain_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                PlayerEvent::TrackEnded => self.play_next(),
                PlayerEvent::StateChanged(PlaybackState::Stopped) => {
                    self.title.clear();
                    self.artist.clear();
                }
                PlayerEvent::Error(err) => self.error = Some(err),
                _ => {}
            }
        }
    }

    /// Handles the global keyboard shortcuts.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (space, next, previous, volume_up, volume_down, stop, save) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::Space),
                input.key_pressed(egui::Key::ArrowRight) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::ArrowLeft) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::ArrowUp) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::ArrowDown) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::S) && input.modifiers.ctrl,
                input.key_pressed(egui::Key::S) && input.modifiers.shift,
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
        if save {
            self.save();
        }
        if volume_up || volume_down {
            let delta = if volume_up { 0.05 } else { -0.05 };
            self.volume = (self.volume + delta).clamp(0.0, 1.0);
            if let Some(engine) = self.engine.as_ref() {
                engine.set_volume(self.volume);
            }
        }
    }

    /// Handles the actions of the transport bar.
    fn handle_player_actions(&mut self, actions: PlayerActions) {
        if actions.toggle_play {
            self.toggle_playback();
        }
        if actions.previous {
            self.play_previous();
        }
        if actions.next {
            self.play_next();
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
        if actions.shuffle {
            self.shuffle = !self.shuffle;
        }
        if actions.repeat {
            self.repeat = match self.repeat {
                RepeatMode::Off => RepeatMode::All,
                RepeatMode::All => RepeatMode::One,
                RepeatMode::One => RepeatMode::Off,
            };
        }
        if actions.equalizer {
            self.show_equalizer = !self.show_equalizer;
        }
        if actions.mini_player {
            self.show_mini_player = !self.show_mini_player;
            if !self.show_mini_player {
                mini_player::close(self.ui_ctx());
            }
        }
    }

    /// Handles the actions of the mini player.
    fn handle_mini_actions(&mut self) {
        let actions = {
            let mut sink = self.mini_sink.lock().unwrap_or_else(|err| err.into_inner());
            crate::mini_player::MiniPlayerActions {
                toggle_play: std::mem::take(&mut sink.toggle_play),
                previous: std::mem::take(&mut sink.previous),
                next: std::mem::take(&mut sink.next),
                seek: sink.seek.take(),
                volume: sink.volume.take(),
                close: std::mem::take(&mut sink.close),
            }
        };

        if actions.toggle_play {
            self.toggle_playback();
        }
        if actions.previous {
            self.play_previous();
        }
        if actions.next {
            self.play_next();
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
        if actions.close {
            self.show_mini_player = false;
        }
    }

    /// The egui context, stored so that the mini player can be closed.
    fn ui_ctx(&self) -> &egui::Context {
        self.ctx.as_ref().expect("the context is always set")
    }

    // ------------------------------------------------------------------ ui

    /// Draws the top bar with the search field and the tabs.
    fn show_top_bar(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        ui.horizontal(|ui| {
            let response = ui.add_sized(
                Vec2::new(340.0, 30.0),
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Search YouTube Music…")
                    .desired_width(340.0),
            );

            let submit =
                response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if ui.button("Search").clicked() || submit {
                self.start_search();
            }

            if self.searching {
                ui.spinner();
            }

            ui.add_space(16.0);

            for view in [View::Online, View::Library, View::Queue] {
                let selected = self.view == view;
                let label = match view {
                    View::Queue => {
                        let count = self.queue.len();
                        format!("{} ({count})", view.label())
                    }
                    _ => view.label().to_string(),
                };

                if ui
                    .selectable_label(selected, label)
                    .on_hover_text(match view {
                        View::Online => "Search YouTube Music",
                        View::Library => "Your local music",
                        View::Queue => "The songs that will play next",
                    })
                    .clicked()
                {
                    self.view = view;
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Add folder…").clicked() {
                    self.add_library_folder();
                }
                if ui.button("Save").clicked() {
                    self.save();
                }
            });
        });

        if let Some(error) = &self.error {
            ui.colored_label(theme.warning, error);
        }
    }

    /// Opens the folder dialog and adds the folder to the library.
    fn add_library_folder(&mut self) {
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Add a music folder")
            .pick_folder()
        else {
            return;
        };

        match self.library.add_root(folder) {
            Ok(()) => {
                self.view = View::Library;
                self.error = None;
            }
            Err(err) => self.error = Some(err.to_string()),
        }
    }

    /// Draws the list of the current view.
    fn show_list(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        match self.view {
            View::Online => self.show_online_list(ui, theme),
            View::Library => self.show_library(ui, theme),
            View::Queue => self.show_queue(ui, theme),
        }
    }

    /// Draws the results of the YouTube Music search.
    fn show_online_list(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        if self.results.is_empty() {
            ui.weak(if self.has_youtube_music() {
                "Search for a song to fill the list."
            } else {
                "yt-dlp is not installed: install it to stream from YouTube Music."
            });
            return;
        }

        let mut to_play = None;
        let mut to_add = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, track) in self.results.iter().enumerate() {
                ui.horizontal(|ui| {
                    let selected = self.queue.get(self.current_index.unwrap_or(0)) == Some(track);

                    let title = ui.add(
                        egui::Label::new(RichText::new(&track.title).strong().color(if selected {
                            theme.accent
                        } else {
                            theme.text
                        }))
                        .truncate(),
                    );
                    let artist = ui.add(
                        egui::Label::new(RichText::new(&track.artist).color(theme.text_dim))
                            .truncate(),
                    );

                    if title.double_clicked() || artist.double_clicked() {
                        to_play = Some(index);
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .small_button("+")
                            .on_hover_text("Add to the queue")
                            .clicked()
                        {
                            to_add = Some(index);
                        }
                        ui.label(
                            RichText::new(
                                track
                                    .duration
                                    .map(crate::player::format_time)
                                    .unwrap_or_default(),
                            )
                            .size(11.0)
                            .color(theme.text_dim)
                            .monospace(),
                        );
                    });
                });
                ui.separator();
            }
        });

        if let Some(index) = to_add {
            let track = self.results[index].clone();
            self.enqueue(&track);
        }
        if let Some(index) = to_play {
            let track = self.results[index].clone();
            self.play_track(track);
        }
    }

    /// Draws the local library.
    fn show_library(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        if self.library.roots.is_empty() {
            ui.weak("No folder yet. Use “Add folder…” to pick your music.");
            return;
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new("Folder").color(theme.text_dim));
            for root in self.library.roots.clone() {
                ui.horizontal(|ui| {
                    ui.label(root.display().to_string());
                    if ui.small_button("✕").on_hover_text("Remove").clicked() {
                        self.library.remove_root(&root);
                    }
                });
            }
            if ui.button("Rescan").clicked() {
                self.library.rescan();
            }
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Filter").color(theme.text_dim));
            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text("Artist, album or title…")
                    .desired_width(280.0),
            );
            ui.label(
                RichText::new(format!("{} tracks", self.library.len()))
                    .size(11.0)
                    .color(theme.text_dim),
            );
        });

        ui.separator();

        let filter = self.filter.to_lowercase();
        let mut to_play = None;
        let mut to_add = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, track) in self.library.tracks.iter().enumerate() {
                if !filter.is_empty() {
                    let haystack = format!(
                        "{} {} {}",
                        track.artist.to_lowercase(),
                        track.album.to_lowercase(),
                        track.title.to_lowercase()
                    );
                    if !haystack.contains(&filter) {
                        continue;
                    }
                }

                ui.horizontal(|ui| {
                    let is_current = self
                        .current_index
                        .map(|current| {
                            self.queue
                                .get(current)
                                .map(|item| item.id == track.path.display().to_string())
                                .unwrap_or(false)
                        })
                        .unwrap_or(false);

                    let title = ui.add(
                        egui::Label::new(
                            RichText::new(&track.title).strong().color(if is_current {
                                theme.accent
                            } else {
                                theme.text
                            }),
                        )
                        .truncate(),
                    );
                    let artist = ui.add(
                        egui::Label::new(RichText::new(&track.artist).color(theme.text_dim))
                            .truncate(),
                    );

                    if title.double_clicked() || artist.double_clicked() {
                        to_play = Some(index);
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .small_button("+")
                            .on_hover_text("Add to the queue")
                            .clicked()
                        {
                            to_add = Some(index);
                        }
                        ui.label(
                            RichText::new(
                                track
                                    .duration
                                    .map(crate::player::format_time)
                                    .unwrap_or_default(),
                            )
                            .size(11.0)
                            .color(theme.text_dim)
                            .monospace(),
                        );
                        if !track.album.is_empty() {
                            ui.label(RichText::new(&track.album).size(11.0).color(theme.text_dim));
                        }
                    });
                });
            }
        });

        if let Some(index) = to_add {
            let track = self.library.tracks[index].to_track();
            self.enqueue(&track);
        }
        if let Some(index) = to_play {
            let track = self.library.tracks[index].to_track();
            self.play_track(track);
        }
    }

    /// Draws the play queue.
    fn show_queue(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        if self.queue.is_empty() {
            ui.weak("The queue is empty.");
            return;
        }

        let mut to_play = None;
        let mut to_remove = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, track) in self.queue.iter().enumerate() {
                ui.horizontal(|ui| {
                    let is_current = self.current_index == Some(index);
                    let title = ui.add(
                        egui::Label::new(
                            RichText::new(&track.title).strong().color(if is_current {
                                theme.accent
                            } else {
                                theme.text
                            }),
                        )
                        .truncate(),
                    );
                    if title.double_clicked() {
                        to_play = Some(index);
                    }
                    ui.add(
                        egui::Label::new(RichText::new(&track.artist).color(theme.text_dim))
                            .truncate(),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("✕").on_hover_text("Remove").clicked() {
                            to_remove = Some(index);
                        }
                    });
                });
            }
        });

        if let Some(index) = to_remove {
            self.queue.remove(index);
            if self.current_index == Some(index) {
                self.current_index = None;
            } else if self.current_index.map(|current| current > index) == Some(true) {
                self.current_index = self.current_index.map(|current| current - 1);
            }
        }
        if let Some(index) = to_play {
            self.play_queue_index(index);
        }
    }

    /// Draws the equalizer window.
    fn show_equalizer_window(&mut self, ctx: &egui::Context) {
        if !self.show_equalizer {
            return;
        }

        let mut open = self.show_equalizer;
        egui::Window::new("Equalizer")
            .open(&mut open)
            .resizable(false)
            .default_width(430.0)
            .show(ctx, |ui| {
                let theme = Theme::from_visuals(ui.visuals());
                let actions = crate::equalizer::show(ui, &mut self.equalizer, &theme);

                if let Some(bands) = actions.bands {
                    if let Some(engine) = self.engine.as_ref() {
                        engine.set_equalizer_bands(bands);
                    }
                }
                if let Some(preamp) = actions.preamp {
                    if let Some(engine) = self.engine.as_ref() {
                        engine.set_equalizer_preamp(preamp);
                    }
                }
                if let Some(enabled) = actions.enabled {
                    if let Some(engine) = self.engine.as_ref() {
                        engine.set_equalizer_enabled(enabled);
                    }
                }
                if actions.reset {
                    if let Some(engine) = self.engine.as_ref() {
                        engine.set_equalizer_bands(self.equalizer.bands);
                        engine.set_equalizer_preamp(self.equalizer.preamp);
                    }
                }
            });

        self.show_equalizer = open;
    }

    /// Draws the mini player.
    fn show_mini_player(&mut self, ctx: &egui::Context) {
        if !self.show_mini_player {
            return;
        }

        let state = MiniPlayerState {
            title: self.title.clone(),
            artist: self.artist.clone(),
            state: self
                .engine
                .as_ref()
                .map(|engine| engine.state())
                .unwrap_or(PlaybackState::Stopped),
            position: self
                .engine
                .as_ref()
                .map(|engine| engine.position())
                .unwrap_or(0.0),
            duration: self
                .engine
                .as_ref()
                .map(|engine| engine.duration())
                .unwrap_or(0.0),
            volume: self.volume,
        };

        mini_player::show(ctx, state, self.mini_sink.clone());
    }
}

/// Decodes the cover art into a texture.
fn decode_cover(ctx: &egui::Context, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &image);
    Some(ctx.load_texture("cover", color_image, egui::TextureOptions::LINEAR))
}

/// Picks a pseudo random index below `len`.
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
        self.ctx = Some(ui.ctx().clone());

        self.drain_messages(ui.ctx());
        self.drain_events();
        self.handle_shortcuts(ui.ctx());
        self.handle_mini_actions();

        // Feed the visualizer with the samples of the output.
        if let Some(engine) = self.engine.as_ref() {
            let samples = engine.drain_tap();
            if samples.is_empty() {
                self.visualizer.decay();
            } else {
                self.visualizer.push_mono(&samples);
            }
        }

        let theme = Theme::from_visuals(ui.visuals());

        egui::Panel::top("top")
            .frame(
                egui::Frame::new()
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .stroke(egui::Stroke::new(1.0, theme.border)),
            )
            .show(ui, |ui| {
                self.show_top_bar(ui, &theme);
            });

        egui::Panel::bottom("player")
            .frame(
                egui::Frame::new()
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .stroke(egui::Stroke::new(1.0, theme.border)),
            )
            .show(ui, |ui| {
                let actions = PlayerBar {
                    title: &self.title,
                    artist: &self.artist,
                    playing: self
                        .engine
                        .as_ref()
                        .map(|engine| engine.state() == PlaybackState::Playing)
                        .unwrap_or(false),
                    paused: self
                        .engine
                        .as_ref()
                        .map(|engine| engine.state() == PlaybackState::Paused)
                        .unwrap_or(false),
                    position: self
                        .engine
                        .as_ref()
                        .map(|engine| engine.position())
                        .unwrap_or(0.0),
                    duration: self
                        .engine
                        .as_ref()
                        .map(|engine| engine.duration())
                        .unwrap_or(0.0),
                    volume: self.volume,
                    shuffle: self.shuffle,
                    repeat: self.repeat != RepeatMode::Off,
                    cover: self.cover.clone(),
                }
                .show(ui, &theme);

                self.handle_player_actions(actions);
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme.background)
                    .inner_margin(egui::Margin::symmetric(12, 10)),
            )
            .show(ui, |ui| {
                self.show_list(ui, &theme);
            });

        self.show_equalizer_window(ui.ctx());
        self.show_mini_player(ui.ctx());

        // Keep the time display and the visualizer up to date.
        ui.ctx().request_repaint_after(Duration::from_millis(50));
    }
}
