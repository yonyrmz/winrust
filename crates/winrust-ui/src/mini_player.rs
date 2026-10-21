//! The compact mini player, shown in its own window.

use std::sync::{Arc, Mutex};

use egui::{Align, Context, Layout, RichText, Sense, Vec2, ViewportBuilder, ViewportId};
use winrust_core::PlaybackState;

use crate::icons::{self, icon_button, Icon};
use crate::theme::Theme;

/// The identifier of the mini player viewport.
pub const MINI_PLAYER_ID: &str = "winrust-mini-player";

/// The actions the user performed in the mini player.
#[derive(Debug, Default, Clone)]
pub struct MiniPlayerActions {
    /// The play or pause button was pressed.
    pub toggle_play: bool,
    /// The previous button was pressed.
    pub previous: bool,
    /// The next button was pressed.
    pub next: bool,
    /// The user seeked to a new position, in the `0.0..=1.0` range.
    pub seek: Option<f32>,
    /// The user changed the volume.
    pub volume: Option<f32>,
    /// The close button was pressed.
    pub close: bool,
}

/// A channel that carries the actions of the mini player back to the app.
pub type ActionSink = Arc<Mutex<MiniPlayerActions>>;

/// Creates the channel used to send the actions back.
pub fn action_sink() -> ActionSink {
    Arc::new(Mutex::new(MiniPlayerActions::default()))
}

/// The state shown by the mini player.
pub struct MiniPlayerState {
    /// The title of the current track.
    pub title: String,
    /// The artist of the current track.
    pub artist: String,
    /// The playback state.
    pub state: PlaybackState,
    /// The current position in seconds.
    pub position: f64,
    /// The duration in seconds.
    pub duration: f64,
    /// The master volume.
    pub volume: f32,
}

impl Default for MiniPlayerState {
    fn default() -> Self {
        Self {
            title: String::new(),
            artist: String::new(),
            state: PlaybackState::Stopped,
            position: 0.0,
            duration: 0.0,
            volume: 1.0,
        }
    }
}

/// The viewport builder of the mini player.
pub fn viewport_builder() -> ViewportBuilder {
    ViewportBuilder::default()
        .with_title("winrust")
        .with_titlebar_shown(false)
        .with_inner_size(Vec2::new(420.0, 104.0))
        .with_min_inner_size(Vec2::new(320.0, 92.0))
        .with_resizable(false)
        .with_always_on_top()
}

/// Shows the mini player in its own window.
pub fn show(ctx: &Context, state: MiniPlayerState, sink: ActionSink) {
    let id = ViewportId::from_hash_of(MINI_PLAYER_ID);

    ctx.show_viewport_deferred(id, viewport_builder(), move |ui, _class| {
        let theme = Theme::from_visuals(ui.visuals());
        let mut actions = MiniPlayerActions::default();

        let frame = egui::Frame::new()
            .fill(theme.surface)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .corner_radius(egui::CornerRadius::same(10))
            .stroke(egui::Stroke::new(1.0, theme.border));

        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                let playing = state.state == PlaybackState::Playing;

                if icon_button(
                    ui,
                    &theme,
                    if playing { Icon::Pause } else { Icon::Play },
                    "Play or pause",
                ) {
                    actions.toggle_play = true;
                }

                if icon_button(ui, &theme, Icon::Previous, "Previous") {
                    actions.previous = true;
                }

                if icon_button(ui, &theme, Icon::Next, "Next") {
                    actions.next = true;
                }

                ui.add_space(6.0);

                ui.vertical(|ui| {
                    ui.add_space(1.0);
                    ui.label(
                        RichText::new(if state.title.is_empty() {
                            "Nothing playing"
                        } else {
                            &state.title
                        })
                        .size(13.0)
                        .strong()
                        .color(if state.title.is_empty() {
                            theme.text_dim
                        } else {
                            theme.text
                        }),
                    );
                    ui.label(
                        RichText::new(&state.artist)
                            .size(11.0)
                            .color(theme.text_dim),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, &theme, Icon::Close, "Close the mini player") {
                        actions.close = true;
                    }

                    ui.add_space(4.0);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
                    icons::draw(ui.painter(), rect, Icon::Volume, theme.text_dim);

                    let mut volume = state.volume;
                    if ui
                        .add_sized(
                            Vec2::new(70.0, 16.0),
                            egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false),
                        )
                        .changed()
                    {
                        actions.volume = Some(volume);
                    }
                });
            });

            // The progress bar.
            let progress = if state.duration > 0.0 {
                (state.position / state.duration).clamp(0.0, 1.0) as f32
            } else {
                0.0
            };
            let mut value = progress;
            let response = ui.add_sized(
                Vec2::new(ui.available_width(), 12.0),
                egui::Slider::new(&mut value, 0.0..=1.0)
                    .show_value(false)
                    .custom_formatter(|_, _| String::new()),
            );
            if response.changed() || response.drag_stopped() {
                actions.seek = Some(value);
            }
        });

        let mut sink = sink.lock().unwrap_or_else(|err| err.into_inner());
        sink.toggle_play |= actions.toggle_play;
        sink.previous |= actions.previous;
        sink.next |= actions.next;
        sink.close |= actions.close;
        if actions.seek.is_some() {
            sink.seek = actions.seek;
        }
        if actions.volume.is_some() {
            sink.volume = actions.volume;
        }
    });
}

/// Closes the mini player window.
pub fn close(ctx: &Context) {
    let id = ViewportId::from_hash_of(MINI_PLAYER_ID);
    ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Close);
}
