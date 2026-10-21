//! A simple RGBA image with the handful of operations the skin engine needs.

use thiserror::Error;

/// Errors raised while decoding an image.
#[derive(Debug, Error)]
pub enum ImageError {
    /// The image data could not be decoded.
    #[error("could not decode the image: {0}")]
    Decode(String),
    /// A sprite rectangle lies outside of the image.
    #[error("the sprite {rect:?} does not fit inside a {width}x{height} image")]
    OutOfBounds {
        /// Requested rectangle.
        rect: Rect,
        /// Image width.
        width: u32,
        /// Image height.
        height: u32,
    },
}

/// A rectangle in image coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Rect {
    /// Creates a new rectangle.
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Right edge (exclusive).
    pub fn right(&self) -> u32 {
        self.x + self.width
    }

    /// Bottom edge (exclusive).
    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }

    /// Whether the rectangle fits inside an image of the given size.
    pub fn fits(&self, width: u32, height: u32) -> bool {
        self.right() <= width && self.bottom() <= height
    }
}

/// An RGBA8 image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Row major RGBA pixels.
    pub pixels: Vec<u8>,
}

impl Image {
    /// Creates an empty (transparent) image.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    /// Decodes an image from its encoded bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, ImageError> {
        let decoded = image::load_from_memory(bytes)
            .map_err(|err| ImageError::Decode(err.to_string()))?
            .into_rgba8();
        let (width, height) = decoded.dimensions();
        Ok(Self {
            width,
            height,
            pixels: decoded.into_raw(),
        })
    }

    /// Returns the colour of a pixel, or `None` when out of bounds.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = ((y as usize * self.width as usize) + x as usize) * 4;
        Some([
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
            self.pixels[index + 3],
        ])
    }

    /// Returns the colour of a pixel, defaulting to transparent black.
    pub fn pixel_or_transparent(&self, x: u32, y: u32) -> [u8; 4] {
        self.pixel(x, y).unwrap_or([0, 0, 0, 0])
    }

    /// Extracts a sub-image.
    pub fn crop(&self, rect: Rect) -> Result<Self, ImageError> {
        if !rect.fits(self.width, self.height) {
            return Err(ImageError::OutOfBounds {
                rect,
                width: self.width,
                height: self.height,
            });
        }

        let mut pixels = Vec::with_capacity((rect.width as usize) * (rect.height as usize) * 4);
        for y in rect.y..rect.bottom() {
            let start = ((y as usize * self.width as usize) + rect.x as usize) * 4;
            let end = start + (rect.width as usize * 4);
            pixels.extend_from_slice(&self.pixels[start..end]);
        }

        Ok(Self {
            width: rect.width,
            height: rect.height,
            pixels,
        })
    }

    /// Whether every pixel is fully transparent.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Draws another image on top of this one at the given position.
    pub fn blit(&mut self, other: &Image, x: i32, y: i32) {
        for row in 0..other.height {
            for column in 0..other.width {
                let source = other.pixel(column, row).unwrap_or([0, 0, 0, 0]);
                if source[3] == 0 {
                    continue;
                }
                let target_x = x + column as i32;
                let target_y = y + row as i32;
                if target_x < 0 || target_y < 0 {
                    continue;
                }
                let (target_x, target_y) = (target_x as u32, target_y as u32);
                if target_x >= self.width || target_y >= self.height {
                    continue;
                }
                let index = ((target_y as usize * self.width as usize) + target_x as usize) * 4;
                self.pixels[index] = source[0];
                self.pixels[index + 1] = source[1];
                self.pixels[index + 2] = source[2];
                self.pixels[index + 3] = source[3];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_extracts_pixels() {
        let mut image = Image::new(4, 4);
        image.pixels[0] = 255;
        let cropped = image.crop(Rect::new(0, 0, 2, 2)).unwrap();
        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.pixels[0], 255);
        assert_eq!(cropped.pixels.len(), 2 * 2 * 4);
    }

    #[test]
    fn crop_rejects_out_of_bounds() {
        let image = Image::new(4, 4);
        assert!(image.crop(Rect::new(2, 2, 4, 4)).is_err());
    }

    #[test]
    fn blit_copies_pixels() {
        let mut base = Image::new(4, 4);
        let mut overlay = Image::new(2, 2);
        overlay.pixels[0..4].copy_from_slice(&[10, 20, 30, 255]);
        base.blit(&overlay, 1, 1);
        assert_eq!(base.pixel(1, 1), Some([10, 20, 30, 255]));
        assert_eq!(base.pixel(0, 0), Some([0, 0, 0, 0]));
    }
}
