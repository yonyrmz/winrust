//! The Winamp style main window.

use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use winrust_skin::image::Rect as SkinRect;
use winrust_skin::sprites::{
    self, ButtonState, ToggleButton, ToggleSprites, TransportButton, TransportSprites,
    WindowSprites,
};
use winrust_skin::Skin;

use crate::renderer::{draw_digits, draw_sprite, draw_visualizer, TextureCache};
use crate::theme::Theme;

/// The state of the player as shown by the main window.
#[derive(Debug, Clone, Default)]
pub struct MainWindowState {
    /// Whether the player is currently playing.
    pub playing: bool,
    /// Whether the player is paused.
    pub paused: bool,
    /// The current position in seconds.
    pub position: f64,
    /// The duration of the current track in seconds.
    pub duration: f64,
    /// The master volume in the `0.0..=1.0` range.
    pub volume: f32,
    /// The balance in the `-1.0..=1.0` range.
    pub balance: f32,
    /// Whether shuffle is enabled.
    pub shuffle: bool,
    /// Whether repeat is enabled.
    pub repeat: bool,
    /// The title of the current track.
    pub title: String,
    /// The artist of the current track.
    pub artist: String,
    /// The spectrum analyzer values.
    pub visualizer: Vec<f32>,
    /// Whether the equalizer window is open.
    pub equalizer_open: bool,
    /// Whether the playlist window is open.
    pub playlist_open: bool,
}

/// The interactions the user performed on the main window.
#[derive(Debug, Clone, Default)]
pub struct MainWindowActions {
    /// A transport button was pressed.
    pub transport: Option<TransportButton>,
    /// A toggle button was pressed.
    pub toggle: Option<ToggleButton>,
    /// The user seeked to a new position, in the `0.0..=1.0` range.
    pub seek: Option<f32>,
    /// The user changed the volume.
    pub volume: Option<f32>,
    /// The user changed the balance.
    pub balance: Option<f32>,
}

/// The Winamp style main window widget.
pub struct MainWindow<'a> {
    /// The loaded skin.
    pub skin: &'a Skin,
    /// The uploaded textures of the skin.
    pub textures: &'a TextureCache,
    /// The colour theme.
    pub theme: &'a Theme,
    /// The zoom factor applied to the skin.
    pub scale: f32,
}

impl<'a> MainWindow<'a> {
    /// Creates the widget.
    pub fn new(skin: &'a Skin, textures: &'a TextureCache, theme: &'a Theme, scale: f32) -> Self {
        Self {
            skin,
            textures,
            theme,
            scale,
        }
    }

    /// The size of the window in points.
    pub fn size(&self) -> Vec2 {
        Vec2::new(
            sprites::MAIN_WIDTH as f32 * self.scale,
            sprites::MAIN_HEIGHT as f32 * self.scale,
        )
    }

    /// Draws the main window and returns the user interactions.
    pub fn show(&self, ui: &mut egui::Ui, state: &MainWindowState) -> MainWindowActions {
        let mut actions = MainWindowActions::default();

        let (response, painter) = ui.allocate_painter(self.size(), Sense::click());
        let origin = response.rect.min;

        // The window background.
        if let Some(texture) = self.textures.get("main.bmp") {
            draw_sprite(
                &painter,
                &texture,
                SkinRect::new(0, 0, sprites::MAIN_WIDTH, sprites::MAIN_HEIGHT),
                response.rect,
            );
        } else {
            painter.rect_filled(
                response.rect,
                egui::CornerRadius::ZERO,
                self.theme.background,
            );
        }

        self.draw_title_bar(&painter, origin, state);
        self.draw_time(&painter, origin, state);
        self.draw_ticker(&painter, origin, state);
        self.draw_visualizer(&painter, origin, state);
        self.draw_position_bar(ui, &painter, origin, state, &mut actions);
        self.draw_volume(ui, &painter, origin, state, &mut actions);
        self.draw_balance(ui, &painter, origin, state, &mut actions);
        self.draw_transport(ui, &painter, origin, state, &mut actions);
        self.draw_toggles(ui, &painter, origin, state, &mut actions);

        actions
    }

    /// Converts skin coordinates into screen coordinates.
    fn point(&self, origin: Pos2, x: u32, y: u32) -> Pos2 {
        Pos2::new(
            origin.x + x as f32 * self.scale,
            origin.y + y as f32 * self.scale,
        )
    }

    /// Converts a skin rectangle into a screen rectangle.
    fn rect(&self, origin: Pos2, rect: SkinRect) -> Rect {
        Rect::from_min_size(
            self.point(origin, rect.x, rect.y),
            Vec2::new(
                rect.width as f32 * self.scale,
                rect.height as f32 * self.scale,
            ),
        )
    }

    /// Draws the title bar.
    fn draw_title_bar(&self, painter: &egui::Painter, origin: Pos2, state: &MainWindowState) {
        let Some(texture) = self.textures.get("titlebar.bmp") else {
            return;
        };

        let source = if state.playing || state.paused {
            WindowSprites::MAIN.titlebar_active
        } else {
            WindowSprites::MAIN.titlebar_inactive
        };
        draw_sprite(
            painter,
            &texture,
            source,
            self.rect(origin, SkinRect::new(0, 0, sprites::MAIN_WIDTH, 14)),
        );
    }

    /// Draws the time display.
    fn draw_time(&self, painter: &egui::Painter, origin: Pos2, state: &MainWindowState) {
        let Some(texture) = self.textures.get("numbers.bmp") else {
            return;
        };

        let total = if state.position.is_finite() && state.position > 0.0 {
            state.position as u64
        } else {
            0
        };
        let minutes = (total / 60) % 100;
        let seconds = total % 60;

        let digits = [
            (minutes / 10) as u32,
            (minutes % 10) as u32,
            (seconds / 10) as u32,
            (seconds % 10) as u32,
        ];

        draw_digits(
            painter,
            &texture,
            &sprites::TIME_DIGIT_POSITIONS,
            &digits,
            origin,
            self.scale,
        );
    }

    /// Draws the scrolling song title.
    fn draw_ticker(&self, painter: &egui::Painter, origin: Pos2, state: &MainWindowState) {
        let text = if state.title.is_empty() {
            "winrust".to_string()
        } else if state.artist.is_empty() {
            state.title.clone()
        } else {
            format!("{} - {}", state.artist, state.title)
        };

        let rect = self.rect(origin, sprites::TICKER_RECT);
        painter.text(
            rect.left_center(),
            Align2::LEFT_CENTER,
            text,
            FontId::monospace(6.0 * self.scale.max(1.0)),
            self.theme.accent,
        );
    }

    /// Draws the spectrum analyzer.
    fn draw_visualizer(&self, painter: &egui::Painter, origin: Pos2, state: &MainWindowState) {
        let rect = self.rect(origin, sprites::VISUALIZER_RECT);
        draw_visualizer(painter, rect, &state.visualizer, self.theme);
    }

    /// Draws the position bar and detects seeking.
    fn draw_position_bar(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        origin: Pos2,
        state: &MainWindowState,
        actions: &mut MainWindowActions,
    ) {
        let bar_rect = self.rect(origin, sprites::POSITION_BAR_RECT);
        if let Some(texture) = self.textures.get("posbar.bmp") {
            draw_sprite(painter, &texture, sprites::POSITION_BAR_SOURCE, bar_rect);
        }

        let progress = if state.duration > 0.0 {
            (state.position / state.duration).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        let thumb_width = 29.0 * self.scale;
        let travel = (bar_rect.width() - thumb_width).max(1.0);
        let thumb_x = bar_rect.min.x + travel * progress;
        let thumb_rect = Rect::from_min_size(
            Pos2::new(thumb_x, bar_rect.min.y),
            Vec2::new(thumb_width, bar_rect.height()),
        );
        if let Some(texture) = self.textures.get("posbar.bmp") {
            draw_sprite(
                painter,
                &texture,
                sprites::POSITION_THUMB_SOURCE,
                thumb_rect,
            );
        }

        let response = ui.interact(bar_rect, ui.scope_id().with("position"), Sense::click());
        if response.clicked() {
            if let Some(position) = response.interact_pointer_pos().map(|pointer| {
                let relative = (pointer.x - bar_rect.min.x - thumb_width / 2.0) / travel;
                relative.clamp(0.0, 1.0)
            }) {
                actions.seek = Some(position);
            }
        }
    }

    /// Draws the volume slider and detects changes.
    fn draw_volume(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        origin: Pos2,
        state: &MainWindowState,
        actions: &mut MainWindowActions,
    ) {
        let rect = self.rect(
            origin,
            SkinRect::new(
                sprites::VOLUME_POSITION.0,
                sprites::VOLUME_POSITION.1,
                68,
                13,
            ),
        );

        if let Some(texture) = self.textures.get("volume.bmp") {
            let frame = ((1.0 - state.volume.clamp(0.0, 1.0)) * 27.0).round() as u32;
            draw_sprite(
                painter,
                &texture,
                sprites::volume_frame_rect(frame.min(27)),
                rect,
            );
        }

        let response = ui.interact(rect, ui.scope_id().with("volume"), Sense::click());
        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let relative = ((pointer.x - rect.min.x) / rect.width().max(1.0)).clamp(0.0, 1.0);
                actions.volume = Some(1.0 - relative);
            }
        }
    }

    /// Draws the balance slider and detects changes.
    fn draw_balance(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        origin: Pos2,
        state: &MainWindowState,
        actions: &mut MainWindowActions,
    ) {
        let rect = self.rect(
            origin,
            SkinRect::new(
                sprites::BALANCE_POSITION.0,
                sprites::BALANCE_POSITION.1,
                38,
                13,
            ),
        );

        if let Some(texture) = self.textures.get("balance.bmp") {
            let frame = (14.0 + state.balance.clamp(-1.0, 1.0) * 14.0).round() as u32;
            draw_sprite(
                painter,
                &texture,
                sprites::balance_frame_rect(frame.clamp(0, 27)),
                rect,
            );
        }

        let response = ui.interact(rect, ui.scope_id().with("balance"), Sense::click());
        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let relative = ((pointer.x - rect.min.x) / rect.width().max(1.0)).clamp(0.0, 1.0);
                actions.balance = Some(relative * 2.0 - 1.0);
            }
        }
    }

    /// Draws the transport buttons and detects clicks.
    fn draw_transport(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        origin: Pos2,
        _state: &MainWindowState,
        actions: &mut MainWindowActions,
    ) {
        let Some(texture) = self.textures.get("cbuttons.bmp") else {
            return;
        };

        let buttons = [
            TransportButton::Previous,
            TransportButton::Play,
            TransportButton::Pause,
            TransportButton::Stop,
            TransportButton::Next,
            TransportButton::Eject,
        ];

        for button in buttons {
            let source = TransportSprites::rect(button, ButtonState::Normal);
            let target = self.rect(
                origin,
                SkinRect::new(
                    sprites::TRANSPORT_POSITION.0 + source.x,
                    sprites::TRANSPORT_POSITION.1 + source.y,
                    source.width,
                    source.height,
                ),
            );

            let response = ui.interact(
                target,
                ui.scope_id().with(("transport", button)),
                Sense::click(),
            );
            let state = if response.hovered() && response.is_pointer_button_down_on() {
                ButtonState::Pressed
            } else {
                ButtonState::Normal
            };

            let source = TransportSprites::rect(button, state);
            draw_sprite(painter, &texture, source, target);

            if response.clicked() && button != TransportButton::Eject {
                actions.transport = Some(button);
            }
        }
    }

    /// Draws the toggle buttons and detects clicks.
    fn draw_toggles(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        origin: Pos2,
        state: &MainWindowState,
        actions: &mut MainWindowActions,
    ) {
        let Some(texture) = self.textures.get("shufrep.bmp") else {
            return;
        };

        let toggles = [
            (ToggleButton::Repeat, state.repeat),
            (ToggleButton::Shuffle, state.shuffle),
            (ToggleButton::Equalizer, state.equalizer_open),
            (ToggleButton::Playlist, state.playlist_open),
        ];

        for (button, active) in toggles {
            let sprite_state = if active {
                ButtonState::Active
            } else {
                ButtonState::Normal
            };
            let source = ToggleSprites::rect(button, sprite_state);
            let target = self.rect(
                origin,
                SkinRect::new(
                    sprites::TOGGLE_POSITION.0 + source.x,
                    sprites::TOGGLE_POSITION.1 + source.y,
                    source.width,
                    source.height,
                ),
            );

            let response = ui.interact(
                target,
                ui.scope_id().with(("toggle", button)),
                Sense::click(),
            );
            draw_sprite(painter, &texture, source, target);

            if response.clicked() {
                actions.toggle = Some(button);
            }
        }
    }
}

/// Draws a framed panel with the classic Winamp look.
pub fn draw_panel(painter: &egui::Painter, rect: Rect, theme: &Theme) {
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme.panel);
    painter.rect(
        rect,
        egui::CornerRadius::ZERO,
        Color32::TRANSPARENT,
        Stroke::new(1.0, theme.border),
        StrokeKind::Middle,
    );
}
