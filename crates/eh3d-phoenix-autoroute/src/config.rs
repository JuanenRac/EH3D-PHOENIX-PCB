// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the autorouter, read from `config/autoroute_native.cfg` (which is also the
//! default built into the program). The file is strict: a key the program does not know, a key
//! written twice, a line that is not `key = value` and a value out of range are refused with the
//! code `PHOENIX-AROUTE-001`. A line that is left out keeps the default. Every pitch, penalty and
//! limit is here and not in the code; the limits count nodes and passes, never time.

use crate::codes::{AutorouteError, SETTINGS_INVALID};

/// The largest settings file that is read, in bytes.
pub const MAX_CONFIG_BYTES: usize = 1 << 16;

/// In which order the nets are routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetOrder {
    /// The nets whose connections are shortest in total first.
    Shortest,
    /// The nets with the widest tracks first, then the shortest.
    Widest,
    /// The nets named in `critical_nets` first, then the shortest.
    Critical,
}

impl NetOrder {
    /// The word of the settings file.
    pub fn word(self) -> &'static str {
        match self {
            NetOrder::Shortest => "shortest",
            NetOrder::Widest => "widest",
            NetOrder::Critical => "critical",
        }
    }

    fn from_word(word: &str) -> Option<NetOrder> {
        match word {
            "shortest" => Some(NetOrder::Shortest),
            "widest" => Some(NetOrder::Widest),
            "critical" => Some(NetOrder::Critical),
            _ => None,
        }
    }
}

/// The settings of the autorouter. Lengths are in nanometres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutorouteConfig {
    /// The pitch of the routing grid.
    pub grid_nm: i64,
    /// The order of the nets.
    pub order: NetOrder,
    /// The nets that go first with the order `critical`, by name.
    pub critical_nets: Vec<String>,
    /// Whether tracks may run at 45 degrees.
    pub diagonal: bool,
    /// The cost of one corner.
    pub corner_penalty_nm: i64,
    /// The cost of one via.
    pub via_penalty_nm: i64,
    /// How much more a step against the preferred direction of its layer costs, in percent.
    pub wrong_way_percent: i64,
    /// The cost of a step through the copper of another net when rip-up looks for the nets that
    /// are in the way.
    pub ripup_penalty_nm: i64,
    /// The cost added to a node of a route that was torn up, for each time it was.
    pub history_penalty_nm: i64,
    /// The most nodes one search may expand.
    pub max_expansions_per_search: u64,
    /// The most nodes all the searches of a run may expand.
    pub max_expansions_total: u64,
    /// The most times a route is torn up to make room for another.
    pub max_ripup_iterations: u32,
    /// The most times one connection may be torn up.
    pub max_rips_per_connection: u32,
    /// The most nodes of the grid (nodes of one layer times layers).
    pub max_nodes: u64,
    /// The most connections of a run.
    pub max_connections: usize,
    /// How many places near a pad are tried as the start or the end of a track.
    pub terminal_candidates: usize,
    /// How far from a pad, in grid pitches beyond its size, those places are looked for.
    pub terminal_radius_cells: i64,
    /// Whether the copper zones of other nets are obstacles.
    pub zones_block: bool,
    /// Whether a via may be put on a pad.
    pub via_in_pad: bool,
}

impl Default for AutorouteConfig {
    /// The values of `config/autoroute_native.cfg`.
    fn default() -> Self {
        AutorouteConfig {
            grid_nm: 250_000,
            order: NetOrder::Shortest,
            critical_nets: Vec::new(),
            diagonal: true,
            corner_penalty_nm: 200_000,
            via_penalty_nm: 3_000_000,
            wrong_way_percent: 50,
            ripup_penalty_nm: 20_000_000,
            history_penalty_nm: 1_000_000,
            max_expansions_per_search: 400_000,
            max_expansions_total: 20_000_000,
            max_ripup_iterations: 40,
            max_rips_per_connection: 4,
            max_nodes: 8_000_000,
            max_connections: 20_000,
            terminal_candidates: 6,
            terminal_radius_cells: 4,
            zones_block: true,
            via_in_pad: false,
        }
    }
}

fn fail(line: usize, message: &str) -> AutorouteError {
    AutorouteError::new(SETTINGS_INVALID, format!("line {line}: {message}"))
}

fn int_in(value: &str, lo: i64, hi: i64) -> Option<i64> {
    value.parse::<i64>().ok().filter(|n| (lo..=hi).contains(n))
}

fn flag(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

impl AutorouteConfig {
    /// Reads the settings from the text of a file.
    pub fn parse(text: &str) -> Result<AutorouteConfig, AutorouteError> {
        if text.len() > MAX_CONFIG_BYTES {
            return Err(AutorouteError::new(
                SETTINGS_INVALID,
                format!("the file is larger than {MAX_CONFIG_BYTES} bytes"),
            ));
        }
        let mut c = AutorouteConfig::default();
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
                "grid_nm" => c.grid_nm = int_in(value, 1_000, 5_000_000).ok_or_else(bad)?,
                "order" => c.order = NetOrder::from_word(value).ok_or_else(bad)?,
                "critical_nets" => {
                    c.critical_nets = value
                        .split(',')
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(str::to_owned)
                        .collect();
                }
                "diagonal" => c.diagonal = flag(value).ok_or_else(bad)?,
                "corner_penalty_nm" => {
                    c.corner_penalty_nm = int_in(value, 0, 100_000_000).ok_or_else(bad)?;
                }
                "via_penalty_nm" => {
                    c.via_penalty_nm = int_in(value, 0, 1_000_000_000).ok_or_else(bad)?;
                }
                "wrong_way_percent" => {
                    c.wrong_way_percent = int_in(value, 0, 1000).ok_or_else(bad)?;
                }
                "ripup_penalty_nm" => {
                    c.ripup_penalty_nm = int_in(value, 0, 1_000_000_000).ok_or_else(bad)?;
                }
                "history_penalty_nm" => {
                    c.history_penalty_nm = int_in(value, 0, 100_000_000).ok_or_else(bad)?;
                }
                "max_expansions_per_search" => {
                    c.max_expansions_per_search =
                        int_in(value, 1, 1_000_000_000).ok_or_else(bad)? as u64;
                }
                "max_expansions_total" => {
                    c.max_expansions_total =
                        int_in(value, 1, 100_000_000_000).ok_or_else(bad)? as u64;
                }
                "max_ripup_iterations" => {
                    c.max_ripup_iterations = int_in(value, 0, 1_000_000).ok_or_else(bad)? as u32;
                }
                "max_rips_per_connection" => {
                    c.max_rips_per_connection = int_in(value, 0, 1000).ok_or_else(bad)? as u32;
                }
                "max_nodes" => {
                    c.max_nodes = int_in(value, 1, 1_000_000_000).ok_or_else(bad)? as u64;
                }
                "max_connections" => {
                    c.max_connections = int_in(value, 1, 10_000_000).ok_or_else(bad)? as usize;
                }
                "terminal_candidates" => {
                    c.terminal_candidates = int_in(value, 1, 64).ok_or_else(bad)? as usize;
                }
                "terminal_radius_cells" => {
                    c.terminal_radius_cells = int_in(value, 0, 64).ok_or_else(bad)?;
                }
                "zones_block" => c.zones_block = flag(value).ok_or_else(bad)?,
                "via_in_pad" => c.via_in_pad = flag(value).ok_or_else(bad)?,
                _ => return Err(fail(line, &format!("unknown key {key}"))),
            }
        }
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_file_parses_to_the_defaults() {
        let shipped = include_str!("../../../config/autoroute_native.cfg");
        assert_eq!(
            AutorouteConfig::parse(shipped).unwrap(),
            AutorouteConfig::default()
        );
    }

    #[test]
    fn a_line_changes_one_value_and_the_rest_keep_the_default() {
        let c = AutorouteConfig::parse(
            "# note\ngrid_nm = 500000\n\norder = critical\ncritical_nets = A, B,,C\n",
        )
        .unwrap();
        assert_eq!(c.grid_nm, 500_000);
        assert_eq!(c.order, NetOrder::Critical);
        assert_eq!(c.critical_nets, ["A", "B", "C"]);
        assert_eq!(c.via_penalty_nm, AutorouteConfig::default().via_penalty_nm);
        assert_eq!(c.order.word(), "critical");
    }

    #[test]
    fn unknown_keys_repeated_keys_and_values_out_of_range_are_refused() {
        for text in [
            "grid = 5\n",
            "grid_nm = 250000\ngrid_nm = 300000\n",
            "grid_nm = 999\n",
            "grid_nm = 5000001\n",
            "grid_nm = wide\n",
            "order = random\n",
            "diagonal = yes\n",
            "wrong_way_percent = 1001\n",
            "max_expansions_per_search = 0\n",
            "max_ripup_iterations = -1\n",
            "terminal_candidates = 65\n",
            "terminal_radius_cells = 65\n",
            "via_in_pad = 1\n",
            "just words\n",
        ] {
            let e = AutorouteConfig::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.starts_with("line "), "{text}: {}", e.detail);
        }
        assert!(AutorouteConfig::parse(&"#".repeat(MAX_CONFIG_BYTES + 1)).is_err());
    }
}
