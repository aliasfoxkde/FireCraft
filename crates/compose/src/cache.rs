//! The live-effect cache: content-hash → filtered pixels (PLANNING
//! §7.3 "live filters compose lazily with caching keyed on content
//! hashes").
//!
//! Keys are FNV-1a 64 over every input that decides the filtered
//! raster — canvas size, stack plan, and either the object's rendering
//! content (paths: geometry + paints, skipping a re-render on hit) or
//! the already-rendered base pixels (groups: children can be edited
//! under a stable id, so only pixels are a safe identity). FNV-1a is
//! not cryptographic, but a collision needs two byte streams that
//! differ while agreeing over 64 folded bits — and the worst case is
//! one stale-filtered frame until the entry is replaced, not
//! corruption: nothing but this cache reads it.

use std::collections::HashMap;

use crate::effects::{KernelOp, Plan};

/// Upper bound on retained entries. Overflow clears the whole map —
/// crude but content-correct, and a live filter edit touches one key
/// per object, so real sessions stay orders of magnitude below it.
const ENTRY_LIMIT: usize = 4096;

/// One filtered raster: premultiplied RGBA8 at the canvas size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Canvas width the pixels were filtered at.
    pub width: u16,
    /// Canvas height the pixels were filtered at.
    pub height: u16,
    /// Premultiplied RGBA8, `width * height * 4` bytes.
    pub pixels: Vec<u8>,
}

/// Kernel-time cache of filtered object rasters, handed to
/// [`render_with_cache`](crate::render_with_cache) by hosts that
/// re-render repeatedly between edits (shells, export previews).
/// Content-keyed: entries can never be wrong, only evicted.
#[derive(Default)]
pub struct EffectCache {
    entries: HashMap<u64, Entry>,
}

impl EffectCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Retained entry for `key`, when present.
    #[must_use]
    pub fn get(&self, key: u64) -> Option<&Entry> {
        self.entries.get(&key)
    }

    /// Stores a filtered raster, discarding everything past
    /// [`ENTRY_LIMIT`].
    pub fn insert(&mut self, key: u64, width: u16, height: u16, pixels: Vec<u8>) {
        if self.entries.len() >= ENTRY_LIMIT {
            self.entries.clear();
        }
        self.entries.insert(
            key,
            Entry {
                width,
                height,
                pixels,
            },
        );
    }

    /// Drops every entry (hosts call this when the palette or a global
    /// setting that escapes the hash changes).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Retained entry count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Cache key for a filtered path: the full rendering content (geometry,
/// paints, paint order, even-odd) plus the plan and canvas. Blend and
/// opacity are excluded on purpose — they wrap the filtered image in
/// the parent layer and never touch the cached bytes.
pub(crate) fn hash_path(path: &doc::PathObject, canvas: (u16, u16), plan: &Plan) -> u64 {
    let mut h = Hasher::new(b"compose-effect-path-v1");
    h.u16_pair(canvas);
    for contour in &path.contours {
        h.bool(contour.closed);
        for point in &contour.points {
            h.f64(point.anchor.x);
            h.f64(point.anchor.y);
            h.opt_xy(point.primary.map(|p| (p.x, p.y)));
            h.opt_xy(point.secondary.map(|p| (p.x, p.y)));
            h.bool(point.is_curve);
        }
    }
    h.bool(path.even_odd);
    h.i64(path.paint.as_ref().map_or(0, |paint| paint.fill_raw));
    hash_fill_model(&mut h, path);
    match path.paint.as_ref().map(|paint| paint.stroke_raw) {
        Some(stroke) => {
            h.bool(true);
            h.i64(stroke);
        }
        None => h.bool(false),
    }
    h.opt_bool(path.fill_on_top);
    hash_plan(&mut h, plan);
    h.finish()
}

/// Cache key for a filtered group: the already-rendered base pixels
/// plus the plan. Children are content-hashable only by rendering them,
/// so the pixels *are* the content identity here.
pub(crate) fn hash_pixels(base: &[u8], canvas: (u16, u16), plan: &Plan) -> u64 {
    let mut h = Hasher::new(b"compose-effect-group-v1");
    h.u16_pair(canvas);
    h.bytes(base);
    hash_plan(&mut h, plan);
    h.finish()
}

/// The fill definition and its placement handles — everything the
/// brush painters read beyond `fill_raw`. A gradient/pattern fill and
/// a same-`fill_raw` solid render differently, so the model rides the
/// key; `brush_placement` (a stroke-path concern, not yet painted) is
/// excluded until the stroke brush consumes it.
fn hash_fill_model(h: &mut Hasher, path: &doc::PathObject) {
    for point in [path.fill_start, path.fill_end, path.fill_third] {
        h.opt_xy(point.map(|p| (p.x, p.y)));
    }
    match path.paint.as_ref().and_then(|paint| paint.fill.as_ref()) {
        Some(fill) => {
            h.bool(true);
            h.bytes(&[fill.shape as u8]);
            h.bool(fill.shape_extended);
            h.i64(fill.texture_blend);
            h.i64(fill.feather);
            h.i64(fill.stamping.unwrap_or(0));
            h.bool(fill.hard_edge);
            for stops in [
                fill.gradient.as_ref().map(|g| &g.vertices),
                fill.gradient.as_ref().map(|g| &g.rgb),
                fill.gradient.as_ref().map(|g| &g.alpha),
            ] {
                match stops {
                    Some(stops) => {
                        h.bool(true);
                        h.usize(stops.len());
                        for stop in stops {
                            h.f64(stop.position);
                            h.i64(stop.color_raw);
                        }
                    }
                    None => h.bool(false),
                }
            }
            match fill.dither {
                Some(dither) => {
                    h.bool(true);
                    for color in dither.colors {
                        h.i64(color.unwrap_or(0));
                    }
                    h.bool(dither.transparent.unwrap_or(false));
                }
                None => h.bool(false),
            }
            match fill.pattern.as_ref() {
                Some(pattern) => {
                    h.bool(true);
                    h.i64(pattern.pixels.0);
                    h.i64(pattern.pixels.1);
                    h.i64(pattern.tile_size);
                    h.usize(pattern.tiles.len());
                    for tile in &pattern.tiles {
                        match tile {
                            doc::TileRef::Tile(id) => {
                                h.bool(true);
                                h.i64(i64::from(*id));
                            }
                            doc::TileRef::Matte(raw) => {
                                h.bool(false);
                                h.i64(*raw);
                            }
                        }
                    }
                }
                None => h.bool(false),
            }
        }
        None => h.bool(false),
    }
}

fn hash_plan(h: &mut Hasher, plan: &Plan) {
    h.usize(plan.ops.len());
    for op in &plan.ops {
        match op {
            KernelOp::BrightnessContrast {
                brightness,
                contrast,
            } => {
                h.tag(b"BC");
                h.i32(*brightness);
                h.i32(*contrast);
            }
            KernelOp::Levels {
                master,
                red,
                green,
                blue,
            } => {
                h.tag(b"LV");
                for controls in [Some(*master), *red, *green, *blue] {
                    match controls {
                        Some(c) => {
                            h.bool(true);
                            h.bytes(&[c.in_black, c.in_white, c.out_black, c.out_white]);
                            h.f64(c.gamma);
                        }
                        None => h.bool(false),
                    }
                }
            }
            KernelOp::Invert => h.tag(b"IV"),
            KernelOp::HueSaturation {
                hue,
                saturation,
                lightness,
                colorize,
            } => {
                h.tag(b"HS");
                h.f64(*hue);
                h.f64(*saturation);
                h.f64(*lightness);
                h.bool(*colorize);
            }
            KernelOp::Clouds { seed, sky, cloud } => {
                h.tag(b"CL");
                h.u64(*seed);
                h.bytes(&[sky.r, sky.g, sky.b, sky.a]);
                h.bytes(&[cloud.r, cloud.g, cloud.b, cloud.a]);
            }
            KernelOp::DifferenceClouds { seed, sky, cloud } => {
                h.tag(b"DC");
                h.u64(*seed);
                h.bytes(&[sky.r, sky.g, sky.b, sky.a]);
                h.bytes(&[cloud.r, cloud.g, cloud.b, cloud.a]);
            }
            KernelOp::GaussianBlur { radius } => {
                h.tag(b"GB");
                h.f64(*radius);
            }
            KernelOp::BoxBlur { radius } => {
                h.tag(b"BB");
                h.u32(*radius);
            }
            KernelOp::UnsharpMask {
                radius,
                amount,
                threshold,
            } => {
                h.tag(b"UM");
                h.f64(*radius);
                h.u32(*amount);
                h.bytes(&[*threshold]);
            }
            KernelOp::Curves { knots } => {
                h.tag(b"CV");
                for channel in [
                    knots.rgb.as_slice(),
                    knots.red.as_slice(),
                    knots.green.as_slice(),
                    knots.blue.as_slice(),
                ] {
                    h.usize(channel.len());
                    for (x, y) in channel {
                        h.f64(*x);
                        h.f64(*y);
                    }
                }
            }
            KernelOp::AutoLevels => h.tag(b"AL"),
            KernelOp::ColorFill {
                fill,
                opacity,
                mode,
            } => {
                h.tag(b"CF");
                h.bytes(fill);
                h.u32(*opacity);
                h.u64((*mode) as u64);
            }
            KernelOp::MotionBlur {
                angle_degrees,
                distance,
            } => {
                h.tag(b"MB");
                h.f64(*angle_degrees);
                h.u32(*distance);
            }
            KernelOp::RadialBlur { amount, quality } => {
                h.tag(b"RB");
                h.u32(*amount);
                h.u32(*quality);
            }
            KernelOp::ZoomBlur { amount, quality } => {
                h.tag(b"ZB");
                h.u32(*amount);
                h.u32(*quality);
            }
            KernelOp::Sharpen => h.tag(b"SH"),
            KernelOp::SharpenMore => h.tag(b"SM"),
            KernelOp::ConvertToAlpha => h.tag(b"CA"),
            KernelOp::AddNoise {
                seed,
                amount,
                monochrome,
            } => {
                h.tag(b"AN");
                h.u64(*seed);
                h.u32(*amount);
                h.bool(*monochrome);
            }
            KernelOp::FindEdges => h.tag(b"FE"),
            KernelOp::DropShadow {
                color,
                angle_degrees,
                distance,
                blur,
                knockout,
            } => {
                h.tag(b"DS");
                h.bytes(&[color.r, color.g, color.b, color.a]);
                h.f64(*angle_degrees);
                h.u32(*distance);
                h.u32(*blur);
                h.bool(*knockout);
            }
            KernelOp::InnerShadow {
                color,
                angle_degrees,
                distance,
                blur,
            } => {
                h.tag(b"IS");
                h.bytes(&[color.r, color.g, color.b, color.a]);
                h.f64(*angle_degrees);
                h.u32(*distance);
                h.u32(*blur);
            }
            KernelOp::SolidShadow { color, show_object } => {
                h.tag(b"SS");
                h.bytes(&[color.r, color.g, color.b, color.a]);
                h.bool(*show_object);
            }
            KernelOp::OuterGlow {
                color,
                blur,
                edge_threshold,
            } => {
                h.tag(b"OG");
                h.bytes(&[color.r, color.g, color.b, color.a]);
                h.u32(*blur);
                h.u8(*edge_threshold);
            }
            KernelOp::InnerGlow {
                color,
                blur,
                edge_threshold,
            } => {
                h.tag(b"IG");
                h.bytes(&[color.r, color.g, color.b, color.a]);
                h.u32(*blur);
                h.u8(*edge_threshold);
            }
            KernelOp::Bevel {
                style,
                angle_degrees,
                width,
                contrast,
                depth,
                start,
                hilite,
                shadow,
                face,
            } => {
                h.tag(b"BV");
                h.u8(*style as u8);
                h.f64(*angle_degrees);
                h.u32(*width);
                h.u32(*contrast);
                h.u32(*depth);
                h.u32(*start);
                h.bytes(&[hilite.r, hilite.g, hilite.b, hilite.a]);
                h.bytes(&[shadow.r, shadow.g, shadow.b, shadow.a]);
                match face {
                    Some(face) => {
                        h.bool(true);
                        h.bytes(&[face.r, face.g, face.b, face.a]);
                    }
                    None => h.bool(false),
                }
            }
        }
    }
}

/// FNV-1a 64, fed through a tiny typed front-end so every hashed
/// quantity has one canonical byte encoding.
struct Hasher {
    state: u64,
}

impl Hasher {
    fn new(domain: &[u8]) -> Self {
        let mut hasher = Self {
            state: 0xcbf2_9ce4_8422_2325,
        };
        hasher.bytes(domain);
        hasher
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state = (self.state ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn tag(&mut self, tag: &[u8; 2]) {
        self.bytes(tag);
    }

    fn u16_pair(&mut self, (width, height): (u16, u16)) {
        self.bytes(&width.to_be_bytes());
        self.bytes(&height.to_be_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.bytes(&(value as u64).to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }

    fn i32(&mut self, value: i32) {
        self.bytes(&value.to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes(&value.to_le_bytes());
    }

    fn f64(&mut self, value: f64) {
        self.bytes(&value.to_bits().to_le_bytes());
    }

    fn bool(&mut self, value: bool) {
        self.bytes(&[u8::from(value)]);
    }

    fn opt_bool(&mut self, value: Option<bool>) {
        match value {
            Some(inner) => {
                self.bytes(&[2]);
                self.bool(inner);
            }
            None => self.bytes(&[3]),
        }
    }

    fn opt_xy(&mut self, point: Option<(f64, f64)>) {
        match point {
            Some((x, y)) => {
                self.bytes(&[1]);
                self.f64(x);
                self.f64(y);
            }
            None => self.bytes(&[0]),
        }
    }

    fn finish(self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::effects::plan;
    use doc::{Contour, Effect, PathObject, PathPoint};

    fn path_object(contours: Vec<Contour>) -> PathObject {
        PathObject::neutral(contours)
    }

    fn point(x: f64, y: f64) -> PathPoint {
        PathPoint {
            anchor: geom::Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        }
    }

    fn square() -> Contour {
        Contour {
            closed: true,
            point_count: 4,
            points: vec![
                point(1.0, 1.0),
                point(7.0, 1.0),
                point(7.0, 7.0),
                point(1.0, 7.0),
            ],
        }
    }

    fn invert_plan() -> Plan {
        plan(&[Effect {
            category: String::new(),
            internal_name: "invert".to_owned(),
            properties: Vec::new(),
        }])
    }

    #[test]
    fn identical_content_hashes_equal() {
        let canvas = (16u16, 16u16);
        let plan = invert_plan();
        let a = hash_path(&path_object(vec![square()]), canvas, &plan);
        let b = hash_path(&path_object(vec![square()]), canvas, &plan);
        assert_eq!(a, b, "same content → same key (cache hit)");
    }

    #[test]
    fn geometry_change_changes_the_key() {
        let canvas = (16u16, 16u16);
        let plan = invert_plan();
        let mut moved = square();
        moved.points[0].anchor = geom::Point::new(1.5, 1.0);
        let a = hash_path(&path_object(vec![square()]), canvas, &plan);
        let b = hash_path(&path_object(vec![moved]), canvas, &plan);
        assert_ne!(a, b, "edited geometry must not hit a stale entry");
    }

    #[test]
    fn paint_and_plan_changes_change_the_key() {
        let canvas = (16u16, 16u16);
        let plan = invert_plan();
        let base = path_object(vec![square()]);

        let mut recolored = base.clone();
        recolored.paint = Some(doc::PaintStyle {
            stroke_raw: 0,
            fill_raw: 0xFF_00_00_FF,
            fill: None,
        });
        assert_ne!(
            hash_path(&base, canvas, &plan),
            hash_path(&recolored, canvas, &plan),
            "paint feeds the base raster"
        );

        let mut even_odd = base.clone();
        even_odd.even_odd = true;
        assert_ne!(
            hash_path(&base, canvas, &plan),
            hash_path(&even_odd, canvas, &plan)
        );

        let mut brightness = plan.clone();
        brightness.ops.push(crate::effects::KernelOp::Invert);
        assert_ne!(
            hash_path(&base, canvas, &plan),
            hash_path(&base, canvas, &brightness),
            "plan is part of the key"
        );

        assert_ne!(
            hash_path(&base, canvas, &plan),
            hash_path(&base, (32, 16), &plan),
            "canvas size is part of the key"
        );
    }

    #[test]
    fn fill_model_changes_change_the_key() {
        // A gradient/pattern fill renders differently from a solid of
        // the same `fill_raw`, and the placement handles steer the
        // ramp — the model below the raw colour rides the key.
        let canvas = (16u16, 16u16);
        let plan = invert_plan();
        let mut base = path_object(vec![square()]);
        base.paint = Some(doc::PaintStyle {
            stroke_raw: 0,
            fill_raw: 0xFF_18_1F_2B,
            fill: None,
        });

        let mut gradient = base.clone();
        gradient.paint.as_mut().expect("paint").fill = Some(doc::Fill {
            shape: doc::FillShape::Linear,
            shape_extended: false,
            texture_blend: 0,
            feather: 0,
            stamping: None,
            hard_edge: false,
            gradient: Some(doc::Gradient {
                vertices: Vec::new(),
                rgb: vec![doc::GradientStop {
                    position: 0.0,
                    color_raw: 0xFF_00_00_FF,
                }],
                alpha: Vec::new(),
            }),
            dither: None,
            pattern: None,
        });
        assert_ne!(
            hash_path(&base, canvas, &plan),
            hash_path(&gradient, canvas, &plan),
            "a gradient fill is not the solid it shares fill_raw with"
        );

        let mut placed = gradient.clone();
        placed.fill_start = Some(geom::Point::new(0.0, 0.0));
        placed.fill_end = Some(geom::Point::new(8.0, 0.0));
        assert_ne!(
            hash_path(&gradient, canvas, &plan),
            hash_path(&placed, canvas, &plan),
            "fill-vector placement steers the ramp"
        );

        let mut pattern = base.clone();
        pattern.paint.as_mut().expect("paint").fill = Some(doc::Fill {
            shape: doc::FillShape::Pattern,
            shape_extended: true,
            texture_blend: 0,
            feather: 0,
            stamping: None,
            hard_edge: false,
            gradient: None,
            dither: None,
            pattern: Some(doc::PatternTile {
                name: None,
                pixels: (128, 128),
                tile_size: 128,
                tiles: vec![doc::TileRef::Tile(3)],
            }),
        });
        assert_ne!(
            hash_path(&base, canvas, &plan),
            hash_path(&pattern, canvas, &plan)
        );
        let mut matte = pattern.clone();
        if let Some(fill) = matte.paint.as_mut().expect("paint").fill.as_mut() {
            fill.pattern.as_mut().expect("pattern").tiles =
                vec![doc::TileRef::Matte(0xFF_00_FF_00)];
        }
        assert_ne!(
            hash_path(&pattern, canvas, &plan),
            hash_path(&matte, canvas, &plan),
            "tile references are content"
        );
    }

    #[test]
    fn blend_and_opacity_do_not_change_the_key() {
        // They wrap the filtered image in the parent layer — the
        // cached bytes never see them.
        let canvas = (16u16, 16u16);
        let plan = invert_plan();
        let mut translucent = path_object(vec![square()]);
        translucent.opacity = 500;
        translucent.blend = 5;
        assert_eq!(
            hash_path(&path_object(vec![square()]), canvas, &plan),
            hash_path(&translucent, canvas, &plan)
        );
    }

    #[test]
    fn pixel_keys_separate_content_and_plan() {
        let canvas = (8u16, 8u16);
        let plan = invert_plan();
        let base = vec![1u8; (canvas.0 * canvas.1 * 4) as usize];
        let other = vec![2u8; base.len()];
        assert_eq!(
            hash_pixels(&base, canvas, &plan),
            hash_pixels(&base, canvas, &plan)
        );
        assert_ne!(
            hash_pixels(&base, canvas, &plan),
            hash_pixels(&other, canvas, &plan)
        );
    }

    #[test]
    fn cache_insert_get_and_limit() {
        let mut cache = EffectCache::new();
        assert!(cache.is_empty());
        cache.insert(1, 4, 4, vec![0; 64]);
        assert_eq!(cache.get(1).map(|entry| entry.width), Some(4));
        assert_eq!(cache.get(2), None);
        for key in 0..ENTRY_LIMIT + 1 {
            cache.insert(key as u64, 1, 1, vec![0; 4]);
        }
        assert!(cache.len() <= ENTRY_LIMIT, "the limit keeps memory bounded");
        cache.clear();
        assert!(cache.is_empty());
    }
}
