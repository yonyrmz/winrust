//! The icons of the interface.
//!
//! The icons are drawn with the painter instead of being taken from a font,
//! so that they always look the same on every platform.

use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::theme::Theme;

/// A clickable icon button.
pub fn icon_button(ui: &mut egui::Ui, theme: &Theme, icon: Icon, tooltip: &str) -> bool {
    let size = Vec2::new(30.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    let color = if hovered { theme.accent } else { theme.text };

    if hovered {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(6), theme.surface_alt);
    }
    draw(ui.painter(), rect, icon, color);

    response.on_hover_text(tooltip).clicked()
}

/// The icons used by the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Go to the previous track.
    Previous,
    /// Start the playback.
    Play,
    /// Pause the playback.
    Pause,
    /// Stop the playback.
    Stop,
    /// Go to the next track.
    Next,
    /// Toggle the shuffle mode.
    Shuffle,
    /// Toggle the repeat mode.
    Repeat,
    /// Open the equalizer.
    Equalizer,
    /// Open the mini player.
    MiniPlayer,
    /// Close the mini player.
    Close,
    /// A speaker.
    Volume,
}

/// Draws an icon inside the given rectangle.
pub fn draw(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let center = rect.center();
    let size = rect.height().min(rect.width()) * 0.55;

    match icon {
        Icon::Play => {
            let half = size * 0.5;
            let points = [
                Pos2::new(center.x - half * 0.6, center.y - half),
                Pos2::new(center.x - half * 0.6, center.y + half),
                Pos2::new(center.x + half, center.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pause => {
            let half = size * 0.5;
            let width = size * 0.28;
            painter.rect_filled(
                Rect::from_center_size(
                    Pos2::new(center.x - half * 0.55, center.y),
                    Vec2::new(width, size),
                ),
                egui::CornerRadius::ZERO,
                color,
            );
            painter.rect_filled(
                Rect::from_center_size(
                    Pos2::new(center.x + half * 0.55, center.y),
                    Vec2::new(width, size),
                ),
                egui::CornerRadius::ZERO,
                color,
            );
        }
        Icon::Stop => {
            painter.rect_filled(
                Rect::from_center_size(center, Vec2::splat(size * 0.85)),
                egui::CornerRadius::ZERO,
                color,
            );
        }
        Icon::Previous => {
            let half = size * 0.5;
            let points = [
                Pos2::new(center.x + half * 0.2, center.y - half * 0.8),
                Pos2::new(center.x + half * 0.2, center.y + half * 0.8),
                Pos2::new(center.x - half * 0.5, center.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                color,
                Stroke::NONE,
            ));
            painter.rect_filled(
                Rect::from_center_size(
                    Pos2::new(center.x + half * 0.6, center.y),
                    Vec2::new(size * 0.16, size * 1.0),
                ),
                egui::CornerRadius::ZERO,
                color,
            );
        }
        Icon::Next => {
            let half = size * 0.5;
            let points = [
                Pos2::new(center.x - half * 0.2, center.y - half * 0.8),
                Pos2::new(center.x - half * 0.2, center.y + half * 0.8),
                Pos2::new(center.x + half * 0.5, center.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                color,
                Stroke::NONE,
            ));
            painter.rect_filled(
                Rect::from_center_size(
                    Pos2::new(center.x - half * 0.6, center.y),
                    Vec2::new(size * 0.16, size * 1.0),
                ),
                egui::CornerRadius::ZERO,
                color,
            );
        }
        Icon::Shuffle => {
            let stroke = Stroke::new(1.6, color);
            let left = rect.left() + size * 0.3;
            let right = rect.right() - size * 0.3;
            let top = center.y - size * 0.45;
            let bottom = center.y + size * 0.45;

            painter.line_segment([Pos2::new(left, top), Pos2::new(center.x, top)], stroke);
            painter.line_segment([Pos2::new(center.x, top), Pos2::new(right, bottom)], stroke);
            painter.line_segment(
                [Pos2::new(left, bottom), Pos2::new(center.x, bottom)],
                stroke,
            );
            painter.line_segment([Pos2::new(center.x, bottom), Pos2::new(right, top)], stroke);
            // The arrow heads.
            painter.line_segment(
                [
                    Pos2::new(right, top),
                    Pos2::new(right - size * 0.3, top - size * 0.2),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(right, bottom),
                    Pos2::new(right - size * 0.3, bottom + size * 0.2),
                ],
                stroke,
            );
        }
        Icon::Repeat => {
            let stroke = Stroke::new(1.6, color);
            let half = size * 0.55;
            let left = center.x - half;
            let right = center.x + half;
            let top = center.y - size * 0.3;
            let bottom = center.y + size * 0.3;

            painter.line_segment([Pos2::new(left, top), Pos2::new(right, top)], stroke);
            painter.line_segment([Pos2::new(right, top), Pos2::new(right, bottom)], stroke);
            painter.line_segment([Pos2::new(right, bottom), Pos2::new(left, bottom)], stroke);
            painter.line_segment(
                [Pos2::new(left, bottom), Pos2::new(left, top + size * 0.25)],
                stroke,
            );
            // The arrow head.
            painter.line_segment(
                [
                    Pos2::new(left, top + size * 0.3),
                    Pos2::new(left - size * 0.25, top + size * 0.05),
                ],
                stroke,
            );
        }
        Icon::Equalizer => {
            let stroke = Stroke::new(1.5, color);
            for (index, x) in [-0.4f32, 0.0, 0.4].iter().enumerate() {
                let x = center.x + x * size;
                painter.line_segment(
                    [
                        Pos2::new(x, center.y - size * 0.5),
                        Pos2::new(x, center.y + size * 0.5),
                    ],
                    stroke,
                );
                let knob = match index {
                    0 => -0.2,
                    1 => 0.25,
                    _ => -0.05,
                };
                painter.rect_filled(
                    Rect::from_center_size(
                        Pos2::new(x, center.y + knob * size),
                        Vec2::new(size * 0.28, size * 0.16),
                    ),
                    egui::CornerRadius::same(2),
                    color,
                );
            }
        }
        Icon::MiniPlayer => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::new(size * 1.1, size * 0.8)),
                egui::CornerRadius::same(2),
                Stroke::new(1.5, color),
                StrokeKind::Middle,
            );
            painter.rect_filled(
                Rect::from_center_size(
                    Pos2::new(center.x - size * 0.2, center.y),
                    Vec2::new(size * 0.2, size * 0.5),
                ),
                egui::CornerRadius::ZERO,
                color,
            );
            painter.rect_filled(
                Rect::from_center_size(
                    Pos2::new(center.x + size * 0.2, center.y),
                    Vec2::new(size * 0.2, size * 0.5),
                ),
                egui::CornerRadius::ZERO,
                color,
            );
        }
        Icon::Close => {
            let stroke = Stroke::new(1.6, color);
            let half = size * 0.4;
            painter.line_segment(
                [
                    Pos2::new(center.x - half, center.y - half),
                    Pos2::new(center.x + half, center.y + half),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + half, center.y - half),
                    Pos2::new(center.x - half, center.y + half),
                ],
                stroke,
            );
        }
        Icon::Volume => {
            let half = size * 0.5;
            let points = [
                Pos2::new(center.x - half, center.y - size * 0.2),
                Pos2::new(center.x - half * 0.4, center.y - size * 0.2),
                Pos2::new(center.x + half * 0.2, center.y - half * 0.7),
                Pos2::new(center.x + half * 0.2, center.y + half * 0.7),
                Pos2::new(center.x - half * 0.4, center.y + size * 0.2),
                Pos2::new(center.x - half, center.y + size * 0.2),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                color,
                Stroke::NONE,
            ));
            painter.circle_stroke(
                Pos2::new(center.x + half * 0.2, center.y),
                size * 0.45,
                Stroke::new(1.4, color),
            );
        }
    }
}
