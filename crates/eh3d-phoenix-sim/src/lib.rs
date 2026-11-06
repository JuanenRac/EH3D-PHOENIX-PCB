// SPDX-License-Identifier: GPL-3.0-or-later
//! A circuit simulator of our own for EH3D-PHOENIX-PCB: linear circuits and non-linear ones
//! (diodes with series resistance, junction capacitance and breakdown, bipolar transistors,
//! MOSFETs of level 1, voltage-controlled switches, operational-amplifier macromodels and
//! comparators), solved by modified nodal analysis. It links no engine and has no dependency
//! beyond the complex numbers of the RF crate.
//!
//! * [`units`], [`expr`], [`netlist`], [`parse`] and [`write`]: the netlist in memory, and the
//!   reader and writer of SPICE-style text (`R C L V I E G F H K D Q M S U X`, `.param`, `.model`,
//!   `.subckt`, the analysis directives `.op .dc .tran .ac`; the suffixes `f p n u m k meg g t
//!   mil`), with typed errors that say the line;
//! * [`mod@include`]: a netlist read from a file, with `.include` of files in the same folder and a
//!   separate file of models;
//! * [`circuit`]: the subcircuits expanded, the nodes numbered, the elements sorted by kind, the
//!   check for nodes with no path to ground;
//! * [`linalg`]: Gaussian elimination with partial pivoting, real and complex, that names the
//!   unknown a singular matrix leaves undetermined;
//! * [`real`], [`dc`], [`tran`]: the stamps, the device equations, the Newton-Raphson iteration,
//!   the operating point (with gmin stepping and source stepping as fallbacks, and a report of
//!   the iterations and the residual), the DC sweep of a source and the transient
//!   analysis (backward Euler and trapezoidal, fixed step or adaptive step that stops at the
//!   corners of the sources, initial conditions);
//! * [`ac`] and [`noise`]: the small-signal sweep (linear, per decade, per octave) with magnitude,
//!   phase and group delay, and the noise at an output (thermal and shot noise);
//! * [`export`]: the netlist of a schematic, with a report of what could not be translated;
//! * [`schgen`], [`modeltable`], [`partvalue`] and [`netcodes`]: the netlist made from a schematic
//!   with no netlist written by hand: the model of each part from a table (`config/sim_models.cfg`)
//!   and from the attributes of the part, values written as `4k7` or `1u5`, stable names for the
//!   nets, subcircuits from `.include` files of one folder, the operating point net by net, and a
//!   report with the codes `PHOENIX-NETLIST-001` to `012` in the eight languages;
//! * [`sketch`]: a small schematic written as a few lines of text, for tests and reference cases;
//! * [`filter`]: the coherence check of a ladder filter of the RF crate against its simulation;
//! * [`output`]: deterministic CSV and JSON tables;
//! * [`settings`] and [`codes`]: every limit and tolerance, read from `config/sim.cfg`, and the
//!   codes `PHOENIX-SIM-001` to `PHOENIX-SIM-024` with their titles in the eight languages.
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
pub mod include;
pub mod linalg;
pub mod modeltable;
pub mod netcodes;
pub mod netlist;
pub mod noise;
pub mod output;
pub mod parse;
pub mod partvalue;
pub mod real;
pub mod schgen;
pub mod settings;
pub mod sketch;
pub mod tran;
pub mod units;
pub mod write;

pub use ac::{ac_analysis, ac_at, AcRequest, AcResult, SweepKind};
pub use circuit::Circuit;
pub use codes::SimError;
pub use dc::{dc_sweep, operating_point, DcRequest, DcResult, OpResult};
pub use export::{netlist_from_schematic, netlist_from_schematic_json, ExportReport};
pub use filter::{compare_filter, Coherence};
pub use include::{read_netlist_file, read_netlist_text};
pub use modeltable::{Device, ModelTable, PartProps, Rule};
pub use netcodes::GenIssue;
pub use netlist::{Directive, Netlist};
pub use noise::{noise_analysis, NoiseContribution, NoiseResult};
pub use parse::parse;
pub use real::Method;
pub use schgen::{
    generate_netlist, generate_netlist_hier, operating_table, read_generated, MeterReading,
    NetlistReport, NetlistText, NodeVoltage, OpTable, SourceCurrent,
};
pub use settings::SimSettings;
pub use sketch::schematic_from_sketch;
pub use tran::{transient, TranRequest, TranResult};
pub use write::to_spice;
