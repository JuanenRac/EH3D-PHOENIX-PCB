// SPDX-License-Identifier: GPL-3.0-or-later
//! The own corpus: small boards, schematics, libraries and manufacturing files written by hand
//! in every format that the suite reads or writes, each with an expectation worked out by hand,
//! and the runner that reads them with the real readers and writers and compares.
//!
//! * [`entry`]: the expectation file (`phoenix-corpus/1`): which file, which reader, what must
//!   come out and how the numbers were worked out;
//! * [`read`]: the readers and writers that run on one piece (EAGLE, KiCad, Gerber and Excellon,
//!   the board as JSON, the autorouter files, the SPICE netlist);
//! * [`trips`]: the round trips between formats, compared by counts and by positions with a
//!   tolerance in nanometres;
//! * [`props`]: properties on small boards that a generator of our own makes from a seed;
//! * [`run`]: the walk over the folder, the check, the list and the report that is the same
//!   every time;
//! * [`codes`] and [`settings`]: the codes `PHOENIX-CORPUS-001` to `PHOENIX-CORPUS-012` with their
//!   titles in the eight languages, and the limits of `config/corpus.cfg`.
//!
//! **State: prototype.** The corpus is written for this repository: it holds no file of a maker
//! or of a user, and passing it shows that the readers and the writers agree with what was worked
//! out by hand for these small pieces, not that they agree with EAGLE, KiCad or a CAM program.

pub mod codes;
pub mod entry;
pub mod facts;
pub mod outcome;
pub mod props;
pub mod read;
pub mod rng;
pub mod run;
pub mod settings;
pub mod trips;

pub use codes::{title, SuiteError, TITLES};
pub use entry::{Entry, Expect, KINDS};
pub use outcome::{judge, Judgement, Outcome, Verdict};
pub use props::{random_board, PropertyResult, PROPERTIES};
pub use run::{check, list, load, Corpus, EntryResult, Loaded, Report};
pub use settings::CorpusSettings;
