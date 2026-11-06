// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the library of footprints (`config/ipc.cfg`) and of the bulk import
//! (`config/import_bulk.cfg`). Every limit and tolerance is read from a plain `key = value` text
//! and not buried in the code. A missing line keeps the default; an unknown key or a value out of
//! range is refused with its line number.

use crate::codes::{IpcError, IDX_SETTINGS_INVALID, SETTINGS_INVALID};
use crate::family::Density;

/// The text of `config/ipc.cfg`, the values the library starts with.
pub const DEFAULT_IPC_CONFIG: &str = include_str!("../../../config/ipc.cfg");

/// The text of `config/import_bulk.cfg`, the values the bulk import starts with.
pub const DEFAULT_BULK_CONFIG: &str = include_str!("../../../config/import_bulk.cfg");

/// Lengths in millimetres as nanometres.
fn nm(mm: f64) -> i64 {
    (mm * 1_000_000.0).round() as i64
}

/// The `key = value` lines of a text, with their numbers; comments and blank lines are left out.
fn pairs(text: &str) -> Result<Vec<(usize, String, String)>, (usize, String)> {
    let mut out = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return Err((line, "not key = value".to_owned()));
        };
        out.push((line, key.trim().to_owned(), value.trim().to_owned()));
    }
    Ok(out)
}

/// A length in millimetres that is not negative and not above a limit.
fn length(value: &str, key: &str, max: f64) -> Result<i64, String> {
    let v: f64 = value
        .parse()
        .map_err(|_| format!("the value of {key} is not a number"))?;
    if !v.is_finite() || !(0.0..=max).contains(&v) {
        return Err(format!("the value of {key} must be from 0 to {max} mm"));
    }
    Ok(nm(v))
}

/// A whole number from a minimum to a maximum.
fn whole(value: &str, key: &str, min: u64, max: u64) -> Result<u64, String> {
    let v: u64 = value
        .parse()
        .map_err(|_| format!("the value of {key} is not a whole number"))?;
    if !(min..=max).contains(&v) {
        return Err(format!("the value of {key} must be from {min} to {max}"));
    }
    Ok(v)
}

/// The settings of the generator, of the checker and of the comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct IpcSettings {
    /// The least distance from the silkscreen to a pad, in nanometres.
    pub silk_clearance_nm: i64,
    /// How far the middle of the pads may be from the origin, in nanometres.
    pub origin_tol_nm: i64,
    /// The tolerance of the distance between pads against the pitch, in nanometres.
    pub pitch_tol_nm: i64,
    /// The tolerance of the courtyard against the one the density asks for, in nanometres.
    pub court_tol_nm: i64,
    /// The largest percentage of paste on an exposed pad.
    pub ep_paste_max: u32,
    /// The tolerance of the comparison with a footprint of the owner, in nanometres.
    pub compare_tol_nm: i64,
    /// The density level used when none is asked for.
    pub default_density: Density,
    /// The largest family file that is read, in bytes.
    pub max_family_bytes: u64,
    /// The most family files that are read from a folder.
    pub max_family_files: usize,
    /// The largest library or footprint file that is read, in bytes.
    pub max_file_bytes: u64,
}

impl Default for IpcSettings {
    fn default() -> Self {
        IpcSettings {
            silk_clearance_nm: 150_000,
            origin_tol_nm: 5_000,
            pitch_tol_nm: 2_000,
            court_tol_nm: 1_000,
            ep_paste_max: 90,
            compare_tol_nm: 50_000,
            default_density: Density::B,
            max_family_bytes: 1024 * 1024,
            max_family_files: 64,
            max_file_bytes: 64 * 1024 * 1024,
        }
    }
}

impl IpcSettings {
    /// Parses a settings text.
    pub fn parse(text: &str) -> Result<IpcSettings, IpcError> {
        let fail = |line: usize, what: String| {
            IpcError::new(SETTINGS_INVALID, format!("line {line}: {what}"))
        };
        let mut s = IpcSettings::default();
        let list = pairs(text).map_err(|(line, what)| fail(line, what))?;
        for (line, key, value) in list {
            let bad = |what: String| fail(line, what);
            match key.as_str() {
                "silk_clearance_mm" => {
                    s.silk_clearance_nm = length(&value, &key, 5.0).map_err(bad)?
                }
                "origin_tol_mm" => s.origin_tol_nm = length(&value, &key, 5.0).map_err(bad)?,
                "pitch_tol_mm" => s.pitch_tol_nm = length(&value, &key, 1.0).map_err(bad)?,
                "courtyard_tol_mm" => s.court_tol_nm = length(&value, &key, 1.0).map_err(bad)?,
                "ep_paste_max_percent" => {
                    s.ep_paste_max = whole(&value, &key, 1, 100).map_err(bad)? as u32
                }
                "compare_tol_mm" => s.compare_tol_nm = length(&value, &key, 5.0).map_err(bad)?,
                "default_density" => {
                    s.default_density = Density::parse(&value)
                        .ok_or_else(|| fail(line, "the density must be A, B or C".to_owned()))?
                }
                "max_family_bytes" => {
                    s.max_family_bytes = whole(&value, &key, 1024, 1 << 30).map_err(bad)?
                }
                "max_family_files" => {
                    s.max_family_files = whole(&value, &key, 1, 4096).map_err(bad)? as usize
                }
                "max_file_bytes" => {
                    s.max_file_bytes = whole(&value, &key, 1024, 1 << 34).map_err(bad)?
                }
                other => return Err(fail(line, format!("unknown key {other}"))),
            }
        }
        Ok(s)
    }

    /// The values of `config/ipc.cfg` as they were when the program was built.
    pub fn builtin() -> IpcSettings {
        IpcSettings::parse(DEFAULT_IPC_CONFIG).unwrap_or_default()
    }
}

/// The settings of the bulk import of libraries.
#[derive(Debug, Clone, PartialEq)]
pub struct BulkSettings {
    /// The most library files read from a folder.
    pub max_files: usize,
    /// The largest library file that is read, in bytes.
    pub max_file_bytes: u64,
    /// The deepest level of folders below the one given.
    pub max_depth: usize,
    /// The most footprints and symbols that are indexed.
    pub max_items: usize,
    /// The grid to which the geometry of a pad is rounded before it is compared, in nanometres.
    pub geometry_round_nm: i64,
    /// The distance from the pads beyond which the origin of a footprint is reported, in
    /// nanometres.
    pub origin_far_nm: i64,
}

impl Default for BulkSettings {
    fn default() -> Self {
        BulkSettings {
            max_files: 5000,
            max_file_bytes: 64 * 1024 * 1024,
            max_depth: 16,
            max_items: 500_000,
            geometry_round_nm: 10_000,
            origin_far_nm: 10_000_000,
        }
    }
}

impl BulkSettings {
    /// Parses a settings text.
    pub fn parse(text: &str) -> Result<BulkSettings, IpcError> {
        let fail = |line: usize, what: String| {
            IpcError::new(IDX_SETTINGS_INVALID, format!("line {line}: {what}"))
        };
        let mut s = BulkSettings::default();
        let list = pairs(text).map_err(|(line, what)| fail(line, what))?;
        for (line, key, value) in list {
            let bad = |what: String| fail(line, what);
            match key.as_str() {
                "max_files" => {
                    s.max_files = whole(&value, &key, 1, 1_000_000).map_err(bad)? as usize
                }
                "max_file_bytes" => {
                    s.max_file_bytes = whole(&value, &key, 1024, 1 << 34).map_err(bad)?
                }
                "max_depth" => s.max_depth = whole(&value, &key, 0, 64).map_err(bad)? as usize,
                "max_items" => {
                    s.max_items = whole(&value, &key, 1, 100_000_000).map_err(bad)? as usize
                }
                "geometry_round_mm" => {
                    let v = length(&value, &key, 1.0).map_err(bad)?;
                    if v == 0 {
                        return Err(fail(
                            line,
                            "geometry_round_mm must be above zero".to_owned(),
                        ));
                    }
                    s.geometry_round_nm = v;
                }
                "origin_far_mm" => s.origin_far_nm = length(&value, &key, 1000.0).map_err(bad)?,
                other => return Err(fail(line, format!("unknown key {other}"))),
            }
        }
        Ok(s)
    }

    /// The values of `config/import_bulk.cfg` as they were when the program was built.
    pub fn builtin() -> BulkSettings {
        BulkSettings::parse(DEFAULT_BULK_CONFIG).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_of_the_repository_parse_and_hold_the_defaults() {
        assert_eq!(
            IpcSettings::parse(DEFAULT_IPC_CONFIG).unwrap(),
            IpcSettings::default()
        );
        assert_eq!(
            BulkSettings::parse(DEFAULT_BULK_CONFIG).unwrap(),
            BulkSettings::default()
        );
    }

    #[test]
    fn a_line_changes_one_value_and_a_bad_line_is_refused_with_its_number() {
        let s =
            IpcSettings::parse("# note\n\ncompare_tol_mm = 0.1\ndefault_density = c\n").unwrap();
        assert_eq!(s.compare_tol_nm, 100_000);
        assert_eq!(s.default_density, Density::C);
        let e = IpcSettings::parse("silk_clearance_mm = 0.2\nnothing = 1\n").unwrap_err();
        assert!(e.detail.starts_with("line 2:"), "{e}");
        let e = IpcSettings::parse("ep_paste_max_percent = 120\n").unwrap_err();
        assert_eq!(e.code, SETTINGS_INVALID);
        let e = IpcSettings::parse("origin_tol_mm\n").unwrap_err();
        assert!(e.detail.contains("not key = value"));
    }

    #[test]
    fn the_bulk_settings_refuse_a_zero_grid_and_unknown_keys() {
        assert!(BulkSettings::parse("geometry_round_mm = 0\n").is_err());
        let e = BulkSettings::parse("max_files = 10\nfoo = 1\n").unwrap_err();
        assert_eq!(e.code, IDX_SETTINGS_INVALID);
        assert_eq!(
            BulkSettings::parse("max_files = 10\n").unwrap().max_files,
            10
        );
    }
}
