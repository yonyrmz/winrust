//! The colour palette of the interface.

use egui::Color32;

/// The colours used by the interface.
#[derive(Debug, Clone)]
pub struct Theme {
    /// The colour of the window background.
    pub background: Color32,
    /// The colour of the cards and of the panels.
    pub surface: Color32,
    /// The colour of the alternating rows of a list.
    pub surface_alt: Color32,
    /// The accent colour, used by the active states.
    pub accent: Color32,
    /// The colour of the text.
    pub text: Color32,
    /// The colour of the secondary text.
    pub text_dim: Color32,
    /// The colour of the borders.
    pub border: Color32,
    /// The colour of the errors.
    pub warning: Color32,
    /// The colour of the progress bars.
    pub progress: Color32,
}

impl Theme {
    /// The dark palette.
    pub fn dark() -> Self {
        Self {
            background: Color32::from_rgb(0x14, 0x15, 0x18),
            surface: Color32::from_rgb(0x1c, 0x1e, 0x22),
            surface_alt: Color32::from_rgb(0x22, 0x24, 0x29),
            accent: Color32::from_rgb(0x6c, 0xa8, 0xff),
            text: Color32::from_rgb(0xe6, 0xe8, 0xec),
            text_dim: Color32::from_rgb(0x8b, 0x92, 0x9e),
            border: Color32::from_rgb(0x2c, 0x2f, 0x36),
            warning: Color32::from_rgb(0xff, 0x6b, 0x6b),
            progress: Color32::from_rgb(0x3d, 0x6b, 0xa5),
        }
    }

    /// The light palette.
    pub fn light() -> Self {
        Self {
            background: Color32::from_rgb(0xf4, 0xf5, 0xf7),
            surface: Color32::from_rgb(0xff, 0xff, 0xff),
            surface_alt: Color32::from_rgb(0xec, 0xee, 0xf2),
            accent: Color32::from_rgb(0x1a, 0x6c, 0xd9),
            text: Color32::from_rgb(0x1a, 0x1c, 0x20),
            text_dim: Color32::from_rgb(0x6b, 0x72, 0x7d),
            border: Color32::from_rgb(0xdc, 0xdf, 0xe4),
            warning: Color32::from_rgb(0xc0, 0x2b, 0x2b),
            progress: Color32::from_rgb(0x9a, 0xc0, 0xf0),
        }
    }

    /// Returns the palette that matches the current egui visuals.
    pub fn from_visuals(visuals: &egui::Visuals) -> Self {
        if visuals.dark_mode {
            Self::dark()
        } else {
            Self::light()
        }
    }
}
