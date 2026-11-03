//! Stable diagnostic codes of the project layer, `PHOENIX-<AREA>-<NNN>`.
//!
//! The table [`REGISTRY`] is the list that `docs/ERROR_CODES.md` repeats under the rows of the
//! model crate; a test keeps them in agreement.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};

/// Strong reference that dangles.
pub const STRONG_REFERENCE_DANGLES: &str = "PHOENIX-MODEL-001";
/// Critical domain that is unknown or newer than this reader.
pub const CRITICAL_DOMAIN_UNKNOWN: &str = "PHOENIX-MODEL-004";
/// Weak reference to an object that no longer exists.
pub const WEAK_REFERENCE_STALE: &str = "PHOENIX-MODEL-005";
/// Field or object marked `must_understand` that the reader does not know.
pub const MUST_UNDERSTAND: &str = "PHOENIX-MODEL-006";
/// The manifest cannot be read or is inconsistent.
pub const MANIFEST_INVALID: &str = "PHOENIX-PROJECT-001";
/// The project is older and no migration is registered.
pub const MIGRATION_MISSING: &str = "PHOENIX-PROJECT-002";
/// An entry lives in a domain the manifest does not declare.
pub const DOMAIN_UNDECLARED: &str = "PHOENIX-PROJECT-003";
/// A domain that is not critical was kept untouched.
pub const DOMAIN_PRESERVED: &str = "PHOENIX-PROJECT-004";
/// The project was opened read-only.
pub const OPENED_READ_ONLY: &str = "PHOENIX-PROJECT-005";
/// A migration step failed.
pub const MIGRATION_FAILED: &str = "PHOENIX-PROJECT-006";

/// Every code of this crate with its title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (STRONG_REFERENCE_DANGLES, "Strong reference dangles"),
    (CRITICAL_DOMAIN_UNKNOWN, "Critical domain unknown or newer"),
    (WEAK_REFERENCE_STALE, "Weak reference is stale"),
    (
        MUST_UNDERSTAND,
        "Field marked must_understand is not understood",
    ),
    (MANIFEST_INVALID, "Manifest invalid"),
    (
        MIGRATION_MISSING,
        "No migration registered for this version",
    ),
    (
        DOMAIN_UNDECLARED,
        "Entry in a domain the manifest does not declare",
    ),
    (DOMAIN_PRESERVED, "Domain preserved untouched"),
    (OPENED_READ_ONLY, "Project opened read-only"),
    (MIGRATION_FAILED, "Migration failed"),
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

    #[test]
    fn a_diagnostic_has_the_code_and_the_title() {
        let d = diagnostic(MUST_UNDERSTAND, Severity::Error, "type x");
        assert_eq!(d.code.as_deref(), Some("PHOENIX-MODEL-006"));
        assert!(d.message.starts_with("Field marked must_understand"));
        assert!(d.message.ends_with("type x"));
    }
}
