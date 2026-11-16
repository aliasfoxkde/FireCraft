//! CS6's panel system: one dock column of stacked, collapsible panels
//! along the right edge, with the Window menu as the second way in.
//!
//! CS6 keeps a *group* of docked panels along the right edge, each
//! with its own header, stacked one above the next; the column
//! scrolls when the stack is taller than the window. egui has no dock
//! system, so the column is a [`egui::SidePanel`] on the right: the
//! panels are stacked inside it in CS6's order, each under a CS6
//! header row.
//!
//! The CS6 header row carries four affordances, left to right: the
//! collapse arrow, the panel title, the overflow (CS6's panel menu —
//! the triangle button in the panel's title bar), and the close
//! button. CS6 closes the Web Layer panel through no button at all —
//! it has no Window-menu row, so nothing could reopen it — and
//! `FireCraft` keeps that asymmetry. The overflow menu carries the two
//! dock-level commands the CS6 help documents (the double arrow at
//! the top of the dock collapses or expands every panel in the
//! column; *Hide Panels* folds the dock away); the exact item list
//! of a CS6 panel menu is not established by our evidence, so the
//! menu carries only the documented ones.
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

/// A command from a panel's CS6 panel menu (the overflow button in
/// the header row).
///
/// The commands are the dock-level ones the CS6 help documents: the
/// double arrow at the top of the dock collapses or expands every
/// panel icon in the column, and *Hide Panels* folds the dock away.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PanelMenu {
    /// The double arrow: collapse every panel in the column.
    CollapseAll,
    /// The double arrow the other way: expand every panel.
    ExpandAll,
    /// CS6's *Hide Panels* — the Tab key.
    HidePanels,
}

/// Which edge the panel column docks to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

/// The edge the panel column docks to. CS6 keeps the group on the
/// right; the panel-snapshot gate pins the choice, so moving the
/// column is a deliberate re-pin, never a silent drift.
const DOCK_SIDE: DockSide = DockSide::Right;

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
    /// The affordances the panel's CS6 header row carries, left to
    /// right.
    ///
    /// Pinned by the panel-snapshot gate so a header that loses its
    /// overflow button — or a close button on the one panel CS6
    /// leaves unclosable — fails the build instead of drifting.
    pub affordances: Vec<&'static str>,
}

/// The CS6 header row's affordances, left to right.
///
/// Every panel carries the collapse arrow, its title and the overflow
/// (the panel menu). The close button belongs to the panels the
/// Window menu can reopen; the Web Layer panel has no Window-menu
/// row, so nothing could reopen it, and CS6 gives it no close button
/// either.
fn header_affordances(spec: &PanelSpec) -> Vec<&'static str> {
    let mut affordances = vec!["collapse", "title", "overflow"];
    if spec.kind != PanelKind::WebLayer {
        affordances.push("close");
    }
    affordances
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
            affordances: header_affordances(spec),
        })
        .collect();
    panels.push(PanelSummary {
        id: panel_id(WEB_PANEL),
        header: WEB_PANEL.to_owned(),
        side: DOCK_SIDE,
        order: PANELS.len(),
        visible_by_default: true,
        toggleable: false,
        affordances: header_affordances(&WEB_PANEL_SPEC),
    });
    panels
}

/// The machine-facing id of a panel: its CS6 name, lowercased with
/// the spaces turned into dashes.
fn panel_id(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
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
    pub(crate) fn dock(&mut self, ui: &mut egui::Ui) {
        if self.panels_hidden {
            return;
        }
        egui::Panel::right(egui::Id::new("cs6_dock"))
            .resizable(true)
            .default_size(DOCK_WIDTH)
            .show_separator_line(true)
            .show(ui, |ui| {
                // The column scrolls: CS6's dock does too when the
                // stack is taller than the window, and it is what keeps
                // every panel reachable at a short viewport.
                egui::ScrollArea::vertical()
                    .id_salt("cs6_dock_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for spec in PANELS {
                            if self.panel_shown(spec.name) {
                                self.dock_section(ui, spec, spec.kind != PanelKind::WebLayer);
                            }
                        }
                        // The Web Layer panel is always present and
                        // carries no close button.
                        self.dock_section(ui, &WEB_PANEL_SPEC, false);
                    });
            });
    }

    /// One panel in the dock: the CS6 header row (collapse arrow,
    /// title, overflow, close) over the panel's content.
    ///
    /// The header's expanded state is egui's own, keyed on the
    /// panel's name, so it survives a restart the way CS6's collapsed
    /// panels do.
    fn dock_section(&mut self, ui: &mut egui::Ui, spec: &PanelSpec, closable: bool) {
        let mut close = false;
        let mut choice: Option<PanelMenu> = None;
        let ctx = ui.ctx().clone();
        let state = egui::collapsing_header::CollapsingState::load_with_default_open(
            &ctx,
            egui::Id::new(("cs6_dock_panel", spec.name)),
            true,
        );
        let header = state.show_header(ui, |ui| {
            ui.strong(spec.name);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if closable {
                    close = ui.small_button("×").clicked();
                }
                // CS6's panel menu — the triangle button in the panel's
                // title bar.
                ui.menu_button("▾", |ui| {
                    ui.selectable_value(
                        &mut choice,
                        Some(PanelMenu::CollapseAll),
                        "Collapse all panels",
                    );
                    ui.selectable_value(
                        &mut choice,
                        Some(PanelMenu::ExpandAll),
                        "Expand all panels",
                    );
                    ui.separator();
                    ui.selectable_value(&mut choice, Some(PanelMenu::HidePanels), "Hide panels");
                });
            });
        });
        // The overflow menu's commands act on the whole dock, so they
        // run after the header is laid out.
        match choice {
            Some(PanelMenu::CollapseAll) => set_every_panel_open(&ctx, false),
            Some(PanelMenu::ExpandAll) => set_every_panel_open(&ctx, true),
            Some(PanelMenu::HidePanels) => self.toggle_all_panels(),
            None => {}
        }
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

/// CS6's double arrow at the top of the dock: collapse (or expand)
/// every panel in the column at once.
///
/// The state lives in egui's own storage, keyed on the panel's name,
/// so this writes the same place the header arrow writes.
fn set_every_panel_open(ctx: &egui::Context, open: bool) {
    for spec in every_panel() {
        let id = egui::Id::new(("cs6_dock_panel", spec.name));
        let mut state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ctx, id, open);
        state.set_open(open);
        state.store(ctx);
    }
}

/// The panels whose header row the dock draws, in order: the
/// Window-menu panels and then the always-present Web Layer panel.
fn every_panel() -> impl Iterator<Item = &'static PanelSpec> {
    PANELS.iter().chain(std::iter::once(&WEB_PANEL_SPEC))
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

    /// Every panel's CS6 header row carries the collapse arrow, the
    /// title and the overflow — and every panel the Window menu can
    /// reopen also carries the close button.
    #[test]
    fn the_header_row_carries_cs6s_affordances() {
        for panel in panel_inventory() {
            assert_eq!(
                &panel.affordances[..3],
                &["collapse", "title", "overflow"],
                "{} loses a CS6 header affordance",
                panel.id
            );
            if panel.toggleable {
                assert_eq!(
                    panel.affordances.last(),
                    Some(&"close"),
                    "{} has no close button",
                    panel.id
                );
            } else {
                assert_eq!(
                    panel.affordances.last(),
                    Some(&"overflow"),
                    "{} carries a close button it cannot reopen",
                    panel.id
                );
            }
        }
    }

    /// The Web Layer panel is the one CS6 panel with no close button:
    /// it has no Window-menu row, so nothing could reopen it.
    #[test]
    fn the_web_layer_panel_has_no_close_button() {
        let web = panel_inventory()
            .into_iter()
            .find(|panel| panel.id == "web-layer")
            .expect("the Web Layer panel is in the inventory");
        assert!(!web.toggleable);
        assert!(!web.affordances.contains(&"close"));
    }

    /// CS6's double arrow at the top of the dock: every panel in the
    /// column collapses (and comes back) at once.
    #[test]
    fn the_double_arrow_collapses_every_panel() {
        let ctx = egui::Context::default();
        for spec in every_panel() {
            let id = egui::Id::new(("cs6_dock_panel", spec.name));
            let mut state =
                egui::collapsing_header::CollapsingState::load_with_default_open(&ctx, id, true);
            state.set_open(true);
            state.store(&ctx);
        }

        set_every_panel_open(&ctx, false);
        for spec in every_panel() {
            let id = egui::Id::new(("cs6_dock_panel", spec.name));
            let state = egui::collapsing_header::CollapsingState::load(&ctx, id)
                .expect("the panel's stored state");
            assert!(!state.is_open(), "{} stayed open", spec.name);
        }

        set_every_panel_open(&ctx, true);
        for spec in every_panel() {
            let id = egui::Id::new(("cs6_dock_panel", spec.name));
            let state = egui::collapsing_header::CollapsingState::load(&ctx, id)
                .expect("the panel's stored state");
            assert!(state.is_open(), "{} stayed collapsed", spec.name);
        }
    }
}
