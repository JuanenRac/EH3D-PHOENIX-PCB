// SPDX-License-Identifier: GPL-3.0-or-later
//! The layout of a panel on boards written here: the turn and the mirror of a board, the origins
//! of the copies, the V-score lines, the tabs and their rows of holes, the marks of the frame and
//! the settings text. The sizes are in millimetres times one million (nanometres) and every
//! expected value is worked out by hand from the dimensions given in the test.

use eh3d_phoenix_panel::codes::{
    BITES_DONT_FIT, GAP_IGNORED, MARKS_OVERLAP, MARKS_SKIPPED, PANEL_TOO_LARGE, SETTINGS_INVALID,
    TABS_DONT_FIT,
};
use eh3d_phoenix_panel::layout::local_map;
use eh3d_phoenix_panel::{plan, BoardShape, Frame, HoleKind, Joint, PanelSettings, Placement, Pt};

const MM: i64 = 1_000_000;

fn board(width_mm: i64, height_mm: i64) -> BoardShape {
    BoardShape {
        origin: Pt { x: 0, y: 0 },
        width: width_mm * MM,
        height: height_mm * MM,
        holes: Vec::new(),
    }
}

/// A matrix with no frame and no marks, so that only the boards and their joints are left.
fn bare(columns: u32, rows: u32, joint: Joint) -> PanelSettings {
    PanelSettings {
        columns,
        rows,
        joint,
        frame: Frame::None,
        fiducial_count: 0,
        tooling_hole_count: 0,
        ..PanelSettings::default()
    }
}

fn pt(x_mm: f64, y_mm: f64) -> Pt {
    Pt {
        x: (x_mm * MM as f64).round() as i64,
        y: (y_mm * MM as f64).round() as i64,
    }
}

// ---------------------------------------------------------------------------------------------
// turning and mirroring

#[test]
fn a_point_goes_where_a_turn_and_a_mirror_put_it() {
    // a box of 10 by 4 and the point (3, 1)
    let at = |rotation, mirror| local_map(3, 1, 10, 4, rotation, mirror);
    assert_eq!(at(0, false), Pt { x: 3, y: 1 });
    // mirrored about the vertical axis: x becomes 10 - 3
    assert_eq!(at(0, true), Pt { x: 7, y: 1 });
    // 90 degrees counter-clockwise: (h - y, x)
    assert_eq!(at(90, false), Pt { x: 3, y: 3 });
    assert_eq!(at(180, false), Pt { x: 7, y: 3 });
    assert_eq!(at(270, false), Pt { x: 1, y: 7 });
    // the mirror comes first: x is 7, then (h - y, x)
    assert_eq!(at(90, true), Pt { x: 3, y: 7 });
    // a full turn and more are the same as the remainder
    assert_eq!(at(360, false), at(0, false));
    assert_eq!(at(450, false), at(90, false));
}

#[test]
fn the_corners_of_the_box_stay_the_corners_of_the_turned_box() {
    // the box of 10 by 4 turned by 90 is a box of 4 by 10
    assert_eq!(local_map(0, 0, 10, 4, 90, false), Pt { x: 4, y: 0 });
    assert_eq!(local_map(10, 4, 10, 4, 90, false), Pt { x: 0, y: 10 });
    assert_eq!(local_map(10, 0, 10, 4, 90, false), Pt { x: 4, y: 10 });
    assert_eq!(local_map(0, 4, 10, 4, 90, false), Pt { x: 0, y: 0 });
}

#[test]
fn four_quarter_turns_bring_a_point_back() {
    let (mut x, mut y, mut w, mut h) = (3, 1, 10, 4);
    for _ in 0..4 {
        let p = local_map(x, y, w, h, 90, false);
        (x, y) = (p.x, p.y);
        std::mem::swap(&mut w, &mut h);
    }
    assert_eq!((x, y, w, h), (3, 1, 10, 4));
}

#[test]
fn a_shape_is_oriented_with_its_box_and_its_holes() {
    let shape = BoardShape {
        origin: Pt { x: 100, y: 200 },
        width: 10,
        height: 4,
        holes: vec![(true, Pt { x: 103, y: 201 }, 800)],
    };
    // the hole is at (3, 1) from the corner of the box
    let turned = shape.oriented(90, false);
    assert_eq!((turned.width, turned.height), (4, 10));
    assert_eq!(turned.origin, Pt { x: 0, y: 0 });
    assert_eq!(turned.holes, vec![(true, Pt { x: 3, y: 3 }, 800)]);
    let half = shape.oriented(180, false);
    assert_eq!((half.width, half.height), (10, 4));
    assert_eq!(half.holes, vec![(true, Pt { x: 7, y: 3 }, 800)]);
    let three = shape.oriented(270, false);
    assert_eq!((three.width, three.height), (4, 10));
    assert_eq!(three.holes[0].1, Pt { x: 1, y: 7 });
    let mirrored = shape.oriented(0, true);
    assert_eq!((mirrored.width, mirrored.height), (10, 4));
    assert_eq!(mirrored.holes[0].1, Pt { x: 7, y: 1 });
    // a turn of 450 is a turn of 90 for the box and for the holes
    assert_eq!(shape.oriented(450, false), turned);
    // nothing is changed but the origin when the board is left as it is
    let same = shape.oriented(0, false);
    assert_eq!(same.holes, vec![(true, Pt { x: 3, y: 1 }, 800)]);
}

// ---------------------------------------------------------------------------------------------
// the plan

#[test]
fn v_score_boards_touch_and_the_lines_run_between_them() {
    let settings = bare(3, 1, Joint::VScore);
    let plan = plan(&board(20, 10), &settings).unwrap();
    // three boards of 20 mm side by side with no gap
    assert_eq!((plan.width, plan.height), (60 * MM, 10 * MM));
    assert_eq!(
        plan.origins,
        vec![pt(0.0, 0.0), pt(20.0, 0.0), pt(40.0, 0.0)]
    );
    assert_eq!(plan.copies.len(), 3);
    // a line between each pair of boards, from edge to edge of the panel
    let lines: Vec<_> = plan.score.iter().map(|l| (l.from, l.to)).collect();
    assert_eq!(
        lines,
        vec![
            (pt(20.0, 0.0), pt(20.0, 10.0)),
            (pt(40.0, 0.0), pt(40.0, 10.0))
        ]
    );
    // the outline of the panel and nothing else, and no tab
    assert_eq!(plan.profile.len(), 4);
    assert!(plan.tabs.is_empty());
    assert!(plan.holes.is_empty());
    assert!(plan.fiducials.is_empty());
}

#[test]
fn a_gap_given_to_a_v_score_panel_is_ignored_and_said_so() {
    let settings = PanelSettings {
        gap_nm: 2 * MM,
        ..bare(2, 1, Joint::VScore)
    };
    let plan = plan(&board(20, 10), &settings).unwrap();
    assert_eq!(plan.width, 40 * MM);
    assert!(plan
        .notes
        .iter()
        .any(|n| n.code == GAP_IGNORED && n.detail == "gap_nm = 2000000 is not used"));
}

#[test]
fn a_frame_adds_the_lines_between_the_rails_and_the_boards() {
    let settings = PanelSettings {
        frame: Frame::LeftRight,
        frame_width_nm: 8 * MM,
        ..bare(2, 1, Joint::VScore)
    };
    let plan = plan(&board(20, 10), &settings).unwrap();
    // the rails are 8 mm and the boards start at 8 mm; the panel is 8 + 40 + 8 wide
    assert_eq!(plan.width, 56 * MM);
    assert_eq!(plan.origins, vec![pt(8.0, 0.0), pt(28.0, 0.0)]);
    let xs: Vec<i64> = plan.score.iter().map(|l| l.from.x).collect();
    assert_eq!(xs, vec![8 * MM, 28 * MM, 48 * MM]);
}

#[test]
fn mouse_bite_tabs_are_spread_along_the_edge_with_their_holes() {
    let plan = plan(&board(20, 10), &bare(2, 1, Joint::MouseBites)).unwrap();
    // two boards of 20 mm and a gap of 2 mm
    assert_eq!((plan.width, plan.height), (42 * MM, 10 * MM));
    assert_eq!(plan.origins, vec![pt(0.0, 0.0), pt(22.0, 0.0)]);
    // two tabs of 3 mm on an edge of 10 mm, at a quarter and three quarters: 2.5 and 7.5 mm,
    // in the middle of the gap, which is at 21 mm
    assert_eq!(plan.tabs, vec![pt(21.0, 2.5), pt(21.0, 7.5)]);
    // three holes of 0.5 mm, 0.8 mm apart, on each tab
    assert_eq!(plan.holes.len(), 6);
    assert!(plan
        .holes
        .iter()
        .all(|h| h.kind == HoleKind::MouseBite && h.diameter == 500_000));
    let ys: Vec<i64> = plan.holes.iter().map(|h| h.at.y).collect();
    assert_eq!(
        ys,
        vec![1_700_000, 2_500_000, 3_300_000, 6_700_000, 7_500_000, 8_300_000]
    );
    assert!(plan.holes.iter().all(|h| h.at.x == 21 * MM));
    // the outline of the panel, and the two facing edges, each broken in three pieces by the
    // two tabs: 0 to 1, 4 to 6 and 9 to 10 mm
    assert_eq!(plan.profile.len(), 4 + 3 + 3);
    let right_of_first: Vec<(i64, i64)> = plan
        .profile
        .iter()
        .filter(|l| l.from.x == 20 * MM && l.to.x == 20 * MM)
        .map(|l| (l.from.y, l.to.y))
        .collect();
    assert_eq!(
        right_of_first,
        vec![(0, MM), (4 * MM, 6 * MM), (9 * MM, 10 * MM)]
    );
    let left_of_second = plan
        .profile
        .iter()
        .filter(|l| l.from.x == 22 * MM && l.to.x == 22 * MM)
        .count();
    assert_eq!(left_of_second, 3);
}

#[test]
fn the_default_frame_has_tooling_holes_and_fiducials_in_its_rails() {
    let settings = PanelSettings::default();
    let plan = plan(&board(20, 10), &settings).unwrap();
    // rails of 8 mm and gaps of 2 mm around and between 2 by 2 boards of 20 by 10 mm
    assert_eq!((plan.width, plan.height), (62 * MM, 42 * MM));
    assert_eq!(
        plan.origins,
        vec![
            pt(10.0, 10.0),
            pt(32.0, 10.0),
            pt(10.0, 22.0),
            pt(32.0, 22.0)
        ]
    );
    // four tooling holes of 3 mm, 4 mm from the corners
    let tooling: Vec<Pt> = plan
        .holes
        .iter()
        .filter(|h| h.kind == HoleKind::Tooling)
        .map(|h| h.at)
        .collect();
    assert_eq!(
        tooling,
        vec![pt(4.0, 4.0), pt(58.0, 4.0), pt(58.0, 38.0), pt(4.0, 38.0)]
    );
    // three fiducials, 12 mm along the rails from the tooling holes of three corners
    let fiducials: Vec<Pt> = plan.fiducials.iter().map(|m| m.at).collect();
    assert_eq!(
        fiducials,
        vec![pt(16.0, 4.0), pt(46.0, 4.0), pt(46.0, 38.0)]
    );
    assert!(plan
        .fiducials
        .iter()
        .all(|m| m.copper_diameter == MM && m.mask_diameter == 2 * MM));
    // twelve tabs along each direction of gaps, three holes in each tab, and the tooling holes
    assert_eq!(plan.tabs.len(), 24);
    assert_eq!(plan.holes.len(), 24 * 3 + 4);
    // the first tab: the gap left of the first column is centred at 9 mm, and the first tab of
    // the row is at a quarter of the 10 mm edge, 2.5 mm above the board
    assert_eq!(plan.tabs[0], pt(9.0, 12.5));
    assert_eq!(plan.tabs[1], pt(9.0, 17.5));
    assert_eq!(plan.tabs[2], pt(9.0, 24.5));
}

#[test]
fn the_boards_of_the_matrix_carry_their_turn_and_mirror() {
    let settings = PanelSettings {
        copy_rotation_deg: 90,
        copy_mirror: true,
        ..bare(2, 1, Joint::VScore)
    };
    let plan = plan(&board(20, 10), &settings).unwrap();
    // turned, the box of every copy is 10 wide and 20 high
    assert_eq!((plan.width, plan.height), (20 * MM, 20 * MM));
    for copy in &plan.copies {
        assert_eq!((copy.width, copy.height), (10 * MM, 20 * MM));
        assert_eq!(copy.rotation_deg, 90);
        assert!(copy.mirror);
    }
}

#[test]
fn the_holes_of_the_board_follow_each_copy() {
    let shape = BoardShape {
        origin: Pt {
            x: 5 * MM,
            y: 7 * MM,
        },
        width: 20 * MM,
        height: 10 * MM,
        holes: vec![
            (true, pt(7.0, 8.0), 800_000),
            (false, pt(24.0, 16.0), 3 * MM),
        ],
    };
    let plan = plan(&shape, &bare(2, 1, Joint::VScore)).unwrap();
    let plated: Vec<Pt> = plan
        .holes
        .iter()
        .filter(|h| h.kind == HoleKind::Plated)
        .map(|h| h.at)
        .collect();
    // (7, 8) is (2, 1) from the corner of the box, and the copies start at 0 and 20 mm
    assert_eq!(plated, vec![pt(2.0, 1.0), pt(22.0, 1.0)]);
    let unplated: Vec<Pt> = plan
        .holes
        .iter()
        .filter(|h| h.kind == HoleKind::NonPlated)
        .map(|h| h.at)
        .collect();
    assert_eq!(unplated, vec![pt(19.0, 9.0), pt(39.0, 9.0)]);
}

#[test]
fn a_panel_without_a_frame_says_why_it_has_no_marks() {
    let settings = PanelSettings {
        fiducial_count: 3,
        tooling_hole_count: 4,
        ..bare(2, 1, Joint::VScore)
    };
    let plan = plan(&board(20, 10), &settings).unwrap();
    assert!(plan.fiducials.is_empty());
    assert!(plan.holes.iter().all(|h| h.kind != HoleKind::Tooling));
    assert!(plan.notes.iter().any(|n| n.code == MARKS_SKIPPED));
}

#[test]
fn a_mark_that_does_not_fit_in_its_rail_is_refused() {
    // a rail of 3 mm cannot hold a tooling hole of 3 mm centred 4 mm from the corner
    let settings = PanelSettings {
        frame_width_nm: 3 * MM,
        ..PanelSettings::default()
    };
    let error = plan(&board(20, 10), &settings).unwrap_err();
    assert_eq!(error.code, "PHOENIX-PANEL-005");
}

#[test]
fn marks_that_touch_each_other_are_refused() {
    // a fiducial only 1 mm from the tooling hole of its corner
    let settings = PanelSettings {
        fiducial_step_nm: MM,
        ..PanelSettings::default()
    };
    let error = plan(&board(20, 10), &settings).unwrap_err();
    assert_eq!(error.code, MARKS_OVERLAP);
}

#[test]
fn tabs_and_holes_that_do_not_fit_are_refused_with_their_numbers() {
    // four tabs of 3 mm on an edge of 10 mm
    let crowded = PanelSettings {
        tabs_per_edge: 4,
        ..bare(2, 1, Joint::MouseBites)
    };
    let error = plan(&board(20, 10), &crowded).unwrap_err();
    assert_eq!(error.code, TABS_DONT_FIT);
    assert_eq!(
        error.detail,
        "4 tabs of 3000000 nm do not fit on an edge of 10000000 nm"
    );
    // five holes 0.8 mm apart take 4 * 0.8 + 0.5 = 3.7 mm, more than a tab of 3 mm
    let wide = PanelSettings {
        mouse_bite_holes: 5,
        ..bare(2, 1, Joint::MouseBites)
    };
    let error = plan(&board(20, 10), &wide).unwrap_err();
    assert_eq!(error.code, BITES_DONT_FIT);
    assert_eq!(
        error.detail,
        "5 holes take 3700000 nm and the tab is 3000000 nm wide"
    );
}

#[test]
fn a_panel_beyond_its_limits_is_refused() {
    let many = PanelSettings {
        max_boards: 5,
        ..bare(3, 2, Joint::VScore)
    };
    let error = plan(&board(20, 10), &many).unwrap_err();
    assert_eq!(error.code, PANEL_TOO_LARGE);
    assert_eq!(error.detail, "6 boards, the limit is 5");
    let large = PanelSettings {
        max_panel_nm: 50 * MM,
        ..bare(3, 1, Joint::VScore)
    };
    let error = plan(&board(20, 10), &large).unwrap_err();
    assert_eq!(error.code, PANEL_TOO_LARGE);
    assert_eq!(
        error.detail,
        "60000000 nm by 10000000 nm, the limit is 50000000 nm"
    );
}

// ---------------------------------------------------------------------------------------------
// the settings text

#[test]
fn a_settings_text_gives_its_values_and_keeps_the_defaults_for_the_rest() {
    let text = "# a panel\ncolumns = 3\n\nrows = 2\njoint = v_score\nframe = top_bottom\n\
        copy_suffix_separator = -\nplace = 1, 1000, 2000, 90, true\n";
    let s = PanelSettings::parse(text).unwrap();
    assert_eq!((s.columns, s.rows), (3, 2));
    assert_eq!(s.joint, Joint::VScore);
    assert_eq!(s.frame, Frame::TopBottom);
    assert_eq!(s.copy_suffix_separator, "-");
    assert_eq!(
        s.placements,
        vec![Placement {
            board: 1,
            x_nm: 1000,
            y_nm: 2000,
            rotation_deg: 90,
            mirror: true
        }]
    );
    // not named in the text: the defaults
    assert_eq!(s.gap_nm, PanelSettings::default().gap_nm);
    assert_eq!(s.max_boards, 400);
}

#[test]
fn a_settings_line_that_cannot_be_used_names_its_line() {
    let error = PanelSettings::parse("columns = 2\nbogus = 1\n").unwrap_err();
    assert_eq!(error.code, SETTINGS_INVALID);
    assert_eq!(error.detail, "line 2: unknown key bogus");
    let error = PanelSettings::parse("rows = many\n").unwrap_err();
    assert_eq!(error.detail, "line 1: the value of rows is not valid");
    let error = PanelSettings::parse("rows 3\n").unwrap_err();
    assert_eq!(error.detail, "line 1: not key = value");
    let error = PanelSettings::parse("frame = diagonal\n").unwrap_err();
    assert_eq!(error.detail, "line 1: the value of frame is not valid");
    let error = PanelSettings::parse("place = 1,2,3\n").unwrap_err();
    assert_eq!(error.detail, "line 1: the value of place is not valid");
}

#[test]
fn a_value_out_of_its_range_is_refused_after_the_text_is_read() {
    for text in [
        "columns = 0\n",
        "rows = 65\n",
        "fiducial_count = 2\n",
        "tooling_hole_count = 1\n",
        "copy_rotation_deg = 45\n",
        "frame_width_nm = 0\n",
        "mouse_bite_pitch_nm = 100000\n",
        "gap_nm = 100000\n",
        "copy_suffix_separator = a b\n",
        "place = 0,0,0,45,false\n",
    ] {
        let error = PanelSettings::parse(text).unwrap_err();
        assert_eq!(error.code, SETTINGS_INVALID, "{text}");
    }
}
