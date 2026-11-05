// SPDX-License-Identifier: GPL-3.0-or-later
//! Forward and backward annotation between a schematic and a board, by stable identity.
//!
//! A component of the board is made from a part of the schematic and holds its id
//! (`Component::symbol`); that link, not a name, is how the two documents are compared.
//!
//! * [`forward`]: the transfer of the schematic to the board. [`plan_forward`] compares the two and
//!   returns a package of changes (new parts, parts that are gone, reference designators, values,
//!   footprints, nets and the nets of the pads) with the board operations that make them; the
//!   package is applied as one step of the undo/redo history of the board;
//! * [`backward`]: the annotation back into the schematic. [`plan_backward`] returns the edits
//!   that write the reference designators, values and footprints of the board into the parts, and
//!   the schematic has its own undo/redo history;
//! * [`report`]: the changes and what could not be judged or done, as text in eight languages and
//!   as JSON, in a fixed order;
//! * [`settings`]: where new components are put, whether components without a part are removed and
//!   the other choices, read from `config/sync.cfg`;
//! * [`codes`]: the stable codes `PHOENIX-SYNC-<NNN>` with their titles in the eight languages.
//!
//! Planning is a dry run that changes neither document. Applying a package twice changes nothing
//! the second time, undoing it gives back the original, and nothing the package does not name
//! (tracks, vias, zones, positions) is touched. The package has been tried on boards and
//! schematics made by hand and by the tests; it has not been compared with the reference tool.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod backward;
pub mod codes;
pub mod forward;
pub mod report;
pub mod settings;

pub use backward::{
    plan_backward, BackwardPackage, SchematicEdit, SchematicHistory, SchematicStep, SyncError,
};
pub use codes::{title, ChangeKind, REGISTRY};
pub use forward::{plan_forward, ForwardPackage};
pub use report::{Change, Direction, Finding, Report, Skipped};
pub use settings::{SettingsError, SyncSettings};
