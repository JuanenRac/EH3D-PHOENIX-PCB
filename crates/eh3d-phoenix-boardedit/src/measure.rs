// SPDX-License-Identifier: GPL-3.0-or-later
//! The measure on the board: two clicks, each of which goes to the centre of a pad or of a via, to
//! the end of a piece of track or to a corner of the outline when one is near, else to the grid when
//! snapping is on. The readings are in whole nanometres; only the angle is a floating-point number.

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, PadRef};
use eh3d_phoenix_model::Point;

use crate::tool::EditState;

/// What a click of the measure went to. The order is the one of preference at the same distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapTo {
    /// The centre of a pad.
    Pad,
    /// The centre of a via.
    Via,
    /// An end of a piece of track.
    TrackEnd,
    /// A corner of the outline or of a cutout.
    Corner,
    /// A point of the grid.
    Grid,
    /// The point that was clicked.
    Free,
}

/// A point of a measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasurePoint {
    /// Where it is.
    pub at: Point,
    /// What it went to.
    pub to: SnapTo,
}

/// The readings between two points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// The difference in x, in nanometres.
    pub dx_nm: i64,
    /// The difference in y, in nanometres.
    pub dy_nm: i64,
    /// The straight distance, in nanometres, rounded to the nearest one.
    pub length_nm: i64,
    /// The angle from the positive x axis, counter-clockwise, in degrees, in (-180, 180].
    pub angle_deg: f64,
}

/// The distance squared between two points.
fn squared(a: Point, b: Point) -> i128 {
    let dx = i128::from(a.x_nm.get()) - i128::from(b.x_nm.get());
    let dy = i128::from(a.y_nm.get()) - i128::from(b.y_nm.get());
    dx * dx + dy * dy
}

impl Reading {
    /// The readings from `a` to `b`.
    pub fn between(a: Point, b: Point) -> Reading {
        let dx = b.x_nm.get() - a.x_nm.get();
        let dy = b.y_nm.get() - a.y_nm.get();
        let square = squared(a, b);
        let root = square.isqrt();
        // to the nearest: the next whole number is nearer when the square is past the half way
        let length = if square - root * root > root {
            root + 1
        } else {
            root
        };
        Reading {
            dx_nm: dx,
            dy_nm: dy,
            length_nm: i64::try_from(length).unwrap_or(i64::MAX),
            angle_deg: (dy as f64).atan2(dx as f64).to_degrees(),
        }
    }
}

/// The notable point of the board nearest to `p` within `radius_nm`, with what it is.
pub fn nearest_notable(board: &Board, p: Point, radius_nm: i64) -> Option<MeasurePoint> {
    let limit = i128::from(radius_nm.max(0)).pow(2);
    let mut best: Option<(i128, SnapTo, (i64, i64), Point)> = None;
    let mut offer = |at: Point, to: SnapTo| {
        let distance = squared(p, at);
        if distance > limit {
            return;
        }
        let key = (distance, to, (at.x_nm.get(), at.y_nm.get()));
        if best.is_none_or(|(d, k, place, _)| key < (d, k, place)) {
            best = Some((key.0, key.1, key.2, at));
        }
    };
    for (cid, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for pid in footprint.pads.keys() {
            let pad = PadRef {
                component: *cid,
                pad: *pid,
            };
            if let Some(at) = pad_position(board, pad) {
                offer(at, SnapTo::Pad);
            }
        }
    }
    for via in board.vias.values() {
        offer(via.at, SnapTo::Via);
    }
    for track in board.tracks.values() {
        for piece in &track.path {
            offer(piece.start(), SnapTo::TrackEnd);
            offer(piece.end(), SnapTo::TrackEnd);
        }
    }
    for corner in &board.outline {
        offer(*corner, SnapTo::Corner);
    }
    for corner in board.outline_cutouts.iter().flatten() {
        offer(*corner, SnapTo::Corner);
    }
    best.map(|(_, to, _, at)| MeasurePoint { at, to })
}

/// The measure in progress or done.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoardMeasure {
    /// The first point.
    pub a: Option<MeasurePoint>,
    /// The second point.
    pub b: Option<MeasurePoint>,
}

impl BoardMeasure {
    /// Where a click at `p` would go on the board of the editor. The radius of the snap is the
    /// one of the settings, or `radius_nm` when that is larger (a view that is far out needs a
    /// larger one).
    pub fn snap(state: &EditState, p: Point, radius_nm: i64) -> MeasurePoint {
        let radius = state.edit.settings.measure_snap_nm.max(radius_nm);
        if let Some(found) = nearest_notable(state.board(), p, radius) {
            return found;
        }
        if state.edit.settings.snap {
            return MeasurePoint {
                at: state.snap(p),
                to: SnapTo::Grid,
            };
        }
        MeasurePoint {
            at: p,
            to: SnapTo::Free,
        }
    }

    /// A click: the first sets the start, the second the end and the third starts again.
    pub fn click(&mut self, point: MeasurePoint) {
        match (self.a, self.b) {
            (Some(_), None) => self.b = Some(point),
            _ => {
                self.a = Some(point);
                self.b = None;
            }
        }
    }

    /// Forgets both points.
    pub fn clear(&mut self) {
        *self = BoardMeasure::default();
    }

    /// The readings: between the two points, or from the first to where the pointer would go
    /// while the second is not set.
    pub fn reading(&self, pointer: Option<MeasurePoint>) -> Option<Reading> {
        let a = self.a?;
        let b = self.b.or(pointer)?;
        Some(Reading::between(a.at, b.at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::BoardEdit;
    use crate::settings::BoardEditSettings;
    use crate::testutil::{fixture, strict_rules};

    fn nm(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn state() -> EditState {
        EditState::new(
            BoardEdit::new(
                fixture().board,
                strict_rules(),
                BoardEditSettings::default(),
            )
            .unwrap(),
        )
    }

    #[test]
    fn a_click_goes_to_the_pad_the_end_the_corner_or_the_grid() {
        let s = state();
        // R1.2 is at (11.5, 10) and the track starts there: the pad wins over the end
        let found = BoardMeasure::snap(&s, nm(11_700_000, 10_100_000), 0);
        assert_eq!(
            (found.at, found.to),
            (nm(11_500_000, 10_000_000), SnapTo::Pad)
        );
        let found = BoardMeasure::snap(&s, nm(200_000, -300_000), 0);
        assert_eq!((found.at, found.to), (nm(0, 0), SnapTo::Corner));
        let found = BoardMeasure::snap(&s, nm(20_100_000, 20_080_000), 0);
        assert_eq!(
            (found.at, found.to),
            (nm(20_000_000, 20_000_000), SnapTo::Grid)
        );
        let mut free = state();
        free.edit.settings.snap = false;
        let found = BoardMeasure::snap(&free, nm(20_100_000, 20_080_000), 0);
        assert_eq!(
            (found.at, found.to),
            (nm(20_100_000, 20_080_000), SnapTo::Free)
        );
        // a larger radius, as of a view far out, reaches a notable point farther away
        let found = BoardMeasure::snap(&free, nm(13_000_000, 10_000_000), 2_000_000);
        assert_eq!(found.to, SnapTo::Pad);
    }

    #[test]
    fn two_clicks_measure_and_the_third_starts_again() {
        let point = |x, y| MeasurePoint {
            at: nm(x, y),
            to: SnapTo::Free,
        };
        let mut m = BoardMeasure::default();
        assert_eq!(m.reading(Some(point(1, 1))), None);
        m.click(point(0, 0));
        let live = m.reading(Some(point(3_000_000, 4_000_000))).unwrap();
        assert_eq!(live.length_nm, 5_000_000);
        m.click(point(0, -2_000_000));
        let done = m.reading(Some(point(9, 9))).unwrap();
        assert_eq!(
            (done.dx_nm, done.dy_nm, done.length_nm),
            (0, -2_000_000, 2_000_000)
        );
        assert!((done.angle_deg + 90.0).abs() < 1e-9);
        m.click(point(5, 5));
        assert_eq!((m.a.map(|p| p.at), m.b), (Some(nm(5, 5)), None));
        m.clear();
        assert_eq!(m, BoardMeasure::default());
    }

    #[test]
    fn the_length_is_the_nearest_whole_nanometre_for_random_points() {
        // a fixed seed: the same points on every run
        let mut seed = 0x1234_5678_9abc_def1u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 200_000_001) as i64 - 100_000_000
        };
        for _ in 0..2_000 {
            let (a, b) = (nm(next(), next()), nm(next(), next()));
            let reading = Reading::between(a, b);
            let square = squared(a, b);
            let length = i128::from(reading.length_nm);
            // |sqrt(square) - length| <= 1/2, in whole numbers
            assert!((2 * length - 1).pow(2) <= 4 * square || length == 0);
            assert!((2 * length + 1).pow(2) >= 4 * square);
            let back = Reading::between(b, a);
            assert_eq!(back.length_nm, reading.length_nm);
            assert_eq!((back.dx_nm, back.dy_nm), (-reading.dx_nm, -reading.dy_nm));
        }
    }

    #[test]
    fn the_snap_never_goes_farther_than_the_radius() {
        let s = state();
        let mut seed = 0x0bad_5eed_1357_9bdfu64;
        let mut next = move |span: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % span) as i64
        };
        for _ in 0..500 {
            let p = nm(next(60_000_000), next(40_000_000));
            if let Some(found) = nearest_notable(s.board(), p, 600_000) {
                assert!(squared(p, found.at) <= 600_000_i128.pow(2));
            }
        }
    }
}
