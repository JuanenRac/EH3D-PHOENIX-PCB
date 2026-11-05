// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-SCHEDIT-<NNN>` of the schematic editor.

use std::fmt;

/// A settings file that is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-SCHEDIT-001";
/// A file of operations that cannot be read or whose operation was refused.
pub const OPERATIONS_INVALID: &str = "PHOENIX-SCHEDIT-002";
/// A result that would overwrite an input file, or that has a format the editor does not write.
pub const OUTPUT_NOT_ALLOWED: &str = "PHOENIX-SCHEDIT-003";

/// The codes of this crate with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (SETTINGS_INVALID, "Settings file is not valid"),
    (OPERATIONS_INVALID, "File of operations is not valid"),
    (OUTPUT_NOT_ALLOWED, "Output file is not allowed"),
];

/// An error of the editor with its code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchEditError {
    /// The stable code.
    pub code: &'static str,
    /// What is wrong, in English.
    pub detail: String,
}

impl SchEditError {
    /// An error with a code and a detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> SchEditError {
        SchEditError {
            code,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for SchEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for SchEditError {}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn an_error_shows_its_code_first() {
        let e = SchEditError::new(OPERATIONS_INVALID, "bad");
        assert_eq!(e.to_string(), "PHOENIX-SCHEDIT-002: bad");
    }
}
