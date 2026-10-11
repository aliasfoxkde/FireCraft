//! Deterministic text layout (`text`, L2).
//!
//! Shapes a [`TextObject`]'s runs over the **bundled fonts only** — no
//! system font scan — and lowers the shaped glyphs to document
//! [`Contour`]s, the same geometry paths use. Rendering text therefore
//! reuses the entire path pipeline (fill, blend, opacity, live
//! effects); layout goldens pin a font + string → contour bytes.
//!
//! Determinism contract: same font bytes + same model object → same
//! contour bytes on every platform. Enforced by building the
//! [`FontSystem`] from embedded binaries with a pinned locale and an
//! empty fallback list, disabling hinting (pure outline scaling), and
//! pinning every Fireworks attribute mapping below.
//!
//! Attribute mappings pinned for M7 (corpus may refine later):
//! - `PTSf` missing or non-positive → [`DEFAULT_POINT_SIZE`].
//! - `LEDf`/`LDMi`: unit 0 = em × size, unit 1 = pixels; missing or
//!   non-positive leading → [`DEFAULT_LEADING_FACTOR`] × size.
//! - `JSTi`: 0 left, 1 centre, 2 right (dictionary). Justified lays out
//!   left — the glyph-stream justification pass is not modeled yet.
//! - `RKNf` kerning/tracking: per-mille of em, accumulated per glyph
//!   index within a line (`kern_px · glyph_index`).
//! - `HSCf` horizontal scale: percent, 100 = identity. Glyph positions
//!   scale about the line origin, glyph ink about its own origin —
//!   both x-only.
//! - `BLSf` baseline shift: pixels, positive raises the line.
//! - `TFTi` flows 2 (follow path) and 3 (fit inside shape) lay out as
//!   flow 0 until M7 task 8 implements text-on-path.
//! - `FONs` resolves against the bundled face list; every unknown name
//!   renders in [`SANS_FAMILY`]. The name itself round-trips untouched
//!   at the format layer; substitution behavior beyond this table waits
//!   on corpus evidence.

use cosmic_text::{
    Align, Attrs, Buffer, CacheKey, CacheKeyFlags, Family, FontSystem, Metrics, Shaping, Style,
    SwashCache, Weight,
};
use doc::{Contour, PathPoint, TextObject, TextStyle};
use swash::zeno::Command;

/// The bundled sans family and the substitute for every unknown `FONs`.
pub const SANS_FAMILY: &str = "Liberation Sans";

/// Point size used when the model carries no usable `PTSf`.
pub const DEFAULT_POINT_SIZE: f64 = 12.0;

/// Leading factor used when the model carries no usable `LEDf`/`LDMi`.
pub const DEFAULT_LEADING_FACTOR: f32 = 1.2;

/// Underline metrics when the shaped line carries no decoration span to
/// read the font's own from (em units, cosmic-text's defaults).
const FALLBACK_UNDERLINE: (f32, f32) = (-0.125, 1.0 / 14.0);

/// `HSCf` percent that means "no scaling".
const HORIZONTAL_SCALE_IDENTITY: f64 = 100.0;

/// `RKNf` is per-mille of em (Fireworks' tracking unit).
const KERNING_PER_MILLE: f64 = 1000.0;

/// The bundled face binaries: `include_bytes!` keeps them in the binary
/// so wasm ships fonts with the layout engine, and the deterministic
/// contract never depends on a filesystem.
const FACES: &[&[u8]] = &[
    include_bytes!("../assets/LiberationSans-Regular.ttf"),
    include_bytes!("../assets/LiberationSans-Bold.ttf"),
];

/// Laid-out text as document geometry: glyph outlines (plus underline
/// rules) in the text object's frame space.
#[derive(Clone, Debug, PartialEq)]
pub struct TextGeometry {
    /// Glyph outlines and underline rectangles, in frame space.
    pub contours: Vec<Contour>,
    /// Bounds of all laid-out ink, or `None` when nothing laid out.
    pub ink: Option<geom::Rect>,
    /// Laid-out block size (widest line width, total height) — the
    /// input to auto-enlarge.
    pub block: (f64, f64),
}

/// Shaping engine over the bundled faces. Construction is the expensive
/// part; hosts (compose goldens, the app) create one and keep it.
pub struct TextEngine {
    font_system: FontSystem,
    swash: SwashCache,
}

impl Default for TextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEngine {
    /// Builds a font system holding exactly the bundled faces, with a
    /// pinned locale and an empty fallback list — nothing on this host
    /// can change what a given text lays out to.
    pub fn new() -> Self {
        let mut db = fontdb::Database::new();
        for face in FACES {
            db.load_font_source(fontdb::Source::Binary(std::sync::Arc::new(*face)));
        }
        Self {
            font_system: FontSystem::new_with_locale_and_db_and_fallback(
                "en".to_owned(),
                db,
                NoFallback,
            ),
            swash: SwashCache::new(),
        }
    }

    /// Lays `object`'s runs out and lowers them to contours. Pure over
    /// the model: the same object always yields the same geometry.
    pub fn glyph_contours(&mut self, object: &TextObject) -> TextGeometry {
        let style = object.style.clone().unwrap_or_default();
        let text: String = object
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<&str>>()
            .concat();
        if text.trim().is_empty() {
            return TextGeometry {
                contours: Vec::new(),
                ink: None,
                block: (0.0, 0.0),
            };
        }

        let size = positive(style.size).unwrap_or(DEFAULT_POINT_SIZE);
        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics {
                font_size: size as f32,
                line_height: line_height_of(&style, size),
            },
        );
        let attrs = Attrs::new()
            .family(Family::Name(family_of(&style)))
            .weight(if style.bold == Some(true) {
                Weight::BOLD
            } else {
                Weight::NORMAL
            })
            .style(if style.italic == Some(true) {
                Style::Italic
            } else {
                Style::Normal
            })
            // Unhinted: outlines are pure scalings of the font's curves,
            // which is what makes the contour bytes platform-stable.
            .cache_key_flags(CacheKeyFlags::DISABLE_HINTING);
        buffer.set_text(
            &text,
            &attrs,
            Shaping::Advanced,
            Some(justification_of(&style)),
        );
        // Normal flow wraps at the frame width; auto-enlarge is a height
        // concern (flows 2/3 are task 8 and fall back to this).
        let frame_width = object.frame.width();
        buffer.set_size((frame_width > 0.0).then_some(frame_width as f32), None);
        buffer.shape_until_scroll(&mut self.font_system, true);

        // Per-line Fireworks adjustments: kerning accumulates by glyph
        // index; horizontal scale multiplies positions about the line
        // origin; baseline shift raises the line.
        let kern_px = kerning_px(&style, size);
        let hs = horizontal_scale_of(&style);
        let baseline_shift = style.baseline_shift.unwrap_or(0.0);

        let mut builder = ContourBuilder::default();
        // The laid-out block hangs off the frame's top-left corner: the
        // first line box starts at the frame top, lines flow down.
        let (origin_x, origin_y) = (object.frame.x0, object.frame.y0);
        let mut widest = 0.0f64;
        let mut total_height = 0.0f64;
        for run in buffer.layout_runs() {
            let line_y = f64::from(run.line_y) - baseline_shift + origin_y;
            for (index, glyph) in run.glyphs.iter().enumerate() {
                let line_x = (f64::from(glyph.x) + kern_px * index as f64) * hs;
                let y = f64::from(glyph.y - glyph.font_size * glyph.y_offset) + line_y;
                let mut emitter = OutlineEmitter {
                    builder: &mut builder,
                    origin: (line_x + origin_x, y),
                    hs,
                };
                let cache_key = CacheKey::new(
                    glyph.font_id,
                    glyph.glyph_id,
                    glyph.font_size,
                    (0.0, 0.0),
                    glyph.font_weight,
                    glyph.cache_key_flags,
                )
                .0;
                if let Some(commands) = self
                    .swash
                    .get_outline_commands(&mut self.font_system, cache_key)
                {
                    for command in commands {
                        emitter.emit(command);
                    }
                }
                widest = widest.max(line_x + f64::from(glyph.w) * hs);
            }
            total_height = f64::from(run.line_top + run.line_height);
            if style.underline == Some(true) {
                emit_underlines(&mut builder, &run, line_y, origin_x, hs, kern_px);
            }
        }

        let mut contours = builder.finish();
        if let Some(transform) = object.transform {
            for contour in &mut contours {
                for point in &mut contour.points {
                    point.anchor = transform * point.anchor;
                    if let Some(primary) = &mut point.primary {
                        *primary = transform * *primary;
                    }
                    if let Some(secondary) = &mut point.secondary {
                        *secondary = transform * *secondary;
                    }
                }
            }
        }
        let ink = ink_of(&contours);
        TextGeometry {
            contours,
            ink,
            block: (widest, total_height),
        }
    }
}

/// One shaped line's underline rules, from the font's own underline
/// metrics (em units; the same formulas cosmic-text's raster path
/// uses). A line with no decoration span draws one rule with the
/// pinned fallback metrics.
fn emit_underlines(
    builder: &mut ContourBuilder,
    run: &cosmic_text::LayoutRun,
    line_y: f64,
    origin_x: f64,
    hs: f64,
    kern_px: f64,
) {
    let extent = |range: std::ops::Range<usize>| {
        let mut x0 = f64::INFINITY;
        let mut x1 = f64::NEG_INFINITY;
        for (index, glyph) in run.glyphs.iter().enumerate() {
            if !range.contains(&index) {
                continue;
            }
            let start = (f64::from(glyph.x) + kern_px * index as f64) * hs + origin_x;
            x0 = x0.min(start);
            x1 = x1.max(start + f64::from(glyph.w) * hs);
        }
        (x1 > x0).then_some((x0, x1))
    };
    if run.decorations.is_empty() {
        if let Some((x0, x1)) = extent(0..run.glyphs.len()) {
            let font_size = run
                .glyphs
                .first()
                .map_or(0.0, |glyph| f64::from(glyph.font_size));
            let (offset, thickness_em) = FALLBACK_UNDERLINE;
            let thickness = (f64::from(thickness_em) * font_size).max(1.0).ceil();
            let y = line_y - f64::from(offset) * font_size;
            builder.rectangle(x0, y, x1, y + thickness);
        }
        return;
    }
    for span in run.decorations {
        let Some((x0, x1)) = extent(span.glyph_range.clone()) else {
            continue;
        };
        let font_size = f64::from(span.font_size);
        let thickness = (f64::from(span.data.underline_metrics.thickness) * font_size)
            .max(1.0)
            .ceil();
        let y = line_y - f64::from(span.data.underline_metrics.offset) * font_size;
        builder.rectangle(x0, y, x1, y + thickness);
    }
}

/// `LEDf`/`LDMi` → pixel line height.
fn line_height_of(style: &TextStyle, size: f64) -> f32 {
    let px = |leading: f64| match style.leading_unit {
        // 0 = em (dictionary); a missing unit reads as em too.
        Some(0) | None => leading * size,
        // 1 = pixels; anything else observed falls back here.
        _ => leading,
    };
    style
        .leading
        .filter(|leading| *leading > 0.0 && leading.is_finite())
        .map(px)
        .unwrap_or(size * f64::from(DEFAULT_LEADING_FACTOR)) as f32
}

/// `JSTi` → the shaper's alignment.
fn justification_of(style: &TextStyle) -> Align {
    match style.justification {
        Some(1) => Align::Center,
        Some(2) => Align::Right,
        _ => Align::Left,
    }
}

/// `RKNf` → tracking pixels per glyph index.
fn kerning_px(style: &TextStyle, size: f64) -> f64 {
    style
        .kerning
        .filter(|kerning| kerning.is_finite())
        .map_or(0.0, |kerning| kerning / KERNING_PER_MILLE * size)
}

/// `HSCf` → horizontal scale factor about the line origin.
fn horizontal_scale_of(style: &TextStyle) -> f64 {
    style
        .horizontal_scale
        .filter(|scale| *scale > 0.0 && scale.is_finite())
        .map_or(1.0, |scale| scale / HORIZONTAL_SCALE_IDENTITY)
}

/// `FONs` resolved against the bundled face list. One family ships
/// today, so every name lands on it — the match documents the extension
/// point (a real face table) without pretending one exists.
fn family_of(style: &TextStyle) -> &str {
    match &style.font {
        Some(font) if font.eq_ignore_ascii_case(SANS_FAMILY) => SANS_FAMILY,
        _ => SANS_FAMILY,
    }
}

fn positive(value: Option<f64>) -> Option<f64> {
    value.filter(|value| *value > 0.0 && value.is_finite())
}

/// Bounds over every contour's anchors and handles.
fn ink_of(contours: &[Contour]) -> Option<geom::Rect> {
    let mut ink: Option<geom::Rect> = None;
    for contour in contours {
        for point in &contour.points {
            include_point(&mut ink, &point.anchor);
            if let Some(handle) = &point.primary {
                include_point(&mut ink, handle);
            }
            if let Some(handle) = &point.secondary {
                include_point(&mut ink, handle);
            }
        }
    }
    ink
}

fn include_point(ink: &mut Option<geom::Rect>, point: &geom::Point) {
    let slot = ink.unwrap_or_else(|| geom::Rect::new(point.x, point.y, point.x, point.y));
    ink.replace(geom::Rect::new(
        slot.x0.min(point.x),
        slot.y0.min(point.y),
        slot.x1.max(point.x),
        slot.y1.max(point.y),
    ));
}

/// Collects zeno outline commands into document contours. One contour
/// per subpath; the closing segment folds into the first point (the
/// model encodes closed contours without a duplicated anchor).
#[derive(Default)]
struct ContourBuilder {
    contours: Vec<Contour>,
    current: Vec<OutlinePoint>,
}

/// One outline anchor with its bezier handles (frame space, y-down).
struct OutlinePoint {
    anchor: geom::Point,
    primary: Option<geom::Point>,
    secondary: Option<geom::Point>,
}

impl ContourBuilder {
    fn move_to(&mut self, x: f64, y: f64) {
        self.flush(false);
        self.current.push(OutlinePoint {
            anchor: geom::Point::new(x, y),
            primary: None,
            secondary: None,
        });
    }

    fn line_to(&mut self, x: f64, y: f64) {
        if self.current.is_empty() {
            self.move_to(x, y);
            return;
        }
        self.current.push(OutlinePoint {
            anchor: geom::Point::new(x, y),
            primary: None,
            secondary: None,
        });
    }

    fn quad_to(&mut self, control: geom::Point, to: geom::Point) {
        // The model's handles are cubic-only; a quadratic lifts exactly
        // into a cubic (the 2/3 rule) with no flattening.
        let from = self.current.last().map_or(control, |point| point.anchor);
        if let Some(last) = self.current.last_mut() {
            last.secondary = Some(geom::Point::new(
                from.x + (control.x - from.x) * (2.0 / 3.0),
                from.y + (control.y - from.y) * (2.0 / 3.0),
            ));
        }
        self.current.push(OutlinePoint {
            anchor: to,
            primary: Some(geom::Point::new(
                to.x + (control.x - to.x) * (2.0 / 3.0),
                to.y + (control.y - to.y) * (2.0 / 3.0),
            )),
            secondary: None,
        });
    }

    fn curve_to(&mut self, control_a: geom::Point, control_b: geom::Point, to: geom::Point) {
        if let Some(last) = self.current.last_mut() {
            last.secondary = Some(control_a);
        }
        self.current.push(OutlinePoint {
            anchor: to,
            primary: Some(control_b),
            secondary: None,
        });
    }

    /// A rectangle as one closed contour (underline rules).
    fn rectangle(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.flush(false);
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
            self.current.push(OutlinePoint {
                anchor: geom::Point::new(x, y),
                primary: None,
                secondary: None,
            });
        }
        self.flush(true);
    }

    /// `Close` from the shaper: every font contour is closed, whether
    /// or not the outline re-emits the start anchor before closing.
    fn close_path(&mut self) {
        self.flush(true);
    }

    /// Folds the current subpath into a contour. A final anchor that
    /// repeats the subpath's start folds into it: the model's closed
    /// contours end at the last distinct anchor, and the closing
    /// segment's incoming control moves to the first point — exactly
    /// the pair `to_bezpath` re-emits for a closed contour.
    fn flush(&mut self, force_closed: bool) {
        if self.current.len() < 2 {
            self.current.clear();
            return;
        }
        let mut closed = force_closed;
        let repeats_start = self
            .current
            .last()
            .is_some_and(|last| last.anchor == self.current[0].anchor);
        if repeats_start {
            let last = self.current.pop();
            if let (Some(primary), Some(first)) = (
                last.and_then(|point| point.primary),
                self.current.first_mut(),
            ) {
                first.primary = Some(primary);
            }
            closed = true;
            if self.current.len() < 2 {
                self.current.clear();
                return;
            }
        }
        let points: Vec<_> = self
            .current
            .drain(..)
            .map(|point| PathPoint {
                anchor: point.anchor,
                primary: point.primary,
                secondary: point.secondary,
                is_curve: point.primary.is_some() || point.secondary.is_some(),
            })
            .collect();
        self.contours.push(Contour {
            closed,
            point_count: points.len() as i64,
            points,
        });
    }

    fn finish(mut self) -> Vec<Contour> {
        self.flush(false);
        self.contours
    }
}

/// Translates one glyph's zeno commands into the builder, applying the
/// glyph's placement and horizontal scale (ink scales about its own
/// origin so it stays anchored at the positioned pen).
struct OutlineEmitter<'a> {
    builder: &'a mut ContourBuilder,
    origin: (f64, f64),
    hs: f64,
}

impl OutlineEmitter<'_> {
    /// Shaper outline commands are y-up above the baseline; the document
    /// is y-down — flip y while placing (horizontal scale is x-only).
    fn map(&self, point: swash::zeno::Point) -> geom::Point {
        let (x, y): (f32, f32) = point.into();
        geom::Point::new(
            self.origin.0 + f64::from(x) * self.hs,
            self.origin.1 - f64::from(y),
        )
    }

    fn emit(&mut self, command: &Command) {
        match command {
            Command::MoveTo(point) => {
                let at = self.map(*point);
                self.builder.move_to(at.x, at.y);
            }
            Command::LineTo(point) => {
                let at = self.map(*point);
                self.builder.line_to(at.x, at.y);
            }
            Command::QuadTo(control, to) => {
                let (control, to) = (self.map(*control), self.map(*to));
                self.builder.quad_to(control, to);
            }
            Command::CurveTo(control_a, control_b, to) => {
                let (a, b, to) = (self.map(*control_a), self.map(*control_b), self.map(*to));
                self.builder.curve_to(a, b, to);
            }
            Command::Close => self.builder.close_path(),
        }
    }
}

/// The empty fallback list: a glyph the bundled faces cannot cover lays
/// out as the shaper's notdef, never as a system font's shape.
struct NoFallback;

impl cosmic_text::Fallback for NoFallback {
    fn common_fallback(&self) -> &[&'static str] {
        &[]
    }

    fn forbidden_fallback(&self) -> &[&'static str] {
        &[]
    }

    fn script_fallback(&self, _script: unicode_script::Script, _locale: &str) -> &[&'static str] {
        &[]
    }
}
