// SPDX-License-Identifier: GPL-3.0-or-later
//! The eight symmetries of the square that the geometric hash tries, and the rounding to the
//! grid, checked with points and angles worked out by hand.

use eh3d_phoenix_sanitizer::hash::{snap, Transform};

const DEG: i64 = 1_000_000;

fn t(quarter: u8, mirror: bool) -> Transform {
    Transform { quarter, mirror }
}

#[test]
fn the_settings_choose_which_symmetries_are_tried_and_the_identity_comes_first() {
    let all = Transform::allowed(true, true);
    assert_eq!(
        all,
        vec![
            t(0, false),
            t(1, false),
            t(2, false),
            t(3, false),
            t(0, true),
            t(1, true),
            t(2, true),
            t(3, true)
        ]
    );
    assert_eq!(Transform::allowed(true, false).len(), 4);
    assert!(Transform::allowed(true, false).iter().all(|x| !x.mirror));
    assert_eq!(
        Transform::allowed(false, true),
        vec![t(0, false), t(0, true)]
    );
    assert_eq!(Transform::allowed(false, false), vec![Transform::IDENTITY]);
}

#[test]
fn each_symmetry_has_its_own_small_number() {
    let ids: Vec<u8> = Transform::allowed(true, true)
        .iter()
        .map(|x| x.id())
        .collect();
    assert_eq!(ids, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(Transform::IDENTITY.id(), 0);
}

#[test]
fn only_the_identity_is_the_identity() {
    assert!(Transform::IDENTITY.is_identity());
    assert!(!t(1, false).is_identity());
    assert!(!t(0, true).is_identity());
}

#[test]
fn a_point_is_turned_counter_clockwise_after_the_mirror_in_x() {
    // the point (3, 5)
    let at = |quarter, mirror| t(quarter, mirror).point(3, 5);
    assert_eq!(at(0, false), (3, 5));
    assert_eq!(at(1, false), (-5, 3));
    assert_eq!(at(2, false), (-3, -5));
    assert_eq!(at(3, false), (5, -3));
    // the mirror changes the sign of x first, and the turn follows
    assert_eq!(at(0, true), (-3, 5));
    assert_eq!(at(1, true), (-5, -3));
    assert_eq!(at(2, true), (3, -5));
    assert_eq!(at(3, true), (5, 3));
    // the turns are counted modulo four
    assert_eq!(at(5, false), at(1, false));
}

#[test]
fn four_quarter_turns_and_two_mirrors_bring_a_point_back() {
    let mut p = (3, 5);
    for _ in 0..4 {
        p = t(1, false).point(p.0, p.1);
    }
    assert_eq!(p, (3, 5));
    let once = t(0, true).point(3, 5);
    assert_eq!(t(0, true).point(once.0, once.1), (3, 5));
}

#[test]
fn the_origin_stays_where_it_is() {
    for x in Transform::allowed(true, true) {
        assert_eq!(x.point(0, 0), (0, 0));
    }
}

#[test]
fn a_direction_is_mirrored_about_the_vertical_axis_and_turned_in_one_turn() {
    let d = |angle_deg: i64, quarter, mirror| t(quarter, mirror).direction(angle_deg * DEG);
    assert_eq!(d(30, 0, false), 30 * DEG);
    assert_eq!(d(30, 1, false), 120 * DEG);
    assert_eq!(d(30, 3, false), 300 * DEG);
    // 270 and a half turn is 450, which is 90 in one turn
    assert_eq!(d(270, 2, false), 90 * DEG);
    // the mirror takes the angle from 180 degrees: 180 - 30 = 150
    assert_eq!(d(30, 0, true), 150 * DEG);
    assert_eq!(d(30, 1, true), 240 * DEG);
    // 180 - 200 = -20, which is 340
    assert_eq!(d(200, 0, true), 340 * DEG);
    // a whole turn is zero
    assert_eq!(d(360, 0, false), 0);
}

#[test]
fn the_orientation_of_a_pad_reverses_in_a_mirror_and_adds_the_quarter_turns() {
    let o = |angle_deg: i64, quarter, mirror| t(quarter, mirror).orientation(angle_deg * DEG);
    assert_eq!(o(30, 0, false), 30 * DEG);
    assert_eq!(o(30, 2, false), 210 * DEG);
    assert_eq!(o(30, 0, true), 330 * DEG);
    assert_eq!(o(30, 1, true), 60 * DEG);
    assert_eq!(o(-90, 0, false), 270 * DEG);
}

#[test]
fn the_angle_of_a_rectangle_only_changes_in_a_mirror() {
    let r = |angle_deg: i64, quarter, mirror| t(quarter, mirror).rect_angle(angle_deg * DEG);
    // its corners carry the quarter turns, so the turn leaves the angle alone
    for quarter in 0..4 {
        assert_eq!(r(30, quarter, false), 30 * DEG);
    }
    assert_eq!(r(30, 0, true), 330 * DEG);
    assert_eq!(r(30, 3, true), 330 * DEG);
    assert_eq!(r(-90, 0, false), 270 * DEG);
    assert_eq!(r(360, 1, false), 0);
}

#[test]
fn rounding_goes_half_away_from_zero() {
    assert_eq!(snap(14, 10), 10);
    assert_eq!(snap(15, 10), 20);
    assert_eq!(snap(-14, 10), -10);
    assert_eq!(snap(-15, 10), -20);
    assert_eq!(snap(0, 10), 0);
    assert_eq!(snap(4, 3), 3);
    assert_eq!(snap(5, 3), 6);
    assert_eq!(snap(-5, 3), -6);
    // a step of zero or less is a step of one: nothing is rounded
    assert_eq!(snap(7, 0), 7);
    assert_eq!(snap(-7, -5), -7);
}

#[test]
fn rounding_is_odd_and_does_not_round_twice() {
    for value in (-100..=100).step_by(7) {
        for step in [1, 3, 10, 25] {
            assert_eq!(snap(-value, step), -snap(value, step), "{value} {step}");
            assert_eq!(snap(snap(value, step), step), snap(value, step));
        }
    }
}

#[test]
fn a_turn_or_a_mirror_gives_the_same_point_before_and_after_the_rounding() {
    let points = [(14, 26), (-15, 35), (4, -4), (0, 5), (-6, -14), (25, 25)];
    for x in Transform::allowed(true, true) {
        for (px, py) in points {
            let after = {
                let moved = x.point(px, py);
                (snap(moved.0, 10), snap(moved.1, 10))
            };
            let before = x.point(snap(px, 10), snap(py, 10));
            assert_eq!(after, before, "{x:?} {px} {py}");
        }
    }
}
