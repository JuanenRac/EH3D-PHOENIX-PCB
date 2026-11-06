// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the advanced checks, read from `config/drc_advanced.cfg` (which is also built
//! into the program as the default). Nothing that is tuned is written in the code of the checks.
//!
//! The file is strict: a key the program does not know, a key written twice (except `iso_row`,
//! which is one line for each row of the table) and a value out of range are refused with the code
//! `PHOENIX-DRC-015`. A line that is left out keeps the default.

use std::collections::{BTreeMap, BTreeSet};

use crate::advanced::codes::CONFIG_INVALID;
use crate::report::Severity;

/// The largest settings file that is read, in bytes.
pub const MAX_CONFIG_BYTES: usize = 1 << 20;

/// The largest length in a settings file, in nanometres.
const MAX_NM: i64 = 1 << 30;

/// The settings file of the repository, which is the default.
const BUILT_IN: &str = include_str!("../../../../config/drc_advanced.cfg");

/// The names of the seven columns of the table of spacing, in order.
const COLUMNS: [&str; 7] = ["b1", "b2", "b3", "b4", "a5", "a6", "a7"];

/// A settings file that cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AdvancedError {
    /// The stable code (`PHOENIX-DRC-015`).
    pub code: &'static str,
    /// What is wrong, in English, with the line.
    pub message: String,
}

/// A drill limit of its own for the nets of a class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrillClass {
    /// The name of the class.
    pub name: String,
    /// The names of its nets.
    pub nets: BTreeSet<String>,
    /// The smallest drill of its nets, in nanometres.
    pub min_nm: i64,
}

/// One row of the table of spacing by voltage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsoRow {
    /// The highest voltage of the row, in volts.
    pub up_to_volts: i64,
    /// The spacing of each column, in nanometres.
    pub nm: [i64; 7],
}

/// The settings of the advanced checks. A level of `None` is a check that is off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedConfig {
    /// The size of a cell of the grid, in nanometres.
    pub grid_cell_nm: i64,
    /// Level of the annular ring check.
    pub annular_ring_level: Option<Severity>,
    /// Thinnest annular ring, in nanometres.
    pub annular_ring_min_nm: i64,
    /// Level of the copper to board edge check.
    pub copper_edge_level: Option<Severity>,
    /// Smallest distance from copper to the edge, in nanometres.
    pub copper_edge_min_nm: i64,
    /// Level of the pad to board edge check.
    pub pad_edge_level: Option<Severity>,
    /// Smallest distance from a pad to the edge, in nanometres.
    pub pad_edge_min_nm: i64,
    /// Level of the smallest drill by class check.
    pub drill_class_level: Option<Severity>,
    /// Smallest drill of the nets that belong to no class, in nanometres.
    pub drill_default_min_nm: i64,
    /// The classes, sorted by name.
    pub drill_classes: Vec<DrillClass>,
    /// Level of the silkscreen over a hole check.
    pub silk_hole_level: Option<Severity>,
    /// Smallest distance from silkscreen to a hole, in nanometres.
    pub silk_hole_min_nm: i64,
    /// Level of the neck check.
    pub neck_level: Option<Severity>,
    /// A track narrower than this that joins wider copper is a neck, in nanometres.
    pub neck_min_width_nm: i64,
    /// Level of the acid trap check.
    pub acid_trap_level: Option<Severity>,
    /// Smallest angle between pieces of tracks of one net, in degrees.
    pub acid_trap_min_angle_deg: i64,
    /// Level of the isolation by voltage check.
    pub isolation_level: Option<Severity>,
    /// The column of the table that is used (0 is `b1`, 6 is `a7`).
    pub iso_column: usize,
    /// The rows of the table, by rising voltage.
    pub iso_rows: Vec<IsoRow>,
    /// The spacing per volt above the last row, in nanometres, for each column.
    pub iso_per_volt_nm: [i64; 7],
    /// The voltage of the nets that are written, by name, in volts.
    pub voltages: BTreeMap<String, i64>,
    /// Level of the copper island check.
    pub island_level: Option<Severity>,
    /// Level of the stub check.
    pub stub_level: Option<Severity>,
    /// Level of the duplicate via check.
    pub via_duplicate_level: Option<Severity>,
    /// Vias this close (or closer) that share a layer are duplicates, in nanometres.
    pub via_duplicate_max_distance_nm: i64,
    /// Level of the pad on no net check.
    pub pad_no_net_level: Option<Severity>,
    /// Level of the part outside the outline check.
    pub outside_outline_level: Option<Severity>,
}

fn fail(line: usize, message: String) -> AdvancedError {
    AdvancedError {
        code: CONFIG_INVALID,
        message: format!("line {line}: {message}"),
    }
}

fn parse_level(value: &str) -> Option<Option<Severity>> {
    match value {
        "off" => Some(None),
        "info" => Some(Some(Severity::Info)),
        "warning" => Some(Some(Severity::Warning)),
        "error" => Some(Some(Severity::Error)),
        _ => None,
    }
}

fn parse_int(value: &str, min: i64, max: i64) -> Option<i64> {
    let n: i64 = value.parse().ok()?;
    (min..=max).contains(&n).then_some(n)
}

impl Default for AdvancedConfig {
    /// The settings file of the repository (`config/drc_advanced.cfg`).
    fn default() -> Self {
        let mut config = AdvancedConfig::all_off();
        if config.apply(BUILT_IN).is_err() {
            return AdvancedConfig::all_off();
        }
        config
    }
}

impl AdvancedConfig {
    /// Every check off.
    pub fn all_off() -> AdvancedConfig {
        AdvancedConfig {
            grid_cell_nm: 1_000_000,
            annular_ring_level: None,
            annular_ring_min_nm: 0,
            copper_edge_level: None,
            copper_edge_min_nm: 0,
            pad_edge_level: None,
            pad_edge_min_nm: 0,
            drill_class_level: None,
            drill_default_min_nm: 0,
            drill_classes: Vec::new(),
            silk_hole_level: None,
            silk_hole_min_nm: 0,
            neck_level: None,
            neck_min_width_nm: 0,
            acid_trap_level: None,
            acid_trap_min_angle_deg: 0,
            isolation_level: None,
            iso_column: 1,
            iso_rows: Vec::new(),
            iso_per_volt_nm: [0; 7],
            voltages: BTreeMap::new(),
            island_level: None,
            stub_level: None,
            via_duplicate_level: None,
            via_duplicate_max_distance_nm: 0,
            pad_no_net_level: None,
            outside_outline_level: None,
        }
    }

    /// Reads a settings text on top of the default: a line that is left out keeps the default.
    /// Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<AdvancedConfig, AdvancedError> {
        let mut config = AdvancedConfig::default();
        config.apply(text)?;
        Ok(config)
    }

    /// The spacing the table asks for between copper with a difference of `volts` between them,
    /// in nanometres.
    pub fn required_spacing_nm(&self, volts: i64) -> i64 {
        let v = volts.abs();
        for row in &self.iso_rows {
            if v <= row.up_to_volts {
                return row.nm[self.iso_column];
            }
        }
        self.iso_per_volt_nm[self.iso_column].saturating_mul(v)
    }

    /// The name of the column of the table that is used (`b2`).
    pub fn iso_column_name(&self) -> &'static str {
        COLUMNS.get(self.iso_column).copied().unwrap_or("b2")
    }

    /// The smallest drill for a net: the one of its class, or the default.
    pub fn drill_min_for(&self, net_name: Option<&str>) -> (i64, Option<&str>) {
        if let Some(name) = net_name {
            for class in &self.drill_classes {
                if class.nets.contains(name) {
                    return (class.min_nm, Some(class.name.as_str()));
                }
            }
        }
        (self.drill_default_min_nm, None)
    }

    fn apply(&mut self, text: &str) -> Result<(), AdvancedError> {
        if text.len() > MAX_CONFIG_BYTES {
            return Err(AdvancedError {
                code: CONFIG_INVALID,
                message: format!(
                    "the file is {} bytes, the limit is {MAX_CONFIG_BYTES}",
                    text.len()
                ),
            });
        }
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut rows_replaced = false;
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| fail(line, "not key = value".to_owned()))?;
            let (key, value) = (key.trim(), value.trim());
            if key != "iso_row" && !seen.insert(key.to_owned()) {
                return Err(fail(line, format!("the key {key} is written twice")));
            }
            let bad = || fail(line, format!("the value of {key} is not valid"));
            let level = || parse_level(value).ok_or_else(bad);
            let nm = || parse_int(value, 0, MAX_NM).ok_or_else(bad);
            match key {
                "grid_cell_nm" => {
                    self.grid_cell_nm = parse_int(value, 1, MAX_NM).ok_or_else(bad)?;
                }
                "annular_ring_level" => self.annular_ring_level = level()?,
                "annular_ring_min_nm" => self.annular_ring_min_nm = nm()?,
                "copper_edge_level" => self.copper_edge_level = level()?,
                "copper_edge_min_nm" => self.copper_edge_min_nm = nm()?,
                "pad_edge_level" => self.pad_edge_level = level()?,
                "pad_edge_min_nm" => self.pad_edge_min_nm = nm()?,
                "drill_class_level" => self.drill_class_level = level()?,
                "drill_default_min_nm" => self.drill_default_min_nm = nm()?,
                "silk_hole_level" => self.silk_hole_level = level()?,
                "silk_hole_min_nm" => self.silk_hole_min_nm = nm()?,
                "neck_level" => self.neck_level = level()?,
                "neck_min_width_nm" => self.neck_min_width_nm = nm()?,
                "acid_trap_level" => self.acid_trap_level = level()?,
                "acid_trap_min_angle_deg" => {
                    self.acid_trap_min_angle_deg = parse_int(value, 0, 180).ok_or_else(bad)?;
                }
                "isolation_level" => self.isolation_level = level()?,
                "iso_column" => {
                    let lower = value.to_ascii_lowercase();
                    self.iso_column = COLUMNS.iter().position(|c| *c == lower).ok_or_else(bad)?;
                }
                "iso_row" => {
                    if !rows_replaced {
                        rows_replaced = true;
                        self.iso_rows.clear();
                    }
                    let numbers: Option<Vec<i64>> = value
                        .split_whitespace()
                        .map(|t| parse_int(t, 0, MAX_NM))
                        .collect();
                    let numbers = numbers.filter(|n| n.len() == 8).ok_or_else(bad)?;
                    let mut nm_columns = [0i64; 7];
                    nm_columns.copy_from_slice(&numbers[1..]);
                    if self
                        .iso_rows
                        .last()
                        .is_some_and(|last| last.up_to_volts >= numbers[0])
                    {
                        return Err(fail(
                            line,
                            "the rows of the table must rise in voltage".to_owned(),
                        ));
                    }
                    self.iso_rows.push(IsoRow {
                        up_to_volts: numbers[0],
                        nm: nm_columns,
                    });
                }
                "iso_per_volt_nm" => {
                    let numbers: Option<Vec<i64>> = value
                        .split_whitespace()
                        .map(|t| parse_int(t, 0, MAX_NM))
                        .collect();
                    let numbers = numbers.filter(|n| n.len() == 7).ok_or_else(bad)?;
                    self.iso_per_volt_nm.copy_from_slice(&numbers);
                }
                "island_level" => self.island_level = level()?,
                "stub_level" => self.stub_level = level()?,
                "via_duplicate_level" => self.via_duplicate_level = level()?,
                "via_duplicate_max_distance_nm" => self.via_duplicate_max_distance_nm = nm()?,
                "pad_no_net_level" => self.pad_no_net_level = level()?,
                "outside_outline_level" => self.outside_outline_level = level()?,
                _ => {
                    if let Some(net) = key.strip_prefix("voltage.") {
                        if net.is_empty() {
                            return Err(bad());
                        }
                        let volts = parse_int(value, -100_000, 100_000).ok_or_else(bad)?;
                        self.voltages.insert(net.to_owned(), volts);
                    } else if let Some(rest) = key.strip_prefix("drill_class.") {
                        self.apply_class(line, rest, value)?;
                    } else {
                        return Err(fail(line, format!("unknown key {key}")));
                    }
                }
            }
        }
        self.finish()
    }

    fn apply_class(&mut self, line: usize, rest: &str, value: &str) -> Result<(), AdvancedError> {
        let bad = || {
            fail(
                line,
                format!("the value of drill_class.{rest} is not valid"),
            )
        };
        let (name, field) = rest.rsplit_once('.').ok_or_else(bad)?;
        if name.is_empty() {
            return Err(bad());
        }
        let at = match self.drill_classes.iter().position(|c| c.name == name) {
            Some(i) => i,
            None => {
                self.drill_classes.push(DrillClass {
                    name: name.to_owned(),
                    nets: BTreeSet::new(),
                    min_nm: 0,
                });
                self.drill_classes.len() - 1
            }
        };
        match field {
            "nets" => {
                let nets: BTreeSet<String> = value
                    .split(',')
                    .map(|n| n.trim().to_owned())
                    .filter(|n| !n.is_empty())
                    .collect();
                if nets.is_empty() {
                    return Err(bad());
                }
                self.drill_classes[at].nets = nets;
            }
            "min_nm" => {
                self.drill_classes[at].min_nm = parse_int(value, 0, MAX_NM).ok_or_else(bad)?;
            }
            _ => return Err(fail(line, format!("unknown key drill_class.{rest}"))),
        }
        Ok(())
    }

    /// The checks that need the whole file: classes in order, no net in two classes, a table when
    /// the isolation check is on.
    fn finish(&mut self) -> Result<(), AdvancedError> {
        self.drill_classes.sort_by_key(|c| c.name.clone());
        let mut owner: BTreeMap<&str, &str> = BTreeMap::new();
        for class in &self.drill_classes {
            if class.nets.is_empty() {
                return Err(AdvancedError {
                    code: CONFIG_INVALID,
                    message: format!("the class {} has no nets", class.name),
                });
            }
            for net in &class.nets {
                if let Some(other) = owner.insert(net.as_str(), class.name.as_str()) {
                    return Err(AdvancedError {
                        code: CONFIG_INVALID,
                        message: format!(
                            "the net {net} is in the classes {other} and {}",
                            class.name
                        ),
                    });
                }
            }
        }
        if self.isolation_level.is_some() && self.iso_rows.is_empty() {
            return Err(AdvancedError {
                code: CONFIG_INVALID,
                message: "the isolation check is on but the table has no rows".to_owned(),
            });
        }
        Ok(())
    }
}
