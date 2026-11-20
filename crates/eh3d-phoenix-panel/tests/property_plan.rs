// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests with a fixed seed of the plan of a panel: boards of random sizes in matrices of random
//! counts, joints, frames, turns and mirrors give plans that keep the same promises whatever the
//! numbers are. The generator always gives the same cases, so a failure is the same on every run.

use eh3d_phoenix_panel::{plan, BoardShape, Frame, Joint, PanelPlan, PanelSettings, Pt};

const MM: i64 = 1_000_000;

/// A small generator that always gives the same numbers for the same seed.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn range(&mut self, low: i64, high: i64) -> i64 {
        low + (self.next() % (high - low + 1) as u64) as i64
    }
}

fn board(width: i64, height: i64) -> BoardShape {
    BoardShape {
        origin: Pt { x: 0, y: 0 },
        width,
        height,
        holes: Vec::new(),
    }
}

/// Settings made of random choices; the frame, the marks and the joints are the ones the
/// defaults give, so that every part of the plan takes part.
fn random_settings(rng: &mut Lcg) -> PanelSettings {
    let frame = match rng.next() % 3 {
        0 => Frame::None,
        1 => Frame::LeftRight,
        _ => Frame::All,
    };
    let framed = frame != Frame::None;
    PanelSettings {
        columns: rng.range(1, 5) as u32,
        rows: rng.range(1, 5) as u32,
        joint: if rng.next().is_multiple_of(2) {
            Joint::MouseBites
        } else {
            Joint::VScore
        },
        gap_nm: rng.range(1, 5) * MM,
        frame,
        fiducial_count: 0,
        tooling_hole_count: if framed { 4 } else { 0 },
        copy_rotation_deg: 90 * (rng.next() % 4) as u32,
        copy_mirror: rng.next().is_multiple_of(2),
        ..PanelSettings::default()
    }
}

struct Case {
    shape: BoardShape,
    settings: PanelSettings,
    plan: PanelPlan,
}

/// The cases whose plan was made; the ones the plan refused are counted apart.
fn cases(seed: u64, wanted: usize) -> (Vec<Case>, usize) {
    let mut rng = Lcg(seed);
    let mut made = Vec::new();
    let mut refused = 0;
    while made.len() < wanted {
        let shape = board(rng.range(8, 60) * MM, rng.range(8, 60) * MM);
        let settings = random_settings(&mut rng);
        match plan(&shape, &settings) {
            Ok(plan) => made.push(Case {
                shape,
                settings,
                plan,
            }),
            Err(_) => refused += 1,
        }
        assert!(
            refused < 10_000,
            "the generator makes too many refused plans"
        );
    }
    (made, refused)
}

#[test]
fn the_same_input_gives_the_same_plan() {
    let (cases, _) = cases(0x5EED_0201, 60);
    for case in &cases {
        let again = plan(&case.shape, &case.settings).expect("the plan was made once");
        assert_eq!(again, case.plan);
    }
}

#[test]
fn there_is_a_copy_for_each_place_and_it_lies_inside_the_panel() {
    let (cases, _) = cases(0x5EED_0202, 150);
    for (n, case) in cases.iter().enumerate() {
        let want = (case.settings.columns * case.settings.rows) as usize;
        let p = &case.plan;
        assert_eq!(p.copies.len(), want, "case {n}");
        assert_eq!(p.origins.len(), want, "case {n}");
        for (copy, origin) in p.copies.iter().zip(&p.origins) {
            assert_eq!(copy.origin, *origin, "case {n}");
            assert!(copy.origin.x >= 0 && copy.origin.y >= 0, "case {n}");
            assert!(copy.origin.x + copy.width <= p.width, "case {n}");
            assert!(copy.origin.y + copy.height <= p.height, "case {n}");
        }
    }
}

#[test]
fn no_two_boards_of_a_panel_overlap_and_the_use_of_the_panel_is_at_most_all_of_it() {
    let (cases, _) = cases(0x5EED_0203, 150);
    for (n, case) in cases.iter().enumerate() {
        let copies = &case.plan.copies;
        for (i, a) in copies.iter().enumerate() {
            for b in &copies[i + 1..] {
                let apart_x =
                    a.origin.x + a.width <= b.origin.x || b.origin.x + b.width <= a.origin.x;
                let apart_y =
                    a.origin.y + a.height <= b.origin.y || b.origin.y + b.height <= a.origin.y;
                assert!(apart_x || apart_y, "case {n}: two boards overlap");
            }
        }
        assert!(case.plan.utilisation_permille() <= 1000, "case {n}");
        assert!(case.plan.boards_area() > 0, "case {n}");
    }
}

#[test]
fn a_turn_of_a_quarter_swaps_the_sides_of_every_copy() {
    let (cases, _) = cases(0x5EED_0204, 100);
    for (n, case) in cases.iter().enumerate() {
        let quarter = case.settings.copy_rotation_deg % 180 == 90;
        for copy in &case.plan.copies {
            let (w, h) = if quarter {
                (case.shape.height, case.shape.width)
            } else {
                (case.shape.width, case.shape.height)
            };
            assert_eq!((copy.width, copy.height), (w, h), "case {n}");
        }
    }
}

#[test]
fn a_turned_board_makes_the_panel_of_the_board_with_its_sides_swapped() {
    let mut rng = Lcg(0x5EED_0205);
    let mut compared = 0;
    for _ in 0..120 {
        let (w, h) = (rng.range(8, 60) * MM, rng.range(8, 60) * MM);
        let mut settings = random_settings(&mut rng);
        settings.copy_rotation_deg = 0;
        let mut turned = settings.clone();
        turned.copy_rotation_deg = 90;
        if let (Ok(plain), Ok(swapped)) =
            (plan(&board(h, w), &settings), plan(&board(w, h), &turned))
        {
            assert_eq!((plain.width, plain.height), (swapped.width, swapped.height));
            assert_eq!(plain.origins, swapped.origins);
            compared += 1;
        }
    }
    assert!(compared > 60, "{compared}");
}

#[test]
fn the_v_score_lines_are_those_between_the_boards_and_between_the_boards_and_the_rails() {
    let (cases, _) = cases(0x5EED_0206, 200);
    let mut scored = 0;
    for (n, case) in cases.iter().enumerate() {
        if case.settings.joint == Joint::VScore {
            let between = case.settings.columns - 1 + case.settings.rows - 1;
            let rails = match case.settings.frame {
                Frame::None => 0,
                Frame::LeftRight => 2,
                _ => 4,
            };
            assert_eq!(
                case.plan.score.len(),
                (between + rails) as usize,
                "case {n}"
            );
            scored += 1;
        } else {
            assert!(case.plan.score.is_empty(), "case {n}");
        }
    }
    assert!(scored > 40, "{scored}");
}

#[test]
fn a_panel_is_never_smaller_than_its_boards_side_by_side() {
    let (cases, _) = cases(0x5EED_0207, 150);
    for (n, case) in cases.iter().enumerate() {
        let (w, h) = if case.settings.copy_rotation_deg % 180 == 90 {
            (case.shape.height, case.shape.width)
        } else {
            (case.shape.width, case.shape.height)
        };
        assert!(
            case.plan.width >= i64::from(case.settings.columns) * w,
            "case {n}"
        );
        assert!(
            case.plan.height >= i64::from(case.settings.rows) * h,
            "case {n}"
        );
    }
}
