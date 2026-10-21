//! The colour theme derived from the loaded skin.

use egui::Color32;
use winrust_skin::Skin;

/// The colours used by the interface.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Background of the panels.
    pub background: Color32,
    /// Background of the raised areas.
    pub panel: Color32,
    /// Background of the display areas.
    pub display_background: Color32,
    /// Border colour.
    pub border: Color32,
    /// Main text colour.
    pub text: Color32,
    /// Colour of the dimmed text.
    pub dimmed: Color32,
    /// Accent colour, used by the LEDs and the active states.
    pub accent: Color32,
    /// Colour of the playlist text.
    pub playlist_text: Color32,
    /// Colour of the text of the current track.
    pub playlist_current: Color32,
    /// Colour of the text of the selected track.
    pub playlist_selected: Color32,
    /// The 24 visualizer colours.
    pub visualizer: [Color32; 24],
}

impl Theme {
    /// Builds a theme from a skin.
    pub fn from_skin(skin: &Skin) -> Self {
        let playlist = &skin.playlist;
        let general = &skin.general;
        let visualizer = &skin.visualizer;

        Self {
            background: Color32::from_rgb(0x18, 0x18, 0x18),
            panel: Color32::from_rgb(0x30, 0x30, 0x30),
            display_background: Color32::from_rgb(0x08, 0x08, 0x08),
            border: Color32::from_rgb(0x55, 0x55, 0x55),
            text: Color32::from_rgb(0xd0, 0xd0, 0xd0),
            dimmed: Color32::from_rgb(0x80, 0x80, 0x80),
            accent: crate::renderer::to_color32(general[0]),
            playlist_text: crate::renderer::to_color32(playlist.foreground),
            playlist_current: crate::renderer::to_color32(playlist.current),
            playlist_selected: crate::renderer::to_color32(playlist.selected),
            visualizer: std::array::from_fn(|index| {
                crate::renderer::to_color32(visualizer[index % visualizer.len()])
            }),
        }
    }

    /// Returns the colour of a visualizer bar.
    pub fn visualizer_color(&self, index: usize, count: usize) -> Color32 {
        if count == 0 {
            return self.accent;
        }
        let position = index as f32 / count as f32;
        let slot = (position * (self.visualizer.len() - 1) as f32).round() as usize;
        self.visualizer[slot % self.visualizer.len()]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_skin(&winrust_skin::default_skin())
    }
}
