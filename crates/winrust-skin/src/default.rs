//! A procedurally generated default skin.
//!
//! The original Winamp artwork is proprietary, so winrust ships its own
//! skin instead. The layout matches the classic player closely enough for
//! the sprite tables to work, but the pixels are drawn from scratch.

use std::collections::HashMap;

use crate::config::{GeneralColors, PlaylistTheme, VisualizerColors};
use crate::image::{Image, Rect};

/// Background colour of the default skin.
pub const BACKGROUND: [u8; 4] = [0x24, 0x24, 0x24, 0xff];
/// Raised panel colour.
pub const PANEL: [u8; 4] = [0x3a, 0x3a, 0x3a, 0xff];
/// Highlight colour of the bevels.
pub const BEVEL_LIGHT: [u8; 4] = [0x6a, 0x6a, 0x6a, 0xff];
/// Shadow colour of the bevels.
pub const BEVEL_DARK: [u8; 4] = [0x10, 0x10, 0x10, 0xff];
/// Colour of the LED digits.
pub const LED: [u8; 4] = [0x00, 0xff, 0x4c, 0xff];
/// Colour of the dimmed LED segments.
pub const LED_OFF: [u8; 4] = [0x00, 0x4a, 0x18, 0xff];

/// Builds the default skin.
pub fn default_skin() -> crate::loader::Skin {
    let mut images: HashMap<String, Image> = HashMap::new();

    images.insert("main.bmp".into(), build_main());
    images.insert("titlebar.bmp".into(), build_titlebar());
    images.insert("cbuttons.bmp".into(), build_cbuttons());
    images.insert("shufrep.bmp".into(), build_shufrep());
    images.insert("numbers.bmp".into(), build_numbers());
    images.insert("posbar.bmp".into(), build_posbar());
    images.insert("volume.bmp".into(), build_volume());
    images.insert("balance.bmp".into(), build_balance());
    images.insert("eqmain.bmp".into(), build_eqmain());
    images.insert("eq_ex.bmp".into(), build_eq_ex());
    images.insert("pledit.bmp".into(), build_pledit());
    images.insert("playpaus.bmp".into(), build_playpaus());
    images.insert("monoster.bmp".into(), build_monoster());

    crate::loader::Skin {
        name: "winrust default".to_string(),
        images,
        playlist: PlaylistTheme {
            background: crate::config::Color::rgb(0x10, 0x10, 0x10),
            foreground: crate::config::Color::rgb(0x00, 0xff, 0x4c),
            current: crate::config::Color::rgb(0xff, 0xff, 0xff),
            selected: crate::config::Color::rgb(0x00, 0xa8, 0xff),
            titlebar_active: crate::config::Color::rgb(0x24, 0x24, 0x24),
            titlebar_inactive: crate::config::Color::rgb(0x18, 0x18, 0x18),
            time: crate::config::Color::rgb(0x00, 0xff, 0x4c),
        },
        general: general_colors(),
        visualizer: visualizer_colors(),
        regions: Default::default(),
    }
}

/// The 24 general purpose colours of the default skin.
fn general_colors() -> GeneralColors {
    let mut colors = [crate::config::Color::rgb(0, 0, 0); 24];
    colors[0] = crate::config::Color::rgb(0x00, 0xff, 0x4c);
    colors[1] = crate::config::Color::rgb(0xff, 0xff, 0xff);
    colors[2] = crate::config::Color::rgb(0x00, 0xa8, 0xff);
    colors[3] = crate::config::Color::rgb(0xff, 0x00, 0x4c);
    colors[4] = crate::config::Color::rgb(0x3a, 0x3a, 0x3a);
    colors[5] = crate::config::Color::rgb(0x24, 0x24, 0x24);
    for (index, color) in colors.iter_mut().enumerate().skip(6) {
        let level = 0x10 + (index as u8 % 8) * 0x10;
        *color = crate::config::Color::rgb(level, level, level);
    }
    colors
}

/// The visualizer colours of the default skin.
fn visualizer_colors() -> VisualizerColors {
    let mut colors = [crate::config::Color::rgb(0, 0, 0); 24];
    for (index, color) in colors.iter_mut().enumerate() {
        let position = index as f32 / 23.0;
        *color = crate::config::Color::rgb(
            (16.0 + position * 224.0) as u8,
            (255.0 - position * 128.0) as u8,
            (32.0 + position * 64.0) as u8,
        );
    }
    colors
}

/// Fills a rectangle with a solid colour.
fn fill(image: &mut Image, rect: Rect, color: [u8; 4]) {
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            if let Some(index) = pixel_index(image, x, y) {
                image.pixels[index..index + 4].copy_from_slice(&color);
            }
        }
    }
}

/// Draws a one pixel wide rectangle outline.
fn stroke(image: &mut Image, rect: Rect, color: [u8; 4]) {
    fill(image, Rect::new(rect.x, rect.y, rect.width, 1), color);
    fill(
        image,
        Rect::new(rect.x, rect.bottom() - 1, rect.width, 1),
        color,
    );
    fill(image, Rect::new(rect.x, rect.y, 1, rect.height), color);
    fill(
        image,
        Rect::new(rect.right() - 1, rect.y, 1, rect.height),
        color,
    );
}

/// Draws a classic raised bevel.
fn bevel(image: &mut Image, rect: Rect) {
    fill(image, Rect::new(rect.x, rect.y, rect.width, 1), BEVEL_LIGHT);
    fill(
        image,
        Rect::new(rect.x, rect.y, 1, rect.height),
        BEVEL_LIGHT,
    );
    fill(
        image,
        Rect::new(rect.x, rect.bottom() - 1, rect.width, 1),
        BEVEL_DARK,
    );
    fill(
        image,
        Rect::new(rect.right() - 1, rect.y, 1, rect.height),
        BEVEL_DARK,
    );
}

/// Draws a sunken bevel.
fn sunken(image: &mut Image, rect: Rect) {
    fill(image, Rect::new(rect.x, rect.y, rect.width, 1), BEVEL_DARK);
    fill(image, Rect::new(rect.x, rect.y, 1, rect.height), BEVEL_DARK);
    fill(
        image,
        Rect::new(rect.x, rect.bottom() - 1, rect.width, 1),
        BEVEL_LIGHT,
    );
    fill(
        image,
        Rect::new(rect.right() - 1, rect.y, 1, rect.height),
        BEVEL_LIGHT,
    );
}

/// Returns the byte offset of a pixel.
fn pixel_index(image: &Image, x: u32, y: u32) -> Option<usize> {
    if x >= image.width || y >= image.height {
        return None;
    }
    Some(((y as usize * image.width as usize) + x as usize) * 4)
}

/// The 5x7 bitmap font used for the LED digits.
const DIGIT_FONT: [[u8; 7]; 11] = [
    // 0
    [
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
    ],
    // 1
    [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ],
    // 2
    [
        0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
    ],
    // 3
    [
        0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
    ],
    // 4
    [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ],
    // 5
    [
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
    ],
    // 6
    [
        0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
    ],
    // 7
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ],
    // 8
    [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ],
    // 9
    [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
    ],
    // minus
    [
        0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
    ],
];

/// Draws a glyph of the digit font.
fn draw_glyph(image: &mut Image, x: u32, y: u32, glyph: usize, color: [u8; 4]) {
    let rows = DIGIT_FONT[glyph.min(DIGIT_FONT.len() - 1)];
    for (row, bits) in rows.iter().enumerate() {
        for column in 0..5 {
            if bits & (1 << (4 - column)) != 0 {
                fill(
                    image,
                    Rect::new(x + column as u32, y + row as u32, 1, 1),
                    color,
                );
            }
        }
    }
}

/// Draws a triangle pointing to the right.
fn triangle_right(image: &mut Image, x: u32, y: u32, width: u32, height: u32, color: [u8; 4]) {
    for row in 0..height {
        let distance = (row as f32 - height as f32 / 2.0).abs();
        let ratio = 1.0 - (2.0 * distance / height as f32).clamp(0.0, 1.0);
        let span = (width as f32 * ratio).round() as u32;
        fill(image, Rect::new(x, y + row, span, 1), color);
    }
}

/// Draws a triangle pointing to the left.
fn triangle_left(image: &mut Image, x: u32, y: u32, width: u32, height: u32, color: [u8; 4]) {
    for row in 0..height {
        let distance = (row as f32 - height as f32 / 2.0).abs();
        let ratio = 1.0 - (2.0 * distance / height as f32).clamp(0.0, 1.0);
        let span = (width as f32 * ratio).round() as u32;
        fill(image, Rect::new(x + width - span, y + row, span, 1), color);
    }
}

/// Builds the `numbers.bmp` bitmap.
fn build_numbers() -> Image {
    let mut image = Image::new(99, 13);
    for digit in 0..11usize {
        draw_glyph(&mut image, (digit as u32) * 9, 3, digit, LED);
    }
    image
}

/// Builds the `main.bmp` bitmap.
fn build_main() -> Image {
    let mut image = Image::new(275, 116);
    fill(&mut image, Rect::new(0, 0, 275, 116), BACKGROUND);

    // Title bar.
    fill(&mut image, Rect::new(0, 0, 275, 14), PANEL);
    fill(&mut image, Rect::new(0, 14, 275, 1), BEVEL_DARK);

    // Song ticker panel.
    sunken(&mut image, Rect::new(109, 24, 158, 11));
    fill(
        &mut image,
        Rect::new(110, 25, 156, 9),
        [0x0a, 0x0a, 0x0a, 0xff],
    );

    // Time display.
    fill(
        &mut image,
        Rect::new(30, 24, 74, 16),
        [0x0a, 0x0a, 0x0a, 0xff],
    );
    sunken(&mut image, Rect::new(30, 24, 74, 16));

    // Visualizer panel.
    sunken(&mut image, Rect::new(22, 42, 80, 19));
    fill(
        &mut image,
        Rect::new(24, 43, 76, 16),
        [0x0a, 0x0a, 0x0a, 0xff],
    );

    // Info area.
    sunken(&mut image, Rect::new(100, 42, 168, 19));
    fill(
        &mut image,
        Rect::new(102, 44, 164, 15),
        [0x0a, 0x0a, 0x0a, 0xff],
    );

    // Position bar.
    sunken(&mut image, Rect::new(15, 71, 250, 12));

    // Volume and balance panels.
    sunken(&mut image, Rect::new(105, 56, 72, 16));
    sunken(&mut image, Rect::new(175, 56, 42, 16));

    // Transport button strip.
    fill(&mut image, Rect::new(14, 86, 140, 21), PANEL);
    bevel(&mut image, Rect::new(14, 86, 140, 21));

    // Toggle button strip.
    fill(&mut image, Rect::new(162, 87, 50, 19), PANEL);

    image
}

/// Builds the `titlebar.bmp` bitmap.
fn build_titlebar() -> Image {
    let mut image = Image::new(275, 43);
    for state in 0..3u32 {
        let y = state * 14 + if state > 0 { 1 } else { 0 };
        let base = match state {
            0 => PANEL,
            1 => [0x1e, 0x1e, 0x1e, 0xff],
            _ => [0x2a, 0x2a, 0x2a, 0xff],
        };
        fill(&mut image, Rect::new(0, y, 275, 14), base);
        fill(&mut image, Rect::new(0, y + 13, 275, 1), BEVEL_DARK);

        // A small "WINRUST" wordmark built from rectangles.
        let accent = if state == 1 { LED } else { LED_OFF };
        for (index, width) in [6u32, 4, 6, 4, 6, 4].iter().enumerate() {
            let x = 8 + index as u32 * 8;
            fill(&mut image, Rect::new(x, y + 5, *width, 4), accent);
        }
    }
    image
}

/// Builds the `cbuttons.bmp` bitmap.
fn build_cbuttons() -> Image {
    let mut image = Image::new(136, 36);
    fill(&mut image, Rect::new(0, 0, 136, 36), PANEL);

    for state in 0..2u32 {
        let y = state * 18;
        let face = if state == 0 {
            PANEL
        } else {
            [0x55, 0x55, 0x55, 0xff]
        };
        let glyph = if state == 0 {
            LED
        } else {
            [0xff, 0xff, 0xff, 0xff]
        };

        for (index, x) in [0u32, 23, 46, 69, 92].iter().enumerate() {
            let width = if index == 4 { 22 } else { 23 };
            fill(&mut image, Rect::new(*x, y, width, 18), face);
            stroke(&mut image, Rect::new(*x, y, width, 18), BEVEL_DARK);
        }

        // Play triangle.
        triangle_right(&mut image, 28, y + 6, 9, 7, glyph);
        // Pause bars.
        fill(&mut image, Rect::new(52, y + 6, 3, 7), glyph);
        fill(&mut image, Rect::new(57, y + 6, 3, 7), glyph);
        // Stop square.
        fill(&mut image, Rect::new(77, y + 6, 7, 7), glyph);
        // Previous: a bar and a left pointing triangle.
        fill(&mut image, Rect::new(6, y + 6, 2, 7), glyph);
        triangle_left(&mut image, 9, y + 6, 8, 7, glyph);
        // Next: a right pointing triangle and a bar.
        triangle_right(&mut image, 99, y + 6, 8, 7, glyph);
        fill(&mut image, Rect::new(108, y + 6, 2, 7), glyph);
    }

    // Eject button (22x16, drawn at x=114).
    fill(&mut image, Rect::new(114, 0, 22, 16), PANEL);
    stroke(&mut image, Rect::new(114, 0, 22, 16), BEVEL_DARK);
    fill(&mut image, Rect::new(120, 4, 8, 7), LED);
    fill(&mut image, Rect::new(120, 11, 8, 1), LED);

    fill(
        &mut image,
        Rect::new(114, 18, 22, 16),
        [0x55, 0x55, 0x55, 0xff],
    );
    stroke(&mut image, Rect::new(114, 18, 22, 16), BEVEL_DARK);
    fill(
        &mut image,
        Rect::new(120, 22, 8, 7),
        [0xff, 0xff, 0xff, 0xff],
    );
    fill(
        &mut image,
        Rect::new(120, 29, 8, 1),
        [0xff, 0xff, 0xff, 0xff],
    );

    image
}

/// Builds the `shufrep.bmp` bitmap.
fn build_shufrep() -> Image {
    let mut image = Image::new(92, 64);
    fill(&mut image, Rect::new(0, 0, 92, 64), PANEL);

    // Repeat button.
    fill(&mut image, Rect::new(0, 0, 28, 15), PANEL);
    stroke(&mut image, Rect::new(0, 0, 28, 15), BEVEL_DARK);
    fill(&mut image, Rect::new(4, 4, 20, 2), LED);
    fill(&mut image, Rect::new(4, 9, 20, 2), LED);

    // Repeat pressed.
    fill(
        &mut image,
        Rect::new(0, 15, 28, 15),
        [0x55, 0x55, 0x55, 0xff],
    );
    stroke(&mut image, Rect::new(0, 15, 28, 15), BEVEL_DARK);
    fill(
        &mut image,
        Rect::new(4, 19, 20, 2),
        [0xff, 0xff, 0xff, 0xff],
    );
    fill(
        &mut image,
        Rect::new(4, 24, 20, 2),
        [0xff, 0xff, 0xff, 0xff],
    );

    // Shuffle button.
    fill(&mut image, Rect::new(28, 0, 22, 15), PANEL);
    stroke(&mut image, Rect::new(28, 0, 22, 15), BEVEL_DARK);
    fill(&mut image, Rect::new(32, 4, 14, 2), LED);
    fill(&mut image, Rect::new(32, 9, 14, 2), LED);

    fill(
        &mut image,
        Rect::new(28, 15, 22, 15),
        [0x55, 0x55, 0x55, 0xff],
    );
    stroke(&mut image, Rect::new(28, 15, 22, 15), BEVEL_DARK);
    fill(
        &mut image,
        Rect::new(32, 19, 14, 2),
        [0xff, 0xff, 0xff, 0xff],
    );
    fill(
        &mut image,
        Rect::new(32, 24, 14, 2),
        [0xff, 0xff, 0xff, 0xff],
    );

    // Equalizer and playlist buttons.
    for x in [0u32, 23] {
        fill(&mut image, Rect::new(x, 30, 23, 12), PANEL);
        stroke(&mut image, Rect::new(x, 30, 23, 12), BEVEL_DARK);
        let glyph = LED;
        fill(&mut image, Rect::new(x + 6, 34, 11, 3), glyph);

        fill(
            &mut image,
            Rect::new(x, 42, 23, 12),
            [0x55, 0x55, 0x55, 0xff],
        );
        stroke(&mut image, Rect::new(x, 42, 23, 12), BEVEL_DARK);
        fill(
            &mut image,
            Rect::new(x + 6, 46, 11, 3),
            [0xff, 0xff, 0xff, 0xff],
        );
    }

    image
}

/// Builds the `posbar.bmp` bitmap.
fn build_posbar() -> Image {
    let mut image = Image::new(277, 10);
    fill(&mut image, Rect::new(0, 0, 248, 10), PANEL);
    sunken(&mut image, Rect::new(0, 0, 248, 10));
    fill(&mut image, Rect::new(248, 0, 29, 10), PANEL);
    fill(&mut image, Rect::new(258, 2, 9, 6), LED);
    stroke(&mut image, Rect::new(248, 0, 29, 10), BEVEL_DARK);
    image
}

/// Builds the `volume.bmp` bitmap.
fn build_volume() -> Image {
    let mut image = Image::new(68, 433);
    for frame in 0..28u32 {
        let y = frame * 13;
        fill(&mut image, Rect::new(0, y, 68, 13), PANEL);
        let lit = (frame as f32 / 27.0) * 60.0;
        fill(&mut image, Rect::new(3, y + 5, lit as u32, 3), LED);
    }
    // The groove.
    fill(&mut image, Rect::new(0, 364, 68, 13), PANEL);
    sunken(&mut image, Rect::new(0, 364, 68, 13));
    // The thumb.
    fill(
        &mut image,
        Rect::new(0, 422, 14, 11),
        [0x60, 0x60, 0x60, 0xff],
    );
    stroke(&mut image, Rect::new(0, 422, 14, 11), BEVEL_DARK);
    image
}

/// Builds the `balance.bmp` bitmap.
fn build_balance() -> Image {
    let mut image = Image::new(38, 433);
    for frame in 0..28u32 {
        let y = frame * 13;
        fill(&mut image, Rect::new(0, y, 38, 13), PANEL);
        let center = 19u32;
        let offset = frame as i32 - 14;
        let x = (center as i32 + offset).clamp(0, 37) as u32;
        fill(&mut image, Rect::new(x, y + 5, 2, 3), LED);
    }
    fill(&mut image, Rect::new(0, 422, 38, 11), PANEL);
    fill(
        &mut image,
        Rect::new(9, 422, 14, 11),
        [0x60, 0x60, 0x60, 0xff],
    );
    stroke(&mut image, Rect::new(9, 422, 14, 11), BEVEL_DARK);
    image
}

/// Builds the `eqmain.bmp` bitmap.
fn build_eqmain() -> Image {
    let mut image = Image::new(275, 116);
    fill(&mut image, Rect::new(0, 0, 275, 116), BACKGROUND);
    fill(&mut image, Rect::new(0, 0, 275, 14), PANEL);
    fill(&mut image, Rect::new(0, 14, 275, 1), BEVEL_DARK);

    // The graph area.
    sunken(&mut image, Rect::new(86, 17, 113, 19));
    fill(
        &mut image,
        Rect::new(87, 18, 111, 17),
        [0x0a, 0x0a, 0x0a, 0xff],
    );

    // The sliders.
    for index in 0..10u32 {
        let x = 78 + index * 14;
        sunken(&mut image, Rect::new(x, 38, 12, 63));
    }
    sunken(&mut image, Rect::new(21, 38, 12, 63));

    // Buttons.
    for x in [14u32, 39, 217] {
        fill(&mut image, Rect::new(x, 17, 23, 12), PANEL);
        stroke(&mut image, Rect::new(x, 17, 23, 12), BEVEL_DARK);
    }

    // Preset button.
    fill(&mut image, Rect::new(217, 17, 44, 12), PANEL);
    stroke(&mut image, Rect::new(217, 17, 44, 12), BEVEL_DARK);

    image
}

/// Builds the `eq_ex.bmp` bitmap.
fn build_eq_ex() -> Image {
    let mut image = Image::new(160, 11);
    for index in 0..11u32 {
        let x = if index == 0 { 0 } else { 13 + (index - 1) * 15 };
        fill(
            &mut image,
            Rect::new(x, 0, 11, 11),
            [0x60, 0x60, 0x60, 0xff],
        );
        stroke(&mut image, Rect::new(x, 0, 11, 11), BEVEL_DARK);
        fill(&mut image, Rect::new(x + 4, 3, 3, 5), LED);
    }
    image
}

/// Builds the `pledit.bmp` bitmap.
fn build_pledit() -> Image {
    let mut image = Image::new(275, 116);
    fill(&mut image, Rect::new(0, 0, 275, 116), BACKGROUND);
    fill(&mut image, Rect::new(0, 0, 275, 14), PANEL);
    fill(&mut image, Rect::new(0, 14, 275, 1), BEVEL_DARK);
    fill(
        &mut image,
        Rect::new(0, 20, 275, 96),
        [0x10, 0x10, 0x10, 0xff],
    );
    image
}

/// Builds the `playpaus.bmp` bitmap.
fn build_playpaus() -> Image {
    let mut image = Image::new(42, 9);
    for index in 0..3u32 {
        let x = index * 14;
        let glyph = if index == 0 { LED } else { LED_OFF };
        match index {
            0 => fill(&mut image, Rect::new(x + 4, 2, 6, 6), glyph),
            1 => {
                fill(&mut image, Rect::new(x + 4, 2, 2, 6), glyph);
                fill(&mut image, Rect::new(x + 8, 2, 2, 6), glyph);
            }
            _ => fill(&mut image, Rect::new(x + 3, 2, 8, 6), glyph),
        }
    }
    image
}

/// Builds the `monoster.bmp` bitmap.
fn build_monoster() -> Image {
    let mut image = Image::new(56, 24);
    fill(&mut image, Rect::new(0, 0, 56, 24), PANEL);
    for index in 0..2u32 {
        let y = index * 12;
        for (column, bits) in DIGIT_FONT[0].iter().enumerate() {
            let _ = column;
            let _ = bits;
        }
        fill(&mut image, Rect::new(4, y + 3, 20, 2), LED);
        fill(&mut image, Rect::new(30, y + 3, 20, 2), LED_OFF);
    }
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_skin_has_the_expected_bitmaps() {
        let skin = default_skin();
        for name in [
            "main.bmp",
            "titlebar.bmp",
            "cbuttons.bmp",
            "numbers.bmp",
            "posbar.bmp",
            "volume.bmp",
            "eqmain.bmp",
            "pledit.bmp",
        ] {
            assert!(skin.has_image(name), "missing {name}");
        }
    }

    #[test]
    fn numbers_contain_all_glyphs() {
        let skin = default_skin();
        let numbers = skin.image("numbers.bmp").unwrap();
        assert_eq!(numbers.width, 99);
        assert_eq!(numbers.height, 13);
        for digit in 0..11u32 {
            let x = digit * 9;
            let lit = (x..x + 9).any(|x| {
                (0..13).any(|y| {
                    let pixel = numbers.pixel(x, y).unwrap();
                    pixel[1] > 100 && pixel[3] > 0
                })
            });
            assert!(lit, "glyph {digit} is empty");
        }
    }
}
