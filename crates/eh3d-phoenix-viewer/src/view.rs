// SPDX-License-Identifier: GPL-3.0-or-later
//! The view: where the drawing is on the screen, the zoom and the conversions between the
//! millimetres of the drawing and the pixels of the window. The world has y up, the screen has y
//! down.

use crate::prim::{Bounds, P};

/// The limits of the zoom, in pixels per millimetre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomLimits {
    /// The smallest zoom.
    pub min: f64,
    /// The largest zoom.
    pub max: f64,
}

/// What part of the drawing is shown in a window of a given size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// The point of the drawing in the middle of the window.
    pub centre: P,
    /// Pixels per millimetre.
    pub scale: f64,
    /// The width of the window in pixels.
    pub width_px: f64,
    /// The height of the window in pixels.
    pub height_px: f64,
}

impl Default for View {
    fn default() -> Self {
        View {
            centre: P::new(0.0, 0.0),
            scale: 4.0,
            width_px: 800.0,
            height_px: 600.0,
        }
    }
}

impl View {
    /// Tells the view how large the window is now.
    pub fn resize(&mut self, width_px: f64, height_px: f64) {
        if width_px.is_finite() && height_px.is_finite() && width_px > 0.0 && height_px > 0.0 {
            self.width_px = width_px;
            self.height_px = height_px;
        }
    }

    /// The pixel of a point of the drawing, from the top left corner of the window.
    pub fn to_screen(&self, p: P) -> (f64, f64) {
        (
            self.width_px / 2.0 + (p.x - self.centre.x) * self.scale,
            self.height_px / 2.0 - (p.y - self.centre.y) * self.scale,
        )
    }

    /// The point of the drawing under a pixel.
    pub fn to_world(&self, sx: f64, sy: f64) -> P {
        P::new(
            self.centre.x + (sx - self.width_px / 2.0) / self.scale,
            self.centre.y - (sy - self.height_px / 2.0) / self.scale,
        )
    }

    /// Moves the drawing with the pointer: the drawing follows a drag of `dx` and `dy` pixels.
    pub fn pan_px(&mut self, dx: f64, dy: f64) {
        self.centre.x -= dx / self.scale;
        self.centre.y += dy / self.scale;
    }

    /// Multiplies the zoom by `factor` keeping the point under the pixel where it is. The zoom
    /// stays inside the limits.
    pub fn zoom_at(&mut self, sx: f64, sy: f64, factor: f64, limits: ZoomLimits) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let anchor = self.to_world(sx, sy);
        self.scale = (self.scale * factor).clamp(limits.min, limits.max);
        self.centre.x = anchor.x - (sx - self.width_px / 2.0) / self.scale;
        self.centre.y = anchor.y + (sy - self.height_px / 2.0) / self.scale;
    }

    /// Shows a whole box with a free margin (a fraction of each side) around it.
    pub fn fit(&mut self, bounds: Bounds, margin_fraction: f64, limits: ZoomLimits) {
        if bounds.is_empty() {
            return;
        }
        let usable = (1.0 - 2.0 * margin_fraction).max(0.01);
        let w = bounds.width().max(1e-6);
        let h = bounds.height().max(1e-6);
        let scale = (self.width_px * usable / w).min(self.height_px * usable / h);
        self.scale = scale.clamp(limits.min, limits.max);
        self.centre = bounds.centre();
    }

    /// The part of the drawing that the window shows.
    pub fn visible_area(&self) -> Bounds {
        let a = self.to_world(0.0, self.height_px);
        let b = self.to_world(self.width_px, 0.0);
        Bounds { min: a, max: b }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: ZoomLimits = ZoomLimits {
        min: 0.05,
        max: 4000.0,
    };

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn screen_and_world_are_inverse_and_y_is_flipped() {
        let v = View {
            centre: P::new(10.0, 20.0),
            scale: 8.0,
            width_px: 400.0,
            height_px: 300.0,
        };
        let (sx, sy) = v.to_screen(P::new(10.0, 20.0));
        assert!(near(sx, 200.0) && near(sy, 150.0));
        // a point above the centre is higher on the screen, so its pixel row is smaller
        let (_, up) = v.to_screen(P::new(10.0, 21.0));
        assert!(up < sy);
        let p = P::new(13.5, 17.25);
        let (x, y) = v.to_screen(p);
        let back = v.to_world(x, y);
        assert!(near(back.x, p.x) && near(back.y, p.y));
    }

    #[test]
    fn a_drag_moves_the_drawing_with_the_pointer() {
        let mut v = View::default();
        let p = P::new(3.0, 4.0);
        let (x0, y0) = v.to_screen(p);
        v.pan_px(25.0, -10.0);
        let (x1, y1) = v.to_screen(p);
        assert!(near(x1 - x0, 25.0) && near(y1 - y0, -10.0));
    }

    #[test]
    fn the_wheel_zooms_about_the_pointer() {
        let mut v = View::default();
        let (sx, sy) = (120.0, 410.0);
        let before = v.to_world(sx, sy);
        v.zoom_at(sx, sy, 2.0, LIMITS);
        assert!(near(v.scale, 8.0));
        let after = v.to_world(sx, sy);
        assert!(near(before.x, after.x) && near(before.y, after.y));
        v.zoom_at(sx, sy, 0.25, LIMITS);
        let again = v.to_world(sx, sy);
        assert!(near(before.x, again.x) && near(before.y, again.y));
    }

    #[test]
    fn the_zoom_stops_at_its_limits_and_ignores_nonsense() {
        let mut v = View::default();
        v.zoom_at(0.0, 0.0, 1e12, LIMITS);
        assert!(near(v.scale, 4000.0));
        v.zoom_at(0.0, 0.0, 1e-12, LIMITS);
        assert!(near(v.scale, 0.05));
        let s = v.scale;
        v.zoom_at(0.0, 0.0, f64::NAN, LIMITS);
        v.zoom_at(0.0, 0.0, -1.0, LIMITS);
        v.zoom_at(0.0, 0.0, 0.0, LIMITS);
        assert!(near(v.scale, s));
    }

    #[test]
    fn fit_shows_the_whole_box_with_a_margin() {
        let mut v = View::default();
        v.resize(1000.0, 500.0);
        let b = Bounds {
            min: P::new(0.0, 0.0),
            max: P::new(100.0, 25.0),
        };
        v.fit(b, 0.05, LIMITS);
        assert!(near(v.centre.x, 50.0) && near(v.centre.y, 12.5));
        // the width is the tight side: 1000 * 0.9 / 100
        assert!(near(v.scale, 9.0));
        let area = v.visible_area();
        assert!(area.contains(b.min) && area.contains(b.max));
        // a point box does not divide by zero and a box of nothing changes nothing
        let mut w = View::default();
        w.fit(
            Bounds {
                min: P::new(1.0, 1.0),
                max: P::new(1.0, 1.0),
            },
            0.05,
            LIMITS,
        );
        assert!(near(w.scale, 4000.0));
        let before = w;
        w.fit(Bounds::EMPTY, 0.05, LIMITS);
        assert_eq!(w, before);
    }

    #[test]
    fn a_window_of_no_size_is_ignored() {
        let mut v = View::default();
        v.resize(0.0, 100.0);
        v.resize(f64::NAN, 100.0);
        v.resize(-5.0, 100.0);
        assert!(near(v.width_px, 800.0) && near(v.height_px, 600.0));
        v.resize(640.0, 480.0);
        assert!(near(v.width_px, 640.0) && near(v.height_px, 480.0));
    }
}
