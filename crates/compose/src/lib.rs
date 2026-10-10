//! FireCraft deterministic CPU compositor (L3).
//!
//! Renders a [`Document`] page in one state (frame) to premultiplied
//! sRGB RGBA8 pixels through `vello_cpu` (decision 0002). Same input →
//! byte-identical output: single-threaded, no clocks, no randomness —
//! this crate is the oracle every later renderer (GPU, export preview)
//! must match.
//!
//! M4 surface (see `docs/planning/firecraft/M4-compositor.md`): paths
//! with solid paints and groups, with per-object opacity layers and
//! blend modes. Images, slices, text and symbol instances render as
//! nothing; gradient/texture paints ride the document's unknown bag.
//! Blend values beyond the rasterizer's 16 W3C mix modes fall back to
//! Normal (recorded limitation, ADR 0002).

mod convert;
mod render;

pub use render::{ComposeError, Render, render};
