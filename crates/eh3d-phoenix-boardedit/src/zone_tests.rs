// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the zones in the editor, on the small board of `testutil`: the board is 60 mm by
//! 40 mm, the signal track runs along y = 10 mm from x = 11.5 mm to x = 28.5 mm and the pads of
//! the ground net are at (8.5, 10) and (31.5, 10).

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Id, Segment};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_zones::{FilledZone, Severity as ZoneSeverity};

use crate::codes::{BoardEditError, OPERATIONS_INVALID, OPERATION_REFUSED, REJECTED_BY_DRC};
use crate::edit::BoardEdit;
use crate::ops::EditOp;
use crate::ops_file::read_operations;
use crate::settings::BoardEditSettings;
use crate::testutil::{fixture, id, strict_rules, Fixture};
use crate::zone::{export_issues, poured_board, Target, ZoneChanges, ZoneData, ZoneProps};
use crate::zone_clip::{Clipboard, CLIP_FORMAT};
use crate::zone_codes::{
    CLIPBOARD_INVALID, FILL_BREAKS_RULES, FILL_MISSING, NOT_REPRESENTABLE, PASTE_REFUSED,
    ZONE_CUTOUT, ZONE_DATA_INVALID, ZONE_NOT_FOUND, ZONE_OUTLINE, ZONE_PROPERTY,
};

fn mm(x: i64, y: i64) -> Point {
    Point::from_nm(x * 1_000_000, y * 1_000_000).unwrap()
}

fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<Point> {
    vec![mm(x0, y0), mm(x1, y0), mm(x1, y1), mm(x0, y1)]
}

fn editor(f: &Fixture) -> BoardEdit {
    BoardEdit::new(
        f.board.clone(),
        strict_rules(),
        BoardEditSettings::default(),
    )
    .unwrap()
}

fn add_op(net: Id, layer: Id, outline: Vec<Point>, props: ZoneProps) -> EditOp {
    EditOp::ZoneAdd {
        id: None,
        net,
        layer,
        outline,
        clearance_nm: None,
        props,
    }
}

fn fill_all() -> EditOp {
    EditOp::ZoneFill { zone: None }
}

/// True when the point (in millimetres) is inside or on a polygon of the fill; the polygons are
/// counter-clockwise.
fn covers(fill: &FilledZone, x_mm: i64, y_mm: i64) -> bool {
    let (px, py) = (x_mm * 1_000_000, y_mm * 1_000_000);
    fill.polygons.iter().any(|poly| {
        let n = poly.len();
        (0..n).all(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let cross = i128::from(b[0] - a[0]) * i128::from(py - a[1])
                - i128::from(b[1] - a[1]) * i128::from(px - a[0]);
            cross >= 0
        })
    })
}

fn has_error(issues: &[crate::zone::ZoneIssue]) -> bool {
    issues.iter().any(|i| i.severity == ZoneSeverity::Error)
}

fn code_of(result: Result<crate::edit::Outcome, BoardEditError>) -> &'static str {
    result.unwrap_err().code
}

#[test]
fn a_zone_is_added_removed_and_both_are_undone() {
    let f = fixture();
    let mut e = editor(&f);
    let before = e.board().to_bytes().unwrap();
    let props = ZoneProps {
        priority: 2,
        ..ZoneProps::default()
    };
    let out = e
        .apply(&add_op(f.gnd, f.top, rect(2, 2, 58, 38), props))
        .unwrap();
    assert_eq!(out.created.len(), 1);
    let z = out.created[0];
    assert_eq!(e.board().zones.len(), 1);
    assert_eq!(e.zone_data().props_of(z).priority, 2);
    assert!(e.undo());
    assert_eq!(e.board().to_bytes().unwrap(), before);
    assert!(e.zone_data().props.is_empty());
    assert!(e.redo());
    assert_eq!(e.zone_data().props_of(z).priority, 2);
    e.apply(&EditOp::ZoneRemove { zone: z }).unwrap();
    assert!(e.board().zones.is_empty());
    assert!(e.zone_data().props.is_empty());
    assert!(e.undo());
    assert_eq!(e.board().zones.len(), 1);
    assert_eq!(e.zone_data().props_of(z).priority, 2);
}

#[test]
fn the_same_operations_give_the_same_bytes_and_the_same_data() {
    let f = fixture();
    let run = || {
        let mut e = editor(&f);
        e.apply(&add_op(
            f.gnd,
            f.top,
            rect(2, 2, 58, 38),
            ZoneProps::default(),
        ))
        .unwrap();
        e.apply(&fill_all()).unwrap();
        (e.board().to_bytes().unwrap(), e.zone_data().to_json())
    };
    assert_eq!(run(), run());
}

#[test]
fn what_cannot_be_a_zone_is_refused_with_its_code_and_nothing_changes() {
    let f = fixture();
    let mut e = editor(&f);
    let plain = ZoneProps::default();
    let bowtie = vec![mm(0, 0), mm(10, 10), mm(10, 0), mm(0, 10)];
    let two = vec![mm(0, 0), mm(5, 5)];
    let line = vec![mm(0, 0), mm(5, 5), mm(10, 10)];
    let bad_spokes = ZoneProps {
        thermal_spokes: Some(3),
        ..ZoneProps::default()
    };
    let negative = ZoneProps {
        min_width_nm: Some(-1),
        ..ZoneProps::default()
    };
    let cases = [
        (add_op(f.gnd, f.top, two, plain.clone()), ZONE_OUTLINE),
        (add_op(f.gnd, f.top, bowtie, plain.clone()), ZONE_OUTLINE),
        (add_op(f.gnd, f.top, line, plain.clone()), ZONE_OUTLINE),
        (
            add_op(id(999), f.top, rect(2, 2, 9, 9), plain.clone()),
            OPERATION_REFUSED,
        ),
        (
            add_op(f.gnd, id(3), rect(2, 2, 9, 9), plain.clone()),
            OPERATION_REFUSED,
        ),
        (
            add_op(f.gnd, f.top, rect(2, 2, 9, 9), bad_spokes),
            ZONE_PROPERTY,
        ),
        (
            add_op(f.gnd, f.top, rect(2, 2, 9, 9), negative),
            ZONE_PROPERTY,
        ),
        (EditOp::ZoneRemove { zone: id(777) }, ZONE_NOT_FOUND),
        (
            EditOp::ZonePriority {
                zone: id(777),
                priority: 1,
            },
            ZONE_NOT_FOUND,
        ),
        (
            EditOp::ZoneCutout {
                zone: id(777),
                polygon: rect(1, 1, 2, 2),
            },
            ZONE_NOT_FOUND,
        ),
        (
            EditOp::ZoneFill {
                zone: Some(id(777)),
            },
            ZONE_NOT_FOUND,
        ),
    ];
    for (op, code) in cases {
        assert_eq!(code_of(e.apply(&op)), code, "{op:?}");
    }
    assert_eq!(e.steps(), 0);
    assert!(e.board().zones.is_empty());
}

#[test]
fn the_fill_leaves_the_track_alone_covers_the_rest_and_proves_no_short() {
    let f = fixture();
    let mut e = editor(&f);
    let z = e
        .apply(&add_op(
            f.gnd,
            f.top,
            rect(2, 2, 58, 38),
            ZoneProps::default(),
        ))
        .unwrap()
        .created[0];
    // before the fill the zone is reported as having none
    let issues = e.zone_issues();
    assert!(issues
        .iter()
        .any(|i| i.code == FILL_MISSING && i.zone == Some(z)));
    let out = e.apply(&fill_all()).unwrap();
    assert!(out.notes.iter().any(|n| n.contains("polygons")), "{out:?}");
    let fill = e.zone_data().fill_of(z).unwrap().clone();
    assert!(!fill.polygons.is_empty());
    assert!(covers(&fill, 20, 30));
    assert!(!covers(&fill, 20, 10), "the signal track is under the fill");
    assert!(!has_error(&e.zone_issues()));
    // the fill is data of the editor: it goes with undo and redo
    assert!(e.undo());
    assert!(e.zone_data().fills.is_empty());
    assert!(e.redo());
    assert_eq!(e.zone_data().fill_of(z), Some(&fill));
    assert_eq!(e.steps(), 2);
}

#[test]
fn copper_added_over_a_fill_is_found_by_the_report() {
    let f = fixture();
    let mut e = editor(&f);
    e.apply(&add_op(
        f.gnd,
        f.top,
        rect(2, 2, 58, 38),
        ZoneProps::default(),
    ))
    .unwrap();
    e.apply(&fill_all()).unwrap();
    assert!(!has_error(&e.zone_issues()));
    e.apply(&EditOp::Via {
        net: f.sig,
        at: mm(20, 30),
        from_layer: None,
        to_layer: None,
    })
    .unwrap();
    let issues = e.zone_issues();
    assert!(issues
        .iter()
        .any(|i| i.code == FILL_BREAKS_RULES && i.severity == ZoneSeverity::Error));
    // a new fill makes room for the via
    e.apply(&fill_all()).unwrap();
    assert!(!has_error(&e.zone_issues()));
}

#[test]
fn the_zone_of_the_higher_priority_wins_and_the_other_keeps_away() {
    let f = fixture();
    let mut e = editor(&f);
    let gnd = e
        .apply(&add_op(
            f.gnd,
            f.top,
            rect(2, 2, 58, 38),
            ZoneProps::default(),
        ))
        .unwrap()
        .created[0];
    // the signal zone lies inside the ground zone: the rules would call it an overlap, but the
    // priorities differ, so it is what the person asked for
    let sig = e
        .apply(&add_op(
            f.sig,
            f.top,
            rect(10, 6, 30, 14),
            ZoneProps {
                priority: 1,
                ..ZoneProps::default()
            },
        ))
        .unwrap()
        .created[0];
    e.apply(&fill_all()).unwrap();
    let (g, s) = (
        e.zone_data().fill_of(gnd).unwrap().clone(),
        e.zone_data().fill_of(sig).unwrap().clone(),
    );
    assert!(covers(&s, 20, 7));
    assert!(!covers(&g, 20, 7));
    assert!(covers(&g, 20, 30));
    // the ground zone takes the first place: the signal zone is swallowed
    let out = e
        .apply(&EditOp::ZonePriority {
            zone: gnd,
            priority: 5,
        })
        .unwrap();
    assert!(out.changed);
    assert!(e.zone_data().fill_of(gnd).is_none());
    let out = e.apply(&fill_all()).unwrap();
    assert!(out.notes.iter().any(|n| n.starts_with("PHOENIX-BZONE-006")));
    let (g, s) = (
        e.zone_data().fill_of(gnd).unwrap().clone(),
        e.zone_data().fill_of(sig).unwrap().clone(),
    );
    assert!(covers(&g, 20, 7));
    assert!(s.polygons.is_empty());
    assert_eq!(e.zone_data().fill_order(e.board())[0], gnd);
}

#[test]
fn two_zones_of_one_priority_that_overlap_are_still_a_rule_error() {
    let f = fixture();
    let mut e = editor(&f);
    e.apply(&add_op(
        f.gnd,
        f.top,
        rect(2, 2, 58, 38),
        ZoneProps::default(),
    ))
    .unwrap();
    let again = add_op(f.sig, f.top, rect(10, 6, 30, 14), ZoneProps::default());
    assert_eq!(code_of(e.apply(&again)), REJECTED_BY_DRC);
    assert_eq!(e.board().zones.len(), 1);
}

#[test]
fn a_cutout_keeps_the_fill_away_and_is_checked_against_the_outline() {
    let f = fixture();
    let mut e = editor(&f);
    let z = e
        .apply(&add_op(
            f.gnd,
            f.top,
            rect(2, 2, 58, 38),
            ZoneProps::default(),
        ))
        .unwrap()
        .created[0];
    let outside = EditOp::ZoneCutout {
        zone: z,
        polygon: rect(55, 35, 70, 45),
    };
    let flat = EditOp::ZoneCutout {
        zone: z,
        polygon: vec![mm(40, 25), mm(45, 25), mm(50, 25)],
    };
    assert_eq!(code_of(e.apply(&outside)), ZONE_CUTOUT);
    assert_eq!(code_of(e.apply(&flat)), ZONE_CUTOUT);
    e.apply(&EditOp::ZoneCutout {
        zone: z,
        polygon: rect(40, 25, 50, 32),
    })
    .unwrap();
    assert_eq!(e.zone_data().props_of(z).cutouts.len(), 1);
    e.apply(&fill_all()).unwrap();
    let fill = e.zone_data().fill_of(z).unwrap().clone();
    assert!(!covers(&fill, 45, 28));
    assert!(covers(&fill, 38, 28));
    assert!(covers(&fill, 20, 30));
    // a cutout that is taken out gives the copper back after a new fill, and the fill is dropped
    let none = EditOp::ZoneEdit {
        zone: z,
        net: None,
        layer: None,
        outline: None,
        clearance_nm: None,
        props: ZoneChanges {
            remove_cutout: Some(3),
            ..ZoneChanges::default()
        },
    };
    assert_eq!(code_of(e.apply(&none)), ZONE_CUTOUT);
    let take = EditOp::ZoneEdit {
        zone: z,
        net: None,
        layer: None,
        outline: None,
        clearance_nm: None,
        props: ZoneChanges {
            remove_cutout: Some(0),
            ..ZoneChanges::default()
        },
    };
    let out = e.apply(&take).unwrap();
    assert!(out.notes.iter().any(|n| n.starts_with("PHOENIX-BZONE-007")));
    assert!(e.zone_data().fill_of(z).is_none());
    e.apply(&fill_all()).unwrap();
    assert!(covers(e.zone_data().fill_of(z).unwrap(), 45, 28));
}

#[test]
fn a_zone_is_edited_in_its_properties_and_its_outline() {
    let f = fixture();
    let mut e = editor(&f);
    let z = e
        .apply(&add_op(
            f.gnd,
            f.top,
            rect(2, 2, 58, 38),
            ZoneProps::default(),
        ))
        .unwrap()
        .created[0];
    e.apply(&fill_all()).unwrap();
    let before = e.board().to_bytes().unwrap();
    let edit = EditOp::ZoneEdit {
        zone: z,
        net: None,
        layer: None,
        outline: Some(rect(4, 4, 50, 30)),
        clearance_nm: Some(300_000),
        props: ZoneChanges {
            thermal_spokes: Some(2),
            priority: Some(3),
            ..ZoneChanges::default()
        },
    };
    let out = e.apply(&edit).unwrap();
    assert!(out.changed);
    assert!(out.notes.iter().any(|n| n.starts_with("PHOENIX-BZONE-007")));
    assert_eq!(e.board().zones[&z].outline, rect(4, 4, 50, 30));
    assert_eq!(e.board().zones[&z].clearance.get(), 300_000);
    let props = e.zone_data().props_of(z);
    assert_eq!((props.priority, props.thermal_spokes), (3, Some(2)));
    assert!(e.zone_data().fill_of(z).is_none());
    // the outline must stay a polygon, and the cutouts must stay inside it
    let bad = EditOp::ZoneEdit {
        zone: z,
        net: None,
        layer: None,
        outline: Some(vec![mm(0, 0), mm(10, 10)]),
        clearance_nm: None,
        props: ZoneChanges::default(),
    };
    assert_eq!(code_of(e.apply(&bad)), ZONE_OUTLINE);
    assert!(e.undo());
    assert_eq!(e.board().to_bytes().unwrap(), before);
    assert!(e.zone_data().fill_of(z).is_some());
}

#[test]
fn the_thermal_relief_is_set_for_each_zone() {
    let f = fixture();
    let mut e = editor(&f);
    let wide = ZoneProps {
        thermal_spoke_width_nm: Some(10_000_000),
        ..ZoneProps::default()
    };
    e.apply(&add_op(f.gnd, f.top, rect(2, 2, 58, 38), wide))
        .unwrap();
    let out = e.apply(&fill_all()).unwrap();
    assert!(
        out.notes.iter().any(|n| n.starts_with("PHOENIX-ZONE-011")),
        "{:?}",
        out.notes
    );
    let mut e = editor(&f);
    let two = ZoneProps {
        thermal_spokes: Some(2),
        thermal_gap_nm: Some(300_000),
        ..ZoneProps::default()
    };
    e.apply(&add_op(f.gnd, f.top, rect(2, 2, 58, 38), two))
        .unwrap();
    let out = e.apply(&fill_all()).unwrap();
    assert!(!out.notes.iter().any(|n| n.contains("PHOENIX-ZONE-011")));
}

#[test]
fn a_group_of_zone_operations_is_one_step() {
    let f = fixture();
    let mut e = editor(&f);
    let group = EditOp::Group {
        ops: vec![
            add_op(f.gnd, f.top, rect(2, 2, 58, 38), ZoneProps::default()),
            fill_all(),
        ],
    };
    e.apply(&group).unwrap();
    assert_eq!(e.steps(), 1);
    assert_eq!(e.zone_data().fills.len(), 1);
    assert!(e.undo());
    assert!(e.board().zones.is_empty());
    assert_eq!(*e.zone_data(), ZoneData::default());
    assert!(!e.can_undo());
    assert!(e.redo());
    assert_eq!(e.zone_data().fills.len(), 1);
}

#[test]
fn a_net_is_pasted_with_an_offset_a_turn_and_a_mirror() {
    let f = fixture();
    let paste = |at: Point, quarter_turns: u8, mirror: bool| EditOp::PasteGroup {
        clipboard: None,
        from_net: Some(f.sig),
        at,
        quarter_turns,
        mirror,
        net_map: BTreeMap::new(),
    };
    let mut e = editor(&f);
    let before = e.board().to_bytes().unwrap();
    let out = e.apply(&paste(mm(11, 20), 0, false)).unwrap();
    assert_eq!(e.board().tracks.len(), 2);
    let t = &e.board().tracks[&out.created[0]];
    assert_eq!(t.net, f.sig);
    assert_eq!(
        t.path,
        vec![Segment::Line {
            start: mm(11, 20),
            end: Point::from_nm(28_000_000, 20_000_000).unwrap(),
        }]
    );
    assert!(e.undo());
    assert_eq!(e.board().to_bytes().unwrap(), before);
    // a quarter turn: the piece of 17 mm along x goes along y
    let out = e.apply(&paste(mm(30, 20), 1, false)).unwrap();
    assert_eq!(
        e.board().tracks[&out.created[0]].path,
        vec![Segment::Line {
            start: mm(30, 20),
            end: mm(30, 37)
        }]
    );
    // a mirror about the vertical axis of the reference point
    let mut e = editor(&f);
    let out = e.apply(&paste(mm(50, 20), 0, true)).unwrap();
    assert_eq!(
        e.board().tracks[&out.created[0]].path,
        vec![Segment::Line {
            start: mm(50, 20),
            end: mm(33, 20)
        }]
    );
    // the same paste on the same board always gives the same ids
    let mut again = editor(&f);
    let second = again.apply(&paste(mm(50, 20), 0, true)).unwrap();
    assert_eq!(second.created, out.created);
    assert_eq!(
        again.board().to_bytes().unwrap(),
        e.board().to_bytes().unwrap()
    );
}

#[test]
fn a_paste_that_adds_a_rule_error_is_refused_even_when_rejection_is_off() {
    let f = fixture();
    let mut e = editor(&f);
    e.settings.reject_new_errors = false;
    let mut map = BTreeMap::new();
    map.insert("SIG".to_owned(), "GND".to_owned());
    let on_top = EditOp::PasteGroup {
        clipboard: None,
        from_net: Some(f.sig),
        // the copy of the signal track, made a ground track, lies on the signal track
        at: Point::from_nm(11_500_000, 10_000_000).unwrap(),
        quarter_turns: 0,
        mirror: false,
        net_map: map,
    };
    assert_eq!(code_of(e.apply(&on_top)), PASTE_REFUSED);
    assert_eq!(e.board().tracks.len(), 1);
    let forced = EditOp::Forced {
        with: Box::new(on_top),
    };
    let out = e.apply(&forced).unwrap();
    assert!(out.forced);
    assert_eq!(e.board().tracks.len(), 2);
}

#[test]
fn a_clipboard_goes_to_another_board_by_the_names_of_nets_and_layers() {
    let f = fixture();
    let e = editor(&f);
    let clip = Clipboard::copy_net(e.board(), e.zone_data(), f.sig).unwrap();
    assert_eq!(clip.format, CLIP_FORMAT);
    assert_eq!(clip.tracks.len(), 1);
    assert_eq!(Clipboard::from_json(&clip.to_json()).unwrap(), clip);
    let mut map = BTreeMap::new();
    map.insert("SIG".to_owned(), "NEW".to_owned());
    let paste = |clipboard: Clipboard, quarter_turns: u8| EditOp::PasteGroup {
        clipboard: Some(clipboard),
        from_net: None,
        at: mm(11, 20),
        quarter_turns,
        mirror: false,
        net_map: map.clone(),
    };
    let mut other = editor(&f);
    let out = other.apply(&paste(clip.clone(), 0)).unwrap();
    assert_eq!(out.created.len(), 1);
    assert_eq!(other.board().nets.len(), 3);
    assert!(other.board().nets.values().any(|n| n.name == "NEW"));
    // what a clipboard cannot be pasted
    let mut no_layer = clip.clone();
    no_layer.tracks[0].layer = "Nope".to_owned();
    assert_eq!(
        code_of(editor(&f).apply(&paste(no_layer, 0))),
        PASTE_REFUSED
    );
    let mut wrong = clip.clone();
    wrong.format = "other/1".to_owned();
    assert_eq!(
        code_of(editor(&f).apply(&paste(wrong, 0))),
        CLIPBOARD_INVALID
    );
    assert_eq!(
        code_of(editor(&f).apply(&paste(clip.clone(), 4))),
        PASTE_REFUSED
    );
    let neither = EditOp::PasteGroup {
        clipboard: None,
        from_net: None,
        at: mm(0, 0),
        quarter_turns: 0,
        mirror: false,
        net_map: BTreeMap::new(),
    };
    assert_eq!(code_of(editor(&f).apply(&neither)), PASTE_REFUSED);
    assert!(Clipboard::from_json("{\"format\":\"phoenix-clipboard/1\",\"extra\":1}").is_err());
}

#[test]
fn a_zone_is_pasted_with_its_properties_and_its_cutouts() {
    let f = fixture();
    let mut e = editor(&f);
    let props = ZoneProps {
        priority: 4,
        thermal_spokes: Some(2),
        cutouts: vec![rect(40, 25, 50, 32)],
        ..ZoneProps::default()
    };
    let z = e
        .apply(&add_op(f.gnd, f.top, rect(2, 2, 58, 38), props.clone()))
        .unwrap()
        .created[0];
    e.apply(&fill_all()).unwrap();
    let out = e
        .apply(&EditOp::PasteGroup {
            clipboard: None,
            from_net: Some(f.gnd),
            at: mm(2, 2),
            quarter_turns: 0,
            mirror: false,
            net_map: BTreeMap::new(),
        })
        .unwrap();
    assert_eq!(e.board().zones.len(), 2);
    let copy = out.created[0];
    assert_ne!(copy, z);
    assert_eq!(e.zone_data().props_of(copy), props);
    // the copy has no fill yet
    assert!(e.zone_data().fill_of(copy).is_none());
    assert!(e.zone_data().fill_of(z).is_some());
}

#[test]
fn the_data_of_zones_is_written_and_read_back_and_bad_files_are_refused() {
    let f = fixture();
    let mut e = editor(&f);
    e.apply(&add_op(
        f.gnd,
        f.top,
        rect(2, 2, 58, 38),
        ZoneProps {
            priority: 1,
            cutouts: vec![rect(40, 25, 50, 32)],
            ..ZoneProps::default()
        },
    ))
    .unwrap();
    e.apply(&fill_all()).unwrap();
    let text = e.zone_data().to_json();
    assert_eq!(&ZoneData::from_json(&text).unwrap(), e.zone_data());
    for bad in [
        "",
        "{}",
        "{\"format\":\"other/1\"}",
        "{\"format\":\"phoenix-zone-data/1\",\"extra\":1}",
    ] {
        assert_eq!(
            ZoneData::from_json(bad).unwrap_err().code,
            ZONE_DATA_INVALID,
            "{bad}"
        );
    }
    // a number of spokes the fill cannot make is a bad property, not a bad file
    let spokes = "{\"format\":\"phoenix-zone-data/1\",\"props\":{\"01890a5d-ac96-7ab2-80e2-4536629c1301\":{\"thermal_spokes\":3}}}";
    assert_eq!(
        ZoneData::from_json(spokes).unwrap_err().code,
        crate::zone_codes::ZONE_PROPERTY
    );
}

#[test]
fn an_editor_can_start_from_the_data_that_was_kept() {
    let f = fixture();
    let mut first = editor(&f);
    let z = first
        .apply(&add_op(
            f.gnd,
            f.top,
            rect(2, 2, 58, 38),
            ZoneProps::default(),
        ))
        .unwrap()
        .created[0];
    first.apply(&fill_all()).unwrap();
    let board = first.board().clone();
    let data = first.zone_data().clone();
    let again = BoardEdit::new(board, strict_rules(), BoardEditSettings::default())
        .unwrap()
        .with_zone_data(data.clone());
    assert_eq!(again.zone_data(), &data);
    assert!(again.zone_data().fill_of(z).is_some());
    assert!(!again.can_undo());
}

#[test]
fn the_poured_board_has_one_zone_for_each_polygon_and_keeps_zones_with_no_fill() {
    let f = fixture();
    let mut e = editor(&f);
    let z = e
        .apply(&add_op(
            f.gnd,
            f.top,
            rect(2, 2, 58, 38),
            ZoneProps::default(),
        ))
        .unwrap()
        .created[0];
    let (plain, issues) = poured_board(e.board(), e.zone_data()).unwrap();
    assert_eq!(plain.zones.len(), 1);
    assert!(issues.iter().any(|i| i.code == FILL_MISSING));
    e.apply(&fill_all()).unwrap();
    let polygons = e.zone_data().fill_of(z).unwrap().polygons.clone();
    let (poured, issues) = poured_board(e.board(), e.zone_data()).unwrap();
    assert!(issues.is_empty());
    assert_eq!(poured.zones.len(), polygons.len());
    assert!(!poured.zones.contains_key(&z));
    for zone in poured.zones.values() {
        assert_eq!((zone.net, zone.layer), (f.gnd, f.top));
        let corners: Vec<[i64; 2]> = zone
            .outline
            .iter()
            .map(|p| [p.x_nm.get(), p.y_nm.get()])
            .collect();
        assert!(polygons.contains(&corners));
    }
    let (again, _) = poured_board(e.board(), e.zone_data()).unwrap();
    assert_eq!(again.to_bytes().unwrap(), poured.to_bytes().unwrap());
    // the board itself is as it was
    assert!(e.board().zones.contains_key(&z));
}

#[test]
fn what_a_format_cannot_say_about_zones_is_named_never_dropped() {
    let f = fixture();
    let mut e = editor(&f);
    let z = e
        .apply(&add_op(
            f.gnd,
            f.top,
            rect(2, 2, 58, 38),
            ZoneProps {
                priority: 2,
                cutouts: vec![rect(40, 25, 50, 32)],
                thermal_spokes: Some(2),
                ..ZoneProps::default()
            },
        ))
        .unwrap()
        .created[0];
    let gerber = export_issues(e.board(), e.zone_data(), Target::Gerber);
    assert!(gerber
        .iter()
        .any(|i| i.code == NOT_REPRESENTABLE && i.zone == Some(z)));
    e.apply(&fill_all()).unwrap();
    assert!(export_issues(e.board(), e.zone_data(), Target::Gerber).is_empty());
    let dsn = export_issues(e.board(), e.zone_data(), Target::Dsn);
    assert_eq!(dsn.len(), 1);
    let kicad = export_issues(e.board(), e.zone_data(), Target::Kicad);
    let lines: Vec<String> = kicad.iter().map(|i| i.line(Lang::En)).collect();
    assert!(lines.iter().any(|l| l.contains("priority")), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("cutouts")), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("thermal")), "{lines:?}");
    assert!(
        lines.iter().any(|l| l.contains("filled copper")),
        "{lines:?}"
    );
}

#[test]
fn the_operations_are_read_from_a_file_with_their_defaults() {
    let f = fixture();
    let text = format!(
        r#"[{{"op":"zone_add","net":"{}","layer":"{}","outline":[{{"x_nm":0,"y_nm":0}},{{"x_nm":5000000,"y_nm":0}},{{"x_nm":5000000,"y_nm":5000000}}],"props":{{"priority":2,"connection":"solid"}}}},
            {{"op":"zone_fill"}},
            {{"op":"zone_priority","zone":"{}","priority":3}},
            {{"op":"paste_group","from_net":"{}","at":{{"x_nm":0,"y_nm":0}}}}]"#,
        f.gnd, f.top, f.sig, f.sig
    );
    let ops = read_operations(&text).unwrap();
    assert_eq!(ops.len(), 4);
    match &ops[0] {
        EditOp::ZoneAdd { props, id, .. } => {
            assert_eq!(props.priority, 2);
            assert_eq!(*id, None);
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(&ops[1], EditOp::ZoneFill { zone: None }));
    match &ops[3] {
        EditOp::PasteGroup {
            quarter_turns,
            mirror,
            net_map,
            ..
        } => assert_eq!((*quarter_turns, *mirror, net_map.len()), (0, false, 0)),
        other => panic!("{other:?}"),
    }
    let unknown = r#"[{"op":"zone_fly"}]"#;
    assert_eq!(
        read_operations(unknown).unwrap_err().code,
        OPERATIONS_INVALID
    );
}
