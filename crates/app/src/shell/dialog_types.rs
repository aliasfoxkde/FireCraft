//! Shared dialog structs used by both native and wasm32 builds.
//!
//! Every dialog the shell can open keeps its state in one of these
//! types. They live here, unconditionally compiled, so the wasm32 build
//! and the desktop build cannot drift apart: there is one definition
//! per dialog, and a new field is a compile error in both.
//!
//! The native build uses them from [`super::dialogs`]; the wasm32 build
//! uses the same types with stubbed `Shell` methods.

use egui::TextureHandle;

/// The display quartet plus name, captured read-only from any
/// property-bearing kind.
pub(crate) struct DisplayState {
    pub(super) opacity: i64,
    pub(super) blend: i64,
    pub(super) visible: bool,
    pub(super) locked: bool,
    pub(super) name: Option<String>,
}

/// The Numeric Transform dialog's fields — CS6's Modify ▸ Transform ▸
/// Numeric Transform: a uniform or per-axis scale, then a rotate and a
/// skew, composed about the selection bounds center and applied as one
/// transaction step (`scale` × `rotate` × `skew`, in that order).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NumericTransformDialog {
    /// Horizontal scale percent (100 = unchanged).
    pub scale_x: f64,
    /// Vertical scale percent.
    pub scale_y: f64,
    /// Uniform-scale toggle: `scale_y` follows `scale_x`.
    pub uniform: bool,
    /// Clockwise rotation in degrees.
    pub rotate: f64,
    /// Horizontal skew angle in degrees.
    pub skew_x: f64,
    /// Vertical skew angle in degrees.
    pub skew_y: f64,
}

impl Default for NumericTransformDialog {
    fn default() -> Self {
        Self {
            scale_x: 100.0,
            scale_y: 100.0,
            uniform: true,
            rotate: 0.0,
            skew_x: 0.0,
            skew_y: 0.0,
        }
    }
}

/// What a confirmed action runs once the user says yes. This is the
/// shell's confirmation mechanism — decision 0117 named its absence
/// as the reason `document.deleteAllInDocument` could not be bound
/// ("a confirmed action that does not exist yet").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConfirmAction {
    /// Empty the document (`document.deleteAllInDocument`).
    DeleteAll,
}

/// The confirm dialog's state: the question asked, and what "Yes"
/// runs. "No" (or any dismissal) leaves the document untouched.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConfirmDialog {
    /// The question shown to the user.
    pub question: String,
    /// What "Yes" runs.
    pub action: ConfirmAction,
}

/// State for the scale-to-selection dialog.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ScaleToDialog {
    /// Target selection width in pixels.
    pub width: f64,
    /// Target selection height in pixels.
    pub height: f64,
}

/// The Feather dialog's field — Select ▸ Feather…: softens the active
/// pixel selection's edge by a radius in pixels (1..=64, the engine's
/// kernel cap).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FeatherDialog {
    /// Feather radius in pixels.
    pub radius: u8,
}

impl Default for FeatherDialog {
    fn default() -> Self {
        Self { radius: 2 }
    }
}

/// The Canvas Size dialog's fields — Modify ▸ Canvas ▸ Canvas Size:
/// absolute width/height in document units plus the CS6 anchor grid
/// (which corner the resize grows from).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CanvasSizeDialog {
    /// New canvas width in document units.
    pub width: f64,
    /// New canvas height in document units.
    pub height: f64,
    /// Anchor column 0..=2 (left/center/right).
    pub anchor_x: usize,
    /// Anchor row 0..=2 (top/middle/bottom).
    pub anchor_y: usize,
}

impl Default for CanvasSizeDialog {
    fn default() -> Self {
        Self {
            width: 640.0,
            height: 480.0,
            anchor_x: 1,
            anchor_y: 1,
        }
    }
}

/// The Canvas Color dialog's field — Modify ▸ Canvas ▸ Canvas Color:
/// the document-wide matte (`document.setCanvasColor`), ABGR raw.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CanvasColorDialog {
    /// Matte color, sRGB bytes (r, g, b).
    pub color: [u8; 3],
}

impl Default for CanvasColorDialog {
    fn default() -> Self {
        Self {
            color: [255, 255, 255],
        }
    }
}

/// The Image Size dialog's fields — Modify ▸ Image Size (CS6
/// command 35226): scales the document into the target rectangle
/// (`document.setImageSize`) and sets the resolution.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ImageSizeDialog {
    /// New width in document units.
    pub width: f64,
    /// New height in document units.
    pub height: f64,
    /// Pixels per unit for the document resolution.
    pub resolution: f64,
    /// Whether the resolution unit is cm (false = inch).
    pub unit_cm: bool,
}

impl Default for ImageSizeDialog {
    fn default() -> Self {
        Self {
            width: 640.0,
            height: 480.0,
            resolution: 72.0,
            unit_cm: false,
        }
    }
}

/// Export-dialog state (File → Export…): format, per-format controls,
/// and the cached 2-Up preview. The controls mirror the `EXPv` fields
/// the model maps; Export stamps them onto a cloned document and runs
/// the same codec path the CLI does. A web workflow replaces the
/// raster format entirely — the dialog then drives
/// `app_cli::ops::export_web`, the same entry point the CLI verb
/// shares.
pub(crate) struct ExportDialog {
    /// Index into `EXPORT_FORMATS`.
    pub(super) format: usize,
    /// Selected web workflow, or `None` for the raster formats.
    pub(super) web: Option<usize>,
    /// Emit the 1px shim row/column (web table workflows).
    pub(super) web_shim: bool,
    /// Give areas without slices their own image (`include_undefined`).
    pub(super) web_undefined: bool,
    /// JPEG quality 1..=100.
    pub(super) quality: i32,
    /// Progressive JPEG scans.
    pub(super) progressive: bool,
    /// Matte for alpha-less formats: false = white, true = black.
    pub(super) matte_black: bool,
    /// GIF loop: 0 infinite, 1 never, 2 repeat n times.
    pub(super) gif_loop: usize,
    /// Repeat count when `gif_loop == 2`.
    pub(super) gif_times: i32,
    /// GIF per-frame ink crop (`AACb`).
    pub(super) gif_crop: bool,
    /// GIF diff frames (`AADb`).
    pub(super) gif_diff: bool,
    /// GIF Floyd–Steinberg amount (`PLDi`), percent.
    pub(super) gif_dither: i32,
    /// Export transparent background for formats that support it (PNG, WebP, SVG).
    pub(super) transparent_bg: bool,
    /// Export quality percentage (applies to lossy formats: JPEG, WebP, AVIF).
    pub(super) export_quality: i32,
    /// Include slices and hotspots in HTML export.
    pub(super) include_slices: bool,
    /// Center the stage in browser window.
    pub(super) center_in_browser: bool,
    /// Include responsive viewport meta tag.
    pub(super) include_viewport_meta: bool,
    /// Export document background color.
    pub(super) export_doc_bg: bool,
    /// HTML export style: 0=HTML5, 1=XHTML, 2=CSS Grid.
    pub(super) html_style: usize,
    /// Cached 2-Up panes and the settings stamp they were made from.
    pub(super) preview: Option<(u64, TextureHandle, String, TextureHandle, String)>,
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self {
            format: 0,
            web: None,
            web_shim: true,
            web_undefined: true,
            quality: 90,
            progressive: false,
            matte_black: false,
            gif_loop: 0,
            gif_times: 3,
            gif_crop: false,
            gif_diff: false,
            gif_dither: 0,
            transparent_bg: false,
            export_quality: 90,
            include_slices: true,
            center_in_browser: true,
            include_viewport_meta: true,
            export_doc_bg: true,
            html_style: 0,
            preview: None,
        }
    }
}
