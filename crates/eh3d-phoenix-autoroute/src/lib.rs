// SPDX-License-Identifier: GPL-3.0-or-later
//! A simple autorouter that needs no outside program.
//!
//! * [`config`]: `config/autoroute_native.cfg`, with strict keys: the pitch of the grid, the order
//!   of the nets, the costs of the search and every limit;
//! * `obstacles`: a map of what a track has to keep away from on each copper layer (pads, vias,
//!   tracks, the board edge and the zones of other nets) and of the drill holes, with the copper
//!   that was on the board in a spatial index (`eh3d-phoenix-spatial`) and the copper that the run
//!   puts down in a map of squares that can take a route away again;
//! * `search`: A* over the nodes of the grid, layer by layer, with vias between layers, a cost for
//!   corners, vias and steps against the preferred direction of a layer, tracks at 0, 45 and 90
//!   degrees, and an order of expansion that never depends on anything but the input;
//! * [`engine`]: the nets in the order of the settings, rip-up and reroute with limits counted in
//!   nodes and passes, and the plan;
//! * [`plan`]: the result as one reversible operation of the board (also as a group of operations
//!   of the editor), judged by the design rules before it is handed out, and a report that is the
//!   same for the same input;
//! * [`codes`]: the codes `PHOENIX-AROUTE-001` to `016` with their titles in the eight languages.
//!
//! All arithmetic is integer (nanometres, with 128 bits in the products). The same board, rules
//! and settings always give the same plan, byte for byte. Tracks that are already on the board
//! are never changed: the plan only adds. A differential pair is routed as two ordinary nets;
//! whether the pair is kept together is for the pair check after the run.
//!
//! **State: prototype.** It has been tried on small boards made by hand and by a generator of
//! the tests, not on boards of real designs. It uses vias that join two copper layers of the
//! stack, puts no teardrops and no arcs, and does not pour zones.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod config;
pub mod engine;
mod geom;
mod obstacles;
pub mod plan;
mod search;

#[cfg(test)]
mod testboard;
#[cfg(test)]
mod tests_cases;
#[cfg(test)]
mod tests_props;

pub use codes::{registry, title, AutorouteError, LANGUAGES};
pub use config::{AutorouteConfig, NetOrder};
pub use engine::{autoroute, Request};
pub use plan::{AutoroutePlan, PlannedTrack, PlannedVia, Report, Unrouted};
