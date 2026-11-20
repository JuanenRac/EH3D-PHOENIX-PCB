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
//! * [`heating`]: the temperature rise of a track or a via that carries a current, by the fit of
//!   IPC-2221 and by the convection model of EH3D-THERMAL-LAB, with the codes `PHOENIX-HEAT-001`
//!   to `PHOENIX-HEAT-012`;
//! * [`job`]: the job file and the reports of `phoenix stackup`;
//! * [`laminates`]: a small library of laminates (epoxy, high-frequency, polyimide, metal base,
//!   copper) with the figures their data sheets state, none of them verified, and a figure that
//!   the sheet does not state clearly left undefined;
//! * [`settings`]: the tuned values, read from `config/stackup.cfg`;
//! * [`report_html`]: the layers of a stack-up as a page of HTML in the eight languages of the
//!   suite, with the depth at which each starts and the total thickness;
//! * [`spread`]: the lowest and the highest impedance of a line when the width, the height of the
//!   dielectric and the permittivity are each off by a stated amount;
//! * [`spread_report`]: the spread and the largest tolerance of the width as text in the eight
//!   languages of the suite;
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
pub mod heating;
pub mod job;
pub mod laminates;
pub mod line;
pub mod material;
#[cfg(feature = "materials")]
pub mod materials_adapter;
pub mod models;
#[cfg(test)]
mod property_tests;
pub mod report_html;
pub mod settings;
pub mod spread;
pub mod spread_report;
pub mod stackup;

pub use error::{Language, StackupError, Warning};
pub use job::{report_json, report_text, LineOutcome, StackupJob};
pub use laminates::{Family, Laminate, LAMINATES};
pub use line::{analyze, synthesize_width, LineKind, LineResult, LineSpec, Synthesis};
pub use material::MaterialSample;
pub use report_html::{stackup_html, stackup_rows, StackRow};
pub use settings::StackupSettings;
pub use spread_report::{max_width_tolerance_text, spread_text};
pub use stackup::{CopperThickness, Layer, Stackup};
