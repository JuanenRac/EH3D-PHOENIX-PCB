// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of an inventory, read from a plain `key = value` file. Defaults are conservative.

use crate::error::CorpusError;

/// Bounds and switches of an inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryConfig {
    /// Most files the inventory will look at.
    pub max_files: usize,
    /// Deepest folder nesting it will enter.
    pub max_depth: usize,
    /// How many bytes at the start of a design file are looked at to tell its kind.
    pub sniff_bytes: usize,
    /// Largest design file that is read whole for the fingerprint; a larger one is counted as oversize.
    pub max_file_bytes: u64,
    /// When true, the report carries one fingerprint of the content of all design files.
    pub fingerprint: bool,
}

impl Default for InventoryConfig {
    fn default() -> Self {
        InventoryConfig {
            max_files: 200_000,
            max_depth: 32,
            sniff_bytes: 8192,
            max_file_bytes: 256 * 1024 * 1024,
            fingerprint: true,
        }
    }
}

impl InventoryConfig {
    /// Parse a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<InventoryConfig, CorpusError> {
        let mut config = InventoryConfig::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or(CorpusError::ConfigSyntax { line })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || CorpusError::ConfigBadValue {
                line,
                key: key.to_string(),
            };
            match key {
                "max_files" => config.max_files = value.parse().map_err(|_| bad())?,
                "max_depth" => config.max_depth = value.parse().map_err(|_| bad())?,
                "sniff_bytes" => {
                    config.sniff_bytes = value.parse().map_err(|_| bad())?;
                    if config.sniff_bytes < 64 {
                        return Err(bad());
                    }
                }
                "max_file_bytes" => config.max_file_bytes = value.parse().map_err(|_| bad())?,
                "fingerprint" => {
                    config.fingerprint = match value {
                        "true" => true,
                        "false" => false,
                        _ => return Err(bad()),
                    }
                }
                _ => {
                    return Err(CorpusError::ConfigUnknownKey {
                        line,
                        key: key.to_string(),
                    })
                }
            }
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        assert_eq!(
            InventoryConfig::parse("# nothing\n").unwrap(),
            InventoryConfig::default()
        );
        let config = InventoryConfig::parse("max_files = 10\nfingerprint = false\n").unwrap();
        assert_eq!(config.max_files, 10);
        assert!(!config.fingerprint);
        assert_eq!(config.max_depth, InventoryConfig::default().max_depth);
    }

    #[test]
    fn bad_lines_are_refused_with_their_number() {
        assert!(matches!(
            InventoryConfig::parse("max_files = ten"),
            Err(CorpusError::ConfigBadValue { line: 1, .. })
        ));
        assert!(matches!(
            InventoryConfig::parse("\nfingerprint = 1"),
            Err(CorpusError::ConfigBadValue { line: 2, .. })
        ));
        assert!(matches!(
            InventoryConfig::parse("sniff_bytes = 8"),
            Err(CorpusError::ConfigBadValue { line: 1, .. })
        ));
        assert!(matches!(
            InventoryConfig::parse("colour = red"),
            Err(CorpusError::ConfigUnknownKey { line: 1, .. })
        ));
        assert!(matches!(
            InventoryConfig::parse("oops"),
            Err(CorpusError::ConfigSyntax { line: 1 })
        ));
    }
}
