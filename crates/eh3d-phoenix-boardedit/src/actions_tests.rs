// SPDX-License-Identifier: GPL-3.0-or-later
use eh3d_phoenix_board::{Segment, Track};
use eh3d_phoenix_model::{Nm, Point};

use crate::actions::{EditAction, SwapClick};
use crate::codes::OPERATION_REFUSED;
use crate::edit::BoardEdit;
use crate::hit::Picked;
use crate::ops::EditOp;
use crate::settings::BoardEditSettings;
use crate::testutil::{fixture, id, strict_rules, Fixture};
use crate::tool::{EditState, Tool};

fn state(f: &Fixture) -> EditState {
    EditState::new(
        BoardEdit::new(
            f.board.clone(),
            strict_rules(),
            BoardEditSettings::default(),
        )
        .unwrap(),
    )
}

fn p(x_mm: i64, y_mm: i64) -> Point {
    Point::from_nm(x_mm * 1_000_000, y_mm * 1_000_000).unwrap()
}

fn extra_track(f: &mut Fixture, n: u16, path: Vec<Segment>) -> eh3d_phoenix_board::Id {
    let t = id(n);
    f.board.tracks.insert(
        t,
        Track {
            net: f.sig,
            layer: f.bottom,
            width: Nm::new(250_000).unwrap(),
            path,
        },
    );
    t
}

#[test]
fn the_actions_that_can_be_chosen_follow_the_selection() {
    let mut f = fixture();
    let a = extra_track(
        &mut f,
        70,
        vec![Segment::Line {
            start: p(5, 30),
            end: p(10, 30),
        }],
    );
    let b = extra_track(
        &mut f,
        71,
        vec![Segment::Line {
            start: p(10, 30),
            end: p(15, 30),
        }],
    );
    let mut s = state(&f);
    let none = s.available_actions();
    assert!(none.contains(&EditAction::TidyTracks) && none.contains(&EditAction::SwapPads));
    assert!(!none.contains(&EditAction::JoinTracks));
    assert!(!none.contains(&EditAction::SnapToGrid));
    assert!(!none.contains(&EditAction::WidenTracks));
    s.selection.insert(Picked::Track(a));
    assert!(s.available_actions().contains(&EditAction::WidenTracks));
    assert!(!s.available_actions().contains(&EditAction::JoinTracks));
    s.selection.insert(Picked::Track(b));
    assert!(s.available_actions().contains(&EditAction::JoinTracks));
    s.selection.clear();
    s.selection.insert(Picked::Component(f.r1));
    assert!(s.available_actions().contains(&EditAction::SnapToGrid));
    assert!(!s.run_action(EditAction::JoinTracks));
}

#[test]
fn join_and_widen_and_narrow_act_on_the_selected_tracks() {
    let mut f = fixture();
    let a = extra_track(
        &mut f,
        70,
        vec![Segment::Line {
            start: p(5, 30),
            end: p(10, 30),
        }],
    );
    let b = extra_track(
        &mut f,
        71,
        vec![Segment::Line {
            start: p(10, 30),
            end: p(15, 30),
        }],
    );
    let mut s = state(&f);
    s.edit.settings.grid_nm = 50_000;
    s.selection.insert(Picked::Track(a));
    assert!(s.run_action(EditAction::WidenTracks));
    assert_eq!(s.board().tracks[&a].width.get(), 300_000);
    assert!(s.run_action(EditAction::NarrowTracks));
    assert_eq!(s.board().tracks[&a].width.get(), 250_000);
    s.selection.insert(Picked::Track(b));
    assert!(s.run_action(EditAction::JoinTracks));
    assert!(!s.board().tracks.contains_key(&b));
    assert_eq!(s.board().tracks[&a].path.len(), 2);
    s.selection.clear();
    assert!(!s.run_action(EditAction::JoinTracks));
}

#[test]
fn snap_to_grid_moves_the_selected_components_only() {
    let mut f = fixture();
    f.board.components.get_mut(&f.r1).unwrap().at = Point::from_nm(10_260_000, 9_740_000).unwrap();
    f.board.components.get_mut(&f.r2).unwrap().at = Point::from_nm(30_260_000, 9_740_000).unwrap();
    let mut s = state(&f);
    s.edit.settings.grid_nm = 500_000;
    s.selection.insert(Picked::Component(f.r1));
    assert!(s.run_action(EditAction::SnapToGrid));
    assert_eq!(
        s.board().components[&f.r1].at,
        Point::from_nm(10_500_000, 9_500_000).unwrap()
    );
    assert_eq!(
        s.board().components[&f.r2].at,
        Point::from_nm(30_260_000, 9_740_000).unwrap()
    );
    assert!(s.undo());
    assert_eq!(
        s.board().components[&f.r1].at,
        Point::from_nm(10_260_000, 9_740_000).unwrap()
    );
}

#[test]
fn the_swap_tool_takes_a_pad_and_swaps_it_with_the_next_one() {
    let f = fixture();
    let mut s = state(&f);
    // the track of SIG would short against the pad that takes its net, so it goes first
    assert!(s.run(&EditOp::DeleteNet { net: f.sig }));
    s.set_tool(Tool::SwapPads);
    let pad_a = f.pad(f.r1, f.pad1);
    let pad_b = f.pad(f.r1, f.pad2);
    // R1 is at (10, 10): pad 1 at 8.5 and pad 2 at 11.5
    assert_eq!(s.swap_click(p(20, 20), 100_000), SwapClick::NoPad);
    assert_eq!(
        s.swap_click(Point::from_nm(8_500_000, 10_000_000).unwrap(), 100_000),
        SwapClick::First(pad_a)
    );
    let before = (s.board().net_of(pad_a), s.board().net_of(pad_b));
    assert_eq!(
        s.swap_click(Point::from_nm(11_500_000, 10_000_000).unwrap(), 100_000),
        SwapClick::Swapped
    );
    assert_eq!(
        (s.board().net_of(pad_a), s.board().net_of(pad_b)),
        (before.1, before.0)
    );
    assert!(s.swap_first.is_none());
    assert!(s.undo());
    assert_eq!(s.board().net_of(pad_a), before.0);
}

#[test]
fn a_refused_swap_drops_the_first_pad_and_cancel_drops_it_too() {
    let f = fixture();
    let mut s = state(&f);
    let at_r1_pad2 = Point::from_nm(11_500_000, 10_000_000).unwrap();
    assert!(matches!(
        s.swap_click(at_r1_pad2, 100_000),
        SwapClick::First(_)
    ));
    // the same pad twice
    assert_eq!(s.swap_click(at_r1_pad2, 100_000), SwapClick::Refused);
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    assert!(s.swap_first.is_none());
    assert!(matches!(
        s.swap_click(at_r1_pad2, 100_000),
        SwapClick::First(_)
    ));
    s.cancel();
    assert!(s.swap_first.is_none());
}

#[test]
fn a_track_is_cut_at_a_point_of_a_piece_and_the_two_parts_keep_the_ends() {
    let f = fixture();
    let mut s = state(&f);
    s.edit.settings.snap = false;
    let ok = s.run(&EditOp::SplitTrackAt {
        track: f.track,
        at: p(20, 10),
    });
    assert!(ok);
    assert_eq!(s.board().tracks.len(), 2);
    let ends: Vec<(Point, Point)> = s
        .board()
        .tracks
        .values()
        .map(|t| (t.path[0].start(), t.path.last().unwrap().end()))
        .collect();
    assert!(ends.contains(&(Point::from_nm(11_500_000, 10_000_000).unwrap(), p(20, 10))));
    assert!(ends.contains(&(p(20, 10), Point::from_nm(28_500_000, 10_000_000).unwrap())));
    assert!(s.undo());
    assert_eq!(s.board().tracks.len(), 1);
    for at in [
        p(20, 11),
        p(5, 5),
        Point::from_nm(11_500_000, 10_000_000).unwrap(),
    ] {
        assert!(!s.run(&EditOp::SplitTrackAt { track: f.track, at }));
    }
}

#[test]
fn the_split_near_a_point_uses_the_track_under_it() {
    let f = fixture();
    let mut s = state(&f);
    s.edit.settings.snap = false;
    assert!(!s.split_track_near(p(20, 30)));
    assert!(s.split_track_near(p(20, 10)));
    assert_eq!(s.board().tracks.len(), 2);
}
