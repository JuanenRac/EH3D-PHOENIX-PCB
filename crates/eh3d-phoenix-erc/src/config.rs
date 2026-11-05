// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the check: how serious each rule is, the matrix of pin types, which pin types
//! drive a net and supply a power input, and the bounds. They are read from `config/erc.cfg`;
//! that file is also built into the program as the default, so no value is written in the code of
//! the checks.

use eh3d_phoenix_rules::Severity;
use eh3d_phoenix_schematic::PinKind;

use crate::codes::{Rule, SETTINGS_INVALID};

/// The largest settings file that is read, in bytes.
pub const MAX_SETTINGS_BYTES: usize = 1 << 20;

/// The settings file of the repository, which is the default.
const BUILT_IN: &str = include_str!("../../../config/erc.cfg");

/// How serious a rule or a pair of pin types is. `Off` turns it off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Allowed, or the rule is off.
    Off,
    /// Worth knowing.
    Info,
    /// Probably wrong.
    Warning,
    /// Wrong.
    Error,
}

impl Level {
    /// The severity of a finding of this level, or `None` for `Off`.
    pub fn severity(self) -> Option<Severity> {
        match self {
            Level::Off => None,
            Level::Info => Some(Severity::Info),
            Level::Warning => Some(Severity::Warning),
            Level::Error => Some(Severity::Error),
        }
    }

    /// Reads a level (`off`, `ok`, `info`, `warning`, `error`).
    pub fn from_name(name: &str) -> Option<Level> {
        match name {
            "off" | "ok" => Some(Level::Off),
            "info" => Some(Level::Info),
            "warning" => Some(Level::Warning),
            "error" => Some(Level::Error),
            _ => None,
        }
    }
}

/// Why a settings file could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    /// The file is larger than [`MAX_SETTINGS_BYTES`].
    #[error("the settings file is {size} bytes, the limit is {limit}")]
    TooLarge {
        /// Its size.
        size: usize,
        /// The limit.
        limit: usize,
    },
    /// A line is not valid.
    #[error("line {line}: {message}")]
    Line {
        /// The line, from 1.
        line: usize,
        /// What is wrong.
        message: String,
    },
}

impl SettingsError {
    /// The stable code of this error.
    pub fn code(&self) -> &'static str {
        SETTINGS_INVALID
    }
}

/// The settings of the electrical rule check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErcConfig {
    levels: [Level; Rule::ALL.len()],
    matrix: [[Level; PinKind::ALL.len()]; PinKind::ALL.len()],
    /// The pin types that drive a signal.
    pub drivers: Vec<PinKind>,
    /// The pin types that supply a power input.
    pub power_sources: Vec<PinKind>,
    /// Net names with a stem shorter than this are never compared with each other.
    pub similar_name_min_stem: usize,
    /// The most findings listed.
    pub max_findings: usize,
}

impl Default for ErcConfig {
    /// The settings file of the repository (`config/erc.cfg`).
    fn default() -> Self {
        let mut config = ErcConfig::empty();
        if config.apply(BUILT_IN).is_err() {
            return ErcConfig::empty();
        }
        config
    }
}

fn parse_kinds(value: &str) -> Option<Vec<PinKind>> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        out.push(PinKind::from_name(part)?);
    }
    Some(out)
}

impl ErcConfig {
    /// Every rule off, every pair of pin types allowed, no driver, no source.
    fn empty() -> ErcConfig {
        ErcConfig {
            levels: [Level::Off; Rule::ALL.len()],
            matrix: [[Level::Off; PinKind::ALL.len()]; PinKind::ALL.len()],
            drivers: Vec::new(),
            power_sources: Vec::new(),
            similar_name_min_stem: 3,
            max_findings: 10_000,
        }
    }

    /// Reads a settings text on top of the default: a line that is left out keeps the default.
    /// Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<ErcConfig, SettingsError> {
        let mut config = ErcConfig::default();
        config.apply(text)?;
        Ok(config)
    }

    fn apply(&mut self, text: &str) -> Result<(), SettingsError> {
        if text.len() > MAX_SETTINGS_BYTES {
            return Err(SettingsError::TooLarge {
                size: text.len(),
                limit: MAX_SETTINGS_BYTES,
            });
        }
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let fail = |message: String| SettingsError::Line { line, message };
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| fail("not key = value".to_owned()))?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || fail(format!("the value of {key} is not valid"));
            if let Some(name) = key.strip_prefix("severity.") {
                let rule =
                    Rule::from_name(name).ok_or_else(|| fail(format!("unknown rule {name}")))?;
                self.levels[rule.index()] = Level::from_name(value).ok_or_else(bad)?;
            } else if let Some(pair) = key.strip_prefix("matrix.") {
                let (a, b) = pair
                    .split_once('.')
                    .ok_or_else(|| fail("a matrix key is matrix.<kind>.<kind>".to_owned()))?;
                let (a, b) = (
                    PinKind::from_name(a).ok_or_else(|| fail(format!("unknown pin type {a}")))?,
                    PinKind::from_name(b).ok_or_else(|| fail(format!("unknown pin type {b}")))?,
                );
                let level = Level::from_name(value).ok_or_else(bad)?;
                self.matrix[a.index()][b.index()] = level;
                self.matrix[b.index()][a.index()] = level;
            } else {
                match key {
                    "drivers" => self.drivers = parse_kinds(value).ok_or_else(bad)?,
                    "power_sources" => self.power_sources = parse_kinds(value).ok_or_else(bad)?,
                    "similar_name_min_stem" => {
                        self.similar_name_min_stem = value.parse().map_err(|_| bad())?;
                    }
                    "max_findings" => {
                        let n: usize = value.parse().map_err(|_| bad())?;
                        if n == 0 {
                            return Err(bad());
                        }
                        self.max_findings = n;
                    }
                    _ => return Err(fail(format!("unknown key {key}"))),
                }
            }
        }
        Ok(())
    }

    /// The level of a rule.
    pub fn level(&self, rule: Rule) -> Level {
        self.levels[rule.index()]
    }

    /// The level of two pins of these types on one net.
    pub fn pair_level(&self, a: PinKind, b: PinKind) -> Level {
        self.matrix[a.index()][b.index()]
    }

    /// Sets the level of a rule.
    pub fn set_level(&mut self, rule: Rule, level: Level) {
        self.levels[rule.index()] = level;
    }

    /// Sets the level of two pin types on one net (in both orders).
    pub fn set_pair_level(&mut self, a: PinKind, b: PinKind, level: Level) {
        self.matrix[a.index()][b.index()] = level;
        self.matrix[b.index()][a.index()] = level;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_of_the_repository_is_the_default_and_is_valid() {
        let mut from_file = ErcConfig::empty();
        from_file.apply(BUILT_IN).unwrap();
        assert_eq!(from_file, ErcConfig::default());
        assert_ne!(ErcConfig::default(), ErcConfig::empty());
        let c = ErcConfig::default();
        assert_eq!(c.level(Rule::PinUnconnected), Level::Error);
        assert_eq!(c.level(Rule::SinglePinNet), Level::Warning);
        assert_eq!(c.pair_level(PinKind::Output, PinKind::Output), Level::Error);
        assert_eq!(
            c.pair_level(PinKind::Output, PinKind::Bidirectional),
            Level::Warning
        );
        assert_eq!(c.pair_level(PinKind::Input, PinKind::Output), Level::Off);
        assert!(c.drivers.contains(&PinKind::Output));
        assert_eq!(c.power_sources, vec![PinKind::PowerOut]);
    }

    #[test]
    fn the_matrix_of_the_file_is_symmetric_and_every_rule_is_set() {
        let c = ErcConfig::default();
        for a in PinKind::ALL {
            for b in PinKind::ALL {
                assert_eq!(c.pair_level(a, b), c.pair_level(b, a), "{a:?} {b:?}");
            }
        }
        for rule in Rule::ALL {
            assert!(
                BUILT_IN.contains(&format!("severity.{} =", rule.name())),
                "{}",
                rule.name()
            );
        }
    }

    #[test]
    fn a_line_overrides_the_default_and_the_rest_stays() {
        let c = ErcConfig::parse(
            "severity.pin_unconnected = warning\nmatrix.input.input = error\ndrivers = output\nmax_findings = 5\n",
        )
        .unwrap();
        assert_eq!(c.level(Rule::PinUnconnected), Level::Warning);
        assert_eq!(c.level(Rule::SinglePinNet), Level::Warning);
        assert_eq!(c.pair_level(PinKind::Input, PinKind::Input), Level::Error);
        assert_eq!(c.drivers, vec![PinKind::Output]);
        assert_eq!(c.max_findings, 5);
        assert_eq!(
            ErcConfig::parse("# nothing\n").unwrap(),
            ErcConfig::default()
        );
    }

    #[test]
    fn bad_settings_name_their_line_and_never_panic() {
        for (text, line) in [
            ("nonsense", 1),
            ("\nseverity.nothing = error", 2),
            ("severity.empty_value = loud", 1),
            ("matrix.input = error", 1),
            ("matrix.input.nothing = error", 1),
            ("drivers = output, nothing", 1),
            ("max_findings = 0", 1),
            ("similar_name_min_stem = x", 1),
            ("unknown_key = 1", 1),
        ] {
            match ErcConfig::parse(text) {
                Err(SettingsError::Line { line: found, .. }) => assert_eq!(found, line, "{text}"),
                other => panic!("{text}: {other:?}"),
            }
        }
        let big = " ".repeat(MAX_SETTINGS_BYTES + 1);
        assert!(matches!(
            ErcConfig::parse(&big),
            Err(SettingsError::TooLarge { .. })
        ));
        assert_eq!(
            SettingsError::TooLarge { size: 0, limit: 0 }.code(),
            SETTINGS_INVALID
        );
    }
}
