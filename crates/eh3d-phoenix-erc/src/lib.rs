// SPDX-License-Identifier: GPL-3.0-or-later
//! The electrical rule check of a schematic.
//!
//! * [`config`]: how serious each rule is, the matrix of pin types, the pin types that drive a net
//!   or supply a power input and the bounds, read from `config/erc.cfg` (which is also the
//!   built-in default); no value is written in the checks;
//! * [`checks`]: the checks in a fixed order: pins that are on no net, nets with a single pin, pins
//!   of types the matrix does not allow together, inputs with no driver, power inputs with no
//!   source, net names that look alike, reference designators used twice or empty, empty values,
//!   parts without a footprint, symbol pins without a number or with a repeated one, and wires and
//!   junctions that join no pin;
//! * [`report`]: the report as text in eight languages and as JSON, with the proven findings
//!   (`confirmed`) and what could not be judged (`inconclusive`) in two lists, in a fixed order;
//! * [`codes`]: the stable codes `PHOENIX-ERC-<NNN>` with their titles in the eight languages.
//!
//! The pin types come from the schematic (`Schematic::pin_kinds`); a pin without one is left out
//! of the checks that need it and the report says so. The rules and the matrix have been tried on
//! schematics made by hand and by the tests; they have not been compared with the reference tool.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod checks;
pub mod codes;
pub mod config;
pub mod report;

pub use checks::check;
pub use codes::{title, Rule, REGISTRY};
pub use config::{ErcConfig, Level, SettingsError};
pub use report::{Finding, Report};
