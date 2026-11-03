//! The canvas and tool surfaces: tool strip, options bar, the main
//! canvas widget, and the nine-slice overlay geometry.
//!
//! Pure egui drawing glue: exercised visually (and later by the
//! `egui_kittest` snapshot gate), not by line coverage.

use super::*;

impl Shell {
    /// World-space page bounds — the Fit All target.
    pub(super) fn page_world(&self) -> Option<geom::Rect> {
        self.editor
            .document()
            .pages
            .get(self.editor.page)
            .map(|page| {
                geom::Rect::new(
                    page.info.canvas.x,
                    page.info.canvas.y,
                    page.info.canvas.x + page.info.canvas.width,
                    page.info.canvas.y + page.info.canvas.height,
                )
            })
    }

    /// The CS6 tool strip: vertical, grouped, glyph buttons painted
    /// with `Painter` primitives (original art, never the Adobe icons).
    pub(super) fn tools_panel(&mut self, ui: &mut egui::Ui) {
        let groups: &[&[Tool]] = &[
            &[Tool::Pointer, Tool::Subselect],
            &[Tool::Rect, Tool::Ellipse, Tool::Line, Tool::Text],
            &[
                Tool::Paintbrush,
                Tool::Pencil,
                Tool::Eraser,
                Tool::PaintBucket,
                Tool::Gradient,
                Tool::Eyedropper,
            ],
            &[
                Tool::Marquee,
                Tool::EllipticalMarquee,
                Tool::Lasso,
                Tool::PolygonalMarquee,
                Tool::MagicWand,
            ],
            &[Tool::Slice, Tool::Hotspot],
            &[Tool::Zoom, Tool::Hand],
        ];
        ui.add_space(4.0);
        for (index, group) in groups.iter().enumerate() {
            if index > 0 {
                ui.separator();
            }
            for tool in *group {
                self.tool_button(ui, *tool);
            }
        }
        ui.separator();
        self.autoshape_button(ui);
    }

    /// One painted tool button; clicking picks the tool.
    pub(super) fn tool_button(&mut self, ui: &mut egui::Ui, tool: Tool) {
        let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(26.0), Sense::click());
        let active = self.editor.tool == tool;
        if response.clicked() {
            self.editor.tool = tool;
            self.status = format!("tool: {}", tool_label(tool));
        }
        // CS6's tool-button double-clicks: Zoom resets to 100%, Hand
        // fits the page in the window.
        if response.double_clicked() {
            match tool {
                Tool::Zoom => self.run_shell(menus::ShellAction::ZoomTo(1.0)),
                Tool::Hand => self.run_shell(menus::ShellAction::FitAll),
                _ => {}
            }
        }
        let painter = ui.painter();
        if active {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x2f, 0x5d, 0x8a));
        } else if response.hovered() {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
        }
        let ink = if active {
            Color32::from_rgb(0xe8, 0xe8, 0xe8)
        } else {
            Color32::from_rgb(0xc4, 0xc4, 0xc4)
        };
        icons::paint(ui, rect, icons::tool_icon(tool), 17.0, ink);
        let key = tool_key(tool);
        if key.is_empty() {
            response.on_hover_text(tool_label(tool));
        } else {
            response.on_hover_text(format!("{} ({key})", tool_label(tool)));
        }
    }

    /// The Auto Shape flyout: the star button toggles a popup listing
    /// every shape with its preview glyph; picking one selects the
    /// tool and closes the flyout.
    pub(super) fn autoshape_button(&mut self, ui: &mut egui::Ui) {
        let shapes = AutoShape::ALL;
        let current = match self.editor.tool {
            Tool::AutoShape(shape) => Some(shape),
            _ => None,
        };
        let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(26.0), Sense::click());
        let mut open = self.autoshape_flyout;
        if response.clicked() {
            open = !open;
            if open && current.is_none() {
                self.editor.tool = Tool::AutoShape(shapes[0]);
            }
        }
        let painter = ui.painter();
        let active = current.is_some();
        if active {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x2f, 0x5d, 0x8a));
        } else if response.hovered() {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
        }
        let ink = if active {
            Color32::from_rgb(0xe8, 0xe8, 0xe8)
        } else {
            Color32::from_rgb(0xc4, 0xc4, 0xc4)
        };
        let shown = current.unwrap_or(shapes[0]);
        icons::paint(
            ui,
            rect,
            icons::tool_icon(Tool::AutoShape(shown)),
            17.0,
            ink,
        );
        let tip = match current {
            Some(shape) => format!("Auto Shape — {}", shape.label()),
            None => "Auto Shape".to_owned(),
        };
        let response = response.on_hover_text(tip);

        let mut picked: Option<AutoShape> = None;
        egui::Popup::from_response(&response)
            .open_bool(&mut open)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                for shape in shapes {
                    let (row, row_response) =
                        ui.allocate_exact_size(egui::Vec2::new(150.0, 20.0), Sense::click());
                    let selected = current == Some(shape);
                    let rp = ui.painter();
                    if selected {
                        rp.rect_filled(row, 2.0, Color32::from_rgb(0x2f, 0x5d, 0x8a));
                    } else if row_response.hovered() {
                        rp.rect_filled(row, 2.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
                    }
                    let ink = if selected || row_response.hovered() {
                        Color32::from_rgb(0xe8, 0xe8, 0xe8)
                    } else {
                        Color32::from_rgb(0xdd, 0xdd, 0xdd)
                    };
                    draw_autoshape_glyph(
                        rp,
                        Rect::from_min_size(
                            Pos2::new(row.left() + 4.0, row.center().y - 8.0),
                            egui::Vec2::splat(16.0),
                        ),
                        shape,
                        ink,
                    );
                    rp.text(
                        Pos2::new(row.left() + 26.0, row.center().y),
                        egui::Align2::LEFT_CENTER,
                        shape.label(),
                        egui::FontId::proportional(12.0),
                        ink,
                    );
                    if row_response.clicked() {
                        picked = Some(shape);
                    }
                }
            });
        self.autoshape_flyout = open;
        if let Some(shape) = picked {
            self.editor.tool = Tool::AutoShape(shape);
            self.autoshape_flyout = false;
            self.status = format!("tool: Auto Shape — {}", shape.label());
        }
    }

    /// Canvas-local screen position of a world point.
    pub(super) fn screen_of(&self, world: geom::Point) -> Pos2 {
        let screen = self.view.to_screen(world);
        Pos2::new(screen.x, screen.y)
    }

    /// The entered definition's nine-slice guides in canvas-local
    /// space: the shape bounds plus one drag handle per edge, at the
    /// current insets. `None` when not editing a guided symbol.
    pub(super) fn nine_slice_overlay(&self) -> Option<(Rect, [(Rect, NineSliceEdge); 4])> {
        let guides = self.editor.editing_definition()?.nine_slice.clone()?;
        let bounds = guides.shape_bounds;
        let screen = Rect::from_min_max(
            self.screen_of(geom::Point::new(bounds.x0, bounds.y0)),
            self.screen_of(geom::Point::new(bounds.x1, bounds.y1)),
        );
        let [top, bottom, left, right] = guides.controls.map(|value| value as f32);
        let (mid_x, mid_y) = (screen.center().x, screen.center().y);
        let handle = |x: f32, y: f32| Rect::from_center_size(Pos2::new(x, y), Vec2::splat(10.0));
        Some((
            screen,
            [
                (handle(mid_x, screen.top() + top), NineSliceEdge::Top),
                (
                    handle(mid_x, screen.bottom() - bottom),
                    NineSliceEdge::Bottom,
                ),
                (handle(screen.left() + left, mid_y), NineSliceEdge::Left),
                (handle(screen.right() - right, mid_y), NineSliceEdge::Right),
            ],
        ))
    }

    /// The nine-slice guide edge under a canvas-local point, if any.
    pub(super) fn nine_slice_edge_at(&self, local: Pos2) -> Option<NineSliceEdge> {
        let (_, handles) = self.nine_slice_overlay()?;
        handles
            .iter()
            .find(|(handle, _)| handle.contains(local))
            .map(|(_, edge)| *edge)
    }

    /// The canvas: compose renders the page, egui shows the texture,
    /// tools drive the editor through world-space input.
    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        let (available, response_rect) = {
            let size = ui.available_size();
            let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
            (size, (rect, response))
        };
        let (rect, response) = response_rect;
        // Remembered for view commands that arrive outside the canvas
        // (menu zoom/fit, keyboard shortcuts) — the anchor is the
        // canvas center of the last laid-out frame.
        self.canvas_rect = Some(rect);

        // Per-tool cursor icons (CS6 semantics); space held anywhere
        // switches to the temporary Hand (the CS6 space-drag pan). While
        // a pan drag is actively moving, the cursor closes (Grabbing).
        let space_held = ui.input(|input| input.key_down(egui::Key::Space));
        let pan_dragging = space_held && response.dragged();
        let cursor = if pan_dragging {
            egui::CursorIcon::Grabbing
        } else if space_held {
            egui::CursorIcon::Grab
        } else {
            match self.editor.tool {
                Tool::Pointer | Tool::Subselect => egui::CursorIcon::Default,
                Tool::Rect | Tool::Ellipse | Tool::Line | Tool::Slice | Tool::Hotspot => {
                    egui::CursorIcon::Crosshair
                }
                Tool::Text => egui::CursorIcon::Text,
                Tool::Zoom => egui::CursorIcon::ZoomIn,
                Tool::Hand => egui::CursorIcon::Grab,
                Tool::AutoShape(_) => egui::CursorIcon::Crosshair,
                Tool::Paintbrush
                | Tool::Pencil
                | Tool::Eraser
                | Tool::PaintBucket
                | Tool::Gradient
                | Tool::Eyedropper => egui::CursorIcon::Crosshair,
                Tool::Marquee
                | Tool::EllipticalMarquee
                | Tool::Lasso
                | Tool::PolygonalMarquee
                | Tool::MagicWand => egui::CursorIcon::Crosshair,
            }
        };
        let response = response.on_hover_cursor(cursor);

        // First fit on a fresh raster keeps the document visible.
        if self.raster.is_none() {
            if let Some(page) = self.editor.document().pages.get(self.editor.page) {
                let world = geom::Rect::new(
                    page.info.canvas.x,
                    page.info.canvas.y,
                    page.info.canvas.x + page.info.canvas.width,
                    page.info.canvas.y + page.info.canvas.height,
                );
                if world.width() > 0.0 && world.height() > 0.0 {
                    self.view.fit(world, available);
                }
            }
        }

        // Raster cache: re-render when the session changed or when the
        // zoom moved off the cached render scale. The raster renders
        // at a multiple of the document resolution so the view blits
        // it near 1:1 instead of linear-filtering a 1× raster (the
        // always-1× blur). Scale clamps to a 16 MPix budget so deep
        // zoom on a large page stays sane.
        let page = self
            .editor
            .document()
            .pages
            .get(self.editor.page)
            .map(|p| (p.info.canvas.width.round(), p.info.canvas.height.round()));
        let desired_scale = {
            let (pw, ph) = page.unwrap_or((1.0, 1.0));
            let zoom = f64::from(self.view.zoom);
            let budget = 16.0 * 1024.0 * 1024.0;
            let max_scale = (budget / (pw * ph).max(1.0)).sqrt();
            zoom.min(max_scale.max(1.0))
        };
        // Between buckets (within 2× of the cached scale) the existing
        // texture stays crisp enough; a crossing re-renders.
        let stale_scale =
            (desired_scale / self.raster_scale) > 2.0 || (self.raster_scale / desired_scale) > 2.0;
        let stamp = self.stamp();
        let texture = match &self.raster {
            Some((texture, made)) if *made == stamp && !stale_scale => texture.clone(),
            _ => {
                let result = if (desired_scale - 1.0).abs() < 0.05 {
                    compose::render(self.editor.document(), self.editor.page, self.editor.frame)
                } else {
                    compose::render_scaled(
                        self.editor.document(),
                        self.editor.page,
                        self.editor.frame,
                        desired_scale,
                    )
                };
                match result {
                    Ok(rendered) => {
                        let image = ColorImage::from_rgba_premultiplied(
                            [rendered.width as usize, rendered.height as usize],
                            &rendered.pixels,
                        );
                        let texture = ui.ctx().load_texture("page", image, TextureOptions::LINEAR);
                        self.raster = Some((texture.clone(), stamp));
                        self.raster_scale = if (desired_scale - 1.0).abs() < 0.05 {
                            1.0
                        } else {
                            desired_scale
                        };
                        texture
                    }
                    Err(error) => {
                        ui.painter()
                            .rect_filled(rect, 0.0, Color32::from_rgb(24, 24, 24));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("cannot compose: {error}"),
                            egui::FontId::proportional(14.0),
                            Color32::from_rgb(200, 120, 120),
                        );
                        return;
                    }
                }
            }
        };

        // Checker-dark backdrop, then the page texture through the view.
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from_rgb(32, 32, 32));
        // The raster may have been rendered at `raster_scale`, so its
        // world size is texture_size / scale, not texture_size.
        let world_w = texture.size()[0] as f64 / self.raster_scale;
        let world_h = texture.size()[1] as f64 / self.raster_scale;
        let top_left = self.view.to_screen(geom::Point::new(0.0, 0.0));
        let size = self.view.vec_to_screen(geom::Vec2::new(world_w, world_h));
        let image_rect = Rect::from_min_size(
            Pos2::new(rect.left() + top_left.x, rect.top() + top_left.y),
            Vec2::new(size.x.max(1.0), size.y.max(1.0)),
        );
        // CS6 pasteboard: the page floats on gray with a soft shadow,
        // so the document edge is visible while panning.
        let shadow = Rect::from_min_size(image_rect.min + Vec2::new(4.0, 6.0), image_rect.size());
        ui.painter()
            .rect_filled(shadow, 6.0, Color32::from_rgb(16, 16, 16));
        ui.painter().image(
            texture.id(),
            image_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );

        // Nine-slice guide overlay while a guided definition is
        // entered: shape bounds, the four guide lines, one handle per
        // edge. Overlay geometry is canvas-local; shift by the widget
        // origin to paint.
        if let Some((bounds, handles)) = self.nine_slice_overlay() {
            let color = Color32::from_rgb(90, 200, 250);
            let shift = egui::Vec2::new(rect.left(), rect.top());
            let bounds = bounds.translate(shift);
            let painter = ui.painter();
            painter.rect_stroke(bounds, 0.0, (1.0, color), egui::StrokeKind::Outside);
            let handles = handles.map(|(handle, edge)| (handle.translate(shift), edge));
            let [(top, _), (bottom, _), (left, _), (right, _)] = handles;
            painter.line_segment(
                [
                    Pos2::new(bounds.left(), top.center().y),
                    Pos2::new(bounds.right(), top.center().y),
                ],
                (1.0, color),
            );
            painter.line_segment(
                [
                    Pos2::new(bounds.left(), bottom.center().y),
                    Pos2::new(bounds.right(), bottom.center().y),
                ],
                (1.0, color),
            );
            painter.line_segment(
                [
                    Pos2::new(left.center().x, bounds.top()),
                    Pos2::new(left.center().x, bounds.bottom()),
                ],
                (1.0, color),
            );
            painter.line_segment(
                [
                    Pos2::new(right.center().x, bounds.top()),
                    Pos2::new(right.center().x, bounds.bottom()),
                ],
                (1.0, color),
            );
            for (handle, _) in handles {
                painter.rect_filled(handle, 2.0, color);
            }
        }

        // Marching ants: the pixel selection's boundary, drawn over
        // the page. The in-progress marquee paints a dashed preview
        // band; the committed mask paints from `selection_edges`
        // (the image border is not a boundary).
        {
            const ANTS: Color32 = Color32::from_rgb(0, 0, 0);
            let to_canvas = |world: geom::Point| {
                let screen = self.view.to_screen(world);
                Pos2::new(rect.left() + screen.x, rect.top() + screen.y)
            };
            let painter = ui.painter();
            // The drag preview: the marquee's live frame, offset by
            // the target image's location.
            if let Some(selection) = self.editor.pixel_selection.as_ref() {
                let location = self.editor.image_location(selection.target);
                let offset = location.unwrap_or_default();
                let a = to_canvas(geom::Point::new(
                    offset.x + selection.start.x.min(selection.current.x),
                    offset.y + selection.start.y.min(selection.current.y),
                ));
                let b = to_canvas(geom::Point::new(
                    offset.x + selection.start.x.max(selection.current.x),
                    offset.y + selection.start.y.max(selection.current.y),
                ));
                let band = Rect::from_two_pos(a, b);
                for shape in egui::Shape::dashed_line(
                    &[
                        band.left_top(),
                        band.right_top(),
                        band.right_bottom(),
                        band.left_bottom(),
                        band.left_top(),
                    ],
                    (1.0, ANTS),
                    4.0,
                    3.0,
                ) {
                    painter.add(shape);
                }
            }
            // The committed selection's boundary.
            if let Some(edges) = self.editor.selection_outline() {
                for [x0, y0, x1, y1] in edges {
                    painter.line_segment(
                        [
                            to_canvas(geom::Point::new(x0, y0)),
                            to_canvas(geom::Point::new(x1, y1)),
                        ],
                        (1.0, ANTS),
                    );
                }
            }
        }

        // Selection affordances: CS6 puts one bounding box with corner
        // and edge handles around the whole selection — the frame is
        // also the transform gesture's geometry, since a press on a
        // handle starts a live scale (see the drag_started routing).
        let union_frame = {
            let ids = self.editor.live_selection();
            let frame =
                crate::editor::union_selection_bounds(self.editor.document(), &ids).map(|bounds| {
                    let shift = egui::Vec2::new(rect.left(), rect.top());
                    let a = self.view.to_screen(geom::Point::new(bounds.x0, bounds.y0)) + shift;
                    let b = self.view.to_screen(geom::Point::new(bounds.x1, bounds.y1)) + shift;
                    Rect::from_two_pos(a, b)
                });
            if let Some(frame) = frame {
                let painter = ui.painter();
                painter.rect_stroke(
                    frame,
                    0.0,
                    (1.0, Color32::from_rgb(120, 180, 255)),
                    egui::StrokeKind::Outside,
                );
                for handle in transform_handle_points(frame) {
                    let square = Rect::from_center_size(handle, Vec2::splat(7.0));
                    painter.rect_filled(square, 0.0, Color32::WHITE);
                    painter.rect_stroke(
                        square,
                        0.0,
                        (1.0, Color32::BLACK),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            frame
        };

        // Subselect over exactly one path: draw the anchors as filled
        // squares — the node-editing surface the drag grabs.
        if self.editor.tool == Tool::Subselect {
            let ids = self.editor.live_selection();
            if let [only] = ids.as_slice() {
                let document = self.editor.document();
                if let Some(data) = document.registry.get(*only)
                    && let ObjectData::Path(path) = data
                {
                    let shift = egui::Vec2::new(rect.left(), rect.top());
                    let painter = ui.painter();
                    for contour in &path.contours {
                        for point in &contour.points {
                            let screen = self.view.to_screen(point.anchor) + shift;
                            let square = Rect::from_center_size(screen, Vec2::splat(6.0));
                            painter.rect_filled(square, 1.0, Color32::from_rgb(0x2f, 0x5d, 0x8a));
                            painter.rect_stroke(
                                square,
                                0.0,
                                (1.0, Color32::WHITE),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                }
            }
        }

        // Gradient fill overlay: the ramp line with a square handle at
        // each end, over the selected path's gradient fill.
        let gradient_frame = self.editor.gradient_handles().map(|(_, start, end)| {
            let shift = egui::Vec2::new(rect.left(), rect.top());
            let a = self.view.to_screen(start) + shift;
            let b = self.view.to_screen(end) + shift;
            let painter = ui.painter();
            painter.line_segment([a, b], (1.0, Color32::from_rgb(0x2f, 0x5d, 0x8a)));
            for point in [a, b] {
                let square = Rect::from_center_size(point, Vec2::splat(9.0));
                painter.rect_filled(square, 2.0, Color32::WHITE);
                painter.rect_stroke(
                    square,
                    0.0,
                    (1.0, Color32::BLACK),
                    egui::StrokeKind::Outside,
                );
            }
            (a, b)
        });

        // Rubber band for a creation drag. A move drag is excluded:
        // the band belongs to shape creation, and painting it during a
        // move made every object drag look like a marquee selection.
        if let Some(drag) = &self.editor.drag
            && drag.moving.is_none()
        {
            let a = self.view.to_screen(drag.start);
            let b = self.view.to_screen(drag.current);
            let band = Rect::from_two_pos(
                Pos2::new(rect.left() + a.x, rect.top() + a.y),
                Pos2::new(rect.left() + b.x, rect.top() + b.y),
            );
            ui.painter().rect_stroke(
                band,
                0.0,
                (1.0, Color32::from_rgb(120, 180, 255)),
                egui::StrokeKind::Outside,
            );
        }

        // Grid overlay: lines across the page at the effective cell
        // size, from the page's grid override or the document grid.
        if self.show_grid {
            let grid_page = self.editor.document().pages.get(self.editor.page);
            let grid = grid_page.map(|p| {
                p.grid
                    .clone()
                    .unwrap_or_else(|| self.editor.document().grid.clone())
            });
            if let (Some(page), Some(grid)) = (grid_page, grid)
                && grid.size.x > 0.0
                && grid.size.y > 0.0
            {
                let ink = Color32::from_rgb(70, 70, 70);
                let painter = ui.painter();
                let x0 = page.info.canvas.x;
                let y0 = page.info.canvas.y;
                let x1 = x0 + page.info.canvas.width;
                let y1 = y0 + page.info.canvas.height;
                let mut gx = grid.offset.x;
                while gx <= x1 {
                    if gx >= x0 {
                        let a = self.view.to_screen(geom::Point::new(gx, y0));
                        let b = self.view.to_screen(geom::Point::new(gx, y1));
                        painter.line_segment(
                            [
                                Pos2::new(rect.left() + a.x, rect.top() + a.y),
                                Pos2::new(rect.left() + b.x, rect.top() + b.y),
                            ],
                            (1.0, ink),
                        );
                    }
                    gx += grid.size.x;
                }
                let mut gy = grid.offset.y;
                while gy <= y1 {
                    if gy >= y0 {
                        let a = self.view.to_screen(geom::Point::new(x0, gy));
                        let b = self.view.to_screen(geom::Point::new(x1, gy));
                        painter.line_segment(
                            [
                                Pos2::new(rect.left() + a.x, rect.top() + a.y),
                                Pos2::new(rect.left() + b.x, rect.top() + b.y),
                            ],
                            (1.0, ink),
                        );
                    }
                    gy += grid.size.y;
                }
            }
        }

        // Guides overlay: the page's committed guide lines, plus the
        // in-progress drag-out draft in a brighter ink. Both clip to
        // the page rectangle.
        if self.show_guides {
            let guides = self
                .editor
                .document()
                .pages
                .get(self.editor.page)
                .map(|p| p.guides.clone());
            if let (Some(page), Some(guides)) =
                (self.editor.document().pages.get(self.editor.page), guides)
            {
                let ink = Color32::from_rgb(0x00, 0xb0, 0xf0);
                let painter = ui.painter();
                let x0 = page.info.canvas.x;
                let y0 = page.info.canvas.y;
                let x1 = x0 + page.info.canvas.width;
                let y1 = y0 + page.info.canvas.height;
                let line = |position: f64, horizontal: bool, color: Color32| {
                    let (a, b) = if horizontal {
                        let a = self.view.to_screen(geom::Point::new(x0, position));
                        let b = self.view.to_screen(geom::Point::new(x1, position));
                        (
                            Pos2::new(rect.left() + a.x, rect.top() + a.y),
                            Pos2::new(rect.left() + b.x, rect.top() + b.y),
                        )
                    } else {
                        let a = self.view.to_screen(geom::Point::new(position, y0));
                        let b = self.view.to_screen(geom::Point::new(position, y1));
                        (
                            Pos2::new(rect.left() + a.x, rect.top() + a.y),
                            Pos2::new(rect.left() + b.x, rect.top() + b.y),
                        )
                    };
                    painter.line_segment([a, b], (1.0, color));
                };
                for &y in &guides.horizontal {
                    line(y, true, ink);
                }
                for &x in &guides.vertical {
                    line(x, false, ink);
                }
                if let Some(draft) = &self.guide_draft {
                    line(
                        draft.position,
                        draft.horizontal,
                        Color32::from_rgb(0x60, 0xd0, 0xff),
                    );
                }
            }
        }

        // Input routing, by tool. Pointer coordinates are widget-local;
        // the view works in the same local space (top_left origin).
        // A press on a nine-slice guide handle drags that guide
        // instead of the tool.
        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.left_top().to_vec2();
                let world = self.view.to_world(local);
                match self.nine_slice_edge_at(local) {
                    Some(edge) => {
                        if let Some(symbol) = self.editor.symbol_scope {
                            if let Err(error) = self.editor.begin_guide_drag(symbol, edge, world) {
                                self.status = format!("guide drag failed: {error}");
                            }
                        }
                    }
                    None => {
                        // A press near the page's top/left edge drags
                        // out a guide (CS6 ruler-edge semantics); it
                        // outranks every tool.
                        if let Some(draft) = self.guide_draft_at(world) {
                            self.guide_draft = Some(draft);
                            return;
                        }
                        // A press on a gradient handle moves the ramp
                        // end; it outranks the transform handles
                        // (the gradient overlay paints above them).
                        let grabbed_gradient = gradient_frame.and_then(|(a, b)| {
                            if Rect::from_center_size(a, Vec2::splat(13.0)).contains(local) {
                                Some(GradientHandle::Start)
                            } else if Rect::from_center_size(b, Vec2::splat(13.0)).contains(local) {
                                Some(GradientHandle::End)
                            } else {
                                None
                            }
                        });
                        if let Some(handle) = grabbed_gradient
                            && let Some((id, _, _)) = self.editor.gradient_handles()
                        {
                            if let Err(error) = self.editor.begin_gradient_drag(id, handle, world) {
                                self.status = format!("gradient drag failed: {error}");
                            }
                            return;
                        }
                        // A press on a selection handle scales the
                        // selection live. It outranks the tool: the
                        // handle is an affordance of the current
                        // selection, not of the active tool.
                        let grabbed =
                            union_frame.and_then(|frame| transform_handle_at(frame, local));
                        if let Some(handle) = grabbed {
                            let ids = self.editor.live_selection();
                            if let Err(error) = self.editor.begin_transform_drag(world, handle, ids)
                            {
                                self.status = format!("transform failed: {error}");
                            }
                        } else if !space_held {
                            match self.editor.tool {
                                Tool::Paintbrush | Tool::Pencil | Tool::Eraser | Tool::Gradient => {
                                    // A paint stroke: the dabs land on
                                    // the image under the pointer.
                                    if let Err(error) = self.editor.begin_stroke(world) {
                                        self.status = format!("paint failed: {error}");
                                    }
                                }
                                Tool::Pointer | Tool::Subselect => {
                                    // A press selects its target first,
                                    // then moves the selection live —
                                    // or (Alt) clones it, CS6 pointer
                                    // semantics. Empty canvas starts a
                                    // (no-op) rubber band. Subselect
                                    // over one selected path grabs the
                                    // anchor under the press instead:
                                    // the node drag outranks the move.
                                    if self.editor.tool == Tool::Subselect
                                        && let Some(id) = self.editor.selected_live()
                                        && let Some((contour, point)) = self
                                            .editor
                                            .node_at(world, f64::from(6.0 / self.view.zoom))
                                        && let Err(error) =
                                            self.editor.begin_node_drag(id, contour, point, world)
                                    {
                                        self.status = format!("reshape failed: {error}");
                                    } else if let Some(hit) = self.editor.hit_at(world) {
                                        let ids = if self.editor.selection_set.contains(&hit) {
                                            self.editor.live_selection()
                                        } else {
                                            self.editor.selection = Some(hit);
                                            self.editor.selection_set = vec![hit];
                                            vec![hit]
                                        };
                                        let outcome = if ui.input(|i| i.modifiers.alt) {
                                            self.editor
                                                .begin_clone_drag(world, &ids)
                                                .map(|_created| ())
                                        } else {
                                            self.editor.begin_move_drag(world, ids)
                                        };
                                        if let Err(error) = outcome {
                                            self.status = format!("drag failed: {error}");
                                        }
                                    } else {
                                        self.editor.begin_drag(world);
                                    }
                                }
                                Tool::PaintBucket | Tool::Eyedropper => {
                                    // Click-only tools: the press
                                    // itself is the whole gesture.
                                }
                                Tool::Marquee
                                | Tool::EllipticalMarquee
                                | Tool::Lasso
                                | Tool::MagicWand => {
                                    // A pixel selection: the shape
                                    // lands on the image under the
                                    // pointer, folded by the
                                    // modifier keys.
                                    self.editor.pixel_mode =
                                        PixelSelectMode::from_modifiers(ui.input(|i| i.modifiers));
                                    if let Err(error) = self.editor.begin_pixel_selection(world) {
                                        self.status = format!("select failed: {error}");
                                    }
                                }
                                Tool::PolygonalMarquee => {
                                    // The polygon's vertices are
                                    // clicks, not a drag.
                                }
                                _ => self.editor.begin_drag(world),
                            }
                        }
                    }
                }
            }
        }
        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.left_top().to_vec2();
                let world = self.view.to_world(local);
                if self.guide_draft.is_some() {
                    if let Some(draft) = &mut self.guide_draft {
                        draft.position = if draft.horizontal { world.y } else { world.x };
                    }
                } else if self.editor.guide_drag.is_some() {
                    self.editor.drag_guide_to(world);
                } else if self.editor.transform_drag.is_some() {
                    self.editor.transform_drag_to(world);
                } else if self.editor.node_drag.is_some() {
                    self.editor.node_drag_to(world);
                } else if self.editor.gradient_drag.is_some() {
                    self.editor.gradient_drag_to(world);
                } else if self.editor.pixel_selection.is_some() {
                    if let Err(error) = self.editor.extend_pixel_selection(world) {
                        self.status = format!("select failed: {error}");
                    }
                } else if self.editor.stroke.is_some() {
                    if let Err(error) = self.editor.stroke_to(world) {
                        self.status = format!("paint failed: {error}");
                    }
                } else if self.editor.drag.is_some() {
                    let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));
                    // Snap-to-guides folds into the pointer position
                    // before the constraint matrix: a moving drag whose
                    // point lands within 4 world px of a guide line
                    // rides that line. Locked guides don't attract.
                    let world = if self.snap_to_guides && !self.guides_locked {
                        self.snap_to_guides(world, 4.0)
                    } else {
                        world
                    };
                    self.editor
                        .drag_to_with(world, DragConstraint { shift, alt });
                } else if self.editor.tool == Tool::Hand
                    || space_held
                    || response.dragged_by(egui::PointerButton::Middle)
                {
                    // Hand tool, space-drag pan, or middle-button pan.
                    self.view.pan_by(response.drag_delta());
                }
                // Any gesture that mutated state above needs the next
                // frame now — egui otherwise sleeps between input
                // events, and a web drag only became visible on
                // release.
                ui.ctx().request_repaint();
            }
        }
        if response.drag_stopped() {
            if self.guide_draft.is_some() {
                self.commit_guide_draft();
            }
            if self.editor.guide_drag.is_some()
                && let Err(error) = self.editor.end_guide_drag()
            {
                self.status = format!("nine-slice drag failed: {error}");
            }
            if let Err(error) = self.editor.end_transform_drag() {
                self.status = format!("transform failed: {error}");
            }
            if let Err(error) = self.editor.end_node_drag() {
                self.status = format!("reshape failed: {error}");
            }
            if let Err(error) = self.editor.end_gradient_drag() {
                self.status = format!("gradient drag failed: {error}");
            }
            if let Err(error) = self.editor.end_drag() {
                self.status = format!("create failed: {error}");
            }
            if let Err(error) = self.editor.end_stroke() {
                self.status = format!("paint failed: {error}");
            }
            if let Err(error) = self.editor.end_pixel_selection() {
                self.status = format!("select failed: {error}");
            }
        }
        // Drop target for the Library panel's drag-to-place: the
        // payload clears itself at end of frame, release included.
        if response.hovered() && ui.input(|input| input.pointer.any_released()) {
            if let Some(symbol) = egui::DragAndDrop::take_payload::<i64>(ui.ctx()) {
                let placed_at = ui
                    .input(|input| input.pointer.latest_pos())
                    .map_or(rect.center(), |pos| pos - rect.left_top().to_vec2());
                if let Err(error) = self
                    .editor
                    .place_symbol(*symbol, self.view.to_world(placed_at))
                {
                    self.status = format!("place failed: {error}");
                }
            }
        }
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.left_top().to_vec2();
                match self.editor.tool {
                    Tool::Pointer | Tool::Subselect => {
                        let world = self.view.to_world(local);
                        if ui.input(|i| i.modifiers.shift) {
                            self.editor.toggle_select_at(world);
                        } else {
                            let _ = self.editor.select_at(world);
                        }
                    }
                    Tool::Hand => {}
                    Tool::PaintBucket => {
                        let world = self.view.to_world(local);
                        match self.editor.fill_target(world) {
                            Ok(true) => {}
                            Ok(false) => {
                                self.status = "no image under the pointer".to_owned();
                            }
                            Err(error) => self.status = format!("fill failed: {error}"),
                        }
                    }
                    Tool::Eyedropper => {
                        let world = self.view.to_world(local);
                        if self.editor.pick_color(world).is_none() {
                            self.status = "no image under the pointer".to_owned();
                        }
                    }
                    Tool::Marquee | Tool::EllipticalMarquee | Tool::Lasso | Tool::MagicWand => {
                        // A click without a drag is still a
                        // gesture: the wand selects, a collapsed
                        // marquee deselects (the engine folds an
                        // empty shape into no selection).
                        let world = self.view.to_world(local);
                        self.editor.pixel_mode =
                            PixelSelectMode::from_modifiers(ui.input(|i| i.modifiers));
                        if self.editor.pixel_selection.is_none()
                            && self.editor.begin_pixel_selection(world).is_ok()
                            && let Err(error) = self.editor.end_pixel_selection()
                        {
                            self.status = format!("select failed: {error}");
                        }
                    }
                    Tool::PolygonalMarquee => {
                        let world = self.view.to_world(local);
                        self.editor.pixel_mode =
                            PixelSelectMode::from_modifiers(ui.input(|i| i.modifiers));
                        if let Err(error) = self.editor.add_polygon_point(world) {
                            self.status = format!("polygon failed: {error}");
                        }
                    }
                    Tool::Zoom => {
                        let factor = if ui.input(|i| i.modifiers.alt) {
                            1.0 / 1.25
                        } else {
                            1.25
                        };
                        self.view.zoom_about(local, factor);
                    }
                    _ => {}
                }
            }
        }
        // Double-click enters a symbol instance's definition (CS6's
        // pointer double-click), like the inspector's Edit Definition.
        if response.double_clicked()
            && matches!(self.editor.tool, Tool::Pointer)
            && let Some(pos) = response.interact_pointer_pos()
        {
            let world = self.view.to_world(pos - rect.left_top().to_vec2());
            let entered = self.editor.hit_at(world).and_then(|id| {
                self.editor
                    .document()
                    .registry
                    .get(id)
                    .map(|data| (id, data))
            });
            if let Some((_id, ObjectData::SymbolInstance(instance))) = entered {
                let number = instance.symbol_number;
                match self.editor.enter_symbol(number) {
                    Ok(()) => self.status = format!("editing symbol {number}"),
                    Err(error) => self.status = format!("enter symbol failed: {error}"),
                }
            }
        }
        // The right-click context menu (CS6's canvas popup): a
        // secondary click picks the object under the pointer when it
        // missed the current selection, then opens a popup of
        // selection verbs — every row routed through `run_action`,
        // the same door the menu bar uses.
        if response.secondary_clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let world = self.view.to_world(pos - rect.left_top().to_vec2());
                self.context_menu_pick(world);
                self.context_menu = Some(pos);
            }
        }
        if let Some(open_at) = self.context_menu {
            let mut open = self.context_menu.is_some();
            let popup_id = egui::Id::new("canvas_context_menu");
            let popup = egui::Popup::new(popup_id, ui.ctx().clone(), open_at, response.layer_id)
                .kind(egui::PopupKind::Menu)
                .layout(egui::Layout::top_down_justified(egui::Align::Min))
                .gap(0.0)
                .anchor(open_at)
                .open_bool(&mut open)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClick);
            popup.show(|ui| self.context_menu_ui(ui));
            self.context_menu = open.then_some(open_at);
        }

        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let anchor = ui
                    .input(|i| i.pointer.latest_pos())
                    .map_or(rect.center(), |pos| pos - rect.left_top().to_vec2());
                self.view
                    .zoom_about(anchor, if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 });
            }
        }
    }

    /// The CS6 context options bar under the menu: the active tool's
    /// name, the zoom control, and quick hints. One row, always
    /// visible — the CS6 pattern Photopea follows.
    pub(super) fn options_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tool_label(self.editor.tool)).strong());
            if !tool_key(self.editor.tool).is_empty() {
                ui.weak(format!("({})", tool_key(self.editor.tool)));
            }
            ui.separator();
            // Zoom readout + in/out, always at hand.
            if ui.small_button("−").clicked() {
                if let Some(rect) = self.canvas_rect {
                    self.view.zoom_about(rect.center(), 1.0 / 1.25);
                }
            }
            ui.label(format!("{}%", (self.view.zoom * 100.0).round() as i64));
            if ui.small_button("+").clicked()
                && let Some(rect) = self.canvas_rect
            {
                self.view.zoom_about(rect.center(), 1.25);
            }
            if ui.small_button("Fit").clicked()
                && let (Some(page), Some(rect)) = (self.page_world(), self.canvas_rect)
            {
                self.view.fit(page, rect.size());
            }
            ui.separator();
            ui.weak("F1 for all shortcuts");
        });
    }

    /// The canvas context menu's pick step, callable headless: a
    /// secondary click that missed the current selection selects the
    /// object under the pointer first (CS6's context-menu pick); a
    /// press on (or a miss away from) any object leaves the selection
    /// alone.
    pub(crate) fn context_menu_pick(&mut self, world: geom::Point) {
        let hits_selection = self
            .editor
            .selection_set
            .iter()
            .any(|id| self.editor.hit_at(world) == Some(*id));
        if hits_selection {
            return;
        }
        if let Some(hit) = self.editor.hit_at(world) {
            self.editor.selection = Some(hit);
            self.editor.selection_set = vec![hit];
        }
    }

    /// The canvas context menu's rows: selection verbs mirroring the
    /// CS6 canvas popup (clipboard, transform, arrange, group), routed
    /// through `run_action` like every other menu surface. An empty
    /// selection leaves only the paste/view rows enabled.
    fn context_menu_ui(&mut self, ui: &mut egui::Ui) {
        use crate::menus::{Action, Arrange, SelAction, ShellAction};
        let has_selection = !self.editor.selection_set.is_empty();
        let rows: &[(&str, bool, Action)] = &[
            ("Cut", has_selection, Action::Shell(ShellAction::Cut)),
            ("Copy", has_selection, Action::Shell(ShellAction::Copy)),
            ("Paste", true, Action::Shell(ShellAction::Paste)),
        ];
        for (label, wired, action) in rows {
            if ui.add_enabled(*wired, egui::Button::new(*label)).clicked() {
                self.run_action(action);
                ui.close();
            }
        }
        if ui
            .add_enabled(has_selection, egui::Button::new("Duplicate\tCtrl+Alt+D"))
            .clicked()
        {
            for id in self.editor.live_selection() {
                self.run_command("object.duplicate", serde_json::json!({ "id": id }));
            }
            ui.close();
        }
        ui.separator();
        let sel_rows: &[(&str, Action)] = &[
            (
                "Numeric Transform...",
                Action::Shell(ShellAction::NumericTransform),
            ),
            (
                "Flip Horizontal",
                Action::Sel(SelAction::Reflect {
                    horizontal: true,
                    vertical: false,
                }),
            ),
            (
                "Flip Vertical",
                Action::Sel(SelAction::Reflect {
                    horizontal: false,
                    vertical: true,
                }),
            ),
            ("Rotate 90° CW", Action::Sel(SelAction::Rotate(90.0))),
            ("Bring to Front", Action::Arrange(Arrange::Front)),
            ("Send to Back", Action::Arrange(Arrange::Back)),
            ("Group\tCtrl+G", Action::Sel(SelAction::Group)),
            ("Ungroup\tCtrl+U", Action::Sel(SelAction::Ungroup)),
            ("Hide\tCtrl+M", Action::Sel(SelAction::Hide)),
            ("Select All", Action::Shell(ShellAction::SelectAll)),
            ("Deselect\tCtrl+D", Action::Shell(ShellAction::Deselect)),
        ];
        for (label, action) in sel_rows {
            if ui
                .add_enabled(has_selection, egui::Button::new(*label))
                .clicked()
            {
                self.run_action(action);
                ui.close();
            }
        }
    }
}

/// The eight transform-handle positions of a selection frame, in
/// [`TransformHandle`] order.
fn transform_handle_points(frame: Rect) -> [Pos2; 8] {
    let middle = frame.center();
    [
        frame.left_top(),
        frame.right_top(),
        frame.right_bottom(),
        frame.left_bottom(),
        Pos2::new(middle.x, frame.top()),
        Pos2::new(middle.x, frame.bottom()),
        Pos2::new(frame.left(), middle.y),
        Pos2::new(frame.right(), middle.y),
    ]
}

/// The handle (if any) under a widget-local press on the selection
/// frame. Hit squares match the painted 7px handles with a small
/// slack so a near-miss still grabs.
fn transform_handle_at(frame: Rect, local: Pos2) -> Option<TransformHandle> {
    const HANDLES: [TransformHandle; 8] = [
        TransformHandle::TopLeft,
        TransformHandle::TopRight,
        TransformHandle::BottomRight,
        TransformHandle::BottomLeft,
        TransformHandle::Top,
        TransformHandle::Bottom,
        TransformHandle::Left,
        TransformHandle::Right,
    ];
    transform_handle_points(frame)
        .iter()
        .zip(HANDLES)
        .find(|(point, _)| Rect::from_center_size(**point, Vec2::splat(11.0)).contains(local))
        .map(|(_, handle)| handle)
}
