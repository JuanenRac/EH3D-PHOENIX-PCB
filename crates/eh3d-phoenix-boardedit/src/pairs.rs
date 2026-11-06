// SPDX-License-Identifier: GPL-3.0-or-later
//! The definition of the differential pairs: two nets, the width of the tracks, the gap between
//! their edges, an optional target for the differential impedance, the tolerance of the lengths and
//! the longest stretch that may run uncoupled. The tuned values are read from
//! `config/diffpairs.cfg` (which is also built into the program as the default); the pairs can be
//! written in that file (`pair.NAME.field = value`) or in a JSON file of the format
//! `phoenix-diffpairs/1`.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Id};
use serde::Deserialize;

use crate::codes::BoardEditError;
use crate::pair_codes::{NET_MISSING, PAIRS_INVALID};

/// The name of the format of the JSON file of pairs.
pub const FILE_FORMAT: &str = "phoenix-diffpairs/1";

/// The largest file of pairs or of settings that is read, in bytes.
pub const MAX_FILE_BYTES: usize = 1 << 20;

/// The largest length or width that is accepted, in nanometres.
const LENGTH_LIMIT_NM: i64 = 1 << 29;

/// The settings file of the repository, which is the default.
const BUILT_IN: &str = include_str!("../../../config/diffpairs.cfg");

fn invalid(what: impl Into<String>) -> BoardEditError {
    BoardEditError::new(PAIRS_INVALID, what)
}

/// The tuned values of the pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairSettings {
    /// The width of the tracks of a pair that gives none, in nanometres.
    pub default_width_nm: i64,
    /// The gap between the edges of the two tracks of a pair that gives none, in nanometres.
    pub default_gap_nm: i64,
    /// How much the lengths of a pair that gives no tolerance may differ, in nanometres.
    pub default_length_tolerance_nm: i64,
    /// The most a pair that gives no limit may run uncoupled, in nanometres.
    pub default_max_uncoupled_nm: i64,
    /// How far the gap may be from the one asked for, in nanometres.
    pub gap_tolerance_nm: i64,
    /// How far the width may be from the one asked for, in nanometres.
    pub width_tolerance_nm: i64,
    /// The widest gap at which two pieces count as coupled, in nanometres.
    pub coupling_range_nm: i64,
    /// How much the coupled lengths of the two nets may differ, in nanometres.
    pub symmetry_tolerance_nm: i64,
    /// How far apart two vias of a pair may be and still be a couple, in nanometres.
    pub via_pair_range_nm: i64,
    /// How far the via of the negative net is moved along the pair at a change of layer, in
    /// nanometres.
    pub via_stagger_nm: i64,
    /// How far the differential impedance may be from the target, in parts per thousand.
    pub impedance_tolerance_permille: i64,
    /// The frequency the impedance is worked out at, in MHz.
    pub impedance_frequency_mhz: i64,
    /// The most pairs a file may hold.
    pub max_pairs: usize,
}

/// One differential pair. A value that is left out takes the default of the settings.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairDef {
    /// The name of the pair.
    pub name: String,
    /// The name of the positive net.
    pub positive: String,
    /// The name of the negative net.
    pub negative: String,
    /// The width of the tracks, in nanometres.
    #[serde(default)]
    pub width_nm: Option<i64>,
    /// The gap between the edges of the two tracks, in nanometres.
    #[serde(default)]
    pub gap_nm: Option<i64>,
    /// How much the lengths of the two nets may differ, in nanometres.
    #[serde(default)]
    pub length_tolerance_nm: Option<i64>,
    /// The most that may run uncoupled, in nanometres.
    #[serde(default)]
    pub max_uncoupled_nm: Option<i64>,
    /// The differential impedance asked for, in ohm.
    #[serde(default)]
    pub target_diff_ohm: Option<f64>,
}

impl PairDef {
    fn new(name: &str) -> PairDef {
        PairDef {
            name: name.to_owned(),
            positive: String::new(),
            negative: String::new(),
            width_nm: None,
            gap_nm: None,
            length_tolerance_nm: None,
            max_uncoupled_nm: None,
            target_diff_ohm: None,
        }
    }

    /// Checks the pair by itself: the names and the ranges.
    pub fn validate(&self) -> Result<(), BoardEditError> {
        if self.name.trim().is_empty() {
            return Err(invalid("a pair has no name"));
        }
        if self.positive.trim().is_empty() || self.negative.trim().is_empty() {
            return Err(invalid(format!(
                "the pair {} needs a positive and a negative net",
                self.name
            )));
        }
        if self.positive == self.negative {
            return Err(invalid(format!(
                "the pair {} has the same net twice",
                self.name
            )));
        }
        for (what, value, low) in [
            ("width_nm", self.width_nm, 1),
            ("gap_nm", self.gap_nm, 1),
            ("length_tolerance_nm", self.length_tolerance_nm, 0),
            ("max_uncoupled_nm", self.max_uncoupled_nm, 0),
        ] {
            if let Some(v) = value {
                if !(low..=LENGTH_LIMIT_NM).contains(&v) {
                    return Err(invalid(format!(
                        "the pair {}: {what} must be between {low} and {LENGTH_LIMIT_NM}",
                        self.name
                    )));
                }
            }
        }
        if let Some(ohm) = self.target_diff_ohm {
            if !ohm.is_finite() || ohm <= 0.0 || ohm > 10_000.0 {
                return Err(invalid(format!(
                    "the pair {}: target_diff_ohm must be above zero and at most 10000",
                    self.name
                )));
            }
        }
        Ok(())
    }
}

/// A pair with its values filled in and its nets found on a board.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPair {
    /// The name of the pair.
    pub name: String,
    /// The name of the positive net.
    pub positive_name: String,
    /// The name of the negative net.
    pub negative_name: String,
    /// The positive net.
    pub positive: Id,
    /// The negative net.
    pub negative: Id,
    /// The width of the tracks, in nanometres.
    pub width_nm: i64,
    /// The gap between the edges of the two tracks, in nanometres.
    pub gap_nm: i64,
    /// How much the lengths may differ, in nanometres.
    pub length_tolerance_nm: i64,
    /// The most that may run uncoupled, in nanometres.
    pub max_uncoupled_nm: i64,
    /// The differential impedance asked for, in ohm.
    pub target_diff_ohm: Option<f64>,
}

/// The first net of a board with a name (in the order of the ids).
pub fn net_by_name(board: &Board, name: &str) -> Option<Id> {
    board
        .nets
        .iter()
        .find(|(_, net)| net.name == name)
        .map(|(id, _)| *id)
}

/// Fills in the values of a pair from the settings and finds its nets on the board.
pub fn resolve(
    board: &Board,
    def: &PairDef,
    settings: &PairSettings,
) -> Result<ResolvedPair, BoardEditError> {
    let find = |name: &str| {
        net_by_name(board, name).ok_or_else(|| {
            BoardEditError::new(
                NET_MISSING,
                format!("the pair {}: the board has no net {name}", def.name),
            )
        })
    };
    Ok(ResolvedPair {
        name: def.name.clone(),
        positive_name: def.positive.clone(),
        negative_name: def.negative.clone(),
        positive: find(&def.positive)?,
        negative: find(&def.negative)?,
        width_nm: def.width_nm.unwrap_or(settings.default_width_nm),
        gap_nm: def.gap_nm.unwrap_or(settings.default_gap_nm),
        length_tolerance_nm: def
            .length_tolerance_nm
            .unwrap_or(settings.default_length_tolerance_nm),
        max_uncoupled_nm: def
            .max_uncoupled_nm
            .unwrap_or(settings.default_max_uncoupled_nm),
        target_diff_ohm: def.target_diff_ohm,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairsFile {
    format: String,
    pairs: Vec<PairDef>,
}

/// The settings of the pairs and the pairs, in the order of their names.
#[derive(Debug, Clone, PartialEq)]
pub struct PairConfig {
    /// The tuned values.
    pub settings: PairSettings,
    /// The pairs, in the order of their names.
    pub pairs: Vec<PairDef>,
}

impl Default for PairConfig {
    /// The settings file of the repository (`config/diffpairs.cfg`).
    fn default() -> Self {
        let mut config = PairConfig::empty();
        if config.apply(BUILT_IN).is_err() {
            return PairConfig::empty();
        }
        config
    }
}

fn parse_pair_field(def: &mut PairDef, field: &str, value: &str) -> Result<(), String> {
    let bad = || format!("the value of {field} is not valid");
    match field {
        "positive" => def.positive = value.to_owned(),
        "negative" => def.negative = value.to_owned(),
        "width_nm" => def.width_nm = Some(value.parse().map_err(|_| bad())?),
        "gap_nm" => def.gap_nm = Some(value.parse().map_err(|_| bad())?),
        "length_tolerance_nm" => def.length_tolerance_nm = Some(value.parse().map_err(|_| bad())?),
        "max_uncoupled_nm" => def.max_uncoupled_nm = Some(value.parse().map_err(|_| bad())?),
        "target_diff_ohm" => def.target_diff_ohm = Some(value.parse().map_err(|_| bad())?),
        _ => return Err(format!("unknown field {field}")),
    }
    Ok(())
}

impl PairConfig {
    fn empty() -> PairConfig {
        PairConfig {
            settings: PairSettings {
                default_width_nm: 1,
                default_gap_nm: 1,
                default_length_tolerance_nm: 0,
                default_max_uncoupled_nm: 0,
                gap_tolerance_nm: 0,
                width_tolerance_nm: 0,
                coupling_range_nm: 1,
                symmetry_tolerance_nm: 0,
                via_pair_range_nm: 1,
                via_stagger_nm: 0,
                impedance_tolerance_permille: 1,
                impedance_frequency_mhz: 1,
                max_pairs: 1,
            },
            pairs: Vec::new(),
        }
    }

    /// Reads a settings text on top of the default: a line that is left out keeps the default.
    /// Blank lines and lines starting with `#` are ignored. A line `pair.NAME.field = value`
    /// defines a pair.
    pub fn parse(text: &str) -> Result<PairConfig, BoardEditError> {
        let mut config = PairConfig::default();
        config.apply(text)?;
        Ok(config)
    }

    fn apply(&mut self, text: &str) -> Result<(), BoardEditError> {
        if text.len() > MAX_FILE_BYTES {
            return Err(invalid(format!(
                "the file is {} bytes, the limit is {MAX_FILE_BYTES}",
                text.len()
            )));
        }
        let mut found: BTreeMap<String, PairDef> = BTreeMap::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let fail = |message: String| invalid(format!("line {line}: {message}"));
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| fail("not key = value".to_owned()))?;
            let (key, value) = (key.trim(), value.trim());
            if let Some(rest) = key.strip_prefix("pair.") {
                let (name, field) = rest
                    .rsplit_once('.')
                    .ok_or_else(|| fail("a pair line is pair.NAME.field".to_owned()))?;
                let def = found
                    .entry(name.to_owned())
                    .or_insert_with(|| PairDef::new(name));
                parse_pair_field(def, field, value).map_err(fail)?;
                continue;
            }
            let bad = || fail(format!("the value of {key} is not valid"));
            let s = &mut self.settings;
            match key {
                "default_width_nm" => s.default_width_nm = value.parse().map_err(|_| bad())?,
                "default_gap_nm" => s.default_gap_nm = value.parse().map_err(|_| bad())?,
                "default_length_tolerance_nm" => {
                    s.default_length_tolerance_nm = value.parse().map_err(|_| bad())?;
                }
                "default_max_uncoupled_nm" => {
                    s.default_max_uncoupled_nm = value.parse().map_err(|_| bad())?;
                }
                "gap_tolerance_nm" => s.gap_tolerance_nm = value.parse().map_err(|_| bad())?,
                "width_tolerance_nm" => s.width_tolerance_nm = value.parse().map_err(|_| bad())?,
                "coupling_range_nm" => s.coupling_range_nm = value.parse().map_err(|_| bad())?,
                "symmetry_tolerance_nm" => {
                    s.symmetry_tolerance_nm = value.parse().map_err(|_| bad())?;
                }
                "via_pair_range_nm" => s.via_pair_range_nm = value.parse().map_err(|_| bad())?,
                "via_stagger_nm" => s.via_stagger_nm = value.parse().map_err(|_| bad())?,
                "impedance_tolerance_permille" => {
                    s.impedance_tolerance_permille = value.parse().map_err(|_| bad())?;
                }
                "impedance_frequency_mhz" => {
                    s.impedance_frequency_mhz = value.parse().map_err(|_| bad())?;
                }
                "max_pairs" => s.max_pairs = value.parse().map_err(|_| bad())?,
                _ => return Err(fail(format!("unknown key {key}"))),
            }
        }
        self.check_settings()?;
        for def in found.into_values() {
            def.validate()?;
            self.insert(def);
        }
        self.sort_and_limit()
    }

    fn insert(&mut self, def: PairDef) {
        match self.pairs.iter_mut().find(|p| p.name == def.name) {
            Some(existing) => *existing = def,
            None => self.pairs.push(def),
        }
    }

    fn sort_and_limit(&mut self) -> Result<(), BoardEditError> {
        self.pairs.sort_by(|a, b| a.name.cmp(&b.name));
        if self.pairs.len() > self.settings.max_pairs {
            return Err(invalid(format!(
                "more than {} pairs",
                self.settings.max_pairs
            )));
        }
        Ok(())
    }

    /// Adds the pairs of a JSON file of the format `phoenix-diffpairs/1`; a pair with the name of
    /// one that is already there replaces it.
    pub fn merge_json(&mut self, text: &str) -> Result<(), BoardEditError> {
        if text.len() > MAX_FILE_BYTES {
            return Err(invalid(format!(
                "the file is {} bytes, the limit is {MAX_FILE_BYTES}",
                text.len()
            )));
        }
        let file: PairsFile = serde_json::from_str(text)
            .map_err(|e| invalid(format!("the file cannot be read: {e}")))?;
        if file.format != FILE_FORMAT {
            return Err(invalid(format!("the format must be {FILE_FORMAT}")));
        }
        for (i, def) in file.pairs.iter().enumerate() {
            def.validate()?;
            if file.pairs[..i].iter().any(|p| p.name == def.name) {
                return Err(invalid(format!("the pair name {} is used twice", def.name)));
            }
        }
        for def in file.pairs {
            self.insert(def);
        }
        self.sort_and_limit()
    }

    /// The pair with a name.
    pub fn find(&self, name: &str) -> Option<&PairDef> {
        self.pairs.iter().find(|p| p.name == name)
    }

    /// Checks the ranges of the settings.
    fn check_settings(&self) -> Result<(), BoardEditError> {
        let s = &self.settings;
        let ranges = [
            ("default_width_nm", s.default_width_nm, 1),
            ("default_gap_nm", s.default_gap_nm, 1),
            (
                "default_length_tolerance_nm",
                s.default_length_tolerance_nm,
                0,
            ),
            ("default_max_uncoupled_nm", s.default_max_uncoupled_nm, 0),
            ("gap_tolerance_nm", s.gap_tolerance_nm, 0),
            ("width_tolerance_nm", s.width_tolerance_nm, 0),
            ("coupling_range_nm", s.coupling_range_nm, 1),
            ("symmetry_tolerance_nm", s.symmetry_tolerance_nm, 0),
            ("via_pair_range_nm", s.via_pair_range_nm, 1),
            ("via_stagger_nm", s.via_stagger_nm, 0),
            (
                "impedance_tolerance_permille",
                s.impedance_tolerance_permille,
                1,
            ),
            ("impedance_frequency_mhz", s.impedance_frequency_mhz, 1),
        ];
        for (name, value, low) in ranges {
            if !(low..=LENGTH_LIMIT_NM).contains(&value) {
                return Err(invalid(format!(
                    "{name} must be between {low} and {LENGTH_LIMIT_NM}"
                )));
            }
        }
        if s.max_pairs == 0 || s.max_pairs > 100_000 {
            return Err(invalid("max_pairs must be between 1 and 100000"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::fixture;

    #[test]
    fn the_file_of_the_repository_is_the_default_and_is_valid() {
        let mut from_file = PairConfig::empty();
        from_file.apply(BUILT_IN).unwrap();
        assert_eq!(from_file, PairConfig::default());
        let s = PairConfig::default().settings;
        assert_eq!(s.default_width_nm, 150_000);
        assert_eq!(s.gap_tolerance_nm, 25_000);
        assert_eq!(s.impedance_tolerance_permille, 100);
        assert!(PairConfig::default().pairs.is_empty());
    }

    #[test]
    fn pairs_are_read_from_the_lines_and_sorted_by_name() {
        let text = "gap_tolerance_nm = 40000\n\
                    pair.USB.positive = USB_P\npair.USB.negative = USB_N\n\
                    pair.USB.target_diff_ohm = 90\npair.USB.gap_nm = 200000\n\
                    pair.ETH.positive = TX_P\npair.ETH.negative = TX_N\n";
        let config = PairConfig::parse(text).unwrap();
        assert_eq!(config.settings.gap_tolerance_nm, 40_000);
        let names: Vec<&str> = config.pairs.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["ETH", "USB"]);
        let usb = config.find("USB").unwrap();
        assert_eq!(usb.gap_nm, Some(200_000));
        assert_eq!(usb.target_diff_ohm, Some(90.0));
        assert_eq!(usb.width_nm, None);
        assert!(config.find("none").is_none());
    }

    #[test]
    fn bad_lines_are_refused_with_the_code_and_never_panic() {
        for text in [
            "nonsense",
            "unknown = 1",
            "default_width_nm = x",
            "default_width_nm = 0",
            "max_pairs = 0",
            "pair.A.positive = P",
            "pair.A.positive = P\npair.A.negative = P",
            "pair.A.positive = P\npair.A.negative = N\npair.A.colour = 1",
            "pair.A.positive = P\npair.A.negative = N\npair.A.width_nm = 0",
            "pair.A.positive = P\npair.A.negative = N\npair.A.target_diff_ohm = nan",
            "pair.A = 1",
        ] {
            let error = PairConfig::parse(text).unwrap_err();
            assert_eq!(error.code, PAIRS_INVALID, "{text}");
        }
        let big = " ".repeat(MAX_FILE_BYTES + 1);
        assert!(PairConfig::parse(&big).is_err());
    }

    #[test]
    fn a_json_file_adds_pairs_and_replaces_one_of_the_same_name() {
        let mut config = PairConfig::parse(
            "pair.USB.positive = A\npair.USB.negative = B\npair.USB.gap_nm = 100000\n",
        )
        .unwrap();
        let json = r#"{"format":"phoenix-diffpairs/1","pairs":[
            {"name":"USB","positive":"USB_P","negative":"USB_N","gap_nm":200000},
            {"name":"CLK","positive":"CK_P","negative":"CK_N","target_diff_ohm":100.0}]}"#;
        config.merge_json(json).unwrap();
        assert_eq!(config.pairs.len(), 2);
        assert_eq!(config.pairs[0].name, "CLK");
        assert_eq!(config.find("USB").unwrap().positive, "USB_P");
        assert_eq!(config.find("USB").unwrap().gap_nm, Some(200_000));
    }

    #[test]
    fn a_bad_json_file_is_refused() {
        let mut config = PairConfig::default();
        for text in [
            "",
            "[]",
            r#"{"format":"x/1","pairs":[]}"#,
            r#"{"format":"phoenix-diffpairs/1"}"#,
            r#"{"format":"phoenix-diffpairs/1","pairs":[{"name":"A","positive":"P","negative":"P"}]}"#,
            r#"{"format":"phoenix-diffpairs/1","pairs":[{"name":"A","positive":"P","negative":"N","gap_nm":-1}]}"#,
            r#"{"format":"phoenix-diffpairs/1","pairs":[{"name":"A","positive":"P","negative":"N","colour":1}]}"#,
            r#"{"format":"phoenix-diffpairs/1","pairs":[
                {"name":"A","positive":"P","negative":"N"},{"name":"A","positive":"Q","negative":"M"}]}"#,
        ] {
            let error = config.merge_json(text).unwrap_err();
            assert_eq!(error.code, PAIRS_INVALID, "{text}");
        }
        assert!(config.pairs.is_empty());
    }

    #[test]
    fn a_pair_takes_the_defaults_and_finds_its_nets_by_name() {
        let f = fixture();
        let config = PairConfig::default();
        let def = PairDef {
            gap_nm: Some(300_000),
            ..PairDef::new("sig")
        };
        let missing = resolve(&f.board, &def, &config.settings).unwrap_err();
        assert_eq!(missing.code, NET_MISSING);
        let def = PairDef {
            positive: "SIG".to_owned(),
            negative: "GND".to_owned(),
            gap_nm: Some(300_000),
            ..PairDef::new("sig")
        };
        let pair = resolve(&f.board, &def, &config.settings).unwrap();
        assert_eq!(pair.positive, f.sig);
        assert_eq!(pair.negative, f.gnd);
        assert_eq!(pair.gap_nm, 300_000);
        assert_eq!(pair.width_nm, config.settings.default_width_nm);
        let lost = PairDef {
            negative: "NOPE".to_owned(),
            ..def
        };
        assert_eq!(
            resolve(&f.board, &lost, &config.settings).unwrap_err().code,
            NET_MISSING
        );
    }
}
