//! Stable codes for the failures of the model foundations: `PHOENIX-<AREA>-<NNN>`.
//!
//! Every error of this crate maps to one code and one common diagnostic (`eh3d-diagnostic`). The
//! code never changes its meaning; the message is English text and a user interface translates by
//! the key `diag.<code>`. The table [`REGISTRY`] is the list that `docs/ERROR_CODES.md` repeats; a
//! test keeps them in agreement.
//!
//! Areas: `UNITS` (coordinates, angles, decimals, arcs) and `JSON`
//! (canonical JSON and quantisation). Other areas are added with the crates that need them.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};

use crate::error::{CanonicalError, ModelError};

/// Every code of this crate with its title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    ("PHOENIX-UNITS-001", "Coordinate outside the design range"),
    ("PHOENIX-UNITS-002", "Angle outside the range"),
    ("PHOENIX-UNITS-003", "Arithmetic overflow"),
    ("PHOENIX-UNITS-004", "Invalid decimal text"),
    ("PHOENIX-UNITS-005", "Degenerate arc"),
    ("PHOENIX-UNITS-006", "Number is not finite"),
    (
        "PHOENIX-JSON-001",
        "Integer not exactly representable as a JSON number",
    ),
    ("PHOENIX-JSON-002", "Serialisation failed"),
    ("PHOENIX-JSON-003", "Number is not finite"),
    ("PHOENIX-JSON-004", "Number too large to quantise exactly"),
];

fn build(code: &'static str, category: Category, message: String) -> Diagnostic {
    Diagnostic::new(0, Severity::Error, message)
        .with_code(code)
        .with_category(category)
        .with_recoverability(Recoverability::Unrecoverable)
}

impl ModelError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        match self {
            ModelError::CoordinateOutOfRange { .. } => "PHOENIX-UNITS-001",
            ModelError::AngleOutOfRange { .. } => "PHOENIX-UNITS-002",
            ModelError::Overflow { .. } => "PHOENIX-UNITS-003",
            ModelError::InvalidDecimal { .. } => "PHOENIX-UNITS-004",
            ModelError::DegenerateArc { .. } => "PHOENIX-UNITS-005",
            ModelError::NotFinite => "PHOENIX-UNITS-006",
        }
    }

    /// The common diagnostic of this error.
    pub fn diagnostic(&self) -> Diagnostic {
        build(self.code(), Category::Geometric, self.to_string())
    }
}

impl CanonicalError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        match self {
            CanonicalError::IntegerNotRepresentable(_) => "PHOENIX-JSON-001",
            CanonicalError::Serialise(_) => "PHOENIX-JSON-002",
            CanonicalError::NotFinite => "PHOENIX-JSON-003",
            CanonicalError::QuantiseRange(_) => "PHOENIX-JSON-004",
        }
    }

    /// The common diagnostic of this error.
    pub fn diagnostic(&self) -> Diagnostic {
        build(self.code(), Category::Format, self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_has_a_registered_code_and_a_diagnostic() {
        let model = [
            ModelError::CoordinateOutOfRange { value: 1, limit: 2 },
            ModelError::AngleOutOfRange { value: 1, limit: 2 },
            ModelError::Overflow { operation: "x" },
            ModelError::InvalidDecimal { reason: "x" },
            ModelError::DegenerateArc { reason: "x" },
            ModelError::NotFinite,
        ];
        let json = [
            CanonicalError::IntegerNotRepresentable("1".into()),
            CanonicalError::Serialise("x".into()),
            CanonicalError::NotFinite,
            CanonicalError::QuantiseRange("1".into()),
        ];
        let mut seen: Vec<&str> = Vec::new();
        let mut check = |code: &'static str, d: Diagnostic| {
            assert!(
                REGISTRY.iter().any(|(c, _)| *c == code),
                "{code} is not registered"
            );
            assert_eq!(d.code.as_deref(), Some(code));
            assert_eq!(d.severity, Severity::Error);
            assert!(!d.message.is_empty());
            seen.push(code);
        };
        for e in &model {
            check(e.code(), e.diagnostic());
        }
        for e in &json {
            check(e.code(), e.diagnostic());
        }
        let mut registered: Vec<&str> = REGISTRY.iter().map(|(c, _)| *c).collect();
        registered.sort_unstable();
        seen.sort_unstable();
        assert_eq!(
            registered, seen,
            "a registered code no error produces, or the reverse"
        );
    }

    #[test]
    fn codes_follow_the_pattern_and_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for (code, title) in REGISTRY {
            let parts: Vec<&str> = code.split('-').collect();
            assert_eq!(parts.len(), 3, "{code}");
            assert_eq!(parts[0], "PHOENIX");
            assert!(parts[1].chars().all(|c| c.is_ascii_uppercase()));
            assert_eq!(parts[2].len(), 3);
            assert!(parts[2].chars().all(|c| c.is_ascii_digit()));
            assert!(!title.is_empty());
            assert!(seen.insert(*code), "{code} repeated");
        }
    }

    #[test]
    fn the_redacted_form_keeps_only_the_code() {
        let d = ModelError::CoordinateOutOfRange { value: 7, limit: 9 }
            .diagnostic()
            .redacted();
        assert_eq!(d.code.as_deref(), Some("PHOENIX-UNITS-001"));
        assert!(d.message.is_empty());
    }

    #[test]
    fn the_documented_table_has_every_code_of_the_registry() {
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
