// SPDX-License-Identifier: GPL-3.0-or-later
//! A spatial index shared by the checks of a board.
//!
//! * [`grid`]: a uniform grid over boxes in integer nanometres. A query gives the items that may
//!   touch a box, in ascending order and without repeats; every caller applies its own exact test
//!   to them, so the result is the one of the search that compares every pair. The index counts
//!   the candidates it hands out, which lets a test tell linear work from quadratic work without
//!   a clock;
//! * [`config`]: `config/spatial.cfg`, with the size of a cell (automatic when it is 0) and the
//!   limits on cells and items;
//! * [`codes`]: the codes `PHOENIX-SPATIAL-001` to `005` with their titles in the eight languages;
//! * [`testgen`]: a seeded generator of boxes, for the tests that compare the index with the
//!   search of every pair.
//!
//! All arithmetic is in `i128`, so the extremes of `i64` do not overflow. **State: prototype.**
//! The index has been compared with the search of every pair on boxes made by the tests, not on
//! boards of real designs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod config;
pub mod grid;
pub mod testgen;

pub use codes::{registry, title, SpatialError, LANGUAGES};
pub use config::SpatialConfig;
pub use grid::{inflate, overlap, pairs, Bx, Index};
