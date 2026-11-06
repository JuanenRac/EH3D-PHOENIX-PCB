// SPDX-License-Identifier: GPL-3.0-or-later
//! Reference cases for the calculations of EH3D-PHOENIX-PCB.
//!
//! Everything the suite computes is a prototype until it is compared with a reference. This crate
//! is the frame of that comparison, and it works without any program that is not in this repository:
//!
//! * [`case`]: the files of cases (`phoenix-oracle/1`, kept in `tests/oracle/`), each case with the
//!   value that an open source publishes (or that follows exactly from a definition), its unit, its
//!   tolerance and who chose it, and the source with its place in it; a case whose value cannot be
//!   cited with certainty is *pending* and carries the reason, never a number;
//! * [`compute`]: the checks, each one a call of the real functions of the stack-up, RF, simulator,
//!   rules, lengths and zones crates with the inputs of the case;
//! * [`run`]: the run and the report (ok, outside the tolerance, pending, failed), the same every
//!   time in text and in JSON;
//! * [`bundled`]: the case files built into the program, or the files of a folder;
//! * [`codes`]: the codes `PHOENIX-ORACLE-001` to `PHOENIX-ORACLE-010` with their titles in the
//!   eight languages.
//!
//! **State: prototype.** Passing these cases shows that a calculation reproduces a published
//! formula or a closed-form solution within the tolerance written next to it. It does not show that
//! the calculation agrees with a field solver, a circuit simulator of another maker, EAGLE, KiCad
//! or a measurement: that comparison is planned in `docs/ORACLE.md` and has not been made.
//!
//! Nothing here opens a window or the network, starts another program or writes a file.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bundled;
mod calc_board;
mod calc_line;
mod calc_netlist;
mod calc_si;
mod calc_sim;
pub mod case;
pub mod codes;
pub mod compute;
pub mod run;

pub use bundled::{bundled_files, read_dir, BUNDLED};
pub use case::{Case, CaseFile, Status, Tolerance, FORMAT};
pub use codes::{title, OracleError, TITLES};
pub use compute::{compute, CHECKS};
pub use run::{list_cases, run_case, run_files, CaseResult, Counts, Report, Verdict};
