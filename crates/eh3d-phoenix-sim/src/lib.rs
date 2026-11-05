// SPDX-License-Identifier: GPL-3.0-or-later
//! A circuit simulator of our own for EH3D-PHOENIX-PCB: linear circuits and non-linear ones
//! (diodes with series resistance, junction capacitance and breakdown, bipolar transistors,
//! MOSFETs of level 1, operational-amplifier macromodels and comparators), solved by modified
//! nodal analysis. It links no engine and has no dependency beyond the complex numbers of the RF
//! crate.
//!
//! * [`units`], [`expr`], [`netlist`], [`parse`] and [`write`]: the netlist in memory, and the
//!   reader and writer of SPICE-style text (`R C L V I E G F H K D Q M U X`, `.param`, `.model`,
//!   `.subckt`; the suffixes `f p n u m k meg g t mil`), with typed errors that say the line;
//! * [`circuit`]: the subcircuits expanded, the nodes numbered, the elements sorted by kind, the
//!   check for nodes with no path to ground;
//! * [`linalg`]: Gaussian elimination with partial pivoting, real and complex, that names the
//!   unknown a singular matrix leaves undetermined;
//! * [`real`], [`dc`], [`tran`]: the stamps, the device equations, the Newton-Raphson iteration,
//!   the operating point (with gmin stepping and source stepping as fallbacks) and the transient
//!   analysis (backward Euler and trapezoidal, fixed step or adaptive step that stops at the
//!   corners of the sources, initial conditions);
//! * [`ac`] and [`noise`]: the small-signal sweep (linear, per decade, per octave) with magnitude,
//!   phase and group delay, and the noise at an output (thermal and shot noise);
//! * [`export`]: the netlist of a schematic, with a report of what could not be translated;
//! * [`filter`]: the coherence check of a ladder filter of the RF crate against its simulation;
//! * [`output`]: deterministic CSV and JSON tables;
//! * [`settings`] and [`codes`]: every limit and tolerance, read from `config/sim.cfg`, and the
//!   codes `PHOENIX-SIM-001` to `PHOENIX-SIM-022` with their titles in the eight languages.
//!
//! **State: prototype.** The results are an own numerical approximation: the tests use values of
//! the textbooks and properties, but nothing has been compared with SPICE or with a measurement.
//! The device models are simple (no high-injection effects, no temperature dependence, no
//! flicker noise) and the op-amp is a macromodel.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ac;
pub mod circuit;
pub mod codes;
pub mod dc;
pub mod export;
pub mod expr;
pub mod filter;
pub mod linalg;
pub mod netlist;
pub mod noise;
pub mod output;
pub mod parse;
pub mod real;
pub mod settings;
pub mod tran;
pub mod units;
pub mod write;

pub use ac::{ac_analysis, ac_at, AcRequest, AcResult, SweepKind};
pub use circuit::Circuit;
pub use codes::SimError;
pub use dc::{operating_point, OpResult};
pub use export::{netlist_from_schematic, netlist_from_schematic_json, ExportReport};
pub use filter::{compare_filter, Coherence};
pub use netlist::Netlist;
pub use noise::{noise_analysis, NoiseContribution, NoiseResult};
pub use parse::parse;
pub use real::Method;
pub use settings::SimSettings;
pub use tran::{transient, TranRequest, TranResult};
pub use write::to_spice;
