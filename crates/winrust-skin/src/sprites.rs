//! Sprite layout tables for the classic Winamp 2.x window layout.
//!
//! The coordinates follow the original player so that third party `.wsz`
//! skins render correctly.

use crate::image::Rect;

/// Width of the main window in pixels.
pub const MAIN_WIDTH: u32 = 275;
/// Height of the main window in pixels.
pub const MAIN_HEIGHT: u32 = 116;
/// Width of the equalizer window in pixels.
pub const EQ_WIDTH: u32 = 275;
/// Height of the equalizer window in pixels.
pub const EQ_HEIGHT: u32 = 116;
/// Width of the playlist window in pixels.
pub const PLAYLIST_WIDTH: u32 = 275;
/// Height of the playlist window in pixels.
pub const PLAYLIST_HEIGHT: u32 = 116;

/// The state of a push button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ButtonState {
    /// The button is idle.
    #[default]
    Normal,
    /// The button is being pressed.
    Pressed,
    /// The button represents an active mode.
    Active,
}

/// A transport button of the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportButton {
    /// Previous track.
    Previous,
    /// Play.
    Play,
    /// Pause.
    Pause,
    /// Stop.
    Stop,
    /// Next track.
    Next,
    /// Open the browser.
    Eject,
}

/// A toggle button of the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToggleButton {
    /// Toggle shuffle.
    Shuffle,
    /// Toggle repeat.
    Repeat,
    /// Open the equalizer.
    Equalizer,
    /// Open the playlist editor.
    Playlist,
}

/// The sprite rectangles of a single window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSprites {
    /// The window background.
    pub background: Rect,
    /// The title bar while the window is focused.
    pub titlebar_active: Rect,
    /// The title bar while the window is not focused.
    pub titlebar_inactive: Rect,
    /// The title bar of the window shade mode.
    pub titlebar_shade: Rect,
}

impl WindowSprites {
    /// The sprites of the main window.
    pub const MAIN: WindowSprites = WindowSprites {
        background: Rect::new(0, 0, MAIN_WIDTH, MAIN_HEIGHT),
        titlebar_active: Rect::new(0, 0, MAIN_WIDTH, 14),
        titlebar_inactive: Rect::new(0, 15, MAIN_WIDTH, 14),
        titlebar_shade: Rect::new(0, 29, MAIN_WIDTH, 14),
    };

    /// The sprites of the equalizer window.
    pub const EQUALIZER: WindowSprites = WindowSprites {
        background: Rect::new(0, 0, EQ_WIDTH, EQ_HEIGHT),
        titlebar_active: Rect::new(0, 0, EQ_WIDTH, 14),
        titlebar_inactive: Rect::new(0, 15, EQ_WIDTH, 14),
        titlebar_shade: Rect::new(0, 29, EQ_WIDTH, 14),
    };

    /// The sprites of the playlist window.
    pub const PLAYLIST: WindowSprites = WindowSprites {
        background: Rect::new(0, 0, PLAYLIST_WIDTH, PLAYLIST_HEIGHT),
        titlebar_active: Rect::new(0, 0, PLAYLIST_WIDTH, 14),
        titlebar_inactive: Rect::new(0, 15, PLAYLIST_WIDTH, 14),
        titlebar_shade: Rect::new(0, 29, PLAYLIST_WIDTH, 14),
    };
}

/// The sprite rectangles of the transport buttons (`cbuttons.bmp`).
pub struct TransportSprites {
    /// Rectangle of a button in the given state.
    pub rect: fn(TransportButton, ButtonState) -> Rect,
}

impl TransportSprites {
    /// Returns the rectangle of the given transport button.
    pub fn rect(button: TransportButton, state: ButtonState) -> Rect {
        let x = match button {
            TransportButton::Previous => 0,
            TransportButton::Play => 23,
            TransportButton::Pause => 46,
            TransportButton::Stop => 69,
            TransportButton::Next => 92,
            TransportButton::Eject => 114,
        };
        let (width, height) = match button {
            TransportButton::Next => (22, 18),
            TransportButton::Eject => (22, 16),
            _ => (23, 18),
        };
        let y = match state {
            ButtonState::Normal => 0,
            ButtonState::Pressed => 18,
            ButtonState::Active => 18,
        };
        Rect::new(x, y, width, height)
    }
}

/// The sprite rectangles of the toggle buttons (`shufrep.bmp`).
pub struct ToggleSprites;

impl ToggleSprites {
    /// Returns the rectangle of the given toggle button.
    pub fn rect(button: ToggleButton, state: ButtonState) -> Rect {
        let (x, y, width, height) = match button {
            ToggleButton::Repeat => (0, 0, 28, 15),
            ToggleButton::Shuffle => (28, 0, 22, 15),
            ToggleButton::Equalizer => (0, 30, 23, 12),
            ToggleButton::Playlist => (23, 30, 23, 12),
        };

        let y = match (button, state) {
            (ToggleButton::Repeat | ToggleButton::Shuffle, ButtonState::Active) => y + 15,
            (ToggleButton::Equalizer | ToggleButton::Playlist, ButtonState::Active) => y + 12,
            (_, ButtonState::Pressed) => y,
            _ => y,
        };

        Rect::new(x, y, width, height)
    }
}

/// The position of the time display digits inside the main window.
pub const TIME_DIGIT_POSITIONS: [(u32, u32); 4] = [
    (48, 26), // minutes, tens
    (60, 26), // minutes, ones
    (78, 26), // seconds, tens
    (90, 26), // seconds, ones
];

/// Position of the minus sign of the time display.
pub const TIME_MINUS_POSITION: (u32, u32) = (36, 26);

/// Position and size of the song ticker.
pub const TICKER_RECT: Rect = Rect::new(111, 27, 154, 6);

/// Position and size of the visualizer area.
pub const VISUALIZER_RECT: Rect = Rect::new(24, 43, 76, 16);

/// Position and size of the position (seek) bar background.
pub const POSITION_BAR_RECT: Rect = Rect::new(16, 72, 248, 10);

/// Position of the transport buttons.
pub const TRANSPORT_POSITION: (u32, u32) = (16, 88);

/// Position of the toggle buttons.
pub const TOGGLE_POSITION: (u32, u32) = (164, 89);

/// Position of the volume slider.
pub const VOLUME_POSITION: (u32, u32) = (107, 57);

/// Position of the balance slider.
pub const BALANCE_POSITION: (u32, u32) = (177, 57);

/// Position of the stereo/mono indicator.
pub const STEREO_POSITION: (u32, u32) = (212, 41);

/// Size of a single digit in `numbers.bmp`.
pub const DIGIT_SIZE: (u32, u32) = (9, 13);

/// Returns the rectangle of a digit inside `numbers.bmp`.
///
/// Digit `10` is the minus sign.
pub fn digit_rect(digit: u32) -> Rect {
    Rect::new(digit * DIGIT_SIZE.0, 0, DIGIT_SIZE.0, DIGIT_SIZE.1)
}

/// Number of background frames stored in `volume.bmp` and `balance.bmp`.
pub const SLIDER_FRAMES: u32 = 28;

/// Height of a single slider frame.
pub const SLIDER_FRAME_HEIGHT: u32 = 13;

/// Returns the rectangle of a volume background frame.
pub fn volume_frame_rect(frame: u32) -> Rect {
    Rect::new(0, frame * SLIDER_FRAME_HEIGHT, 68, SLIDER_FRAME_HEIGHT)
}

/// Returns the rectangle of a balance background frame.
pub fn balance_frame_rect(frame: u32) -> Rect {
    Rect::new(0, frame * SLIDER_FRAME_HEIGHT, 38, SLIDER_FRAME_HEIGHT)
}

/// Rectangle of the volume slider groove.
pub const VOLUME_GROOVE_RECT: Rect = Rect::new(0, 364, 68, 13);

/// Rectangle of the volume slider thumb.
pub const VOLUME_THUMB_RECT: Rect = Rect::new(0, 422, 14, 11);

/// Rectangle of the balance slider groove.
pub const BALANCE_GROOVE_RECT: Rect = Rect::new(0, 422, 38, 11);

/// Rectangle of the balance slider thumb.
pub const BALANCE_THUMB_RECT: Rect = Rect::new(9, 422, 14, 11);

/// Rectangle of the position bar background inside `posbar.bmp`.
pub const POSITION_BAR_SOURCE: Rect = Rect::new(0, 0, 248, 10);

/// Rectangle of the position bar thumb inside `posbar.bmp`.
pub const POSITION_THUMB_SOURCE: Rect = Rect::new(248, 0, 29, 10);

/// The sprite rectangles of the equalizer sliders (`eq_ex.bmp`).
pub struct EqSliderSprites;

impl EqSliderSprites {
    /// Returns the rectangle of the thumb of an equalizer slider.
    ///
    /// `band` is the index of the band, `preamp` selects the preamp slider.
    pub fn thumb_rect(band: usize, preamp: bool) -> Rect {
        if preamp {
            return Rect::new(0, 0, 11, 11);
        }
        Rect::new(13 + (band as u32) * 15, 0, 11, 11)
    }
}

/// The equalizer band slider positions inside the equalizer window.
pub const EQ_BAND_POSITIONS: [(u32, u32); 10] = [
    (78, 38),
    (92, 38),
    (106, 38),
    (120, 38),
    (134, 38),
    (148, 38),
    (162, 38),
    (176, 38),
    (190, 38),
    (204, 38),
];

/// Position of the equalizer preamp slider.
pub const EQ_PREAMP_POSITION: (u32, u32) = (21, 38);

/// Position of the equalizer on/off button.
pub const EQ_ENABLED_POSITION: (u32, u32) = (14, 18);

/// Position of the equalizer auto button.
pub const EQ_AUTO_POSITION: (u32, u32) = (39, 18);

/// Position of the equalizer presets button.
pub const EQ_PRESETS_POSITION: (u32, u32) = (217, 18);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_buttons_do_not_overlap() {
        let buttons = [
            TransportButton::Previous,
            TransportButton::Play,
            TransportButton::Pause,
            TransportButton::Stop,
            TransportButton::Next,
            TransportButton::Eject,
        ];
        for (index, first) in buttons.iter().enumerate() {
            for second in buttons.iter().skip(index + 1) {
                let a = TransportSprites::rect(*first, ButtonState::Normal);
                let b = TransportSprites::rect(*second, ButtonState::Normal);
                let overlap = a.x < b.right() && b.x < a.right();
                assert!(!overlap, "{first:?} overlaps {second:?}");
            }
        }
    }

    #[test]
    fn digit_rectangles_are_sequential() {
        assert_eq!(digit_rect(0), Rect::new(0, 0, 9, 13));
        assert_eq!(digit_rect(9), Rect::new(81, 0, 9, 13));
        assert_eq!(digit_rect(10), Rect::new(90, 0, 9, 13));
    }
}
