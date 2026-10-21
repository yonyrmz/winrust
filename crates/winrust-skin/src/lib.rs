//! Skin engine for winrust.
//!
//! The crate loads Winamp `.wsz` archives, which are ZIP files containing a
//! set of bitmaps and plain text configuration files, and exposes them as a
//! collection of sprites that the UI can draw.
//!
//! A default skin is generated procedurally so that winrust never has to
//! ship proprietary artwork.

pub mod config;
pub mod default;
pub mod image;
pub mod loader;
pub mod sprites;

pub use config::{Color, PlaylistTheme, Region, RegionSet};
pub use image::{Image, ImageError, Rect};
pub use loader::{load_wsz, load_wsz_bytes, Skin, SkinError};
pub use sprites::{ButtonState, ToggleButton, TransportButton, WindowSprites};

/// Returns the built in default skin.
pub fn default_skin() -> Skin {
    default::default_skin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_skin_loads() {
        let skin = default_skin();
        assert!(skin.has_image("main.bmp"));
        assert_eq!(skin.playlist.foreground.g, 255);
    }
}
