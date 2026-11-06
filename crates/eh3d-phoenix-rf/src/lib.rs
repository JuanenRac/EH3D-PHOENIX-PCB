// SPDX-License-Identifier: GPL-3.0-or-later
//! Everyday RF tools for EH3D-PHOENIX-PCB.
//!
//! * [`complex`] and [`matrix`]: a complex number of this crate and square complex matrices;
//! * [`touchstone`]: the reader and the writer of Touchstone files (`.s1p`, `.s2p`, `.sNp`);
//! * [`network`]: two-port parameters (S, Z, Y, ABCD), the conversions between them, the
//!   cascade, and the figures read from them (reflection, VSWR, return and insertion loss,
//!   group delay, gain, stability, passivity, reciprocity);
//! * [`matching`]: L networks, single stubs and quarter-wave transformers for complex
//!   impedances, with component values and electrical lengths;
//! * [`smith`]: the coordinates of the Smith chart for a point or a sweep, and its circles;
//! * [`filter`]: Butterworth, Chebyshev and Bessel prototypes, their transformation into
//!   low-pass, high-pass, band-pass and band-stop ladders and their response from ABCD matrices;
//! * [`line`]: the link to the transmission lines of `eh3d-phoenix-stackup`;
//! * [`matchnet`]: pi and T matching networks with a chosen loaded Q, the bandwidth of a
//!   network, the double stub and the equivalents with lines, with lengths in metres;
//! * [`trace`]: the path of a matching network on the Smith chart as data and as a small SVG;
//! * [`eseries`] and [`filterx`]: the series E12, E24 and E96, a filter with components of a
//!   series and the effect of the rounding on the band edges;
//! * [`netdesc`] and [`mask`]: a network of lumped elements and lines written as text, its S
//!   parameters and figures over a sweep, and the check of a response against a mask;
//! * [`circles`]: stability circles and the simultaneous conjugate match from S parameters;
//! * [`report`]: the text of `phoenix rf`;
//! * [`settings`]: the limits and tolerances, read from `config/rf.cfg`;
//! * [`error`]: typed errors with the codes `PHOENIX-RF-001` to `PHOENIX-RF-013` and their
//!   titles in the eight languages of the suite.
//!
//! **State: prototype.** The formulas are the published ones and the tests use values of the
//! textbooks, but nothing here has been compared with a network analyser, with a field solver or
//! with another RF program, and the filters are ideal (no loss, no parasitics, no tolerance of
//! the components).
//!
//! Nothing here opens a file, a window or the network, and nothing panics on its input.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod circles;
pub mod complex;
pub mod error;
pub mod eseries;
pub mod filter;
pub mod filterx;
pub mod line;
pub mod mask;
pub mod matching;
pub mod matchnet;
pub mod matrix;
pub mod netdesc;
pub mod network;
pub mod report;
pub mod settings;
pub mod smith;
pub mod touchstone;
pub mod trace;

pub use complex::Complex;
pub use error::{Language, RfError};
pub use matrix::CMatrix;
pub use network::Mat2;
pub use settings::RfSettings;
pub use touchstone::Touchstone;
