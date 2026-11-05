// SPDX-License-Identifier: GPL-3.0-or-later
//! Board stack-up and transmission-line calculations for EH3D-PHOENIX-PCB.
//!
//! * [`stackup`]: the layers of a board (copper in ounces or micrometres with a roughness,
//!   cores, prepregs and solder mask), integer nanometres for every thickness, checked and
//!   serialisable;
//! * [`material`]: the dielectric constant and the loss tangent of a material, interpolated
//!   between the points a data sheet publishes and never extrapolated;
//! * [`models`]: the closed-form quasi-static models (microstrip by Hammerstad and Jensen,
//!   stripline by Cohn and Wadell, coplanar waveguide with and without a ground plane by Simons,
//!   coupled pairs by the fits of IPC-2141), each documented with its source and its range;
//! * [`line`]: a line taken from a stack-up, its impedance, effective permittivity, conductor and
//!   dielectric loss in dB/cm, delay and guided wavelength at a frequency, and the synthesis of a
//!   track width from a target impedance;
//! * [`job`]: the job file and the reports of `phoenix stackup`;
//! * [`settings`]: the tuned values, read from `config/stackup.cfg`;
//! * [`error`]: typed errors and warnings with the codes `PHOENIX-STACK-001` to
//!   `PHOENIX-STACK-016` and their titles in the eight languages of the suite.
//!
//! **State: prototype.** The formulas are published ones, applied here with the checks of their
//! range, but nothing in this crate has been compared with a field solver or with a measurement
//! of a real board. A result is an estimate of the quasi-static line, not a simulation.
//!
//! Nothing here opens a file, a window or the network, and nothing panics on its input.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod elliptic;
pub mod error;
pub mod job;
pub mod line;
pub mod material;
#[cfg(feature = "materials")]
pub mod materials_adapter;
pub mod models;
pub mod settings;
pub mod stackup;

pub use error::{Language, StackupError, Warning};
pub use job::{report_json, report_text, LineOutcome, StackupJob};
pub use line::{analyze, synthesize_width, LineKind, LineResult, LineSpec, Synthesis};
pub use material::MaterialSample;
pub use settings::StackupSettings;
pub use stackup::{CopperThickness, Layer, Stackup};
