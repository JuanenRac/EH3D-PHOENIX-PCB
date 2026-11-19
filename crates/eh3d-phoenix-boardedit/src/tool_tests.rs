// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests of the editing tool: snapping, picking, the drag, the rectangle, the tools and the keys.

use eh3d_phoenix_board::{Component, PadRef, Side};
use eh3d_phoenix_model::{Point, Udeg};

use crate::codes::{COMPONENT_LOCKED, OPERATION_REFUSED};
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

fn nm(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn with_free_part(f: &mut Fixture) -> eh3d_phoenix_board::Id {
    let part = id(60);
    f.board.components.insert(
        part,
        Component {
            refdes: "R3".to_owned(),
            value: "1k".to_owned(),
            symbol: None,
            footprint: f.footprint,
            at: f.pt(40, 30),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    part
}

#[test]
fn a_point_snaps_to_the_grid_and_not_when_snapping_is_off() {
    let f = fixture();
    let mut s = state(&f);
    assert_eq!(s.snap(nm(130_000, 380_000)), nm(250_000, 500_000));
    assert_eq!(s.snap(nm(-130_000, 0)), nm(-250_000, 0));
    s.edit.settings.snap = false;
    assert_eq!(s.snap(nm(130_000, 380_000)), nm(130_000, 380_000));
}

#[test]
fn the_state_starts_on_the_top_layer_with_the_select_tool() {
    let f = fixture();
    let mut s = state(&f);
    assert_eq!(s.tool, Tool::Select);
    assert_eq!(s.active_layer, Some(f.top));
    assert_eq!(s.place_footprint, Some(f.footprint));
    assert!(s.set_active_layer(f.bottom));
    assert!(!s.set_active_layer(id(99)));
    assert_eq!(s.active_layer, Some(f.bottom));
}

#[test]
fn a_click_selects_and_a_drag_moves_the_component_with_its_track() {
    let f = fixture();
    let mut s = state(&f);
    s.pointer_pressed(f.pt(9, 10), false);
    assert!(s.selection.contains(&Picked::Component(f.r1)));
    assert!(s.drag.is_some());
    s.pointer_moved(f.pt(9, 13));
    let preview = s.drag_preview().unwrap();
    assert_eq!(preview.components[&f.r1].at, f.pt(10, 13));
    assert_eq!(s.board().components[&f.r1].at, f.pt(10, 10));
    s.pointer_released(f.pt(9, 13));
    assert!(s.drag.is_none());
    assert_eq!(s.board().components[&f.r1].at, f.pt(10, 13));
    assert!(s.error.is_none(), "{:?}", s.error);
    assert_eq!(s.edit.steps(), 1);
    assert!(s.undo());
    assert_eq!(s.board().components[&f.r1].at, f.pt(10, 10));
}

#[test]
fn a_locked_component_is_picked_but_does_not_drag() {
    let f = fixture();
    let mut s = state(&f);
    s.pointer_pressed(f.pt(9, 10), false);
    s.pointer_released(f.pt(9, 10));
    assert!(s.toggle_lock_selection());
    s.pointer_pressed(f.pt(9, 10), false);
    assert!(s.drag.is_none());
    assert!(s.selection.contains(&Picked::Component(f.r1)));
    assert!(!s.delete_selection());
    assert_eq!(s.error.as_ref().unwrap().code, COMPONENT_LOCKED);
    assert!(s.toggle_lock_selection());
    assert!(s.delete_selection());
    assert!(s.selection.is_empty());
    assert!(!s.board().components.contains_key(&f.r1));
}

#[test]
fn a_rectangle_selects_what_is_inside_and_lights_the_net() {
    let f = fixture();
    let mut s = state(&f);
    s.pointer_pressed(f.pt(50, 30), false);
    assert!(s.marquee.is_some());
    s.pointer_moved(f.pt(1, 1));
    s.pointer_released(f.pt(1, 1));
    assert!(s.marquee.is_none());
    assert_eq!(s.selection.len(), 3);
    assert_eq!(s.highlight_net, Some(f.sig));
    // a click on nothing, without shift, empties the selection
    s.pointer_pressed(f.pt(50, 30), false);
    s.pointer_released(f.pt(50, 30));
    assert!(s.selection.is_empty());
}

#[test]
fn shift_adds_to_the_selection_and_takes_off_what_is_there() {
    let f = fixture();
    let mut s = state(&f);
    s.pointer_pressed(f.pt(9, 10), false);
    s.pointer_released(f.pt(9, 10));
    s.pointer_pressed(f.pt(29, 10), true);
    s.pointer_released(f.pt(29, 10));
    assert_eq!(s.selected_components().len(), 2);
    s.pointer_pressed(f.pt(29, 10), true);
    s.pointer_released(f.pt(29, 10));
    assert_eq!(s.selected_components(), vec![f.r1]);
}

#[test]
fn the_route_tool_draws_a_track_from_a_pad_through_the_clicks() {
    let f = fixture();
    let mut s = state(&f);
    s.set_tool(Tool::Route);
    s.pointer_pressed(f.pt(8, 10), false);
    assert_eq!(s.route_net, Some(f.gnd));
    assert_eq!(s.route_points, vec![nm(8_500_000, 10_000_000)]);
    s.pointer_pressed(nm(8_500_000, 14_000_000), false);
    s.pointer_pressed(f.pt(15, 14), false);
    assert_eq!(s.route_preview(f.pt(16, 16)).len(), 5);
    assert!(s.finish_route());
    assert!(s.error.is_none(), "{:?}", s.error);
    assert!(s.route_points.is_empty() && s.route_net.is_none());
    assert_eq!(s.board().tracks.len(), 2);
    // nothing under the pointer and no net lit: no net, no route
    s.set_highlight(None);
    s.pointer_pressed(f.pt(50, 30), false);
    assert!(s.route_points.is_empty());
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
}

#[test]
fn the_preview_of_a_route_follows_the_shape_and_the_order_of_the_corner() {
    let f = fixture();
    let mut s = state(&f);
    s.set_tool(Tool::Route);
    s.diagonal = false;
    s.route_points = vec![nm(0, 0)];
    assert_eq!(
        s.route_preview(nm(4_000_000, 3_000_000)),
        vec![nm(0, 0), nm(4_000_000, 0), nm(4_000_000, 3_000_000)]
    );
    s.horizontal_first = false;
    assert_eq!(
        s.route_preview(nm(4_000_000, 3_000_000)),
        vec![nm(0, 0), nm(0, 3_000_000), nm(4_000_000, 3_000_000)]
    );
    s.cancel();
    assert!(s.route_preview(nm(1, 1)).is_empty());
}

#[test]
fn the_via_and_delete_tools_act_on_what_is_under_the_pointer() {
    let f = fixture();
    let mut s = state(&f);
    s.set_tool(Tool::Via);
    s.pointer_pressed(f.pt(20, 10), false);
    assert_eq!(s.board().vias.len(), 1);
    s.pointer_pressed(f.pt(50, 30), false);
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    s.set_tool(Tool::Delete);
    s.pointer_pressed(f.pt(20, 10), false);
    assert!(s.board().vias.is_empty());
    s.pointer_pressed(f.pt(20, 10), false);
    assert!(s.board().tracks.is_empty());
    s.pointer_pressed(f.pt(50, 30), false);
    assert_eq!(s.edit.steps(), 3);
}

#[test]
fn the_place_tool_makes_components_with_a_new_reference_each_time() {
    let f = fixture();
    let mut s = state(&f);
    s.set_tool(Tool::Place);
    s.pointer_pressed(f.pt(40, 30), false);
    s.pointer_pressed(f.pt(45, 30), false);
    let refs: Vec<String> = s
        .board()
        .components
        .values()
        .map(|c| c.refdes.clone())
        .collect();
    assert!(refs.contains(&"U3".to_owned()), "{refs:?}");
    assert!(refs.contains(&"U4".to_owned()), "{refs:?}");
    s.place_footprint = None;
    s.pointer_pressed(f.pt(50, 30), false);
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    // an undo takes a selected component off the board and out of the selection
    s.place_footprint = Some(f.footprint);
    s.set_tool(Tool::Select);
    s.pointer_pressed(f.pt(44, 30), false);
    s.pointer_released(f.pt(44, 30));
    assert_eq!(s.selection.len(), 1);
    assert!(s.undo());
    assert!(s.selection.is_empty());
    assert!(s.redo());
}

#[test]
fn the_selection_is_turned_and_mirrored_as_one_step_each() {
    let mut f = fixture();
    let part = with_free_part(&mut f);
    let mut s = state(&f);
    assert!(!s.rotate_selection(1));
    s.selection.insert(Picked::Component(part));
    assert!(s.rotate_selection(-1));
    assert_eq!(s.board().components[&part].rotation.get(), 270_000_000);
    assert!(s.mirror_selection());
    assert_eq!(s.board().components[&part].side, Side::Bottom);
    assert_eq!(s.edit.steps(), 2);
}

#[test]
fn the_pads_of_the_selected_parts_are_marked_as_test_points_and_unmarked_by_the_same_button() {
    let mut f = fixture();
    let part = with_free_part(&mut f);
    let (pad1, pad2) = (f.pad1, f.pad2);
    let mark = |pad| PadRef {
        component: part,
        pad,
    };
    let mut s = state(&f);
    // nothing selected, nothing done
    assert!(!s.toggle_test_points_selection());
    s.selection.insert(Picked::Component(part));
    // a lock does not stop it
    assert!(s.edit.apply(&EditOp::Lock { id: part }).is_ok());
    assert!(s.toggle_test_points_selection());
    assert!(s.board().is_test_point(mark(pad1)) && s.board().is_test_point(mark(pad2)));
    assert_eq!(s.board().test_points.len(), 2);
    // all marked: the same button takes the marks off
    assert!(s.toggle_test_points_selection());
    assert!(s.board().test_points.is_empty());
    // one pad marked: the button marks the rest
    assert!(s
        .edit
        .apply(&EditOp::TestPoint {
            pad: mark(pad1),
            marked: true
        })
        .is_ok());
    assert!(s.toggle_test_points_selection());
    assert_eq!(s.board().test_points.len(), 2);
    // each press is one step and undo gives back the state before it
    assert!(s.undo());
    assert_eq!(s.board().test_points, vec![mark(pad1)]);
}

#[test]
fn the_test_pad_tool_marks_the_pad_under_the_pointer_and_unmarks_it_on_the_next_click() {
    let mut f = fixture();
    let part = with_free_part(&mut f);
    let pad1 = PadRef {
        component: part,
        pad: f.pad1,
    };
    let at = eh3d_phoenix_board::connectivity::pad_position(&f.board, pad1).unwrap();
    let mut s = state(&f);
    s.set_tool(Tool::TestPad);
    s.pointer_pressed(at, false);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert_eq!(s.board().test_points, vec![pad1]);
    // the other pad of the part is not touched
    assert!(!s.board().is_test_point(PadRef {
        component: part,
        pad: f.pad2
    }));
    // a click on the same pad takes the mark off
    s.pointer_pressed(at, false);
    assert!(s.board().test_points.is_empty());
    // each click is one step
    assert!(s.undo());
    assert_eq!(s.board().test_points, vec![pad1]);
    // a click where there is no pad is refused and changes nothing
    let steps = s.edit.steps();
    s.pointer_pressed(f.pt(90, 90), false);
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    assert_eq!(s.edit.steps(), steps);
}

#[test]
fn the_tune_tool_asks_for_the_length_in_the_state() {
    let f = fixture();
    let mut s = state(&f);
    s.set_tool(Tool::Tune);
    s.tune_target_nm = 19_000_000;
    s.pointer_pressed(f.pt(20, 10), false);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert_eq!(s.edit.net_length_nm(f.sig), Some(19_000_000));
    s.pointer_pressed(f.pt(9, 10), false);
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
}

#[test]
fn the_ratsnest_comes_from_the_nets_the_pads_were_given() {
    let f = fixture();
    let s = state(&f);
    assert_eq!(s.ratsnest().len(), 1);
    let mut lit = state(&f);
    lit.set_highlight(Some(f.gnd));
    assert_eq!(lit.highlight_net, Some(f.gnd));
    assert_eq!(lit.net_at(f.pt(50, 30)), Some(f.gnd));
}

/// The board of the fixture with a documentation layer.
fn with_documentation_layer(f: &mut Fixture) -> eh3d_phoenix_board::Id {
    let layer = id(70);
    f.board.layers.insert(
        layer,
        eh3d_phoenix_board::Layer {
            name: "Notes".to_owned(),
            kind: eh3d_phoenix_board::LayerKind::Documentation,
            side: None,
        },
    );
    layer
}

#[test]
fn the_dimension_tool_makes_a_dimension_from_three_clicks() {
    use eh3d_phoenix_board::DimensionKind;
    let mut f = fixture();
    let layer = with_documentation_layer(&mut f);
    let mut s = state(&f);
    s.set_tool(Tool::Dimension);
    // an aligned one: the two points measured and a point of the line, 2 mm to the left
    s.pointer_pressed(f.pt(10, 10), false);
    s.pointer_pressed(f.pt(13, 14), false);
    assert_eq!(s.dimension_points.len(), 2);
    assert!(s.board().dimensions.is_empty());
    s.pointer_pressed(f.pt(10, 10), false);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert!(s.dimension_points.is_empty());
    assert_eq!(s.board().dimensions.len(), 1);
    let d = s.board().dimensions.values().next().unwrap();
    assert_eq!(
        (d.kind, d.layer, d.third),
        (DimensionKind::Aligned, layer, None)
    );
    assert_eq!(d.length_nm(), 5_000_000);
    assert_eq!(d.offset.get(), 0);
    // the sizes come from the settings
    assert_eq!(
        d.text_height.get(),
        s.edit.settings.dimension_text_height_nm
    );
    assert_eq!(d.stroke.get(), s.edit.settings.dimension_stroke_nm);
    // one click, one step; undo takes it away
    assert_eq!(s.edit.steps(), 1);
    assert!(s.undo());
    assert!(s.board().dimensions.is_empty());
}

#[test]
fn the_dimension_tool_draws_every_kind_with_the_points_it_needs() {
    use eh3d_phoenix_board::DimensionKind;
    let mut f = fixture();
    with_documentation_layer(&mut f);
    let mut s = state(&f);
    s.set_tool(Tool::Dimension);
    // a horizontal one with the line 2 mm above
    s.set_dimension_kind(DimensionKind::Horizontal);
    for p in [f.pt(10, 10), f.pt(20, 15), f.pt(15, 12)] {
        s.pointer_pressed(p, false);
    }
    let horizontal = s.board().dimensions.values().last().unwrap();
    assert_eq!(
        (horizontal.length_nm(), horizontal.offset.get()),
        (10_000_000, 2_000_000)
    );
    // a radius: the centre, a point of the circle and the end of the leader, 2 mm beyond
    s.set_dimension_kind(DimensionKind::Radius);
    for p in [f.pt(30, 30), f.pt(33, 30), f.pt(35, 30)] {
        s.pointer_pressed(p, false);
    }
    let radius = s
        .board()
        .dimensions
        .values()
        .find(|d| d.kind == DimensionKind::Radius)
        .unwrap();
    assert_eq!(
        (radius.length_nm(), radius.offset.get()),
        (3_000_000, 2_000_000)
    );
    // a diameter
    s.set_dimension_kind(DimensionKind::Diameter);
    for p in [f.pt(40, 40), f.pt(48, 40), f.pt(48, 40)] {
        s.pointer_pressed(p, false);
    }
    assert!(s
        .board()
        .dimensions
        .values()
        .any(|d| d.kind == DimensionKind::Diameter && d.length_nm() == 8_000_000));
    // an angle takes four clicks: the vertex, a point on each ray and a point of the arc
    s.set_dimension_kind(DimensionKind::Angle);
    for p in [f.pt(50, 50), f.pt(60, 50), f.pt(50, 60)] {
        s.pointer_pressed(p, false);
    }
    assert_eq!(s.dimension_points.len(), 3);
    s.pointer_pressed(f.pt(55, 50), false);
    assert!(s.error.is_none(), "{:?}", s.error);
    let angle = s
        .board()
        .dimensions
        .values()
        .find(|d| d.kind == DimensionKind::Angle)
        .unwrap();
    assert_eq!(angle.angle_udeg(), Some(90_000_000));
    assert_eq!(angle.offset.get(), 5_000_000);
    assert_eq!(s.board().dimensions.len(), 4);
    // changing the kind or choosing another tool drops the points in progress
    s.pointer_pressed(f.pt(1, 1), false);
    s.set_dimension_kind(DimensionKind::Vertical);
    assert!(s.dimension_points.is_empty());
    s.pointer_pressed(f.pt(1, 1), false);
    s.set_tool(Tool::Select);
    assert!(s.dimension_points.is_empty());
}

#[test]
fn the_dimension_tool_refuses_what_it_cannot_draw_and_changes_nothing() {
    use eh3d_phoenix_board::DimensionKind;
    // a board without a documentation layer
    let f = fixture();
    let mut s = state(&f);
    s.set_tool(Tool::Dimension);
    for p in [f.pt(10, 10), f.pt(13, 14), f.pt(10, 10)] {
        s.pointer_pressed(p, false);
    }
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    assert!(s.board().dimensions.is_empty());
    assert_eq!(s.edit.steps(), 0);
    // the same point twice measures nothing
    let mut g = fixture();
    with_documentation_layer(&mut g);
    let mut s = state(&g);
    s.set_tool(Tool::Dimension);
    s.set_dimension_kind(DimensionKind::Aligned);
    for p in [g.pt(10, 10), g.pt(10, 10), g.pt(12, 10)] {
        s.pointer_pressed(p, false);
    }
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    assert!(s.board().dimensions.is_empty());
    // a wrong number of points to the method that takes them all at once
    assert!(!s.add_dimension_through(&[g.pt(1, 1)]));
}

#[test]
fn the_operations_add_and_remove_a_dimension_on_a_documentation_layer_only() {
    use eh3d_phoenix_board::{Dimension, DimensionKind};
    use eh3d_phoenix_model::Nm;
    let mut f = fixture();
    let layer = with_documentation_layer(&mut f);
    let make = |layer| Dimension {
        layer,
        kind: DimensionKind::Horizontal,
        from: f.pt(0, 0),
        to: f.pt(10, 0),
        third: None,
        offset: Nm::new(2_000_000).unwrap(),
        text_height: Nm::new(1_000_000).unwrap(),
        stroke: Nm::new(100_000).unwrap(),
    };
    let mut s = state(&f);
    assert!(s.run(&EditOp::AddDimension {
        dimension: make(layer)
    }));
    let added = *s.board().dimensions.keys().next().unwrap();
    // a copper layer and a layer the board does not have
    assert!(!s.run(&EditOp::AddDimension {
        dimension: make(f.top)
    }));
    assert_eq!(s.error.as_ref().unwrap().code, OPERATION_REFUSED);
    assert!(!s.run(&EditOp::AddDimension {
        dimension: make(id(99))
    }));
    assert_eq!(s.board().dimensions.len(), 1);
    // it can be removed, and a dimension that is not there cannot
    assert!(s.run(&EditOp::RemoveDimension { dimension: added }));
    assert!(s.board().dimensions.is_empty());
    assert!(!s.run(&EditOp::RemoveDimension { dimension: added }));
}
