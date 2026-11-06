// SPDX-License-Identifier: GPL-3.0-or-later
//! The operations of the board editor, as the file of operations of `phoenix boardedit` writes
//! them (`{"op": "route", "net": "...", "layer": "...", "points": [...]}`). Each one is turned
//! into the reversible operations of `eh3d-phoenix-board` (one batch, all or nothing) by the
//! planner of this crate, and checked by the design rules before it is kept.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Id, PadRef, Side};
use eh3d_phoenix_model::Point;
use serde::{Deserialize, Serialize};

use crate::shape::{MeanderSide, Shape};
use crate::zone::{ZoneChanges, ZoneProps};
use crate::zone_clip::Clipboard;

/// A change of layer along a routed pair: the part of the pair after a vertex goes to another
/// copper layer, with a via for each net.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairLayerChange {
    /// The vertex of the centre line, from 1 (the end of the first piece).
    pub vertex: usize,
    /// The copper layer of the part that follows.
    pub layer: Id,
}

/// One operation of the editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EditOp {
    /// Places a new component of a footprint that the board has.
    Place {
        /// The id of the component; made by the editor when it is left out.
        #[serde(default)]
        id: Option<Id>,
        /// The reference designator, which no other component has.
        refdes: String,
        /// The value.
        #[serde(default)]
        value: String,
        /// The footprint.
        footprint: Id,
        /// Where the origin of the footprint goes.
        at: Point,
        /// The rotation in micro-degrees.
        #[serde(default)]
        rotation_udeg: i64,
        /// The side; the top when it is left out.
        #[serde(default)]
        side: Option<Side>,
    },
    /// Moves a component, and turns it when a rotation is given.
    Move {
        /// The component.
        id: Id,
        /// The new place.
        at: Point,
        /// The new rotation in micro-degrees; it stays when left out.
        #[serde(default)]
        rotation_udeg: Option<i64>,
        /// Whether the tracks on its pads follow; the setting `follow_tracks` when left out.
        #[serde(default)]
        follow: Option<bool>,
    },
    /// Turns a component about its origin.
    Rotate {
        /// The component.
        id: Id,
        /// The angle to add in micro-degrees.
        delta_udeg: i64,
        /// Whether the tracks on its pads follow.
        #[serde(default)]
        follow: Option<bool>,
    },
    /// Puts a component on the other side of the board (it is mirrored about the vertical axis).
    Mirror {
        /// The component.
        id: Id,
        /// Whether the tracks on its pads follow.
        #[serde(default)]
        follow: Option<bool>,
    },
    /// Locks a component: moving, turning, mirroring and removing it is refused.
    Lock {
        /// The component.
        id: Id,
    },
    /// Unlocks a component.
    Unlock {
        /// The component.
        id: Id,
    },
    /// Removes a component with the nets of its pads.
    Remove {
        /// The component.
        id: Id,
    },
    /// Draws a track through points on one layer, with horizontal and vertical pieces and, with
    /// the diagonal shape, 45-degree pieces.
    Route {
        /// The net.
        net: Id,
        /// The copper layer.
        layer: Id,
        /// The points, at least two.
        points: Vec<Point>,
        /// The shape; the setting `diagonal_routes` decides when left out.
        #[serde(default)]
        shape: Option<Shape>,
        /// The width in nanometres; the width of the net class when left out.
        #[serde(default)]
        width_nm: Option<i64>,
        /// Whether a corner goes first along x; when left out both orders are tried.
        #[serde(default)]
        horizontal_first: Option<bool>,
    },
    /// Draws a track like `route` and pushes aside, in parallel, the tracks of other nets that its
    /// clearance would invade (only horizontal, vertical and 45-degree pieces are pushed; pads,
    /// vias and the edge of the board are fixed). All or nothing: it is refused, with a
    /// `PHOENIX-SHOVE-<NNN>` code, when the push cannot be done within the depth and the number of
    /// tracks of the settings, or would leave a rule error.
    RouteShove {
        /// The net.
        net: Id,
        /// The copper layer.
        layer: Id,
        /// The points, at least two.
        points: Vec<Point>,
        /// The shape; the setting `diagonal_routes` decides when left out.
        #[serde(default)]
        shape: Option<Shape>,
        /// The width in nanometres; the width of the net class when left out.
        #[serde(default)]
        width_nm: Option<i64>,
        /// Whether a corner goes first along x; when left out both orders are tried.
        #[serde(default)]
        horizontal_first: Option<bool>,
    },
    /// Puts an arc of a given radius, tangent to both pieces, at a vertex of a track.
    RouteArc {
        /// The track.
        track: Id,
        /// The vertex, from 1 (the end of the first piece).
        vertex: usize,
        /// The radius in nanometres, not below `arc_min_radius_nm`.
        radius_nm: i64,
    },
    /// Rounds the corners of a track, or of every track of a net, with the largest radius that
    /// fits up to the one given (a corner that has no room for the smallest radius stays).
    RoundCorners {
        /// The track; every track of the net when left out.
        #[serde(default)]
        track: Option<Id>,
        /// The net; the net of the track when only the track is given.
        #[serde(default)]
        net: Option<Id>,
        /// The radius in nanometres, not below `arc_min_radius_nm`.
        radius_nm: i64,
    },
    /// Adds a via, sized by the rules and the net class.
    Via {
        /// The net.
        net: Id,
        /// Where it goes.
        at: Point,
        /// The first layer; the top copper layer when left out.
        #[serde(default)]
        from_layer: Option<Id>,
        /// The last layer; the bottom copper layer when left out.
        #[serde(default)]
        to_layer: Option<Id>,
    },
    /// Moves the part of a track after a vertex to another layer and puts a via at the vertex.
    ChangeLayer {
        /// The track.
        track: Id,
        /// The vertex, from 1 (the end of the first piece).
        vertex: usize,
        /// The copper layer of the part that moves.
        layer: Id,
    },
    /// Deletes one piece of a track; the rest of the track stays in one or two tracks.
    DeleteSegment {
        /// The track.
        track: Id,
        /// The piece, from 0.
        index: usize,
    },
    /// Deletes a track.
    DeleteTrack {
        /// The track.
        track: Id,
    },
    /// Deletes a via.
    DeleteVia {
        /// The via.
        via: Id,
    },
    /// Deletes every track and via of a net (its zones and the nets of its pads stay).
    DeleteNet {
        /// The net.
        net: Id,
    },
    /// Draws the shortest route between two pads: a straight piece when they are in line, an L
    /// otherwise. A pad that has no net takes the net of the other.
    RouteBetween {
        /// The first pad.
        from: PadRef,
        /// The second pad.
        to: PadRef,
        /// The copper layer.
        layer: Id,
        /// The shape; the setting `diagonal_routes` decides when left out.
        #[serde(default)]
        shape: Option<Shape>,
    },
    /// Pastes a net: gives the pads the net of this name (made when the board has none) and joins
    /// them in the order given, each with the shortest route to the next.
    PasteNet {
        /// The name of the net.
        name: String,
        /// The pads, at least two.
        pads: Vec<PadRef>,
        /// The copper layer.
        layer: Id,
        /// The shape; the setting `diagonal_routes` decides when left out.
        #[serde(default)]
        shape: Option<Shape>,
    },
    /// Adds meanders to a net until its copper length is the target.
    TuneLength {
        /// The net.
        net: Id,
        /// The length asked for in nanometres.
        target_nm: i64,
        /// The track to put the meanders on; the longest straight pieces of the net when left out.
        #[serde(default)]
        track: Option<Id>,
        /// The side of the meanders.
        #[serde(default)]
        side: MeanderSide,
    },
    /// Draws the two tracks of a differential pair side by side along a centre line, with the
    /// width and the gap of the pair, and a via for each net where the pair changes layer.
    RoutePair {
        /// The name of the pair.
        pair: String,
        /// The copper layer of the first part.
        layer: Id,
        /// The points of the centre line, at least two.
        points: Vec<Point>,
        /// The shape; the setting `diagonal_routes` decides when left out.
        #[serde(default)]
        shape: Option<Shape>,
        /// The width of the tracks in nanometres; the width of the pair when left out.
        #[serde(default)]
        width_nm: Option<i64>,
        /// The gap between the edges of the two tracks in nanometres; the gap of the pair when
        /// left out.
        #[serde(default)]
        gap_nm: Option<i64>,
        /// Whether a corner goes first along x; when left out both orders are tried.
        #[serde(default)]
        horizontal_first: Option<bool>,
        /// The changes of layer, in the order of their vertices.
        #[serde(default)]
        layer_changes: Vec<PairLayerChange>,
    },
    /// Makes the shorter net of a pair as long as the other with meanders, to within the
    /// tolerance of the pair.
    TunePair {
        /// The name of the pair.
        pair: String,
        /// The side of the meanders; the side away from the other net first when left out.
        #[serde(default)]
        side: MeanderSide,
    },
    /// Moves the straight pieces of the negative net whose gap to the positive net is out of
    /// tolerance so that the gap is the one of the pair.
    FixPairGap {
        /// The name of the pair.
        pair: String,
    },
    /// Replaces the outline of the board with the outer contour of a DXF drawing (`docs/DXF.md`):
    /// the lines and arcs on the layer are joined into closed contours, the largest outer one
    /// becomes the outline and the contours inside it become the cutouts of the outline. All or nothing:
    /// a drawing with no closed contour or with a contour that crosses itself is refused, and the
    /// design rules are checked before and after like for any other operation.
    OutlineFromDxf {
        /// The text of the DXF file.
        dxf: String,
        /// The layer to take the contours from, without regard to case; every layer when left
        /// out.
        #[serde(default)]
        layer: Option<String>,
        /// How far apart two ends may be and still be joined, in nanometres; the setting
        /// `join_tolerance_nm` of `config/dxf.cfg` when left out.
        #[serde(default)]
        join_tolerance_nm: Option<i64>,
        /// The text of a settings file `config/dxf.cfg` (the tolerances of the arcs and the
        /// splines and the limits of the reader); the defaults when left out.
        #[serde(default)]
        settings: Option<String>,
    },
    /// Adds a copper zone: a closed polygon of copper of one net on one copper layer, with its
    /// properties (`docs/BOARD_ZONES.md`). The fill is made by `zone_fill`.
    ZoneAdd {
        /// The id of the zone; made by the editor when it is left out.
        #[serde(default)]
        id: Option<Id>,
        /// The net.
        net: Id,
        /// The copper layer.
        layer: Id,
        /// The outline, at least three points; the last joins the first.
        outline: Vec<Point>,
        /// The clearance of the zone in nanometres; the clearance of the rules when left out.
        #[serde(default)]
        clearance_nm: Option<i64>,
        /// The priority, the minimum width and island, the connection and the thermal relief.
        #[serde(default)]
        props: ZoneProps,
    },
    /// Changes the properties or the outline of a zone. What is left out stays; the fill of the
    /// zone is dropped and has to be made again.
    ZoneEdit {
        /// The zone.
        zone: Id,
        /// The new net.
        #[serde(default)]
        net: Option<Id>,
        /// The new copper layer.
        #[serde(default)]
        layer: Option<Id>,
        /// The new outline (its vertices).
        #[serde(default)]
        outline: Option<Vec<Point>>,
        /// The new clearance in nanometres.
        #[serde(default)]
        clearance_nm: Option<i64>,
        /// The new properties.
        #[serde(default)]
        props: ZoneChanges,
    },
    /// Removes a zone with its properties and its fill.
    ZoneRemove {
        /// The zone.
        zone: Id,
    },
    /// Fills the zones with the crate `eh3d-phoenix-zones` and keeps the polygons of copper as data
    /// of the zones. All the zones are filled together (the priorities depend on each other); with
    /// a zone given only its fill is kept.
    ZoneFill {
        /// The zone whose fill is kept; every zone when left out.
        #[serde(default)]
        zone: Option<Id>,
    },
    /// Gives a zone a priority; a zone of a higher priority is filled first and the others keep
    /// away from it.
    ZonePriority {
        /// The zone.
        zone: Id,
        /// The priority.
        priority: i32,
    },
    /// Cuts a hole in a zone: the fill keeps away from the polygon, which must lie inside the
    /// outline.
    ZoneCutout {
        /// The zone.
        zone: Id,
        /// The polygon of the hole, at least three points.
        polygon: Vec<Point>,
    },
    /// Pastes a group of tracks, vias and zones (a copy of a net, or a clipboard made earlier,
    /// maybe from another board) with an offset, a turn by quarter turns and a mirror. The ids are
    /// new; the nets and layers are found by name. It is refused whenever the design rules find a
    /// new error, also when `reject_new_errors` is off.
    PasteGroup {
        /// The clipboard; left out when `from_net` is given.
        #[serde(default)]
        clipboard: Option<Clipboard>,
        /// A net of this board whose tracks, vias and zones are copied; left out when a clipboard
        /// is given.
        #[serde(default)]
        from_net: Option<Id>,
        /// Where the reference point of the group (the lower left corner of its box) goes.
        at: Point,
        /// Quarter turns counter-clockwise, from 0 to 3.
        #[serde(default)]
        quarter_turns: u8,
        /// Whether the group is mirrored about the vertical axis of the reference point (before
        /// the turn).
        #[serde(default)]
        mirror: bool,
        /// Names of nets to give the copies instead of their own, by name.
        #[serde(default)]
        net_map: BTreeMap<String, String>,
    },
    /// Several operations that are kept or undone as one step.
    Group {
        /// The operations, in order.
        ops: Vec<EditOp>,
    },
    /// An operation that is kept even if it adds a rule error (the report still says so).
    Forced {
        /// The operation.
        with: Box<EditOp>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_operation_is_written_and_read_back_the_same() {
        let text = r#"{"op":"forced","with":{"op":"group","ops":[{"op":"lock","id":"01000000-0000-7000-8000-000000000001"}]}}"#;
        let op: EditOp = serde_json::from_str(text).unwrap();
        let back = serde_json::to_string(&op).unwrap();
        assert_eq!(serde_json::from_str::<EditOp>(&back).unwrap(), op);
        assert!(matches!(op, EditOp::Forced { .. }));
    }

    #[test]
    fn the_push_and_the_arcs_are_read_with_their_defaults() {
        let text = r#"{"op":"route_shove","net":"01000000-0000-7000-8000-000000000001","layer":"01000000-0000-7000-8000-000000000002","points":[{"x_nm":0,"y_nm":0},{"x_nm":5,"y_nm":0}]}"#;
        match serde_json::from_str::<EditOp>(text).unwrap() {
            EditOp::RouteShove {
                shape,
                width_nm,
                horizontal_first,
                points,
                ..
            } => {
                assert_eq!((shape, width_nm, horizontal_first), (None, None, None));
                assert_eq!(points.len(), 2);
            }
            other => panic!("{other:?}"),
        }
        let text = r#"{"op":"round_corners","net":"01000000-0000-7000-8000-000000000001","radius_nm":800000}"#;
        match serde_json::from_str::<EditOp>(text).unwrap() {
            EditOp::RoundCorners {
                track, radius_nm, ..
            } => {
                assert_eq!(track, None);
                assert_eq!(radius_nm, 800_000);
            }
            other => panic!("{other:?}"),
        }
        let text = r#"{"op":"route_arc","track":"01000000-0000-7000-8000-000000000003","vertex":1,"radius_nm":500000}"#;
        let op: EditOp = serde_json::from_str(text).unwrap();
        let back = serde_json::to_string(&op).unwrap();
        assert_eq!(serde_json::from_str::<EditOp>(&back).unwrap(), op);
        assert!(serde_json::from_str::<EditOp>(
            r#"{"op":"route_arc","track":"01000000-0000-7000-8000-000000000003","vertex":1}"#
        )
        .is_err());
    }

    #[test]
    fn what_is_left_out_takes_its_default() {
        let text =
            r#"{"op":"tune_length","net":"01000000-0000-7000-8000-000000000001","target_nm":5}"#;
        let op: EditOp = serde_json::from_str(text).unwrap();
        match op {
            EditOp::TuneLength { track, side, .. } => {
                assert_eq!(track, None);
                assert_eq!(side, MeanderSide::Auto);
            }
            other => panic!("{other:?}"),
        }
        assert!(serde_json::from_str::<EditOp>(r#"{"op":"fly"}"#).is_err());
    }
}
