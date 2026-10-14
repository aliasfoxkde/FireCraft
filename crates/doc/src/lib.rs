//! FireCraft typed document model (`doc`, L1).
//!
//! The model mirrors Fireworks document semantics: a document (`MKBv`)
//! holds pages (`PDCv`), pages hold layer lists, layers hold one cell
//! per state ("frame"), and cells hold object ids into a flat registry.
//! Objects are the Fireworks element union: paths, text, groups, images,
//! slices, symbol instances.
//!
//! The node-tree mapping lives in [`fwmap`]: read (`mkTS` tree →
//! `Document` + unknown bag) and write (the reverse), losslessly —
//! anything the model does not yet interpret is preserved verbatim and
//! re-emitted. `fwpng` itself stays a standalone crate; this crate
//! depends on it (L1 over L0), not the other way around.

pub mod fwmap;
pub mod library;
pub mod model;
pub mod registry;

pub use model::{
    Canvas, Cell, Contour, Dither, Document, DocumentInfo, Effect, EffectProperty, EffectValue,
    ExportOptions, Fill, FillShape, Gradient, GradientStop, Grid, GroupObject, Guides, ImageObject,
    Layer, LayerList, NineSliceGuides, Page, PageInfo, PaintStyle, PathObject, PathPoint,
    PatternTile, PrimitiveSource, SavedSelection, SelectionMask, SharedLayerDef, SharedLayerLink,
    SliceBackground, SliceBehavior, SliceObject, StyleEntry, StyleText, SymbolAnimation,
    SymbolDefinition, SymbolInstanceObject, SymbolType, TextObject, TextOnPath, TextRun, TextStyle,
    TileBank, TileRef,
};
pub use registry::{ObjectData, ObjectId, Registry};
// `EffectValue::Raw` exposes the node type; re-export it so consumers
// of the model (engine commands, tests) can name it without a direct
// fwpng dependency edge.
pub use fwpng::nodetree::{Node, NodeValue};
