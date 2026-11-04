//! CS6's panel system: each panel is its own window, as in the
//! reference application — draggable, collapsible, resizable, and
//! closable, with the Window menu as the second way in.
//!
//! CS6 keeps a *group* of docked panels along the right edge and lets
//! them float free. egui has no dock system, so a panel is an
//! [`egui::Window`] anchored in a tidy right-hand column: the
//! placement reads as a dock until the user drags a panel away, which
//! is the same affordance CS6 offers.
//!
//! Panel *content* lives in [`super::panels`]; this module owns the
//! inventory, the placement, and the open/closed bookkeeping.

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
];

/// The Web Layer panel has no Window-menu row in CS6 — it is always
/// present, so it is listed separately.
pub(crate) const WEB_PANEL: &str = "Web Layer";

/// One panel of the CS6 panel inventory.
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
}

/// The vertical step between the anchored panels, in points — the
/// height each dock window is opened at.
const DOCK_STEP: f32 = 260.0;

impl Shell {
    /// Draws the panel windows.
    ///
    /// `Hide Panels` (the CS6 Tab key) folds the whole group; the
    /// Window menu toggles one panel at a time, and a panel closed
    /// with its own close button leaves the group the same way.
    pub(crate) fn panel_windows(&mut self, ctx: &egui::Context) {
        if self.panels_hidden {
            return;
        }
        for (index, spec) in PANELS.iter().enumerate() {
            self.one_panel_window(ctx, spec, index);
        }
        // The Web Layer panel has no Window-menu row, so it carries no
        // close button — nothing could reopen it.
        egui::Window::new(WEB_PANEL)
            .collapsible(true)
            .resizable(true)
            .default_width(250.0)
            .default_height(240.0)
            .anchor(
                egui::Align2::RIGHT_TOP,
                egui::vec2(-8.0, 8.0 + DOCK_STEP * PANELS.len() as f32),
            )
            .show(ctx, |ui| {
                ui.set_min_height(180.0);
                self.web_panel(ui);
            });
    }

    /// One panel window, anchored in the right-hand dock column.
    fn one_panel_window(&mut self, ctx: &egui::Context, spec: &PanelSpec, index: usize) {
        let mut open = self.panel_shown(spec.name);
        egui::Window::new(spec.name)
            .open(&mut open)
            .collapsible(true)
            .resizable(true)
            .default_width(250.0)
            .default_height(240.0)
            .anchor(
                egui::Align2::RIGHT_TOP,
                egui::vec2(-8.0, 8.0 + DOCK_STEP * index as f32),
            )
            .show(ctx, |ui| {
                ui.set_min_height(180.0);
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
                }
            });
        // The close button and the Window menu are the same switch.
        if !open {
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
                "Info"
            ]
        );
    }
}
