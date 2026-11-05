//! Stable diagnostic codes of the manufacturing export, `PHOENIX-EXPORT-<NNN>`.
//!
//! The table [`REGISTRY`] is the list that `docs/ERROR_CODES.md` repeats; a test keeps them in
//! agreement.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};

/// A pad rotated by an angle that is not a right angle was written unrotated.
pub const PAD_ROTATION: &str = "PHOENIX-EXPORT-001";
/// A zone was written as its outline, not poured.
pub const ZONE_NOT_POURED: &str = "PHOENIX-EXPORT-002";
/// The board does not give the stack order of its copper layers, so no copper file function was written.
pub const STACK_ORDER_MISSING: &str = "PHOENIX-EXPORT-003";
/// An arc has no centre that can be computed and was left out.
pub const ARC_SKIPPED: &str = "PHOENIX-EXPORT-004";
/// A file could not be written by the writer of its format.
pub const WRITE_FAILED: &str = "PHOENIX-EXPORT-005";
/// A drill is written as a round hole although its pad is not round.
pub const SLOT_AS_HOLE: &str = "PHOENIX-EXPORT-006";
/// The package could not be assembled.
pub const PACKAGE_FAILED: &str = "PHOENIX-EXPORT-007";

/// A silkscreen layer does not say which side of the board it is on, so its file has no function.
pub const SILK_SIDE_MISSING: &str = "PHOENIX-EXPORT-008";
/// Characters of a silkscreen text that the font does not have were written as boxes.
pub const SILK_NO_GLYPH: &str = "PHOENIX-EXPORT-009";
/// A silkscreen item that cannot be drawn was left out.
pub const SILK_UNDRAWN: &str = "PHOENIX-EXPORT-010";

/// An export profile is not valid.
pub const PROFILE_INVALID: &str = "PHOENIX-EXPORT-011";
/// The profile asked for is not in the profile file.
pub const PROFILE_NOT_FOUND: &str = "PHOENIX-EXPORT-012";
/// The output could not be written to its folder.
pub const OUTPUT_FAILED: &str = "PHOENIX-EXPORT-013";
/// A written file does not agree with the model.
pub const VERIFY_MISMATCH: &str = "PHOENIX-EXPORT-014";
/// The board has no placed component, so the assembly files are empty.
pub const NO_COMPONENTS: &str = "PHOENIX-EXPORT-015";
/// A drilled oblong pad turned by an angle that is not a right angle was written as a round hole.
pub const SLOT_ROTATION: &str = "PHOENIX-EXPORT-016";
/// A solder mask or paste layer does not say which side of the board it is on, so its file has no function.
pub const SIDE_MISSING: &str = "PHOENIX-EXPORT-017";
/// A written file could not be read back by the check.
pub const VERIFY_UNREADABLE: &str = "PHOENIX-EXPORT-018";

/// The codes of this crate with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (PAD_ROTATION, "Pad rotation is not a right angle"),
    (ZONE_NOT_POURED, "Zone written as its outline"),
    (STACK_ORDER_MISSING, "Copper stack order not given"),
    (ARC_SKIPPED, "Arc left out"),
    (WRITE_FAILED, "File could not be written"),
    (SLOT_AS_HOLE, "Non-round pad drilled as a round hole"),
    (PACKAGE_FAILED, "Package could not be assembled"),
    (SILK_SIDE_MISSING, "Silkscreen side not given"),
    (SILK_NO_GLYPH, "Silkscreen characters without a glyph"),
    (SILK_UNDRAWN, "Silkscreen item left out"),
    (PROFILE_INVALID, "Export profile is not valid"),
    (PROFILE_NOT_FOUND, "Export profile not found"),
    (OUTPUT_FAILED, "Output could not be written"),
    (VERIFY_MISMATCH, "Written file differs from the model"),
    (NO_COMPONENTS, "Board has no components"),
    (SLOT_ROTATION, "Slot pad not on a right angle"),
    (SIDE_MISSING, "Layer side not given"),
    (VERIFY_UNREADABLE, "Written file could not be read back"),
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
    fn the_codes_are_unique_and_well_formed() {
        let mut seen = std::collections::BTreeSet::new();
        for (code, title) in REGISTRY {
            let parts: Vec<&str> = code.split('-').collect();
            assert_eq!(parts.len(), 3, "{code}");
            assert_eq!((parts[0], parts[1]), ("PHOENIX", "EXPORT"));
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
