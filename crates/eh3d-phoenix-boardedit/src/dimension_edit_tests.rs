// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the preview of the dimension in progress and of the editing of a dimension that is
//! already on the board.

use eh3d_phoenix_board::{Dimension, DimensionKind, Id, Layer, LayerKind};
use eh3d_phoenix_model::{Nm, Point};

use crate::codes::OPERATION_REFUSED;
use crate::dimension_edit::{build_dimension, handles, moved, with_kind, DimensionHandle};
use crate::edit::BoardEdit;
use crate::settings::BoardEditSettings;
use crate::testutil::{fixture, id, strict_rules, Fixture};
use crate::tool::{EditState, Tool};

fn state_with_layer() -> (Fixture, EditState, Id) {
    let mut f = fixture();
    let layer = id(70);
    f.board.layers.insert(
        layer,
        Layer {
            name: "Notes".to_owned(),
            kind: LayerKind::Documentation,
            side: None,
        },
    );
    let mut s = EditState::new(
        BoardEdit::new(
            f.board.clone(),
            strict_rules(),
            BoardEditSettings::default(),
        )
        .unwrap(),
    );
    s.set_tool(Tool::Dimension);
    (f, s, layer)
}

/// An aligned dimension from (10, 10) to (13, 14) with the line through the first point.
fn with_one_dimension() -> (Fixture, EditState, Id) {
    let (f, mut s, _) = state_with_layer();
    assert!(s.add_dimension_through(&[f.pt(10, 10), f.pt(13, 14), f.pt(10, 10)]));
    let dimension = *s.board().dimensions.keys().next().unwrap();
    (f, s, dimension)
}

fn only(s: &EditState) -> &Dimension {
    assert_eq!(s.board().dimensions.len(), 1);
    s.board().dimensions.values().next().unwrap()
}

#[test]
fn nothing_is_previewed_until_a_point_is_given() {
    let (f, mut s, _) = state_with_layer();
    assert!(s.dimension_preview(f.pt(20, 20)).is_none());
    s.pointer_pressed(f.pt(10, 10), false);
    let shown = s.dimension_preview(f.pt(13, 14)).unwrap();
    assert_eq!(
        (shown.kind, shown.length_nm()),
        (DimensionKind::Aligned, 5_000_000)
    );
    // the preview does not touch the board or the points
    assert!(s.board().dimensions.is_empty());
    assert_eq!(s.dimension_points.len(), 1);
    // the cursor on the first point measures nothing
    assert!(s.dimension_preview(f.pt(10, 10)).is_none());
}

#[test]
fn the_preview_follows_the_pointer_for_the_line_and_for_the_arc_of_an_angle() {
    let (f, mut s, _) = state_with_layer();
    s.pointer_pressed(f.pt(10, 10), false);
    s.pointer_pressed(f.pt(20, 10), false);
    // the third click is the point of the line, 2 mm to the left of the direction of the points
    let line = s.dimension_preview(f.pt(15, 12)).unwrap();
    assert_eq!(line.offset.get(), 2_000_000);
    s.set_dimension_kind(DimensionKind::Angle);
    s.pointer_pressed(f.pt(10, 10), false);
    s.pointer_pressed(f.pt(20, 10), false);
    // with the vertex and the first ray, the cursor is the second ray
    let angle = s.dimension_preview(f.pt(10, 20)).unwrap();
    assert_eq!(angle.kind, DimensionKind::Angle);
    assert_eq!(angle.third, Some(f.pt(10, 20)));
    assert_eq!(angle.offset.get(), 10_000_000);
    s.pointer_pressed(f.pt(10, 20), false);
    // with all but the point of the arc, the cursor is the arc: its radius is the distance
    let arc = s.dimension_preview(f.pt(10, 15)).unwrap();
    assert_eq!(arc.offset.get(), 5_000_000);
    assert!(s.board().dimensions.is_empty());
}

#[test]
fn a_board_without_a_documentation_layer_previews_nothing() {
    let f = fixture();
    let mut s = EditState::new(
        BoardEdit::new(
            f.board.clone(),
            strict_rules(),
            BoardEditSettings::default(),
        )
        .unwrap(),
    );
    s.set_tool(Tool::Dimension);
    s.pointer_pressed(f.pt(10, 10), false);
    assert!(s.dimension_preview(f.pt(13, 14)).is_none());
}

#[test]
fn shift_and_a_click_take_a_point_and_the_next_click_puts_it_down_as_one_step() {
    let (f, mut s, dimension) = with_one_dimension();
    let steps = s.edit.steps();
    s.pointer_pressed(f.pt(13, 14), true);
    assert!(s.error.is_none(), "{:?}", s.error);
    let grab = s.dimension_grab.unwrap();
    assert_eq!(
        (grab.dimension, grab.handle),
        (dimension, DimensionHandle::To)
    );
    // while it is taken the preview shows the dimension with the point at the pointer
    let shown = s.dimension_preview(f.pt(20, 14)).unwrap();
    assert_eq!(shown.to, f.pt(20, 14));
    assert_eq!(only(&s).to, f.pt(13, 14));
    s.pointer_pressed(f.pt(20, 14), false);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert!(s.dimension_grab.is_none());
    assert_eq!(only(&s).to, f.pt(20, 14));
    assert_eq!(only(&s).from, f.pt(10, 10));
    assert_eq!(s.edit.steps(), steps + 1);
    // the new dimension is the one selected
    let selected = s.selected_dimension.unwrap();
    assert!(s.board().dimensions.contains_key(&selected));
    assert!(s.undo());
    assert_eq!(only(&s).to, f.pt(13, 14));
    // the dimension that was selected is gone after the undo of its add
    assert!(s.undo());
    assert!(s.selected_dimension.is_none());
}

#[test]
fn the_dimension_line_is_moved_with_its_middle_point() {
    let (f, mut s, _) = with_one_dimension();
    s.edit.settings.pick_tolerance_nm = 100_000;
    let nm = |x: i64, y: i64| Point::from_nm(x, y).unwrap();
    // a click near the middle of the two points but not on a point of the dimension
    s.pointer_pressed(f.pt(12, 12), true);
    assert!(s.dimension_grab.is_none());
    // the middle of the line, which has no offset yet, is the middle of the two points
    s.pointer_pressed(nm(11_500_000, 12_000_000), true);
    assert_eq!(s.dimension_grab.unwrap().handle, DimensionHandle::Line);
    s.pointer_pressed(nm(11_500_000, 13_000_000), false);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert_eq!(only(&s).offset.get(), 600_000);
}

#[test]
fn the_delete_tool_removes_a_dimension_by_one_of_its_points_as_one_step() {
    let (f, mut s, _) = with_one_dimension();
    s.set_tool(Tool::Delete);
    // a click away from every point of the dimension and every object does nothing
    s.pointer_pressed(f.pt(40, 30), false);
    assert_eq!(only(&s).kind, DimensionKind::Aligned);
    assert_eq!(s.edit.steps(), 1);
    assert!(s.error.is_none(), "{:?}", s.error);
    // a click on the second point measured removes it
    s.pointer_pressed(f.pt(13, 14), false);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert!(s.board().dimensions.is_empty());
    assert_eq!(s.edit.steps(), 2);
    assert!(s.undo());
    assert_eq!(only(&s).to, f.pt(13, 14));
}

#[test]
fn a_click_that_is_not_on_a_point_takes_nothing_and_says_so() {
    let (f, mut s, _) = with_one_dimension();
    s.pointer_pressed(f.pt(40, 30), true);
    assert!(s.dimension_grab.is_none());
    assert!(s.selected_dimension.is_none());
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    assert_eq!(s.edit.steps(), 1);
}

#[test]
fn a_point_put_where_the_dimension_measures_nothing_is_refused_and_changes_nothing() {
    let (f, mut s, dimension) = with_one_dimension();
    let before = s.board().clone();
    assert!(!s.move_dimension_point(dimension, DimensionHandle::To, f.pt(10, 10)));
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    assert_eq!(s.board(), &before);
    // a dimension that is not there
    assert!(!s.move_dimension_point(id(99), DimensionHandle::To, f.pt(11, 11)));
    assert_eq!(s.board(), &before);
    // an angle has no third point to move in another kind
    assert!(!s.move_dimension_point(dimension, DimensionHandle::Third, f.pt(11, 11)));
    assert_eq!(s.board(), &before);
}

#[test]
fn a_dimension_changes_kind_with_its_points_and_its_line() {
    let (f, mut s, dimension) = with_one_dimension();
    assert!(s.change_dimension_kind(dimension, DimensionKind::Horizontal));
    let d = only(&s);
    assert_eq!(
        (d.kind, d.from, d.to),
        (DimensionKind::Horizontal, f.pt(10, 10), f.pt(13, 14))
    );
    assert_eq!((d.length_nm(), d.offset.get()), (3_000_000, 2_000_000));
    // the same kind is no change of the board but is not an error
    let selected = s.selected_dimension.unwrap();
    assert!(s.change_dimension_kind(selected, DimensionKind::Horizontal));
    assert_eq!(s.edit.steps(), 2);
    // an angle needs a third point
    let before = s.board().clone();
    assert!(!s.change_dimension_kind(selected, DimensionKind::Angle));
    assert_eq!(s.board(), &before);
    // one undo takes back the change of kind
    assert!(s.undo());
    assert_eq!(only(&s).kind, DimensionKind::Aligned);
}

#[test]
fn an_angle_changes_to_another_kind_with_the_line_through_its_second_point() {
    let (f, mut s, _) = state_with_layer();
    s.set_dimension_kind(DimensionKind::Angle);
    assert!(s.add_dimension_through(&[f.pt(10, 10), f.pt(20, 10), f.pt(10, 20), f.pt(15, 10)]));
    let angle = *s.board().dimensions.keys().next().unwrap();
    assert!(s.change_dimension_kind(angle, DimensionKind::Horizontal));
    let d = only(&s);
    assert_eq!((d.kind, d.third), (DimensionKind::Horizontal, None));
    // the line passes through the second point, which is level with the first
    assert_eq!((d.length_nm(), d.offset.get()), (10_000_000, 0));
}

#[test]
fn the_pure_functions_give_the_handles_the_moves_and_the_kinds() {
    let (f, _s, _) = state_with_layer();
    let layer = id(70);
    let (h, t) = (Nm::new(1_500_000).unwrap(), Nm::new(150_000).unwrap());
    assert!(build_dimension(DimensionKind::Aligned, &[f.pt(1, 1)], layer, h, t).is_none());
    assert!(build_dimension(
        DimensionKind::Aligned,
        &[f.pt(1, 1), f.pt(1, 1), f.pt(2, 2)],
        layer,
        h,
        t
    )
    .is_none());
    let radius = build_dimension(
        DimensionKind::Radius,
        &[f.pt(10, 10), f.pt(15, 10), f.pt(18, 10)],
        layer,
        h,
        t,
    )
    .unwrap();
    // a radius has its centre, the point on the circle and the end of the leader
    let kinds: Vec<DimensionHandle> = handles(&radius).iter().map(|(k, _)| *k).collect();
    assert_eq!(
        kinds,
        [
            DimensionHandle::From,
            DimensionHandle::To,
            DimensionHandle::Line
        ]
    );
    assert_eq!(handles(&radius)[2].1, f.pt(18, 10));
    let angle = build_dimension(
        DimensionKind::Angle,
        &[f.pt(10, 10), f.pt(20, 10), f.pt(10, 20), f.pt(15, 10)],
        layer,
        h,
        t,
    )
    .unwrap();
    // an angle has no line handle, and its third point moves
    let kinds: Vec<DimensionHandle> = handles(&angle).iter().map(|(k, _)| *k).collect();
    assert_eq!(
        kinds,
        [
            DimensionHandle::From,
            DimensionHandle::To,
            DimensionHandle::Third
        ]
    );
    assert!(moved(&angle, DimensionHandle::Line, f.pt(1, 1)).is_none());
    assert_eq!(
        moved(&angle, DimensionHandle::Third, f.pt(10, 30))
            .unwrap()
            .third,
        Some(f.pt(10, 30))
    );
    assert!(moved(&radius, DimensionHandle::Third, f.pt(1, 1)).is_none());
    assert!(with_kind(&radius, DimensionKind::Angle).is_none());
    assert_eq!(with_kind(&radius, DimensionKind::Radius).unwrap(), radius);
    let diameter = with_kind(&radius, DimensionKind::Diameter).unwrap();
    assert_eq!(
        (diameter.kind, diameter.offset.get()),
        (DimensionKind::Diameter, 3_000_000)
    );
}

#[test]
fn the_editing_mode_takes_a_point_without_shift_and_never_starts_a_new_dimension() {
    let (f, mut s, dimension) = with_one_dimension();
    let steps = s.edit.steps();
    s.set_dimension_edit_mode(true);
    // a click that is not on a point takes nothing and starts no dimension
    s.pointer_pressed(f.pt(40, 30), false);
    assert!(s.dimension_grab.is_none());
    assert!(s.dimension_points.is_empty());
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    // a click on a point takes it, the next click puts it down as one step
    s.pointer_pressed(f.pt(13, 14), false);
    assert_eq!(s.dimension_grab.unwrap().dimension, dimension);
    s.pointer_pressed(f.pt(20, 14), false);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert_eq!(only(&s).to, f.pt(20, 14));
    assert_eq!(s.edit.steps(), steps + 1);
    assert!(s.dimension_grab.is_none());
}

#[test]
fn leaving_the_editing_mode_draws_dimensions_again() {
    let (f, mut s, _) = with_one_dimension();
    s.set_dimension_edit_mode(true);
    s.set_dimension_edit_mode(false);
    s.pointer_pressed(f.pt(30, 30), false);
    assert_eq!(s.dimension_points.len(), 1);
    // turning the mode on drops the dimension in progress
    s.set_dimension_edit_mode(true);
    assert!(s.dimension_points.is_empty());
}
