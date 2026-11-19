// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the test points. What counts as a test point, which side is probed and every
//! limit are read from a plain `key = value` text, `config/testpoints.cfg`, and not buried in the
//! code. A missing line keeps the default; an unknown key or a value out of range is refused with
//! its line number. A list is words separated by commas.

use crate::codes::{Severity, TestPointError, SETTINGS_INVALID};

/// The side of the board that the probes touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeSide {
    /// Both sides.
    Both,
    /// Only the top side.
    Top,
    /// Only the bottom side.
    Bottom,
}

impl ProbeSide {
    /// The word of the settings and the reports.
    pub fn as_str(self) -> &'static str {
        match self {
            ProbeSide::Both => "both",
            ProbeSide::Top => "top",
            ProbeSide::Bottom => "bottom",
        }
    }
}

/// The settings of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestPointSettings {
    /// A part whose reference starts with one of these (and goes on with a digit, a mark or
    /// nothing) is a test point.
    pub prefixes: Vec<String>,
    /// A part whose footprint name holds one of these words (any case) is a test point.
    pub footprint_words: Vec<String>,
    /// Whether the pads the board marks as test points count (`Board::test_points`).
    pub use_marks: bool,
    /// Whether the prefixes and the footprint words recognise test points. With both off nothing
    /// is recognised.
    pub use_names: bool,
    /// Whether a test point must have an opening in the solder mask on its side (a solder mask
    /// layer of its side, or of no side, among the layers of its pad); one that has none gets
    /// `PHOENIX-TP-013`.
    pub require_mask_opening: bool,
    /// The side the probes touch.
    pub probe_side: ProbeSide,
    /// The test points every net must have on the probed side.
    pub required_per_net: usize,
    /// A net with fewer pads than this is not asked to have a test point.
    pub min_pads_for_test: usize,
    /// Names of nets that are not asked to have a test point; a name ending in `*` matches every
    /// net that starts with the rest. Any case.
    pub ignore_nets: Vec<String>,
    /// The smallest side of a test point, in nanometres; 0 leaves it unjudged.
    pub min_pad_nm: i64,
    /// The smallest distance between the centres of two test points on the same side, in
    /// nanometres; 0 leaves it unjudged.
    pub min_spacing_nm: i64,
    /// The smallest distance from the centre of a test point to the edge of the board, in
    /// nanometres; 0 leaves it unjudged.
    pub min_edge_nm: i64,
    /// The smallest distance from the centre of a test point to the centre of a pad of any other
    /// part on the same side that is not itself a test point, in nanometres: a probe needs room
    /// around the point. 0 leaves it unjudged.
    pub min_part_clearance_nm: i64,
    /// How serious a net without a test point is.
    pub uncovered_severity: Severity,
    /// The most objects (parts, pads of parts and nets) a board may hold for the check to run.
    pub max_items: usize,
}

impl Default for TestPointSettings {
    fn default() -> Self {
        TestPointSettings {
            prefixes: vec!["TP".to_owned()],
            footprint_words: vec![
                "testpoint".to_owned(),
                "test_point".to_owned(),
                "test-point".to_owned(),
            ],
            use_marks: true,
            use_names: true,
            require_mask_opening: false,
            probe_side: ProbeSide::Both,
            required_per_net: 1,
            min_pads_for_test: 2,
            ignore_nets: vec!["NC".to_owned(), "N/C".to_owned(), "UNCONNECTED*".to_owned()],
            min_pad_nm: 1_000_000,
            min_spacing_nm: 1_270_000,
            min_edge_nm: 1_000_000,
            min_part_clearance_nm: 0,
            uncovered_severity: Severity::Warning,
            max_items: 1_000_000,
        }
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

impl TestPointSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<TestPointSettings, TestPointError> {
        let mut s = TestPointSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                TestPointError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                TestPointError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            match key {
                "testpoint_prefixes" => s.prefixes = parse_list(value),
                "testpoint_footprint_words" => s.footprint_words = parse_list(value),
                "use_marks" => s.use_marks = parse_bool(value).ok_or_else(bad)?,
                "use_names" => s.use_names = parse_bool(value).ok_or_else(bad)?,
                "require_mask_opening" => {
                    s.require_mask_opening = parse_bool(value).ok_or_else(bad)?
                }
                "probe_side" => {
                    s.probe_side = match value {
                        "both" => ProbeSide::Both,
                        "top" => ProbeSide::Top,
                        "bottom" => ProbeSide::Bottom,
                        _ => return Err(bad()),
                    }
                }
                "required_per_net" => s.required_per_net = value.parse().map_err(|_| bad())?,
                "min_pads_for_test" => s.min_pads_for_test = value.parse().map_err(|_| bad())?,
                "ignore_nets" => s.ignore_nets = parse_list(value),
                "min_pad_nm" => s.min_pad_nm = value.parse().map_err(|_| bad())?,
                "min_spacing_nm" => s.min_spacing_nm = value.parse().map_err(|_| bad())?,
                "min_edge_nm" => s.min_edge_nm = value.parse().map_err(|_| bad())?,
                "min_part_clearance_nm" => {
                    s.min_part_clearance_nm = value.parse().map_err(|_| bad())?
                }
                "uncovered_severity" => {
                    s.uncovered_severity = Severity::from_word(value).ok_or_else(bad)?
                }
                "max_items" => s.max_items = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(TestPointError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the calculations rely on.
    pub fn validate(&self) -> Result<(), TestPointError> {
        let fail = |what: &str| Err(TestPointError::new(SETTINGS_INVALID, what.to_string()));
        if self.required_per_net > 1000 {
            return fail("required_per_net must be between 0 and 1000");
        }
        if self.min_pads_for_test > 1_000_000 {
            return fail("min_pads_for_test must be between 0 and 1000000");
        }
        for (name, v) in [
            ("min_pad_nm", self.min_pad_nm),
            ("min_spacing_nm", self.min_spacing_nm),
            ("min_edge_nm", self.min_edge_nm),
            ("min_part_clearance_nm", self.min_part_clearance_nm),
        ] {
            if !(0..(1 << 30)).contains(&v) {
                return fail(&format!("{name} must be between 0 and 2^30 nanometres"));
            }
        }
        if !(1..=100_000_000).contains(&self.max_items) {
            return fail("max_items must be between 1 and 100000000");
        }
        Ok(())
    }

    /// True when the name of a net is one the settings do not ask to have a test point.
    pub fn ignores(&self, net: &str) -> bool {
        let net = net.to_ascii_uppercase();
        self.ignore_nets.iter().any(|pattern| {
            let pattern = pattern.to_ascii_uppercase();
            match pattern.strip_suffix('*') {
                Some(start) => net.starts_with(start),
                None => net == pattern,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid_and_a_blank_text_keeps_them() {
        assert!(TestPointSettings::default().validate().is_ok());
        assert_eq!(
            TestPointSettings::parse("# nothing\n\n").unwrap(),
            TestPointSettings::default()
        );
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_defaults() {
        let text = include_str!("../../../config/testpoints.cfg");
        assert_eq!(
            TestPointSettings::parse(text).unwrap(),
            TestPointSettings::default()
        );
    }

    #[test]
    fn lines_change_the_values_and_lists_are_split_at_commas() {
        let s = TestPointSettings::parse(
            "testpoint_prefixes = TP, J\nprobe_side = bottom\nrequired_per_net = 2\nignore_nets = GND, VCC*\nuncovered_severity = error\nmin_spacing_nm = 0\n",
        )
        .unwrap();
        assert_eq!(s.prefixes, vec!["TP".to_owned(), "J".to_owned()]);
        assert_eq!(s.probe_side, ProbeSide::Bottom);
        assert_eq!(s.required_per_net, 2);
        assert_eq!(s.uncovered_severity, Severity::Error);
        assert_eq!(s.min_spacing_nm, 0);
        assert!(s.ignores("gnd") && s.ignores("VCC3V3") && !s.ignores("SIG"));
        let empty = TestPointSettings::parse("ignore_nets =\n").unwrap();
        assert!(empty.ignore_nets.is_empty());
    }

    #[test]
    fn the_room_around_a_test_point_is_a_limit_that_is_off_by_default() {
        assert_eq!(TestPointSettings::default().min_part_clearance_nm, 0);
        let s = TestPointSettings::parse("min_part_clearance_nm = 2500000\n").unwrap();
        assert_eq!(s.min_part_clearance_nm, 2_500_000);
        for bad in [
            "min_part_clearance_nm = -1",
            "min_part_clearance_nm = x",
            "min_part_clearance_nm = 1073741824",
        ] {
            assert!(
                TestPointSettings::parse(&format!("{bad}\n")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn the_two_ways_of_recognising_a_test_point_are_on_and_can_be_switched_off() {
        let s = TestPointSettings::default();
        assert!(s.use_marks && s.use_names);
        let off = TestPointSettings::parse("use_marks = false\nuse_names = false\n").unwrap();
        assert!(!off.use_marks && !off.use_names);
        // the opening of the solder mask is not asked for unless the settings say so
        assert!(!s.require_mask_opening);
        let masked = TestPointSettings::parse("require_mask_opening = true\n").unwrap();
        assert!(masked.require_mask_opening);
        for text in [
            "use_marks = yes\n",
            "use_names = 1\n",
            "use_marks =\n",
            "require_mask_opening = on\n",
        ] {
            assert!(TestPointSettings::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        let unknown = TestPointSettings::parse("a = 1\n").unwrap_err();
        assert_eq!(unknown.code, SETTINGS_INVALID);
        assert!(unknown.detail.contains("line 1"), "{}", unknown.detail);
        for text in [
            "probe_side = left\n",
            "required_per_net = -1\n",
            "required_per_net = 1001\n",
            "min_pad_nm = -1\n",
            "min_edge_nm = 2000000000\n",
            "uncovered_severity = loud\n",
            "max_items = 0\n",
            "max_items = x\n",
            "probe_side\n",
        ] {
            assert!(TestPointSettings::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn the_default_ignored_nets_are_the_not_connected_ones() {
        let s = TestPointSettings::default();
        assert!(s.ignores("NC") && s.ignores("n/c") && s.ignores("unconnected-(U1-Pad3)"));
        assert!(!s.ignores("GND"));
    }
}
