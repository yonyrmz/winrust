//! Helpers that turn skin bitmaps into egui textures and draw them.

use std::collections::HashMap;

use egui::{
    Color32, ColorImage, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, StrokeKind,
    TextureHandle, TextureOptions,
};
use winrust_skin::image::Image;

use crate::theme::Theme;

/// Converts a skin colour into an egui colour.
pub fn to_color32(color: winrust_skin::Color) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

/// Converts an RGBA pixel into an egui colour.
pub fn pixel_to_color(pixel: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3])
}

/// Uploads a skin bitmap into an egui texture.
pub fn texture_from_image(image: &Image) -> egui::ColorImage {
    ColorImage::from_rgba_unmultiplied([image.width as usize, image.height as usize], &image.pixels)
}

/// Caches the textures of a skin.
#[derive(Default)]
pub struct TextureCache {
    textures: HashMap<String, TextureHandle>,
}

impl TextureCache {
    /// Creates an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Uploads every bitmap of a skin that is not cached yet.
    pub fn upload(&mut self, ctx: &egui::Context, skin: &winrust_skin::Skin) {
        for (name, image) in &skin.images {
            if self.textures.contains_key(name) {
                continue;
            }
            let handle = ctx.load_texture(
                format!("skin:{name}"),
                texture_from_image(image),
                TextureOptions::NEAREST,
            );
            self.textures.insert(name.clone(), handle);
        }
    }

    /// Returns the texture of a bitmap.
    pub fn get(&self, name: &str) -> Option<TextureHandle> {
        self.textures.get(name).cloned()
    }

    /// Forgets every cached texture.
    pub fn clear(&mut self) {
        self.textures.clear();
    }
}

/// Draws a rectangular region of a texture.
pub fn draw_sprite(
    painter: &Painter,
    texture: &TextureHandle,
    source: winrust_skin::Rect,
    target: Rect,
) {
    let width = texture.size_vec2().x.max(1.0);
    let height = texture.size_vec2().y.max(1.0);

    let uv = Rect::from_min_max(
        Pos2::new(source.x as f32 / width, source.y as f32 / height),
        Pos2::new(
            source.right() as f32 / width,
            source.bottom() as f32 / height,
        ),
    );

    painter.image(texture.id(), target, uv, Color32::WHITE);
}

/// Draws a sprite scaled to the given size.
pub fn draw_sprite_scaled(
    painter: &Painter,
    texture: &TextureHandle,
    source: winrust_skin::Rect,
    target: Rect,
) {
    draw_sprite(painter, texture, source, target);
}

/// Draws a horizontal progress bar.
pub fn draw_progress(painter: &Painter, rect: Rect, progress: f32, theme: &Theme) {
    painter.rect_filled(rect, CornerRadius::ZERO, theme.panel);
    let progress = progress.clamp(0.0, 1.0);
    if progress > 0.0 {
        let filled = Rect::from_min_max(
            rect.min,
            Pos2::new(rect.min.x + rect.width() * progress, rect.max.y),
        );
        painter.rect_filled(filled, CornerRadius::ZERO, theme.accent);
    }
    painter.rect(
        rect,
        CornerRadius::ZERO,
        Color32::TRANSPARENT,
        Stroke::new(1.0, theme.border),
        StrokeKind::Middle,
    );
}

/// Draws the LED style digits of the time display.
pub fn draw_digits(
    painter: &Painter,
    texture: &TextureHandle,
    positions: &[(u32, u32)],
    digits: &[u32],
    origin: Pos2,
    scale: f32,
) {
    for (index, digit) in digits.iter().enumerate() {
        let Some((x, y)) = positions.get(index) else {
            break;
        };
        let source = winrust_skin::sprites::digit_rect(*digit);
        let target = Rect::from_min_size(
            Pos2::new(
                origin.x + (*x as f32) * scale,
                origin.y + (*y as f32) * scale,
            ),
            egui::Vec2::new(source.width as f32 * scale, source.height as f32 * scale),
        );
        draw_sprite(painter, texture, source, target);
    }
}

/// Draws the spectrum analyzer.
pub fn draw_visualizer(painter: &Painter, rect: Rect, bars: &[f32], theme: &Theme) {
    painter.rect_filled(rect, CornerRadius::ZERO, theme.display_background);

    if bars.is_empty() {
        return;
    }

    let bar_width = rect.width() / bars.len() as f32;
    for (index, value) in bars.iter().enumerate() {
        let height = (value.clamp(0.0, 1.0)) * rect.height();
        if height <= 0.5 {
            continue;
        }
        let bar = Rect::from_min_max(
            Pos2::new(rect.min.x + index as f32 * bar_width, rect.max.y - height),
            Pos2::new(
                rect.min.x + (index as f32 + 1.0) * bar_width - 1.0,
                rect.max.y,
            ),
        );
        let color = theme.visualizer_color(index, bars.len());
        painter.rect_filled(bar, CornerRadius::ZERO, color);
    }
}

/// Draws a piece of text using the LED style of the theme.
pub fn draw_led_text(painter: &Painter, pos: Pos2, text: &str, size: f32, color: Color32) {
    painter.text(
        pos,
        egui::Align2::LEFT_CENTER,
        text,
        FontId::monospace(size),
        color,
    );
}
