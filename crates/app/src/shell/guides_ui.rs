//! The Edit Grid dialog and the guide drag-out draft. The grid and
//! guides themselves live in the document (page overrides over the
//! document grid); this module is only the surfaces that edit them.

use super::*;

/// A guide being dragged out from a canvas-edge press (CS6: press on
/// a ruler edge, drag onto the page). The draft paints a live line;
/// the release commits through `page.addGuide` or discards.
pub(crate) struct GuideDraft {
    /// `true` = horizontal guide (tracks the pointer's y).
    pub horizontal: bool,
    /// Live position in world coordinates.
    pub position: f64,
}

impl Shell {
    /// A canvas-edge press (within 8 px of the page's top or left
    /// edge) starts a guide draft — while guides are shown and
    /// unlocked. Returns the draft to install, `None` when the press
    /// belongs to the tool.
    pub(super) fn guide_draft_at(&self, world: geom::Point) -> Option<GuideDraft> {
        if !self.show_guides || self.guides_locked {
            return None;
        }
        let page = self.page_world()?;
        if world.y - page.y0 < 8.0 {
            Some(GuideDraft {
                horizontal: true,
                position: page.y0,
            })
        } else if world.x - page.x0 < 8.0 {
            Some(GuideDraft {
                horizontal: false,
                position: page.x0,
            })
        } else {
            None
        }
    }

    /// The nearest guide line to `world` within `tolerance` world px:
    /// each axis checks its guides independently, so one drag can ride
    /// both a horizontal and a vertical line at once.
    pub(super) fn snap_to_guides(&self, world: geom::Point, tolerance: f64) -> geom::Point {
        let Some(page) = self.editor.document().pages.get(self.editor.page) else {
            return world;
        };
        let near = |positions: &[f64], value: f64| {
            positions
                .iter()
                .copied()
                .min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs()))
                .filter(|p| (p - value).abs() <= tolerance)
        };
        geom::Point {
            x: near(&page.guides.vertical, world.x).unwrap_or(world.x),
            y: near(&page.guides.horizontal, world.y).unwrap_or(world.y),
        }
    }

    /// Commits the live draft: snap-to-grid folds in first, then one
    /// `page.addGuide`. Locked guides refuse at press time, but the
    /// release re-checks.
    pub(super) fn commit_guide_draft(&mut self) {
        let Some(draft) = self.guide_draft.take() else {
            return;
        };
        if self.guides_locked {
            return;
        }
        let mut position = draft.position;
        if self.snap_to_grid
            && let Some(page) = self.editor.document().pages.get(self.editor.page)
        {
            let grid = page
                .grid
                .clone()
                .unwrap_or_else(|| self.editor.document().grid.clone());
            position = if draft.horizontal {
                grid.offset.y + ((position - grid.offset.y) / grid.size.y).round() * grid.size.y
            } else {
                grid.offset.x + ((position - grid.offset.x) / grid.size.x).round() * grid.size.x
            };
        }
        let kind = if draft.horizontal {
            "horizontal"
        } else {
            "vertical"
        };
        let page = self.editor.page;
        let result = self.editor.apply(
            "page.addGuide",
            serde_json::json!({ "page": page, "position": position, "kind": kind }),
        );
        match result {
            Ok(()) => self.status = "guide added".to_owned(),
            Err(error) => self.status = format!("guide failed: {error}"),
        }
    }
}

/// The View ▸ Grid ▸ Edit Grid... dialog: origin and cell size of the
/// page's grid override, applied through `page.setGridOrigin` /
/// `page.setGridSize` as one transaction.
#[derive(Clone)]
pub(crate) struct GridDialog {
    /// Grid origin, world coordinates.
    pub origin_x: f64,
    pub origin_y: f64,
    /// Cell size, world units.
    pub size_x: f64,
    pub size_y: f64,
}

impl GridDialog {
    /// Seeds from the grid the page currently renders with (its own
    /// override, else the document grid).
    pub(crate) fn from_document(document: &doc::Document) -> Self {
        let grid = document
            .pages
            .first()
            .and_then(|page| page.grid.as_ref())
            .unwrap_or(&document.grid);
        Self {
            origin_x: grid.offset.x,
            origin_y: grid.offset.y,
            size_x: grid.size.x,
            size_y: grid.size.y,
        }
    }
}

impl Shell {
    /// The Edit Grid dialog body. OK writes both fields through the
    /// engine commands (invalid sizes are refused onto the status
    /// line, dialog stays open); Cancel discards.
    pub(super) fn grid_dialog_ui(&mut self, ui: &mut egui::Ui) {
        let Some(mut dialog) = self.grid_dialog.take() else {
            return;
        };
        egui::Grid::new("grid_dialog")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.add(egui::DragValue::new(&mut dialog.origin_x));
                ui.label("Origin X");
                ui.add(egui::DragValue::new(&mut dialog.origin_y));
                ui.label("Origin Y");
                ui.add(
                    egui::DragValue::new(&mut dialog.size_x)
                        .range(1.0..=10_000.0)
                        .suffix(" px"),
                );
                ui.label("Cell Width");
                ui.add(
                    egui::DragValue::new(&mut dialog.size_y)
                        .range(1.0..=10_000.0)
                        .suffix(" px"),
                );
                ui.label("Cell Height");
            });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("OK").clicked() {
                let page = self.editor.page;
                let result = self
                    .editor
                    .session
                    .begin_transaction("Edit Grid")
                    .and_then(|()| {
                        let first = self.editor.apply(
                            "page.setGridOrigin",
                            serde_json::json!({
                                "page": page,
                                "x": dialog.origin_x, "y": dialog.origin_y,
                            }),
                        );
                        first.and_then(|()| {
                            self.editor.apply(
                                "page.setGridSize",
                                serde_json::json!({
                                    "page": page,
                                    "x": dialog.size_x, "y": dialog.size_y,
                                }),
                            )
                        })
                    });
                match result {
                    Ok(()) => {
                        let _ = self.editor.session.commit_transaction();
                        self.status = "grid updated".to_owned();
                        self.grid_dialog = None;
                    }
                    Err(error) => {
                        let _ = self.editor.session.abort_transaction();
                        self.status = format!("grid update failed: {error}");
                        self.grid_dialog = Some(dialog.clone());
                    }
                }
            }
        });
        if self.grid_dialog.is_none() {
            return;
        }
        self.grid_dialog = Some(dialog);
    }
}
