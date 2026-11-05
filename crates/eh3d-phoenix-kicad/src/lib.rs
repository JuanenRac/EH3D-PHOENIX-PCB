// SPDX-License-Identifier: GPL-3.0-or-later
//! Readers of KiCad files (versions 6 to 9) into the design model of EH3D-PHOENIX-PCB. They only
//! read.
//!
//! * [`sexpr`]: a bounded reader of the s-expressions KiCad writes, with the line and the column of
//!   every item, limits of size, depth and nodes from the settings, and typed errors;
//! * [`symbols`]: `.kicad_sym` libraries into the symbols of the schematic model;
//! * [`footprint`]: `.kicad_mod` files into the footprints of the board model;
//! * [`schematic`]: `.kicad_sch` files, with their sheets, into the schematic model, with the nets
//!   computed from the drawing;
//! * [`pcb`]: `.kicad_pcb` files into the board model, in integer nanometres;
//! * [`project`]: `.kicad_pro` files (JSON) into the net classes and the design rules;
//! * [`import`]: the importer of the format hub, with the quality report and the fidelity level;
//! * [`settings`]: the limits, read from `config/kicad.cfg`;
//! * [`error`]: typed errors with the codes `PHOENIX-KICAD-001` to `PHOENIX-KICAD-014` and their
//!   titles in the eight languages of the suite.
//!
//! **State: prototype.** The readers are written from the public description of the file formats
//! and tried on files written by hand for the tests (versions 6, 7 and 8 and damaged files). They
//! have not been compared with KiCad itself, nor with a public collection of designs: what is
//! called a net here is computed by this code, and `docs/KICAD.md` says what is read and what is
//! not.
//!
//! Nothing here opens a file, a window or the network (the sheets of a schematic are read through a
//! function the caller gives), and nothing panics on its input.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod arcs;
mod conv;
pub mod error;
pub mod footprint;
pub mod import;
mod layers;
pub mod pcb;
pub mod project;
pub mod schematic;
pub mod settings;
pub mod sexpr;
pub mod symbols;

pub use conv::is_safe_relative_name;
pub use eh3d_phoenix_hub::QualityItem;
pub use error::{title, KicadError, Language};
pub use footprint::{read_footprint, ReadFootprint};
pub use import::{detect_kind, fidelity_of, Fidelity, KicadImporter, KicadKind};
pub use pcb::{read_pcb, ReadPcb};
pub use project::{read_project, NetClassLimits, ReadProject};
pub use schematic::{read_schematic, read_schematic_with_sheets, ReadSchematic};
pub use settings::KicadSettings;
pub use symbols::{read_symbol_library, ReadSymbols};

use eh3d_phoenix_board::Id;

/// Ids for the objects read from a file: the same file always gives the same ids, one after the
/// other. Real ids come from the caller's generator; this is for tools that only look.
pub fn sequential_ids() -> Result<impl FnMut() -> Id, KicadError> {
    const BASE_MS: u64 = 1_700_000_000_000;
    let mut last = Id::from_parts(BASE_MS, 0, 0).map_err(|e| KicadError::Derived {
        detail: format!("cannot make an id: {e:?}"),
    })?;
    let mut n: u64 = 0;
    Ok(move || {
        n += 1;
        if let Ok(id) = Id::from_parts(BASE_MS + n, 0, n) {
            last = id;
        }
        last
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ids_are_distinct_and_in_order() {
        let mut ids = sequential_ids().unwrap();
        let a = ids();
        let b = ids();
        let c = ids();
        assert!(a < b && b < c);
        let mut again = sequential_ids().unwrap();
        assert_eq!(again(), a);
    }
}
