// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the spatial index, read from `config/spatial.cfg` (which is also the default
//! built into the program). The file is strict: a key the program does not know, a key written
//! twice, a line that is not `key = value` and a value out of range are refused with the code
//! `PHOENIX-SPATIAL-001`. A line that is left out keeps the default.

use crate::codes::{SpatialError, SETTINGS_INVALID};

/// The largest settings file that is read, in bytes.
pub const MAX_CONFIG_BYTES: usize = 1 << 16;

/// The settings of the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpatialConfig {
    /// The size of a cell in nanometres; 0 chooses it from the boxes.
    pub cell_nm: i64,
    /// The most entries the grid may hold (one for each cell that an item covers).
    pub max_cells: u64,
    /// An item that covers more cells than this is kept in the list of big items.
    pub max_cells_per_item: u32,
    /// The most items one index accepts.
    pub max_items: u64,
}

impl Default for SpatialConfig {
    /// The values of `config/spatial.cfg`.
    fn default() -> Self {
        SpatialConfig {
            cell_nm: 0,
            max_cells: 4_000_000,
            max_cells_per_item: 4096,
            max_items: 4_000_000,
        }
    }
}

fn fail(line: usize, message: &str) -> SpatialError {
    SpatialError::new(SETTINGS_INVALID, format!("line {line}: {message}"))
}

impl SpatialConfig {
    /// Reads the settings from the text of a file.
    pub fn parse(text: &str) -> Result<SpatialConfig, SpatialError> {
        if text.len() > MAX_CONFIG_BYTES {
            return Err(SpatialError::new(
                SETTINGS_INVALID,
                format!("the file is larger than {MAX_CONFIG_BYTES} bytes"),
            ));
        }
        let mut config = SpatialConfig::default();
        let mut seen: Vec<&str> = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((key, value)) = trimmed.split_once('=') else {
                return Err(fail(line, "not key = value"));
            };
            let (key, value) = (key.trim(), value.trim());
            if seen.contains(&key) {
                return Err(fail(line, &format!("{key} is written twice")));
            }
            seen.push(key);
            let bad = || fail(line, &format!("the value of {key} is not valid"));
            match key {
                "cell_nm" => {
                    config.cell_nm = value
                        .parse::<i64>()
                        .ok()
                        .filter(|n| (0..=1_i64 << 40).contains(n))
                        .ok_or_else(bad)?;
                }
                "max_cells" => {
                    config.max_cells = value
                        .parse::<u64>()
                        .ok()
                        .filter(|n| (1..=1_u64 << 28).contains(n))
                        .ok_or_else(bad)?;
                }
                "max_cells_per_item" => {
                    config.max_cells_per_item = value
                        .parse::<u32>()
                        .ok()
                        .filter(|n| (1..=1_u32 << 20).contains(n))
                        .ok_or_else(bad)?;
                }
                "max_items" => {
                    config.max_items = value
                        .parse::<u64>()
                        .ok()
                        .filter(|n| (1..=1_u64 << 28).contains(n))
                        .ok_or_else(bad)?;
                }
                _ => return Err(fail(line, &format!("unknown key {key}"))),
            }
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_file_parses_to_the_defaults() {
        let shipped = include_str!("../../../config/spatial.cfg");
        assert_eq!(
            SpatialConfig::parse(shipped).unwrap(),
            SpatialConfig::default()
        );
    }

    #[test]
    fn a_line_changes_one_value_and_the_rest_keep_the_default() {
        let c = SpatialConfig::parse("# note\ncell_nm = 2500\n\nmax_items = 10\n").unwrap();
        assert_eq!(c.cell_nm, 2500);
        assert_eq!(c.max_items, 10);
        assert_eq!(c.max_cells, SpatialConfig::default().max_cells);
    }

    #[test]
    fn unknown_keys_repeated_keys_and_values_out_of_range_are_refused() {
        for text in [
            "cell = 5\n",
            "cell_nm = 5\ncell_nm = 6\n",
            "cell_nm = -1\n",
            "cell_nm = 1099511627777\n",
            "max_cells = 0\n",
            "max_cells = many\n",
            "max_cells_per_item = 0\n",
            "max_items = 268435457\n",
            "just words\n",
        ] {
            let e = SpatialConfig::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.starts_with("line "), "{text}: {}", e.detail);
        }
        assert!(SpatialConfig::parse(&"#".repeat(MAX_CONFIG_BYTES + 1)).is_err());
    }
}
