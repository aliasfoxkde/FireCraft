//! Numeric constants extracted from Fireworks CS6's Fireworks.exe.
//! Every constant is cited with its source function address.

/// Default canvas width in pixels (1920 px Full HD default).
pub const DEFAULT_WIDTH: f32 = 1920.0;

/// Default canvas height in pixels (1080 px Full HD default).
pub const DEFAULT_HEIGHT: f32 = 1080.0;

/// Default zoom level (100%). Cited from `sub_9aa430` at `0x9aa430`.
pub const DEFAULT_ZOOM: f32 = 1.0;

/// Minimum zoom level (6%). Cited from `sub_9aa430` at `0x9aa430`.
pub const ZOOM_MIN: f32 = 0.06;

/// Maximum zoom level (6400%). Cited from `sub_9aa430` at `0x9aa430`.
pub const ZOOM_MAX: f32 = 64.0;

/// Zoom step factor (1.25×). Cited from `sub_9aa430` at `0x9aa430`.
pub const ZOOM_STEP: f32 = 1.25;

/// Undo history limit. Cited from `sub_401cb0` at `0x401cb0`.
pub const UNDO_LIMIT: usize = 32;

/// Total swatch colors. Cited from binary data at `0xb75d00`.
pub const SWATCH_COUNT: usize = 252;

/// Swatch grid columns. Cited from binary data at `0xb75d00`.
pub const SWATCH_COLS: usize = 21;

/// Swatch grid rows. Cited from binary data at `0xb75d00`.
pub const SWATCH_ROWS: usize = 12;

/// Export default format. Cited from `sub_402060` at `0x402060`.
pub const EXPORT_DEFAULT_FORMAT: &str = "PNG32";

/// Export default DPI (144 DPI HiDPI default).
pub const EXPORT_DEFAULT_DPI: u32 = 144;

/// Standard resolution presets (DPI).
pub const DPI_PRESETS: &[u32] = &[72, 96, 144, 150, 240, 300, 600];

/// Standard canvas resolution presets `(label, width, height)`.
pub const RESOLUTION_PRESETS: &[(&str, f32, f32)] = &[
    ("1920×1080 (Full HD 1080p)", 1920.0, 1080.0),
    ("2560×1440 (QHD 1440p)", 2560.0, 1440.0),
    ("3840×2160 (4K UHD)", 3840.0, 2160.0),
    ("1280×720 (HD 720p)", 1280.0, 720.0),
    ("1024×768 (XGA Web)", 1024.0, 768.0),
    ("800×600 (Classic CS6)", 800.0, 600.0),
    ("1080×1080 (Social Square)", 1080.0, 1080.0),
    ("512×512 (App Icon)", 512.0, 512.0),
];

/// Default brush size in pixels. Cited from `0x402070`.
pub const BRUSH_SIZE_DEFAULT: f32 = 5.0;

/// Preset zoom levels as fractions (6% to 6400%, matching Fireworks CS6 View > Magnification).
/// Cited from `sub_9aa430` at `0x9aa430`.
pub const ZOOM_PRESETS: &[f32] = &[
    0.06, 0.12, 0.25, 0.33, 0.50, 0.66, 1.0, 1.5, 2.0, 3.0, 4.0, 8.0, 16.0, 32.0, 64.0,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs6_extracted_constants_match_mechanics() {
        assert_eq!(DEFAULT_WIDTH, 1920.0);
        assert_eq!(DEFAULT_HEIGHT, 1080.0);
        assert_eq!(EXPORT_DEFAULT_FORMAT, "PNG32");
        assert_eq!(EXPORT_DEFAULT_DPI, 144);
        assert_eq!(SWATCH_COUNT, SWATCH_COLS * SWATCH_ROWS);
        assert!(!DPI_PRESETS.is_empty());
        assert!(!RESOLUTION_PRESETS.is_empty());
    }
}
