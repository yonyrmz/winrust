//! Parsers for the plain text configuration files shipped inside a `.wsz`.

use std::collections::HashMap;

use thiserror::Error;

/// Errors raised while parsing a skin configuration file.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// A line could not be parsed.
    #[error("invalid line {line_number}: {line}")]
    InvalidLine {
        /// One based line number.
        line_number: usize,
        /// The offending line.
        line: String,
    },
}

/// An RGBA colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

impl Color {
    /// Creates a fully opaque colour.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Parses a colour from a `255,255,255` or `#RRGGBB` string.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if let Some(hex) = value.strip_prefix('#') {
            if hex.len() == 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                return Some(Self::rgb(r, g, b));
            }
            return None;
        }

        let parts: Vec<&str> = value.split(',').map(str::trim).collect();
        if parts.len() < 3 {
            return None;
        }
        let r = parts[0].parse().ok()?;
        let g = parts[1].parse().ok()?;
        let b = parts[2].parse().ok()?;
        let a = parts
            .get(3)
            .and_then(|value| value.parse().ok())
            .unwrap_or(255);
        Some(Self { r, g, b, a })
    }
}

/// The colours of the playlist window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistTheme {
    /// Background colour of the playlist.
    pub background: Color,
    /// Colour of the regular text.
    pub foreground: Color,
    /// Colour of the text of the currently playing track.
    pub current: Color,
    /// Colour of the text of the selected track.
    pub selected: Color,
    /// Colour of the "normal" title bar of the playlist.
    pub titlebar_active: Color,
    /// Colour of the "inactive" title bar of the playlist.
    pub titlebar_inactive: Color,
    /// Colour of the time display.
    pub time: Color,
}

impl Default for PlaylistTheme {
    fn default() -> Self {
        Self {
            background: Color::rgb(0, 0, 0),
            foreground: Color::rgb(0, 255, 0),
            current: Color::rgb(0, 255, 0),
            selected: Color::rgb(0, 0, 255),
            titlebar_active: Color::rgb(0, 0, 0),
            titlebar_inactive: Color::rgb(0, 0, 0),
            time: Color::rgb(0, 255, 0),
        }
    }
}

/// The 24 general purpose colours of the classic skin format.
pub type GeneralColors = [Color; 24];

/// The colours used by the visualizer.
pub type VisualizerColors = [Color; 24];

/// Parses a `pledit.txt` file.
pub fn parse_pledit(contents: &str) -> Result<PlaylistTheme, ConfigError> {
    let mut theme = PlaylistTheme::default();
    let mut found = false;

    for (index, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('[') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let Some(color) = Color::parse(value) else {
            return Err(ConfigError::InvalidLine {
                line_number: index + 1,
                line: line.to_string(),
            });
        };
        found = true;

        match key.as_str() {
            "normal" => theme.foreground = color,
            "current" => theme.current = color,
            "selected" => theme.selected = color,
            "normalbg" => theme.background = color,
            "titlebar" => theme.titlebar_active = color,
            "titlebarsel" => theme.titlebar_inactive = color,
            "time" => theme.time = color,
            _ => {}
        }
    }

    if !found {
        return Err(ConfigError::InvalidLine {
            line_number: 1,
            line: contents.lines().next().unwrap_or_default().to_string(),
        });
    }

    Ok(theme)
}

/// Parses a `gen.txt` file, which holds 24 colours.
///
/// The classic format stores the colours as a flat list of `r,g,b` triplets.
/// A four component `r,g,b,a` layout is accepted as well.
pub fn parse_general(contents: &str) -> Result<GeneralColors, ConfigError> {
    let values: Vec<u8> = contents
        .split(',')
        .filter_map(|value| value.trim().parse().ok())
        .collect();

    let stride = if values.len().is_multiple_of(3) {
        3
    } else if values.len().is_multiple_of(4) {
        4
    } else {
        return Err(ConfigError::InvalidLine {
            line_number: 1,
            line: contents.to_string(),
        });
    };

    if values.len() < 24 * stride {
        return Err(ConfigError::InvalidLine {
            line_number: 1,
            line: contents.to_string(),
        });
    }

    let mut colors = [Color::rgb(0, 0, 0); 24];
    for (index, color) in colors.iter_mut().enumerate() {
        let base = index * stride;
        *color = Color {
            r: values[base],
            g: values[base + 1],
            b: values[base + 2],
            a: if stride == 4 { values[base + 3] } else { 255 },
        };
    }
    Ok(colors)
}

/// Parses a `viscolor.txt` file, which holds 24 comma separated colours.
pub fn parse_visualizer(contents: &str) -> Result<VisualizerColors, ConfigError> {
    parse_general(contents)
}

/// A rectangular region of a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// Left edge.
    pub left: u32,
    /// Top edge.
    pub top: u32,
    /// Right edge (exclusive).
    pub right: u32,
    /// Bottom edge (exclusive).
    pub bottom: u32,
}

impl Region {
    /// Whether the given point lies inside the region.
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

/// The named regions of the main window, used to implement transparency.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegionSet {
    /// Regions of the main window.
    pub main: Vec<Region>,
    /// Regions of the equalizer window.
    pub equalizer: Vec<Region>,
    /// Regions of the playlist window.
    pub playlist: Vec<Region>,
}

/// Parses a `region.txt` file.
pub fn parse_regions(contents: &str) -> RegionSet {
    let mut regions = RegionSet::default();
    let mut current = RegionTarget::None;

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with('[') {
            current = match lower.as_str() {
                "[normal]" => RegionTarget::Main,
                "[equalizer]" => RegionTarget::Equalizer,
                "[playlist]" => RegionTarget::Playlist,
                "[windowshade]" => RegionTarget::Main,
                _ => RegionTarget::None,
            };
            continue;
        }

        let Some((target, values)) = line.split_once([':', '=']) else {
            continue;
        };
        let target = target.trim().to_ascii_lowercase();
        let numbers: Vec<u32> = values
            .split(',')
            .filter_map(|value| value.trim().parse().ok())
            .collect();
        if numbers.len() < 4 {
            continue;
        }

        let region = Region {
            left: numbers[0],
            top: numbers[1],
            right: numbers[2],
            bottom: numbers[3],
        };

        match target.as_str() {
            "normal" => regions.main.push(region),
            "eqmain" => regions.equalizer.push(region),
            "playlist" => regions.playlist.push(region),
            _ => match current {
                RegionTarget::Main => regions.main.push(region),
                RegionTarget::Equalizer => regions.equalizer.push(region),
                RegionTarget::Playlist => regions.playlist.push(region),
                RegionTarget::None => {}
            },
        }
    }

    regions
}

/// The window a region belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegionTarget {
    /// The main window.
    Main,
    /// The equalizer window.
    Equalizer,
    /// The playlist window.
    Playlist,
    /// No window.
    None,
}

/// Parses a `nums_ex.txt` file, which describes the extra digit glyphs.
pub fn parse_numbers_ex(contents: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pledit() {
        let theme = parse_pledit(
            "[Text]\nNormal=#00FF00\nCurrent=#FFFFFF\nSelected=#0000FF\nNormalBG=#000000\n",
        )
        .unwrap();
        assert_eq!(theme.foreground, Color::rgb(0, 255, 0));
        assert_eq!(theme.current, Color::rgb(255, 255, 255));
        assert_eq!(theme.selected, Color::rgb(0, 0, 255));
    }

    #[test]
    fn parses_general_colors() {
        let mut values = Vec::new();
        for index in 0..24u8 {
            values.push(format!("{},{},{}", index, index, index));
        }
        let colors = parse_general(&values.join(",")).unwrap();
        assert_eq!(colors.len(), 24);
        assert_eq!(colors[0], Color::rgb(0, 0, 0));
        assert_eq!(colors[2], Color::rgb(2, 2, 2));
        assert_eq!(colors[23], Color::rgb(23, 23, 23));
    }

    #[test]
    fn parses_regions() {
        let regions = parse_regions("[Normal]\nNormal=10,20,30,40\n[Equalizer]\nEqMain=1,2,3,4\n");
        assert_eq!(regions.main.len(), 1);
        assert_eq!(regions.main[0].left, 10);
        assert_eq!(regions.equalizer.len(), 1);
    }

    #[test]
    fn rejects_short_general_colors() {
        assert!(parse_general("1,2,3").is_err());
    }
}
