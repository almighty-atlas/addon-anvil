//! Dux-inspired semantic colors. Optional matugen JSON; no Dux runtime dependency.
use iced::Color;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Colors {
    pub background: Color,
    pub surface_container: Color,
    pub surface_container_high: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub primary: Color,
    pub on_primary: Color,
    pub outline: Color,
    pub error: Color,
}

#[derive(Deserialize)]
struct ColorFile {
    background: String,
    surface_container: String,
    surface_container_high: String,
    on_surface: String,
    on_surface_variant: String,
    primary: String,
    on_primary: String,
    outline: String,
    error: String,
}

fn hex_color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(hex, 16).ok()?;
    Some(Color::from_rgb8(
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ))
}

impl Colors {
    pub fn fallback() -> Self {
        Self {
            background: Color::from_rgb8(20, 21, 27),
            surface_container: Color::from_rgb8(29, 30, 38),
            surface_container_high: Color::from_rgb8(42, 43, 54),
            on_surface: Color::from_rgb8(240, 238, 247),
            on_surface_variant: Color::from_rgb8(188, 187, 205),
            primary: Color::from_rgb8(186, 170, 255),
            on_primary: Color::from_rgb8(37, 27, 77),
            outline: Color::from_rgb8(111, 111, 132),
            error: Color::from_rgb8(255, 168, 180),
        }
    }

    fn from_json(source: &str) -> Option<Self> {
        let data: ColorFile = serde_json::from_str(source).ok()?;
        Some(Self {
            background: hex_color(&data.background)?,
            surface_container: hex_color(&data.surface_container)?,
            surface_container_high: hex_color(&data.surface_container_high)?,
            on_surface: hex_color(&data.on_surface)?,
            on_surface_variant: hex_color(&data.on_surface_variant)?,
            primary: hex_color(&data.primary)?,
            on_primary: hex_color(&data.on_primary)?,
            outline: hex_color(&data.outline)?,
            error: hex_color(&data.error)?,
        })
    }

    fn file_path() -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        Some(config.join("addon-anvil").join("colors.json"))
    }

    pub fn active() -> Self {
        let Some(path) = Self::file_path() else {
            return Self::fallback();
        };
        // Color configuration is not allowed to consume arbitrary memory.
        if !std::fs::metadata(&path)
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() <= 4096)
        {
            return Self::fallback();
        }
        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| Self::from_json(&content))
            .unwrap_or_else(Self::fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_matugen_hex_and_rejects_invalid_values() {
        assert!(hex_color("#A1b2C3").is_some());
        assert!(hex_color("#aabbccdd").is_none());
        assert!(hex_color("#xyz123").is_none());
        assert!(hex_color("aabbcc").is_none());
    }

    #[test]
    fn falls_back_for_incomplete_palette() {
        assert!(Colors::from_json(r##"{"primary":"#123456"}"##).is_none());
    }

    #[test]
    fn parses_complete_palette() {
        let json = r##"{
            "background":"#101010", "surface_container":"#202020",
            "surface_container_high":"#303030", "on_surface":"#ffffff",
            "on_surface_variant":"#bbbbbb", "primary":"#ABCDEF",
            "on_primary":"#101010", "outline":"#666666", "error":"#ff4444"
        }"##;
        let palette = Colors::from_json(json).unwrap();
        assert_eq!(palette.primary, Color::from_rgb8(0xAB, 0xCD, 0xEF));
    }
}
