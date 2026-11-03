//! Stable diagnostic codes of the EAGLE reader, `PHOENIX-EAGLE-<NNN>`.
//!
//! The table [`REGISTRY`] is the list that `docs/ERROR_CODES.md` repeats; a test keeps them in
//! agreement.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};

/// The text is not well-formed XML.
pub const NOT_XML: &str = "PHOENIX-EAGLE-001";
/// The document is not an EAGLE file (the root element is not `eagle`, or there is no drawing).
pub const NOT_EAGLE: &str = "PHOENIX-EAGLE-002";
/// The file declares a version outside 6 to 9, or none.
pub const VERSION_UNSUPPORTED: &str = "PHOENIX-EAGLE-003";
/// A size, depth or node limit was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-EAGLE-004";
/// The file is not valid UTF-8 and was read as Latin-1.
pub const ENCODING_FALLBACK: &str = "PHOENIX-EAGLE-005";
/// A device names a package that the library does not have.
pub const PACKAGE_MISSING: &str = "PHOENIX-EAGLE-006";
/// A device gate names a symbol that the library does not have.
pub const SYMBOL_MISSING: &str = "PHOENIX-EAGLE-007";
/// A connection names a pad or pin that does not exist.
pub const CONNECT_DANGLES: &str = "PHOENIX-EAGLE-008";
/// An element the reader does not know was counted but not interpreted.
pub const ELEMENT_UNKNOWN: &str = "PHOENIX-EAGLE-009";
/// The file could not be read from disk.
pub const IO_FAILED: &str = "PHOENIX-EAGLE-010";
/// A copy differs from its original, or an original changed while it was read.
pub const COPY_MISMATCH: &str = "PHOENIX-EAGLE-011";
/// The file starts like an EAGLE binary file (the bytes 0x10 0x80), which this reader does not read.
pub const BINARY_FORMAT: &str = "PHOENIX-EAGLE-012";
/// The file is of another program that uses the same extension.
pub const OTHER_PROGRAM: &str = "PHOENIX-EAGLE-013";
/// The file starts with the byte 0x10 like the binary files but without the second byte 0x80.
pub const BINARY_VARIANT: &str = "PHOENIX-EAGLE-014";

/// A binary file without its XML file, or an XML file without its binary file.
pub const PAIR_INCOMPLETE: &str = "PHOENIX-EAGLE-015";
/// A side of a pair is not what it says (a binary file that is not binary, an XML that is not EAGLE).
pub const PAIR_SIDE: &str = "PHOENIX-EAGLE-016";
/// A file of a pair is not in the manifest, differs from it, or the manifest is wrong.
pub const PAIR_HASH: &str = "PHOENIX-EAGLE-017";
/// The summaries of the two sides of a pair differ.
pub const PAIR_DIFFERS: &str = "PHOENIX-EAGLE-018";
/// The pairs were verified and not compared, for lack of a reader of the binary side.
pub const PAIR_NOT_COMPARED: &str = "PHOENIX-EAGLE-019";

/// Every code of this crate with its title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (NOT_XML, "Not well-formed XML"),
    (NOT_EAGLE, "Not an EAGLE document"),
    (VERSION_UNSUPPORTED, "EAGLE version outside 6 to 9"),
    (LIMIT_EXCEEDED, "Size, depth or node limit exceeded"),
    (ENCODING_FALLBACK, "Not valid UTF-8, read as Latin-1"),
    (PACKAGE_MISSING, "Device names a missing package"),
    (SYMBOL_MISSING, "Gate names a missing symbol"),
    (CONNECT_DANGLES, "Connection names a missing pad or pin"),
    (ELEMENT_UNKNOWN, "Element counted but not interpreted"),
    (IO_FAILED, "File could not be read"),
    (COPY_MISMATCH, "Copy differs from the original"),
    (BINARY_FORMAT, "EAGLE binary format is not read"),
    (OTHER_PROGRAM, "File of another program"),
    (BINARY_VARIANT, "Binary variant is not read"),
    (PAIR_INCOMPLETE, "Pair is incomplete"),
    (PAIR_SIDE, "Side of a pair is not what it says"),
    (PAIR_HASH, "Hash of a pair does not match"),
    (PAIR_DIFFERS, "The two sides of a pair differ"),
    (PAIR_NOT_COMPARED, "Pairs verified but not compared"),
];

/// Builds the diagnostic of `code` (which must be in [`REGISTRY`]) with a detail.
pub fn diagnostic(code: &'static str, severity: Severity, detail: &str) -> Diagnostic {
    let title = REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("Unregistered code", |(_, t)| t);
    let message = if detail.is_empty() {
        title.to_owned()
    } else {
        format!("{title}: {detail}")
    };
    let recoverability = if severity == Severity::Error {
        Recoverability::Unrecoverable
    } else {
        Recoverability::Recoverable
    };
    Diagnostic::new(0, severity, message)
        .with_code(code)
        .with_category(Category::Format)
        .with_recoverability(recoverability)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_codes_are_unique_and_well_formed() {
        let mut seen = std::collections::BTreeSet::new();
        for (code, title) in REGISTRY {
            let parts: Vec<&str> = code.split('-').collect();
            assert_eq!(parts.len(), 3, "{code}");
            assert_eq!(parts[0], "PHOENIX");
            assert_eq!(parts[1], "EAGLE");
            assert!(parts[2].len() == 3 && parts[2].bytes().all(|b| b.is_ascii_digit()));
            assert!(!title.is_empty());
            assert!(seen.insert(*code));
        }
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
