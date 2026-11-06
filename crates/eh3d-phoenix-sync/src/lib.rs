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
//! * [`annotate`]: the annotation back into the schematic as a plan that is read before it is
//!   applied: [`plan_annotation`] lists the changes the board makes to the schematic (references,
//!   the renumbering by position, values, footprints, equivalent pins and gates connected the other
//!   way round, components and parts that only one document has), each with a state (applicable,
//!   conflict, ignored) and a reason, and a confirmation code that must be given to apply it to a
//!   copy; the rules are read from `config/annotate.cfg` ([`annotate_settings`]);
//! * [`diff`]: [`compare`], the comparison of the two documents in both directions at once, as a
//!   fixed list of differences and conflicts; the codes `PHOENIX-ANNOT-<NNN>` of both are in
//!   [`annot_codes`];
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

pub mod annot_codes;
pub mod annotate;
pub mod annotate_settings;
pub mod backward;
pub mod codes;
pub mod diff;
pub mod forward;
pub mod report;
pub mod settings;

pub use annot_codes::{DiffKind, ItemKind};
pub use annotate::{plan_annotation, AnnotateError, AnnotatePlan, PlanItem, Status};
pub use annotate_settings::AnnotateSettings;
pub use backward::{
    plan_backward, BackwardPackage, PinMove, SchematicEdit, SchematicHistory, SchematicStep,
    SyncError,
};
pub use codes::{title, ChangeKind, REGISTRY};
pub use diff::{compare, DiffEntry, DiffReport};
pub use forward::{plan_forward, ForwardPackage};
pub use report::{Change, Direction, Finding, Report, Skipped};
pub use settings::{SettingsError, SyncSettings};
