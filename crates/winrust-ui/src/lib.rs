//! The winrust user interface.
//!
//! The crate provides the Winamp inspired main window, the YouTube Music
//! browser and the playlist and equalizer windows.

pub mod app;
pub mod main_window;
pub mod renderer;
pub mod state;
pub mod theme;

pub use app::WinrustApp;
pub use main_window::{MainWindow, MainWindowActions, MainWindowState};
pub use renderer::TextureCache;
pub use theme::Theme;

/// Starts the winrust user interface.
pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("winrust")
            .with_inner_size([900.0, 640.0])
            .with_min_inner_size([560.0, 420.0]),
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
