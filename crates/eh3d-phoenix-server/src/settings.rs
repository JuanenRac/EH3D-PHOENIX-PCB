// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the server mode, read from the plain `key = value` text `config/server.cfg`.
//! The keys of the link (`server_address`, `server_token`, `server_machine`, `server_timeout_s`,
//! `server_retries`, `server_mdns` and `server_max_answer_mib`) are the ones of the client that
//! every program of the suite uses, with the same ranges; the rest are the limits of this mode.
//! A missing line keeps the default; an unknown key or a value out of range is refused with its
//! line number. Nothing is sent unless `allow_network` is true.

use eh3d_serverclient as sc;

use crate::codes::{ServerError, SETTINGS_INVALID};

/// The settings of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSettings {
    /// The link: address, token, machine, time limit, retries and discovery.
    pub link: sc::Settings,
    /// Whether the command may send anything over the network. Off in the settings that come
    /// with the program.
    pub allow_network: bool,
    /// Whether the job is started after it is made (it is made `queued` and starts only when a
    /// person or a program starts it).
    pub start_job: bool,
    /// The pause between two questions about the job, in milliseconds.
    pub poll_ms: u64,
    /// How many times the job is asked about before it is given up.
    pub max_polls: u32,
    /// The most items (tracks, clearances, holes, parts or measurements) a request may hold.
    pub max_items: usize,
    /// The most gaps between copper that a rule check request lists, the smallest first; 0 sends
    /// none.
    pub clearance_max: usize,
    /// A gap is listed only when it is smaller than this, in nanometres.
    pub clearance_below_nm: i64,
    /// A board with more tracks than this sends no gaps (looking at every pair would take too
    /// long); 0 sends none.
    pub clearance_max_tracks: usize,
    /// The most pieces of two tracks that are compared to make the gaps.
    pub clearance_pair_budget: u64,
    /// The narrowest track the maker takes, in nanometres, put over the rule of the station;
    /// 0 leaves the rule of the station.
    pub rule_min_track_nm: i64,
    /// The smallest gap between copper, in nanometres; 0 leaves the rule of the station.
    pub rule_min_gap_nm: i64,
    /// The smallest hole, in nanometres; 0 leaves the rule of the station.
    pub rule_min_drill_nm: i64,
    /// The biggest hole that is drilled and not routed, in nanometres; 0 leaves the rule of the
    /// station.
    pub rule_max_drill_nm: i64,
    /// The smallest distance from the wall of a hole to the edge, in nanometres; 0 leaves the
    /// rule of the station.
    pub rule_min_edge_nm: i64,
    /// The most problems that a report lists.
    pub max_listed: usize,
}

impl Default for ServerSettings {
    fn default() -> Self {
        ServerSettings {
            link: sc::Settings::default(),
            allow_network: false,
            start_job: true,
            poll_ms: 300,
            max_polls: 400,
            max_items: 50_000,
            clearance_max: 200,
            clearance_below_nm: 1_000_000,
            clearance_max_tracks: 3_000,
            clearance_pair_budget: 4_000_000,
            rule_min_track_nm: 0,
            rule_min_gap_nm: 0,
            rule_min_drill_nm: 0,
            rule_max_drill_nm: 0,
            rule_min_edge_nm: 0,
            max_listed: 100,
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

/// A value of the rule that is 0 (not sent) or inside the range the station accepts.
fn rule_value(value: &str, low: i64, high: i64) -> Option<i64> {
    let v: i64 = value.parse().ok()?;
    (v == 0 || (low..=high).contains(&v)).then_some(v)
}

impl ServerSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<ServerSettings, ServerError> {
        let mut s = ServerSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                ServerError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                ServerError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            if sc::settings::is_server_key(key) {
                s.link
                    .set(key, value)
                    .map_err(|e| ServerError::new(SETTINGS_INVALID, format!("line {line}: {e}")))?;
                continue;
            }
            match key {
                "allow_network" => s.allow_network = parse_bool(value).ok_or_else(bad)?,
                "start_job" => s.start_job = parse_bool(value).ok_or_else(bad)?,
                "poll_ms" => s.poll_ms = value.parse().map_err(|_| bad())?,
                "max_polls" => s.max_polls = value.parse().map_err(|_| bad())?,
                "max_items" => s.max_items = value.parse().map_err(|_| bad())?,
                "clearance_max" => s.clearance_max = value.parse().map_err(|_| bad())?,
                "clearance_below_nm" => s.clearance_below_nm = value.parse().map_err(|_| bad())?,
                "clearance_max_tracks" => {
                    s.clearance_max_tracks = value.parse().map_err(|_| bad())?
                }
                "clearance_pair_budget" => {
                    s.clearance_pair_budget = value.parse().map_err(|_| bad())?
                }
                "rule_min_track_nm" => {
                    s.rule_min_track_nm = rule_value(value, 10_000, 5_000_000).ok_or_else(bad)?
                }
                "rule_min_gap_nm" => {
                    s.rule_min_gap_nm = rule_value(value, 10_000, 5_000_000).ok_or_else(bad)?
                }
                "rule_min_drill_nm" => {
                    s.rule_min_drill_nm = rule_value(value, 50_000, 10_000_000).ok_or_else(bad)?
                }
                "rule_max_drill_nm" => {
                    s.rule_max_drill_nm = rule_value(value, 100_000, 50_000_000).ok_or_else(bad)?
                }
                "rule_min_edge_nm" => {
                    s.rule_min_edge_nm = rule_value(value, 1, 10_000_000).ok_or_else(bad)?
                }
                "max_listed" => s.max_listed = value.parse().map_err(|_| bad())?,
                _ => {
                    return Err(ServerError::new(
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
    pub fn validate(&self) -> Result<(), ServerError> {
        let fail = |what: &str| Err(ServerError::new(SETTINGS_INVALID, what.to_string()));
        if !(10..=60_000).contains(&self.poll_ms) {
            return fail("poll_ms must be between 10 and 60000");
        }
        if !(1..=100_000).contains(&self.max_polls) {
            return fail("max_polls must be between 1 and 100000");
        }
        if !(1..=200_000).contains(&self.max_items) {
            return fail("max_items must be between 1 and 200000");
        }
        if self.clearance_max > 10_000 {
            return fail("clearance_max must be between 0 and 10000");
        }
        if !(0..(1 << 30)).contains(&self.clearance_below_nm) {
            return fail("clearance_below_nm must be between 0 and 2^30 nanometres");
        }
        if self.clearance_max_tracks > 100_000 {
            return fail("clearance_max_tracks must be between 0 and 100000");
        }
        if !(1..=1_000_000_000).contains(&self.clearance_pair_budget) {
            return fail("clearance_pair_budget must be between 1 and 1000000000");
        }
        if !(1..=10_000).contains(&self.max_listed) {
            return fail("max_listed must be between 1 and 10000");
        }
        Ok(())
    }

    /// The rules that are put over the station, in millimetres text, as `(key, nanometres)`
    /// pairs for the ones that are set.
    pub fn rule_overrides(&self) -> Vec<(&'static str, i64)> {
        [
            ("min_track_mm", self.rule_min_track_nm),
            ("min_gap_mm", self.rule_min_gap_nm),
            ("min_drill_mm", self.rule_min_drill_nm),
            ("max_drill_mm", self.rule_max_drill_nm),
            ("min_edge_mm", self.rule_min_edge_nm),
        ]
        .into_iter()
        .filter(|(_, nm)| *nm > 0)
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid_and_a_blank_text_keeps_them() {
        assert!(ServerSettings::default().validate().is_ok());
        assert_eq!(
            ServerSettings::parse("# nothing\n\n").unwrap(),
            ServerSettings::default()
        );
        // nothing is sent and no address is set until the owner says so
        let d = ServerSettings::default();
        assert!(!d.allow_network);
        assert!(!d.link.is_configured());
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_defaults() {
        let text = include_str!("../../../config/server.cfg");
        assert_eq!(
            ServerSettings::parse(text).unwrap(),
            ServerSettings::default()
        );
    }

    #[test]
    fn the_keys_of_the_link_are_read_with_the_ranges_of_the_client() {
        let s = ServerSettings::parse(
            "server_address = 192.168.0.180:18200\nserver_token = abc\nserver_machine = pcb-1\nserver_timeout_s = 30\nserver_retries = 4\nserver_mdns = on\nserver_max_answer_mib = 16\n",
        )
        .unwrap();
        assert_eq!(s.link.server_address, "192.168.0.180:18200");
        assert_eq!(s.link.server_token.expose(), "abc");
        assert_eq!(s.link.server_machine, "pcb-1");
        assert_eq!(s.link.server_timeout_s, 30);
        assert_eq!(s.link.server_retries, 4);
        assert!(s.link.server_mdns);
        assert_eq!(s.link.server_max_answer_mib, 16);
        for bad in [
            "server_timeout_s = 0",
            "server_timeout_s = 121",
            "server_retries = 6",
            "server_mdns = maybe",
            "server_address = a b",
            "server_max_answer_mib = 257",
        ] {
            let e = ServerSettings::parse(&format!("{bad}\n")).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{bad}");
            assert!(e.detail.contains("line 1"), "{}", e.detail);
        }
    }

    #[test]
    fn the_token_is_not_shown_by_the_debug_output() {
        let s = ServerSettings::parse("server_token = very-secret-token\n").unwrap();
        assert!(!format!("{s:?}").contains("very-secret-token"));
    }

    #[test]
    fn lines_change_the_values_of_the_mode() {
        let s = ServerSettings::parse(
            "allow_network = true\nstart_job = false\npoll_ms = 50\nmax_polls = 7\nmax_items = 100\nclearance_max = 0\nclearance_below_nm = 500000\nclearance_max_tracks = 10\nclearance_pair_budget = 99\nmax_listed = 5\n",
        )
        .unwrap();
        assert!(s.allow_network && !s.start_job);
        assert_eq!((s.poll_ms, s.max_polls, s.max_items), (50, 7, 100));
        assert_eq!(s.clearance_max, 0);
        assert_eq!(s.clearance_below_nm, 500_000);
        assert_eq!((s.clearance_max_tracks, s.clearance_pair_budget), (10, 99));
        assert_eq!(s.max_listed, 5);
    }

    #[test]
    fn the_rules_put_over_the_station_are_off_at_zero_and_have_its_ranges() {
        assert!(ServerSettings::default().rule_overrides().is_empty());
        let s = ServerSettings::parse(
            "rule_min_track_nm = 150000\nrule_min_gap_nm = 200000\nrule_min_drill_nm = 300000\nrule_max_drill_nm = 6500000\nrule_min_edge_nm = 300000\n",
        )
        .unwrap();
        assert_eq!(
            s.rule_overrides(),
            vec![
                ("min_track_mm", 150_000),
                ("min_gap_mm", 200_000),
                ("min_drill_mm", 300_000),
                ("max_drill_mm", 6_500_000),
                ("min_edge_mm", 300_000),
            ]
        );
        for bad in [
            "rule_min_track_nm = 9999",
            "rule_min_track_nm = 5000001",
            "rule_min_gap_nm = -1",
            "rule_min_drill_nm = 49999",
            "rule_max_drill_nm = 50000001",
            "rule_min_edge_nm = 10000001",
            "rule_min_edge_nm = x",
        ] {
            assert!(ServerSettings::parse(&format!("{bad}\n")).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        let unknown = ServerSettings::parse("\na = 1\n").unwrap_err();
        assert_eq!(unknown.code, SETTINGS_INVALID);
        assert!(unknown.detail.contains("line 2"), "{}", unknown.detail);
        for text in [
            "allow_network = yes\n",
            "start_job = 1\n",
            "poll_ms = 9\n",
            "poll_ms = 60001\n",
            "max_polls = 0\n",
            "max_items = 0\n",
            "max_items = 200001\n",
            "clearance_max = 10001\n",
            "clearance_below_nm = -1\n",
            "clearance_max_tracks = 100001\n",
            "clearance_pair_budget = 0\n",
            "max_listed = 0\n",
            "no equals sign\n",
        ] {
            assert!(ServerSettings::parse(text).is_err(), "{text}");
        }
    }

    /// A generator with a fixed seed, so every run makes the same texts.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }

        fn pick<'a>(&mut self, words: &[&'a str]) -> &'a str {
            words[(self.next() % words.len() as u64) as usize]
        }
    }

    #[test]
    fn a_text_of_valid_values_is_read_back_as_the_values_that_were_written() {
        let mut rng = Lcg(0x5e77_0001);
        for _ in 0..80 {
            let poll = 10 + rng.next() % 59_991;
            let polls = 1 + rng.next() % 100_000;
            let items = 1 + rng.next() % 200_000;
            let max = rng.next() % 10_001;
            let below = rng.next() % (1 << 30);
            let timeout = 1 + rng.next() % 120;
            let retries = rng.next() % 6;
            let allow = rng.next() & 1 == 0;
            let text = format!(
                "allow_network = {allow}\npoll_ms = {poll}\nmax_polls = {polls}\nmax_items = {items}\nclearance_max = {max}\nclearance_below_nm = {below}\nserver_timeout_s = {timeout}\nserver_retries = {retries}\n"
            );
            let s = ServerSettings::parse(&text).unwrap();
            assert_eq!(s.allow_network, allow);
            assert_eq!((s.poll_ms, u64::from(s.max_polls)), (poll, polls));
            assert_eq!((s.max_items as u64, s.clearance_max as u64), (items, max));
            assert_eq!(s.clearance_below_nm, below as i64);
            assert_eq!(s.link.server_timeout_s, timeout);
            assert_eq!(u64::from(s.link.server_retries), retries);
        }
    }

    #[test]
    fn random_lines_never_panic_and_a_refusal_always_names_its_line() {
        let keys = [
            "allow_network",
            "poll_ms",
            "max_items",
            "server_address",
            "server_retries",
            "rule_min_track_nm",
            "clearance_max",
            "nope",
            "",
        ];
        let values = ["true", "0", "-1", "99999999999", "x", "", "1 2", "=", "#"];
        let mut rng = Lcg(0x5e77_0002);
        let (mut accepted, mut refused) = (0, 0);
        for _ in 0..300 {
            let mut text = String::new();
            for _ in 0..(1 + rng.next() % 4) {
                text.push_str(&format!("{} = {}\n", rng.pick(&keys), rng.pick(&values)));
            }
            match ServerSettings::parse(&text) {
                Ok(_) => accepted += 1,
                Err(e) => {
                    refused += 1;
                    assert_eq!(e.code, SETTINGS_INVALID);
                    // a refusal of a line says which; a refusal of a range may not
                    assert!(
                        e.detail.contains("line ") || e.detail.contains("must be"),
                        "{}",
                        e.detail
                    );
                }
            }
        }
        // both outcomes happen, so neither was met by chance
        assert!(accepted > 10 && refused > 10, "{accepted} {refused}");
    }
}
