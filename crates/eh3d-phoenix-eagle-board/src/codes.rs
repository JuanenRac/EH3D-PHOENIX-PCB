//! The codes of the board writer and reader, `PHOENIX-EAGLE-<NNN>`, continuing the table of the
//! EAGLE reader.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};

/// An item was written in a simpler form than the model holds it.
pub const ITEM_SIMPLIFIED: &str = "PHOENIX-EAGLE-020";
/// An item cannot be written in an EAGLE file and was left out.
pub const ITEM_LEFT_OUT: &str = "PHOENIX-EAGLE-021";
/// A part of an EAGLE file was not read into the board.
pub const NOT_READ: &str = "PHOENIX-EAGLE-022";
/// A file cannot be read into a board at all.
pub const UNREADABLE: &str = "PHOENIX-EAGLE-023";

/// The codes of this crate with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (ITEM_SIMPLIFIED, "Item written in a simpler form"),
    (ITEM_LEFT_OUT, "Item left out of the EAGLE file"),
    (NOT_READ, "Part of the EAGLE file not read into the board"),
    (UNREADABLE, "EAGLE file cannot be read into a board"),
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
    Diagnostic::new(0, severity, message)
        .with_code(code)
        .with_category(Category::Format)
        .with_recoverability(if severity == Severity::Error {
            Recoverability::Unrecoverable
        } else {
            Recoverability::Recoverable
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        let mut seen = std::collections::BTreeSet::new();
        for (code, title) in REGISTRY {
            assert!(seen.insert(*code));
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
