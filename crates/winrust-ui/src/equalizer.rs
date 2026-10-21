//! The equalizer panel.

use egui::{Color32, RichText, Vec2};
use winrust_dsp::{MAX_GAIN_DB, MIN_GAIN_DB, WINAMP_BANDS};

use crate::theme::Theme;

/// The state of the equalizer.
pub struct EqualizerState {
    /// The gain of every band, in dB.
    pub bands: [f32; winrust_dsp::BAND_COUNT],
    /// The gain of the preamp, in dB.
    pub preamp: f32,
    /// Whether the equalizer is active.
    pub enabled: bool,
}

/// The actions of the equalizer.
#[derive(Debug, Default)]
pub struct EqualizerActions {
    /// The bands changed.
    pub bands: Option<[f32; winrust_dsp::BAND_COUNT]>,
    /// The preamp changed.
    pub preamp: Option<f32>,
    /// The enabled state changed.
    pub enabled: Option<bool>,
    /// The reset button was pressed.
    pub reset: bool,
}

/// Draws the equalizer.
pub fn show(ui: &mut egui::Ui, state: &mut EqualizerState, theme: &Theme) -> EqualizerActions {
    let mut actions = EqualizerActions::default();

    ui.horizontal(|ui| {
        let mut enabled = state.enabled;
        if ui
            .checkbox(&mut enabled, RichText::new("Enabled").color(theme.text))
            .changed()
        {
            actions.enabled = Some(enabled);
            state.enabled = enabled;
        }

        if ui
            .button("Reset")
            .on_hover_text("Flatten every band")
            .clicked()
        {
            actions.reset = true;
            state.bands = [0.0; winrust_dsp::BAND_COUNT];
            state.preamp = 0.0;
        }

        ui.add_space(16.0);

        ui.label(RichText::new("Preamp").color(theme.text_dim));
        let mut preamp = state.preamp;
        if ui
            .add_sized(
                Vec2::new(150.0, 18.0),
                egui::Slider::new(&mut preamp, MIN_GAIN_DB..=MAX_GAIN_DB).suffix(" dB"),
            )
            .changed()
        {
            state.preamp = preamp;
            actions.preamp = Some(preamp);
        }
    });

    ui.add_space(8.0);

    ui.horizontal(|ui| {
        for band in 0..winrust_dsp::BAND_COUNT {
            ui.vertical(|ui| {
                let mut gain = state.bands[band];
                let response = ui.add_sized(
                    Vec2::new(26.0, 130.0),
                    egui::Slider::new(&mut gain, MIN_GAIN_DB..=MAX_GAIN_DB).vertical(),
                );
                if response.changed() {
                    state.bands[band] = gain;
                    actions.bands = Some(state.bands);
                }
                ui.small(
                    RichText::new(format_band(band))
                        .size(9.0)
                        .color(theme.text_dim),
                );
            });
        }
    });

    actions
}

/// Formats the label of a band.
fn format_band(band: usize) -> String {
    let frequency = WINAMP_BANDS[band];
    if frequency >= 1000.0 {
        format!("{}k", (frequency / 1000.0) as u32)
    } else {
        format!("{frequency}")
    }
}

/// The colour of a band, from green to red.
pub fn band_color(gain: f32) -> Color32 {
    let ratio = ((gain - MIN_GAIN_DB) / (MAX_GAIN_DB - MIN_GAIN_DB)).clamp(0.0, 1.0);
    Color32::from_rgb(
        (96.0 + ratio * 159.0) as u8,
        (192.0 - ratio * 96.0) as u8,
        (96.0 - ratio * 48.0) as u8,
    )
}
