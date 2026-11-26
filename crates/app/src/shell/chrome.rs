//! CS6 Studio workspace chrome extracted from the Experiment UX design and
//! wired directly onto FireCraft's real `Editor` + `engine::Session` command
//! registry:
//! - Header bar (10 CS6 menus + quick Open/Hand/Zoom + Zoom combo + Palette / Full / Theme / Workspace)
//! - Main Toolbar (File / Edit / History / Arrange / Transform / Demo Showcase)
//! - Left 2-column collapsible CS6 Tools Palette with pinned Fill/Stroke footer
//! - Sculpted CS6 Document Tab Strip + Original/Preview/2-Up/4-Up view bar + Page selector
//! - Crisp top & left Canvas Rulers (px/in/cm/mm)
//! - Unified Canvas Footer Bar (Format, Snap, Rulers, Guides, State Transport, Zoom, Resolution, DPI)
//! - Bottom Resizable Properties Inspector Dock (`Properties | Symbol Properties | Info`)
//! - Crisp Bottom Status Bar (`Zoom | Tool | Cursor X, Y`)

use compose::abgr_pack;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2, pos2};

use super::theme::{self, Palette, ThemeMode};
use super::vector_icons::{self, Icon};
use super::*;

/// Canvas display mode (CS6 Document Window tabs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CanvasViewMode {
    #[default]
    Original,
    Preview,
    TwoUp,
    FourUp,
}

impl CanvasViewMode {
    pub(crate) const ALL: [(Self, Icon, &'static str); 4] = [
        (Self::Original, Icon::Pen, "Original"),
        (Self::Preview, Icon::Eye, "Preview"),
        (Self::TwoUp, Icon::Slice, "2-Up"),
        (Self::FourUp, Icon::Scale, "4-Up"),
    ];
}

/// Map a FireCraft [`Tool`] to its crisp programmatic [`vector_icons::Icon`].
#[must_use]
pub(crate) fn tool_vector_icon(tool: Tool) -> Icon {
    match tool {
        Tool::Pointer => Icon::Select,
        Tool::Subselect => Icon::Subselect,
        Tool::Scale | Tool::Skew | Tool::Distort => Icon::Scale,
        Tool::Crop => Icon::Crop,
        Tool::Marquee | Tool::EllipticalMarquee => Icon::Marquee,
        Tool::Lasso | Tool::PolygonalMarquee => Icon::Lasso,
        Tool::MagicWand | Tool::SelectSimilar => Icon::Wand,
        Tool::Paintbrush => Icon::Brush,
        Tool::Pencil => Icon::Pencil,
        Tool::Eraser => Icon::Eraser,
        Tool::Blur | Tool::Smudge => Icon::Blur,
        Tool::Sharpen => Icon::Sharpen,
        Tool::Dodge => Icon::Dodge,
        Tool::Burn => Icon::Burn,
        Tool::RubberStamp => Icon::Stamp,
        Tool::RedEyeRemoval => Icon::Eye,
        Tool::ReplaceColor => Icon::Fill,
        Tool::Line => Icon::Line,
        Tool::Pen | Tool::VectorPath | Tool::RedrawPath | Tool::Freeform | Tool::ReshapeArea => {
            Icon::Pen
        }
        Tool::Rect => Icon::Rectangle,
        Tool::Ellipse => Icon::Ellipse,
        Tool::AutoShape(shape) => match shape {
            AutoShape::Star => Icon::Star,
            AutoShape::RoundedRect => Icon::Rounded,
            _ => Icon::Polygon,
        },
        Tool::Text => Icon::Text,
        Tool::Knife => Icon::Knife,
        Tool::Slice | Tool::PolygonSlice | Tool::HideShowSlices => Icon::Slice,
        Tool::Hotspot | Tool::PolygonHotspot | Tool::CircleHotspot => Icon::Hotspot,
        Tool::Eyedropper => Icon::Picker,
        Tool::PaintBucket | Tool::DefaultStrokeFill | Tool::SwapStrokeFill | Tool::NoStrokeFill => {
            Icon::Fill
        }
        Tool::Gradient => Icon::Gradient,
        Tool::Hand => Icon::Hand,
        Tool::Zoom => Icon::Zoom,
        Tool::ToggleScreenMode => Icon::Fit,
    }
}

/// Small clickable vector icon button used in the top header & main toolbar.
pub(crate) fn icon_toolbar_button(
    ui: &mut Ui,
    icon: Icon,
    active: bool,
    tip: &str,
) -> egui::Response {
    let v = ui.visuals().clone();
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
    if ui.is_rect_visible(rect) {
        if active {
            ui.painter()
                .rect_filled(rect, 0.0, v.selection.bg_fill.gamma_multiply(0.55));
            ui.painter().rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, v.selection.stroke.color),
                egui::StrokeKind::Inside,
            );
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(rect, 0.0, v.widgets.hovered.bg_fill);
            ui.painter().rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, v.widgets.hovered.bg_stroke.color),
                egui::StrokeKind::Inside,
            );
        }
        let col = if active {
            Color32::WHITE
        } else {
            v.text_color()
        };
        vector_icons::draw(ui.painter(), rect.shrink(1.0), icon, col, 1.25);
    }
    resp.on_hover_text(tip)
}

fn toolbar_sep(ui: &mut Ui, color: Color32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(7.0, 20.0), Sense::hover());
    ui.painter().line_segment(
        [
            pos2(r.center().x, r.top() + 3.0),
            pos2(r.center().x, r.bottom() - 3.0),
        ],
        Stroke::new(1.0, color),
    );
}

fn compact_tab_label(name: &str, max_chars: usize) -> String {
    let chars: Vec<char> = name.chars().collect();
    let cap = max_chars.max(4);
    if chars.len() <= cap {
        name.to_owned()
    } else {
        let keep = cap - 1;
        format!("{}…", chars[..keep].iter().collect::<String>())
    }
}

fn footer_icon_button(ui: &mut Ui, icon: Icon, enabled: bool, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(20.0, 18.0), Sense::click());
    let hovered = enabled && resp.hovered();
    if hovered {
        ui.painter()
            .rect_filled(rect.shrink(1.0), 0.0, Color32::from_white_alpha(20));
    }
    let color = if enabled {
        if hovered {
            Color32::WHITE
        } else {
            Color32::from_gray(204)
        }
    } else {
        Color32::from_gray(100)
    };
    vector_icons::draw(ui.painter(), rect.shrink(2.0), icon, color, 1.15);
    let clicked = resp.clicked() && enabled;
    if enabled {
        resp.on_hover_text(tip);
    } else {
        resp.on_hover_text(format!("{tip} (unavailable)"));
    }
    clicked
}

fn abgr_to_color32(raw: i64) -> Color32 {
    let bits = raw as u32;
    let r = (bits & 0xFF) as u8;
    let g = ((bits >> 8) & 0xFF) as u8;
    let b = ((bits >> 16) & 0xFF) as u8;
    let a = ((bits >> 24) & 0xFF) as u8;
    Color32::from_rgba_unmultiplied(r, g, b, if a == 0 && bits != 0 { 255 } else { a })
}

impl Shell {
    /// Resolve the active [`Palette`] for `self.theme_mode`.
    #[must_use]
    pub fn active_palette(&self) -> Palette {
        theme::resolve(self.theme_mode)
    }

    /// Apply CS6 styling to `ctx` whenever the selected theme changes.
    pub(super) fn apply_active_theme(&mut self, ctx: &egui::Context) {
        let theme_id = egui::Id::new("firecraft_applied_theme_mode");
        let last = ctx.data(|d| d.get_temp::<ThemeMode>(theme_id));
        if last != Some(self.theme_mode) {
            let p = self.active_palette();
            theme::apply_cs6_style(ctx, &p, p.is_dark());
            ctx.data_mut(|d| d.insert_temp(theme_id, self.theme_mode));
        }
    }

    /// Configure the rich CS6 Studio workspace defaults for interactive runs
    /// (called from `Shell::new` when no persisted user layout overrides them).
    pub fn apply_studio_defaults(&mut self) {
        self.show_rulers = true;
        self.show_bottom_inspector = true;
        self.bottom_collapsed = false;
        self.bottom_height = 148.0;
        self.show_welcome = true;
        self.apply_workspace_preset("Expanded Mode");
        self.load_demo_showcase();
    }

    /// Switch workspace layout presets (Expanded Mode, Iconic Mode, Compact Mode).
    pub(crate) fn apply_workspace_preset(&mut self, preset: &'static str) {
        self.workspace_preset = preset;
        match preset {
            "Iconic Mode" => {
                self.panels_hidden = false;
                self.right_dock_collapsed = true;
                self.tools_hidden = false;
                self.tools_collapsed = false;
                self.show_bottom_inspector = true;
                self.ensure_cs6_dock_groups();
                self.status = "Workspace: Iconic Mode".to_owned();
            }
            "Compact Mode" => {
                self.panels_hidden = false;
                self.right_dock_collapsed = false;
                self.tools_hidden = false;
                self.tools_collapsed = true;
                self.show_bottom_inspector = true;
                self.bottom_collapsed = true;
                self.ensure_cs6_dock_groups();
                self.status = "Workspace: Compact Mode".to_owned();
            }
            _ => {
                self.panels_hidden = false;
                self.right_dock_collapsed = false;
                self.tools_hidden = false;
                self.tools_collapsed = false;
                self.show_bottom_inspector = true;
                self.bottom_collapsed = false;
                self.ensure_cs6_dock_groups();
                self.status = "Workspace: Expanded Mode".to_owned();
            }
        }
    }

    /// Arrange the right dock into CS6's four stacked tabbed groups
    /// (Optimize|History|Align|Path, Pages|States|Layers|Behaviors,
    /// Styles|Color Palette|Color Mixer|Swatches,
    /// Info|Special Characters|Image Editing|Auto Shapes|Document Library|CSS Properties)
    /// while moving Properties to the dedicated Bottom Inspector Dock.
    pub(crate) fn ensure_cs6_dock_groups(&mut self) {
        let grouped_specs: &[(&[&'static str], &'static str, f32)] = &[
            (&["Optimize", "History", "Align", "Path"], "Optimize", 148.0),
            (&["Pages", "States", "Layers", "Behaviors"], "Layers", 178.0),
            (
                &["Styles", "Color Palette", "Color Mixer", "Swatches"],
                "Color Palette",
                172.0,
            ),
            (
                &[
                    "Info",
                    "Special Characters",
                    "Image Editing",
                    "Auto Shapes",
                    "Document Library",
                    "CSS Properties",
                ],
                "Auto Shapes",
                122.0,
            ),
        ];

        let mut visible: Vec<&'static str> = Vec::new();
        for (members, _, _) in grouped_specs {
            visible.extend_from_slice(members);
        }

        // Hide panels that are either moved to the bottom inspector ("Properties")
        // or not part of the default 4-group stack.
        self.panel_hidden = panel_windows::PANELS
            .iter()
            .filter(|spec| !visible.contains(&spec.name))
            .map(|spec| spec.name)
            .collect();

        self.panel_groups.clear();
        for (idx, (members, active, height)) in grouped_specs.iter().enumerate() {
            self.next_panel_group_id = self.next_panel_group_id.wrapping_add(1);
            self.panel_groups.push(panel_windows::PanelGroup {
                members: members.to_vec(),
                active,
                id: self.next_panel_group_id,
                floating_pos: Pos2::new(60.0 + idx as f32 * 24.0, 96.0 + idx as f32 * 24.0),
                floating_width: 268.0,
                stack_parent: None,
                collapsed: idx == 3, // Collapse Group 4 initially so Groups 1-3 breathe cleanly
                body_height: *height,
            });
        }
    }

    /// Replace the active document with the 1920×1080 FireCraft Studio Showcase.
    pub(crate) fn load_demo_showcase(&mut self) {
        self.editor = demo::build_showcase_editor();
        self.doc_name = "FireCraft_Showcase.fw.png".to_owned();
        self.doc_path = None;
        self.raster = None;
        self.raster_scale = 1.0;
        self.view = View::default();
        self.status = "Loaded 1080p FireCraft Studio Showcase".to_owned();
    }

    /// Public wrapper for `--blank` CLI launch flag.
    pub fn run_shell_new_blank(&mut self) {
        self.new_blank_1080p_document();
    }

    /// Replace the active document with a clean 1920×1080 blank white canvas.
    pub(crate) fn new_blank_1080p_document(&mut self) {
        let mut editor = Editor::untitled();
        let _ = editor.apply(
            "page.setCanvasSize",
            serde_json::json!({ "page": 0, "width": 1920.0, "height": 1080.0 }),
        );
        let _ = editor.apply(
            "document.setResolution",
            serde_json::json!({ "resolution": 144.0, "unit": "inch" }),
        );
        self.editor = editor;
        self.doc_name = "Untitled-1.fw.png".to_owned();
        self.doc_path = None;
        self.raster = None;
        self.raster_scale = 1.0;
        self.view = View::default();
        self.status = "Created blank 1920×1080 document".to_owned();
    }

    /// Zoom helper anchored to the canvas center.
    pub(crate) fn set_zoom_exact(&mut self, zoom: f32) {
        let clamped = zoom.clamp(constants::ZOOM_MIN, constants::ZOOM_MAX);
        if let Some(rect) = self.canvas_rect {
            let factor = clamped / self.view.zoom.max(0.01);
            self.view.zoom_about(rect.center(), factor);
        } else {
            self.view.zoom = clamped;
        }
        self.status = format!("Zoom: {:.0}%", self.view.zoom * 100.0);
    }

    /// Fit current page in the canvas viewport.
    pub(crate) fn fit_canvas_in_window(&mut self) {
        if let (Some(page), Some(rect)) = (self.page_world(), self.canvas_rect) {
            self.view.fit(page, rect.size());
            self.status = format!("Fit in window ({:.0}%)", self.view.zoom * 100.0);
        }
    }

    /// Top Application Menu Bar (10 CS6 menus + Quick Open/Hand/Zoom + Zoom Combo +
    /// Palette / Full / Theme / Workspace Preset controls).
    pub(super) fn cs6_menu_bar(&mut self, ui: &mut Ui) {
        let p = self.active_palette();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
            egui::MenuBar::new().ui(ui, |ui| {
                for menu in menus::MENUBAR {
                    ui.menu_button(menu.label, |ui| self.entries_ui(ui, menu.entries));
                }
            });

            toolbar_sep(ui, p.stroke);

            if icon_toolbar_button(ui, Icon::Folder, false, "Open Document (Ctrl+O)").clicked() {
                self.run_shell(menus::ShellAction::Open);
            }
            if icon_toolbar_button(
                ui,
                Icon::Hand,
                self.editor.tool == Tool::Hand,
                "Hand Tool (H)",
            )
            .clicked()
            {
                self.pick_tool(Tool::Hand);
            }
            if icon_toolbar_button(
                ui,
                Icon::Zoom,
                self.editor.tool == Tool::Zoom,
                "Zoom Tool (Z)",
            )
            .clicked()
            {
                self.pick_tool(Tool::Zoom);
            }

            let cur_zoom_pct = (self.view.zoom * 100.0).round() as i32;
            egui::ComboBox::from_id_salt("header_zoom_combo")
                .width(64.0)
                .selected_text(format!("{cur_zoom_pct}%"))
                .show_ui(ui, |ui| {
                    for &preset in constants::ZOOM_PRESETS {
                        let lbl = format!("{:.0}%", preset * 100.0);
                        if ui
                            .selectable_label((self.view.zoom - preset).abs() < 0.01, lbl)
                            .clicked()
                        {
                            self.set_zoom_exact(preset);
                        }
                    }
                    ui.separator();
                    if ui.button("Fit in Window (Ctrl+0)").clicked() {
                        self.fit_canvas_in_window();
                    }
                });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);

                egui::ComboBox::from_id_salt("workspace_preset_combo")
                    .width(120.0)
                    .selected_text(if self.screen_mode == ScreenMode::FullScreen {
                        "Full Screen"
                    } else {
                        self.workspace_preset
                    })
                    .show_ui(ui, |ui| {
                        for preset in ["Expanded Mode", "Iconic Mode", "Compact Mode"] {
                            if ui
                                .selectable_label(self.workspace_preset == preset, preset)
                                .clicked()
                            {
                                self.apply_workspace_preset(preset);
                            }
                        }
                        ui.separator();
                        if ui.button("Cycle Screen Mode (F)").clicked() {
                            self.run_shell(menus::ShellAction::CycleScreenMode);
                        }
                        if ui.button("Welcome Screen…").clicked() {
                            self.show_welcome = true;
                        }
                    });

                egui::ComboBox::from_id_salt("header_theme_combo")
                    .width(126.0)
                    .selected_text(self.theme_mode.label())
                    .show_ui(ui, |ui| {
                        for mode in ThemeMode::ALL {
                            if ui
                                .selectable_label(self.theme_mode == mode, mode.label())
                                .clicked()
                            {
                                self.theme_mode = mode;
                            }
                        }
                    });

                if ui
                    .add_sized(
                        [38.0, 19.0],
                        egui::Button::new(if self.window_fullscreen {
                            "Exit"
                        } else {
                            "Full"
                        }),
                    )
                    .on_hover_text("Toggle Window Fullscreen (F11)")
                    .clicked()
                {
                    self.window_fullscreen = !self.window_fullscreen;
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                            self.window_fullscreen,
                        ));
                }

                if ui
                    .add_sized([58.0, 19.0], egui::Button::new("Palette"))
                    .on_hover_text("Open Command Palette (Ctrl+P)")
                    .clicked()
                {
                    self.toggle_palette();
                }
            });
        });
    }

    /// CS6 Main Toolbar (Row 2, 28px): File, Edit, History, Arrange, Transform, Demo Showcase.
    pub(super) fn cs6_main_toolbar(&mut self, ui: &mut Ui) {
        let p = self.active_palette();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);

            if icon_toolbar_button(ui, Icon::New, false, "New Document (Ctrl+N)").clicked() {
                self.run_shell(menus::ShellAction::New);
            }
            if icon_toolbar_button(ui, Icon::Open, false, "Open Document (Ctrl+O)").clicked() {
                self.run_shell(menus::ShellAction::Open);
            }
            if icon_toolbar_button(ui, Icon::Save, false, "Save Document (Ctrl+S)").clicked() {
                self.run_shell(menus::ShellAction::Save);
            }
            if icon_toolbar_button(ui, Icon::Import, false, "Import Image (Ctrl+R)").clicked() {
                self.run_shell(menus::ShellAction::Import);
            }
            if icon_toolbar_button(ui, Icon::Export, false, "Export (Ctrl+Shift+R)").clicked() {
                self.run_shell(menus::ShellAction::Export);
            }

            toolbar_sep(ui, p.stroke);

            if icon_toolbar_button(ui, Icon::Cut, false, "Cut (Ctrl+X)").clicked() {
                self.run_shell(menus::ShellAction::Cut);
            }
            if icon_toolbar_button(ui, Icon::Copy, false, "Copy (Ctrl+C)").clicked() {
                self.run_shell(menus::ShellAction::Copy);
            }
            if icon_toolbar_button(ui, Icon::Paste, false, "Paste (Ctrl+V)").clicked() {
                self.run_shell(menus::ShellAction::Paste);
            }
            if icon_toolbar_button(ui, Icon::Duplicate, false, "Duplicate (Ctrl+Alt+D)").clicked() {
                self.run_sel(menus::SelAction::Duplicate);
            }

            toolbar_sep(ui, p.stroke);

            if icon_toolbar_button(ui, Icon::Undo, false, "Undo (Ctrl+Z)").clicked() {
                self.run_shell(menus::ShellAction::Undo);
            }
            if icon_toolbar_button(ui, Icon::Redo, false, "Redo (Ctrl+Y)").clicked() {
                self.run_shell(menus::ShellAction::Redo);
            }

            toolbar_sep(ui, p.stroke);

            if icon_toolbar_button(ui, Icon::Group, false, "Group (Ctrl+G)").clicked() {
                self.run_sel(menus::SelAction::Group);
            }
            if icon_toolbar_button(ui, Icon::Ungroup, false, "Ungroup (Ctrl+Shift+G)").clicked() {
                self.run_sel(menus::SelAction::Ungroup);
            }

            toolbar_sep(ui, p.stroke);

            if icon_toolbar_button(ui, Icon::BringFront, false, "Bring to Front").clicked() {
                self.run_action(&menus::Action::Arrange(menus::Arrange::Front));
            }
            if icon_toolbar_button(ui, Icon::BringForward, false, "Bring Forward").clicked() {
                self.run_action(&menus::Action::Arrange(menus::Arrange::Forward));
            }
            if icon_toolbar_button(ui, Icon::SendBackward, false, "Send Backward").clicked() {
                self.run_action(&menus::Action::Arrange(menus::Arrange::Backward));
            }
            if icon_toolbar_button(ui, Icon::SendBack, false, "Send to Back").clicked() {
                self.run_action(&menus::Action::Arrange(menus::Arrange::Back));
            }

            toolbar_sep(ui, p.stroke);

            if icon_toolbar_button(ui, Icon::RotateCCW, false, "Rotate 90° CCW").clicked() {
                self.run_sel(menus::SelAction::Rotate(-90.0));
            }
            if icon_toolbar_button(ui, Icon::RotateCW, false, "Rotate 90° CW").clicked() {
                self.run_sel(menus::SelAction::Rotate(90.0));
            }
            if icon_toolbar_button(ui, Icon::FlipH, false, "Flip Horizontal").clicked() {
                self.run_sel(menus::SelAction::Reflect {
                    horizontal: true,
                    vertical: false,
                });
            }
            if icon_toolbar_button(ui, Icon::FlipV, false, "Flip Vertical").clicked() {
                self.run_sel(menus::SelAction::Reflect {
                    horizontal: false,
                    vertical: true,
                });
            }

            toolbar_sep(ui, p.stroke);

            if ui
                .small_button("Demo Showcase")
                .on_hover_text("Load the 1080p FireCraft Studio vector + text + slice showcase")
                .clicked()
            {
                self.load_demo_showcase();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} ({})",
                        tool_label(self.editor.tool),
                        if tool_key(self.editor.tool).is_empty() {
                            "—"
                        } else {
                            tool_key(self.editor.tool)
                        }
                    ))
                    .small()
                    .color(p.text_dim),
                );
            });
        });
    }

    /// CS6 2-column collapsible Left Tools Palette with top grip bar, subtle category
    /// divider bands, programmatic multi-tone vector icons, flyout corner triangles,
    /// and a pinned Fill/Stroke + screen-mode footer that never clips at 960×640.
    pub(super) fn cs6_tools_panel(&mut self, ui: &mut Ui) {
        let p = self.active_palette();

        // Top grip bar with << / >> collapse toggle
        let bar_w = ui.available_width().max(26.0);
        let (bar_r, _) = ui.allocate_exact_size(Vec2::new(bar_w, 15.0), Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(bar_r, 0.0, p.panel_alt);
        painter.line_segment(
            [bar_r.left_bottom(), bar_r.right_bottom()],
            Stroke::new(1.0, p.stroke),
        );

        let btn_r = Rect::from_min_max(
            pos2(bar_r.right() - 18.0, bar_r.top() + 1.0),
            pos2(bar_r.right() - 1.0, bar_r.bottom() - 1.0),
        );
        let btn_resp = ui.interact(
            btn_r,
            ui.make_persistent_id("cs6_tools_collapse_btn"),
            Sense::click(),
        );
        if btn_resp.hovered() {
            painter.rect_filled(btn_r, 0.0, p.well);
        }
        painter.text(
            btn_r.center(),
            Align2::CENTER_CENTER,
            if self.tools_collapsed { ">>" } else { "<<" },
            FontId::proportional(9.0),
            if btn_resp.hovered() {
                p.selection
            } else {
                p.text_dim
            },
        );
        if btn_resp
            .on_hover_text(if self.tools_collapsed {
                "Expand tool palette (2 columns)"
            } else {
                "Collapse tool palette (1 column)"
            })
            .clicked()
        {
            self.tools_collapsed = !self.tools_collapsed;
        }

        // Dotted CS6 grip
        let dots = ((bar_r.width() - 24.0) / 4.0).floor().clamp(2.0, 6.0) as usize;
        for i in 0..dots {
            painter.rect_filled(
                Rect::from_center_size(
                    pos2(bar_r.left() + 5.0 + i as f32 * 4.0, bar_r.center().y),
                    Vec2::splat(1.5),
                ),
                0.0,
                p.text_dim,
            );
        }

        // Reserve space for the pinned bottom color/screen footer so it never clips
        let footer_h = if self.tools_collapsed { 62.0 } else { 52.0 };
        let body_h = (ui.available_height() - footer_h).max(120.0);
        let columns = if self.tools_collapsed { 1 } else { 2 };
        let compact = body_h < 420.0;

        egui::ScrollArea::vertical()
            .id_salt("cs6_tools_scroll")
            .max_height(body_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for category in toolstrip::CATEGORIES {
                    // Skip the Colors/View action buttons that live cleanly in our pinned footer
                    if category.name == "Colors" {
                        // Draw Eyedropper / Paint Bucket / Gradient only; swatches live in footer
                    }
                    ui.add_space(if compact { 2.0 } else { 4.0 });
                    let (band, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), if compact { 3.0 } else { 5.0 }),
                        Sense::hover(),
                    );
                    ui.painter()
                        .rect_filled(band, 0.0, p.panel_alt.gamma_multiply(0.7));
                    ui.painter().line_segment(
                        [band.left_bottom(), band.right_bottom()],
                        Stroke::new(1.0, p.stroke.gamma_multiply(0.45)),
                    );

                    // Filter out the three color-footer panel buttons (rendered in pinned footer)
                    let groups: Vec<&toolstrip::ToolGroup> = category
                        .groups
                        .iter()
                        .filter(|g| {
                            !matches!(
                                g.leader().tool,
                                Tool::DefaultStrokeFill
                                    | Tool::SwapStrokeFill
                                    | Tool::NoStrokeFill
                                    | Tool::ToggleScreenMode
                            )
                        })
                        .collect();

                    for row in groups.chunks(columns) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(3.0, 2.0);
                            for group in row {
                                let shown = group
                                    .tools
                                    .iter()
                                    .copied()
                                    .find(|e| e.tool == self.editor.tool)
                                    .unwrap_or_else(|| group.leader());
                                let active = group.tools.iter().any(|e| e.tool == self.editor.tool);
                                let flyout = group.is_flyout();
                                let side = if compact { 22.0 } else { 26.0 };
                                let (rect, resp) =
                                    ui.allocate_exact_size(Vec2::splat(side), Sense::click());
                                if ui.is_rect_visible(rect) {
                                    let ptr = ui.painter();
                                    if active {
                                        ptr.rect_filled(rect, 1.0, p.accent.gamma_multiply(0.75));
                                        ptr.rect_stroke(
                                            rect,
                                            1.0,
                                            Stroke::new(1.0, p.selection),
                                            egui::StrokeKind::Inside,
                                        );
                                    } else if resp.hovered() {
                                        ptr.rect_filled(rect, 1.0, p.surface);
                                        ptr.rect_stroke(
                                            rect,
                                            1.0,
                                            Stroke::new(1.0, p.accent),
                                            egui::StrokeKind::Inside,
                                        );
                                    }
                                    let col = if active { Color32::WHITE } else { p.text };
                                    vector_icons::draw(
                                        ptr,
                                        rect.shrink(1.0),
                                        tool_vector_icon(shown.tool),
                                        col,
                                        1.3,
                                    );
                                    if flyout {
                                        let a = pos2(rect.right() - 7.5, rect.bottom() - 1.5);
                                        ptr.add(egui::Shape::convex_polygon(
                                            vec![
                                                a,
                                                pos2(a.x + 6.0, a.y),
                                                pos2(a.x + 6.0, a.y - 6.0),
                                            ],
                                            p.selection,
                                            Stroke::NONE,
                                        ));
                                    }
                                }
                                let tip = if group.key.is_empty() {
                                    shown.label.to_owned()
                                } else {
                                    format!("{} ({})", shown.label, group.key)
                                };
                                let resp = resp.on_hover_text(tip);
                                let corner_click = resp.interact_pointer_pos().is_some_and(|pos| {
                                    pos.x >= rect.right() - 9.0 && pos.y >= rect.bottom() - 9.0
                                });
                                if resp.clicked() {
                                    if toolstrip::is_panel_button(shown.tool) {
                                        self.run_panel_button(shown.tool);
                                    } else if flyout && corner_click {
                                        let tools: Vec<Tool> =
                                            group.tools.iter().map(|e| e.tool).collect();
                                        self.tool_flyout = Some((
                                            category.name,
                                            tools,
                                            pos2(rect.right() + 2.0, rect.top()),
                                        ));
                                    } else {
                                        self.pick_tool(shown.tool);
                                    }
                                } else if flyout && resp.secondary_clicked() {
                                    let tools: Vec<Tool> =
                                        group.tools.iter().map(|e| e.tool).collect();
                                    self.tool_flyout = Some((
                                        category.name,
                                        tools,
                                        pos2(rect.right() + 2.0, rect.top()),
                                    ));
                                }
                            }
                        });
                    }

                    if category.auto_shapes {
                        ui.add_space(2.0);
                        self.autoshape_button(ui);
                    }
                }
            });

        // Pinned CS6 Fill/Stroke + Quick Actions Footer (always visible, never clipped)
        ui.separator();
        ui.add_space(2.0);
        let fill_rgba = self.editor.fill_color;
        let stroke_rgba = self.editor.stroke_color;
        let mut fill_c32 =
            Color32::from_rgba_unmultiplied(fill_rgba[0], fill_rgba[1], fill_rgba[2], fill_rgba[3]);
        let mut stroke_c32 = Color32::from_rgba_unmultiplied(
            stroke_rgba[0],
            stroke_rgba[1],
            stroke_rgba[2],
            stroke_rgba[3],
        );

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(3.0, 2.0);
            if ui
                .color_edit_button_srgba(&mut fill_c32)
                .on_hover_text("Fill Color")
                .changed()
            {
                self.editor.fill_color = [fill_c32.r(), fill_c32.g(), fill_c32.b(), fill_c32.a()];
                self.fill_color = fill_c32;
            }
            if !self.tools_collapsed
                && ui
                    .color_edit_button_srgba(&mut stroke_c32)
                    .on_hover_text("Stroke Color")
                    .changed()
            {
                self.editor.stroke_color = [
                    stroke_c32.r(),
                    stroke_c32.g(),
                    stroke_c32.b(),
                    stroke_c32.a(),
                ];
                self.stroke_color = stroke_c32;
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
            if ui
                .small_button("⇄")
                .on_hover_text("Swap Fill & Stroke (X)")
                .clicked()
            {
                self.run_panel_button(Tool::SwapStrokeFill);
            }
            if ui
                .small_button("▣")
                .on_hover_text("Default Colors (D)")
                .clicked()
            {
                self.run_panel_button(Tool::DefaultStrokeFill);
            }
            if !self.tools_collapsed
                && ui
                    .small_button("⊘")
                    .on_hover_text("No Fill / Clear Stroke")
                    .clicked()
            {
                self.run_panel_button(Tool::NoStrokeFill);
            }
            if !self.tools_collapsed
                && ui
                    .small_button("⛶")
                    .on_hover_text("Cycle Screen Mode (F)")
                    .clicked()
            {
                self.run_panel_button(Tool::ToggleScreenMode);
            }
        });

        self.tool_flyout_ui(ui);
    }

    /// Sculpted CS6 Document Tab Strip + Canvas View Mode Bar (`Original | Preview | 2-Up | 4-Up` + Page).
    pub(super) fn cs6_document_tab_strip(&mut self, ui: &mut Ui) {
        let p = self.active_palette();
        egui::Frame::new()
            .fill(p.panel_alt)
            .inner_margin(egui::Margin::symmetric(4, 2))
            .show(ui, |ui| {
                // Row 1: Sculpted Document Tabs
                let mut switch_to: Option<usize> = None;
                let mut close_active = false;
                let mut close_parked: Option<usize> = None;
                let mut create_new = false;
                let mut close_others = false;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

                    // Mouse-wheel switching across open tabs
                    let total_tabs = 1 + self.tabs.len();
                    let strip_rect =
                        Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), 22.0));
                    if total_tabs > 1 && ui.rect_contains_pointer(strip_rect) {
                        let wheel = ui.input(|i| {
                            let mut d = 0.0f32;
                            for ev in &i.raw.events {
                                if let egui::Event::MouseWheel { delta, .. } = ev {
                                    d += if delta.y.abs() >= delta.x.abs() {
                                        delta.y
                                    } else {
                                        delta.x
                                    };
                                }
                            }
                            d
                        });
                        if wheel.abs() > 0.5 && !self.tabs.is_empty() {
                            switch_to = Some(if wheel > 0.0 { self.tabs.len() - 1 } else { 0 });
                        }
                    }

                    let controls_reserve = 56.0;
                    let tab_area = (ui.available_width() - controls_reserve).max(90.0);
                    let max_visible = ((tab_area / 100.0).floor() as usize).max(1);
                    let parked_count = self.tabs.len();
                    let visible_parked = parked_count.min(max_visible.saturating_sub(1));
                    let hidden_count = parked_count.saturating_sub(visible_parked);
                    let tab_w = (tab_area / (1 + visible_parked) as f32 - 3.0).clamp(92.0, 210.0);

                    // Active document tab (index 0 visually)
                    let active_label = if self.editor.session.is_dirty() {
                        format!("*{}", self.doc_name)
                    } else {
                        self.doc_name.clone()
                    };
                    let (tab_r, tab_resp) =
                        ui.allocate_exact_size(Vec2::new(tab_w, 21.0), Sense::click());
                    let close_r = Rect::from_center_size(
                        pos2(tab_r.right() - 10.0, tab_r.center().y),
                        Vec2::splat(13.0),
                    );
                    let close_resp = ui.interact(
                        close_r,
                        ui.make_persistent_id("active_doc_tab_close"),
                        Sense::click(),
                    );
                    let ptr = ui.painter();
                    ptr.rect_filled(tab_r, 0.0, p.panel);
                    ptr.rect_stroke(
                        tab_r,
                        0.0,
                        Stroke::new(1.0, p.stroke),
                        egui::StrokeKind::Middle,
                    );
                    ptr.line_segment(
                        [tab_r.left_top(), tab_r.right_top()],
                        Stroke::new(2.0, p.selection),
                    );
                    ptr.text(
                        pos2(tab_r.left() + 8.0, tab_r.center().y),
                        Align2::LEFT_CENTER,
                        compact_tab_label(&active_label, ((tab_w - 28.0) / 6.2) as usize),
                        FontId::proportional(11.0),
                        p.text,
                    );
                    if close_resp.hovered() {
                        ptr.rect_filled(close_r, 0.0, p.accent.gamma_multiply(0.45));
                    }
                    ptr.text(
                        close_r.center(),
                        Align2::CENTER_CENTER,
                        "×",
                        FontId::proportional(11.5),
                        if close_resp.hovered() {
                            p.text
                        } else {
                            p.text_dim
                        },
                    );
                    if close_resp
                        .on_hover_text("Close Document (Ctrl+W)")
                        .clicked()
                        || tab_resp.middle_clicked()
                    {
                        close_active = true;
                    }
                    tab_resp.context_menu(|ui| {
                        if ui.button("New Document").clicked() {
                            create_new = true;
                            ui.close();
                        }
                        if !self.tabs.is_empty() && ui.button("Close Other Tabs").clicked() {
                            close_others = true;
                            ui.close();
                        }
                        if ui.button("Close Tab").clicked() {
                            close_active = true;
                            ui.close();
                        }
                    });

                    // Parked tabs (newest first)
                    let parked_meta: Vec<(usize, String, bool)> = self
                        .tabs
                        .iter()
                        .enumerate()
                        .rev()
                        .take(visible_parked)
                        .map(|(idx, t)| (idx, t.doc_name.clone(), t.editor.session.is_dirty()))
                        .collect();

                    for (idx, name, dirty) in parked_meta {
                        let label = if dirty { format!("*{name}") } else { name };
                        let (r, resp) =
                            ui.allocate_exact_size(Vec2::new(tab_w, 21.0), Sense::click());
                        let cr = Rect::from_center_size(
                            pos2(r.right() - 10.0, r.center().y),
                            Vec2::splat(13.0),
                        );
                        let cresp = ui.interact(
                            cr,
                            ui.make_persistent_id(("parked_doc_tab_close", idx)),
                            Sense::click(),
                        );
                        let ptr = ui.painter();
                        ptr.rect_filled(r, 0.0, if resp.hovered() { p.well } else { p.panel_alt });
                        ptr.rect_stroke(
                            r,
                            0.0,
                            Stroke::new(1.0, p.stroke),
                            egui::StrokeKind::Middle,
                        );
                        ptr.text(
                            pos2(r.left() + 8.0, r.center().y),
                            Align2::LEFT_CENTER,
                            compact_tab_label(&label, ((tab_w - 28.0) / 6.2) as usize),
                            FontId::proportional(11.0),
                            p.text_dim,
                        );
                        if cresp.hovered() {
                            ptr.rect_filled(cr, 0.0, p.accent.gamma_multiply(0.45));
                        }
                        ptr.text(
                            cr.center(),
                            Align2::CENTER_CENTER,
                            "×",
                            FontId::proportional(11.5),
                            if cresp.hovered() { p.text } else { p.text_dim },
                        );
                        if cresp.clicked() || resp.middle_clicked() {
                            close_parked = Some(idx);
                        } else if resp.clicked() {
                            switch_to = Some(idx);
                        }
                    }

                    // Overflow menu if additional tabs are hidden
                    if hidden_count > 0 {
                        ui.menu_button(format!("… +{hidden_count}"), |ui| {
                            let all_parked: Vec<(usize, String)> = self
                                .tabs
                                .iter()
                                .enumerate()
                                .map(|(idx, t)| (idx, t.doc_name.clone()))
                                .collect();
                            for (idx, name) in all_parked {
                                if ui.button(name).clicked() {
                                    switch_to = Some(idx);
                                    ui.close();
                                }
                            }
                        });
                    }

                    // "+" New Document tab button
                    if ui
                        .small_button("+")
                        .on_hover_text("New Document Tab (Ctrl+N)")
                        .clicked()
                    {
                        create_new = true;
                    }
                });

                if create_new {
                    self.run_shell(menus::ShellAction::New);
                } else if close_others {
                    self.tabs.clear();
                    self.status = "Closed other document tabs".to_owned();
                } else if let Some(idx) = close_parked {
                    if idx < self.tabs.len() {
                        self.tabs.remove(idx);
                        self.status = "Closed background document tab".to_owned();
                    }
                } else if close_active {
                    self.run_shell(menus::ShellAction::CloseDocument);
                } else if let Some(idx) = switch_to {
                    self.activate_tab(idx);
                }

                // Row 2: Original | Preview | 2-Up | 4-Up + Page Selector
                egui::Frame::new()
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0, p.stroke))
                    .inner_margin(egui::Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                            for (mode, icon, label) in CanvasViewMode::ALL {
                                let active = self.view_mode == mode;
                                let w = 24.0 + label.len() as f32 * 5.8;
                                let (r, resp) =
                                    ui.allocate_exact_size(Vec2::new(w, 19.0), Sense::click());
                                let ptr = ui.painter();
                                if active {
                                    ptr.rect_filled(r, 0.0, p.selection.gamma_multiply(0.28));
                                    ptr.rect_stroke(
                                        r,
                                        0.0,
                                        Stroke::new(1.0, p.selection),
                                        egui::StrokeKind::Middle,
                                    );
                                } else if resp.hovered() {
                                    ptr.rect_filled(r, 0.0, p.well);
                                }
                                let ic_r = Rect::from_min_size(
                                    pos2(r.left() + 3.0, r.top() + 2.0),
                                    Vec2::splat(15.0),
                                );
                                vector_icons::draw(
                                    ptr,
                                    ic_r,
                                    icon,
                                    if active { p.selection } else { p.text_dim },
                                    1.0,
                                );
                                ptr.text(
                                    pos2(ic_r.right() + 3.0, r.center().y),
                                    Align2::LEFT_CENTER,
                                    label,
                                    FontId::proportional(10.5),
                                    if active { p.text } else { p.text_dim },
                                );
                                if resp.clicked() {
                                    self.view_mode = mode;
                                    self.status = format!("{label} view active");
                                }
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let page_names: Vec<String> = self
                                        .editor
                                        .document()
                                        .pages
                                        .iter()
                                        .enumerate()
                                        .map(|(i, pg)| {
                                            pg.name
                                                .as_deref()
                                                .filter(|s| !s.is_empty())
                                                .map_or_else(
                                                    || format!("Page {}", i + 1),
                                                    str::to_owned,
                                                )
                                        })
                                        .collect();
                                    let cur_name = page_names
                                        .get(self.editor.page)
                                        .cloned()
                                        .unwrap_or_else(|| "Page 1".to_owned());
                                    egui::ComboBox::from_id_salt("canvas_page_selector")
                                        .width(96.0)
                                        .selected_text(cur_name)
                                        .show_ui(ui, |ui| {
                                            for (pi, pname) in page_names.iter().enumerate() {
                                                if ui
                                                    .selectable_label(self.editor.page == pi, pname)
                                                    .clicked()
                                                {
                                                    self.editor.page = pi;
                                                    self.editor.frame = 0;
                                                    self.editor.selection = None;
                                                    self.editor.selection_set.clear();
                                                    self.raster = None;
                                                }
                                            }
                                        });
                                },
                            );
                        });
                    });
            });
    }

    /// Unified 24px Canvas Footer Bar directly below the canvas viewport:
    /// Format badge | Snap pill | Rulers/Guides toggles | State Transport | Zoom | Size | DPI.
    pub(super) fn cs6_canvas_footer_bar(&mut self, ui: &mut Ui) {
        let p = self.active_palette();
        let (pw, ph, dpi, format_lbl, state_count) = self
            .editor
            .document()
            .pages
            .get(self.editor.page)
            .map(|pg| {
                let fmt = match pg.export.as_ref().and_then(|e| e.format) {
                    Some(0) => "GIF",
                    Some(1) => "JPEG",
                    Some(3) => "TIFF/BMP",
                    Some(4) => "Animated GIF",
                    _ => "PNG32",
                };
                let states = pg.info.frame_count.max(1) as usize;
                (
                    pg.info.canvas.width.round() as i32,
                    pg.info.canvas.height.round() as i32,
                    pg.info.canvas.resolution.round() as u32,
                    fmt.to_owned(),
                    states,
                )
            })
            .unwrap_or((1920, 1080, 144, "PNG32".to_owned(), 1));

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
            ui.label(
                RichText::new(format!("{format_lbl} (Document)"))
                    .small()
                    .color(p.text_dim),
            );
            ui.separator();

            let snap_on = self.snap_to_guides || self.snap_to_grid || self.snap_to_smart_guides;
            if ui
                .selectable_label(
                    snap_on,
                    RichText::new(if snap_on { "Snap: On" } else { "Snap: Off" }).small(),
                )
                .on_hover_text("Toggle Smart Guides & Grid/Guide Snapping")
                .clicked()
            {
                let next = !snap_on;
                self.snap_to_guides = next;
                self.snap_to_smart_guides = next;
                self.snap_to_grid = next;
                self.status = if next {
                    "Snapping enabled".to_owned()
                } else {
                    "Snapping disabled".to_owned()
                };
            }

            ui.separator();

            if ui
                .selectable_label(self.show_rulers, RichText::new("Rulers").small())
                .on_hover_text("Toggle Canvas Rulers (Ctrl+Alt+R)")
                .clicked()
            {
                self.show_rulers = !self.show_rulers;
            }
            if ui
                .selectable_label(self.show_guides, RichText::new("Guides").small())
                .on_hover_text("Toggle Guides")
                .clicked()
            {
                self.show_guides = !self.show_guides;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // DPI ComboBox
                let cur_dpi = if dpi == 0 { 144 } else { dpi };
                egui::ComboBox::from_id_salt("canvas_footer_dpi")
                    .width(66.0)
                    .selected_text(format!("{cur_dpi} ppi"))
                    .show_ui(ui, |ui| {
                        for &preset_dpi in constants::DPI_PRESETS {
                            if ui
                                .selectable_label(
                                    cur_dpi == preset_dpi,
                                    format!("{preset_dpi} ppi"),
                                )
                                .clicked()
                            {
                                self.run_command(
                                    "document.setResolution",
                                    serde_json::json!({
                                        "resolution": f64::from(preset_dpi),
                                        "unit": "inch",
                                    }),
                                );
                            }
                        }
                    });

                // Canvas Size ComboBox
                egui::ComboBox::from_id_salt("canvas_footer_res")
                    .width(116.0)
                    .selected_text(format!("{pw} × {ph}"))
                    .show_ui(ui, |ui| {
                        for &(label, w, h) in constants::RESOLUTION_PRESETS {
                            let active = (pw as f32 - w).abs() < 0.5 && (ph as f32 - h).abs() < 0.5;
                            if ui.selectable_label(active, label).clicked() {
                                self.run_command(
                                    "page.setCanvasSize",
                                    serde_json::json!({
                                        "page": self.editor.page,
                                        "width": f64::from(w),
                                        "height": f64::from(h),
                                    }),
                                );
                                self.raster = None;
                            }
                        }
                        ui.separator();
                        if ui.button("Custom Canvas Size…").clicked() {
                            self.run_shell(menus::ShellAction::CanvasSize);
                        }
                    });

                // Zoom ComboBox
                let zoom_pct = (self.view.zoom * 100.0).round() as i32;
                egui::ComboBox::from_id_salt("canvas_footer_zoom")
                    .width(60.0)
                    .selected_text(format!("{zoom_pct}%"))
                    .show_ui(ui, |ui| {
                        for pct in [12, 25, 33, 50, 66, 100, 150, 200, 300, 400] {
                            if ui
                                .selectable_label(zoom_pct == pct, format!("{pct}%"))
                                .clicked()
                            {
                                self.set_zoom_exact(pct as f32 / 100.0);
                            }
                        }
                        ui.separator();
                        if ui.button("Fit in Window").clicked() {
                            self.fit_canvas_in_window();
                        }
                    });

                ui.separator();

                // Canonical State Transport (RTL order: Last, Next, "1 / N", Play/Pause, Prev, First)
                let cur_state = self.editor.frame.min(state_count - 1);
                let can_prev = cur_state > 0;
                let can_next = cur_state + 1 < state_count;
                let can_play = state_count > 1;

                if footer_icon_button(ui, Icon::StateLast, can_next, "Last State") && can_next {
                    self.editor.frame = state_count - 1;
                    self.raster = None;
                }
                if footer_icon_button(ui, Icon::StateNext, can_next, "Next State") && can_next {
                    self.editor.frame = (cur_state + 1).min(state_count - 1);
                    self.raster = None;
                }
                ui.label(
                    RichText::new(format!("{} / {}", cur_state + 1, state_count))
                        .small()
                        .strong(),
                );
                let play_icon = if self.playback.playing() {
                    Icon::StatePause
                } else {
                    Icon::StatePlay
                };
                if footer_icon_button(
                    ui,
                    play_icon,
                    can_play,
                    if self.playback.playing() {
                        "Pause Animation"
                    } else {
                        "Play Animation"
                    },
                ) && can_play
                {
                    if self.playback.playing() {
                        self.playback.stop();
                    } else if let Some(strip) =
                        timeline::TimelineStrip::read(self.editor.document(), self.editor.page)
                    {
                        self.start_playback(self.editor.page, self.editor.frame, &strip);
                    }
                }
                if footer_icon_button(ui, Icon::StatePrev, can_prev, "Previous State") && can_prev {
                    self.editor.frame = cur_state.saturating_sub(1);
                    self.raster = None;
                }
                if footer_icon_button(ui, Icon::StateFirst, can_prev, "First State") && can_prev {
                    self.editor.frame = 0;
                    self.raster = None;
                }
            });
        });
    }

    /// Crisp 22px Bottom Status Bar.
    pub(super) fn cs6_status_bar(&self, ui: &mut Ui) {
        let p = self.active_palette();
        let zoom_pct = (self.view.zoom * 100.0).round() as i32;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Zoom: {zoom_pct}%"))
                    .small()
                    .color(p.text),
            );
            ui.separator();
            ui.label(
                RichText::new(format!("Tool: {}", tool_label(self.editor.tool)))
                    .small()
                    .color(p.text),
            );
            if let Some(pos) = self.cursor_doc_pos {
                ui.separator();
                ui.label(
                    RichText::new(format!("X: {:.0}, Y: {:.0}", pos.x, pos.y))
                        .small()
                        .color(p.text_dim),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} · page {} · state {} · undo {} · redo {}",
                        self.doc_name,
                        self.editor.page + 1,
                        self.editor.frame + 1,
                        self.editor.session.undo_len(),
                        self.editor.session.redo_len(),
                    ))
                    .small()
                    .color(p.text_dim),
                );
                if !self.status.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(&self.status).small().color(p.text));
                }
            });
        });
    }

    /// Crisp CS6 top horizontal & left vertical Canvas Rulers (`px / in / cm / mm`).
    pub(super) fn draw_canvas_rulers(&mut self, ui: &mut Ui, rect: Rect) {
        if !self.show_rulers {
            return;
        }
        let p = self.active_palette();
        let dpi = self
            .editor
            .document()
            .pages
            .get(self.editor.page)
            .map(|pg| pg.info.canvas.resolution as f32)
            .filter(|d| *d > 1.0)
            .unwrap_or(144.0);
        let ruler_sz = 16.0;
        let top_bar = Rect::from_min_size(rect.min, Vec2::new(rect.width(), ruler_sz));
        let left_bar = Rect::from_min_size(rect.min, Vec2::new(ruler_sz, rect.height()));
        let corner = Rect::from_min_size(rect.min, Vec2::splat(ruler_sz));

        let painter = ui.painter();
        painter.rect_filled(top_bar, 0.0, p.panel_alt);
        painter.rect_filled(left_bar, 0.0, p.panel_alt);
        painter.line_segment(
            [
                pos2(rect.left(), rect.top() + ruler_sz),
                pos2(rect.right(), rect.top() + ruler_sz),
            ],
            Stroke::new(1.0, p.stroke),
        );
        painter.line_segment(
            [
                pos2(rect.left() + ruler_sz, rect.top()),
                pos2(rect.left() + ruler_sz, rect.bottom()),
            ],
            Stroke::new(1.0, p.stroke),
        );

        let zoom = self.view.zoom.max(0.01);
        let origin_screen = self.view.to_screen(geom::Point::new(0.0, 0.0));
        let origin_x = rect.left() + origin_screen.x;
        let origin_y = rect.top() + origin_screen.y;

        let px_per_unit = self.ruler_unit.px_per_unit(dpi);
        let screen_per_unit = px_per_unit * zoom;
        let step_units: f32 = if screen_per_unit < 0.8 {
            100.0
        } else if screen_per_unit < 2.0 {
            50.0
        } else if screen_per_unit < 8.0 {
            10.0
        } else if screen_per_unit < 25.0 {
            5.0
        } else {
            1.0
        };
        let step_px = step_units * px_per_unit;
        let subdivs = self.ruler_unit.subdivisions().max(2);

        // Horizontal ruler ticks
        let start_doc_x = ((rect.left() + ruler_sz - origin_x) / zoom / step_px).floor() as i32;
        let end_doc_x = ((rect.right() - origin_x) / zoom / step_px).ceil() as i32;
        for i in start_doc_x..=end_doc_x.min(start_doc_x + 200) {
            let doc_x = i as f32 * step_px;
            let sx = origin_x + doc_x * zoom;
            if sx >= rect.left() + ruler_sz && sx <= rect.right() {
                painter.line_segment(
                    [pos2(sx, rect.top() + 5.0), pos2(sx, rect.top() + ruler_sz)],
                    Stroke::new(1.0, p.text_dim),
                );
                let label_val = i as f32 * step_units;
                let lbl = if (label_val.fract()).abs() < 0.01 {
                    format!("{:.0}", label_val)
                } else {
                    format!("{:.1}", label_val)
                };
                painter.text(
                    pos2(sx + 2.0, rect.top() + 1.0),
                    Align2::LEFT_TOP,
                    lbl,
                    FontId::proportional(8.5),
                    p.text_dim,
                );
            }
            for s in 1..subdivs {
                let sub_sx = sx + (step_px * zoom) * (s as f32 / subdivs as f32);
                if sub_sx >= rect.left() + ruler_sz && sub_sx <= rect.right() {
                    painter.line_segment(
                        [
                            pos2(sub_sx, rect.top() + 11.0),
                            pos2(sub_sx, rect.top() + ruler_sz),
                        ],
                        Stroke::new(0.8, p.stroke),
                    );
                }
            }
        }

        // Vertical ruler ticks
        let start_doc_y = ((rect.top() + ruler_sz - origin_y) / zoom / step_px).floor() as i32;
        let end_doc_y = ((rect.bottom() - origin_y) / zoom / step_px).ceil() as i32;
        for i in start_doc_y..=end_doc_y.min(start_doc_y + 200) {
            let doc_y = i as f32 * step_px;
            let sy = origin_y + doc_y * zoom;
            if sy >= rect.top() + ruler_sz && sy <= rect.bottom() {
                painter.line_segment(
                    [
                        pos2(rect.left() + 5.0, sy),
                        pos2(rect.left() + ruler_sz, sy),
                    ],
                    Stroke::new(1.0, p.text_dim),
                );
                let label_val = i as f32 * step_units;
                let lbl = format!("{:.0}", label_val);
                painter.text(
                    pos2(rect.left() + 1.0, sy + 1.0),
                    Align2::LEFT_TOP,
                    lbl,
                    FontId::proportional(8.0),
                    p.text_dim,
                );
            }
            for s in 1..subdivs {
                let sub_sy = sy + (step_px * zoom) * (s as f32 / subdivs as f32);
                if sub_sy >= rect.top() + ruler_sz && sub_sy <= rect.bottom() {
                    painter.line_segment(
                        [
                            pos2(rect.left() + 11.0, sub_sy),
                            pos2(rect.left() + ruler_sz, sub_sy),
                        ],
                        Stroke::new(0.8, p.stroke),
                    );
                }
            }
        }

        // Live cursor tracking hairlines on rulers
        if let Some(pos) = self.cursor_doc_pos {
            let cx = origin_x + pos.x * zoom;
            let cy = origin_y + pos.y * zoom;
            if cx >= rect.left() + ruler_sz && cx <= rect.right() {
                painter.line_segment(
                    [pos2(cx, rect.top()), pos2(cx, rect.top() + ruler_sz)],
                    Stroke::new(1.0, p.selection),
                );
            }
            if cy >= rect.top() + ruler_sz && cy <= rect.bottom() {
                painter.line_segment(
                    [pos2(rect.left(), cy), pos2(rect.left() + ruler_sz, cy)],
                    Stroke::new(1.0, p.selection),
                );
            }
        }

        // Unit toggle corner box ("px" / "in" / "cm" / "mm")
        painter.rect_filled(corner, 0.0, p.panel);
        painter.rect_stroke(
            corner,
            0.0,
            Stroke::new(1.0, p.stroke),
            egui::StrokeKind::Inside,
        );
        painter.text(
            corner.center(),
            Align2::CENTER_CENTER,
            self.ruler_unit.short_code(),
            FontId::proportional(8.5),
            p.text_dim,
        );
        let corner_resp = ui.interact(
            corner,
            ui.make_persistent_id("cs6_ruler_unit_corner"),
            Sense::click(),
        );
        if corner_resp
            .on_hover_text("Click to cycle ruler units (px / in / cm / mm)")
            .clicked()
        {
            let all = constants::RulerUnit::ALL;
            let idx = all.iter().position(|u| *u == self.ruler_unit).unwrap_or(0);
            self.ruler_unit = all[(idx + 1) % all.len()];
            self.status = format!("Ruler units: {}", self.ruler_unit.label());
        }
    }

    /// Bottom Resizable CS6 Properties Inspector Dock (`Properties | Symbol Properties | Document Info`).
    pub(super) fn cs6_bottom_properties_dock(&mut self, ui: &mut Ui) {
        let p = self.active_palette();
        const TABS: &[&str] = &["◊ Properties", "Symbol Properties", "Document Info"];

        // Slim owner-controlled resize grip at top edge when expanded
        if !self.bottom_collapsed {
            let (grip, grip_resp) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 4.0), Sense::drag());
            if grip_resp.hovered() || grip_resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                ui.painter().line_segment(
                    [grip.left_center(), grip.right_center()],
                    Stroke::new(1.5, p.selection),
                );
            }
            if grip_resp.dragged() {
                let dy = ui.ctx().input(|i| i.pointer.delta().y);
                self.bottom_height = (self.bottom_height - dy).clamp(118.0, 340.0);
                ui.ctx().request_repaint();
            }
            grip_resp.on_hover_text("Drag to resize Properties Inspector");
        }

        // CS6 Panel Header Bar (single header, collapse chevron + sculpted tabs)
        let (bar, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.0), Sense::hover());
        let ptr = ui.painter();
        ptr.rect_filled(bar, 0.0, p.panel_alt);
        ptr.line_segment(
            [bar.left_bottom(), bar.right_bottom()],
            Stroke::new(1.0, p.stroke),
        );

        // Collapse chevron button
        let chev_r =
            Rect::from_min_size(pos2(bar.left() + 3.0, bar.top() + 3.0), Vec2::splat(14.0));
        let chev_resp = ui.interact(
            chev_r,
            ui.make_persistent_id("bottom_inspector_collapse"),
            Sense::click(),
        );
        if chev_resp.hovered() {
            ptr.rect_filled(chev_r, 0.0, p.well);
        }
        let (cx, cy) = (chev_r.center().x, chev_r.center().y);
        let chev_col = if chev_resp.hovered() {
            p.selection
        } else {
            p.text_dim
        };
        if self.bottom_collapsed {
            ptr.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - 2.5, cy - 3.5),
                    pos2(cx + 2.5, cy + 0.5),
                    pos2(cx - 2.5, cy + 3.5),
                ],
                chev_col,
                Stroke::NONE,
            ));
        } else {
            ptr.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - 3.5, cy - 2.5),
                    pos2(cx + 3.5, cy - 2.5),
                    pos2(cx, cy + 2.5),
                ],
                chev_col,
                Stroke::NONE,
            ));
        }
        if chev_resp
            .on_hover_text(if self.bottom_collapsed {
                "Expand Properties Inspector"
            } else {
                "Collapse Properties Inspector"
            })
            .clicked()
        {
            self.bottom_collapsed = !self.bottom_collapsed;
        }

        let mut tx = bar.left() + 22.0;
        for (i, tab_name) in TABS.iter().enumerate() {
            let w = (14.0 + tab_name.len() as f32 * 5.8).min(120.0);
            let tr = Rect::from_min_size(pos2(tx, bar.top() + 1.0), Vec2::new(w, 19.0));
            let resp = ui.interact(tr, ui.make_persistent_id(("bottom_tab", i)), Sense::click());
            let is_active = self.bottom_tab == i;
            if is_active {
                ptr.rect_filled(tr, 0.0, p.panel);
                ptr.line_segment(
                    [tr.left_top(), tr.right_top()],
                    Stroke::new(1.5, p.selection),
                );
                ptr.line_segment(
                    [tr.left_top(), tr.left_bottom()],
                    Stroke::new(1.0, p.stroke),
                );
                ptr.line_segment(
                    [tr.right_top(), tr.right_bottom()],
                    Stroke::new(1.0, p.stroke),
                );
            } else {
                ptr.rect_filled(tr, 0.0, if resp.hovered() { p.well } else { p.panel_alt });
                ptr.rect_stroke(
                    tr,
                    0.0,
                    Stroke::new(1.0, p.stroke),
                    egui::StrokeKind::Middle,
                );
            }
            ptr.text(
                tr.center(),
                Align2::CENTER_CENTER,
                *tab_name,
                FontId::proportional(10.5),
                if is_active { p.text } else { p.text_dim },
            );
            if resp.clicked() {
                self.bottom_tab = i;
                self.bottom_collapsed = false;
            }
            tx += w + 2.0;
        }

        if self.bottom_collapsed {
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("bottom_inspector_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| match self.bottom_tab {
                0 => self.cs6_horizontal_properties_body(ui),
                1 => self.cs6_symbol_properties_body(ui),
                _ => self.info_panel(ui),
            });
    }

    /// Horizontal CS6 Properties Inspector body:
    /// - No selection: Document thumbnail, Canvas color swatch, Canvas Size..., Image Size..., Fit Canvas, Resolution/DPI.
    /// - Selection: Object thumbnail + Name + 2×2 W/H/X/Y + Fill/Stroke/Opacity/Blend + quick live filters & full properties.
    fn cs6_horizontal_properties_body(&mut self, ui: &mut Ui) {
        let p = self.active_palette();
        let selection = self.editor.live_selection();

        if selection.is_empty() {
            let (pw, ph, dpi, bg_raw, fmt, state_count) = self
                .editor
                .document()
                .pages
                .get(self.editor.page)
                .map(|pg| {
                    let fmt = match pg.export.as_ref().and_then(|e| e.format) {
                        Some(0) => "GIF",
                        Some(1) => "JPEG",
                        Some(3) => "TIFF/BMP",
                        Some(4) => "Animated GIF",
                        _ => "PNG32",
                    };
                    (
                        pg.info.canvas.width.round() as i32,
                        pg.info.canvas.height.round() as i32,
                        pg.info.canvas.resolution.round() as u32,
                        pg.info.canvas.background_raw,
                        fmt.to_owned(),
                        pg.info.frame_count.max(1) as usize,
                    )
                })
                .unwrap_or((
                    1920,
                    1080,
                    144,
                    abgr_pack([255, 255, 255, 255]),
                    "PNG32".to_owned(),
                    1,
                ));

            let bg_c32 = abgr_to_color32(bg_raw);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // Document Thumbnail Box
                let (thumb_r, _) = ui.allocate_exact_size(Vec2::new(34.0, 42.0), Sense::hover());
                ui.painter().rect_filled(thumb_r, 0.0, bg_c32);
                ui.painter().rect_stroke(
                    thumb_r,
                    0.0,
                    Stroke::new(1.0, p.stroke),
                    egui::StrokeKind::Middle,
                );

                ui.vertical(|ui| {
                    ui.label(RichText::new("Document").small().strong().color(p.selection));
                    ui.label(RichText::new(&self.doc_name).small().color(p.text));
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Canvas:").small().color(p.text_dim));
                        let mut edit_bg = bg_c32;
                        if ui.color_edit_button_srgba(&mut edit_bg).changed() {
                            let raw = abgr_pack([
                                edit_bg.r(),
                                edit_bg.g(),
                                edit_bg.b(),
                                edit_bg.a(),
                            ]);
                            self.run_command(
                                "document.setCanvasColor",
                                serde_json::json!({ "color_raw": raw }),
                            );
                            self.raster = None;
                        }
                        if ui.button("Canvas Size…").clicked() {
                            self.run_shell(menus::ShellAction::CanvasSize);
                        }
                        if ui.button("Image Size…").clicked() {
                            self.run_shell(menus::ShellAction::ImageSize);
                        }
                        if ui.button("Fit Canvas").clicked() {
                            self.run_command(
                                "document.setDocumentCanvasSizeToDocumentExtents",
                                serde_json::json!({}),
                            );
                            self.raster = None;
                        }
                        ui.label(RichText::new("Size:").small().color(p.text_dim));
                        egui::ComboBox::from_id_salt("bottom_prop_size")
                            .width(124.0)
                            .selected_text(format!("{pw}×{ph} px"))
                            .show_ui(ui, |ui| {
                                for &(label, w, h) in constants::RESOLUTION_PRESETS {
                                    if ui
                                        .selectable_label(
                                            (pw as f32 - w).abs() < 0.5
                                                && (ph as f32 - h).abs() < 0.5,
                                            label,
                                        )
                                        .clicked()
                                    {
                                        self.run_command(
                                            "page.setCanvasSize",
                                            serde_json::json!({
                                                "page": self.editor.page,
                                                "width": f64::from(w),
                                                "height": f64::from(h),
                                            }),
                                        );
                                        self.raster = None;
                                    }
                                }
                            });
                        ui.label(RichText::new("DPI:").small().color(p.text_dim));
                        let cur_dpi = if dpi == 0 { 144 } else { dpi };
                        egui::ComboBox::from_id_salt("bottom_prop_dpi")
                            .width(74.0)
                            .selected_text(format!("{cur_dpi} ppi"))
                            .show_ui(ui, |ui| {
                                for &preset_dpi in constants::DPI_PRESETS {
                                    if ui
                                        .selectable_label(
                                            cur_dpi == preset_dpi,
                                            format!("{preset_dpi} ppi"),
                                        )
                                        .clicked()
                                    {
                                        self.run_command(
                                            "document.setResolution",
                                            serde_json::json!({
                                                "resolution": f64::from(preset_dpi),
                                                "unit": "inch",
                                            }),
                                        );
                                    }
                                }
                            });
                    });
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!(
                            "Dimensions: {pw}×{ph} px @ {} dpi · Export: {fmt} · State {} of {state_count} · {} object(s)",
                            if dpi == 0 { 144 } else { dpi },
                            self.editor.frame + 1,
                            self.editor.document().registry.len(),
                        ))
                        .small()
                        .color(p.text_dim),
                    );
                });
            });
            return;
        }

        // Single or Multi selection: crisp horizontal CS6 bar + full FireCraft inspector controls
        let first_id = selection[0];
        let meta = self.editor.document().registry.get(first_id).map(|obj| {
            let (kind_lbl, icon, name, opacity) = match obj {
                ObjectData::Path(path) => (
                    "Vector Path",
                    Icon::Rectangle,
                    path.name.clone().unwrap_or_else(|| "Path".to_owned()),
                    path.opacity,
                ),
                ObjectData::Text(t) => (
                    "Text",
                    Icon::Text,
                    t.name.clone().unwrap_or_else(|| "Text".to_owned()),
                    t.opacity,
                ),
                ObjectData::Image(img) => (
                    "Bitmap Image",
                    Icon::Brush,
                    img.name.clone().unwrap_or_else(|| "Bitmap".to_owned()),
                    img.opacity,
                ),
                ObjectData::Group(g) => (
                    "Group",
                    Icon::Group,
                    g.name.clone().unwrap_or_else(|| "Group".to_owned()),
                    g.opacity,
                ),
                ObjectData::Slice(s) => (
                    "Web Slice",
                    Icon::Slice,
                    s.name.clone().unwrap_or_else(|| "Slice".to_owned()),
                    1000,
                ),
                ObjectData::SymbolInstance(sym) => (
                    "Symbol Instance",
                    Icon::Star,
                    sym.name.clone().unwrap_or_else(|| "Symbol".to_owned()),
                    sym.opacity,
                ),
            };
            (kind_lbl, icon, name, opacity)
        });

        let bounds = union_selection_bounds(self.editor.document(), &selection);
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            if let Some((kind_lbl, icon, mut name, opacity_milli)) = meta {
                let (thumb_r, _) = ui.allocate_exact_size(Vec2::new(30.0, 38.0), Sense::hover());
                ui.painter().rect_filled(thumb_r, 0.0, p.well);
                ui.painter().rect_stroke(
                    thumb_r,
                    0.0,
                    Stroke::new(1.0, p.stroke),
                    egui::StrokeKind::Middle,
                );
                vector_icons::draw(ui.painter(), thumb_r.shrink(3.0), icon, p.selection, 1.2);

                ui.vertical(|ui| {
                    let hdr = if selection.len() > 1 {
                        format!("{} Objects Selected", selection.len())
                    } else {
                        kind_lbl.to_owned()
                    };
                    ui.label(RichText::new(hdr).small().strong().color(p.selection));
                    if selection.len() == 1
                        && ui
                            .add(egui::TextEdit::singleline(&mut name).desired_width(116.0))
                            .lost_focus()
                    {
                        self.run_command(
                            "object.rename",
                            serde_json::json!({ "id": first_id, "name": name }),
                        );
                    }
                });

                if let Some(b) = bounds {
                    ui.separator();
                    let mut x = b.x0;
                    let mut y = b.y0;
                    let mut w = (b.x1 - b.x0).max(1.0);
                    let mut h = (b.y1 - b.y0).max(1.0);
                    let mut changed = false;
                    egui::Grid::new("bottom_prop_xywh")
                        .num_columns(4)
                        .spacing([4.0, 2.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("W:").small().color(p.text_dim));
                            changed |= ui
                                .add(egui::DragValue::new(&mut w).speed(1.0).range(1.0..=65535.0))
                                .changed();
                            ui.label(RichText::new("X:").small().color(p.text_dim));
                            changed |= ui.add(egui::DragValue::new(&mut x).speed(1.0)).changed();
                            ui.end_row();
                            ui.label(RichText::new("H:").small().color(p.text_dim));
                            changed |= ui
                                .add(egui::DragValue::new(&mut h).speed(1.0).range(1.0..=65535.0))
                                .changed();
                            ui.label(RichText::new("Y:").small().color(p.text_dim));
                            changed |= ui.add(egui::DragValue::new(&mut y).speed(1.0)).changed();
                            ui.end_row();
                        });
                    if changed && selection.len() == 1 {
                        self.run_command(
                            "object.setBounds",
                            serde_json::json!({
                                "id": first_id,
                                "x0": x,
                                "y0": y,
                                "x1": x + w,
                                "y1": y + h,
                            }),
                        );
                    }
                }

                ui.separator();

                // Opacity & quick actions
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Opacity:").small().color(p.text_dim));
                        let mut pct = (opacity_milli as f64 / 10.0).clamp(0.0, 100.0);
                        if ui
                            .add(
                                egui::DragValue::new(&mut pct)
                                    .range(0.0..=100.0)
                                    .suffix("%"),
                            )
                            .changed()
                        {
                            let raw = (pct * 10.0).round() as i64;
                            for id in &selection {
                                self.run_command(
                                    "object.setOpacity",
                                    serde_json::json!({ "id": id, "opacity": raw }),
                                );
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.small_button("Duplicate").clicked() {
                            self.run_sel(menus::SelAction::Duplicate);
                        }
                        if ui.small_button("Delete").clicked() {
                            self.run_sel(menus::SelAction::Delete);
                        }
                        if selection.len() >= 2 && ui.small_button("Group").clicked() {
                            self.run_sel(menus::SelAction::Group);
                        }
                    });
                });
            }
        });

        // Below the crisp CS6 summary row, expose FireCraft's deep inspector (Fill/Stroke/Filters/Text/Masks)
        ui.separator();
        self.properties_panel(ui);
    }

    fn cs6_symbol_properties_body(&mut self, ui: &mut Ui) {
        let p = self.active_palette();
        let sym_count = self.editor.document().symbols.len();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Symbol Library Instances:")
                    .small()
                    .color(p.text_dim),
            );
            if ui
                .small_button("Convert Selection to Symbol (F8)")
                .clicked()
            {
                self.run_sel(menus::SelAction::ConvertToSymbol);
            }
            ui.label(
                RichText::new(format!(
                    "{sym_count} symbol definition(s) in current document"
                ))
                .small()
                .color(p.text),
            );
        });
    }
}
