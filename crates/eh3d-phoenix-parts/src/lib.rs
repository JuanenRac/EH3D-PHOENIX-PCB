// SPDX-License-Identifier: GPL-3.0-or-later
//! The local parts base. The components the user uses are kept in files of the user, JSON
//! (`phoenix-parts/1`) and CSV, in one folder; nothing is ever fetched from a network.
//!
//! * [`model`]: a part (value, tolerance, power, voltage, footprint, symbol, maker, part number,
//!   free parameters, simulation model, suppliers with prices by quantity break, state, notes and
//!   alternatives);
//! * [`read`] and [`write`]: reading the files with the line of every fault and the limits of the
//!   settings, and writing them in the same order every time; a file is only written when it does
//!   not exist yet;
//! * [`index`]: the lookups by id, part number and value, and the search by text with filters;
//! * [`check`]: the checker of the base (footprints that do not exist, part numbers with other
//!   data, prices without a currency, price breaks out of order, models and alternatives that do
//!   not exist, ids used twice);
//! * [`link`]: the link to the bill of materials, by value and footprint or by part number, with
//!   the report of the parts without a match or with several, and the proposals of alternatives
//!   for the parts that are obsolete;
//! * [`report`]: the text and the JSON of the commands;
//! * [`settings`] and [`codes`]: the limits, read from `config/parts.cfg`, and the codes
//!   `PHOENIX-PARTS-001` to `018` with their titles in the eight languages.
//!
//! **State: prototype.** It has been tried on small bases written by the tests only. The prices
//! are the ones of the files of the user, with no check that they are current; the footprints are
//! checked only against the names the caller knows.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod check;
pub mod codes;
pub mod index;
pub mod link;
pub mod model;
pub mod read;
pub mod report;
pub mod settings;
pub mod write;

pub use check::{check_db, CheckContext};
pub use codes::{title, Finding, PartsError, Severity};
pub use index::{Found, Index, Query};
pub use link::{
    entries_of, link_bom, LinkEntry, LinkLine, LinkReport, Linked, Outcome, Proposal, Via,
};
pub use model::{norm_value, Offer, Part, PartsDb, Status, Tier, FORMAT};
pub use read::{load, parse_any, parse_csv, parse_json};
pub use settings::PartsSettings;
pub use write::{to_csv, to_json, write_new};
