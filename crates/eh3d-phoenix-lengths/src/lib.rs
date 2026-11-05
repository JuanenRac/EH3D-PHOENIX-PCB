// SPDX-License-Identifier: GPL-3.0-or-later
//! Lengths of nets, groups that must match and differential pairs.
//!
//! * [`measure`]: the copper length of each net (tracks and vias, straight pieces to the nearest
//!   nanometre, arcs estimated) and its delay, with the delay of each layer taken from the settings,
//!   from the stack-up for the width of the track, or from the default, in that order;
//! * [`analyse`]: the file of groups and pairs, the groups judged by the spread of their lengths
//!   (or their distance to a target), the pairs by the difference of their lengths, the width of
//!   their tracks, the gap between the edges of the coupled pieces and the length that runs
//!   uncoupled, and the report that holds it all;
//! * [`settings`]: every delay, tolerance and limit, read from `config/lengths.cfg`;
//! * [`report`]: the text and JSON of a report, the same every time;
//! * [`codes`]: the codes `PHOENIX-LEN-001` to `PHOENIX-LEN-019` with their titles in the eight
//!   languages.
//!
//! **State: prototype.** It has been tried on boards made by hand and by the tests only. The length
//! of a net is the total copper of the net, not the path between two pads, so a branch counts; a
//! via is a barrel of the length the stack-up gives between its layers; the delay of the stack-up is
//! a quasi-static estimate of a microstrip or a stripline of the width of the track, not a
//! measurement; the separation of a pair is measured on straight pieces only.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod analyse;
pub mod codes;
mod geom;
pub mod measure;
pub mod report;
pub mod settings;

pub use analyse::{
    analyse, GroupNet, GroupResult, GroupSpec, LengthFile, LengthReport, PairResult, PairSpec,
    FILE_FORMAT,
};
pub use codes::{title, Finding, LengthError, Severity};
pub use measure::{measure_nets, LayerLength, NetLength};
pub use report::{findings_text, millimetres, report_json, report_text};
pub use settings::LengthSettings;
