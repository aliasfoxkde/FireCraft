# FireCraft tutorials

These short tutorials are based on the current FireCraft controls. Names and behaviors can change during the active GUI integration; compare them with the in-app Help and current [feature list](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/FEATURES.md).

## Create and save a first design

1. Start FireCraft and choose **File → New**. Use the page size and resolution controls to set the canvas.
2. Pick a tool from the categorized Tools rail. Hover over an icon to read its name and shortcut; press a tool-group shortcut repeatedly to cycle related tools.
3. Draw a shape, then use the Properties panel to set its fill, stroke and geometry. Use the Layers panel to rename, reorder, hide or lock content.
4. Press **Ctrl+S** to save an editable Fireworks PNG (`.fw.png`). Choose **File → Export** for a flattened image or web assets.

## Navigate the workspace

- Use **Ctrl+P** to search the command palette; use the arrow keys to choose a result and Enter to run it. It currently uses fuzzy subsequence matching on command IDs; descriptive/user-language aliases are planned.
- Press **F1** or open **Help → Keyboard Shortcuts** to see the active keymap. Choose another preset under **Edit → Preferences → Keyboard**.
- Use **Tab** to hide and restore panels when the canvas needs more room. Focus behavior is an active acceptance check for the experiment integration.
- Collapse a panel from its header control. Dragging, floating, docking and snapping behavior is still undergoing interactive validation; report issues with the reproduction steps.

## Work with vector and bitmap content

- Select vector paths with Pointer; use Subselect to edit path points and handles.
- For raster work, import an image, use a bitmap selection tool, then apply paint or retouch tools. Use masks or live effects when you need reversible edits.
- Copy and paste within a document with **Ctrl+C** and **Ctrl+V**. System clipboard interoperability is under review; if content does not paste, include source application, content type and OS in a bug report.

## Report a bug or request a feature

Use [New Issue](https://github.com/aliasfoxkde/FireCraft/issues/new/choose). Pick the bug, feature request or change-request template. Include the FireCraft version, operating system, steps to reproduce, expected result and observed result. Remove private or sensitive content from screenshots and sample documents before attaching them.

## More references

- [Keyboard shortcuts](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/SHORTCUTS.md)
- [Feature list](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/FEATURES.md)
- [Public roadmap](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PUBLIC-ROADMAP.md)
- [CS6 parity matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md)
