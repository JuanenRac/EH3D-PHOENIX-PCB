// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the sanitizer. Every limit, tolerance, weight and layer list is read from a
//! plain `key = value` text, `config/sanitizer.cfg`, and not buried in the code. A missing line
//! keeps the default; an unknown key or a value out of range is refused with its line number.

use eh3d_phoenix_corpus::InventoryConfig;

use crate::codes::{SanError, SETTINGS_INVALID};

/// How the candidate that is kept of a group is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterPick {
    /// The richest member: with a description, most used, most drawing items.
    Richest,
    /// The first member in the order of the library paths and names.
    First,
}

/// The settings of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct SanitizerSettings {
    /// The bounds of the walk and of the size of a file, the same as the corpus inventory uses
    /// (`max_files`, `max_depth`, `max_file_bytes`).
    pub limits: InventoryConfig,
    /// The most nested elements of one library file.
    pub xml_max_depth: usize,
    /// The most elements of one library file.
    pub xml_max_nodes: usize,
    /// Coordinates are rounded to this grid before they are hashed, in nanometres.
    pub round_nm: i64,
    /// Angles are rounded to this grid before they are hashed, in micro-degrees.
    pub angle_round_udeg: i64,
    /// Two items that differ by a turn of 90 degrees count as the same shape.
    pub allow_rotation: bool,
    /// Two items that are mirrored in X count as the same shape.
    pub allow_mirror: bool,
    /// The origin of the item is not part of its shape (a shift counts as the same shape).
    pub ignore_origin: bool,
    /// The lowest score, 0 to 100, at which two items are put in the same group.
    pub threshold: u32,
    /// The distance at which the position of a pad or a pin stops matching, in nanometres.
    pub tolerance_position_nm: i64,
    /// The difference of size at which a pad stops matching, in nanometres.
    pub tolerance_size_nm: i64,
    /// The distance at which a point of an outline or a drawing stops matching, in nanometres.
    pub tolerance_outline_nm: i64,
    /// The distance between the points sampled on lines and arcs, in nanometres.
    pub sample_step_nm: i64,
    /// The most points sampled for one class of drawing of one item.
    pub max_samples: usize,
    /// The weight of the pads in the score of a footprint.
    pub weight_pads: u32,
    /// The weight of the outline in the score of a footprint.
    pub weight_outline: u32,
    /// The weight of the silkscreen in the score of a footprint.
    pub weight_silk: u32,
    /// The weight of the pins in the score of a symbol.
    pub weight_symbol_pins: u32,
    /// The weight of the body in the score of a symbol.
    pub weight_symbol_body: u32,
    /// The layers whose lines are the outline of a footprint.
    pub outline_layers: Vec<u32>,
    /// The layers whose lines are the silkscreen of a footprint.
    pub silk_layers: Vec<u32>,
    /// The most pairs compared in one run; the comparison stops there and says so.
    pub max_pairs: u64,
    /// The most pads or pins that an item may have to be compared by position.
    pub max_anchors: usize,
    /// How the candidate to keep is chosen.
    pub master_pick: MasterPick,
    /// How many times an output file is tried when another program holds it.
    pub write_retries: u32,
    /// The wait between two tries, in milliseconds.
    pub write_retry_ms: u64,
}

impl Default for SanitizerSettings {
    fn default() -> Self {
        SanitizerSettings {
            limits: InventoryConfig::default(),
            xml_max_depth: 256,
            xml_max_nodes: 20_000_000,
            round_nm: 5_000,
            angle_round_udeg: 1_000_000,
            allow_rotation: false,
            allow_mirror: false,
            ignore_origin: false,
            threshold: 90,
            tolerance_position_nm: 200_000,
            tolerance_size_nm: 100_000,
            tolerance_outline_nm: 100_000,
            sample_step_nm: 100_000,
            max_samples: 2_000,
            weight_pads: 50,
            weight_outline: 25,
            weight_silk: 25,
            weight_symbol_pins: 50,
            weight_symbol_body: 50,
            outline_layers: vec![20, 39, 40, 41, 42],
            silk_layers: vec![21, 22, 51, 52],
            max_pairs: 2_000_000,
            max_anchors: 512,
            master_pick: MasterPick::Richest,
            write_retries: 5,
            write_retry_ms: 50,
        }
    }
}

fn parse_list(value: &str) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        out.push(part.parse().ok()?);
    }
    Some(out)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

impl SanitizerSettings {
    /// Parse a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<SanitizerSettings, SanError> {
        let mut s = SanitizerSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                SanError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                SanError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "max_files" => s.limits.max_files = value.parse().map_err(|_| bad())?,
                "max_depth" => s.limits.max_depth = value.parse().map_err(|_| bad())?,
                "max_file_bytes" => s.limits.max_file_bytes = value.parse().map_err(|_| bad())?,
                "xml_max_depth" => s.xml_max_depth = value.parse().map_err(|_| bad())?,
                "xml_max_nodes" => s.xml_max_nodes = value.parse().map_err(|_| bad())?,
                "round_nm" => s.round_nm = value.parse().map_err(|_| bad())?,
                "angle_round_udeg" => s.angle_round_udeg = value.parse().map_err(|_| bad())?,
                "allow_rotation" => s.allow_rotation = parse_bool(value).ok_or_else(bad)?,
                "allow_mirror" => s.allow_mirror = parse_bool(value).ok_or_else(bad)?,
                "ignore_origin" => s.ignore_origin = parse_bool(value).ok_or_else(bad)?,
                "threshold" => s.threshold = value.parse().map_err(|_| bad())?,
                "tolerance_position_nm" => {
                    s.tolerance_position_nm = value.parse().map_err(|_| bad())?
                }
                "tolerance_size_nm" => s.tolerance_size_nm = value.parse().map_err(|_| bad())?,
                "tolerance_outline_nm" => {
                    s.tolerance_outline_nm = value.parse().map_err(|_| bad())?
                }
                "sample_step_nm" => s.sample_step_nm = value.parse().map_err(|_| bad())?,
                "max_samples" => s.max_samples = value.parse().map_err(|_| bad())?,
                "weight_pads" => s.weight_pads = value.parse().map_err(|_| bad())?,
                "weight_outline" => s.weight_outline = value.parse().map_err(|_| bad())?,
                "weight_silk" => s.weight_silk = value.parse().map_err(|_| bad())?,
                "weight_symbol_pins" => s.weight_symbol_pins = value.parse().map_err(|_| bad())?,
                "weight_symbol_body" => s.weight_symbol_body = value.parse().map_err(|_| bad())?,
                "outline_layers" => s.outline_layers = parse_list(value).ok_or_else(bad)?,
                "silk_layers" => s.silk_layers = parse_list(value).ok_or_else(bad)?,
                "max_pairs" => s.max_pairs = value.parse().map_err(|_| bad())?,
                "max_anchors" => s.max_anchors = value.parse().map_err(|_| bad())?,
                "master_pick" => {
                    s.master_pick = match value {
                        "richest" => MasterPick::Richest,
                        "first" => MasterPick::First,
                        _ => return Err(bad()),
                    }
                }
                "write_retries" => s.write_retries = value.parse().map_err(|_| bad())?,
                "write_retry_ms" => s.write_retry_ms = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(SanError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the algorithms rely on.
    pub fn validate(&self) -> Result<(), SanError> {
        let fail = |what: &str| Err(SanError::new(SETTINGS_INVALID, what.to_string()));
        if !(1..=1_000_000).contains(&self.round_nm) {
            return fail("round_nm must be between 1 and 1000000");
        }
        if !(1..=90_000_000).contains(&self.angle_round_udeg) {
            return fail("angle_round_udeg must be between 1 and 90000000");
        }
        if self.threshold > 100 {
            return fail("threshold must be between 0 and 100");
        }
        if self.tolerance_position_nm < 1
            || self.tolerance_size_nm < 1
            || self.tolerance_outline_nm < 1
        {
            return fail("the tolerances must be at least 1 nm");
        }
        if self.sample_step_nm < 1 || self.max_samples < 16 {
            return fail("sample_step_nm must be at least 1 and max_samples at least 16");
        }
        if self.weight_pads + self.weight_outline + self.weight_silk == 0 {
            return fail("the weights of a footprint cannot all be zero");
        }
        if self.weight_symbol_pins + self.weight_symbol_body == 0 {
            return fail("the weights of a symbol cannot all be zero");
        }
        if self.max_anchors == 0 {
            return fail("max_anchors must be at least 1");
        }
        Ok(())
    }

    /// The limits of the XML parser: the size is the file limit of the walk.
    pub fn xml_limits(&self) -> eh3d_phoenix_eagle::Limits {
        eh3d_phoenix_eagle::Limits {
            max_bytes: usize::try_from(self.limits.max_file_bytes).unwrap_or(usize::MAX),
            max_depth: self.xml_max_depth,
            max_nodes: self.xml_max_nodes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        assert_eq!(
            SanitizerSettings::parse("# nothing\n").unwrap(),
            SanitizerSettings::default()
        );
        let s = SanitizerSettings::parse(
            "threshold = 70\nallow_mirror = true\nsilk_layers = 21, 22\nmaster_pick = first\nmax_files = 9\n",
        )
        .unwrap();
        assert_eq!(s.threshold, 70);
        assert!(s.allow_mirror);
        assert_eq!(s.silk_layers, vec![21, 22]);
        assert_eq!(s.master_pick, MasterPick::First);
        assert_eq!(s.limits.max_files, 9);
        assert_eq!(s.round_nm, SanitizerSettings::default().round_nm);
    }

    #[test]
    fn bad_lines_are_refused_with_their_number() {
        for (text, needle) in [
            ("threshold = high", "line 1"),
            ("\nthreshold = 101", "threshold"),
            ("colour = red", "unknown key"),
            ("oops", "line 1"),
            ("round_nm = 0", "round_nm"),
            ("allow_rotation = 1", "line 1"),
            (
                "weight_pads = 0\nweight_outline = 0\nweight_silk = 0",
                "weights",
            ),
            ("silk_layers = 21,x", "line 1"),
            ("master_pick = best", "line 1"),
        ] {
            let e = SanitizerSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
    }

    #[test]
    fn the_shipped_settings_file_is_the_defaults() {
        let text = include_str!("../../../config/sanitizer.cfg");
        assert_eq!(
            SanitizerSettings::parse(text).unwrap(),
            SanitizerSettings::default()
        );
    }
}
