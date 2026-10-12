//! Sidecar bag for nodes the model does not yet interpret.
//!
//! Reading bags every child outside the mapped spine of its container,
//! verbatim; writing splices the bagged nodes back at their recorded
//! child indices. Round-trips therefore never lose content the model
//! cannot express yet (see `docs/planning/firecraft/M3-doc-model.md`,
//! decision 3).

use fwpng::nodetree::Node;

/// One preserved node: the container path it was found under, its child
/// index there, and the node itself.
#[derive(Clone, Debug)]
pub struct UnknownEntry {
    path: String,
    index: usize,
    node: Node,
}

/// The bag itself. `take_under` is the write-side consume primitive;
/// `iter` serves inspection and tests.
#[derive(Clone, Debug, Default)]
pub struct UnknownBag {
    entries: Vec<UnknownEntry>,
}

impl UnknownBag {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `node` as the `index`th child of the container at `path`.
    pub fn insert(&mut self, path: &str, index: usize, node: Node) {
        self.entries.push(UnknownEntry {
            path: path.to_owned(),
            index,
            node,
        });
    }

    /// Removes and returns every entry bagged under `path`, ordered by
    /// child index — the exact input [`crate::fwmap::writer`] needs to
    /// splice them back into place.
    pub fn take_under(&mut self, path: &str) -> Vec<(usize, Node)> {
        let mut taken = Vec::new();
        let mut kept = Vec::new();
        for entry in self.entries.drain(..) {
            if entry.path == path {
                taken.push((entry.index, entry.node));
            } else {
                kept.push(entry);
            }
        }
        taken.sort_by_key(|(index, _)| *index);
        self.entries = kept;
        taken
    }

    /// Drops every entry recorded at or below `path`. Used when a
    /// container is later preserved whole: its partial bags are subsumed
    /// by the verbatim node and must not resurface as orphans on write.
    pub fn purge_under(&mut self, path: &str) {
        let prefix = format!("{path}/");
        self.entries
            .retain(|entry| entry.path != path && !entry.path.starts_with(&prefix));
    }

    /// Drops every entry whose recorded path starts with `prefix`.
    /// Used when a model-driven list changes length: per-index bags of
    /// entries past the new count have no host slot (the `FNMv` list
    /// shrinks with the recorded durations — the model wins on count).
    pub fn purge_prefix(&mut self, prefix: &str) {
        self.entries.retain(|entry| !entry.path.starts_with(prefix));
    }

    /// Number of preserved nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when nothing is preserved.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterates `(path, child index, node)` for inspection and tests.
    pub fn iter(&self) -> impl Iterator<Item = (&str, usize, &Node)> {
        self.entries
            .iter()
            .map(|entry| (entry.path.as_str(), entry.index, &entry.node))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    fn leaf(fourcc: &[u8; 4], value: i64) -> Node {
        Node {
            fourcc: *fourcc,
            value: NodeValue::Int(value),
        }
    }

    use fwpng::nodetree::{Node, NodeValue};

    #[test]
    fn take_under_returns_only_that_container_sorted_by_index() {
        let mut bag = UnknownBag::new();
        bag.insert("/MKBv", 3, leaf(b"PLLv", 1));
        bag.insert("/MKBv/LYLv/LAYv", 0, leaf(b"LSAv", 2));
        bag.insert("/MKBv", 1, leaf(b"FLDv", 3));

        let taken = bag.take_under("/MKBv");
        let indices: Vec<usize> = taken.iter().map(|(index, _)| *index).collect();
        assert_eq!(indices, vec![1, 3], "sorted by child index");

        assert_eq!(bag.len(), 1, "the LAYv entry stays");
        let (path, index, _) = bag.iter().next().unwrap();
        assert_eq!(path, "/MKBv/LYLv/LAYv");
        assert_eq!(index, 0);
        assert!(bag.take_under("/MKBv").is_empty(), "consume is removal");
    }

    #[test]
    fn empty_bag_is_empty() {
        let mut bag = UnknownBag::new();
        assert!(bag.is_empty());
        assert_eq!(bag.len(), 0);
        assert!(bag.take_under("/anywhere").is_empty());
    }
}
