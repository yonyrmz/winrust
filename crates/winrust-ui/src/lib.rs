//! The winrust user interface.
//!
//! The crate provides the main window, the mini player, the equalizer and
//! the two sources of music: a YouTube Music search and a local library.

pub mod app;
pub mod equalizer;
pub mod icons;
pub mod mini_player;
pub mod player;
pub mod state;
pub mod theme;

pub use app::{View, WinrustApp};
pub use theme::Theme;

/// Starts the winrust user interface.
pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("winrust")
            .with_inner_size([1024.0, 680.0])
            .with_min_inner_size([680.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "winrust",
        native_options,
        Box::new(|_cc| {
            let app = WinrustApp::new()?;
            Ok(Box::new(app))
        }),
    )
    .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)
}
