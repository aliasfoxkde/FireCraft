//! Document summaries (`firecraft-cli info`): the typed model as JSON.

use doc::{Document, ObjectData};

/// Builds the JSON summary for one document. Derived counts only —
/// nothing here invents numbers (the M4 lesson about marketing
/// literals applies to tooling too).
#[must_use]
pub fn summarize(document: &Document) -> serde_json::Value {
    let mut kinds = KindCounts::default();
    for (_, object) in document.registry.iter() {
        kinds.count(object);
    }
    serde_json::json!({
        "format": {
            "version": document.info.version,
            "revision": document.info.revision,
        },
        "frame_count": document.info.frame_count,
        "canvas": canvas(&document.info.canvas),
        "pages": document.pages.iter().map(page).collect::<Vec<_>>(),
        "symbols": document.symbols.len(),
        "objects": {
            "live": document.registry.len(),
            "kinds": kinds.value(),
        },
    })
}

fn canvas(canvas: &doc::Canvas) -> serde_json::Value {
    serde_json::json!({
        "x": canvas.x,
        "y": canvas.y,
        "width": canvas.width,
        "height": canvas.height,
        "resolution": canvas.resolution,
    })
}

fn page(page: &doc::Page) -> serde_json::Value {
    serde_json::json!({
        "name": page.name,
        "is_master": page.is_master,
        "canvas": canvas(&page.info.canvas),
        "frame_count": page.info.frame_count,
        "layers": page.layers.layers.iter().map(layer).collect::<Vec<_>>(),
    })
}

fn layer(layer: &doc::Layer) -> serde_json::Value {
    let elements: usize = layer.cells.iter().map(|cell| cell.elements.len()).sum();
    serde_json::json!({
        "name": layer.name,
        "display": layer.display,
        "is_background": layer.is_background,
        "cells": layer.cells.len(),
        "element_refs": elements,
    })
}

#[derive(Default)]
struct KindCounts {
    paths: usize,
    texts: usize,
    groups: usize,
    images: usize,
    slices: usize,
    symbol_instances: usize,
}

impl KindCounts {
    fn count(&mut self, object: &ObjectData) {
        match object {
            ObjectData::Path(_) => self.paths += 1,
            ObjectData::Text(_) => self.texts += 1,
            ObjectData::Group(_) => self.groups += 1,
            ObjectData::Image(_) => self.images += 1,
            ObjectData::Slice(_) => self.slices += 1,
            ObjectData::SymbolInstance(_) => self.symbol_instances += 1,
        }
    }

    fn value(&self) -> serde_json::Value {
        serde_json::json!({
            "paths": self.paths,
            "texts": self.texts,
            "groups": self.groups,
            "images": self.images,
            "slices": self.slices,
            "symbol_instances": self.symbol_instances,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use engine::fuzzdrive::{FuzzRng, random_document};

    #[test]
    fn summary_counts_match_the_document() {
        let mut rng = FuzzRng::new(4);
        let document = random_document(&mut rng);
        let summary = summarize(&document);
        assert_eq!(
            summary["pages"].as_array().map(Vec::len),
            Some(document.pages.len())
        );
        assert_eq!(
            summary["objects"]["live"].as_u64(),
            Some(document.registry.len() as u64)
        );
        let paths = document
            .registry
            .iter()
            .filter(|(_, o)| matches!(o, ObjectData::Path(_)))
            .count() as u64;
        assert_eq!(summary["objects"]["kinds"]["paths"].as_u64(), Some(paths));
        assert_eq!(
            summary["frame_count"].as_i64(),
            Some(document.info.frame_count)
        );
    }
}
