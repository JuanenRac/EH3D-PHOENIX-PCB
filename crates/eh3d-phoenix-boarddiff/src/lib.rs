// SPDX-License-Identifier: GPL-3.0-or-later
//! The comparison of two boards: what the new board has that the old one has not, what it lost and
//! what it changed, for two revisions of the same design.
//!
//! * [`snapshot`]: a board reduced to plain facts that do not depend on the ids of its objects
//!   (the parts by reference, the pins by `REF.PAD`, the copper of each net on each layer, the
//!   vias, the zones and the outline);
//! * [`compare`]: the two snapshots compared, as a list of differences in a fixed order, with the
//!   tolerances and the kinds of object that the settings choose;
//! * [`settings`]: those tolerances and choices, the parts and nets that are left out and the
//!   limits, read from `config/boarddiff.cfg`;
//! * [`report`]: the text, with its words in the eight languages, and the JSON of a report;
//! * [`codes`]: the codes `PHOENIX-DIFF-001` to `PHOENIX-DIFF-017` with their titles in the eight
//!   languages.
//!
//! **State: prototype.** It has been tried on boards made by hand and by the tests. Parts are
//! matched by reference and pins by reference and pad number, so a board whose references were
//! renumbered looks like every part removed and added; a track is not matched to a track, only the
//! number and the length of the tracks of a net on a layer are compared (an arc counts as its arc),
//! and the shape of a zone or of the outline is compared by its size and area, not by its corners.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod compare;
pub mod report;
pub mod settings;
pub mod snapshot;

use eh3d_phoenix_board::Board;

pub use codes::{title, DiffError, Difference};
pub use compare::{compare, Count, DiffReport};
pub use report::{report_json, report_text, FORMAT};
pub use settings::DiffSettings;
pub use snapshot::{snapshot, Snapshot};

/// Compares two boards with the settings: the old one first, the new one second.
pub fn compare_boards(
    old: &Board,
    new: &Board,
    settings: &DiffSettings,
) -> Result<DiffReport, DiffError> {
    let (a, b) = (snapshot(old, settings)?, snapshot(new, settings)?);
    Ok(compare(&a, &b, settings))
}
