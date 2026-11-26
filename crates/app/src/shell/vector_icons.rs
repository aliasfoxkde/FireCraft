#![allow(
    dead_code,
    clippy::too_many_lines,
    clippy::suboptimal_flops,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::missing_docs_in_private_items
)]
//! Small line-art icons drawn with the painter, so the app needs no icon font.

use egui::{self, Color32, Painter, Pos2, Rect, Stroke, Vec2, pos2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    New,
    Open,
    Save,
    Import,
    Export,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Undo,
    Redo,
    Select,
    Subselect,
    Scale,
    Marquee,
    Lasso,
    Wand,
    Crop,
    Hand,
    Zoom,
    Brush,
    Pencil,
    Eraser,
    Stamp,
    Line,
    Rectangle,
    Rounded,
    Ellipse,
    Polygon,
    Star,
    Pen,
    Knife,
    Text,
    Picker,
    Fill,
    Gradient,
    Blur,
    Sharpen,
    Dodge,
    Burn,
    Slice,
    Hotspot,
    ZoomIn,
    ZoomOut,
    Fit,
    Group,
    Ungroup,
    BringFront,
    BringForward,
    SendBackward,
    SendBack,
    RotateCCW,
    RotateCW,
    FlipH,
    FlipV,
    Eye,
    EyeOff,
    Lock,
    Unlock,
    Trash,
    Folder,
    StateFirst,
    StatePrev,
    StatePlay,
    StatePause,
    StateNext,
    StateLast,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AlignIcon {
    RelativeToObject,
    RelativeToCanvas,
    AlignLeft,
    AlignCenterH,
    AlignRight,
    AlignTop,
    AlignCenterV,
    AlignBottom,
    DistributeTop,
    DistributeCenterV,
    DistributeBottom,
    DistributeLeft,
    DistributeCenterH,
    DistributeRight,
    MatchWidth,
    MatchHeight,
    MatchBoth,
    SpaceVertical,
    SpaceHorizontal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PathIcon {
    // Combine Paths (6)
    Join,
    Split,
    Union,
    Intersect,
    Punch,
    Crop,
    // Alter Paths Row 1 (7)
    Simplify,
    ExpandStroke,
    InsetExpand,
    HardEdge,
    AntiAlias,
    FeatherEdge,
    StrokeToFill,
    // Alter Paths Row 2 (6)
    ReverseDirection,
    ClosePath,
    OpenPath,
    PathToMarquee,
    SnapToPixel,
    FlattenPath,
    // Edit Points Row 1 (7)
    AddPoints,
    DeletePoints,
    CornerPoint,
    SmoothPoint,
    RoundPoints,
    FilletPoints,
    ChamferPoints,
    // Edit Points Row 2 (7)
    BreakPath,
    JoinPoints,
    AlignPointsH,
    AlignPointsV,
    KnifeSplit,
    ExtendPath,
    SharpApex,
    // Select Points Row 1 (5)
    InversePoints,
    SelectAllPoints,
    FirstPoint,
    PrevPoint,
    NextPoint,
}

fn stroke_of(c: Color32, w: f32) -> Stroke {
    Stroke::new(w, c)
}

fn arc_points(center: Pos2, radius: f32, start: f32, end: f32, steps: usize) -> Vec<Pos2> {
    (0..=steps)
        .map(|i| {
            let t = start + (end - start) * i as f32 / steps as f32;
            pos2(center.x + radius * t.cos(), center.y + radius * t.sin())
        })
        .collect()
}

/// Draws `icon` inside `rect` (a square button face).
pub fn draw(painter: &Painter, rect: Rect, icon: Icon, color: Color32, width: f32) {
    let r = rect.shrink(3.0);
    let cx = r.center().x;
    let cy = r.center().y;
    let s = r.width().min(r.height());
    let u = s * 0.5;
    let st = stroke_of(color, width.max(1.0));
    let dot = |p: Pos2| painter.circle_filled(p, 1.4, color);
    let accent = Color32::from_rgb(232, 163, 61);
    let accent_blue = Color32::from_rgb(78, 168, 240);
    match icon {
        Icon::New => {
            // CS6 crisp page with folded dog-ear & amber '+' badge
            let doc_r = Rect::from_center_size(r.center(), Vec2::new(s * 0.72, s * 0.88));
            painter.rect_filled(doc_r, 0.0, color.gamma_multiply(0.14));
            painter.rect_stroke(
                doc_r,
                0.0,
                Stroke::new(1.2, color),
                egui::StrokeKind::Middle,
            );
            painter.line_segment(
                [pos2(cx, cy - u * 0.30), pos2(cx, cy + u * 0.30)],
                stroke_of(accent, 1.6),
            );
            painter.line_segment(
                [pos2(cx - u * 0.30, cy), pos2(cx + u * 0.30, cy)],
                stroke_of(accent, 1.6),
            );
        }
        Icon::Open => {
            // CS6 folder tab + front flap
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.82, cy + u * 0.62),
                    pos2(cx - u * 0.82, cy - u * 0.52),
                    pos2(cx - u * 0.22, cy - u * 0.52),
                    pos2(cx - u * 0.04, cy - u * 0.26),
                    pos2(cx + u * 0.82, cy - u * 0.26),
                    pos2(cx + u * 0.82, cy + u * 0.62),
                ],
                accent.gamma_multiply(0.28),
                Stroke::new(1.2, color),
            ));
            painter.line_segment(
                [
                    pos2(cx - u * 0.76, cy + u * 0.02),
                    pos2(cx + u * 0.76, cy + u * 0.02),
                ],
                Stroke::new(1.0, accent),
            );
        }
        Icon::Save => {
            // CS6 floppy disk with top metal shutter & bottom label area
            painter.rect_filled(r.shrink(0.5), 0.0, accent_blue.gamma_multiply(0.20));
            painter.rect_stroke(
                r.shrink(0.5),
                0.0,
                Stroke::new(1.2, color),
                egui::StrokeKind::Middle,
            );
            painter.rect_filled(
                Rect::from_min_size(
                    pos2(cx - u * 0.45, cy - u * 0.82),
                    Vec2::new(u * 0.9, u * 0.48),
                ),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_min_size(
                    pos2(cx - u * 0.52, cy + u * 0.14),
                    Vec2::new(u * 1.04, u * 0.62),
                ),
                0.0,
                color.gamma_multiply(0.45),
            );
        }
        Icon::Import => {
            painter.line_segment(
                [pos2(cx, cy - u * 0.8), pos2(cx, cy + u * 0.45)],
                stroke_of(accent_blue, 1.5),
            );
            painter.line_segment(
                [pos2(cx - u * 0.38, cy + u * 0.08), pos2(cx, cy + u * 0.52)],
                stroke_of(accent_blue, 1.5),
            );
            painter.line_segment(
                [pos2(cx + u * 0.38, cy + u * 0.08), pos2(cx, cy + u * 0.52)],
                stroke_of(accent_blue, 1.5),
            );
            painter.line_segment(
                [
                    pos2(cx - u * 0.75, cy + u * 0.8),
                    pos2(cx + u * 0.75, cy + u * 0.8),
                ],
                st,
            );
        }
        Icon::Export => {
            painter.line_segment(
                [pos2(cx, cy + u * 0.45), pos2(cx, cy - u * 0.58)],
                stroke_of(accent, 1.5),
            );
            painter.line_segment(
                [pos2(cx - u * 0.38, cy - u * 0.18), pos2(cx, cy - u * 0.62)],
                stroke_of(accent, 1.5),
            );
            painter.line_segment(
                [pos2(cx + u * 0.38, cy - u * 0.18), pos2(cx, cy - u * 0.62)],
                stroke_of(accent, 1.5),
            );
            painter.line_segment(
                [
                    pos2(cx - u * 0.75, cy + u * 0.8),
                    pos2(cx + u * 0.75, cy + u * 0.8),
                ],
                st,
            );
        }
        Icon::Cut => {
            // Scissors blades + finger rings
            painter.circle_stroke(
                pos2(cx - u * 0.42, cy + u * 0.52),
                u * 0.28,
                stroke_of(accent, 1.3),
            );
            painter.circle_stroke(
                pos2(cx + u * 0.42, cy + u * 0.52),
                u * 0.28,
                stroke_of(accent, 1.3),
            );
            painter.line_segment(
                [
                    pos2(cx - u * 0.28, cy + u * 0.28),
                    pos2(cx + u * 0.48, cy - u * 0.72),
                ],
                st,
            );
            painter.line_segment(
                [
                    pos2(cx + u * 0.28, cy + u * 0.28),
                    pos2(cx - u * 0.48, cy - u * 0.72),
                ],
                st,
            );
        }
        Icon::Copy => {
            let back = Rect::from_min_size(
                pos2(cx - u * 0.72, cy - u * 0.72),
                Vec2::new(u * 0.92, u * 1.12),
            );
            let front = Rect::from_min_size(
                pos2(cx - u * 0.16, cy - u * 0.24),
                Vec2::new(u * 0.92, u * 1.12),
            );
            painter.rect_stroke(back, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_filled(front, 0.0, accent_blue.gamma_multiply(0.24));
            painter.rect_stroke(front, 0.0, st, egui::StrokeKind::Middle);
        }
        Icon::Paste => {
            let board =
                Rect::from_center_size(pos2(cx, cy + u * 0.16), Vec2::new(u * 1.25, u * 1.42));
            painter.rect_filled(board, 0.0, accent.gamma_multiply(0.22));
            painter.rect_stroke(board, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_filled(
                Rect::from_center_size(pos2(cx, cy - u * 0.58), Vec2::new(u * 0.72, u * 0.34)),
                0.0,
                accent,
            );
            let sheet = Rect::from_center_size(
                pos2(cx + u * 0.12, cy + u * 0.28),
                Vec2::new(u * 0.72, u * 0.86),
            );
            painter.rect_filled(sheet, 0.0, color.gamma_multiply(0.85));
        }
        Icon::Duplicate => {
            painter.rect_stroke(
                Rect::from_center_size(
                    pos2(cx - u * 0.25, cy + u * 0.25),
                    Vec2::new(u * 1.1, u * 1.1),
                ),
                1.0,
                st,
                egui::StrokeKind::Middle,
            );
            painter.rect_stroke(
                Rect::from_center_size(
                    pos2(cx + u * 0.25, cy - u * 0.25),
                    Vec2::new(u * 1.1, u * 1.1),
                ),
                1.0,
                st,
                egui::StrokeKind::Middle,
            );
        }
        Icon::Undo => {
            let arc = arc_points(
                pos2(cx + u * 0.05, cy + u * 0.22),
                u * 0.58,
                std::f32::consts::PI * 1.05,
                std::f32::consts::TAU * 0.96,
                12,
            );
            painter.add(egui::Shape::line(
                arc,
                stroke_of(color, (width * 1.15).max(1.4)),
            ));
            let tip = pos2(cx - u * 0.54, cy + u * 0.14);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(tip.x - u * 0.22, tip.y - u * 0.12),
                    pos2(tip.x + u * 0.22, tip.y - u * 0.30),
                    pos2(tip.x + u * 0.06, tip.y + u * 0.22),
                ],
                accent_blue,
                egui::Stroke::NONE,
            ));
        }
        Icon::Redo => {
            let arc = arc_points(
                pos2(cx - u * 0.05, cy + u * 0.22),
                u * 0.58,
                std::f32::consts::PI * 1.04,
                std::f32::consts::TAU * 0.95,
                12,
            );
            painter.add(egui::Shape::line(
                arc,
                stroke_of(color, (width * 1.15).max(1.4)),
            ));
            let tip = pos2(cx + u * 0.54, cy + u * 0.14);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(tip.x + u * 0.22, tip.y - u * 0.12),
                    pos2(tip.x - u * 0.22, tip.y - u * 0.30),
                    pos2(tip.x - u * 0.06, tip.y + u * 0.22),
                ],
                accent_blue,
                egui::Stroke::NONE,
            ));
        }
        Icon::Select => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.42, cy - u * 0.82),
                    pos2(cx - u * 0.42, cy + u * 0.62),
                    pos2(cx - u * 0.05, cy + u * 0.25),
                    pos2(cx + u * 0.52, cy + u * 0.12),
                ],
                color.gamma_multiply(0.28),
                st,
            ));
            painter.line_segment(
                [
                    pos2(cx - u * 0.02, cy + u * 0.24),
                    pos2(cx + u * 0.25, cy + u * 0.84),
                ],
                stroke_of(color, 1.8),
            );
        }
        Icon::Subselect => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.35, cy - u * 0.78),
                    pos2(cx - u * 0.35, cy + u * 0.58),
                    pos2(cx + u * 0.5, cy + u * 0.08),
                ],
                Color32::WHITE.gamma_multiply(0.85),
                stroke_of(color, 1.3),
            ));
        }
        Icon::Scale => {
            let box_r = r.shrink(2.5);
            painter.rect_stroke(box_r, 0.0, st, egui::StrokeKind::Middle);
            for corner in [
                box_r.left_top(),
                box_r.right_top(),
                box_r.left_bottom(),
                box_r.right_bottom(),
            ] {
                painter.rect_filled(
                    Rect::from_center_size(corner, Vec2::splat(3.4)),
                    0.0,
                    accent_blue,
                );
            }
            // Diagonal scale arrow
            painter.line_segment(
                [
                    pos2(cx - u * 0.35, cy + u * 0.35),
                    pos2(cx + u * 0.35, cy - u * 0.35),
                ],
                stroke_of(accent, width.max(1.2)),
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx + u * 0.42, cy - u * 0.42),
                    pos2(cx + u * 0.08, cy - u * 0.42),
                    pos2(cx + u * 0.42, cy - u * 0.08),
                ],
                accent,
                egui::Stroke::NONE,
            ));
        }
        Icon::Marquee => {
            // Dashed marching-ants rectangle with accent_blue corner ticks
            let mr = r.shrink(1.0);
            let dash_st = stroke_of(color, width.max(1.1));
            for i in 0..3 {
                let t0 = i as f32 / 3.0;
                let t1 = t0 + 0.20;
                painter.line_segment(
                    [
                        pos2(mr.left() + mr.width() * t0, mr.top()),
                        pos2(mr.left() + mr.width() * t1, mr.top()),
                    ],
                    dash_st,
                );
                painter.line_segment(
                    [
                        pos2(mr.left() + mr.width() * t0, mr.bottom()),
                        pos2(mr.left() + mr.width() * t1, mr.bottom()),
                    ],
                    dash_st,
                );
                painter.line_segment(
                    [
                        pos2(mr.left(), mr.top() + mr.height() * t0),
                        pos2(mr.left(), mr.top() + mr.height() * t1),
                    ],
                    dash_st,
                );
                painter.line_segment(
                    [
                        pos2(mr.right(), mr.top() + mr.height() * t0),
                        pos2(mr.right(), mr.top() + mr.height() * t1),
                    ],
                    dash_st,
                );
            }
            painter.rect_filled(mr.shrink(3.5), 0.0, accent_blue.gamma_multiply(0.18));
        }
        Icon::Lasso => {
            painter.add(egui::Shape::ellipse_filled(
                pos2(cx, cy - u * 0.16),
                Vec2::new(u * 0.72, u * 0.50),
                accent_blue.gamma_multiply(0.14),
            ));
            painter.add(egui::Shape::ellipse_stroke(
                pos2(cx, cy - u * 0.16),
                Vec2::new(u * 0.72, u * 0.50),
                st,
            ));
            // Knot + rope tail
            painter.circle_filled(pos2(cx - u * 0.15, cy + u * 0.34), 2.0, accent);
            painter.line_segment(
                [
                    pos2(cx - u * 0.15, cy + u * 0.34),
                    pos2(cx - u * 0.45, cy + u * 0.85),
                ],
                st,
            );
        }
        Icon::Wand => {
            painter.line_segment(
                [
                    pos2(cx - u * 0.65, cy + u * 0.75),
                    pos2(cx + u * 0.28, cy - u * 0.18),
                ],
                stroke_of(color, 1.8),
            );
            painter.circle_filled(pos2(cx + u * 0.42, cy - u * 0.32), 2.4, accent);
            painter.circle_filled(pos2(cx + u * 0.70, cy - u * 0.60), 1.4, accent);
            painter.circle_filled(pos2(cx + u * 0.15, cy - u * 0.75), 1.3, accent);
            painter.circle_filled(pos2(cx + u * 0.78, cy + u * 0.05), 1.3, accent);
        }
        Icon::Crop => {
            let crop_st = stroke_of(color, (width * 1.2).max(1.4));
            painter.rect_filled(
                Rect::from_center_size(pos2(cx, cy), Vec2::splat(u * 0.92)),
                0.0,
                accent.gamma_multiply(0.16),
            );
            painter.line_segment(
                [
                    pos2(cx - u * 0.46, cy - u * 0.92),
                    pos2(cx - u * 0.46, cy + u * 0.46),
                ],
                crop_st,
            );
            painter.line_segment(
                [
                    pos2(cx - u * 0.46, cy + u * 0.46),
                    pos2(cx + u * 0.92, cy + u * 0.46),
                ],
                crop_st,
            );
            painter.line_segment(
                [
                    pos2(cx + u * 0.46, cy - u * 0.46),
                    pos2(cx + u * 0.46, cy + u * 0.92),
                ],
                crop_st,
            );
            painter.line_segment(
                [
                    pos2(cx - u * 0.92, cy - u * 0.46),
                    pos2(cx + u * 0.46, cy - u * 0.46),
                ],
                crop_st,
            );
            painter.line_segment(
                [
                    pos2(cx - u * 0.42, cy + u * 0.42),
                    pos2(cx + u * 0.42, cy - u * 0.42),
                ],
                stroke_of(accent, 1.1),
            );
        }
        Icon::Hand => {
            // Palm
            painter.rect_filled(
                Rect::from_center_size(
                    pos2(cx + u * 0.04, cy + u * 0.28),
                    Vec2::new(u * 0.92, u * 0.72),
                ),
                3.0,
                accent.gamma_multiply(0.22),
            );
            painter.rect_stroke(
                Rect::from_center_size(
                    pos2(cx + u * 0.04, cy + u * 0.28),
                    Vec2::new(u * 0.92, u * 0.72),
                ),
                3.0,
                st,
                egui::StrokeKind::Middle,
            );
            // 4 fingers
            for (fx, fy_top) in [
                (cx - u * 0.32, cy - u * 0.56),
                (cx - u * 0.08, cy - u * 0.78),
                (cx + u * 0.16, cy - u * 0.72),
                (cx + u * 0.40, cy - u * 0.46),
            ] {
                painter.line_segment(
                    [pos2(fx, fy_top), pos2(fx, cy - u * 0.04)],
                    stroke_of(color, (width * 1.25).max(1.5)),
                );
            }
            // Angled thumb
            painter.line_segment(
                [
                    pos2(cx - u * 0.42, cy + u * 0.26),
                    pos2(cx - u * 0.74, cy - u * 0.06),
                ],
                stroke_of(color, (width * 1.3).max(1.6)),
            );
        }
        Icon::Zoom | Icon::ZoomIn | Icon::ZoomOut => {
            let lens_c = pos2(cx - u * 0.16, cy - u * 0.16);
            let lens_r = u * 0.54;
            painter.circle_filled(lens_c, lens_r * 0.86, accent_blue.gamma_multiply(0.18));
            painter.circle_stroke(lens_c, lens_r, st);
            painter.line_segment(
                [
                    pos2(cx + u * 0.24, cy + u * 0.24),
                    pos2(cx + u * 0.80, cy + u * 0.80),
                ],
                stroke_of(accent, (width * 1.6).max(2.0)),
            );
            if icon == Icon::ZoomIn {
                let plus_st = stroke_of(accent_blue, width.max(1.2));
                painter.line_segment(
                    [
                        pos2(lens_c.x - u * 0.26, lens_c.y),
                        pos2(lens_c.x + u * 0.26, lens_c.y),
                    ],
                    plus_st,
                );
                painter.line_segment(
                    [
                        pos2(lens_c.x, lens_c.y - u * 0.26),
                        pos2(lens_c.x, lens_c.y + u * 0.26),
                    ],
                    plus_st,
                );
            } else if icon == Icon::ZoomOut {
                painter.line_segment(
                    [
                        pos2(lens_c.x - u * 0.26, lens_c.y),
                        pos2(lens_c.x + u * 0.26, lens_c.y),
                    ],
                    stroke_of(accent_blue, width.max(1.2)),
                );
            }
        }
        Icon::Brush => {
            // Angled handle
            painter.line_segment(
                [
                    pos2(cx - u * 0.08, cy + u * 0.08),
                    pos2(cx + u * 0.72, cy - u * 0.72),
                ],
                stroke_of(color, (width * 1.7).max(2.1)),
            );
            // Metallic ferrule
            painter.line_segment(
                [
                    pos2(cx - u * 0.28, cy + u * 0.28),
                    pos2(cx - u * 0.06, cy + u * 0.06),
                ],
                stroke_of(accent, (width * 2.1).max(2.5)),
            );
            // Tapered bristle tip
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.76, cy + u * 0.76),
                    pos2(cx - u * 0.42, cy + u * 0.16),
                    pos2(cx - u * 0.16, cy + u * 0.42),
                ],
                accent_blue,
                st,
            ));
        }
        Icon::Pencil => {
            // Angled pencil barrel
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.34, cy + u * 0.14),
                    pos2(cx + u * 0.50, cy - u * 0.70),
                    pos2(cx + u * 0.74, cy - u * 0.46),
                    pos2(cx - u * 0.10, cy + u * 0.38),
                ],
                accent.gamma_multiply(0.32),
                st,
            ));
            // Sharpened wood cone & graphite tip
            painter.add(egui::Shape::closed_line(
                vec![
                    pos2(cx - u * 0.74, cy + u * 0.74),
                    pos2(cx - u * 0.34, cy + u * 0.14),
                    pos2(cx - u * 0.10, cy + u * 0.38),
                ],
                st,
            ));
            dot(pos2(cx - u * 0.74, cy + u * 0.74));
        }
        Icon::Eraser => {
            // Upper sleeve (accent)
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.30, cy - u * 0.08),
                    pos2(cx + u * 0.16, cy - u * 0.68),
                    pos2(cx + u * 0.74, cy - u * 0.18),
                    pos2(cx + u * 0.28, cy + u * 0.42),
                ],
                accent.gamma_multiply(0.30),
                st,
            ));
            // Lower rubber tip
            painter.add(egui::Shape::closed_line(
                vec![
                    pos2(cx - u * 0.68, cy + u * 0.38),
                    pos2(cx - u * 0.30, cy - u * 0.08),
                    pos2(cx + u * 0.28, cy + u * 0.42),
                    pos2(cx - u * 0.10, cy + u * 0.80),
                ],
                st,
            ));
        }
        Icon::Stamp => {
            painter.rect_filled(
                Rect::from_center_size(pos2(cx, cy + u * 0.55), Vec2::new(u * 1.4, u * 0.35)),
                1.0,
                color,
            );
            painter.circle_stroke(pos2(cx, cy - u * 0.35), u * 0.35, st);
            painter.line_segment([pos2(cx, cy), pos2(cx, cy + u * 0.4)], st);
        }
        Icon::Line => {
            painter.line_segment(
                [
                    pos2(cx - u * 0.85, cy + u * 0.85),
                    pos2(cx + u * 0.85, cy - u * 0.85),
                ],
                st,
            );
        }
        Icon::Rectangle => {
            painter.rect_stroke(r, 0.0, st, egui::StrokeKind::Middle);
        }
        Icon::Rounded => {
            painter.rect_stroke(r, 3.5, st, egui::StrokeKind::Middle);
        }
        Icon::Ellipse => {
            painter.add(egui::Shape::ellipse_stroke(r.center(), r.size() * 0.48, st));
        }
        Icon::Polygon => {
            painter.add(egui::Shape::closed_line(
                vec![
                    pos2(cx, cy - u * 0.9),
                    pos2(cx + u * 0.85, cy - u * 0.2),
                    pos2(cx + u * 0.55, cy + u * 0.85),
                    pos2(cx - u * 0.55, cy + u * 0.85),
                    pos2(cx - u * 0.85, cy - u * 0.2),
                ],
                st,
            ));
        }
        Icon::Star => {
            let pts = polygon_vertices(Vec2::new(cx, cy), u * 0.9, 5, 0.45, 0.0)
                .into_iter()
                .map(|v| pos2(v.x, v.y))
                .collect();
            painter.add(egui::Shape::closed_line(pts, st));
        }
        Icon::Pen => {
            // Classic fountain pen nib
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.74, cy + u * 0.74),
                    pos2(cx - u * 0.22, cy - u * 0.56),
                    pos2(cx + u * 0.14, cy - u * 0.50),
                    pos2(cx + u * 0.50, cy - u * 0.14),
                    pos2(cx + u * 0.56, cy + u * 0.22),
                ],
                accent_blue.gamma_multiply(0.22),
                st,
            ));
            // Slit + breather hole
            painter.line_segment(
                [
                    pos2(cx - u * 0.72, cy + u * 0.72),
                    pos2(cx - u * 0.08, cy + u * 0.08),
                ],
                st,
            );
            painter.circle_filled(pos2(cx - u * 0.04, cy + u * 0.04), 2.0, accent);
            // Ferrule base
            painter.line_segment(
                [
                    pos2(cx + u * 0.14, cy - u * 0.50),
                    pos2(cx + u * 0.50, cy - u * 0.14),
                ],
                stroke_of(accent, 2.0),
            );
        }
        Icon::Knife => {
            // Craft knife handle (accent)
            painter.line_segment(
                [
                    pos2(cx + u * 0.04, cy - u * 0.04),
                    pos2(cx + u * 0.74, cy - u * 0.74),
                ],
                stroke_of(accent, (width * 2.1).max(2.6)),
            );
            // Sharp beveled blade
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.76, cy + u * 0.76),
                    pos2(cx - u * 0.18, cy - u * 0.08),
                    pos2(cx + u * 0.08, cy + u * 0.18),
                ],
                color.gamma_multiply(0.28),
                st,
            ));
        }
        Icon::Text => {
            let bold_st = stroke_of(color, (width * 1.4).max(1.8));
            // Top bar + serif ticks
            painter.line_segment(
                [
                    pos2(cx - u * 0.66, cy - u * 0.68),
                    pos2(cx + u * 0.66, cy - u * 0.68),
                ],
                bold_st,
            );
            painter.line_segment(
                [
                    pos2(cx - u * 0.66, cy - u * 0.68),
                    pos2(cx - u * 0.66, cy - u * 0.44),
                ],
                st,
            );
            painter.line_segment(
                [
                    pos2(cx + u * 0.66, cy - u * 0.68),
                    pos2(cx + u * 0.66, cy - u * 0.44),
                ],
                st,
            );
            // Stem + bottom serif
            painter.line_segment([pos2(cx, cy - u * 0.68), pos2(cx, cy + u * 0.72)], bold_st);
            painter.line_segment(
                [
                    pos2(cx - u * 0.34, cy + u * 0.72),
                    pos2(cx + u * 0.34, cy + u * 0.72),
                ],
                stroke_of(accent_blue, width.max(1.2)),
            );
        }
        Icon::Picker => {
            // Squeeze bulb (accent)
            painter.circle_filled(pos2(cx + u * 0.50, cy - u * 0.50), u * 0.32, accent);
            // Collar
            painter.line_segment(
                [
                    pos2(cx + u * 0.10, cy - u * 0.34),
                    pos2(cx + u * 0.34, cy - u * 0.10),
                ],
                stroke_of(color, (width * 1.8).max(2.2)),
            );
            // Glass pipette tube + droplet tip
            painter.add(egui::Shape::closed_line(
                vec![
                    pos2(cx - u * 0.76, cy + u * 0.76),
                    pos2(cx + u * 0.12, cy - u * 0.26),
                    pos2(cx + u * 0.26, cy - u * 0.12),
                ],
                st,
            ));
            painter.circle_filled(pos2(cx - u * 0.72, cy + u * 0.72), 1.8, accent_blue);
        }
        Icon::Fill => {
            // Tilted paint bucket
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.16, cy - u * 0.68),
                    pos2(cx + u * 0.62, cy + u * 0.10),
                    pos2(cx + u * 0.06, cy + u * 0.68),
                    pos2(cx - u * 0.72, cy - u * 0.10),
                ],
                color.gamma_multiply(0.22),
                st,
            ));
            // Pouring paint drop (accent_blue)
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.68, cy - u * 0.04),
                    pos2(cx - u * 0.82, cy + u * 0.48),
                    pos2(cx - u * 0.52, cy + u * 0.48),
                ],
                accent_blue,
                egui::Stroke::NONE,
            ));
            // Handle arc
            painter.line_segment(
                [
                    pos2(cx - u * 0.16, cy - u * 0.68),
                    pos2(cx + u * 0.24, cy - u * 0.78),
                ],
                stroke_of(accent, 1.4),
            );
        }
        Icon::Gradient => {
            for i in 0..6 {
                let t = i as f32 / 6.0;
                let c = color.gamma_multiply(0.2 + 0.8 * t);
                painter.rect_filled(
                    Rect::from_min_size(
                        pos2(cx - u + 2.0 * u * t, cy - u * 0.65),
                        Vec2::new(2.0 * u / 6.0 + 0.5, u * 1.3),
                    ),
                    0.0,
                    c,
                );
            }
            painter.rect_stroke(r.shrink(1.0), 0.0, st, egui::StrokeKind::Middle);
        }
        Icon::Blur => {
            // Teardrop water droplet
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx, cy - u * 0.82),
                    pos2(cx + u * 0.56, cy + u * 0.18),
                    pos2(cx, cy + u * 0.76),
                    pos2(cx - u * 0.56, cy + u * 0.18),
                ],
                accent_blue.gamma_multiply(0.28),
                st,
            ));
            painter.circle_filled(pos2(cx - u * 0.16, cy + u * 0.22), 1.6, accent_blue);
        }
        Icon::Sharpen => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx, cy - u * 0.82),
                    pos2(cx + u * 0.70, cy + u * 0.70),
                    pos2(cx - u * 0.70, cy + u * 0.70),
                ],
                accent_blue.gamma_multiply(0.22),
                st,
            ));
            painter.line_segment(
                [pos2(cx, cy - u * 0.82), pos2(cx, cy + u * 0.70)],
                stroke_of(accent, 1.1),
            );
        }
        Icon::Dodge => {
            painter.circle_filled(
                pos2(cx - u * 0.22, cy - u * 0.22),
                u * 0.44,
                accent_blue.gamma_multiply(0.22),
            );
            painter.circle_stroke(pos2(cx - u * 0.22, cy - u * 0.22), u * 0.44, st);
            painter.line_segment(
                [
                    pos2(cx + u * 0.12, cy + u * 0.12),
                    pos2(cx + u * 0.78, cy + u * 0.78),
                ],
                stroke_of(color, (width * 1.5).max(1.8)),
            );
        }
        Icon::Burn => {
            painter.circle_filled(pos2(cx, cy), u * 0.42, accent.gamma_multiply(0.75));
            painter.circle_stroke(pos2(cx, cy), u * 0.74, st);
        }
        Icon::Slice => {
            let slice_green = Color32::from_rgb(76, 188, 108);
            painter.rect_filled(r.shrink(1.0), 0.0, slice_green.gamma_multiply(0.24));
            painter.rect_stroke(
                r.shrink(1.0),
                0.0,
                stroke_of(slice_green, 1.3),
                egui::StrokeKind::Middle,
            );
            painter.line_segment(
                [pos2(r.left(), cy), pos2(r.right(), cy)],
                stroke_of(slice_green, 1.0),
            );
            painter.line_segment(
                [pos2(cx, r.top()), pos2(cx, r.bottom())],
                stroke_of(slice_green, 1.0),
            );
        }
        Icon::Hotspot => {
            let hs_blue = Color32::from_rgb(72, 168, 240);
            painter.rect_filled(r.shrink(2.0), 0.0, hs_blue.gamma_multiply(0.22));
            painter.circle_stroke(pos2(cx, cy), u * 0.68, stroke_of(hs_blue, 1.3));
            painter.circle_filled(pos2(cx, cy), u * 0.26, hs_blue);
        }
        Icon::Fit => {
            let b = r.shrink(1.0);
            let k = u * 0.38;
            for (corner, dx, dy) in [
                (b.left_top(), k, k),
                (b.right_top(), -k, k),
                (b.left_bottom(), k, -k),
                (b.right_bottom(), -k, -k),
            ] {
                painter.line_segment(
                    [corner, pos2(corner.x + dx, corner.y)],
                    stroke_of(accent, 1.4),
                );
                painter.line_segment(
                    [corner, pos2(corner.x, corner.y + dy)],
                    stroke_of(accent, 1.4),
                );
            }
            painter.rect_stroke(b.shrink(3.5), 0.0, st, egui::StrokeKind::Middle);
        }
        Icon::Group => {
            let b1 = Rect::from_min_size(
                pos2(cx - u * 0.75, cy - u * 0.75),
                Vec2::new(u * 0.85, u * 0.85),
            );
            let b2 = Rect::from_min_size(
                pos2(cx - u * 0.05, cy - u * 0.05),
                Vec2::new(u * 0.85, u * 0.85),
            );
            painter.rect_stroke(b1, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_filled(b2, 0.0, color.gamma_multiply(0.35));
            painter.rect_stroke(b2, 0.0, st, egui::StrokeKind::Middle);
            for pt in [
                r.left_top(),
                r.right_top(),
                r.left_bottom(),
                r.right_bottom(),
            ] {
                painter.rect_filled(Rect::from_center_size(pt, Vec2::splat(2.4)), 0.0, color);
            }
        }
        Icon::Ungroup => {
            let b1 = Rect::from_min_size(
                pos2(cx - u * 0.85, cy - u * 0.85),
                Vec2::new(u * 0.72, u * 0.72),
            );
            let b2 = Rect::from_min_size(
                pos2(cx + u * 0.12, cy + u * 0.12),
                Vec2::new(u * 0.72, u * 0.72),
            );
            painter.rect_stroke(b1, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_stroke(b2, 0.0, st, egui::StrokeKind::Middle);
            painter.line_segment(
                [
                    pos2(cx - u * 0.25, cy + u * 0.25),
                    pos2(cx + u * 0.25, cy - u * 0.25),
                ],
                st,
            );
        }
        Icon::BringFront => {
            let back = Rect::from_min_size(
                pos2(cx - u * 0.75, cy - u * 0.75),
                Vec2::new(u * 0.9, u * 0.9),
            );
            let front = Rect::from_min_size(
                pos2(cx - u * 0.1, cy - u * 0.1),
                Vec2::new(u * 0.9, u * 0.9),
            );
            painter.rect_stroke(back, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_filled(front, 0.0, accent_blue);
        }
        Icon::BringForward => {
            let back = Rect::from_min_size(
                pos2(cx - u * 0.75, cy - u * 0.75),
                Vec2::new(u * 0.85, u * 0.85),
            );
            let front = Rect::from_min_size(
                pos2(cx - u * 0.1, cy - u * 0.1),
                Vec2::new(u * 0.85, u * 0.85),
            );
            painter.rect_stroke(back, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_filled(front, 0.0, accent_blue.gamma_multiply(0.45));
            painter.rect_stroke(front, 0.0, st, egui::StrokeKind::Middle);
        }
        Icon::SendBackward => {
            let back = Rect::from_min_size(
                pos2(cx - u * 0.75, cy - u * 0.75),
                Vec2::new(u * 0.85, u * 0.85),
            );
            let front = Rect::from_min_size(
                pos2(cx - u * 0.1, cy - u * 0.1),
                Vec2::new(u * 0.85, u * 0.85),
            );
            painter.rect_filled(back, 0.0, accent.gamma_multiply(0.45));
            painter.rect_stroke(back, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_stroke(front, 0.0, st, egui::StrokeKind::Middle);
        }
        Icon::SendBack => {
            let back = Rect::from_min_size(
                pos2(cx - u * 0.75, cy - u * 0.75),
                Vec2::new(u * 0.9, u * 0.9),
            );
            let front = Rect::from_min_size(
                pos2(cx - u * 0.1, cy - u * 0.1),
                Vec2::new(u * 0.9, u * 0.9),
            );
            painter.rect_filled(back, 0.0, accent);
            painter.rect_stroke(front, 0.0, st, egui::StrokeKind::Middle);
        }
        Icon::RotateCCW => {
            painter.add(egui::Shape::line(
                arc_points(pos2(cx, cy + u * 0.1), u * 0.62, 0.0, 4.4, 12),
                st,
            ));
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.65, cy - u * 0.1),
                    pos2(cx - u * 0.25, cy - u * 0.35),
                    pos2(cx - u * 0.25, cy + u * 0.15),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::RotateCW => {
            painter.add(egui::Shape::line(
                arc_points(
                    pos2(cx, cy + u * 0.1),
                    u * 0.62,
                    -1.3,
                    std::f32::consts::PI,
                    12,
                ),
                st,
            ));
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx + u * 0.65, cy - u * 0.1),
                    pos2(cx + u * 0.25, cy - u * 0.35),
                    pos2(cx + u * 0.25, cy + u * 0.15),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::FlipH => {
            painter.line_segment([pos2(cx, cy - u * 0.85), pos2(cx, cy + u * 0.85)], st);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.2, cy - u * 0.55),
                    pos2(cx - u * 0.85, cy + u * 0.55),
                    pos2(cx - u * 0.2, cy + u * 0.55),
                ],
                color,
                Stroke::NONE,
            ));
            painter.add(egui::Shape::closed_line(
                vec![
                    pos2(cx + u * 0.2, cy - u * 0.55),
                    pos2(cx + u * 0.85, cy + u * 0.55),
                    pos2(cx + u * 0.2, cy + u * 0.55),
                ],
                st,
            ));
        }
        Icon::FlipV => {
            painter.line_segment([pos2(cx - u * 0.85, cy), pos2(cx + u * 0.85, cy)], st);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.55, cy - u * 0.2),
                    pos2(cx + u * 0.55, cy - u * 0.85),
                    pos2(cx + u * 0.55, cy - u * 0.2),
                ],
                color,
                Stroke::NONE,
            ));
            painter.add(egui::Shape::closed_line(
                vec![
                    pos2(cx - u * 0.55, cy + u * 0.2),
                    pos2(cx + u * 0.55, cy + u * 0.85),
                    pos2(cx + u * 0.55, cy + u * 0.2),
                ],
                st,
            ));
        }
        Icon::Eye => {
            painter.add(egui::Shape::ellipse_stroke(
                pos2(cx, cy),
                Vec2::new(u * 0.85, u * 0.48),
                st,
            ));
            painter.circle_filled(pos2(cx, cy), u * 0.24, color);
        }
        Icon::EyeOff => {
            painter.add(egui::Shape::ellipse_stroke(
                pos2(cx, cy),
                Vec2::new(u * 0.85, u * 0.45),
                Stroke::new(1.0, color.gamma_multiply(0.45)),
            ));
            painter.line_segment(
                [
                    pos2(cx - u * 0.7, cy + u * 0.55),
                    pos2(cx + u * 0.7, cy - u * 0.55),
                ],
                Stroke::new(1.0, color.gamma_multiply(0.55)),
            );
        }
        Icon::Lock => {
            let body = Rect::from_center_size(pos2(cx, cy + u * 0.25), Vec2::new(u * 1.1, u * 0.8));
            painter.rect_filled(body, 1.0, color);
            painter.add(egui::Shape::line(
                arc_points(
                    pos2(cx, cy - u * 0.15),
                    u * 0.35,
                    std::f32::consts::PI,
                    std::f32::consts::TAU,
                    8,
                ),
                st,
            ));
        }
        Icon::Unlock => {
            let body = Rect::from_center_size(pos2(cx, cy + u * 0.25), Vec2::new(u * 1.1, u * 0.8));
            painter.rect_stroke(
                body,
                1.0,
                Stroke::new(1.0, color.gamma_multiply(0.45)),
                egui::StrokeKind::Middle,
            );
        }
        Icon::Trash => {
            painter.line_segment(
                [
                    pos2(cx - u * 0.65, cy - u * 0.5),
                    pos2(cx + u * 0.65, cy - u * 0.5),
                ],
                st,
            );
            painter.rect_stroke(
                Rect::from_center_size(pos2(cx, cy + u * 0.15), Vec2::new(u * 0.95, u * 1.15)),
                0.0,
                st,
                egui::StrokeKind::Middle,
            );
            painter.line_segment([pos2(cx, cy - u * 0.25), pos2(cx, cy + u * 0.5)], st);
        }
        Icon::Folder => {
            painter.add(egui::Shape::closed_line(
                vec![
                    pos2(cx - u * 0.85, cy + u * 0.6),
                    pos2(cx - u * 0.85, cy - u * 0.45),
                    pos2(cx - u * 0.2, cy - u * 0.45),
                    pos2(cx - u * 0.05, cy - u * 0.2),
                    pos2(cx + u * 0.85, cy - u * 0.2),
                    pos2(cx + u * 0.85, cy + u * 0.6),
                ],
                st,
            ));
        }
        Icon::StateFirst | Icon::StateLast => {
            let back = matches!(icon, Icon::StateFirst);
            let bar_x = if back { cx - u * 0.75 } else { cx + u * 0.75 };
            painter.line_segment(
                [pos2(bar_x, cy - u * 0.7), pos2(bar_x, cy + u * 0.7)],
                Stroke::new(1.4, color),
            );
            let dir = if back { -1.0 } else { 1.0 };
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx + dir * u * 0.55, cy - u * 0.65),
                    pos2(cx + dir * u * 0.55, cy + u * 0.65),
                    pos2(cx - dir * u * 0.35, cy),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::StatePrev | Icon::StateNext => {
            let dir = if matches!(icon, Icon::StatePrev) {
                -1.0
            } else {
                1.0
            };
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx + dir * u * 0.65, cy - u * 0.75),
                    pos2(cx + dir * u * 0.65, cy + u * 0.75),
                    pos2(cx - dir * u * 0.45, cy),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::StatePlay => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(cx - u * 0.45, cy - u * 0.75),
                    pos2(cx - u * 0.45, cy + u * 0.75),
                    pos2(cx + u * 0.7, cy),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::StatePause => {
            let w = u * 0.28;
            for dx in [-u * 0.32, u * 0.32] {
                painter.rect_filled(
                    Rect::from_center_size(pos2(cx + dx, cy), Vec2::new(w, u * 1.5)),
                    0.0,
                    color,
                );
            }
        }
    }
}

/// Draws authentic Fireworks CS6 Align panel icons (`screenshots/Fireworks_Layout_Design.jpg`).
pub fn draw_align(painter: &Painter, rect: Rect, icon: AlignIcon, color: Color32) {
    let r = rect.shrink(2.5);
    let cx = r.center().x;
    let cy = r.center().y;
    let st = Stroke::new(1.15, color);
    let fill_dim = color.gamma_multiply(0.42);
    let draw_box = |b: Rect, filled: bool| {
        if filled {
            painter.rect_filled(b, 0.0, fill_dim);
        }
        painter.rect_stroke(b, 0.0, Stroke::new(1.0, color), egui::StrokeKind::Middle);
    };

    match icon {
        AlignIcon::RelativeToObject => {
            let box_r = r.shrink(2.0);
            painter.rect_stroke(box_r, 0.0, st, egui::StrokeKind::Middle);
            for pt in [
                box_r.left_top(),
                box_r.right_top(),
                box_r.left_bottom(),
                box_r.right_bottom(),
            ] {
                painter.rect_filled(Rect::from_center_size(pt, Vec2::splat(2.5)), 0.0, color);
            }
        }
        AlignIcon::RelativeToCanvas => {
            painter.rect_stroke(r, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_filled(r.shrink(2.5), 0.0, fill_dim);
            painter.line_segment([pos2(r.left(), cy), pos2(r.right(), cy)], st);
            painter.line_segment([pos2(cx, r.top()), pos2(cx, r.bottom())], st);
        }
        AlignIcon::AlignLeft => {
            let x = r.left() + 1.0;
            painter.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], st);
            draw_box(
                Rect::from_min_size(pos2(x + 2.0, cy - 5.0), Vec2::new(9.5, 4.0)),
                true,
            );
            draw_box(
                Rect::from_min_size(pos2(x + 2.0, cy + 1.0), Vec2::new(6.0, 4.0)),
                false,
            );
        }
        AlignIcon::AlignCenterH => {
            painter.line_segment([pos2(cx, r.top()), pos2(cx, r.bottom())], st);
            draw_box(
                Rect::from_center_size(pos2(cx, cy - 3.0), Vec2::new(11.0, 4.0)),
                true,
            );
            draw_box(
                Rect::from_center_size(pos2(cx, cy + 3.0), Vec2::new(6.5, 4.0)),
                false,
            );
        }
        AlignIcon::AlignRight => {
            let x = r.right() - 1.0;
            painter.line_segment([pos2(x, r.top()), pos2(x, r.bottom())], st);
            draw_box(
                Rect::from_min_size(pos2(x - 11.5, cy - 5.0), Vec2::new(9.5, 4.0)),
                true,
            );
            draw_box(
                Rect::from_min_size(pos2(x - 8.0, cy + 1.0), Vec2::new(6.0, 4.0)),
                false,
            );
        }
        AlignIcon::AlignTop => {
            let y = r.top() + 1.0;
            painter.line_segment([pos2(r.left(), y), pos2(r.right(), y)], st);
            draw_box(
                Rect::from_min_size(pos2(cx - 5.0, y + 2.0), Vec2::new(4.0, 9.5)),
                true,
            );
            draw_box(
                Rect::from_min_size(pos2(cx + 1.0, y + 2.0), Vec2::new(4.0, 6.0)),
                false,
            );
        }
        AlignIcon::AlignCenterV => {
            painter.line_segment([pos2(r.left(), cy), pos2(r.right(), cy)], st);
            draw_box(
                Rect::from_center_size(pos2(cx - 3.0, cy), Vec2::new(4.0, 11.0)),
                true,
            );
            draw_box(
                Rect::from_center_size(pos2(cx + 3.0, cy), Vec2::new(4.0, 6.5)),
                false,
            );
        }
        AlignIcon::AlignBottom => {
            let y = r.bottom() - 1.0;
            painter.line_segment([pos2(r.left(), y), pos2(r.right(), y)], st);
            draw_box(
                Rect::from_min_size(pos2(cx - 5.0, y - 11.5), Vec2::new(4.0, 9.5)),
                true,
            );
            draw_box(
                Rect::from_min_size(pos2(cx + 1.0, y - 8.0), Vec2::new(4.0, 6.0)),
                false,
            );
        }
        AlignIcon::DistributeTop => {
            let y1 = r.top() + 1.5;
            let y2 = cy + 0.5;
            painter.line_segment([pos2(r.left(), y1), pos2(r.right(), y1)], st);
            painter.line_segment([pos2(r.left(), y2), pos2(r.right(), y2)], st);
            draw_box(
                Rect::from_min_size(pos2(cx - 4.5, y1), Vec2::new(9.0, 3.8)),
                true,
            );
            draw_box(
                Rect::from_min_size(pos2(cx - 3.0, y2), Vec2::new(6.0, 4.2)),
                false,
            );
        }
        AlignIcon::DistributeCenterV => {
            let y1 = cy - 3.5;
            let y2 = cy + 3.5;
            painter.line_segment([pos2(r.left(), y1), pos2(r.right(), y1)], st);
            painter.line_segment([pos2(r.left(), y2), pos2(r.right(), y2)], st);
            draw_box(
                Rect::from_center_size(pos2(cx, y1), Vec2::new(9.5, 4.0)),
                true,
            );
            draw_box(
                Rect::from_center_size(pos2(cx, y2), Vec2::new(6.5, 4.0)),
                false,
            );
        }
        AlignIcon::DistributeBottom => {
            let y1 = cy - 0.5;
            let y2 = r.bottom() - 1.5;
            painter.line_segment([pos2(r.left(), y1), pos2(r.right(), y1)], st);
            painter.line_segment([pos2(r.left(), y2), pos2(r.right(), y2)], st);
            draw_box(
                Rect::from_min_size(pos2(cx - 4.5, y1 - 4.0), Vec2::new(9.0, 4.0)),
                true,
            );
            draw_box(
                Rect::from_min_size(pos2(cx - 3.0, y2 - 4.0), Vec2::new(6.0, 4.0)),
                false,
            );
        }
        AlignIcon::DistributeLeft => {
            let x1 = r.left() + 1.5;
            let x2 = cx + 0.5;
            painter.line_segment([pos2(x1, r.top()), pos2(x1, r.bottom())], st);
            painter.line_segment([pos2(x2, r.top()), pos2(x2, r.bottom())], st);
            draw_box(
                Rect::from_min_size(pos2(x1, cy - 4.5), Vec2::new(4.0, 9.0)),
                true,
            );
            draw_box(
                Rect::from_min_size(pos2(x2, cy - 3.0), Vec2::new(4.0, 6.0)),
                false,
            );
        }
        AlignIcon::DistributeCenterH => {
            let x1 = cy_offset(cx, -3.5);
            let x2 = cy_offset(cx, 3.5);
            painter.line_segment([pos2(x1, r.top()), pos2(x1, r.bottom())], st);
            painter.line_segment([pos2(x2, r.top()), pos2(x2, r.bottom())], st);
            draw_box(
                Rect::from_center_size(pos2(x1, cy), Vec2::new(4.0, 9.5)),
                true,
            );
            draw_box(
                Rect::from_center_size(pos2(x2, cy), Vec2::new(4.0, 6.5)),
                false,
            );
        }
        AlignIcon::DistributeRight => {
            let x1 = cx - 0.5;
            let x2 = r.right() - 1.5;
            painter.line_segment([pos2(x1, r.top()), pos2(x1, r.bottom())], st);
            painter.line_segment([pos2(x2, r.top()), pos2(x2, r.bottom())], st);
            draw_box(
                Rect::from_min_size(pos2(x1 - 4.0, cy - 4.5), Vec2::new(4.0, 9.0)),
                true,
            );
            draw_box(
                Rect::from_min_size(pos2(x2 - 4.0, cy - 3.0), Vec2::new(4.0, 6.0)),
                false,
            );
        }
        AlignIcon::MatchWidth => {
            draw_box(
                Rect::from_center_size(pos2(cx, cy - 3.0), Vec2::new(11.0, 4.0)),
                true,
            );
            draw_box(
                Rect::from_center_size(pos2(cx, cy + 3.0), Vec2::new(11.0, 4.0)),
                false,
            );
            painter.line_segment(
                [pos2(cx - 5.5, r.top() + 0.5), pos2(cx + 5.5, r.top() + 0.5)],
                st,
            );
        }
        AlignIcon::MatchHeight => {
            draw_box(
                Rect::from_center_size(pos2(cx - 3.0, cy), Vec2::new(4.0, 11.0)),
                true,
            );
            draw_box(
                Rect::from_center_size(pos2(cx + 3.0, cy), Vec2::new(4.0, 11.0)),
                false,
            );
            painter.line_segment(
                [
                    pos2(r.left() + 0.5, cy - 5.5),
                    pos2(r.left() + 0.5, cy + 5.5),
                ],
                st,
            );
        }
        AlignIcon::MatchBoth => {
            draw_box(
                Rect::from_min_size(pos2(cx - 5.5, cy - 5.5), Vec2::new(7.0, 7.0)),
                true,
            );
            draw_box(
                Rect::from_min_size(pos2(cx - 1.5, cy - 1.5), Vec2::new(7.0, 7.0)),
                false,
            );
        }
        AlignIcon::SpaceVertical => {
            draw_box(
                Rect::from_center_size(pos2(cx, r.top() + 2.5), Vec2::new(10.0, 3.5)),
                true,
            );
            draw_box(
                Rect::from_center_size(pos2(cx, r.bottom() - 2.5), Vec2::new(10.0, 3.5)),
                false,
            );
            painter.line_segment([pos2(cx, cy - 2.0), pos2(cx, cy + 2.0)], st);
        }
        AlignIcon::SpaceHorizontal => {
            draw_box(
                Rect::from_center_size(pos2(r.left() + 2.5, cy), Vec2::new(3.5, 10.0)),
                true,
            );
            draw_box(
                Rect::from_center_size(pos2(r.right() - 2.5, cy), Vec2::new(3.5, 10.0)),
                false,
            );
            painter.line_segment([pos2(cx - 2.0, cy), pos2(cx + 2.0, cy)], st);
        }
    }
}

#[inline]
fn cy_offset(base: f32, d: f32) -> f32 {
    base + d
}

/// Draws authentic Fireworks CS6 Path panel icons (`screenshots/Fireworks_Layout_Design.jpg`).
pub fn draw_path(painter: &Painter, rect: Rect, icon: PathIcon, color: Color32) {
    let r = rect.shrink(2.5);
    let cx = r.center().x;
    let cy = r.center().y;
    let st = Stroke::new(1.1, color);
    let fill_dim = color.gamma_multiply(0.42);
    let fill_bright = color.gamma_multiply(0.75);
    let b1 = Rect::from_min_size(pos2(cx - 6.0, cy - 5.5), Vec2::new(7.5, 7.5));
    let b2 = Rect::from_min_size(pos2(cx - 1.5, cy - 1.5), Vec2::new(7.5, 7.5));
    let node = |p: Pos2| {
        painter.rect_filled(Rect::from_center_size(p, Vec2::splat(2.6)), 0.0, color);
    };

    match icon {
        // ---- Combine Paths (6) ----------------------------------------------
        PathIcon::Join => {
            painter.rect_stroke(b1, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_stroke(b2, 0.0, st, egui::StrokeKind::Middle);
            node(b1.left_top());
            node(b2.right_bottom());
        }
        PathIcon::Split => {
            let s1 = Rect::from_min_size(pos2(cx - 6.5, cy - 6.0), Vec2::new(5.5, 5.5));
            let s2 = Rect::from_min_size(pos2(cx + 1.0, cy + 0.5), Vec2::new(5.5, 5.5));
            painter.rect_stroke(s1, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_stroke(s2, 0.0, st, egui::StrokeKind::Middle);
        }
        PathIcon::Union => {
            painter.rect_filled(b1, 0.0, fill_dim);
            painter.rect_filled(b2, 0.0, fill_dim);
            painter.add(egui::Shape::closed_line(
                vec![
                    b1.left_top(),
                    pos2(b1.right(), b1.top()),
                    pos2(b1.right(), b2.top()),
                    b2.right_top(),
                    b2.right_bottom(),
                    pos2(b2.left(), b2.bottom()),
                    pos2(b2.left(), b1.bottom()),
                    b1.left_bottom(),
                ],
                st,
            ));
        }
        PathIcon::Intersect => {
            painter.rect_stroke(b1, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_stroke(b2, 0.0, st, egui::StrokeKind::Middle);
            let inter = b1.intersect(b2);
            painter.rect_filled(inter, 0.0, fill_bright);
        }
        PathIcon::Punch => {
            painter.rect_filled(b1, 0.0, fill_bright);
            painter.rect_stroke(b1, 0.0, st, egui::StrokeKind::Middle);
            painter.rect_filled(b2, 0.0, Color32::from_black_alpha(160));
            painter.rect_stroke(b2, 0.0, st, egui::StrokeKind::Middle);
        }
        PathIcon::Crop => {
            let inter = b1.intersect(b2);
            painter.rect_filled(inter, 0.0, fill_bright);
            painter.rect_stroke(inter.expand(1.5), 0.0, st, egui::StrokeKind::Middle);
        }
        // ---- Alter Paths Row 1 (7) ------------------------------------------
        PathIcon::Simplify => {
            painter.add(egui::Shape::line(
                vec![
                    pos2(r.left(), cy + 3.0),
                    pos2(cx - 2.5, cy - 4.0),
                    pos2(cx + 2.5, cy + 4.0),
                    pos2(r.right(), cy - 3.0),
                ],
                st,
            ));
            node(pos2(cx, cy));
        }
        PathIcon::ExpandStroke => {
            painter.rect_filled(
                Rect::from_center_size(pos2(cx, cy), Vec2::new(11.0, 5.0)),
                0.0,
                fill_dim,
            );
            painter.rect_stroke(
                Rect::from_center_size(pos2(cx, cy), Vec2::new(11.0, 5.0)),
                0.0,
                st,
                egui::StrokeKind::Middle,
            );
            painter.line_segment([pos2(cx - 4.5, cy), pos2(cx + 4.5, cy)], st);
        }
        PathIcon::InsetExpand => {
            painter.rect_stroke(r.shrink(0.5), 0.0, st, egui::StrokeKind::Middle);
            painter.rect_stroke(r.shrink(3.5), 0.0, st, egui::StrokeKind::Middle);
        }
        PathIcon::HardEdge => {
            painter.add(egui::Shape::ellipse_filled(
                pos2(cx, cy),
                Vec2::splat(5.0),
                fill_bright,
            ));
            painter.circle_stroke(pos2(cx, cy), 5.0, st);
        }
        PathIcon::AntiAlias => {
            painter.circle_filled(pos2(cx, cy), 5.5, fill_dim);
            painter.circle_filled(pos2(cx, cy), 3.5, fill_bright);
        }
        PathIcon::FeatherEdge => {
            painter.circle_stroke(pos2(cx, cy), 5.5, Stroke::new(0.8, fill_dim));
            painter.circle_filled(pos2(cx, cy), 3.0, fill_bright);
        }
        PathIcon::StrokeToFill => {
            painter.rect_stroke(r.shrink(1.5), 1.5, st, egui::StrokeKind::Middle);
            painter.rect_filled(r.shrink(4.0), 0.0, fill_bright);
        }
        // ---- Alter Paths Row 2 (6) ------------------------------------------
        PathIcon::ReverseDirection => {
            painter.line_segment([pos2(r.left() + 1.0, cy), pos2(r.right() - 1.0, cy)], st);
            painter.line_segment(
                [pos2(r.left() + 1.0, cy), pos2(r.left() + 4.5, cy - 3.5)],
                st,
            );
            painter.line_segment(
                [pos2(r.right() - 1.0, cy), pos2(r.right() - 4.5, cy + 3.5)],
                st,
            );
        }
        PathIcon::ClosePath => {
            painter.add(egui::Shape::closed_line(
                vec![
                    pos2(cx - 5.0, cy + 4.0),
                    pos2(cx, cy - 5.0),
                    pos2(cx + 5.0, cy + 4.0),
                ],
                st,
            ));
            node(pos2(cx, cy - 5.0));
        }
        PathIcon::OpenPath => {
            painter.add(egui::Shape::line(
                vec![
                    pos2(cx - 5.0, cy + 4.0),
                    pos2(cx, cy - 5.0),
                    pos2(cx + 5.0, cy + 4.0),
                ],
                st,
            ));
            node(pos2(cx - 5.0, cy + 4.0));
            node(pos2(cx + 5.0, cy + 4.0));
        }
        PathIcon::PathToMarquee => {
            for i in 0..4 {
                let x = r.left() + i as f32 * 3.2;
                painter.line_segment([pos2(x, r.top() + 1.0), pos2(x + 1.8, r.top() + 1.0)], st);
                painter.line_segment(
                    [pos2(x, r.bottom() - 1.0), pos2(x + 1.8, r.bottom() - 1.0)],
                    st,
                );
            }
            painter.line_segment(
                [
                    pos2(r.left() + 1.0, r.top() + 2.0),
                    pos2(r.left() + 1.0, r.bottom() - 2.0),
                ],
                st,
            );
            painter.line_segment(
                [
                    pos2(r.right() - 1.0, r.top() + 2.0),
                    pos2(r.right() - 1.0, r.bottom() - 2.0),
                ],
                st,
            );
        }
        PathIcon::SnapToPixel => {
            painter.line_segment([pos2(r.left(), cy), pos2(r.right(), cy)], st);
            painter.line_segment([pos2(cx, r.top()), pos2(cx, r.bottom())], st);
            node(pos2(cx, cy));
        }
        PathIcon::FlattenPath => {
            painter.line_segment(
                [
                    pos2(r.left() + 1.0, cy + 3.0),
                    pos2(r.right() - 1.0, cy + 3.0),
                ],
                st,
            );
            painter.rect_filled(
                Rect::from_center_size(pos2(cx, cy - 1.5), Vec2::new(8.0, 5.0)),
                0.0,
                fill_bright,
            );
        }
        // ---- Edit Points Row 1 (7) ------------------------------------------
        PathIcon::AddPoints => {
            painter.line_segment([pos2(r.left(), cy + 2.0), pos2(r.right(), cy + 2.0)], st);
            node(pos2(cx, cy + 2.0));
            painter.line_segment([pos2(cx, cy - 5.5), pos2(cx, cy - 1.5)], st);
            painter.line_segment([pos2(cx - 2.0, cy - 3.5), pos2(cx + 2.0, cy - 3.5)], st);
        }
        PathIcon::DeletePoints => {
            painter.line_segment([pos2(r.left(), cy + 2.0), pos2(r.right(), cy + 2.0)], st);
            node(pos2(cx, cy + 2.0));
            painter.line_segment([pos2(cx - 2.5, cy - 3.5), pos2(cx + 2.5, cy - 3.5)], st);
        }
        PathIcon::CornerPoint => {
            painter.add(egui::Shape::line(
                vec![
                    pos2(r.left() + 1.0, cy + 4.5),
                    pos2(cx, cy - 4.5),
                    pos2(r.right() - 1.0, cy + 4.5),
                ],
                st,
            ));
            node(pos2(cx, cy - 4.5));
        }
        PathIcon::SmoothPoint => {
            let pts = arc_points(
                pos2(cx, cy + 4.0),
                5.5,
                std::f32::consts::PI,
                std::f32::consts::TAU,
                10,
            );
            painter.add(egui::Shape::line(pts, st));
            painter.line_segment(
                [pos2(cx - 5.5, cy - 1.5), pos2(cx + 5.5, cy - 1.5)],
                Stroke::new(0.9, fill_bright),
            );
            node(pos2(cx, cy - 1.5));
        }
        PathIcon::RoundPoints => {
            painter.add(egui::Shape::line(
                vec![
                    pos2(cx - 5.0, cy + 4.5),
                    pos2(cx - 2.5, cy - 2.5),
                    pos2(cx + 2.5, cy - 2.5),
                    pos2(cx + 5.0, cy + 4.5),
                ],
                st,
            ));
            node(pos2(cx - 2.5, cy - 2.5));
            node(pos2(cx + 2.5, cy - 2.5));
        }
        PathIcon::FilletPoints => {
            painter.line_segment([pos2(cx - 4.5, cy + 4.5), pos2(cx - 4.5, cy - 1.0)], st);
            painter.line_segment([pos2(cx - 1.0, cy - 4.5), pos2(cx + 4.5, cy - 4.5)], st);
            painter.add(egui::Shape::line(
                arc_points(
                    pos2(cx - 1.0, cy - 1.0),
                    3.5,
                    std::f32::consts::PI,
                    std::f32::consts::FRAC_PI_2 * 3.0,
                    6,
                ),
                st,
            ));
        }
        PathIcon::ChamferPoints => {
            painter.add(egui::Shape::line(
                vec![
                    pos2(cx - 4.5, cy + 4.5),
                    pos2(cx - 4.5, cy - 1.0),
                    pos2(cx - 1.0, cy - 4.5),
                    pos2(cx + 4.5, cy - 4.5),
                ],
                st,
            ));
            node(pos2(cx - 4.5, cy - 1.0));
            node(pos2(cx - 1.0, cy - 4.5));
        }
        // ---- Edit Points Row 2 (7) ------------------------------------------
        PathIcon::BreakPath => {
            painter.line_segment([pos2(r.left(), cy + 2.0), pos2(cx - 2.0, cy - 2.0)], st);
            painter.line_segment([pos2(cx + 2.0, cy - 2.0), pos2(r.right(), cy + 2.0)], st);
            node(pos2(cx - 2.0, cy - 2.0));
            node(pos2(cx + 2.0, cy - 2.0));
        }
        PathIcon::JoinPoints => {
            painter.line_segment([pos2(r.left(), cy), pos2(r.right(), cy)], st);
            node(pos2(cx - 3.0, cy));
            node(pos2(cx + 3.0, cy));
        }
        PathIcon::AlignPointsH => {
            painter.line_segment([pos2(r.left(), cy), pos2(r.right(), cy)], st);
            node(pos2(cx - 4.0, cy));
            node(pos2(cx, cy));
            node(pos2(cx + 4.0, cy));
        }
        PathIcon::AlignPointsV => {
            painter.line_segment([pos2(cx, r.top()), pos2(cx, r.bottom())], st);
            node(pos2(cx, cy - 4.0));
            node(pos2(cx, cy));
            node(pos2(cx, cy + 4.0));
        }
        PathIcon::KnifeSplit => {
            painter.line_segment(
                [
                    pos2(r.left() + 1.0, r.bottom() - 1.0),
                    pos2(r.right() - 1.0, r.top() + 1.0),
                ],
                st,
            );
            node(pos2(cx, cy));
        }
        PathIcon::ExtendPath => {
            painter.line_segment([pos2(r.left() + 1.0, cy + 3.0), pos2(cx, cy)], st);
            painter.line_segment(
                [pos2(cx, cy), pos2(r.right() - 1.0, cy - 3.0)],
                Stroke::new(1.0, fill_dim),
            );
            node(pos2(cx, cy));
            node(pos2(r.right() - 1.0, cy - 3.0));
        }
        PathIcon::SharpApex => {
            painter.add(egui::Shape::line(
                vec![
                    pos2(cx - 4.0, cy + 4.5),
                    pos2(cx, cy - 5.0),
                    pos2(cx + 4.0, cy + 4.5),
                ],
                st,
            ));
        }
        // ---- Select Points Row 1 (5) ----------------------------------------
        PathIcon::InversePoints => {
            painter.circle_stroke(pos2(cx, cy), 5.0, st);
            node(pos2(cx - 5.0, cy));
            painter.rect_stroke(
                Rect::from_center_size(pos2(cx + 5.0, cy), Vec2::splat(2.8)),
                0.0,
                st,
                egui::StrokeKind::Middle,
            );
        }
        PathIcon::SelectAllPoints => {
            painter.circle_stroke(pos2(cx, cy), 4.8, st);
            for pt in [
                pos2(cx, cy - 4.8),
                pos2(cx + 4.8, cy),
                pos2(cx, cy + 4.8),
                pos2(cx - 4.8, cy),
            ] {
                node(pt);
            }
        }
        PathIcon::FirstPoint => {
            painter.circle_stroke(pos2(cx, cy), 4.8, st);
            painter.rect_filled(
                Rect::from_center_size(pos2(cx, cy - 4.8), Vec2::splat(3.6)),
                0.0,
                color,
            );
        }
        PathIcon::PrevPoint => {
            painter.circle_stroke(pos2(cx, cy), 4.8, st);
            node(pos2(cx - 4.8, cy));
            painter.line_segment([pos2(cx, cy - 2.0), pos2(cx - 2.5, cy)], st);
        }
        PathIcon::NextPoint => {
            painter.circle_stroke(pos2(cx, cy), 4.8, st);
            node(pos2(cx + 4.8, cy));
            painter.line_segment([pos2(cx, cy - 2.0), pos2(cx + 2.5, cy)], st);
        }
    }
}

fn polygon_vertices(
    center: Vec2,
    radius: f32,
    sides: u32,
    star_inner: f32,
    angle_rad: f32,
) -> Vec<Vec2> {
    let n = sides.max(3) as usize;
    let is_star = (star_inner - 1.0).abs() > 0.01;
    let total = if is_star { n * 2 } else { n };
    let mut pts = Vec::with_capacity(total);
    for i in 0..total {
        let frac = i as f32 / total as f32;
        let theta = angle_rad - std::f32::consts::FRAC_PI_2 + frac * std::f32::consts::TAU;
        let r = if is_star && i % 2 == 1 {
            radius * star_inner.clamp(0.1, 1.0)
        } else {
            radius
        };
        pts.push(Vec2::new(
            center.x + r * theta.cos(),
            center.y + r * theta.sin(),
        ));
    }
    pts
}

#[cfg(test)]
mod tests {
    use super::{AlignIcon, Icon, PathIcon, draw, draw_align, draw_path};
    use egui::{Color32, Context, Id, LayerId, Order, Rect, Vec2, pos2};

    #[test]
    fn every_painter_icon_variant_emits_shapes() {
        let context = Context::default();
        let icons = [
            Icon::New,
            Icon::Open,
            Icon::Save,
            Icon::Import,
            Icon::Export,
            Icon::Cut,
            Icon::Copy,
            Icon::Paste,
            Icon::Duplicate,
            Icon::Undo,
            Icon::Redo,
            Icon::Select,
            Icon::Subselect,
            Icon::Scale,
            Icon::Marquee,
            Icon::Lasso,
            Icon::Wand,
            Icon::Crop,
            Icon::Hand,
            Icon::Zoom,
            Icon::Brush,
            Icon::Pencil,
            Icon::Eraser,
            Icon::Stamp,
            Icon::Line,
            Icon::Rectangle,
            Icon::Rounded,
            Icon::Ellipse,
            Icon::Polygon,
            Icon::Star,
            Icon::Pen,
            Icon::Knife,
            Icon::Text,
            Icon::Picker,
            Icon::Fill,
            Icon::Gradient,
            Icon::Blur,
            Icon::Sharpen,
            Icon::Dodge,
            Icon::Burn,
            Icon::Slice,
            Icon::Hotspot,
            Icon::ZoomIn,
            Icon::ZoomOut,
            Icon::Fit,
            Icon::Group,
            Icon::Ungroup,
            Icon::BringFront,
            Icon::BringForward,
            Icon::SendBackward,
            Icon::SendBack,
            Icon::RotateCCW,
            Icon::RotateCW,
            Icon::FlipH,
            Icon::FlipV,
            Icon::Eye,
            Icon::EyeOff,
            Icon::Lock,
            Icon::Unlock,
            Icon::Trash,
            Icon::Folder,
            Icon::StateFirst,
            Icon::StatePrev,
            Icon::StatePlay,
            Icon::StatePause,
            Icon::StateNext,
            Icon::StateLast,
        ];
        let align_icons = [
            AlignIcon::RelativeToObject,
            AlignIcon::RelativeToCanvas,
            AlignIcon::AlignLeft,
            AlignIcon::AlignCenterH,
            AlignIcon::AlignRight,
            AlignIcon::AlignTop,
            AlignIcon::AlignCenterV,
            AlignIcon::AlignBottom,
            AlignIcon::DistributeTop,
            AlignIcon::DistributeCenterV,
            AlignIcon::DistributeBottom,
            AlignIcon::DistributeLeft,
            AlignIcon::DistributeCenterH,
            AlignIcon::DistributeRight,
            AlignIcon::MatchWidth,
            AlignIcon::MatchHeight,
            AlignIcon::MatchBoth,
            AlignIcon::SpaceVertical,
            AlignIcon::SpaceHorizontal,
        ];
        let path_icons = [
            PathIcon::Join,
            PathIcon::Split,
            PathIcon::Union,
            PathIcon::Intersect,
            PathIcon::Punch,
            PathIcon::Crop,
            PathIcon::Simplify,
            PathIcon::ExpandStroke,
            PathIcon::InsetExpand,
            PathIcon::HardEdge,
            PathIcon::AntiAlias,
            PathIcon::FeatherEdge,
            PathIcon::StrokeToFill,
            PathIcon::ReverseDirection,
            PathIcon::ClosePath,
            PathIcon::OpenPath,
            PathIcon::PathToMarquee,
            PathIcon::SnapToPixel,
            PathIcon::FlattenPath,
            PathIcon::AddPoints,
            PathIcon::DeletePoints,
            PathIcon::CornerPoint,
            PathIcon::SmoothPoint,
            PathIcon::RoundPoints,
            PathIcon::FilletPoints,
            PathIcon::ChamferPoints,
            PathIcon::BreakPath,
            PathIcon::JoinPoints,
            PathIcon::AlignPointsH,
            PathIcon::AlignPointsV,
            PathIcon::KnifeSplit,
            PathIcon::ExtendPath,
            PathIcon::SharpApex,
            PathIcon::InversePoints,
            PathIcon::SelectAllPoints,
            PathIcon::FirstPoint,
            PathIcon::PrevPoint,
            PathIcon::NextPoint,
        ];

        let output = context.run_ui(Default::default(), |ui| {
            let painter = ui
                .painter()
                .clone()
                .with_layer_id(LayerId::new(Order::Foreground, Id::new("icons")));
            for (index, icon) in icons.into_iter().enumerate() {
                let origin = pos2((index % 10) as f32 * 32.0, (index / 10) as f32 * 32.0);
                draw(
                    &painter,
                    Rect::from_min_size(origin, Vec2::splat(24.0)),
                    icon,
                    Color32::WHITE,
                    1.5,
                );
            }
            for (index, icon) in align_icons.into_iter().enumerate() {
                let origin = pos2(index as f32 * 32.0, 320.0);
                draw_align(
                    &painter,
                    Rect::from_min_size(origin, Vec2::splat(24.0)),
                    icon,
                    Color32::WHITE,
                );
            }
            for (index, icon) in path_icons.into_iter().enumerate() {
                let origin = pos2(index as f32 * 32.0, 352.0);
                draw_path(
                    &painter,
                    Rect::from_min_size(origin, Vec2::splat(24.0)),
                    icon,
                    Color32::WHITE,
                );
            }
        });

        assert!(!output.shapes.is_empty());
    }
}
