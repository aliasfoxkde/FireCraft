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
use crate::shell::panel_windows::DOCK_CONTENT_WIDTH;

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

    /// Align the definition's content center with the insertion box center.
    fn symbol_insert_translation(&self, number: i64) -> (f64, f64) {
        let (x, y, size) = self.insert_box();
        self.editor
            .definition_content_bounds(number)
            .map_or((x, y), |bounds| {
                let center = bounds.center();
                (x + size / 2.0 - center.x, y + size / 2.0 - center.y)
            })
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

    /// Window ▸ Slices: every slice in the document, with its type,
    /// link, and export override — the panel the `slice.*` verbs were
    /// written for. Clicking a row selects the slice on the canvas.
    pub(super) fn slices_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Click a slice to select it; edit its link in URL.");
        ui.separator();
        let rows: Vec<(doc::ObjectId, String, String, String)> = self
            .editor
            .document()
            .registry
            .iter()
            .filter_map(|(id, data)| match data {
                doc::ObjectData::Slice(slice) => {
                    let kind = match slice.shape {
                        1 => "ellipse".to_owned(),
                        2 => "polygon".to_owned(),
                        _ => "rect".to_owned(),
                    };
                    let kind = if slice.slice_type == Some(2) {
                        format!("{kind} · background")
                    } else {
                        format!("{kind} · foreground")
                    };
                    let link = slice.link.clone().unwrap_or_else(|| "—".to_owned());
                    let export = slice.export.as_ref().map_or_else(
                        || "page default".to_owned(),
                        |export| {
                            let format = match export.format.unwrap_or(2) {
                                0 => "gif",
                                1 => "jpeg",
                                _ => "png",
                            };
                            format!("override: {format}")
                        },
                    );
                    Some((id, slice_name(slice), kind, format!("{link} · {export}")))
                }
                _ => None,
            })
            .collect();
        if rows.is_empty() {
            ui.label("(no slices — insert one with Edit ▸ Insert)");
            return;
        }
        let selected = self.editor.live_selection();
        for (id, name, kind, detail) in &rows {
            let active = selected.contains(id);
            let row = ui.selectable_label(active, format!("{name} · {kind}"));
            if row.clicked() {
                self.editor.set_selection(vec![*id]);
            }
            ui.label(detail);
        }
    }

    /// Window ▸ CSS Properties: the CSS the selected slice's
    /// background knobs and link produce, as CS6's panel shows it.
    /// Read-only — the edit surfaces are the slice verbs.
    pub(super) fn css_properties_panel(&mut self, ui: &mut egui::Ui) {
        let Some(snapshot) = self.selected_web_object() else {
            ui.label("Select a slice to see its CSS.");
            return;
        };
        ui.label(format!("Target: {}", snapshot.name));
        let Some(doc::ObjectData::Slice(slice)) = self.editor.document().registry.get(snapshot.id)
        else {
            return;
        };
        let css = slice_css(slice);
        if css.is_empty() {
            ui.label("(no CSS properties — this slice carries no background)");
            return;
        }
        ui.add(
            egui::TextEdit::multiline(&mut css.as_str())
                .desired_width(DOCK_CONTENT_WIDTH)
                .desired_rows(6)
                .interactive(false),
        );
    }

    /// Window ▸ Find and Replace: searches text objects and replaces
    /// through `text.setText` — CS6's dialog narrowed to the model
    /// this build carries (whole text-object runs).
    pub(super) fn find_replace_panel(&mut self, ui: &mut egui::Ui) {
        let mut query = self.find_query.clone().unwrap_or_default();
        ui.label("Find (text objects)");
        ui.text_edit_singleline(&mut query);
        self.find_query = Some(query.clone());
        if query.is_empty() {
            ui.label("(type to search text objects)");
            return;
        }
        let matches: Vec<(doc::ObjectId, String)> = self
            .editor
            .document()
            .registry
            .iter()
            .filter_map(|(id, data)| match data {
                doc::ObjectData::Text(text) => {
                    let characters = text_characters(text);
                    characters.contains(&query).then_some((id, characters))
                }
                _ => None,
            })
            .collect();
        if matches.is_empty() {
            ui.label(format!("no text matches \"{query}\""));
            return;
        }
        ui.label(format!("{} match(es)", matches.len()));
        let mut replacement = self.find_replacement.clone().unwrap_or_default();
        ui.label("Replace with (empty removes the text)");
        ui.text_edit_singleline(&mut replacement);
        self.find_replacement = Some(replacement.clone());
        let mut replace_all = false;
        for (id, text) in &matches {
            ui.horizontal(|ui| {
                ui.label(format!("\"{}\"", truncate_for_panel(text, 32)));
                if ui.small_button("replace").clicked() {
                    self.replace_text(*id, &query, &replacement);
                }
                if ui.small_button("all").clicked() {
                    replace_all = true;
                }
            });
        }
        if replace_all {
            for (id, _) in &matches {
                self.replace_text(*id, &query, &replacement);
            }
            self.status = format!("replaced in {} text object(s)", matches.len());
        }
    }

    /// Replaces every occurrence of `query` in a text object
    /// (`text.setText` with the rewritten run).
    fn replace_text(&mut self, id: doc::ObjectId, query: &str, replacement: &str) {
        let Some(doc::ObjectData::Text(text)) = self.editor.document().registry.get(id) else {
            return;
        };
        let updated = text_characters(text).replace(query, replacement);
        self.run_command(
            "text.setText",
            serde_json::json!({ "id": id, "text": updated }),
        );
    }

    /// Window ▸ Common Library: CS6's shipped symbol libraries. The
    /// libraries ship as proprietary CS6 resources, so this build
    /// seeds the panel from the document's own symbols instead — the
    /// same drag-to-place the Document Library panel carries, minus
    /// the packaged content.
    pub(super) fn common_library_panel(&mut self, ui: &mut egui::Ui) {
        ui.weak(
            "The CS6 shipped libraries are proprietary resources; \
             this panel lists the document's own symbols.",
        );
        ui.separator();
        let rows: Vec<(i64, String, i64)> = self
            .editor
            .document()
            .symbols
            .iter()
            .map(|definition| {
                (
                    definition.symbol_number,
                    definition.name.clone(),
                    definition.frame_count,
                )
            })
            .collect();
        if rows.is_empty() {
            ui.label("(no symbols — create one in Document Library)");
            return;
        }
        for (number, name, states) in &rows {
            ui.horizontal(|ui| {
                let row = ui.selectable_label(false, format!("{name} · {states}"));
                if row.drag_started() {
                    egui::DragAndDrop::set_payload(ui.ctx(), *number);
                }
                if row.clicked() {
                    let (x, y) = self.symbol_insert_translation(*number);
                    let placed = self.run_command(
                        "symbol.createInstance",
                        serde_json::json!({
                            "symbol": number,
                            "x": x, "y": y,
                            "page": self.editor.page,
                            "layer": self.editor.active_layer(),
                            "cell": self.editor.frame,
                        }),
                    );
                    if placed {
                        self.status = format!("{name} instance inserted.");
                    }
                }
            });
        }
    }
}

/// The slice's CSS: the background knobs (`CBTb` family) and the
/// link, as declarations. Empty when the slice carries nothing the
/// panel can express.
fn slice_css(slice: &doc::SliceObject) -> String {
    let mut rules: Vec<String> = Vec::new();
    if let Some(background) = &slice.background {
        if let Some(repeat) = background.repeat {
            rules.push(match repeat {
                1 => "background-repeat: repeat".to_owned(),
                2 => "background-repeat: repeat-x".to_owned(),
                3 => "background-repeat: repeat-y".to_owned(),
                _ => "background-repeat: no-repeat".to_owned(),
            });
        }
        if let Some(attachment) = background.attachment {
            rules.push(match attachment {
                1 => "background-attachment: scroll".to_owned(),
                _ => "background-attachment: fixed".to_owned(),
            });
        }
        if let Some(h) = background.h_position {
            let value = match h {
                0 => "left".to_owned(),
                1 => "center".to_owned(),
                2 => "right".to_owned(),
                _ => background
                    .h_value
                    .map_or_else(|| "0%".to_owned(), |v| format!("{v}px")),
            };
            rules.push(format!("background-position-x: {value}"));
        }
        if let Some(v) = background.v_position {
            let value = match v {
                0 => "top".to_owned(),
                1 => "center".to_owned(),
                2 => "bottom".to_owned(),
                _ => background
                    .v_value
                    .map_or_else(|| "0%".to_owned(), |v| format!("{v}px")),
            };
            rules.push(format!("background-position-y: {value}"));
        }
    }
    if let Some(link) = &slice.link {
        rules.push(format!("/* link: {link} */"));
    }
    let mut css = rules.join(";\n");
    if !css.is_empty() {
        css.push(';');
    }
    css
}

/// A text object's characters: its runs concatenated.
fn text_characters(text: &doc::TextObject) -> String {
    text.runs.iter().map(|run| run.text.as_str()).collect()
}

/// Shortens panel row text to `max` characters.
fn truncate_for_panel(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        let cut: String = text.chars().take(max).collect();
        format!("{cut}…")
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

    #[test]
    fn symbol_placement_centers_offset_content_on_the_canvas() {
        let mut shell = super::super::tests::gui_shell(super::super::tests::gui_editor());
        let id = shell.editor.current_cell().unwrap().elements[0];
        shell
            .editor
            .apply(
                "symbol.create",
                serde_json::json!({
                    "ids": [id], "name": "offset rectangle", "symbol_type": "graphic",
                }),
            )
            .unwrap();
        let number = shell.editor.document().symbols[0].symbol_number;
        let bounds = shell.editor.definition_content_bounds(number).unwrap();
        let (x, y) = shell.symbol_insert_translation(number);
        let canvas = &shell.editor.document().pages[0].info.canvas;
        assert_eq!(x + bounds.center().x, canvas.x + canvas.width / 2.0);
        assert_eq!(y + bounds.center().y, canvas.y + canvas.height / 2.0);
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
