// SPDX-License-Identifier: GPL-3.0-or-later
//! The `Xform` of a placed feature: a mirror, a turn, a scale and a move, applied to whole
//! nanometres.
//!
//! The file says how a group of features is placed with an `Xform` (`mirror`, `rotation` in degrees
//! counter-clockwise, `scale`, `xOffset` and `yOffset`) and a `Location`. The order is the one of
//! the standard: the mirror about the vertical axis first (`x` becomes `-x`), then the scale, then
//! the turn about the origin, then the move by the offsets and the location. A turn by a multiple
//! of a quarter of a turn is done with exact integer arithmetic; any other angle goes through the
//! sine and the cosine and is rounded half away from zero. Nothing wraps: a result that does not
//! fit in 64 bits is held at the limit of the type.

/// A mirror, a scale, a turn and a move.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xf {
    /// The move in x after the turn, in nanometres.
    pub dx: i64,
    /// The move in y after the turn, in nanometres.
    pub dy: i64,
    /// The turn in degrees, counter-clockwise.
    pub rotation_deg: f64,
    /// Whether x is mirrored (about the vertical axis) before the turn.
    pub mirror: bool,
    /// The scale; 1.0 leaves the size.
    pub scale: f64,
}

impl Xf {
    /// Only a move.
    pub const fn at(dx: i64, dy: i64) -> Xf {
        Xf {
            dx,
            dy,
            rotation_deg: 0.0,
            mirror: false,
            scale: 1.0,
        }
    }

    /// True when the place of a point does not change but for the move.
    pub fn is_plain(&self) -> bool {
        !self.mirror && self.rotation_deg == 0.0 && self.scale == 1.0
    }

    /// True when the direction of an arc is reversed (a mirror reverses it).
    pub fn reverses_arcs(&self) -> bool {
        self.mirror
    }

    /// Where a point of the group goes.
    pub fn apply(&self, point: (i64, i64)) -> (i64, i64) {
        if self.is_plain() {
            return (
                point.0.saturating_add(self.dx),
                point.1.saturating_add(self.dy),
            );
        }
        let mut x = i128::from(point.0);
        let y = i128::from(point.1);
        if self.mirror {
            x = -x;
        }
        let (mut x, mut y) = if self.scale == 1.0 {
            (x, y)
        } else {
            (scaled(x, self.scale), scaled(y, self.scale))
        };
        (x, y) = turn(x, y, self.rotation_deg);
        (
            clamp(x.saturating_add(i128::from(self.dx))),
            clamp(y.saturating_add(i128::from(self.dy))),
        )
    }
}

fn clamp(v: i128) -> i64 {
    i64::try_from(v).unwrap_or(if v < 0 { i64::MIN } else { i64::MAX })
}

fn scaled(v: i128, scale: f64) -> i128 {
    if !scale.is_finite() {
        return v;
    }
    let r = (v as f64 * scale).round();
    if r >= 1.0e30 {
        i128::from(i64::MAX)
    } else if r <= -1.0e30 {
        i128::from(i64::MIN)
    } else {
        r as i128
    }
}

fn turn(x: i128, y: i128, degrees: f64) -> (i128, i128) {
    if !degrees.is_finite() {
        return (x, y);
    }
    let d = degrees.rem_euclid(360.0);
    if d == 0.0 {
        return (x, y);
    }
    if d == 90.0 {
        return (-y, x);
    }
    if d == 180.0 {
        return (-x, -y);
    }
    if d == 270.0 {
        return (y, -x);
    }
    let (s, c) = d.to_radians().sin_cos();
    let nx = (x as f64 * c - y as f64 * s).round();
    let ny = (x as f64 * s + y as f64 * c).round();
    (nx as i128, ny as i128)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_move_only_adds_the_offset() {
        let xf = Xf::at(10, -20);
        assert!(xf.is_plain());
        assert_eq!(xf.apply((1, 2)), (11, -18));
        assert_eq!(xf.apply((i64::MAX, 0)).0, i64::MAX);
    }

    #[test]
    fn quarter_turns_are_exact() {
        let mut xf = Xf::at(0, 0);
        for (deg, want) in [
            (90.0, (-2, 1)),
            (180.0, (-1, -2)),
            (270.0, (2, -1)),
            (360.0, (1, 2)),
            (-90.0, (2, -1)),
            (450.0, (-2, 1)),
        ] {
            xf.rotation_deg = deg;
            assert_eq!(xf.apply((1, 2)), want, "{deg}");
        }
    }

    #[test]
    fn the_mirror_comes_before_the_turn_and_the_move_after() {
        let xf = Xf {
            dx: 100,
            dy: 200,
            rotation_deg: 90.0,
            mirror: true,
            scale: 1.0,
        };
        // (3, 4) -> mirror (-3, 4) -> turn 90 (-4, -3) -> move (96, 197)
        assert_eq!(xf.apply((3, 4)), (96, 197));
        assert!(xf.reverses_arcs());
    }

    #[test]
    fn other_angles_round_half_away_from_zero() {
        let xf = Xf {
            rotation_deg: 45.0,
            ..Xf::at(0, 0)
        };
        assert_eq!(xf.apply((1_000_000, 0)), (707_107, 707_107));
        let back = Xf {
            rotation_deg: -45.0,
            ..Xf::at(0, 0)
        };
        assert_eq!(back.apply((1_000_000, 0)), (707_107, -707_107));
    }

    #[test]
    fn a_scale_multiplies_before_the_turn() {
        let xf = Xf {
            scale: 2.0,
            rotation_deg: 90.0,
            ..Xf::at(1, 1)
        };
        assert_eq!(xf.apply((3, 0)), (1, 7));
    }

    #[test]
    fn nothing_wraps_and_a_bad_angle_is_ignored() {
        let xf = Xf {
            rotation_deg: 180.0,
            ..Xf::at(0, 0)
        };
        assert_eq!(xf.apply((i64::MIN, 0)).0, i64::MAX);
        let nan = Xf {
            rotation_deg: f64::NAN,
            ..Xf::at(5, 5)
        };
        assert_eq!(nan.apply((1, 1)), (6, 6));
        let wild = Xf {
            scale: f64::INFINITY,
            mirror: true,
            ..Xf::at(0, 0)
        };
        assert_eq!(wild.apply((2, 3)), (-2, 3));
    }

    #[test]
    fn a_turn_and_its_inverse_agree_with_a_fixed_seed() {
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..200 {
            let x = (next() % 2_000_000_000) as i64 - 1_000_000_000;
            let y = (next() % 2_000_000_000) as i64 - 1_000_000_000;
            let quarter = [0.0, 90.0, 180.0, 270.0][(next() % 4) as usize];
            let there = Xf {
                rotation_deg: quarter,
                mirror: next() % 2 == 0,
                ..Xf::at(0, 0)
            };
            let back = Xf {
                rotation_deg: -quarter,
                ..Xf::at(0, 0)
            };
            let (tx, ty) = there.apply((x, y));
            let (rx, ry) = back.apply((tx, ty));
            let want_x = if there.mirror { -x } else { x };
            assert_eq!((rx, ry), (want_x, y));
        }
    }
}
