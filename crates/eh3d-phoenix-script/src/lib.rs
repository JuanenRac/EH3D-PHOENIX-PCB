// SPDX-License-Identifier: GPL-3.0-or-later
//! The script interpreter: the command language of the old workflow, built from its public
//! description.
//!
//! * [`parse`]: the reader of script files (commands ended by `;` or a line break, `#` comments,
//!   texts in single quotes, coordinates in parentheses, lengths with a unit);
//! * [`values`]: lengths with the units `mm`, `mil`, `inch`, `micron` and `grid`, and rotations;
//! * [`actions`]: what each command means, shared by the run and by the report, so both agree on
//!   which forms are understood (`GRID`, `LAYER`, `DISPLAY`, `CHANGE`, `WIRE`, `VIA`, `CIRCLE`,
//!   `RECT`, `TEXT`, `MOVE`, `ROTATE`, `DELETE`, `NAME`, `VALUE`, `SET`, `GROUP`, `RATSNEST`, `DRC`,
//!   `ERC`, `WRITE`, `OPEN` and `SCRIPT`);
//! * [`exec`]: the run, which changes a copy of the board only through the reversible operations of
//!   the board model, all or nothing, with a batch that undoes it byte for byte;
//! * [`report`]: the compatibility report, per command, per script and per folder, computed without
//!   running anything, as text, JSON and one HTML page;
//! * [`cam`]: output jobs read into a profile of the export;
//! * [`files`]: the only way a script reaches a file: relative paths below the folder of the
//!   script, inside the folders that the settings allow;
//! * [`settings`] and [`codes`]: every limit, default and permission, read from `config/script.cfg`,
//!   and the codes `PHOENIX-SCRIPT-<NNN>` with their titles in the eight languages.
//!
//! * [`v2`]: the scripts of the second version, `phoenix-script/2`, a small language with variables,
//!   bounded loops and conditions on measures (counts, lengths, findings of the rule checks) that
//!   selects objects of a board, a schematic or a library by a query and changes them through the
//!   same reversible operations as the editors; a whole script is one step of the history, and it
//!   runs in a sandbox with limits of steps, memory and output and no access to the network, to a
//!   program or to a file but the data folder of the settings. `phoenix script check` reads it
//!   without running it, `phoenix script list` shows the examples of `examples/scripts/` and
//!   `phoenix script help` prints the reference written from the table of functions.
//!
//! A script never runs a program, never reaches an absolute path or a path with `..`, and is held
//! to the limits of the settings; a command it does not know is a diagnostic with its line and
//! column. The commands were written from the public description of the language and from nothing
//! else; the result has not been compared with the original tool, and no script of a third party
//! was used. **State: prototype.**
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod actions;
pub mod cam;
pub mod codes;
pub mod exec;
pub mod files;
pub mod parse;
pub mod report;
pub mod settings;
pub mod v2;
pub mod values;

pub use actions::{classify, interpret, Action, Status};
pub use cam::{import_cam, CamImport};
pub use codes::Diag;
pub use exec::{commit_writes, run_board, run_schematic, Run, RunContext};
pub use parse::{parse_script, Parsed};
pub use report::{analyse_path, analyse_text, Folder, ScriptReport};
pub use settings::ScriptSettings;
pub use v2::check::{check_text, CheckReport};
pub use v2::interp::{run as run_v2, Doc, Outcome, PendingFile, Plan};
pub use v2::syntax::{parse_program, version_of, Program, Target, Version};

#[cfg(test)]
mod testdata;

#[cfg(test)]
mod exec_tests;
