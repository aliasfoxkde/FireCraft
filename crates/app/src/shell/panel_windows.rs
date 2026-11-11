//! CS6's panel system: one dock column of stacked, collapsible panels
//! along the right edge, with the Window menu as the second way in.
//!
//! CS6 keeps a *group* of docked panels along the right edge, each
//! with its own header, stacked one above the next; the column
//! scrolls when the stack is taller than the window. egui has no dock
//! system, so the column is a [`egui::SidePanel`] on the right: the
//! panels are stacked inside it in CS6's order, each under a CS6
//! header (collapse arrow, name, close button).
//!
//! Why a side panel and not free-floating windows: the dock must
//! *reserve* its width. As floating windows the panels covered the
//! canvas, and their fixed vertical step meant the lower ones fell
//! below the fold — at 1280×800 only the first two panels were
//! reachable. A side panel shrinks the canvas instead of covering it,
//! and the column scrolls, so every panel stays reachable at any
//! viewport height.
//!
//! Panel *content* lives in [`super::panels`]; this module owns the
//! inventory, the dock, and the open/closed bookkeeping.

use super::*;

/// The panels this shell implements, in the CS6 right-dock order.
pub(crate) const PANELS: &[PanelSpec] = &[
    PanelSpec {
        name: "Properties",
        kind: PanelKind::Properties,
        open_by_default: true,
    },
    PanelSpec {
        name: "Pages",
        kind: PanelKind::Pages,
        open_by_default: true,
    },
    PanelSpec {
        name: "States",
        kind: PanelKind::States,
        open_by_default: true,
    },
    PanelSpec {
        name: "Layers",
        kind: PanelKind::Layers,
        open_by_default: true,
    },
    PanelSpec {
        name: "Styles",
        kind: PanelKind::Styles,
        open_by_default: false,
    },
    PanelSpec {
        name: "Document Library",
        kind: PanelKind::Library,
        open_by_default: false,
    },
    PanelSpec {
        name: "History",
        kind: PanelKind::History,
        open_by_default: false,
    },
    PanelSpec {
        name: "Color Mixer",
        kind: PanelKind::ColorMixer,
        open_by_default: false,
    },
    PanelSpec {
        name: "Swatches",
        kind: PanelKind::Swatches,
        open_by_default: false,
    },
    PanelSpec {
        name: "Info",
        kind: PanelKind::Info,
        open_by_default: false,
    },
    PanelSpec {
        name: "Auto Shapes",
        kind: PanelKind::AutoShapes,
        open_by_default: false,
    },
    PanelSpec {
        name: "URL",
        kind: PanelKind::Url,
        open_by_default: false,
    },
    PanelSpec {
        name: "Behaviors",
        kind: PanelKind::Behaviors,
        open_by_default: false,
    },
    PanelSpec {
        name: "Optimize",
        kind: PanelKind::Optimize,
        open_by_default: false,
    },
    PanelSpec {
        name: "Slices",
        kind: PanelKind::Slices,
        open_by_default: false,
    },
    PanelSpec {
        name: "HTML Preview",
        kind: PanelKind::HtmlPreview,
        open_by_default: false,
    },
    PanelSpec {
        name: "CSS Properties",
        kind: PanelKind::CssProperties,
        open_by_default: false,
    },
    PanelSpec {
        name: "Find and Replace",
        kind: PanelKind::FindReplace,
        open_by_default: false,
    },
    PanelSpec {
        name: "Common Library",
        kind: PanelKind::CommonLibrary,
        open_by_default: false,
    },
];

/// The Web Layer panel has no Window-menu row in CS6 — it is always
/// present, so it is listed separately.
pub(crate) const WEB_PANEL: &str = "Web Layer";

/// The Web Layer panel's spec — the one panel with no Window-menu row
/// and no close button (nothing could reopen it).
const WEB_PANEL_SPEC: PanelSpec = PanelSpec {
    name: WEB_PANEL,
    kind: PanelKind::WebLayer,
    open_by_default: true,
};

/// One panel of the CS6 panel inventory.
#[derive(Clone, Copy)]
pub(crate) struct PanelSpec {
    /// The panel's title, as CS6 names it (and as the Window menu
    /// row names it).
    pub(crate) name: &'static str,
    /// Which content the panel draws.
    pub(crate) kind: PanelKind,
    /// Whether CS6 opens the panel on a fresh document.
    pub(crate) open_by_default: bool,
}

/// The content a panel draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PanelKind {
    Properties,
    Pages,
    States,
    Layers,
    Styles,
    Library,
    History,
    ColorMixer,
    Swatches,
    Info,
    AutoShapes,
    Url,
    Behaviors,
    WebLayer,
    Optimize,
    Slices,
    HtmlPreview,
    CssProperties,
    FindReplace,
    CommonLibrary,
}

/// Which edge the panel column docks to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DockSide {
    /// The column hugs the left edge of the window.
    Left,
    /// The column hugs the right edge of the window.
    Right,
}

impl DockSide {
    /// The side's name, as the panel snapshot records it.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match *self {
            DockSide::Left => "left",
            DockSide::Right => "right",
        }
    }

    /// The other dock: where a panel goes when it is moved.
    #[must_use]
    pub fn other(self) -> Self {
        match self {
            DockSide::Left => DockSide::Right,
            DockSide::Right => DockSide::Left,
        }
    }
}

/// The edge the panel column docks to. CS6 keeps the group on the
/// right; the panel-snapshot gate pins the choice, so moving the
/// column is a deliberate re-pin, never a silent drift.
const DOCK_SIDE: DockSide = DockSide::Right;

/// The payload a panel drag carries: which panel is being moved.
#[derive(Clone, Copy)]
struct PanelDrag {
    /// The dragged panel's CS6 name.
    name: &'static str,
}

/// One panel of the CS6 inventory, as plain data.
///
/// The panel *structure* — the set, the dock order, the dock side,
/// the header text, the fresh-document visibility — is what the
/// panel-snapshot gate pins. The content a panel draws is not: it
/// changes with every feature, and a content change is reviewed as
/// code, not as a golden diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelSummary {
    /// The panel's stable id: its CS6 name, lowercased with the
    /// spaces turned into dashes.
    pub id: String,
    /// The header text the panel's window carries.
    pub header: String,
    /// The edge the panel column docks to.
    pub side: DockSide,
    /// The panel's place in the dock, counted from the top.
    pub order: usize,
    /// Whether a fresh document opens the panel.
    pub visible_by_default: bool,
    /// Whether the Window menu carries a row that toggles the panel.
    pub toggleable: bool,
}

/// The CS6 panel inventory, in dock order, as plain data.
///
/// The Web Layer panel has no Window-menu row in CS6 — it is always
/// present — so it is listed after the toggleable panels, at the
/// bottom of the dock.
#[must_use]
pub fn panel_inventory() -> Vec<PanelSummary> {
    let mut panels: Vec<PanelSummary> = PANELS
        .iter()
        .enumerate()
        .map(|(order, spec)| PanelSummary {
            id: panel_id(spec.name),
            header: spec.name.to_owned(),
            side: DOCK_SIDE,
            order,
            visible_by_default: spec.open_by_default,
            toggleable: true,
        })
        .collect();
    panels.push(PanelSummary {
        id: panel_id(WEB_PANEL),
        header: WEB_PANEL.to_owned(),
        side: DOCK_SIDE,
        order: PANELS.len(),
        visible_by_default: true,
        toggleable: false,
    });
    panels
}

/// The machine-facing id of a panel: its CS6 name, lowercased with
/// the spaces turned into dashes.
fn panel_id(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

/// The spec of a panel this shell knows, including the Web Layer
/// panel — which has no Window-menu row but is a panel all the same.
pub(crate) fn panel_spec(name: &str) -> Option<PanelSpec> {
    PANELS
        .iter()
        .copied()
        .find(|spec| spec.name == name)
        .or_else(|| (WEB_PANEL == name).then_some(WEB_PANEL_SPEC))
}

/// The width of the right dock, in points — CS6's dock is a narrow
/// column; the canvas keeps the rest.
pub(crate) const DOCK_WIDTH: f32 = 258.0;

/// The width reserved for a panel's content inside the dock, after the
/// dock frame, the scroll bar, and the panel's own padding.
pub(crate) const DOCK_CONTENT_WIDTH: f32 = DOCK_WIDTH - 26.0;

impl Shell {
    /// Draws the CS6 right dock: the column of stacked panels.
    ///
    /// `Hide Panels` (the CS6 Tab key) folds the whole dock; the
    /// Window menu toggles one panel at a time, and a panel closed
    /// with its own close button leaves the rest of the dock alone.
    ///
    /// Returns the column's inner rect — the drop target a dragged
    /// panel is released over.
    pub(crate) fn dock(&mut self, ui: &mut egui::Ui) -> Option<egui::Rect> {
        if self.panels_hidden {
            return None;
        }
        self.dock_column(ui, DockSide::Right)
    }

    /// The left panel column — the second dock a panel can live in.
    ///
    /// It exists only when a panel is docked left or a panel is being
    /// dragged: a drop target has to be there before the drag
    /// reaches it, or there would be nowhere to drop the panel.
    pub(crate) fn left_dock(&mut self, ui: &mut egui::Ui) -> Option<egui::Rect> {
        if self.panels_hidden {
            return None;
        }
        let dragging = egui::DragAndDrop::has_payload_of_type::<PanelDrag>(ui.ctx());
        if !dragging && self.panels_in_dock(DockSide::Left).is_empty() {
            return None;
        }
        self.dock_column(ui, DockSide::Left)
    }

    /// One dock column: the panels that live in it, stacked.
    fn dock_column(&mut self, ui: &mut egui::Ui, side: DockSide) -> Option<egui::Rect> {
        let (column_id, scroll_id) = match side {
            DockSide::Left => ("cs6_left_dock", "cs6_left_dock_scroll"),
            DockSide::Right => ("cs6_dock", "cs6_dock_scroll"),
        };
        let column = match side {
            DockSide::Left => egui::Panel::left(egui::Id::new(column_id)),
            DockSide::Right => egui::Panel::right(egui::Id::new(column_id)),
        };
        let mut rect = None;
        column
            .resizable(true)
            .default_size(DOCK_WIDTH)
            .show_separator_line(true)
            .show(ui, |ui| {
                rect = Some(ui.max_rect());
                // The column scrolls: CS6's dock does too when the
                // stack is taller than the window, and it is what keeps
                // every panel reachable at a short viewport.
                egui::ScrollArea::vertical()
                    .id_salt(scroll_id)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for spec in PANELS {
                            if self.panel_shown(spec.name) && self.panel_home(spec.name) == side {
                                self.dock_section(ui, spec, true, side);
                            }
                        }
                        // The Web Layer panel is always present and
                        // carries no close button.
                        if self.panel_home(WEB_PANEL) == side {
                            self.dock_section(ui, &WEB_PANEL_SPEC, false, side);
                        }
                    });
            });
        rect
    }

    /// Resolves a finished panel drag: a panel released over a dock
    /// column moves into that column, and a release anywhere else
    /// cancels the move. Either way the payload is consumed — a
    /// payload left behind would keep the left column on screen
    /// forever.
    pub(crate) fn finish_panel_drag(
        &mut self,
        ui: &egui::Ui,
        right: Option<egui::Rect>,
        left: Option<egui::Rect>,
    ) {
        if !ui.input(|input| input.pointer.any_released()) {
            return;
        }
        let Some(drag) = egui::DragAndDrop::take_payload::<PanelDrag>(ui.ctx()) else {
            return;
        };
        let pointer = ui.ctx().pointer_interact_pos();
        let over = |column: Option<egui::Rect>| {
            pointer.is_some_and(|pos| column.is_some_and(|rect| rect.contains(pos)))
        };
        let target = if over(left) {
            Some(DockSide::Left)
        } else if over(right) {
            Some(DockSide::Right)
        } else {
            None
        };
        if let Some(side) = target {
            self.move_panel_to_dock(drag.name, side);
        }
    }

    /// Drops a panel drag in flight. Called when the docks are not
    /// drawn: a payload left behind would move a panel on a later
    /// release, long after the drag the user made.
    pub(crate) fn cancel_panel_drag(ctx: &egui::Context) {
        if egui::DragAndDrop::has_payload_of_type::<PanelDrag>(ctx) {
            egui::DragAndDrop::clear_payload(ctx);
        }
    }

    /// One panel in the dock: a CS6 header (collapse arrow, name,
    /// close button) over the panel's content.
    ///
    /// The header's expanded state is egui's own, keyed on the
    /// panel's name, so it survives a restart the way CS6's collapsed
    /// panels do.
    ///
    /// The panel's title is its drag handle — CS6 moves a panel by
    /// its header — and the small arrow beside the close button
    /// moves it to the other dock for anyone who cannot drag.
    fn dock_section(
        &mut self,
        ui: &mut egui::Ui,
        spec: &PanelSpec,
        closable: bool,
        side: DockSide,
    ) {
        let mut close = false;
        let mut move_to_other = false;
        let state = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            egui::Id::new(("cs6_dock_panel", spec.name)),
            true,
        );
        let header = state.show_header(ui, |ui| {
            let _drag = ui.dnd_drag_source(
                egui::Id::new(("cs6_panel_drag", spec.name)),
                PanelDrag { name: spec.name },
                |ui| {
                    ui.strong(spec.name);
                },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if closable {
                    close = ui.small_button("×").clicked();
                }
                move_to_other = ui
                    .small_button(match side {
                        DockSide::Left => "»",
                        DockSide::Right => "«",
                    })
                    .on_hover_text(match side {
                        DockSide::Left => "Move the panel to the right dock",
                        DockSide::Right => "Move the panel to the left dock",
                    })
                    .clicked();
            });
        });
        header.body_unindented(|ui| {
            ui.set_min_width(DOCK_CONTENT_WIDTH);
            match spec.kind {
                PanelKind::Properties => self.properties_panel(ui),
                PanelKind::Pages => self.pages_section(ui),
                PanelKind::States => self.states_section(ui),
                PanelKind::Layers => self.layers_section(ui),
                PanelKind::Styles => self.styles_panel(ui),
                PanelKind::Library => self.library_panel(ui),
                PanelKind::History => self.history_section(ui),
                PanelKind::ColorMixer => self.fill_stroke_panel(ui),
                PanelKind::Swatches => self.swatches_panel(ui),
                PanelKind::Info => self.info_panel(ui),
                PanelKind::AutoShapes => self.auto_shapes_panel(ui),
                PanelKind::Url => self.url_panel(ui),
                PanelKind::Behaviors => self.behaviors_panel(ui),
                PanelKind::WebLayer => self.web_panel(ui),
                PanelKind::Optimize => self.optimize_panel(ui),
                PanelKind::Slices => self.slices_panel(ui),
                PanelKind::HtmlPreview => self.html_preview_panel(ui),
                PanelKind::CssProperties => self.css_properties_panel(ui),
                PanelKind::FindReplace => self.find_replace_panel(ui),
                PanelKind::CommonLibrary => self.common_library_panel(ui),
            }
        });
        // The close button and the Window menu are the same switch.
        if close {
            self.hide_panel(spec.name);
        }
        if move_to_other {
            self.move_panel_to_dock(spec.name, side.other());
        }
    }

    /// The dock a panel lives in. A panel the user has not moved
    /// lives in the right stack, where CS6 keeps its panel group.
    pub(crate) fn panel_home(&self, name: &str) -> DockSide {
        self.panel_homes
            .iter()
            .find(|(panel, _)| *panel == name)
            .map_or(DOCK_SIDE, |(_, side)| *side)
    }

    /// Moves a panel into a dock. A panel moved back to the default
    /// dock drops out of the moved set, so the saved layout stays
    /// small and "not mentioned" keeps meaning "the default".
    pub(crate) fn move_panel_to_dock(&mut self, name: &'static str, side: DockSide) {
        self.panel_homes.retain(|(panel, _)| *panel != name);
        if side != DOCK_SIDE {
            self.panel_homes.push((name, side));
        }
    }

    /// The panels that live in a dock, in CS6's roster order.
    pub(crate) fn panels_in_dock(&self, side: DockSide) -> Vec<&'static str> {
        PANELS
            .iter()
            .filter(|spec| self.panel_home(spec.name) == side)
            .map(|spec| spec.name)
            .collect()
    }

    /// Records a panel as hidden (the Window menu's toggle uses the
    /// same list).
    fn hide_panel(&mut self, name: &'static str) {
        if let Some(pos) = self.panel_hidden.iter().position(|n| *n == name) {
            self.panel_hidden.remove(pos);
        }
        self.panel_hidden.push(name);
    }

    /// The panels CS6 opens on a fresh document — the initial value of
    /// the Window menu's visibility set.
    pub(crate) fn default_open_panels() -> Vec<&'static str> {
        PANELS
            .iter()
            .filter(|panel| !panel.open_by_default)
            .map(|panel| panel.name)
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The panel inventory carries the CS6 panel names the Window menu
    /// toggles, and every one of those names is a panel this shell
    /// can draw.
    #[test]
    fn the_inventory_covers_the_window_menu_panels() {
        let names: Vec<&str> = PANELS.iter().map(|panel| panel.name).collect();
        for menu_name in [
            "Properties",
            "Pages",
            "States",
            "Layers",
            "Styles",
            "Document Library",
            "History",
            "Color Mixer",
            "Swatches",
            "Info",
            "Auto Shapes",
            "URL",
            "Behaviors",
            "Optimize",
            "Slices",
            "HTML Preview",
            "CSS Properties",
            "Find and Replace",
            "Common Library",
        ] {
            assert!(names.contains(&menu_name), "{menu_name} has no panel");
        }
        // The Web Layer panel is separate from the toggleable set.
        assert!(!names.contains(&WEB_PANEL));
    }

    /// Panel names are unique — two panels sharing a title would make
    /// the Window menu's toggle ambiguous.
    #[test]
    fn the_inventory_has_unique_names() {
        let mut names: Vec<&str> = PANELS.iter().map(|panel| panel.name).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate panel names");
    }

    /// The default-open set matches CS6's fresh-document roster:
    /// Properties/Pages/States/Layers visible, the rest closed until
    /// asked for.
    ///
    /// The two web panels — Slices and HTML Preview — are closed by
    /// default too, and the Window menu carries a row for each, so
    /// both are reachable.
    #[test]
    fn the_default_open_set_is_the_cs6_roster() {
        let hidden = Shell::default_open_panels();
        assert_eq!(
            hidden,
            vec![
                "Styles",
                "Document Library",
                "History",
                "Color Mixer",
                "Swatches",
                "Info",
                "Auto Shapes",
                "URL",
                "Behaviors",
                "Optimize",
                "Slices",
                "HTML Preview",
                "CSS Properties",
                "Find and Replace",
                "Common Library"
            ]
        );
    }

    /// The dock is narrow enough to leave the canvas room and wide
    /// enough for CS6's panel content.
    #[test]
    fn the_dock_width_is_cs6s() {
        const { assert!(DOCK_WIDTH >= 240.0) };
        const { assert!(DOCK_WIDTH <= 300.0) };
    }
}
