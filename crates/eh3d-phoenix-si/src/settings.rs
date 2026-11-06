// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the signal integrity estimates. Every threshold, driver, load and limit is read
//! from a plain `key = value` text, `config/si.cfg`, and not buried in the code. A missing line
//! keeps the default; an unknown key or a value out of range is refused with its line number.
//! The values of a single net are lines `rise_time_ps.<net>`, `rs_ohm.<net>`, `rl_ohm.<net>` and
//! `cl_pf.<net>`.

use std::collections::BTreeMap;

use crate::codes::{SiError, SETTINGS_INVALID};

/// The largest length a setting may take, in nanometres (the coordinate range of the model).
const LENGTH_LIMIT_NM: i64 = 1 << 30;

/// The thresholds, drivers, loads and limits of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct SiSettings {
    /// The 10 to 90 percent rise time of the edges, in picoseconds.
    pub rise_time_ps: f64,
    /// The rise time of single nets, by name.
    pub net_rise_ps: BTreeMap<String, f64>,
    /// The output resistance of the driver, in ohm.
    pub rs_ohm: f64,
    /// The output resistance of single nets, by name.
    pub net_rs_ohm: BTreeMap<String, f64>,
    /// The resistance of the load to ground, in ohm; zero is no resistive load (an open end).
    pub rl_ohm: f64,
    /// The resistive load of single nets, by name.
    pub net_rl_ohm: BTreeMap<String, f64>,
    /// The capacitance of the load, in picofarad.
    pub cl_pf: f64,
    /// The capacitance of the load of single nets, by name.
    pub net_cl_pf: BTreeMap<String, f64>,
    /// The widest gap between the edges of two tracks at which they still count as coupled, in
    /// nanometres.
    pub coupling_range_nm: i64,
    /// The shortest parallel run that is looked at, in nanometres.
    pub min_parallel_nm: i64,
    /// The largest angle between two pieces that still count as parallel, in degrees.
    pub parallel_tolerance_deg: f64,
    /// The near-end crosstalk from which a warning is given, in percent of the step.
    pub next_warn_pct: f64,
    /// The near-end crosstalk from which an error is given, in percent of the step.
    pub next_error_pct: f64,
    /// The far-end crosstalk (absolute value) from which a warning is given, in percent.
    pub fext_warn_pct: f64,
    /// The far-end crosstalk (absolute value) from which an error is given, in percent.
    pub fext_error_pct: f64,
    /// The largest ratio of mutual to self inductance for which the weak-coupling formulas are
    /// trusted.
    pub weak_coupling_limit: f64,
    /// The overshoot from which a warning is given, in percent of the final level.
    pub overshoot_warn_pct: f64,
    /// The overshoot from which an error is given, in percent of the final level.
    pub overshoot_error_pct: f64,
    /// The ringback from which a warning is given, in percent of the final level.
    pub ringback_warn_pct: f64,
    /// The band around the final level inside which the line counts as settled, in percent.
    pub settle_band_pct: f64,
    /// The steps the rise time is cut into by the simulation of reflections.
    pub steps_per_rise: u32,
    /// The delays of the line that the simulation of reflections covers.
    pub delays_simulated: u32,
    /// The most steps of one simulation.
    pub max_steps: usize,
    /// The bounces listed in the diagram of reflections.
    pub bounces: usize,
    /// The time constant of the RC termination in delays of the line.
    pub rc_tau_over_td: f64,
    /// The slowing of the rise time by the loss of the line from which a warning is given, in
    /// percent.
    pub rise_degradation_warn_pct: f64,
    /// The bit rate of the eye diagram, in megabit per second.
    pub bit_rate_mbps: f64,
    /// The order of the pseudo-random sequence of the eye diagram, 7 or 9.
    pub prbs: u32,
    /// The samples of one bit of the eye diagram.
    pub samples_per_ui: usize,
    /// The order of the low-pass filter that stands for the loss of the line, 1 or 2.
    pub eye_order: u32,
    /// The height of the eye below which a warning is given, as a fraction of the swing.
    pub eye_height_warn: f64,
    /// The width of the eye below which a warning is given, as a fraction of the bit.
    pub eye_width_warn: f64,
    /// The distance between the points at which a track is looked at over its reference plane,
    /// in nanometres.
    pub plane_sample_nm: i64,
    /// The longest via stub that is accepted, in nanometres.
    pub max_stub_nm: i64,
    /// The fraction of the rise time that the round trip of a stub may take.
    pub stub_tr_fraction: f64,
    /// The delay of a via barrel, in picoseconds per millimetre.
    pub via_delay_ps_per_mm: f64,
    /// The length of a via when the stack-up does not give it, in nanometres.
    pub default_via_length_nm: i64,
    /// The impedance of a line when there is no usable stack-up, in ohm.
    pub default_z0_ohm: f64,
    /// The effective permittivity of a line when there is no usable stack-up.
    pub default_eps_eff: f64,
    /// The height of a track over its reference plane when there is no usable stack-up, in
    /// nanometres.
    pub default_h_nm: i64,
    /// The relative permittivity of the dielectric when there is no usable stack-up.
    pub default_er: f64,
    /// The loss tangent of the dielectric when there is no usable stack-up.
    pub default_tan_d: f64,
    /// The most nets looked at in one run.
    pub max_nets: usize,
    /// The most pieces of track looked at in one run.
    pub max_items: usize,
}

impl Default for SiSettings {
    fn default() -> Self {
        SiSettings {
            rise_time_ps: 500.0,
            net_rise_ps: BTreeMap::new(),
            rs_ohm: 25.0,
            net_rs_ohm: BTreeMap::new(),
            rl_ohm: 0.0,
            net_rl_ohm: BTreeMap::new(),
            cl_pf: 5.0,
            net_cl_pf: BTreeMap::new(),
            coupling_range_nm: 1_500_000,
            min_parallel_nm: 1_000_000,
            parallel_tolerance_deg: 1.0,
            next_warn_pct: 5.0,
            next_error_pct: 10.0,
            fext_warn_pct: 5.0,
            fext_error_pct: 10.0,
            weak_coupling_limit: 0.3,
            overshoot_warn_pct: 10.0,
            overshoot_error_pct: 25.0,
            ringback_warn_pct: 10.0,
            settle_band_pct: 5.0,
            steps_per_rise: 20,
            delays_simulated: 40,
            max_steps: 400_000,
            bounces: 8,
            rc_tau_over_td: 4.0,
            rise_degradation_warn_pct: 20.0,
            bit_rate_mbps: 100.0,
            prbs: 7,
            samples_per_ui: 32,
            eye_order: 1,
            eye_height_warn: 0.6,
            eye_width_warn: 0.6,
            plane_sample_nm: 500_000,
            max_stub_nm: 1_500_000,
            stub_tr_fraction: 0.1,
            via_delay_ps_per_mm: 6.9,
            default_via_length_nm: 1_600_000,
            default_z0_ohm: 50.0,
            default_eps_eff: 3.0,
            default_h_nm: 200_000,
            default_er: 4.3,
            default_tan_d: 0.02,
            max_nets: 2000,
            max_items: 200_000,
        }
    }
}

/// Checks that `v` is a finite number inside `lo..=hi`.
fn within(v: f64, lo: f64, hi: f64) -> bool {
    v.is_finite() && (lo..=hi).contains(&v)
}

/// The single-net values that a key with a net name after a dot sets.
enum NetKey {
    Rise,
    Rs,
    Rl,
    Cl,
}

/// The kind of value and the net of a key such as `rs_ohm.CLK`.
fn net_key(key: &str) -> Option<(NetKey, &str)> {
    if let Some(name) = key.strip_prefix("rise_time_ps.") {
        Some((NetKey::Rise, name))
    } else if let Some(name) = key.strip_prefix("rs_ohm.") {
        Some((NetKey::Rs, name))
    } else if let Some(name) = key.strip_prefix("rl_ohm.") {
        Some((NetKey::Rl, name))
    } else {
        key.strip_prefix("cl_pf.").map(|name| (NetKey::Cl, name))
    }
}

impl SiSettings {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<SiSettings, SiError> {
        let mut s = SiSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                SiError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                SiError::new(
                    SETTINGS_INVALID,
                    format!("line {line}: the value of {key} is not valid"),
                )
            };
            let number = || value.parse::<f64>().map_err(|_| bad());
            let integer = || value.parse::<i64>().map_err(|_| bad());
            let count = || value.parse::<usize>().map_err(|_| bad());
            let small = || value.parse::<u32>().map_err(|_| bad());
            if let Some((kind, name)) = net_key(key) {
                let v = number()?;
                let in_range = match kind {
                    NetKey::Rise => within(v, 1.0, 1.0e6),
                    NetKey::Rs | NetKey::Cl => within(v, 0.0, 1.0e4),
                    NetKey::Rl => within(v, 0.0, 1.0e9),
                };
                if name.is_empty() || !in_range {
                    return Err(bad());
                }
                let map = match kind {
                    NetKey::Rise => &mut s.net_rise_ps,
                    NetKey::Rs => &mut s.net_rs_ohm,
                    NetKey::Rl => &mut s.net_rl_ohm,
                    NetKey::Cl => &mut s.net_cl_pf,
                };
                map.insert(name.to_owned(), v);
                continue;
            }
            match key {
                "rise_time_ps" => s.rise_time_ps = number()?,
                "rs_ohm" => s.rs_ohm = number()?,
                "rl_ohm" => s.rl_ohm = number()?,
                "cl_pf" => s.cl_pf = number()?,
                "coupling_range_nm" => s.coupling_range_nm = integer()?,
                "min_parallel_nm" => s.min_parallel_nm = integer()?,
                "parallel_tolerance_deg" => s.parallel_tolerance_deg = number()?,
                "next_warn_pct" => s.next_warn_pct = number()?,
                "next_error_pct" => s.next_error_pct = number()?,
                "fext_warn_pct" => s.fext_warn_pct = number()?,
                "fext_error_pct" => s.fext_error_pct = number()?,
                "weak_coupling_limit" => s.weak_coupling_limit = number()?,
                "overshoot_warn_pct" => s.overshoot_warn_pct = number()?,
                "overshoot_error_pct" => s.overshoot_error_pct = number()?,
                "ringback_warn_pct" => s.ringback_warn_pct = number()?,
                "settle_band_pct" => s.settle_band_pct = number()?,
                "steps_per_rise" => s.steps_per_rise = small()?,
                "delays_simulated" => s.delays_simulated = small()?,
                "max_steps" => s.max_steps = count()?,
                "bounces" => s.bounces = count()?,
                "rc_tau_over_td" => s.rc_tau_over_td = number()?,
                "rise_degradation_warn_pct" => s.rise_degradation_warn_pct = number()?,
                "bit_rate_mbps" => s.bit_rate_mbps = number()?,
                "prbs" => s.prbs = small()?,
                "samples_per_ui" => s.samples_per_ui = count()?,
                "eye_order" => s.eye_order = small()?,
                "eye_height_warn" => s.eye_height_warn = number()?,
                "eye_width_warn" => s.eye_width_warn = number()?,
                "plane_sample_nm" => s.plane_sample_nm = integer()?,
                "max_stub_nm" => s.max_stub_nm = integer()?,
                "stub_tr_fraction" => s.stub_tr_fraction = number()?,
                "via_delay_ps_per_mm" => s.via_delay_ps_per_mm = number()?,
                "default_via_length_nm" => s.default_via_length_nm = integer()?,
                "default_z0_ohm" => s.default_z0_ohm = number()?,
                "default_eps_eff" => s.default_eps_eff = number()?,
                "default_h_nm" => s.default_h_nm = integer()?,
                "default_er" => s.default_er = number()?,
                "default_tan_d" => s.default_tan_d = number()?,
                "max_nets" => s.max_nets = count()?,
                "max_items" => s.max_items = count()?,
                _ => {
                    return Err(SiError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: unknown key {key}"),
                    ))
                }
            }
            // the order of a warning and an error limit is checked once all the lines are read
            if let Err(what) = s.validate() {
                let deferred = matches!(
                    what.as_str(),
                    "next_error_pct" | "fext_error_pct" | "overshoot_error_pct"
                );
                if !deferred {
                    return Err(SiError::new(
                        SETTINGS_INVALID,
                        format!("line {line}: the value of {key} is out of range ({what})"),
                    ));
                }
            }
        }
        s.validate()
            .map_err(|what| SiError::new(SETTINGS_INVALID, what))?;
        Ok(s)
    }

    /// Checks every value; the error names the first one out of range.
    pub fn validate(&self) -> Result<(), String> {
        let checks = [
            ("rise_time_ps", within(self.rise_time_ps, 1.0, 1.0e6)),
            ("rs_ohm", within(self.rs_ohm, 0.0, 1.0e4)),
            ("rl_ohm", within(self.rl_ohm, 0.0, 1.0e9)),
            ("cl_pf", within(self.cl_pf, 0.0, 1.0e4)),
            (
                "coupling_range_nm",
                (1..=LENGTH_LIMIT_NM).contains(&self.coupling_range_nm),
            ),
            (
                "min_parallel_nm",
                (0..=LENGTH_LIMIT_NM).contains(&self.min_parallel_nm),
            ),
            (
                "parallel_tolerance_deg",
                within(self.parallel_tolerance_deg, 0.0, 45.0),
            ),
            ("next_warn_pct", within(self.next_warn_pct, 0.0, 1000.0)),
            (
                "next_error_pct",
                within(self.next_error_pct, self.next_warn_pct, 1000.0),
            ),
            ("fext_warn_pct", within(self.fext_warn_pct, 0.0, 1000.0)),
            (
                "fext_error_pct",
                within(self.fext_error_pct, self.fext_warn_pct, 1000.0),
            ),
            (
                "weak_coupling_limit",
                within(self.weak_coupling_limit, 0.01, 0.95),
            ),
            (
                "overshoot_warn_pct",
                within(self.overshoot_warn_pct, 0.0, 1000.0),
            ),
            (
                "overshoot_error_pct",
                within(self.overshoot_error_pct, self.overshoot_warn_pct, 1000.0),
            ),
            (
                "ringback_warn_pct",
                within(self.ringback_warn_pct, 0.0, 1000.0),
            ),
            ("settle_band_pct", within(self.settle_band_pct, 0.1, 50.0)),
            ("steps_per_rise", (2..=1000).contains(&self.steps_per_rise)),
            (
                "delays_simulated",
                (2..=10_000).contains(&self.delays_simulated),
            ),
            ("max_steps", (1000..=50_000_000).contains(&self.max_steps)),
            ("bounces", (1..=1000).contains(&self.bounces)),
            ("rc_tau_over_td", within(self.rc_tau_over_td, 0.1, 1000.0)),
            (
                "rise_degradation_warn_pct",
                within(self.rise_degradation_warn_pct, 0.0, 10_000.0),
            ),
            ("bit_rate_mbps", within(self.bit_rate_mbps, 0.001, 1.0e6)),
            ("prbs", self.prbs == 7 || self.prbs == 9),
            ("samples_per_ui", (8..=1024).contains(&self.samples_per_ui)),
            ("eye_order", self.eye_order == 1 || self.eye_order == 2),
            ("eye_height_warn", within(self.eye_height_warn, 0.0, 1.0)),
            ("eye_width_warn", within(self.eye_width_warn, 0.0, 1.0)),
            (
                "plane_sample_nm",
                (1000..=LENGTH_LIMIT_NM).contains(&self.plane_sample_nm),
            ),
            (
                "max_stub_nm",
                (0..=LENGTH_LIMIT_NM).contains(&self.max_stub_nm),
            ),
            (
                "stub_tr_fraction",
                within(self.stub_tr_fraction, 0.001, 10.0),
            ),
            (
                "via_delay_ps_per_mm",
                within(self.via_delay_ps_per_mm, 0.1, 100.0),
            ),
            (
                "default_via_length_nm",
                (1..=LENGTH_LIMIT_NM).contains(&self.default_via_length_nm),
            ),
            ("default_z0_ohm", within(self.default_z0_ohm, 1.0, 1000.0)),
            ("default_eps_eff", within(self.default_eps_eff, 1.0, 100.0)),
            (
                "default_h_nm",
                (1000..=LENGTH_LIMIT_NM).contains(&self.default_h_nm),
            ),
            ("default_er", within(self.default_er, 1.0, 100.0)),
            ("default_tan_d", within(self.default_tan_d, 0.0, 1.0)),
            ("max_nets", (1..=10_000_000).contains(&self.max_nets)),
            ("max_items", (1..=100_000_000).contains(&self.max_items)),
        ];
        match checks.iter().find(|(_, ok)| !ok) {
            Some((name, _)) => Err((*name).to_owned()),
            None => Ok(()),
        }
    }

    /// The rise time of a net, in picoseconds.
    pub fn rise_for(&self, net: &str) -> f64 {
        self.net_rise_ps
            .get(net)
            .copied()
            .unwrap_or(self.rise_time_ps)
    }

    /// The output resistance of the driver of a net, in ohm.
    pub fn rs_for(&self, net: &str) -> f64 {
        self.net_rs_ohm.get(net).copied().unwrap_or(self.rs_ohm)
    }

    /// The resistance of the load of a net to ground; `None` when there is none.
    pub fn rl_for(&self, net: &str) -> Option<f64> {
        let v = self.net_rl_ohm.get(net).copied().unwrap_or(self.rl_ohm);
        (v > 0.0).then_some(v)
    }

    /// The capacitance of the load of a net, in picofarad.
    pub fn cl_for(&self, net: &str) -> f64 {
        self.net_cl_pf.get(net).copied().unwrap_or(self.cl_pf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid() {
        assert!(SiSettings::default().validate().is_ok());
    }

    #[test]
    fn the_file_of_the_repository_holds_the_defaults() {
        let text = include_str!("../../../config/si.cfg");
        assert_eq!(SiSettings::parse(text).unwrap(), SiSettings::default());
    }

    #[test]
    fn a_text_changes_the_values_and_the_single_nets() {
        let s = SiSettings::parse(
            "# comment\nrise_time_ps = 300\nrs_ohm.CLK = 10\nrl_ohm = 50\ncl_pf.CLK = 2.5\nprbs = 9\n",
        )
        .unwrap();
        assert_eq!(s.rise_time_ps, 300.0);
        assert_eq!(s.rs_for("CLK"), 10.0);
        assert_eq!(s.rs_for("OTHER"), 25.0);
        assert_eq!(s.rl_for("CLK"), Some(50.0));
        assert_eq!(s.cl_for("CLK"), 2.5);
        assert_eq!(s.prbs, 9);
        assert_eq!(SiSettings::default().rl_for("X"), None);
        assert_eq!(s.rise_for("CLK"), 300.0);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        let unknown = SiSettings::parse("a = 1\n").unwrap_err();
        assert_eq!(unknown.code, SETTINGS_INVALID);
        assert!(unknown.detail.contains("line 1"));
        let range = SiSettings::parse("\nprbs = 8\n").unwrap_err();
        assert!(range.detail.contains("line 2"), "{}", range.detail);
        assert!(SiSettings::parse("rise_time_ps = x\n").is_err());
        assert!(SiSettings::parse("no equals\n").is_err());
        assert!(SiSettings::parse("rs_ohm. = 3\n").is_err());
        assert!(SiSettings::parse("next_warn_pct = 20\nnext_error_pct = 10\n").is_err());
    }
}
