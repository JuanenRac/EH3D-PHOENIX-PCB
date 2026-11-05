// SPDX-License-Identifier: GPL-3.0-or-later
//! The values a board is checked against, and where they come from.
//!
//! No value is written in the code of the checks: every limit comes from a rule file. There are
//! two sources:
//!
//! * a JSON rule file of this tool (`phoenix-rules/1`), with every length in nanometres in a field
//!   whose name ends in `_nm`; a field that is left out turns its rule off (zero is off);
//! * the `<designrules>` of an EAGLE board file, read by [`RuleValues::from_eagle_board`]. EAGLE
//!   has more parameters than this engine uses: the clearances between wires, pads, vias and SMD
//!   pads are folded into one clearance (the largest), and every parameter that is not used is
//!   listed in an information diagnostic, never dropped silently. The mapping is written from the
//!   format as documented and has not been checked against the reference tool.

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_eagle::xml::{parse, Limits};
use eh3d_phoenix_model::Nm;
use serde::{Deserialize, Serialize};

use crate::codes::{
    REGISTRY, RULEFILE_NO_RULES, RULEFILE_PARAM, RULEFILE_UNREADABLE, RULEFILE_VALUE,
};

/// The largest rule file that is read, in bytes.
pub const MAX_RULE_FILE_BYTES: usize = 1 << 20;

/// The format name that a rule file must declare.
pub const FORMAT: &str = "phoenix-rules/1";

/// Why a rule file could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuleFileError {
    /// The file is larger than [`MAX_RULE_FILE_BYTES`].
    #[error("the rule file is {size} bytes, the limit is {limit}")]
    TooLarge {
        /// Its size.
        size: usize,
        /// The limit.
        limit: usize,
    },
    /// The text is not a rule file.
    #[error("the rule file cannot be read: {0}")]
    Unreadable(String),
    /// The file declares another format.
    #[error("the rule file declares the format {found:?}, expected \"phoenix-rules/1\"")]
    WrongFormat {
        /// What it declares.
        found: String,
    },
    /// A value is negative or outside the design range.
    #[error("the value of {name} is not valid: {value}")]
    BadValue {
        /// The name of the field.
        name: &'static str,
        /// The value.
        value: i64,
    },
    /// The board file has no `<designrules>`.
    #[error("the board file holds no design rules")]
    NoDesignRules,
    /// The board file is not an EAGLE board.
    #[error("the board file cannot be read: {0}")]
    BoardUnreadable(String),
}

impl RuleFileError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        match self {
            RuleFileError::TooLarge { .. }
            | RuleFileError::Unreadable(_)
            | RuleFileError::WrongFormat { .. }
            | RuleFileError::BoardUnreadable(_) => RULEFILE_UNREADABLE,
            RuleFileError::BadValue { .. } => RULEFILE_VALUE,
            RuleFileError::NoDesignRules => RULEFILE_NO_RULES,
        }
    }

    /// The common diagnostic of this error.
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::new(0, Severity::Error, self.to_string())
            .with_code(self.code())
            .with_category(Category::Format)
            .with_recoverability(Recoverability::Unrecoverable)
    }
}

/// The numbers a board is checked against. A length of zero turns its rule off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleValues {
    /// Narrowest track.
    pub min_track_width: Nm,
    /// Closest approach of copper of different nets, edge to edge.
    pub min_clearance: Nm,
    /// Smallest drill of a via or a pad.
    pub min_drill: Nm,
    /// Smallest via pad diameter.
    pub min_via_diameter: Nm,
    /// Thinnest ring of copper around a drill.
    pub min_annular_ring: Nm,
    /// Closest approach of copper to the edge of the board.
    pub min_edge_clearance: Nm,
    /// Least solder mask between two pad openings.
    pub min_mask_bridge: Nm,
    /// Least distance between the walls of two drill holes.
    pub min_drill_spacing: Nm,
    /// How far apart two pieces of copper may be and still count as touching.
    pub connect_tolerance: Nm,
    /// Ask for the check of silkscreen over pads.
    pub silkscreen_over_pads: bool,
    /// Ask for the check of silkscreen over the solder mask opening of a pad.
    pub silkscreen_over_mask_openings: bool,
}

impl Default for RuleValues {
    /// Every rule off: a board without rules has nothing to break.
    fn default() -> Self {
        RuleValues {
            min_track_width: Nm::ZERO,
            min_clearance: Nm::ZERO,
            min_drill: Nm::ZERO,
            min_via_diameter: Nm::ZERO,
            min_annular_ring: Nm::ZERO,
            min_edge_clearance: Nm::ZERO,
            min_mask_bridge: Nm::ZERO,
            min_drill_spacing: Nm::ZERO,
            connect_tolerance: Nm::ZERO,
            silkscreen_over_pads: false,
            silkscreen_over_mask_openings: false,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct RulesJson {
    min_track_width_nm: i64,
    min_clearance_nm: i64,
    min_drill_nm: i64,
    min_via_diameter_nm: i64,
    min_annular_ring_nm: i64,
    min_edge_clearance_nm: i64,
    min_mask_bridge_nm: i64,
    min_drill_spacing_nm: i64,
    connect_tolerance_nm: i64,
    silkscreen_over_pads: bool,
    silkscreen_over_mask_openings: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileJson {
    format: String,
    #[serde(default)]
    name: String,
    rules: RulesJson,
}

fn length(name: &'static str, value: i64) -> Result<Nm, RuleFileError> {
    if value < 0 {
        return Err(RuleFileError::BadValue { name, value });
    }
    Nm::new(value).map_err(|_| RuleFileError::BadValue { name, value })
}

impl RuleValues {
    /// Reads a JSON rule file of this tool. The result carries the name the file gives itself.
    pub fn from_json(text: &str) -> Result<(String, RuleValues), RuleFileError> {
        if text.len() > MAX_RULE_FILE_BYTES {
            return Err(RuleFileError::TooLarge {
                size: text.len(),
                limit: MAX_RULE_FILE_BYTES,
            });
        }
        let file: FileJson =
            serde_json::from_str(text).map_err(|e| RuleFileError::Unreadable(e.to_string()))?;
        if file.format != FORMAT {
            return Err(RuleFileError::WrongFormat { found: file.format });
        }
        let r = &file.rules;
        let values = RuleValues {
            min_track_width: length("min_track_width_nm", r.min_track_width_nm)?,
            min_clearance: length("min_clearance_nm", r.min_clearance_nm)?,
            min_drill: length("min_drill_nm", r.min_drill_nm)?,
            min_via_diameter: length("min_via_diameter_nm", r.min_via_diameter_nm)?,
            min_annular_ring: length("min_annular_ring_nm", r.min_annular_ring_nm)?,
            min_edge_clearance: length("min_edge_clearance_nm", r.min_edge_clearance_nm)?,
            min_mask_bridge: length("min_mask_bridge_nm", r.min_mask_bridge_nm)?,
            min_drill_spacing: length("min_drill_spacing_nm", r.min_drill_spacing_nm)?,
            connect_tolerance: length("connect_tolerance_nm", r.connect_tolerance_nm)?,
            silkscreen_over_pads: r.silkscreen_over_pads,
            silkscreen_over_mask_openings: r.silkscreen_over_mask_openings,
        };
        Ok((file.name, values))
    }

    /// Writes the values as a JSON rule file; reading it back gives the same values.
    pub fn to_json(&self, name: &str) -> String {
        let file = FileJson {
            format: FORMAT.to_owned(),
            name: name.to_owned(),
            rules: RulesJson {
                min_track_width_nm: self.min_track_width.get(),
                min_clearance_nm: self.min_clearance.get(),
                min_drill_nm: self.min_drill.get(),
                min_via_diameter_nm: self.min_via_diameter.get(),
                min_annular_ring_nm: self.min_annular_ring.get(),
                min_edge_clearance_nm: self.min_edge_clearance.get(),
                min_mask_bridge_nm: self.min_mask_bridge.get(),
                min_drill_spacing_nm: self.min_drill_spacing.get(),
                connect_tolerance_nm: self.connect_tolerance.get(),
                silkscreen_over_pads: self.silkscreen_over_pads,
                silkscreen_over_mask_openings: self.silkscreen_over_mask_openings,
            },
        };
        let mut text = serde_json::to_string_pretty(&file).unwrap_or_default();
        text.push('\n');
        text
    }

    /// Reads the `<designrules>` of an EAGLE board file. The diagnostics tell what was folded
    /// together or not used (`PHOENIX-RULEFILE-003`, information).
    pub fn from_eagle_board(text: &str) -> Result<(RuleValues, Vec<Diagnostic>), RuleFileError> {
        let root = parse(text, &Limits::default())
            .map_err(|e| RuleFileError::BoardUnreadable(e.to_string()))?;
        if root.name != "eagle" {
            return Err(RuleFileError::BoardUnreadable(
                "the root element is not eagle".to_owned(),
            ));
        }
        let rules = root
            .child("drawing")
            .and_then(|d| d.child("board"))
            .and_then(|b| b.child("designrules"))
            .ok_or(RuleFileError::NoDesignRules)?;

        let mut values = RuleValues::default();
        let mut diagnostics = Vec::new();
        let mut clearances: Vec<(String, Nm)> = Vec::new();
        let mut rings: Vec<(String, Nm)> = Vec::new();
        let mut unused: Vec<String> = Vec::new();
        for p in rules.children_named("param") {
            let (Some(name), Some(raw)) = (p.attr("name"), p.attr("value")) else {
                continue;
            };
            let parsed = parse_length(raw);
            match (name, parsed) {
                ("msWidth", Some(v)) => values.min_track_width = v,
                ("msDrill", Some(v)) => values.min_drill = v,
                ("mdCopperDimension", Some(v)) => values.min_edge_clearance = v,
                ("mdDrill", Some(v)) => values.min_drill_spacing = v,
                (
                    "mdWireWire" | "mdWirePad" | "mdWireVia" | "mdPadPad" | "mdPadVia" | "mdViaVia"
                    | "mdSmdPad" | "mdSmdVia" | "mdSmdSmd",
                    Some(v),
                ) => clearances.push((name.to_owned(), v)),
                ("rlMinPadTop" | "rlMinPadBottom" | "rlMinViaOuter", Some(v)) => {
                    rings.push((name.to_owned(), v));
                }
                _ => unused.push(name.to_owned()),
            }
        }
        if let Some(largest) = clearances.iter().map(|(_, v)| *v).max() {
            values.min_clearance = largest;
            if clearances.iter().any(|(_, v)| *v != largest) {
                diagnostics.push(note(format!(
                    "the clearances between wires, pads, vias and SMD pads differ ({}); the largest, {} nm, is used for all",
                    clearances
                        .iter()
                        .map(|(n, _)| n.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    largest.get()
                )));
            }
        }
        if let Some(largest) = rings.iter().map(|(_, v)| *v).max() {
            values.min_annular_ring = largest;
            if rings.iter().any(|(_, v)| *v != largest) {
                diagnostics.push(note(format!(
                    "the minimum rings differ ({}); the largest, {} nm, is used for all",
                    rings
                        .iter()
                        .map(|(n, _)| n.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    largest.get()
                )));
            }
        }
        if !unused.is_empty() {
            unused.sort();
            unused.dedup();
            diagnostics.push(note(format!(
                "{} parameters are not used: {}",
                unused.len(),
                unused.join(", ")
            )));
        }
        Ok((values, diagnostics))
    }
}

fn note(detail: String) -> Diagnostic {
    let title = REGISTRY
        .iter()
        .find(|(c, _)| *c == RULEFILE_PARAM)
        .map_or("", |(_, t)| *t);
    Diagnostic::new(0, Severity::Info, format!("{title}: {detail}"))
        .with_code(RULEFILE_PARAM)
        .with_category(Category::Format)
        .with_recoverability(Recoverability::Recoverable)
}

/// A length such as `8mil`, `0.2mm`, `0.01inch`, `50micron` or a bare number (millimetres), in
/// nanometres. Rounds half up at the nanometre; `None` when the text is not a plain non-negative
/// decimal with a known unit.
pub(crate) fn parse_length(text: &str) -> Option<Nm> {
    let lower = text.trim().to_ascii_lowercase();
    let (number, unit_nm): (&str, i128) = if let Some(n) = lower.strip_suffix("mil") {
        (n, 25_400)
    } else if let Some(n) = lower.strip_suffix("micron") {
        (n, 1_000)
    } else if let Some(n) = lower.strip_suffix("inch") {
        (n, 25_400_000)
    } else if let Some(n) = lower.strip_suffix("mm") {
        (n, 1_000_000)
    } else {
        (lower.as_str(), 1_000_000)
    };
    let number = number.trim();
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty())
        || !digits(whole)
        || !digits(fraction)
        || fraction.len() > 9
        || whole.len() > 12
    {
        return None;
    }
    let whole: i128 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let (frac, scale): (i128, i128) = if fraction.is_empty() {
        (0, 1)
    } else {
        (
            fraction.parse().ok()?,
            10_i128.checked_pow(fraction.len() as u32)?,
        )
    };
    let scaled = whole
        .checked_mul(unit_nm)?
        .checked_add(frac.checked_mul(unit_nm)?.checked_add(scale / 2)? / scale)?;
    Nm::from_i128(scaled).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_are_read_with_their_unit() {
        assert_eq!(parse_length("8mil").map(Nm::get), Some(203_200));
        assert_eq!(parse_length("0.2mm").map(Nm::get), Some(200_000));
        assert_eq!(parse_length("0.01inch").map(Nm::get), Some(254_000));
        assert_eq!(parse_length("50micron").map(Nm::get), Some(50_000));
        assert_eq!(parse_length("0.3").map(Nm::get), Some(300_000));
        assert_eq!(parse_length(" 6.5 MIL ").map(Nm::get), Some(165_100));
        assert_eq!(parse_length(".5mm").map(Nm::get), Some(500_000));
        for bad in [
            "",
            "mil",
            "-1mm",
            "1e3mm",
            "abc",
            "1.2.3mm",
            "1,5mm",
            "0.1234567890mm",
        ] {
            assert_eq!(parse_length(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_json_rule_file_round_trips() {
        let values = RuleValues {
            min_track_width: Nm::new(150_000).unwrap(),
            min_clearance: Nm::new(200_000).unwrap(),
            min_drill_spacing: Nm::new(254_000).unwrap(),
            connect_tolerance: Nm::new(1_000).unwrap(),
            silkscreen_over_pads: true,
            ..RuleValues::default()
        };
        let text = values.to_json("fab one");
        let (name, back) = RuleValues::from_json(&text).unwrap();
        assert_eq!((name.as_str(), back), ("fab one", values));
        assert_eq!(back.to_json("fab one"), text);
    }

    #[test]
    fn bad_rule_files_get_a_code_and_never_panic() {
        let cases = [
            ("not json", RULEFILE_UNREADABLE),
            ("{}", RULEFILE_UNREADABLE),
            (r#"{"format":"other/9","rules":{}}"#, RULEFILE_UNREADABLE),
            (
                r#"{"format":"phoenix-rules/1","rules":{"min_clearance":1}}"#,
                RULEFILE_UNREADABLE,
            ),
            (
                r#"{"format":"phoenix-rules/1","rules":{"min_clearance_nm":-5}}"#,
                RULEFILE_VALUE,
            ),
            (
                r#"{"format":"phoenix-rules/1","rules":{"min_drill_nm":9999999999}}"#,
                RULEFILE_VALUE,
            ),
        ];
        for (text, code) in cases {
            let error = RuleValues::from_json(text).unwrap_err();
            assert_eq!(error.code(), code, "{text}");
            assert_eq!(error.diagnostic().code.as_deref(), Some(code));
        }
        let big = " ".repeat(MAX_RULE_FILE_BYTES + 1);
        assert!(matches!(
            RuleValues::from_json(&big),
            Err(RuleFileError::TooLarge { .. })
        ));
    }

    #[test]
    fn the_example_rule_file_of_the_repository_is_valid() {
        let text = include_str!("../../../config/rules.example.json");
        let (name, values) = RuleValues::from_json(text).unwrap();
        assert!(name.starts_with("example"));
        assert_eq!(values.min_track_width.get(), 150_000);
        assert!(values.silkscreen_over_pads && values.silkscreen_over_mask_openings);
    }

    #[test]
    fn a_missing_field_turns_its_rule_off() {
        let (_, v) = RuleValues::from_json(r#"{"format":"phoenix-rules/1","rules":{}}"#).unwrap();
        assert_eq!(v, RuleValues::default());
    }

    const BOARD: &str = r#"<eagle version="9.6.2"><drawing><board><designrules name="x">
        <param name="mdWireWire" value="8mil"/>
        <param name="mdWirePad" value="10mil"/>
        <param name="mdCopperDimension" value="0.4mm"/>
        <param name="mdDrill" value="0.25mm"/>
        <param name="msWidth" value="6mil"/>
        <param name="msDrill" value="0.3mm"/>
        <param name="rlMinPadTop" value="10mil"/>
        <param name="rvPadTop" value="0.25"/>
        <param name="psTop" value="-1"/>
        </designrules></board></drawing></eagle>"#;

    #[test]
    fn eagle_design_rules_are_mapped_and_what_is_folded_or_unused_is_said() {
        let (v, notes) = RuleValues::from_eagle_board(BOARD).unwrap();
        assert_eq!(v.min_clearance.get(), 254_000);
        assert_eq!(v.min_track_width.get(), 152_400);
        assert_eq!(v.min_drill.get(), 300_000);
        assert_eq!(v.min_edge_clearance.get(), 400_000);
        assert_eq!(v.min_drill_spacing.get(), 250_000);
        assert_eq!(v.min_annular_ring.get(), 254_000);
        assert_eq!(notes.len(), 2);
        assert!(notes
            .iter()
            .all(|n| n.code.as_deref() == Some(RULEFILE_PARAM) && n.severity == Severity::Info));
        assert!(notes[1].message.contains("psTop") && notes[1].message.contains("rvPadTop"));
    }

    #[test]
    fn a_board_without_design_rules_or_not_a_board_is_refused() {
        let none = r#"<eagle version="9.6.2"><drawing><board/></drawing></eagle>"#;
        assert_eq!(
            RuleValues::from_eagle_board(none).unwrap_err().code(),
            RULEFILE_NO_RULES
        );
        for text in ["", "not xml", "<other/>"] {
            assert_eq!(
                RuleValues::from_eagle_board(text).unwrap_err().code(),
                RULEFILE_UNREADABLE,
                "{text:?}"
            );
        }
    }
}
