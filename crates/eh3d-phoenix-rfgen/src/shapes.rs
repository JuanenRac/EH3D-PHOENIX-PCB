// SPDX-License-Identifier: GPL-3.0-or-later
//! Shapes that more than one generator draws: the right-angle bend with a mitered outer corner.

use crate::codes::{RfgenError, NOT_REALISABLE, PARAMETER_INVALID};
use crate::layout::{nm_from_f64, P};

/// The length of the cut along each outer edge of a mitered right-angle bend of width `w_nm`, for
/// a miter fraction `m` of the diagonal of the corner square (Douville and James): the cut
/// crosses the outer edges at `2 m w` from the outer corner, rounded to a nanometre.
pub fn miter_leg_nm(w_nm: i64, fraction: f64) -> Result<i64, RfgenError> {
    nm_from_f64(2.0 * fraction * w_nm as f64)
}

/// The outline of a right-angle bend of width `w_nm` with the outer corner cut off.
///
/// The centre line comes in along the unit vector `d_in`, reaches `corner`, and turns left or
/// right. The incoming arm reaches `a_in` back from the corner and the outgoing arm `a_out`
/// forward, measured along the centre lines. The cut passes `leg_nm` from the outer corner along
/// both outer edges. The points are counter-clockwise, all in integers: the rotation is a whole
/// number of quarter turns.
pub fn mitered_bend(
    corner: P,
    d_in: P,
    left: bool,
    w_nm: i64,
    a_in: i64,
    a_out: i64,
    leg_nm: i64,
) -> Result<Vec<P>, RfgenError> {
    let turns = match d_in {
        (1, 0) => 0,
        (0, 1) => 1,
        (-1, 0) => 2,
        (0, -1) => 3,
        _ => {
            return Err(RfgenError::new(
                PARAMETER_INVALID,
                "the direction of a bend must be along an axis",
            ))
        }
    };
    if w_nm <= 0 || (w_nm & 1) != 0 {
        return Err(RfgenError::new(
            PARAMETER_INVALID,
            "the width of a bend must be a positive even number of nanometres",
        ));
    }
    let half = w_nm / 2;
    if leg_nm <= 0 || a_in < leg_nm - half || a_out < leg_nm - half || a_in <= half || a_out <= half
    {
        return Err(RfgenError::new(
            NOT_REALISABLE,
            "the arms of the bend are shorter than its miter",
        ));
    }
    // incoming along +x, outer corner at the lower right, outgoing along +y (a left turn)
    let mut points: Vec<P> = vec![
        (-a_in, -half),
        (half - leg_nm, -half),
        (half, -half + leg_nm),
        (half, a_out),
        (-half, a_out),
        (-half, half),
        (-a_in, half),
    ];
    if !left {
        points = points.into_iter().map(|(x, y)| (x, -y)).rev().collect();
    }
    Ok(points
        .into_iter()
        .map(|(x, y)| {
            let (x, y) = match turns {
                0 => (x, y),
                1 => (-y, x),
                2 => (-x, -y),
                _ => (y, -x),
            };
            (x + corner.0, y + corner.1)
        })
        .collect())
}

/// Twice the signed area of a polygon, positive when it runs counter-clockwise.
pub fn twice_area(points: &[P]) -> i128 {
    let mut sum: i128 = 0;
    for (i, a) in points.iter().enumerate() {
        let b = points[(i + 1) % points.len()];
        sum += i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1);
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_left_bend_has_the_cut_on_the_outer_corner() {
        let w = 2_000_000;
        let leg = miter_leg_nm(w, 0.5).unwrap();
        assert_eq!(leg, 2_000_000);
        let p = mitered_bend((0, 0), (1, 0), true, w, 5_000_000, 6_000_000, leg).unwrap();
        assert_eq!(p.len(), 7);
        assert_eq!(p[0], (-5_000_000, -1_000_000));
        assert_eq!(p[1], (-1_000_000, -1_000_000));
        assert_eq!(p[2], (1_000_000, 1_000_000));
        assert_eq!(p[3], (1_000_000, 6_000_000));
        assert_eq!(p[5], (-1_000_000, 1_000_000));
        assert!(twice_area(&p) > 0);
    }

    #[test]
    fn the_area_is_that_of_the_two_arms_minus_the_cut() {
        // two arms of width w: (a_in + half) * w + (a_out - half) * w ... the L has the area of the
        // arm along x from -a_in to +half and the arm along y from -half to a_out, with the
        // corner square counted once; the cut removes a right triangle with legs `leg`
        let (w, a_in, a_out, leg) = (
            2_000_000_i128,
            5_000_000_i128,
            6_000_000_i128,
            1_500_000_i128,
        );
        let l_shape = (a_in + w / 2) * w + (a_out - w / 2) * w;
        let cut = leg * leg / 2;
        let p = mitered_bend(
            (0, 0),
            (1, 0),
            true,
            w as i64,
            a_in as i64,
            a_out as i64,
            leg as i64,
        )
        .unwrap();
        assert_eq!(twice_area(&p), 2 * (l_shape - cut));
    }

    #[test]
    fn a_right_turn_and_other_directions_keep_the_area_and_the_orientation() {
        let area = twice_area(
            &mitered_bend(
                (0, 0),
                (1, 0),
                true,
                2_000_000,
                5_000_000,
                6_000_000,
                1_500_000,
            )
            .unwrap(),
        );
        for (d, left) in [
            ((1, 0), false),
            ((0, 1), true),
            ((0, 1), false),
            ((-1, 0), true),
            ((-1, 0), false),
            ((0, -1), true),
            ((0, -1), false),
        ] {
            let p =
                mitered_bend((7, -3), d, left, 2_000_000, 5_000_000, 6_000_000, 1_500_000).unwrap();
            assert_eq!(twice_area(&p), area, "{d:?} {left}");
        }
    }

    #[test]
    fn the_outgoing_arm_ends_where_the_direction_says() {
        // heading down, turning left: the outgoing arm runs along +x
        let p = mitered_bend(
            (100, 200),
            (0, -1),
            true,
            2_000_000,
            4_000_000,
            5_000_000,
            1_000_000,
        )
        .unwrap();
        let far = p.iter().map(|q| q.0).max().unwrap();
        assert_eq!(far, 100 + 5_000_000);
        let back = p.iter().map(|q| q.1).max().unwrap();
        assert_eq!(back, 200 + 4_000_000);
    }

    #[test]
    fn a_bad_bend_is_refused() {
        assert!(mitered_bend((0, 0), (1, 1), true, 2_000_000, 5, 5, 1).is_err());
        assert!(mitered_bend((0, 0), (1, 0), true, 2_000_001, 5_000_000, 5_000_000, 1).is_err());
        assert!(mitered_bend((0, 0), (1, 0), true, 0, 5, 5, 1).is_err());
        // arms shorter than the miter
        assert!(mitered_bend(
            (0, 0),
            (1, 0),
            true,
            2_000_000,
            1_000_000,
            5_000_000,
            3_000_000
        )
        .is_err());
        assert!(mitered_bend((0, 0), (1, 0), true, 2_000_000, 5_000_000, 5_000_000, 0).is_err());
    }
}
