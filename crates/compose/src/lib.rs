//! `FireCraft` deterministic CPU compositor (L3).
//!
//! Renders a [`Document`] page in one state (frame) to premultiplied
//! sRGB RGBA8 pixels through `vello_cpu` (decision 0002). Same input →
//! byte-identical output: single-threaded, no clocks, no randomness —
//! this crate is the oracle every later renderer (GPU, export preview)
//! must match.
//!
//! M4 surface (see `docs/planning/firecraft/M4-compositor.md`): paths
//! with solid paints and groups, with per-object opacity layers and
//! blend modes. M7 adds live effects: an object's `EFLv` stack is
//! interpreted to `filters` kernels ([`effects`]) and applied to a
//! full-canvas offscreen raster of the object, content-cached through
//! [`EffectCache`] (decision 0005). Images, slices, text and symbol
//! instances render as nothing; gradient/texture paints ride the
//! document's unknown bag. Blend values beyond the rasterizer's 16 W3C
//! mix modes fall back to Normal (recorded limitation, ADR 0002).

mod cache;
mod convert;
#[allow(clippy::module_name_repetitions)]
mod effects;
mod render;

pub use cache::EffectCache;
pub use convert::{abgr, blend_mode, blend_value};
pub use effects::{FILTER_CATALOG, KernelOp, Plan, RECOGNIZED_GUIDS, apply_stack, plan};
// The kernel surface type, re-exported so consumers of the effect
// interpreter (the engine's bake commands) need no second edge.
pub use filters::Surface;
pub use render::{
    ComposeError, Render, matte_premultiplied, render, render_subset, render_transparent,
    render_with_cache, straight_alpha,
};
// The mix vocabulary `blend_mode` speaks, re-exported so export
// surfaces can match on it without a second vello dependency edge.
pub use vello_cpu::peniko::Mix;
