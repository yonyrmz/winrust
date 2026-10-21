//! The transport bar shown at the bottom of the main window.

use egui::{Align, Color32, Layout, RichText, Sense, Vec2};
use winrust_core::PlaybackState;

use crate::icons::{self, Icon};
use crate::theme::Theme;

/// A clickable icon button that can be highlighted.
fn icon_button(ui: &mut egui::Ui, theme: &Theme, icon: Icon, tooltip: &str, active: bool) -> bool {
    let size = Vec2::new(30.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    let color = if active || hovered {
        theme.accent
    } else {
        theme.text
    };

    if active || hovered {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(6), theme.surface_alt);
    }
    icons::draw(ui.painter(), rect, icon, color);

    response.on_hover_text(tooltip).clicked()
}

/// The actions the user performed.
#[derive(Debug, Default)]
pub struct PlayerActions {
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
    /// The shuffle button was pressed.
    pub shuffle: bool,
    /// The repeat button was pressed.
    pub repeat: bool,
    /// The equalizer button was pressed.
    pub equalizer: bool,
    /// The mini player button was pressed.
    pub mini_player: bool,
}

/// The state shown by the transport bar.
pub struct PlayerBar<'a> {
    /// The title of the current track.
    pub title: &'a str,
    /// The artist of the current track.
    pub artist: &'a str,
    /// Whether the player is playing.
    pub playing: bool,
    /// Whether the player is paused.
    pub paused: bool,
    /// The current position in seconds.
    pub position: f64,
    /// The duration in seconds.
    pub duration: f64,
    /// The master volume.
    pub volume: f32,
    /// Whether shuffle is enabled.
    pub shuffle: bool,
    /// Whether repeat is enabled.
    pub repeat: bool,
    /// The cover art, when there is one.
    pub cover: Option<egui::TextureHandle>,
}

impl<'a> PlayerBar<'a> {
    /// Draws the bar and returns the user actions.
    pub fn show(&self, ui: &mut egui::Ui, theme: &Theme) -> PlayerActions {
        let mut actions = PlayerActions::default();

        let frame = egui::Frame::new()
            .fill(theme.surface)
            .inner_margin(egui::Margin::symmetric(14, 8))
            .stroke(egui::Stroke::new(1.0, theme.border));

        frame.show(ui, |ui| {
            // The first row: the cover, the title and the controls.
            ui.horizontal(|ui| {
                self.show_cover(ui, theme);

                ui.vertical(|ui| {
                    ui.add_space(2.0);
                    let title = if self.title.is_empty() {
                        "Nothing playing"
                    } else {
                        self.title
                    };
                    ui.label(RichText::new(title).size(14.0).strong().color(
                        if self.title.is_empty() {
                            theme.text_dim
                        } else {
                            theme.text
                        },
                    ));
                    ui.label(RichText::new(self.artist).size(12.0).color(theme.text_dim));
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, theme, Icon::MiniPlayer, "Mini player", false) {
                        actions.mini_player = true;
                    }
                    if icon_button(ui, theme, Icon::Equalizer, "Equalizer", false) {
                        actions.equalizer = true;
                    }
                    if icon_button(ui, theme, Icon::Repeat, "Repeat", self.repeat) {
                        actions.repeat = true;
                    }
                    if icon_button(ui, theme, Icon::Shuffle, "Shuffle", self.shuffle) {
                        actions.shuffle = true;
                    }

                    ui.add_space(10.0);

                    let (rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
                    icons::draw(ui.painter(), rect, Icon::Volume, theme.text_dim);

                    let mut volume = self.volume;
                    if ui
                        .add_sized(
                            Vec2::new(90.0, 16.0),
                            egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false),
                        )
                        .changed()
                    {
                        actions.volume = Some(volume);
                    }
                });
            });

            ui.add_space(6.0);

            // The second row: the transport and the progress.
            ui.horizontal(|ui| {
                if icon_button(ui, theme, Icon::Previous, "Previous", false) {
                    actions.previous = true;
                }
                if icon_button(
                    ui,
                    theme,
                    if self.playing {
                        Icon::Pause
                    } else {
                        Icon::Play
                    },
                    "Play or pause",
                    self.playing,
                ) {
                    actions.toggle_play = true;
                }
                if icon_button(ui, theme, Icon::Next, "Next", false) {
                    actions.next = true;
                }

                ui.add_space(10.0);

                self.show_progress(ui, theme, &mut actions);
            });
        });

        actions
    }

    /// Draws the cover art.
    fn show_cover(&self, ui: &mut egui::Ui, theme: &Theme) {
        let size = Vec2::splat(48.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());

        match &self.cover {
            Some(texture) => {
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(6), theme.surface_alt);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "♪",
                    egui::FontId::proportional(20.0),
                    theme.text_dim,
                );
            }
        }

        response.on_hover_text("Now playing");
    }

    /// Draws the seek bar and the elapsed time.
    fn show_progress(&self, ui: &mut egui::Ui, theme: &Theme, actions: &mut PlayerActions) {
        let progress = if self.duration > 0.0 {
            (self.position / self.duration).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        ui.label(
            RichText::new(format_time(self.position))
                .size(11.0)
                .color(theme.text_dim)
                .monospace(),
        );

        let mut value = progress;
        let response = ui.add_sized(
            Vec2::new(ui.available_width() - 60.0, 14.0),
            egui::Slider::new(&mut value, 0.0..=1.0)
                .show_value(false)
                .custom_formatter(|_, _| String::new()),
        );

        if response.changed() || response.drag_stopped() {
            actions.seek = Some(value);
        }

        ui.label(
            RichText::new(format_time(self.duration))
                .size(11.0)
                .color(theme.text_dim)
                .monospace(),
        );
    }
}

/// Formats a number of seconds as `m:ss`.
pub fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "0:00".to_string();
    }
    let total = seconds as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

/// The colour of the playback state.
pub fn state_color(state: PlaybackState, theme: &Theme) -> Color32 {
    match state {
        PlaybackState::Playing => theme.accent,
        PlaybackState::Paused => theme.text_dim,
        PlaybackState::Stopped => theme.text_dim,
    }
}
