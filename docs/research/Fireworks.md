# Fireworks document structure

This note introduces the node-tree vocabulary used by the detailed [Fireworks Node Types](fireworks-png-nodes.md) dictionary.

## Native document shape

A Fireworks PNG (`.fw.png`) is a PNG container with private chunks that preserve editable document data alongside the rendered preview. The `mkTS` chunk carries the node tree: typed values and nested nodes describe pages, layers, objects, paths, text, symbols, and export settings. Raster content is stored in tile-bank chunks such as `mkBT`; nodes refer to tile identifiers rather than embedding full images inline. Unknown chunks and nodes matter because a lossless editor must preserve data it does not yet understand.

The tree is not just a scene graph. It also carries application state and compatibility details that affect round-tripping, editing, and export. FireCraft's reader/writer maps recognized fields into a typed document model and retains unknown material for re-emission.

## How to read the dictionary

- A node heading identifies a node type or on-disk field.
- Child markers describe observed multiplicity: `*` means zero or more, `+` one or more, `?` optional, and `(x2)` exactly two.
- Values are observationally documented. Corpus files take precedence when they contradict a note.
- The dictionary records compatibility behavior, not permission to copy Adobe source code or artwork.

## Deeper references

- [Full node dictionary](fireworks-png-nodes.md)
- [Format and architecture plan](../PLANNING.md)
- [Research evidence and attribution](../ATTRIBUTION.md)
- [Document reader/writer](../../crates/fwpng/)
