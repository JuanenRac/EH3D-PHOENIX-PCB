// SPDX-License-Identifier: GPL-3.0-or-later
//! The router, the ratsnest and the editor on boards made by hand: two pads at 45 degrees, an
//! obstacle in the middle, a way that does not exist, a change of layer, edits that are undone byte
//! for byte, and a property over seeded random boards: a track that the router returns always
//! passes the rules.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Operation, Pad, PadNet, PadRef,
    PadShape, Segment, Side, Track,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
use eh3d_phoenix_route::codes::{
    ANGLE_BROKEN, EDIT_INVALID, EDIT_REJECTED, ELEMENT_NOT_FOUND, LAYER_UNUSABLE, LIMIT_EXCEEDED,
    NET_MISMATCH, NO_PATH, SAME_PLACE, STOPPED,
};
use eh3d_phoenix_route::{
    add_via, copper_layers, delete_track, diff, findings, move_component, move_vertex, ratsnest,
    rotate_component, route, set_track_width, via_for_layer_change, Editor, Element, Mode, RatLine,
    RouteRequest, RouteSettings, RuleContext,
};
use eh3d_phoenix_rules::{
    analyse_board, check_board, codes, NetClass, NetClasses, RuleValues, Severity,
};

const TOP: u32 = 1;
const BOT: u32 = 2;
const FOOTPRINT: u32 = 40;
const COMPONENT: u32 = 43;
const PAD0: u32 = 100;

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}
fn nm(v: i64) -> Nm {
    Nm::new(v).unwrap()
}
fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}
fn line(a: (i64, i64), b: (i64, i64)) -> Segment {
    Segment::Line {
        start: pt(a.0, a.1),
        end: pt(b.0, b.1),
    }
}

/// An empty board of 20 mm by 20 mm with one or two copper layers.
fn base(two_layers: bool) -> Board {
    let mut b = Board::default();
    let mut layers = vec![(TOP, "Top")];
    if two_layers {
        layers.push((BOT, "Bottom"));
    }
    for (n, name) in &layers {
        b.layers.insert(
            id(*n),
            Layer {
                name: (*name).into(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.copper_stack.push(id(*n));
    }
    b.outline = vec![
        pt(0, 0),
        pt(20_000_000, 0),
        pt(20_000_000, 20_000_000),
        pt(0, 20_000_000),
    ];
    b
}

fn add_net(b: &mut Board, n: u32, name: &str) -> Id {
    b.nets.insert(id(n), Net { name: name.into() });
    id(n)
}

/// One pad of the test component: its place, the layers it is on, its net and its diameter.
struct PadSpec {
    at: (i64, i64),
    layers: Vec<u32>,
    net: Option<Id>,
    diameter: i64,
}

fn spec(at: (i64, i64), layers: &[u32], net: Option<Id>) -> PadSpec {
    PadSpec {
        at,
        layers: layers.to_vec(),
        net,
        diameter: 600_000,
    }
}

/// Puts one component U1 with these pads on the board; the pad `i` is `pad_ref(i)`.
fn add_pads(b: &mut Board, pads: &[PadSpec]) {
    let mut map = BTreeMap::new();
    let mut pad_nets = Vec::new();
    for (i, p) in pads.iter().enumerate() {
        let pid = id(PAD0 + i as u32);
        map.insert(
            pid,
            Pad {
                number: (i + 1).to_string(),
                shape: PadShape::Circle {
                    diameter: nm(p.diameter),
                },
                at: pt(p.at.0, p.at.1),
                rotation: Udeg::ZERO,
                drill: None,
                layers: p.layers.iter().map(|l| id(*l)).collect(),
                roundness: 0,
                plated: true,
            },
        );
        if let Some(net) = p.net {
            pad_nets.push(PadNet {
                component: id(COMPONENT),
                pad: pid,
                net,
            });
        }
    }
    pad_nets.sort_by_key(|p| p.pad_ref());
    b.footprints.insert(
        id(FOOTPRINT),
        Footprint {
            name: "F".into(),
            pads: map,
        },
    );
    b.components.insert(
        id(COMPONENT),
        Component {
            refdes: "U1".into(),
            value: "x".into(),
            symbol: None,
            footprint: id(FOOTPRINT),
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    b.pad_nets = pad_nets;
}

fn pad_ref(i: u32) -> PadRef {
    PadRef {
        component: id(COMPONENT),
        pad: id(PAD0 + i),
    }
}

fn add_track(b: &mut Board, n: u32, net: Id, layer: u32, width: i64, path: Vec<Segment>) {
    b.tracks.insert(
        id(n),
        Track {
            net,
            layer: id(layer),
            width: nm(width),
            path,
        },
    );
}

fn ctx() -> RuleContext {
    RuleContext {
        rules: RuleValues {
            min_track_width: nm(150_000),
            min_clearance: nm(200_000),
            min_drill: nm(300_000),
            min_via_diameter: nm(500_000),
            min_annular_ring: nm(125_000),
            ..RuleValues::default()
        },
        classes: NetClasses::default(),
    }
}

fn request(from: u32, to: u32, mode: Mode) -> RouteRequest {
    RouteRequest {
        from: Element::Pad(pad_ref(from)),
        to: Element::Pad(pad_ref(to)),
        layer: None,
        mode,
    }
}

fn run(
    b: &Board,
    req: &RouteRequest,
) -> Result<eh3d_phoenix_route::RoutePlan, eh3d_phoenix_route::RouteError> {
    let mut n = 500_u32;
    let mut new_id = || {
        n += 1;
        id(n)
    };
    route(b, &ctx(), &RouteSettings::default(), req, &mut new_id)
}

/// Two pads of net A at 45 degrees on a two-layer board; `extra` more pads follow.
fn diagonal_board(extra: &[PadSpec]) -> (Board, Id) {
    let mut b = base(true);
    let a = add_net(&mut b, 10, "A");
    add_net(&mut b, 11, "B");
    let mut pads = vec![
        spec((2_000_000, 2_000_000), &[TOP], Some(a)),
        spec((6_000_000, 6_000_000), &[TOP], Some(a)),
    ];
    for p in extra {
        pads.push(PadSpec {
            at: p.at,
            layers: p.layers.clone(),
            net: p.net,
            diameter: p.diameter,
        });
    }
    add_pads(&mut b, &pads);
    (b, a)
}

fn passes_the_rules(b: &Board) -> bool {
    let c = ctx();
    let clean = check_board(b, &c.rules).passes();
    let no_short = analyse_board(b, c.rules.connect_tolerance)
        .confirmed
        .iter()
        .all(|f| f.code != codes::COPPER_SHORT);
    clean && no_short
}

#[test]
fn two_pads_at_45_degrees_get_one_straight_piece() {
    let (b, a) = diagonal_board(&[]);
    let plan = run(&b, &request(0, 1, Mode::Around)).unwrap();
    assert!(plan.complete && plan.notes.is_empty());
    assert_eq!((plan.tracks.len(), plan.vias.len()), (1, 0));
    let track = &plan.tracks[0].1;
    assert_eq!(track.net, a);
    assert_eq!(track.layer, id(TOP));
    assert_eq!(track.width, nm(250_000));
    assert_eq!(
        track.path,
        vec![line((2_000_000, 2_000_000), (6_000_000, 6_000_000))]
    );
    assert_eq!((plan.corners, plan.length_nm), (0, 5_656_855));
    let mut after = b.clone();
    after.apply(&plan.operation).unwrap();
    assert!(passes_the_rules(&after));
    assert!(!plan.diff.adds_error());
    assert!(after.connectivity().is_fully_connected());
}

#[test]
fn the_same_board_and_request_give_the_same_plan() {
    let (b, _) = diagonal_board(&[spec((4_000_000, 4_000_000), &[TOP], None)]);
    let first = run(&b, &request(0, 1, Mode::Around)).unwrap();
    let second = run(&b, &request(0, 1, Mode::Around)).unwrap();
    assert_eq!(first, second);
    let stop_a = run(&b, &request(0, 1, Mode::Stop)).unwrap();
    let stop_b = run(&b, &request(0, 1, Mode::Stop)).unwrap();
    assert_eq!(stop_a, stop_b);
}

#[test]
fn the_mode_that_stops_ends_in_front_of_an_obstacle() {
    let (b, _) = diagonal_board(&[spec((4_000_000, 4_000_000), &[TOP], Some(id(11)))]);
    let plan = run(&b, &request(0, 1, Mode::Stop)).unwrap();
    assert!(!plan.complete);
    assert_eq!(plan.notes, vec![STOPPED]);
    assert_eq!(plan.tracks.len(), 1);
    // the pad has a radius of 300 um, the track a half width of 125 um and the clearance with its
    // margin is 201 um: the end is the last point of the diagonal that is 626 um from the pad
    let path = &plan.tracks[0].1.path;
    assert_eq!(
        path,
        &vec![line((2_000_000, 2_000_000), (3_557_351, 3_557_351))]
    );
    assert!(
        plan.length_nm > 2_202_000 && plan.length_nm < 2_203_000,
        "{}",
        plan.length_nm
    );
    let mut after = b.clone();
    after.apply(&plan.operation).unwrap();
    assert!(passes_the_rules(&after));
}

#[test]
fn the_mode_that_goes_around_finds_a_way_that_passes_the_rules() {
    let (b, _) = diagonal_board(&[spec((4_000_000, 4_000_000), &[TOP], Some(id(11)))]);
    let plan = run(&b, &request(0, 1, Mode::Around)).unwrap();
    assert!(plan.complete);
    assert!(plan.vias.is_empty());
    assert!(plan.corners >= 2, "{}", plan.corners);
    assert!(plan.length_nm > 5_656_855);
    // every piece of the track runs at 0, 45 or 90 degrees
    for (_, track) in &plan.tracks {
        for segment in &track.path {
            let (s, e) = (segment.start(), segment.end());
            let dx = (e.x_nm.get() - s.x_nm.get()).abs();
            let dy = (e.y_nm.get() - s.y_nm.get()).abs();
            assert!(dx == 0 || dy == 0 || dx == dy, "{dx} {dy}");
        }
    }
    let mut after = b.clone();
    after.apply(&plan.operation).unwrap();
    assert!(passes_the_rules(&after));
    assert!(after.connectivity().is_fully_connected());
}

#[test]
fn a_wall_with_no_gap_gives_a_code_and_leaves_the_board_alone() {
    let mut b = base(false);
    let a = add_net(&mut b, 10, "A");
    let w = add_net(&mut b, 11, "WALL");
    add_pads(
        &mut b,
        &[
            spec((2_000_000, 2_000_000), &[TOP], Some(a)),
            spec((6_000_000, 6_000_000), &[TOP], Some(a)),
        ],
    );
    add_track(
        &mut b,
        20,
        w,
        TOP,
        250_000,
        vec![line((4_000_000, 0), (4_000_000, 20_000_000))],
    );
    let before = b.to_bytes().unwrap();
    for mode in [Mode::Around, Mode::Stop] {
        let result = run(&b, &request(0, 1, mode));
        if mode == Mode::Around {
            assert_eq!(result.unwrap_err().code, NO_PATH);
        } else {
            // the straight way is blocked, but the track up to the wall is a valid track
            let plan = result.unwrap();
            assert!(!plan.complete);
            let mut after = b.clone();
            after.apply(&plan.operation).unwrap();
            assert!(passes_the_rules(&after));
        }
    }
    assert_eq!(b.to_bytes().unwrap(), before);
}

#[test]
fn a_change_of_layer_places_a_via_by_the_rules() {
    let mut b = base(true);
    let a = add_net(&mut b, 10, "A");
    add_pads(
        &mut b,
        &[
            spec((3_000_000, 3_000_000), &[TOP], Some(a)),
            spec((9_000_000, 3_000_000), &[BOT], Some(a)),
        ],
    );
    let plan = run(&b, &request(0, 1, Mode::Around)).unwrap();
    assert!(plan.complete);
    assert_eq!(plan.vias.len(), 1);
    assert!((1..=2).contains(&plan.tracks.len()));
    let via = &plan.vias[0].1;
    assert_eq!((via.drill, via.diameter), (nm(300_000), nm(600_000)));
    assert_eq!((via.from_layer, via.to_layer), (id(TOP), id(BOT)));
    let mut after = b.clone();
    after.apply(&plan.operation).unwrap();
    assert!(passes_the_rules(&after));
    assert!(after.connectivity().is_fully_connected());
    assert!(ratsnest(&after, &RouteSettings::default())
        .unwrap()
        .is_empty());
    // the mode that stops cannot change layer and says so
    let e = run(&b, &request(0, 1, Mode::Stop)).unwrap_err();
    assert_eq!(e.code, NO_PATH);
}

#[test]
fn the_net_class_gives_the_width_and_the_via() {
    let mut b = base(true);
    let a = add_net(&mut b, 10, "A");
    add_pads(
        &mut b,
        &[
            spec((3_000_000, 3_000_000), &[TOP], Some(a)),
            spec((9_000_000, 3_000_000), &[BOT], Some(a)),
        ],
    );
    let mut context = ctx();
    context.classes = NetClasses::new(vec![NetClass {
        name: "power".into(),
        nets: vec!["A".into()],
        min_track_width: nm(400_000),
        min_clearance: nm(0),
        min_via_diameter: nm(800_000),
        min_via_drill: nm(0),
    }])
    .unwrap();
    let mut n = 500_u32;
    let mut new_id = || {
        n += 1;
        id(n)
    };
    let plan = route(
        &b,
        &context,
        &RouteSettings::default(),
        &request(0, 1, Mode::Around),
        &mut new_id,
    )
    .unwrap();
    assert!(plan.tracks.iter().all(|(_, t)| t.width == nm(400_000)));
    assert_eq!(plan.vias[0].1.diameter, nm(800_000));
    let mut after = b.clone();
    after.apply(&plan.operation).unwrap();
    assert!(findings(&after, &context)
        .iter()
        .all(|f| f.severity != Severity::Error));
}

#[test]
fn requests_that_cannot_be_served_have_their_own_codes() {
    let mut b = base(true);
    let a = add_net(&mut b, 10, "A");
    let c = add_net(&mut b, 11, "C");
    add_pads(
        &mut b,
        &[
            spec((3_000_000, 3_000_000), &[TOP], Some(a)),
            spec((9_000_000, 3_000_000), &[TOP], Some(c)),
            spec((9_000_000, 9_000_000), &[TOP], Some(a)),
            spec((3_000_000, 9_000_000), &[TOP], None),
        ],
    );
    assert_eq!(
        run(&b, &request(0, 1, Mode::Around)).unwrap_err().code,
        NET_MISMATCH
    );
    assert_eq!(
        run(&b, &request(0, 3, Mode::Around)).unwrap_err().code,
        NET_MISMATCH
    );
    assert_eq!(
        run(&b, &request(0, 0, Mode::Around)).unwrap_err().code,
        SAME_PLACE
    );
    let mut missing = request(0, 2, Mode::Around);
    missing.to = Element::PadName {
        refdes: "X9".into(),
        number: "1".into(),
    };
    assert_eq!(run(&b, &missing).unwrap_err().code, ELEMENT_NOT_FOUND);
    // by name the same pads are found
    let mut named = request(0, 2, Mode::Around);
    named.from = Element::PadName {
        refdes: "U1".into(),
        number: "1".into(),
    };
    assert!(run(&b, &named).unwrap().complete);
    // a layer the pad is not on, and a layer that is not copper
    let mut wrong = request(0, 2, Mode::Around);
    wrong.layer = Some(id(BOT));
    assert_eq!(run(&b, &wrong).unwrap_err().code, LAYER_UNUSABLE);
    wrong.layer = Some(id(999));
    assert_eq!(run(&b, &wrong).unwrap_err().code, LAYER_UNUSABLE);
    assert_eq!(copper_layers(&b), vec![id(TOP), id(BOT)]);
}

#[test]
fn the_ratsnest_joins_the_pieces_of_a_net_by_the_shortest_lines() {
    let mut b = base(true);
    let a = add_net(&mut b, 10, "A");
    add_pads(
        &mut b,
        &[
            spec((2_000_000, 2_000_000), &[TOP], Some(a)),
            spec((4_000_000, 2_000_000), &[TOP], Some(a)),
            spec((12_000_000, 2_000_000), &[TOP], Some(a)),
            spec((12_000_000, 10_000_000), &[TOP], None),
        ],
    );
    let settings = RouteSettings::default();
    let lines: Vec<RatLine> = ratsnest(&b, &settings).unwrap();
    let mut lengths: Vec<i64> = lines.iter().map(|l| l.length_nm).collect();
    lengths.sort_unstable();
    assert_eq!(lengths, vec![2_000_000, 8_000_000]);
    assert!(lines.iter().all(|l| l.net == a));
    assert_eq!(lines, ratsnest(&b, &settings).unwrap());
    // a track between the first two pads leaves one line, from the track to the third pad
    add_track(
        &mut b,
        20,
        a,
        TOP,
        250_000,
        vec![line((2_000_000, 2_000_000), (4_000_000, 2_000_000))],
    );
    let rest = ratsnest(&b, &settings).unwrap();
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].length_nm, 8_000_000);
    // a limit on the work is an error with a code
    let tight = RouteSettings {
        ratsnest_max_pairs: 1,
        ..RouteSettings::default()
    };
    assert_eq!(ratsnest(&b, &tight).unwrap_err().code, LIMIT_EXCEEDED);
}

#[test]
fn applying_and_undoing_an_edit_gives_the_original_bytes() {
    let (b, _) = diagonal_board(&[]);
    let plan = run(&b, &request(0, 1, Mode::Around)).unwrap();
    let mut board = b.clone();
    let original = board.to_bytes().unwrap();
    let mut editor = Editor::new(ctx(), &RouteSettings::default());
    let changes = editor.apply(&mut board, plan.operation.clone()).unwrap();
    assert!(!changes.adds_error());
    let edited = board.to_bytes().unwrap();
    assert_ne!(edited, original);
    assert_eq!(board.tracks.len(), 1);
    assert!(editor.undo(&mut board).is_some());
    assert_eq!(board.to_bytes().unwrap(), original);
    assert!(editor.redo(&mut board).is_some());
    assert_eq!(board.to_bytes().unwrap(), edited);
    assert!(editor.undo(&mut board).is_some());
    assert!(editor.undo(&mut board).is_none());
    assert_eq!(board.to_bytes().unwrap(), original);
    assert!(editor.history().done().is_empty());
}

#[test]
fn an_edit_that_adds_an_error_is_refused_and_leaves_no_trace() {
    let mut b = base(true);
    let a = add_net(&mut b, 10, "A");
    let c = add_net(&mut b, 11, "C");
    add_track(
        &mut b,
        20,
        a,
        TOP,
        250_000,
        vec![line((2_000_000, 10_000_000), (18_000_000, 10_000_000))],
    );
    let crossing = Operation::AddTrack {
        id: id(21),
        track: Track {
            net: c,
            layer: id(TOP),
            width: nm(250_000),
            path: vec![line((10_000_000, 2_000_000), (10_000_000, 18_000_000))],
        },
    };
    let original = b.to_bytes().unwrap();
    let mut editor = Editor::new(ctx(), &RouteSettings::default());
    let e = editor.apply(&mut b, crossing.clone()).unwrap_err();
    assert_eq!(e.code, EDIT_REJECTED);
    assert_eq!(b.to_bytes().unwrap(), original);
    assert!(editor.history().done().is_empty());
    // with the strict mode off the edit goes through and the report says what it added
    let lax = RouteSettings {
        strict_edits: false,
        ..RouteSettings::default()
    };
    let mut editor = Editor::new(ctx(), &lax);
    let changes = editor.apply(&mut b, crossing).unwrap();
    assert!(changes.adds_error());
    let undone = editor.undo(&mut b).unwrap();
    assert_eq!(undone.removed.len(), changes.added.len());
    assert_eq!(b.to_bytes().unwrap(), original);
    // an operation the board refuses is an error and nothing changes
    let missing = Operation::RemoveTrack { id: id(77) };
    assert!(editor.apply(&mut b, missing).is_err());
    assert_eq!(b.to_bytes().unwrap(), original);
}

fn board_with_two_piece_track() -> Board {
    let mut b = base(true);
    let a = add_net(&mut b, 10, "A");
    add_pads(&mut b, &[spec((2_000_000, 2_000_000), &[TOP], Some(a))]);
    add_track(
        &mut b,
        20,
        a,
        TOP,
        250_000,
        vec![
            line((2_000_000, 2_000_000), (6_000_000, 2_000_000)),
            line((6_000_000, 2_000_000), (6_000_000, 6_000_000)),
        ],
    );
    b
}

#[test]
fn tracks_are_edited_by_reversible_operations() {
    let mut b = board_with_two_piece_track();
    let original = b.to_bytes().unwrap();
    let mut editor = Editor::new(ctx(), &RouteSettings::default());
    // the end of the second piece moves along its own direction: the angles stay
    let op = move_vertex(&b, id(20), 2, pt(6_000_000, 8_000_000), true).unwrap();
    editor.apply(&mut b, op).unwrap();
    assert_eq!(
        b.tracks[&id(20)].path[1],
        line((6_000_000, 2_000_000), (6_000_000, 8_000_000))
    );
    // the corner may not leave the grid of angles with keep_angles, and may without it
    let e = move_vertex(&b, id(20), 1, pt(7_000_000, 3_000_000), true).unwrap_err();
    assert_eq!(e.code, ANGLE_BROKEN);
    let free = move_vertex(&b, id(20), 1, pt(7_000_000, 3_000_000), false).unwrap();
    editor.apply(&mut b, free).unwrap();
    assert_eq!(b.tracks[&id(20)].path[0].end(), pt(7_000_000, 3_000_000));
    assert_eq!(b.tracks[&id(20)].path[1].start(), pt(7_000_000, 3_000_000));
    // a vertex that does not exist, and a piece that would shrink to a point
    assert_eq!(
        move_vertex(&b, id(20), 3, pt(0, 0), false)
            .unwrap_err()
            .code,
        EDIT_INVALID
    );
    assert_eq!(
        move_vertex(&b, id(20), 1, pt(2_000_000, 2_000_000), false)
            .unwrap_err()
            .code,
        EDIT_INVALID
    );
    // width and deletion
    let wide = set_track_width(&b, id(20), nm(400_000)).unwrap();
    editor.apply(&mut b, wide).unwrap();
    assert_eq!(b.tracks[&id(20)].width, nm(400_000));
    let gone = delete_track(&b, id(20)).unwrap();
    editor.apply(&mut b, gone).unwrap();
    assert!(b.tracks.is_empty());
    assert_eq!(delete_track(&b, id(20)).unwrap_err().code, EDIT_INVALID);
    // four edits are undone one by one down to the original bytes
    for _ in 0..4 {
        assert!(editor.undo(&mut b).is_some());
    }
    assert_eq!(b.to_bytes().unwrap(), original);
}

#[test]
fn components_move_and_turn_and_vias_follow_the_rules() {
    let mut b = board_with_two_piece_track();
    let original = b.to_bytes().unwrap();
    let mut editor = Editor::new(ctx(), &RouteSettings::default());
    let moved = move_component(&b, id(COMPONENT), pt(1_000_000, 0)).unwrap();
    editor.apply(&mut b, moved).unwrap();
    assert_eq!(b.components[&id(COMPONENT)].at, pt(1_000_000, 0));
    let turned = rotate_component(&b, id(COMPONENT), 90_000_000).unwrap();
    editor.apply(&mut b, turned).unwrap();
    assert_eq!(
        b.components[&id(COMPONENT)].rotation,
        Udeg::new(90_000_000).unwrap()
    );
    let back = rotate_component(&b, id(COMPONENT), -180_000_000).unwrap();
    editor.apply(&mut b, back).unwrap();
    assert_eq!(
        b.components[&id(COMPONENT)].rotation,
        Udeg::new(270_000_000).unwrap()
    );
    assert_eq!(b.components[&id(COMPONENT)].at, pt(1_000_000, 0));
    assert_eq!(
        move_component(&b, id(5), pt(0, 0)).unwrap_err().code,
        EDIT_INVALID
    );
    let via = via_for_layer_change(
        &b,
        &ctx(),
        &RouteSettings::default(),
        id(10),
        pt(10_000_000, 10_000_000),
        (id(TOP), id(BOT)),
    )
    .unwrap();
    assert_eq!((via.drill, via.diameter), (nm(300_000), nm(600_000)));
    let changes = editor.apply(&mut b, add_via(id(30), via)).unwrap();
    assert!(!changes.adds_error());
    assert_eq!(b.vias.len(), 1);
    assert_eq!(
        via_for_layer_change(
            &b,
            &ctx(),
            &RouteSettings::default(),
            id(77),
            pt(0, 0),
            (id(TOP), id(BOT))
        )
        .unwrap_err()
        .code,
        EDIT_INVALID
    );
    for _ in 0..4 {
        assert!(editor.undo(&mut b).is_some());
    }
    assert_eq!(b.to_bytes().unwrap(), original);
}

#[test]
fn the_difference_of_two_boards_names_what_an_edit_added() {
    let mut b = board_with_two_piece_track();
    let before = findings(&b, &ctx());
    let narrow = set_track_width(&b, id(20), nm(100_000)).unwrap();
    b.apply(&narrow).unwrap();
    let after = findings(&b, &ctx());
    let changes = diff(&before, &after);
    assert!(changes.added.iter().any(|f| f.code == codes::TRACK_NARROW));
    assert!(changes.adds_error());
}

/// A small generator of numbers, the same on every machine.
fn next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

#[test]
fn a_track_that_the_router_returns_always_passes_the_rules() {
    let mut routed = 0;
    for seed in 0..20_u64 {
        let mut state = seed + 1;
        // distinct places of a grid of 2 mm: the first two are the ends of the track
        let mut places: Vec<(i64, i64)> = Vec::new();
        while places.len() < 8 {
            let x = 2_000_000 * (1 + (next(&mut state) % 9) as i64);
            let y = 2_000_000 * (1 + (next(&mut state) % 9) as i64);
            if !places.contains(&(x, y)) {
                places.push((x, y));
            }
        }
        let mut b = base(true);
        let a = add_net(&mut b, 10, "A");
        let n1 = add_net(&mut b, 11, "B");
        let n2 = add_net(&mut b, 12, "C");
        let mut pads = vec![
            spec(places[0], &[TOP], Some(a)),
            spec(places[1], &[TOP], Some(a)),
        ];
        for (i, place) in places.iter().enumerate().skip(2) {
            let net = if i % 2 == 0 { n1 } else { n2 };
            // some obstacles are on both layers, some only on the top
            let layers: &[u32] = if next(&mut state).is_multiple_of(2) {
                &[TOP, BOT]
            } else {
                &[TOP]
            };
            pads.push(spec(*place, layers, Some(net)));
        }
        add_pads(&mut b, &pads);
        assert!(
            passes_the_rules(&b),
            "seed {seed}: the board itself is clean"
        );
        let mode = if seed % 4 == 0 {
            Mode::Stop
        } else {
            Mode::Around
        };
        match run(&b, &request(0, 1, mode)) {
            Ok(plan) => {
                routed += 1;
                let mut after = b.clone();
                after.apply(&plan.operation).unwrap();
                assert!(passes_the_rules(&after), "seed {seed}");
                assert!(!plan.diff.adds_error(), "seed {seed}");
                if plan.complete {
                    let joined = after
                        .connectivity()
                        .nets
                        .iter()
                        .find(|n| n.net == a)
                        .is_some_and(|n| n.islands.len() == 1);
                    assert!(joined, "seed {seed}");
                }
            }
            Err(e) => assert_eq!(e.code, NO_PATH, "seed {seed}: {e}"),
        }
    }
    assert!(routed >= 10, "{routed}");
}
