//! The winrust music player.

use std::process::ExitCode;

use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("WINRUST_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    match winrust_ui::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!("winrust failed to start: {err}");
            ExitCode::FAILURE
        }
    }
}
