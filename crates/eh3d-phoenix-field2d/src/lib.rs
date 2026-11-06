// SPDX-License-Identifier: GPL-3.0-or-later
//! A two-dimensional field solver for the cross section of a transmission line, for
//! EH3D-PHOENIX-PCB.
//!
//! * [`mesh`]: the lines of a non-uniform rectangular mesh, fine at the edges of the conductors
//!   and growing by a ratio away from them;
//! * [`grid`]: the grid with a permittivity, a loss tangent and a conductor number in each cell,
//!   the finite-volume form of the Laplace equation, its solution by conjugate gradients or
//!   successive over-relaxation with limits of iterations and cells, and the capacitance matrix;
//! * [`shape`] and [`section`]: a cross section as rectangles, and the section of a line taken
//!   from a stack-up with the permittivity of each layer at the frequency;
//! * [`analysis`]: from the capacitance with and without the dielectric the impedance and
//!   effective permittivity, the inductance, the coupling of a pair (`Cm`, `Lm`), the
//!   differential and common impedance, the dielectric loss and the comparison with the closed
//!   formulas of the stack-up crate;
//! * [`report`]: the text and JSON reports;
//! * [`settings`] and [`codes`]: the tuned values, read from `config/field2d.cfg`, and the codes
//!   `PHOENIX-FIELD-001` to `PHOENIX-FIELD-013` with their titles in the eight languages.
//!
//! **State: prototype.** The solver is checked against solutions that are known exactly (parallel
//! plates, capacitors in series), against the coaxial line of circular conductors drawn as steps
//! of the mesh, against the stripline of Cohn and against the microstrip formula of Hammerstad
//! and Jensen, and its error is seen to fall as the mesh is refined; it has not been compared
//! with another field solver or with a measurement. The box that closes the open side is an
//! approximation, the copper loss is not computed, the solder mask is left out, and the
//! conductors are rectangles.
//!
//! Every sum runs in a fixed order over `f64`, so the same input gives the same bytes. Nothing
//! here opens a file, a window or the network, and nothing panics on its input.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod analysis;
pub mod codes;
pub mod grid;
pub mod mesh;
pub mod report;
pub mod section;
pub mod settings;
pub mod shape;

pub use analysis::{
    analyze_shape, run_job, solve_line, CaseOutcome, Comparison, FieldResult, LineField, PairResult,
};
pub use codes::{title, FieldError};
pub use grid::{Grid, Rect, Solution, Wall, Walls};
pub use report::{report_json, report_text, report_value};
pub use section::{section_from_stackup, Note, Section};
pub use settings::{Field2dSettings, Method};
pub use shape::{Block, Electrode, Shape};

use eh3d_phoenix_stackup::models::{C0 as SPEED_OF_LIGHT, MU0};

/// The speed of light in vacuum, in m/s.
pub const C0: f64 = SPEED_OF_LIGHT;
/// The permittivity of vacuum, in F/m, `1 / (mu0 c0^2)`, with the `mu0` of the stack-up crate.
pub const EPS0: f64 = 1.0 / (MU0 * SPEED_OF_LIGHT * SPEED_OF_LIGHT);
