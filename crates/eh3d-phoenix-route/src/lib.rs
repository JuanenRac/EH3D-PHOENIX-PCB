// SPDX-License-Identifier: GPL-3.0-or-later
//! The editing and routing logic of the board, without an interface.
//!
//! * `ratsnest`: the connections still to be routed, a minimum spanning tree per net over the
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
//! * [`dsn`], [`ses`] and [`autorouter`]: routing by an external autorouter through files. The
//!   board is exported as a Specctra DSN file, a Specctra session returned by the program is
//!   read as a plan that is checked against the rules and applied all or nothing, and an optional
//!   adapter, off unless its settings in `config/autorouter.cfg` switch it on, can start the
//!   program;
//! * [`settings`] and [`codes`]: every pitch, penalty and limit, read from `config/route.cfg`, and
//!   the codes `PHOENIX-ROUTE-001` to `PHOENIX-ROUTE-028` with their titles in the eight
//!   languages.
//!
//! **State: prototype.** It has been tried on boards made by hand and by the tests only. Pads are
//! avoided as the smallest circle around them, zones are not routed around and tracks have no
//! arcs; the connections it counts are those of the connectivity of the board, with no tolerance.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod autorouter;
pub mod codes;
pub mod dsn;
pub mod edit;
mod geom;
pub mod live;
pub mod ratsnest;
pub mod router;
pub mod ses;
pub mod settings;
mod sexpr;
mod space;

pub use autorouter::{launch_plan, run_name, AutorouterConfig, Launch};
pub use codes::RouteError;
pub use dsn::{export as export_dsn, Dsn};
pub use edit::{
    add_via, delete_track, move_component, move_vertex, rotate_component, set_track_width,
    via_for_layer_change, Editor,
};
pub use live::{diff, findings, LiveDiff, RuleContext};
pub use ratsnest::{item_label, ratsnest, ratsnest_with, RatLine, RatsnestWork};
pub use router::{
    copper_layers, route, track_width, via_size, Element, Mode, RoutePlan, RouteRequest,
};
pub use ses::{import as import_ses, SesPlan};
pub use settings::RouteSettings;
