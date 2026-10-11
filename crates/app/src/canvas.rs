//! Canvas view state: pan/zoom between document world space and egui
//! screen points. Document coordinates are canvas pixels (identity
//! transform, see `compose::render`); the view adds nothing to the
//! document itself.

/// Pan offset and zoom scale, with the world↔screen conversions the
/// tools and the painter both need. Zoom is a factor (1.0 = 100%).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// World coordinate at the canvas widget's top-left origin.
    pub pan: geom::Point,
    /// Screen points per world unit.
    pub zoom: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            pan: geom::Point::ZERO,
            zoom: 1.0,
        }
    }
}

impl View {
    /// World position under a screen point.
    #[must_use]
    pub fn to_world(&self, screen: egui::Pos2) -> geom::Point {
        geom::Point::new(
            f64::from(screen.x) / f64::from(self.zoom) + self.pan.x,
            f64::from(screen.y) / f64::from(self.zoom) + self.pan.y,
        )
    }

    /// Screen point for a world position.
    #[must_use]
    pub fn to_screen(&self, world: geom::Point) -> egui::Pos2 {
        egui::pos2(
            ((world.x - self.pan.x) as f32) * self.zoom,
            ((world.y - self.pan.y) as f32) * self.zoom,
        )
    }

    /// Screen vector for a world span.
    #[must_use]
    pub fn vec_to_screen(&self, vector: geom::Vec2) -> egui::Vec2 {
        egui::vec2((vector.x as f32) * self.zoom, (vector.y as f32) * self.zoom)
    }

    /// Pans by a screen-space drag delta.
    pub fn pan_by(&mut self, delta: egui::Vec2) {
        self.pan.x -= f64::from(delta.x) / f64::from(self.zoom);
        self.pan.y -= f64::from(delta.y) / f64::from(self.zoom);
    }

    /// Zooms about a screen anchor so the world point under it stays
    /// put. Clamped to a sane range.
    pub fn zoom_about(&mut self, anchor: egui::Pos2, factor: f32) {
        let before = self.to_world(anchor);
        self.zoom = (self.zoom * factor).clamp(0.05, 64.0);
        // Solve pan so that to_world(anchor) == `before` again.
        self.pan.x = before.x - f64::from(anchor.x) / f64::from(self.zoom);
        self.pan.y = before.y - f64::from(anchor.y) / f64::from(self.zoom);
    }

    /// Fits a world-space rect into a screen-size viewport with a
    /// small margin, centering it.
    pub fn fit(&mut self, world: geom::Rect, viewport: egui::Vec2) {
        if world.width() <= 0.0 || world.height() <= 0.0 || viewport.x <= 0.0 || viewport.y <= 0.0 {
            return;
        }
        let margin = 24.0;
        let zoom = f32::min(
            (viewport.x - margin) / world.width() as f32,
            (viewport.y - margin) / world.height() as f32,
        );
        self.zoom = zoom.clamp(0.05, 64.0);
        self.pan.x = world.x0
            - f64::from(viewport.x - world.width() as f32 * self.zoom) / 2.0 / f64::from(self.zoom);
        self.pan.y = world.y0
            - f64::from(viewport.y - world.height() as f32 * self.zoom)
                / 2.0
                / f64::from(self.zoom);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn world_and_screen_round_trip() {
        let view = View {
            pan: geom::Point::new(30.0, -12.0),
            zoom: 2.0,
        };
        let world = geom::Point::new(100.0, 250.0);
        let round = view.to_world(view.to_screen(world));
        assert!((round.x - world.x).abs() < 1e-9);
        assert!((round.y - world.y).abs() < 1e-9);
    }

    #[test]
    fn zoom_about_keeps_the_anchor_still() {
        let mut view = View::default();
        let anchor = egui::pos2(150.0, 90.0);
        let before = view.to_world(anchor);
        view.zoom_about(anchor, 1.5);
        let after = view.to_world(anchor);
        assert!((before.x - after.x).abs() < 1e-9);
        assert!((before.y - after.y).abs() < 1e-9);
        assert!((view.zoom - 1.5).abs() < 1e-9);
    }

    #[test]
    fn fit_centers_the_document() {
        let mut view = View::default();
        let world = geom::Rect::new(0.0, 0.0, 400.0, 300.0);
        view.fit(world, egui::vec2(800.0, 600.0));
        let center_screen = view.to_screen(geom::Point::new(200.0, 150.0));
        assert!((center_screen.x - 400.0).abs() < 1.0);
        assert!((center_screen.y - 300.0).abs() < 1.0);
    }
}
