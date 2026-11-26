//! The `.phoenixproj` project of the design tool.
//!
//! * [`manifest`]: `project.json` (domains, links, migrations, origin) as canonical JSON;
//! * [`project`]: the project in memory, opening and saving it on the shared container
//!   (`eh3d-container`), the migration framework, the handling of domains that are not understood,
//!   the `must_understand` rule and the check of strong and weak links;
//! * [`pcb_branch`]: the branch `pcb/` of the `.eh3d` project that the programs of the suite
//!   share, read and written with the other branches kept as they are;
//! * [`codes`]: the stable diagnostic codes `PHOENIX-MODEL-*` and `PHOENIX-PROJECT-*`.
//!
//! The design objects themselves (components, nets, tracks) are not here: a domain is a folder of
//! entries whose schema belongs to the crate that owns the domain.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod manifest;
pub mod pcb_branch;
pub mod project;

pub use manifest::{
    Domain, Link, Manifest, ManifestProblem, Strength, FORMAT_MAJOR, FORMAT_MINOR, FORMAT_VERSION,
    KNOWN_DOMAINS,
};
pub use pcb_branch::{read_pcb, write_pcb, NewPcbProject, PcbError, PcbOrigin, PcbParts};
pub use project::{check_links, Migration, OpenOptions, Opened, Project, ProjectError};
