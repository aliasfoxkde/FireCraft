# Documents and Formats

## Native document

FireCraft's editable format is Fireworks PNG (`.fw.png`): a PNG container with private chunks for the node tree, raster tile banks, previews, and compatibility data. FireCraft preserves unknown structures where possible so documents can remain editable across partial format understanding.

Read the [full node dictionary](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/research/fireworks-png-nodes.md) and [format plan](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PLANNING.md).

## Import and export

Raster, vector, layered, and web-export formats have different limits and options. See the [feature matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/FEATURES.md) and [Usage](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/USAGE.md) for the current supported extensions and command examples.

## Fidelity and limitations

The [generated parity report](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/planning/generated/parity.md) and [decision records](https://github.com/aliasfoxkde/FireCraft/tree/main/docs/decisions) document format behavior, known refusals, and evidence-gated areas. Do not treat an unsupported feature as a valid conversion unless the CLI or UI reports it as supported.
