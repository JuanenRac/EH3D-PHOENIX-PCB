// SPDX-License-Identifier: GPL-3.0-or-later
//! The scripts of the second version, `phoenix-script/2`: a small language with variables, bounded
//! loops and conditions that works on a board, a schematic or a library through the same
//! reversible operations as the editors.
//!
//! * [`syntax`]: the header, the tokens, the tree and the reader, with line and column in every
//!   error;
//! * [`interp`]: the run, which works on a copy of the document, counts steps, memory, output,
//!   loop turns and operations against the limits of `config/script.cfg`, and ends with one batch
//!   that is one step of the history, or with nothing at all;
//! * [`check`]: the static check of a script (syntax, names, number of arguments, the documents a
//!   function applies to, an estimate of the steps) without running it;
//! * [`help`]: the reference of the language, written from the one table of functions;
//! * [`templates`]: the example scripts of `examples/scripts/` that `phoenix script list` shows;
//! * [`codes`]: the codes `PHOENIX-SCRIPT-043` onwards with their titles in the eight languages.
//!
//! The version 1 of the language (the commands of the old workflow, `docs/SCRIPT.md`) is still read
//! and run as before; a script is of the second version when its first significant line is
//! `phoenix-script/2`. A script has no access to the network, to a program or to a file but the
//! data folder of the settings, and the run is held to limits of steps, memory and output.

pub mod check;
pub mod codes;
pub(crate) mod funcs;
pub(crate) mod funcs_board;
pub(crate) mod funcs_core;
pub(crate) mod funcs_doc;
pub(crate) mod funcs_io;
pub(crate) mod funcs_sch;
pub mod help;
pub mod interp;
pub mod report;
pub mod syntax;
pub mod templates;
pub mod value;
pub(crate) mod world;

#[cfg(test)]
mod tests;
