//! Flat, document-wide object registry: generational slots keyed by
//! [`ObjectId`]. Structural lists (page layers, group children) hold ids;
//! z-order is list order; parent/child edges are derivable by walking.

/// A stable handle to one object in the document registry. Survives
/// removals of *other* objects; using a stale id (after its slot was
/// freed and reused) fails by generation mismatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Builds an id from its parts. Payloads already admit forged ids
    /// through serde, so this adds no new risk — it exists so callers
    /// outside a registry can name stale slots (tests, dangling-id
    /// probes).
    #[must_use]
    pub fn from_parts(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// A deliberately stale id, for exercising the dangling-id path.
    #[cfg(test)]
    pub(crate) fn dangling(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Slot {
    generation: u32,
    object: Option<ObjectData>,
}

/// The registry itself. Dense enough for iteration, stable enough for
/// edits: removal frees the slot for reuse and bumps its generation.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Puts `object` back into its exact slot with its exact
    /// generation, undoing a [`Registry::remove`] — every id handed
    /// out before the removal becomes live again. The slot must be
    /// empty and `id` must name it.
    pub fn restore(&mut self, id: ObjectId, object: ObjectData) -> Option<()> {
        let free_pos = self.free.iter().position(|&free| free == id.index)?;
        let slot = self.slots.get_mut(id.index())?;
        if slot.object.is_some() {
            return None;
        }
        self.free.swap_remove(free_pos);
        slot.generation = id.generation();
        slot.object = Some(object);
        Some(())
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

    /// Undoes an [`Registry::insert`]: the slot returns to its exact
    /// pre-insert state. A generation-0 slot was freshly allocated by
    /// the insert (it never existed before) and is popped outright; a
    /// recycled slot keeps its generation and returns to the free
    /// list — precisely the state `insert` found it in, so documents
    /// compare equal across create→undo. `None` when `id` is not live.
    pub fn retract_insert(&mut self, id: ObjectId) -> Option<()> {
        let slot = self.slots.get(id.index())?;
        if slot.generation != id.generation() || slot.object.is_none() {
            return None;
        }
        if id.generation() == 0 {
            self.slots.pop();
        } else {
            let slot = self.slots.get_mut(id.index())?;
            slot.object = None;
            self.free.push(id.index);
        }
        Some(())
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    #[test]
    fn retract_of_a_trailing_insert_pops_the_slot() {
        let mut registry = Registry::new();
        let before = registry.clone();
        let id = registry.insert(sample_path());
        registry.retract_insert(id).expect("live id");
        assert_eq!(registry, before, "trailing slot is popped outright");
        assert!(registry.retract_insert(id).is_none(), "already retracted");
    }

    #[test]
    fn retract_of_a_recycled_insert_restores_the_free_slot() {
        let mut registry = Registry::new();
        let first = registry.insert(sample_path());
        registry.remove(first).expect("remove");
        let before = registry.clone();
        let second = registry.insert(sample_path());
        registry.retract_insert(second).expect("live id");
        assert_eq!(
            registry, before,
            "recycled slot returns to its exact pre-insert state"
        );
        // The retracted id is stale; the original id revives normally.
        assert!(registry.get(second).is_none());
        registry.restore(first, sample_path()).expect("restorable");
        assert!(registry.get(first).is_some());
    }
}
