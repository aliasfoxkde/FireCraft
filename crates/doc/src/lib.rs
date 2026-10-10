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
pub mod model;
pub mod registry;

pub use model::{
    Canvas, Cell, Contour, Document, DocumentInfo, Grid, GroupObject, Guides, ImageObject, Layer,
    LayerList, Page, PageInfo, PathObject, PathPoint, PrimitiveSource, SliceObject,
    SymbolDefinition, SymbolInstanceObject, TextObject, TileRef,
};
pub use registry::{ObjectData, ObjectId, Registry};
