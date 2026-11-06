// SPDX-License-Identifier: GPL-3.0-or-later
//! Deep reading of EAGLE files (prototype).
//!
//! * [`dru`]: the design rule files (`.dru`) read and written without loss, turned into the rules
//!   of this tool and back, with a finding for every parameter that has no equivalent on the other
//!   side;
//! * [`cam`]: the CAM jobs (`.cam`) in their three forms, turned into an export profile with a
//!   finding for every option, layer and device that the profile does not respect;
//! * [`variants`]: the attributes of parts and the assembly variants, resolved into a view that
//!   the bill of materials and the placement file use;
//! * [`layers`]: the table of the standard layers, the map of the user layers that
//!   `config/eagle_layers.cfg` configures, and the layers of a file;
//! * [`entities`]: dimensions, holes, vias, polygons, smashed and locked elements, signal classes,
//!   design rules, autorouter passes and modules, as records;
//! * [`check`]: the loss report of a file (`phoenix eagle-check`): the proof that rewriting the
//!   tree keeps everything and the list of what the design model simplifies or leaves out;
//! * [`codes`]: the stable codes `PHOENIX-EAGLEX-<NNN>` with their titles in the eight languages.
//!
//! The file itself is always kept as the XML tree of `eh3d-phoenix-eagle`: what a record does not
//! hold stays in the tree as it is, and writing the tree gives back what was read. The formats are
//! written from their documentation; no file saved by EAGLE has been compared with them.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cam;
pub mod check;
pub mod codes;
pub mod dru;
pub mod entities;
pub mod layers;
pub mod variants;

pub use cam::{cam_to_profile, CamConversion, CamJob};
pub use check::{check_eagle, EagleCheck};
pub use codes::{lines, title, DeepError, Finding, Severity, TITLES};
pub use dru::{dru_to_rules, rule_file_to_dru, rules_to_dru, DruFile, DruToRules, RulesToDru};
pub use entities::{read_entities, Entities};
pub use layers::{read_layers, resolve, standard_table, FileLayers, LayerMap, LayerRole};
pub use variants::{
    apply_to_board, read_variants, write_variants, Applied, VariantSet, VariantView,
};

/// The text of a file that is meant to be text: UTF-8, or Latin-1 when the bytes are not valid
/// UTF-8 (the second element is true then). A byte order mark is dropped.
pub fn decode_text(bytes: &[u8]) -> (String, bool) {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => (text.to_owned(), false),
        Err(_) => (bytes.iter().map(|&b| char::from(b)).collect(), true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_decoded_as_utf8_then_latin1() {
        assert_eq!(decode_text(b"abc"), ("abc".to_owned(), false));
        assert_eq!(
            decode_text(&[0xef, 0xbb, 0xbf, b'a']),
            ("a".to_owned(), false)
        );
        assert_eq!(decode_text("é".as_bytes()), ("é".to_owned(), false));
        assert_eq!(decode_text(&[b'a', 0xe9]), ("a\u{e9}".to_owned(), true));
    }
}
