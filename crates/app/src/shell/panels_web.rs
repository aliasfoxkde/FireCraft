//! The web-object panels of the CS6 right dock: Auto Shapes, URL, and
//! Behaviors.
//!
//! CS6 groups these three with the Properties panel: the Auto Shapes
//! library (the shape gallery), the URL panel (the selected object's
//! hyperlink and alt text), and the Behaviors panel (the selected
//! slice's interaction behaviors). Each panel here drives the engine
//! command that owns the data — `object.createPath` / `autoshape.*` for
//! the shapes, `slice.setLink` for the URL, `slice.addBehavior` /
//! `slice.removeBehavior` for the behaviors — so no panel invents
//! model state.

use super::*;
use crate::editor::AutoShape;

/// The size, in world units, of a shape inserted from the Auto Shapes
/// panel.
const INSERT_SIZE: f64 = 120.0;

/// The behavior kind CS6's popup menu carries (`BETi` 9 — the only
/// kind the node dictionary documents).
const POPUP_MENU_BEHAVIOR: i64 = 9;

/// The behavior kind the web export reads as "swap image".
const SWAP_IMAGE_BEHAVIOR: i64 = 2;

/// The export-facing key a swap-image behavior's target rides on
/// (decision 0017).
const SWAP_TARGET_KEY: &str = "swapTarget";

/// What the Auto Shapes panel can insert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ShapeKind {
    /// A primitive path (`object.createPath`): the rectangle and the
    /// ellipse CS6's gallery leads with.
    Primitive(&'static str),
    /// One of the Auto Shape tools.
    Auto(AutoShape),
}

/// The panel's roster, in CS6's gallery order: the two primitives,
/// then the Auto Shape tools.
fn shape_roster() -> Vec<(&'static str, ShapeKind)> {
    let mut roster = vec![
        ("Rectangle", ShapeKind::Primitive("rect")),
        ("Ellipse", ShapeKind::Primitive("ellipse")),
    ];
    roster.extend(
        AutoShape::ALL
            .into_iter()
            .map(|shape| (shape.label(), ShapeKind::Auto(shape))),
    );
    roster
}

/// The label of one behavior kind. Only the kinds this build
/// documents are named; the rest show their raw number.
fn behavior_label(kind: i64) -> String {
    match kind {
        POPUP_MENU_BEHAVIOR => "Popup menu".to_owned(),
        SWAP_IMAGE_BEHAVIOR => "Swap image".to_owned(),
        other => format!("Behavior {other}"),
    }
}

/// The link editor's edit-in-progress state: the object it belongs
/// to, the two fields it edits, and the stored values it was seeded
/// from.
///
/// The draft is shared by the Window ▸ URL panel and the Properties
/// inspector, so the two surfaces can never disagree about a slice's
/// link. The seed values are what make that safe: when the document
/// changes underneath the draft — the inspector applied a new link —
/// the stored value no longer matches the seed and the draft follows
/// it.
pub(crate) struct UrlDraft {
    /// The slice or hotspot the draft edits.
    pub(crate) target: doc::ObjectId,
    /// The anchor URL being edited.
    pub(crate) link: String,
    /// The alt text being edited.
    pub(crate) alt: String,
    /// The stored link the draft was seeded from.
    pub(crate) source_link: Option<String>,
    /// The stored alt text the draft was seeded from.
    pub(crate) source_alt: Option<String>,
}

/// A snapshot of the selected web object — the panel reads the
/// document read-only, so the fields it shows are copied out before
/// any command runs.
pub(super) struct WebSnapshot {
    /// The slice or hotspot.
    pub(super) id: doc::ObjectId,
    /// Its display name.
    pub(super) name: String,
    /// Its anchor URL.
    pub(super) link: Option<String>,
    /// Its alt text.
    pub(super) alt: Option<String>,
    /// Its behavior kinds, in order.
    pub(super) behaviors: Vec<i64>,
}

impl Shell {
    /// Window ▸ Auto Shapes: the CS6 shape gallery. CS6 drags a shape
    /// from the panel onto the canvas; a click here inserts the same
    /// shape at the canvas center.
    pub(super) fn auto_shapes_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Click a shape to insert it at the canvas center.");
        ui.separator();
        let roster = shape_roster();
        egui::Grid::new("auto_shapes")
            .num_columns(2)
            .show(ui, |ui| {
                for (label, kind) in roster {
                    if ui.button(label).clicked() {
                        self.insert_shape(kind, label);
                    }
                    ui.end_row();
                }
            });
    }

    /// Inserts the panel's shape at the canvas center.
    pub(super) fn insert_shape(&mut self, kind: ShapeKind, label: &str) {
        let (x, y, size) = self.insert_box();
        let page = self.editor.page;
        let layer = self.editor.active_layer();
        let cell = self.editor.frame;
        let inserted = match kind {
            ShapeKind::Primitive(primitive) => self.run_command(
                "object.createPath",
                serde_json::json!({
                    "kind": primitive,
                    "x": x, "y": y,
                    "width": size, "height": size,
                    "page": page, "layer": layer, "cell": cell,
                    "name": null,
                }),
            ),
            ShapeKind::Auto(shape) => {
                let payload = shape.payload_in_rect(page, layer, cell, x, y, size);
                self.run_command(shape.command(), payload)
            }
        };
        if inserted {
            self.status = format!("{label} inserted.");
        }
    }

    /// The box a panel-inserted shape occupies: the canvas center,
    /// `INSERT_SIZE` wide and high. A document with no canvas yet
    /// starts at the origin.
    fn insert_box(&self) -> (f64, f64, f64) {
        let (x, y, width, height) = self.editor.document().pages.get(self.editor.page).map_or(
            (0.0, 0.0, 0.0, 0.0),
            |page| {
                let canvas = &page.info.canvas;
                (canvas.x, canvas.y, canvas.width, canvas.height)
            },
        );
        let size = INSERT_SIZE
            .min(if width > 0.0 { width } else { INSERT_SIZE })
            .min(if height > 0.0 { height } else { INSERT_SIZE });
        let left = if width > 0.0 {
            x + (width - size) / 2.0
        } else {
            x
        };
        let top = if height > 0.0 {
            y + (height - size) / 2.0
        } else {
            y
        };
        (left, top, size)
    }

    /// Window ▸ URL: the selected slice's or hotspot's anchor link
    /// and alt text (`slice.setLink`).
    pub(super) fn url_panel(&mut self, ui: &mut egui::Ui) {
        let Some(snapshot) = self.selected_web_object() else {
            ui.label("Select a slice or hotspot to edit its link.");
            return;
        };
        ui.label(format!("Target: {}", snapshot.name));
        self.slice_link_editor(
            ui,
            snapshot.id,
            snapshot.link.as_ref(),
            snapshot.alt.as_ref(),
            true,
        );
    }

    /// The shared anchor-link and alt-text editor.
    ///
    /// CS6 shows these two fields in the `Properties` panel, and
    /// `FireCraft`'s URL panel is the same data seen from the web side.
    /// Both surfaces render *this* editor, so there is one draft and
    /// one writer: the fields can never show different links, and an
    /// apply from either surface is visible in the other.
    ///
    /// `with_button` adds the explicit Apply button the URL panel
    /// has; the inspector applies on Enter instead, matching the
    /// other inspector fields.
    pub(super) fn slice_link_editor(
        &mut self,
        ui: &mut egui::Ui,
        target: doc::ObjectId,
        link: Option<&String>,
        alt: Option<&String>,
        with_button: bool,
    ) {
        // The draft follows the selection and the stored values: a
        // new target re-seeds it, and so does a stored value that
        // changed under it (the inspector applied a link).
        let stale = draft_is_stale(self.url_draft.as_ref(), target, link, alt);
        if stale {
            self.url_draft = Some(UrlDraft {
                target,
                link: link.cloned().unwrap_or_default(),
                alt: alt.cloned().unwrap_or_default(),
                source_link: link.cloned(),
                source_alt: alt.cloned(),
            });
        }
        let mut apply = false;
        let mut enter = false;
        if let Some(draft) = self.url_draft.as_mut() {
            ui.label("URL");
            let link_field = ui.text_edit_singleline(&mut draft.link);
            ui.label("Alt text");
            let alt_field = ui.text_edit_singleline(&mut draft.alt);
            enter = (link_field.lost_focus() || alt_field.lost_focus())
                && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if with_button {
                apply = ui.button("Apply").clicked();
            }
        }
        if apply || enter {
            let (link, alt) = self.url_draft.as_ref().map_or_else(
                || (String::new(), String::new()),
                |draft| (draft.link.trim().to_owned(), draft.alt.trim().to_owned()),
            );
            self.apply_link(target, &link, &alt);
        }
    }

    /// Sets a web object's anchor link and alt text (`slice.setLink`).
    /// An empty field clears the stored value.
    pub(super) fn apply_link(&mut self, id: doc::ObjectId, link: &str, alt: &str) -> bool {
        self.run_command(
            "slice.setLink",
            serde_json::json!({
                "id": id,
                "link": optional_field(link.trim()),
                "alt": optional_field(alt.trim()),
            }),
        )
    }

    /// Window ▸ Behaviors: the selected slice's interaction
    /// behaviors. The kinds this build documents are offered; the
    /// stored kinds are listed and removable whatever they are.
    pub(super) fn behaviors_panel(&mut self, ui: &mut egui::Ui) {
        let Some(snapshot) = self.selected_web_object() else {
            ui.label("Select a slice or hotspot to edit its behaviors.");
            return;
        };
        ui.label(format!("Target: {}", snapshot.name));
        if snapshot.behaviors.is_empty() {
            ui.label("(no behaviors)");
        }
        for (index, kind) in snapshot.behaviors.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(behavior_label(*kind));
                if ui
                    .small_button("remove")
                    .on_hover_text("Remove this behavior")
                    .clicked()
                {
                    self.remove_behavior(snapshot.id, index);
                }
            });
        }
        ui.separator();
        if ui
            .button("Add popup menu")
            .on_hover_text("Attach a popup-menu behavior to this slice")
            .clicked()
        {
            self.add_behavior(snapshot.id, POPUP_MENU_BEHAVIOR);
        }
        // The swap-image behavior names the slice it swaps to, so it
        // needs a target before it can be attached.
        let targets = self.swap_targets(snapshot.id);
        let mut chosen = self.swap_target.clone();
        egui::ComboBox::from_id_salt("swap_target")
            .selected_text(chosen.as_deref().unwrap_or("(choose a slice)"))
            .show_ui(ui, |ui| {
                for (name, _) in &targets {
                    ui.selectable_value(&mut chosen, Some(name.clone()), name);
                }
            });
        self.swap_target = chosen.clone();
        if ui
            .button("Add swap image")
            .on_hover_text("Swap this slice for another one on click")
            .clicked()
        {
            let Some(name) = chosen else {
                self.status = "swap image: choose the slice to swap to.".to_owned();
                return;
            };
            self.add_swap_behavior(snapshot.id, &name);
        }
    }

    /// Appends one behavior kind to a web object
    /// (`slice.addBehavior`).
    pub(super) fn add_behavior(&mut self, id: doc::ObjectId, kind: i64) -> bool {
        self.run_command(
            "slice.addBehavior",
            serde_json::json!({ "id": id, "kind": kind }),
        )
    }

    /// Drops the behavior at `index` (`slice.removeBehavior`).
    pub(super) fn remove_behavior(&mut self, id: doc::ObjectId, index: usize) -> bool {
        self.run_command(
            "slice.removeBehavior",
            serde_json::json!({ "id": id, "index": index }),
        )
    }

    /// Attaches a swap-image behavior: the target rides the export
    /// key, then the behavior kind is appended.
    pub(super) fn add_swap_behavior(&mut self, id: doc::ObjectId, target: &str) -> bool {
        let named = self.run_command(
            "slice.setData",
            serde_json::json!({
                "id": id,
                "key": SWAP_TARGET_KEY,
                "value": target,
            }),
        );
        named && self.add_behavior(id, SWAP_IMAGE_BEHAVIOR)
    }

    /// The other slices in the document, as swap-image targets.
    fn swap_targets(&self, exclude: doc::ObjectId) -> Vec<(String, doc::ObjectId)> {
        self.editor
            .document()
            .registry
            .iter()
            .filter(|(id, _)| *id != exclude)
            .filter_map(|(id, data)| match data {
                doc::ObjectData::Slice(slice) => Some((slice_name(slice), id)),
                _ => None,
            })
            .collect()
    }

    /// The selected slice or hotspot, snapshotted for a panel to
    /// read and then command.
    pub(super) fn selected_web_object(&self) -> Option<WebSnapshot> {
        let id = self.editor.live_selection().into_iter().find(|id| {
            matches!(
                self.editor.document().registry.get(*id),
                Some(doc::ObjectData::Slice(_))
            )
        })?;
        let doc::ObjectData::Slice(slice) = self.editor.document().registry.get(id)? else {
            return None;
        };
        Some(WebSnapshot {
            id,
            name: slice_name(slice),
            link: slice.link.clone(),
            alt: slice.alt.clone(),
            behaviors: slice
                .behaviors
                .iter()
                .map(|behavior| behavior.kind)
                .collect(),
        })
    }
}

/// A slice's display name: its object name, then its internal name.
fn slice_name(slice: &doc::SliceObject) -> String {
    slice
        .name
        .clone()
        .or_else(|| slice.internal_name.clone())
        .unwrap_or_else(|| "(unnamed slice)".to_owned())
}

/// An optional payload field: an empty string clears the field.
fn optional_field(value: &str) -> serde_json::Value {
    if value.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(value.to_owned())
    }
}

/// Whether the link draft must re-seed from the stored values.
///
/// The draft is stale when it edits another object, or when the
/// stored link or alt text changed under it — the Properties
/// inspector and the URL panel share one draft, so an apply in
/// either place must show up in the other.
fn draft_is_stale(
    draft: Option<&UrlDraft>,
    target: doc::ObjectId,
    link: Option<&String>,
    alt: Option<&String>,
) -> bool {
    draft.is_none_or(|draft| {
        draft.target != target
            || draft.source_link.as_ref() != link
            || draft.source_alt.as_ref() != alt
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The gallery leads with the two primitives and then carries
    /// every Auto Shape tool — the roster CS6's panel shows.
    #[test]
    fn the_gallery_covers_the_primitives_and_every_auto_shape() {
        let roster = shape_roster();
        assert_eq!(roster[0], ("Rectangle", ShapeKind::Primitive("rect")));
        assert_eq!(roster[1], ("Ellipse", ShapeKind::Primitive("ellipse")));
        assert_eq!(roster.len(), AutoShape::ALL.len() + 2);
        for shape in AutoShape::ALL {
            assert!(
                roster
                    .iter()
                    .any(|(_, kind)| *kind == ShapeKind::Auto(shape)),
                "{} missing from the gallery",
                shape.label()
            );
        }
    }

    /// The behavior kinds this build documents are named; an unknown
    /// kind shows its number rather than a guess.
    #[test]
    fn behavior_labels_name_only_the_documented_kinds() {
        assert_eq!(behavior_label(POPUP_MENU_BEHAVIOR), "Popup menu");
        assert_eq!(behavior_label(SWAP_IMAGE_BEHAVIOR), "Swap image");
        assert_eq!(behavior_label(7), "Behavior 7");
    }

    /// The link draft follows the selection: a different object
    /// re-seeds it.
    #[test]
    fn the_link_draft_follows_the_selection() {
        let first = doc::ObjectId::from_parts(1, 0);
        let second = doc::ObjectId::from_parts(2, 0);
        let link = "https://example.invalid/".to_owned();
        let draft = UrlDraft {
            target: first,
            link: link.clone(),
            alt: String::new(),
            source_link: Some(link.clone()),
            source_alt: None,
        };
        assert!(!draft_is_stale(Some(&draft), first, Some(&link), None,));
        assert!(draft_is_stale(Some(&draft), second, Some(&link), None));
    }

    /// A stored value that changed under the draft — the Properties
    /// inspector applied a link while the URL panel was open —
    /// re-seeds the draft, so the two surfaces cannot show different
    /// links.
    #[test]
    fn the_link_draft_follows_a_stored_value_change() {
        let target = doc::ObjectId::from_parts(1, 0);
        let seeded = "https://example.invalid/old".to_owned();
        let applied = "https://example.invalid/new".to_owned();
        let draft = UrlDraft {
            target,
            link: seeded.clone(),
            alt: String::new(),
            source_link: Some(seeded.clone()),
            source_alt: None,
        };
        assert!(draft_is_stale(Some(&draft), target, Some(&applied), None));
        assert!(!draft_is_stale(Some(&draft), target, Some(&seeded), None));
    }

    /// With no draft at all, the editor seeds one.
    #[test]
    fn a_missing_link_draft_is_stale() {
        let target = doc::ObjectId::from_parts(1, 0);
        assert!(draft_is_stale(None, target, None, None));
    }

    /// A panel-inserted shape carries the placement fields its
    /// command requires — the box for the frame-bound shapes, the
    /// center for the circle-bound ones, the endpoints for the
    /// line-bound ones.
    #[test]
    fn the_panel_payload_places_every_shape_family() {
        let star = AutoShape::Star.payload_in_rect(0, 1, 2, 10.0, 20.0, 100.0);
        assert_eq!(star["centerX"], 60.0);
        assert_eq!(star["centerY"], 70.0);
        assert_eq!(star["outer"], 50.0);
        assert_eq!(star["page"], 0);
        assert_eq!(star["layer"], 1);
        assert_eq!(star["cell"], 2);

        let lshape = AutoShape::LShape.payload_in_rect(0, 0, 0, 10.0, 20.0, 100.0);
        assert_eq!(lshape["x"], 10.0);
        assert_eq!(lshape["y"], 20.0);
        assert_eq!(lshape["width"], 100.0);
        assert_eq!(lshape["height"], 100.0);

        let measure = AutoShape::Measure.payload_in_rect(0, 0, 0, 10.0, 20.0, 100.0);
        assert_eq!(measure["startX"], 10.0);
        assert_eq!(measure["startY"], 70.0);
        assert_eq!(measure["endX"], 110.0);
        assert_eq!(measure["endY"], 70.0);
    }

    /// An empty field clears the stored value; a filled one is sent
    /// as-is.
    #[test]
    fn an_empty_field_clears_the_value() {
        assert_eq!(optional_field(""), serde_json::Value::Null);
        assert_eq!(
            optional_field("https://example.org"),
            serde_json::Value::String("https://example.org".to_owned())
        );
    }
}
