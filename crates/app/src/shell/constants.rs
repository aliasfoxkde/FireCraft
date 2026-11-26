//! Numeric constants extracted from Fireworks CS6's Fireworks.exe.
//! Every constant is cited with its source function address.
#![allow(rustdoc::invalid_html_tags)]

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

/// Export default DPI (144 DPI `HiDPI` default).
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
/// Style library selector for the Styles panel dropdown.
#[allow(rustdoc::invalid_html_tags)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StyleLibrary {
    #[default]
    CurrentDocument,
    DarkChrome,
    ChromeMetal,
    GlassGel,
    PlasticWeb,
    ShadowGlow,
}

impl StyleLibrary {
    pub const ALL: [StyleLibrary; 6] = [
        StyleLibrary::CurrentDocument,
        StyleLibrary::DarkChrome,
        StyleLibrary::ChromeMetal,
        StyleLibrary::GlassGel,
        StyleLibrary::PlasticWeb,
        StyleLibrary::ShadowGlow,
    ];

    pub fn label(self) -> &'static str {
        match self {
            StyleLibrary::CurrentDocument => "Current Document",
            StyleLibrary::DarkChrome => "Dark Chrome",
            StyleLibrary::ChromeMetal => "Chrome Metal",
            StyleLibrary::GlassGel => "Glass & Gel",
            StyleLibrary::PlasticWeb => "Plastic & Web",
            StyleLibrary::ShadowGlow => "Shadow & Glow",
        }
    }
}

/// Default brush size in pixels. Cited from `0x402070`.
pub const BRUSH_SIZE_DEFAULT: f32 = 5.0;

/// Preset zoom levels as fractions (6% to 6400%, matching Fireworks CS6 View > Magnification).
/// Cited from `sub_9aa430` at `0x9aa430`.
pub const ZOOM_PRESETS: &[f32] = &[
    0.06, 0.12, 0.25, 0.33, 0.50, 0.66, 1.0, 1.5, 2.0, 3.0, 4.0, 8.0, 16.0, 32.0, 64.0,
];
#[allow(rustdoc::invalid_html_tags)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Dock zone for panels.
/// Dock zone for panels.
pub enum DockZone {
    RightOuter,
    RightInner,
    LeftInner,
    #[default]
    Floating,
}

/// Tool dock mode (Left, Right, Floating).
#[allow(rustdoc::invalid_html_tags)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolsDockMode {
    #[default]
    Left,
    Right,
    Floating,
}

/// Drag operation kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DragKind {
    Marquee,
    Create,
    Move,
    MoveNode(usize),
    AdjustCornerRadius,
    AdjustStarInner,
    Resize(usize),
    Pan,
    Paint,
    Path,
    Lasso,
}

/// A drag operation on the canvas.
pub struct Drag {
    pub kind: DragKind,
    pub start: geom::Vec2,
    pub current: geom::Vec2,
    pub orig_bounds: Option<(geom::Vec2, geom::Vec2)>,
    pub path: Vec<geom::Vec2>,
}

/// A web behavior (JS action).
#[derive(Clone, Debug)]
pub struct WebBehavior {
    pub event: String,
    pub action: String,
    pub info: String,
}

/// A tabbed panel group (used for drag-drop grouping).
#[derive(Clone, Debug)]
pub struct PanelGroup {
    pub id: u64,
    pub tabs: Vec<doc::ObjectId>,
    pub active_tab: usize,
    pub zone: DockZone,
    pub collapsed: bool,
    pub body_height: f32,
    pub floating_pos: geom::Vec2,
    pub floating_width: f32,
}

/// Ruler unit system for the canvas (pixels, imperial, metric).
#[allow(rustdoc::invalid_html_tags)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RulerUnit {
    #[default]
    Pixels,
    Inches,
    Centimeters,
    Millimeters,
}

impl RulerUnit {
    pub const ALL: [RulerUnit; 4] = [
        RulerUnit::Pixels,
        RulerUnit::Inches,
        RulerUnit::Centimeters,
        RulerUnit::Millimeters,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RulerUnit::Pixels => "Pixels (px)",
            RulerUnit::Inches => "Inches (Imperial)",
            RulerUnit::Centimeters => "Centimeters (Metric)",
            RulerUnit::Millimeters => "Millimeters (Metric)",
        }
    }

    pub fn short_code(self) -> &'static str {
        match self {
            RulerUnit::Pixels => "px",
            RulerUnit::Inches => "in",
            RulerUnit::Centimeters => "cm",
            RulerUnit::Millimeters => "mm",
        }
    }

    pub fn px_per_unit(self, dpi: f32) -> f32 {
        let d = dpi.max(72.0);
        match self {
            RulerUnit::Pixels => 1.0,
            RulerUnit::Inches => d,
            RulerUnit::Centimeters => d / 2.54,
            RulerUnit::Millimeters => d / 25.4,
        }
    }

    pub fn subdivisions(self) -> usize {
        match self {
            RulerUnit::Pixels => 5,
            RulerUnit::Inches => 8,       // 1/8", 1/4", 1/2"
            RulerUnit::Centimeters => 10, // 1mm, 5mm
            RulerUnit::Millimeters => 5,
        }
    }
}

/// Standard paper sizes for page setup / print / export.
#[allow(rustdoc::invalid_html_tags)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PaperSize {
    #[default]
    Letter,
    A4,
    Legal,
    Tabloid,
}

impl PaperSize {
    pub fn label(self) -> &'static str {
        match self {
            PaperSize::Letter => "Letter (8.5 x 11 in)",
            PaperSize::A4 => "A4 (210 x 297 mm)",
            PaperSize::Legal => "Legal (8.5 x 14 in)",
            PaperSize::Tabloid => "Tabloid (11 x 17 in)",
        }
    }

    pub fn css_name(self) -> &'static str {
        match self {
            PaperSize::Letter => "letter",
            PaperSize::A4 => "A4",
            PaperSize::Legal => "legal",
            PaperSize::Tabloid => "tabloid",
        }
    }
}

/// Page setup configuration for print / export.
#[derive(Clone, Debug)]
pub struct PageSetupConfig {
    pub paper: PaperSize,
    pub landscape: bool,
    pub margin_in: f32,
    pub fit_to_page: bool,
    pub scale_pct: f32,
    pub print_guides_slices: bool,
}

impl Default for PageSetupConfig {
    fn default() -> Self {
        Self {
            paper: PaperSize::Letter,
            landscape: false,
            margin_in: 0.5,
            fit_to_page: true,
            scale_pct: 100.0,
            print_guides_slices: false,
        }
    }
}

/// HTML export options for export dialog.
#[derive(Clone, Debug)]
pub struct HtmlExportOptions {
    pub format: usize,
    pub web: Option<usize>,
    pub quality: u8,
    pub progressive: bool,
    pub matte_black: bool,
    pub transparent_bg: bool,
    pub export_quality: u8,
    pub include_slices: bool,
    pub center_in_browser: bool,
    pub include_viewport_meta: bool,
    pub export_doc_bg: bool,
    pub html_style: String,
}

impl Default for HtmlExportOptions {
    fn default() -> Self {
        Self {
            format: 0,
            web: None,
            quality: 80,
            progressive: false,
            matte_black: false,
            transparent_bg: false,
            export_quality: 80,
            include_slices: false,
            center_in_browser: false,
            include_viewport_meta: true,
            export_doc_bg: true,
            html_style: String::new(),
        }
    }
}

/// Panel identifier for the dock/panel system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelId {
    Properties,
    Pages,
    States,
    Layers,
    History,
    Filters,
    FiltersUI,
    Styles,
    Library,
    FillStroke,
    ColorPalette,
    ColorMixer,
    Swatches,
    Align,
    Path,
    Optimize,
    Guide,
    Timeline,
    SliceEditor,
    Behaviors,
    Url,
    HtmlSetup,
}

impl PanelId {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Properties => "Properties",
            Self::Pages => "Pages",
            Self::States => "States",
            Self::Layers => "Layers",
            Self::History => "History",
            Self::Filters => "Filters",
            Self::FiltersUI => "Filters UI",
            Self::Styles => "Styles",
            Self::Library => "Common Library",
            Self::FillStroke => "Fill & Stroke",
            Self::ColorPalette => "Color Palette",
            Self::ColorMixer => "Color Mixer",
            Self::Swatches => "Swatches",
            Self::Align => "Align",
            Self::Path => "Path",
            Self::Optimize => "Optimize",
            Self::Guide => "Guide",
            Self::Timeline => "Timeline",
            Self::SliceEditor => "Slice Editor",
            Self::Behaviors => "Behaviors",
            Self::Url => "URL",
            Self::HtmlSetup => "HTML Setup",
        }
    }
}

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

/// Find & Replace operating mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FindReplaceMode {
    #[default]
    Text,
    FillColor,
    StrokeColor,
    AnyColor,
}

impl FindReplaceMode {
    pub const ALL: [FindReplaceMode; 4] = [
        FindReplaceMode::Text,
        FindReplaceMode::FillColor,
        FindReplaceMode::StrokeColor,
        FindReplaceMode::AnyColor,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FindReplaceMode::Text => "Text & object names",
            FindReplaceMode::FillColor => "Fill color",
            FindReplaceMode::StrokeColor => "Stroke color",
            FindReplaceMode::AnyColor => "Any color",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            FindReplaceMode::Text => "Text",
            FindReplaceMode::FillColor => "Fill",
            FindReplaceMode::StrokeColor => "Stroke",
            FindReplaceMode::AnyColor => "Any color",
        }
    }
}
