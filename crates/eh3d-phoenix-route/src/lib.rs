// SPDX-License-Identifier: GPL-3.0-or-later
//! The editing and routing logic of the board, without an interface.
//!
//! * [`ratsnest`]: the connections still to be routed, a minimum spanning tree per net over the
//!   islands that the connectivity of the board finds, always in the same order;
//! * [`router`]: an interactive router for tracks at 0, 45 and 90 degrees. It stops in front of the
//!   first obstacle or goes around it on a coarse mesh with the fewest corners, takes the width of
//!   the track and the size of the vias from the rules and the net class, changes layer with a via
//!   when that is needed, and never returns a track that the rules refuse: a plan is judged on a
//!   copy of the board and an impossible route is an error with a code;
//! * [`edit`]: moving and turning components, deleting tracks, moving vertices, changing widths and
//!   adding vias as reversible operations of the board, and an editor with undo and redo;
//! * [`live`]: the proven findings of the rules and the difference between two boards, so that an
//!   edit reports what it adds and removes;
//! * [`settings`] and [`codes`]: every pitch, penalty and limit, read from `config/route.cfg`, and
//!   the codes `PHOENIX-ROUTE-001` to `PHOENIX-ROUTE-020` with their titles in the eight
//!   languages.
//!
//! **State: prototype.** It has been tried on boards made by hand and by the tests only. Pads are
//! avoided as the smallest circle around them, zones are not routed around and tracks have no
//! arcs; the connections it counts are those of the connectivity of the board, with no tolerance.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod edit;
mod geom;
pub mod live;
pub mod ratsnest;
pub mod router;
pub mod settings;
mod space;

pub use codes::RouteError;
pub use edit::{
    add_via, delete_track, move_component, move_vertex, rotate_component, set_track_width,
    via_for_layer_change, Editor,
};
pub use live::{diff, findings, LiveDiff, RuleContext};
pub use ratsnest::{item_label, ratsnest, RatLine};
pub use router::{
    copper_layers, route, track_width, via_size, Element, Mode, RoutePlan, RouteRequest,
};
pub use settings::RouteSettings;
