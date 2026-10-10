//! Flat, document-wide object registry: generational slots keyed by
//! [`ObjectId`]. Structural lists (page layers, group children) hold ids;
//! z-order is list order; parent/child edges are derivable by walking.

/// A stable handle to one object in the document registry. Survives
/// removals of *other* objects; using a stale id (after its slot was
/// freed and reused) fails by generation mismatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectId {
    index: u32,
    generation: u32,
}

impl ObjectId {
    /// Index of the underlying slot.
    #[must_use]
    pub fn index(self) -> usize {
        self.index as usize
    }

    /// Slot generation (incremented on each reuse).
    #[must_use]
    pub fn generation(self) -> u32 {
        self.generation
    }

    /// A deliberately stale id, for exercising the dangling-id path.
    #[cfg(test)]
    pub(crate) fn dangling(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Slot {
    generation: u32,
    object: Option<ObjectData>,
}

/// The registry itself. Dense enough for iteration, stable enough for
/// edits: removal frees the slot for reuse and bumps its generation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Registry {
    slots: Vec<Slot>,
    free: Vec<u32>,
}

impl Registry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts an object, returning its id.
    pub fn insert(&mut self, object: ObjectData) -> ObjectId {
        let index = match self.free.pop() {
            Some(free) => free as usize,
            None => {
                self.slots.push(Slot {
                    generation: 0,
                    object: None,
                });
                self.slots.len() - 1
            }
        };
        let slot = &mut self.slots[index];
        slot.object = Some(object);
        ObjectId {
            index: index as u32,
            generation: slot.generation,
        }
    }

    /// Removes an object; the slot is reusable and any existing id for
    /// it becomes stale.
    pub fn remove(&mut self, id: ObjectId) -> Option<ObjectData> {
        let slot = self.slots.get_mut(id.index())?;
        if slot.generation != id.generation() {
            return None;
        }
        let removed = slot.object.take()?;
        slot.generation += 1;
        self.free.push(id.index);
        Some(removed)
    }

    /// The object behind `id`, or `None` when stale.
    #[must_use]
    pub fn get(&self, id: ObjectId) -> Option<&ObjectData> {
        let slot = self.slots.get(id.index())?;
        if slot.generation != id.generation() {
            return None;
        }
        slot.object.as_ref()
    }

    /// Mutable access behind `id`, or `None` when stale.
    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut ObjectData> {
        let slot = self.slots.get_mut(id.index())?;
        if slot.generation != id.generation() {
            return None;
        }
        slot.object.as_mut()
    }

    /// Number of live objects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len() - self.free.len()
    }

    /// `true` when no live objects.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Live `(id, object)` pairs, in slot order.
    pub fn iter(&self) -> impl Iterator<Item = (ObjectId, &ObjectData)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            let object = slot.object.as_ref()?;
            Some((
                ObjectId {
                    index: index as u32,
                    generation: slot.generation,
                },
                object,
            ))
        })
    }

    /// Mutable `(id, object)` pairs, in slot order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ObjectId, &mut ObjectData)> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| {
                let object = slot.object.as_mut()?;
                Some((
                    ObjectId {
                        index: index as u32,
                        generation: slot.generation,
                    },
                    object,
                ))
            })
    }
}

/// One object in the document. Kinds mirror the Fireworks element union
/// (`ELMv`); each variant maps to exactly one node type.
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectData {
    /// `PTHv` — vector path, with optional primitive source.
    Path(PathObject),
    /// `TXTv` — text object (runs preserved verbatim; deep engine M6).
    Text(TextObject),
    /// `GRPv` — group (including mask-structured groups).
    Group(GroupObject),
    /// `IMGv` — raster image backed by `mkBT` tile references.
    Image(ImageObject),
    /// `URLv` — slice / hotspot.
    Slice(SliceObject),
    /// `ALSv` — symbol instance referencing a [`SymbolDefinition`].
    SymbolInstance(SymbolInstanceObject),
}

use crate::model::{
    GroupObject, ImageObject, PathObject, SliceObject, SymbolInstanceObject, TextObject,
};

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::model::tests::sample_path;

    #[test]
    fn insert_get_remove_round_trip() {
        let mut registry = Registry::new();
        let id = registry.insert(sample_path());
        assert_eq!(registry.len(), 1);
        assert!(matches!(registry.get(id), Some(ObjectData::Path(_))));
        let removed = registry.remove(id).expect("live id");
        assert!(matches!(removed, ObjectData::Path(_)));
        assert!(registry.is_empty());
        assert!(registry.get(id).is_none(), "id is stale after removal");
    }

    #[test]
    fn generation_detects_slot_reuse() {
        let mut registry = Registry::new();
        let first = registry.insert(sample_path());
        registry.remove(first).expect("remove");
        let second = registry.insert(sample_path());
        assert_eq!(first.index(), second.index(), "slot is reused");
        assert_ne!(first.generation(), second.generation());
        assert!(registry.get(first).is_none(), "old id must be stale");
        assert!(registry.get(second).is_some());
    }

    #[test]
    fn iter_reports_only_live_slots() {
        let mut registry = Registry::new();
        let a = registry.insert(sample_path());
        let b = registry.insert(sample_path());
        registry.remove(a).expect("remove");
        let live: Vec<ObjectId> = registry.iter().map(|(id, _)| id).collect();
        assert_eq!(live, vec![b]);
    }
}
