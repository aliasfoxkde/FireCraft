//! The web-object panels of the CS6 right dock: Auto Shapes, URL,
//! Behaviors, and Slices.
//!
//! CS6 groups these with the Properties panel: the Auto Shapes
//! library (the shape gallery), the URL panel (the selected object's
//! hyperlink and alt text), the Behaviors panel (the selected slice's
//! interaction behaviors), and the Slices panel (the document's web
//! objects with their export settings). Each panel here drives the
//! engine command that owns the data — `object.createPath` /
//! `autoshape.*` for the shapes, `slice.setLink` for the URL,
//! `slice.addBehavior` / `slice.removeBehavior` /
//! `slice.addSwapImageBehaviorFromPoint` for the behaviors, and
//! `slice.setType` / `slice.setExport` / `hotspot.setShape` for the
//! slices — so no panel invents model state.
//!
//! The event/action catalog the Behaviors panel offers lives in
//! [`super::slice_editor`], next to the mapping from CS6's pairs onto
//! the verbs that persist them.

use std::cell::RefCell;

use super::*;
use crate::editor::AutoShape;
use crate::shell::panel_windows::DOCK_CONTENT_WIDTH;
use crate::shell::slice_editor::{
    BehaviorAction, BehaviorEvent, BehaviorPlan, ExportFormat, SWAP_IMAGE_BEHAVIOR,
    SWAP_TARGET_KEY, SliceKind, SliceRow, SliceShape, behavior_label, is_targeted, plan,
    slice_name, slice_rows,
};

/// The size, in world units, of a shape inserted from the Auto Shapes
/// panel.
const INSERT_SIZE: f64 = 120.0;

thread_local! {
    /// The Behaviors panel's editor draft: the CS6 event/action pair
    /// the panel is set to add next. CS6 keeps this choice in its
    /// "Add Behavior" dialog; here it lives between frames so the
    /// panel's two dropdowns hold their selection.
    static BEHAVIOR_DRAFT: RefCell<(BehaviorEvent, BehaviorAction)> =
        const { RefCell::new((BehaviorEvent::Click, BehaviorAction::SwapImage)) };
}

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
    /// The slice its swap-family behaviors name.
    pub(super) swap_target: Option<String>,
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
    /// behaviors.
    ///
    /// CS6's panel is a two-axis editor — an *event* and an *action* —
    /// over a stack of the behaviors the selected web object carries.
    /// This panel keeps that shape: the two dropdowns, the stack with
    /// its remove buttons, and the pairs the engine cannot persist
    /// named with the reason instead of dropped.
    pub(super) fn behaviors_panel(&mut self, ui: &mut egui::Ui) {
        let Some(snapshot) = self.selected_web_object() else {
            ui.label("Select a slice or hotspot to edit its behaviors.");
            return;
        };
        ui.label(format!("Target: {}", snapshot.name));
        ui.separator();
        ui.label("Behaviors");
        if snapshot.behaviors.is_empty() {
            ui.weak("(no behaviors)");
        }
        for (index, kind) in snapshot.behaviors.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(behavior_label(*kind));
                if is_targeted(*kind)
                    && let Some(target) = &snapshot.swap_target
                {
                    ui.weak(format!("→ {target}"));
                }
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

        // CS6's "Add Behavior" dialog, as two dropdowns.
        let (mut event, mut action) = BEHAVIOR_DRAFT.with(|draft| *draft.borrow());
        ui.horizontal(|ui| {
            ui.label("Event");
            egui::ComboBox::from_id_salt("behavior_event")
                .selected_text(event.label())
                .show_ui(ui, |ui| {
                    for option in BehaviorEvent::ALL {
                        ui.selectable_value(&mut event, option, option.label());
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Action");
            egui::ComboBox::from_id_salt("behavior_action")
                .selected_text(action.label())
                .show_ui(ui, |ui| {
                    for option in BehaviorAction::ALL {
                        ui.selectable_value(&mut action, option, option.label());
                    }
                });
        });
        BEHAVIOR_DRAFT.with(|draft| *draft.borrow_mut() = (event, action));

        let choice = plan(event, action);
        match choice {
            BehaviorPlan::Kind { kind, target } => {
                let text = if target {
                    format!(
                        "Adds \"{}\" with the target slice below.",
                        behavior_label(kind)
                    )
                } else {
                    format!("Adds \"{}\".", behavior_label(kind))
                };
                ui.weak(text);
            }
            BehaviorPlan::Link => {
                ui.weak("Sets the target's URL (below).");
            }
            BehaviorPlan::Unsupported(reason) => {
                ui.colored_label(Color32::from_rgb(220, 120, 120), reason);
            }
        }

        // The swap family names another slice; the target is chosen
        // from the document's other web objects.
        if matches!(choice, BehaviorPlan::Kind { target: true, .. }) {
            let targets = self.swap_targets(snapshot.id);
            let mut chosen = self.swap_target.clone();
            egui::ComboBox::from_id_salt("behavior_target")
                .selected_text(chosen.as_deref().unwrap_or("(choose a slice)"))
                .show_ui(ui, |ui| {
                    for (name, _) in &targets {
                        ui.selectable_value(&mut chosen, Some(name.clone()), name);
                    }
                });
            self.swap_target = chosen.clone();
        }
        // The link family writes the object's URL, so the field the
        // URL panel and the inspector share is shown here too.
        if matches!(choice, BehaviorPlan::Link) {
            self.slice_link_editor(
                ui,
                snapshot.id,
                snapshot.link.as_ref(),
                snapshot.alt.as_ref(),
                true,
            );
        }

        let addable = choice.is_supported();
        if ui
            .add_enabled(addable, egui::Button::new("Add behavior"))
            .on_hover_text("Attach this behavior to the target")
            .clicked()
        {
            self.add_cs6_behavior(snapshot.id, event, action);
        }
        self.unsupported_behaviors(ui);
    }

    /// The CS6 event/action pairs the engine cannot persist, with the
    /// reason. CS6 offers them; this build says so rather than
    /// pretending the panel can save them.
    fn unsupported_behaviors(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Not supported by the engine", |ui| {
            ui.weak(
                "CS6 offers these; the engine has no place to keep them, \
                 so they cannot be added.",
            );
            for event in BehaviorEvent::ALL {
                for action in BehaviorAction::ALL {
                    let BehaviorPlan::Unsupported(reason) = plan(event, action) else {
                        continue;
                    };
                    ui.label(format!("{} / {}: {reason}", event.label(), action.label()));
                }
            }
        });
    }

    /// Adds the CS6 event/action pair to a web object, through the
    /// verb that owns it.
    fn add_cs6_behavior(
        &mut self,
        id: doc::ObjectId,
        event: BehaviorEvent,
        action: BehaviorAction,
    ) {
        match plan(event, action) {
            BehaviorPlan::Kind {
                kind,
                target: false,
            } => {
                self.add_behavior(id, kind);
            }
            BehaviorPlan::Kind { kind, target: true } => {
                let Some(target) = self.swap_target.clone() else {
                    self.status = format!(
                        "{} / {}: choose the slice the behavior names.",
                        event.label(),
                        action.label()
                    );
                    return;
                };
                self.add_targeted_behavior(id, kind, &target);
            }
            BehaviorPlan::Link => {
                // The engine stores one anchor link per web object, so
                // the link family writes that link.
                let (link, alt) = self.url_draft.as_ref().map_or_else(
                    || (String::new(), String::new()),
                    |draft| (draft.link.trim().to_owned(), draft.alt.trim().to_owned()),
                );
                self.apply_link(id, &link, &alt);
            }
            BehaviorPlan::Unsupported(reason) => {
                self.status = format!("{} / {}: {reason}.", event.label(), action.label());
            }
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
        self.set_swap_target(id, target) && self.add_behavior(id, SWAP_IMAGE_BEHAVIOR)
    }

    /// Attaches a behavior that names another slice.
    ///
    /// The swap-image kind has a CS6 verb of its own
    /// (`slice.addSwapImageBehaviorFromPoint`): the receiving slice is
    /// the selected one and the point names the slice it swaps to.
    /// The nav-bar kinds have no such verb — their target rides the
    /// export key and the kind is appended.
    fn add_targeted_behavior(&mut self, id: doc::ObjectId, kind: i64, target: &str) -> bool {
        if kind == SWAP_IMAGE_BEHAVIOR
            && let Some(point) = self.swap_point(id, target)
        {
            // The verb reads the document's selection, so the
            // receiving slice has to be the selected one.
            self.editor.set_selection(vec![id]);
            if let Err(error) = self.editor.sync_selection() {
                self.status = format!("selection failed: {error}");
                return false;
            }
            let attached = self.run_command(
                "slice.addSwapImageBehaviorFromPoint",
                serde_json::json!({ "where": { "x": point.x, "y": point.y } }),
            );
            if attached {
                // The verb resolves the target from the point; the
                // export reads the target's *name*, so the two have to
                // agree.
                if self.swap_target_of(id).as_deref() != Some(target) {
                    self.set_swap_target(id, target);
                }
                return true;
            }
        }
        if kind == SWAP_IMAGE_BEHAVIOR {
            // The point verb refused — the target is not on the
            // canvas under that name. The name rides the export key
            // instead.
            self.add_swap_behavior(id, target)
        } else {
            // The nav-bar kinds have no point verb: the target's name
            // rides the export key and the kind is appended.
            self.set_swap_target(id, target) && self.add_behavior(id, kind)
        }
    }

    /// Names the slice a swap-family behavior points at
    /// (`slice.setData`).
    fn set_swap_target(&mut self, id: doc::ObjectId, target: &str) -> bool {
        self.run_command(
            "slice.setData",
            serde_json::json!({
                "id": id,
                "key": SWAP_TARGET_KEY,
                "value": target,
            }),
        )
    }

    /// The swap target a web object names, if any.
    fn swap_target_of(&self, id: doc::ObjectId) -> Option<String> {
        let doc::ObjectData::Slice(slice) = self.editor.document().registry.get(id)? else {
            return None;
        };
        crate::shell::slice_editor::swap_target_of(slice)
    }

    /// A point on the target slice that the engine resolves to the
    /// target rather than to the receiving slice.
    ///
    /// The CS6 verb finds the slice under a point, and the topmost
    /// one wins — so the point is taken from the target's frame, and
    /// away from the receiving slice's frame when the two overlap.
    fn swap_point(&self, receiver: doc::ObjectId, target_name: &str) -> Option<geom::Point> {
        let document = self.editor.document();
        let target = document.registry.iter().find_map(|(_, data)| match data {
            doc::ObjectData::Slice(slice) if slice_name(slice) == target_name => Some(slice),
            _ => None,
        })?;
        let frame = target.frame;
        let receiver_frame = document.registry.get(receiver).and_then(|data| match data {
            doc::ObjectData::Slice(slice) => Some(slice.frame),
            _ => None,
        });
        let mid_x = (frame.x0 + frame.x1) / 2.0;
        let mid_y = (frame.y0 + frame.y1) / 2.0;
        let candidates = [
            geom::Point::new(mid_x, mid_y),
            geom::Point::new(frame.x0 + 1.0, frame.y0 + 1.0),
            geom::Point::new(frame.x1 - 1.0, frame.y0 + 1.0),
            geom::Point::new(frame.x0 + 1.0, frame.y1 - 1.0),
            geom::Point::new(frame.x1 - 1.0, frame.y1 - 1.0),
            geom::Point::new(frame.x0 + 1.0, mid_y),
            geom::Point::new(frame.x1 - 1.0, mid_y),
            geom::Point::new(mid_x, frame.y0 + 1.0),
            geom::Point::new(mid_x, frame.y1 - 1.0),
        ];
        candidates
            .into_iter()
            .find(|point| {
                frame.contains(*point) && receiver_frame.is_none_or(|frame| !frame.contains(*point))
            })
            .or(candidates.first().copied())
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
            swap_target: crate::shell::slice_editor::swap_target_of(slice),
        })
    }

    /// Window ▸ Slices: every web object in the document — slices and
    /// hotspots — with its type, export settings, and behaviors, and
    /// an editor for the selected one.
    ///
    /// Clicking a row selects the web object on the canvas; clicking a
    /// slice on the canvas selects its row here, because the panel
    /// reads the same selection the canvas writes.
    pub(super) fn slices_panel(&mut self, ui: &mut egui::Ui) {
        ui.weak("Click a slice or hotspot to select it on the canvas.");
        ui.separator();
        let rows = slice_rows(self.editor.document());
        if rows.is_empty() {
            ui.label("(no slices or hotspots — draw one with the Slice tool)");
            return;
        }
        let selected = self.editor.live_selection();
        let mut picked = None;
        for row in &rows {
            let active = selected.contains(&row.id);
            ui.horizontal(|ui| {
                if ui.selectable_label(active, row.name.clone()).clicked() {
                    picked = Some(row.id);
                }
                ui.weak(row.kind.label());
            });
            ui.weak(row.detail());
        }
        if let Some(id) = picked {
            // The canvas draws the selection, so the panel's pick has
            // to land in the view's selection — and in the document's,
            // for the selection-scoped verbs.
            self.editor.set_selection(vec![id]);
            if let Err(error) = self.editor.sync_selection() {
                self.status = format!("selection failed: {error}");
            }
        }
        let selected = self.editor.live_selection();
        let Some(row) = rows.iter().find(|row| selected.contains(&row.id)).cloned() else {
            ui.separator();
            ui.weak("Select a slice to edit its type, shape, and export settings.");
            return;
        };
        ui.separator();
        self.slice_inspector(ui, &row);
    }

    /// The editor for the selected web object: its type, its shape,
    /// its export settings, and its link.
    fn slice_inspector(&mut self, ui: &mut egui::Ui, row: &SliceRow) {
        ui.label(format!("{} — {}", row.name, row.kind.label()));
        ui.weak(row.dimensions());

        // The type: a foreground slice, a background slice, or a
        // hotspot (`slice.setType`).
        let mut kind = row.kind;
        egui::ComboBox::from_id_salt(("slice_type", row.id))
            .selected_text(kind.label())
            .show_ui(ui, |ui| {
                for option in SliceKind::ALL {
                    ui.selectable_value(&mut kind, option, option.label());
                }
            });
        if kind != row.kind {
            self.set_slice_kind(row.id, kind);
        }

        // The shape (`hotspot.setShape`).
        let mut shape = row.shape;
        egui::ComboBox::from_id_salt(("slice_shape", row.id))
            .selected_text(shape.label())
            .show_ui(ui, |ui| {
                for option in SliceShape::ALL {
                    ui.selectable_value(&mut shape, option, option.label());
                }
            });
        if shape != row.shape {
            self.set_slice_shape(row.id, shape);
        }

        // The export format (`slice.setExport`).
        let mut format = if row.overridden {
            Some(row.format)
        } else {
            None
        };
        egui::ComboBox::from_id_salt(("slice_export", row.id))
            .selected_text(format.map_or("Page default", ExportFormat::label))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut format, None, "Page default");
                for option in ExportFormat::ALL {
                    ui.selectable_value(&mut format, Some(option), option.label());
                }
            });
        if format
            != if row.overridden {
                Some(row.format)
            } else {
                None
            }
        {
            self.set_slice_format(row.id, format);
        }
        if row.format == ExportFormat::Jpeg {
            let mut quality = row.jpeg_quality.unwrap_or(80);
            if ui
                .add(egui::Slider::new(&mut quality, 0..=100).text("JPEG quality"))
                .changed()
            {
                self.set_jpeg_quality(row.id, quality);
            }
        }

        // The anchor link and the alt text.
        self.slice_link_editor(ui, row.id, row.link.as_ref(), row.alt.as_ref(), true);

        ui.label(row.behavior_count());
        if ui
            .button("Show HTML preview")
            .on_hover_text("Open the panel with this document's web export")
            .clicked()
        {
            self.toggle_panel("HTML Preview");
        }
    }

    /// Sets a web object's type (`slice.setType`): a foreground
    /// slice, a background slice, or a hotspot.
    fn set_slice_kind(&mut self, id: doc::ObjectId, kind: SliceKind) -> bool {
        self.run_command(
            "slice.setType",
            serde_json::json!({
                "id": id,
                "slice_type": kind.type_value(),
            }),
        )
    }

    /// Sets a web object's shape (`hotspot.setShape`).
    ///
    /// The verb addresses the document's selection, so the object is
    /// selected before it runs.
    fn set_slice_shape(&mut self, id: doc::ObjectId, shape: SliceShape) -> bool {
        self.editor.set_selection(vec![id]);
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return false;
        }
        self.run_command(
            "hotspot.setShape",
            serde_json::json!({
                "what": "hotspots and slices",
                "shape": shape.command_name(),
            }),
        )
    }

    /// Sets a web object's export format (`slice.setExport`); no
    /// format drops the override and follows the page again.
    fn set_slice_format(&mut self, id: doc::ObjectId, format: Option<ExportFormat>) -> bool {
        match format {
            None => self.run_command(
                "slice.setExport",
                serde_json::json!({ "id": id, "clear": true }),
            ),
            Some(format) => self.run_command(
                "slice.setExport",
                serde_json::json!({ "id": id, "format": format.code() }),
            ),
        }
    }

    /// Sets a web object's JPEG quality (`slice.setExport`).
    fn set_jpeg_quality(&mut self, id: doc::ObjectId, quality: i64) -> bool {
        self.run_command(
            "slice.setExport",
            serde_json::json!({ "id": id, "jpeg_quality": quality }),
        )
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
        assert_eq!(behavior_label(SWAP_IMAGE_BEHAVIOR), "Swap Image");
        assert_eq!(
            behavior_label(crate::shell::slice_editor::POPUP_MENU_BEHAVIOR),
            "Set Pop-up"
        );
        assert_eq!(behavior_label(7), "Roll Over Restore (nav bar)");
        assert_eq!(behavior_label(42), "Behavior 42");
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

    /// Draws a web object and names it — the panel's rows need names
    /// the swap family can point at.
    fn draw_slice(
        editor: &mut crate::editor::Editor,
        name: &str,
        frame: (f64, f64, f64, f64),
    ) -> doc::ObjectId {
        editor
            .apply(
                "slice.create",
                serde_json::json!({
                    "shape": "rect",
                    "x": frame.0, "y": frame.1,
                    "width": frame.2 - frame.0, "height": frame.3 - frame.1,
                    "points": null, "page": 0,
                    "name": name, "link": null, "alt": null,
                }),
            )
            .expect("slice.create applies");
        editor
            .document()
            .registry
            .iter()
            .find_map(|(id, data)| match data {
                doc::ObjectData::Slice(slice) if slice.name.as_deref() == Some(name) => Some(id),
                _ => None,
            })
            .expect("the slice is registered")
    }

    /// A shell with two web objects: `plate` and `button`.
    fn slice_shell() -> (Shell, doc::ObjectId, doc::ObjectId) {
        let mut editor = crate::editor::Editor::untitled();
        let plate = draw_slice(&mut editor, "plate", (0.0, 0.0, 40.0, 20.0));
        let button = draw_slice(&mut editor, "button", (60.0, 0.0, 100.0, 20.0));
        (super::super::tests::gui_shell(editor), plate, button)
    }

    /// The web object behind an id.
    fn slice_of(shell: &Shell, id: doc::ObjectId) -> doc::SliceObject {
        match shell.editor.document().registry.get(id) {
            Some(doc::ObjectData::Slice(slice)) => slice.clone(),
            _ => panic!("the object is a web object"),
        }
    }

    /// The Slices panel's type editor writes the engine's `TSLi`
    /// value: a background slice carries 2, a hotspot carries none.
    #[test]
    fn the_type_editor_writes_the_engines_slice_type() {
        let (mut shell, plate, _) = slice_shell();
        assert!(shell.set_slice_kind(plate, SliceKind::Background));
        assert_eq!(slice_of(&shell, plate).slice_type, Some(2));
        assert!(shell.set_slice_kind(plate, SliceKind::Hotspot));
        assert_eq!(slice_of(&shell, plate).slice_type, None);
        assert!(shell.set_slice_kind(plate, SliceKind::Foreground));
        assert_eq!(slice_of(&shell, plate).slice_type, Some(0));
    }

    /// The export editor sets the slice's own format and drops the
    /// override again when the page default is chosen.
    #[test]
    fn the_export_editor_sets_and_clears_the_override() {
        let (mut shell, plate, _) = slice_shell();
        assert!(shell.set_slice_format(plate, Some(ExportFormat::Jpeg)));
        let export = slice_of(&shell, plate).export.expect("the override is set");
        assert_eq!(export.format, Some(1));
        assert!(shell.set_slice_format(plate, None));
        assert!(slice_of(&shell, plate).export.is_none());
    }

    /// The shape editor runs the CS6 shape verb on the selected web
    /// object.
    #[test]
    fn the_shape_editor_runs_the_shape_verb() {
        let (mut shell, plate, _) = slice_shell();
        assert!(shell.set_slice_shape(plate, SliceShape::Ellipse));
        assert_eq!(slice_of(&shell, plate).shape, 1);
        assert!(shell.set_slice_shape(plate, SliceShape::Polygon));
        assert_eq!(slice_of(&shell, plate).shape, 2);
    }

    /// A swap behavior added from the panel round-trips through the
    /// engine: the behavior kind is on the object and the target's
    /// name rides the export key the web export reads.
    #[test]
    fn a_swap_behavior_round_trips_through_the_engine() {
        let (mut shell, plate, _) = slice_shell();
        assert!(
            shell.add_targeted_behavior(plate, SWAP_IMAGE_BEHAVIOR, "button"),
            "the swap behavior attaches"
        );
        let slice = slice_of(&shell, plate);
        assert_eq!(slice.behaviors.len(), 1);
        assert_eq!(slice.behaviors[0].kind, SWAP_IMAGE_BEHAVIOR);
        assert_eq!(
            crate::shell::slice_editor::swap_target_of(&slice).as_deref(),
            Some("button")
        );
    }

    /// The swap point names the target slice, not the slice the
    /// behavior is added to — even when the two overlap.
    #[test]
    fn the_swap_point_names_the_target_not_the_receiver() {
        let mut editor = crate::editor::Editor::untitled();
        let plate = draw_slice(&mut editor, "plate", (0.0, 0.0, 50.0, 20.0));
        let button = draw_slice(&mut editor, "button", (20.0, 0.0, 60.0, 20.0));
        let shell = super::super::tests::gui_shell(editor);
        let point = shell
            .swap_point(plate, "button")
            .expect("the target has a point");
        let button_frame = slice_of(&shell, button).frame;
        let plate_frame = slice_of(&shell, plate).frame;
        assert!(
            button_frame.contains(point),
            "{point:?} is not on the target"
        );
        assert!(!plate_frame.contains(point), "{point:?} is on the receiver");
    }
}
