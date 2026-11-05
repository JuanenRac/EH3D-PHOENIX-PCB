// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the simulator. Every limit, tolerance and default is read from a plain
//! `key = value` text, `config/sim.cfg`, and not buried in the code. A missing line keeps the
//! default; an unknown key or a value out of range is refused with its line number.

use crate::codes::{SimError, SETTINGS_INVALID};

/// Boltzmann constant in J/K.
const BOLTZMANN: f64 = 1.380_649e-23;
/// Elementary charge in C.
const CHARGE: f64 = 1.602_176_634e-19;

/// The limits, tolerances and defaults of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct SimSettings {
    /// The largest netlist file that is read, in bytes.
    pub max_file_bytes: u64,
    /// The most lines of a netlist.
    pub max_lines: usize,
    /// The most elements after the subcircuits are expanded.
    pub max_elements: usize,
    /// The most unknowns (node voltages and branch currents) of the equations.
    pub max_unknowns: usize,
    /// The deepest nesting of subcircuit instances.
    pub max_subckt_depth: usize,
    /// The deepest nesting of an expression in braces.
    pub max_expr_depth: usize,
    /// The most frequencies of an AC sweep.
    pub max_ac_points: usize,
    /// The most time steps of a transient analysis.
    pub max_tran_steps: usize,
    /// The most values (points times unknowns) of one result.
    pub max_result_cells: usize,
    /// The temperature in degrees Celsius; it sets the thermal voltage of the diodes.
    pub temperature_c: f64,
    /// The relative tolerance of the Newton-Raphson iteration and of the adaptive step.
    pub reltol: f64,
    /// The absolute tolerance of a voltage, in volts.
    pub vntol: f64,
    /// The absolute tolerance of a current, in amperes.
    pub abstol: f64,
    /// The conductance across every diode (and across every node when a circuit is started), in
    /// siemens.
    pub gmin: f64,
    /// A pivot smaller than this fraction of the largest value of its column is zero.
    pub pivot_tolerance: f64,
    /// The most iterations of one Newton-Raphson solution.
    pub newton_max_iter: usize,
    /// The steps of the source stepping that is tried when the plain iteration fails.
    pub source_steps: usize,
    /// The conductance that holds an initial condition of a capacitor, in siemens.
    pub ic_conductance: f64,
    /// The smallest adaptive step as a fraction of the step asked for.
    pub min_step_fraction: f64,
    /// The first adaptive step as a fraction of the step asked for.
    pub initial_step_fraction: f64,
    /// The safety factor of the adaptive step.
    pub step_safety: f64,
    /// The most the adaptive step grows at once.
    pub step_max_growth: f64,
    /// The most the adaptive step shrinks at once.
    pub step_min_shrink: f64,
    /// The group delay is taken from the phase at the frequency minus and plus this fraction.
    pub delay_step_fraction: f64,
    /// A node with no path to ground stops an operating-point analysis.
    pub floating_is_error: bool,
    /// The names that mean ground in a netlist, in lower case.
    pub netlist_ground: Vec<String>,
    /// The names of nets that mean ground in a schematic, in lower case.
    pub schematic_ground: Vec<String>,
    /// The saturation current of a diode model that does not give one, in amperes.
    pub diode_is: f64,
    /// The emission coefficient of a diode model that does not give one.
    pub diode_n: f64,
    /// How many times an output file is tried again when another program holds it.
    pub write_retries: u32,
    /// The wait between two tries, in milliseconds.
    pub write_retry_ms: u64,
    /// The frequencies of the coherence check against an RF filter.
    pub coherence_points: usize,
    /// The largest difference of magnitude accepted in the coherence check, in dB.
    pub coherence_db_tolerance: f64,
    /// The largest difference of phase accepted in the coherence check, in degrees.
    pub coherence_phase_tolerance_deg: f64,
    /// The largest difference of group delay accepted in the coherence check, as a fraction of
    /// one period.
    pub coherence_delay_tolerance: f64,
}

impl Default for SimSettings {
    fn default() -> Self {
        SimSettings {
            max_file_bytes: 4 * 1024 * 1024,
            max_lines: 200_000,
            max_elements: 20_000,
            max_unknowns: 1_500,
            max_subckt_depth: 16,
            max_expr_depth: 32,
            max_ac_points: 100_000,
            max_tran_steps: 1_000_000,
            max_result_cells: 10_000_000,
            temperature_c: 27.0,
            reltol: 1e-3,
            vntol: 1e-6,
            abstol: 1e-12,
            gmin: 1e-12,
            pivot_tolerance: 1e-12,
            newton_max_iter: 100,
            source_steps: 20,
            ic_conductance: 1e9,
            min_step_fraction: 1e-6,
            initial_step_fraction: 0.1,
            step_safety: 0.9,
            step_max_growth: 2.0,
            step_min_shrink: 0.25,
            delay_step_fraction: 1e-4,
            floating_is_error: true,
            netlist_ground: vec!["0".to_string(), "gnd".to_string()],
            schematic_ground: ["0", "gnd", "agnd", "dgnd", "pgnd"]
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            diode_is: 1e-14,
            diode_n: 1.0,
            write_retries: 5,
            write_retry_ms: 50,
            coherence_points: 41,
            coherence_db_tolerance: 1e-6,
            coherence_phase_tolerance_deg: 1e-5,
            coherence_delay_tolerance: 1e-4,
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

fn parse_names(value: &str) -> Option<Vec<String>> {
    let names: Vec<String> = value
        .split(',')
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| !part.is_empty())
        .collect();
    if names.is_empty() {
        None
    } else {
        Some(names)
    }
}

impl SimSettings {
    /// Parse a settings text. Blank lines and lines starting with `#` are ignored.
    pub fn parse(text: &str) -> Result<SimSettings, SimError> {
        let mut s = SimSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                SimError::at(SETTINGS_INVALID, line, "not key = value".to_string())
            })?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || {
                SimError::at(
                    SETTINGS_INVALID,
                    line,
                    format!("the value of {key} is not valid"),
                )
            };
            match key {
                "max_file_bytes" => s.max_file_bytes = value.parse().map_err(|_| bad())?,
                "max_lines" => s.max_lines = value.parse().map_err(|_| bad())?,
                "max_elements" => s.max_elements = value.parse().map_err(|_| bad())?,
                "max_unknowns" => s.max_unknowns = value.parse().map_err(|_| bad())?,
                "max_subckt_depth" => s.max_subckt_depth = value.parse().map_err(|_| bad())?,
                "max_expr_depth" => s.max_expr_depth = value.parse().map_err(|_| bad())?,
                "max_ac_points" => s.max_ac_points = value.parse().map_err(|_| bad())?,
                "max_tran_steps" => s.max_tran_steps = value.parse().map_err(|_| bad())?,
                "max_result_cells" => s.max_result_cells = value.parse().map_err(|_| bad())?,
                "temperature_c" => s.temperature_c = value.parse().map_err(|_| bad())?,
                "reltol" => s.reltol = value.parse().map_err(|_| bad())?,
                "vntol" => s.vntol = value.parse().map_err(|_| bad())?,
                "abstol" => s.abstol = value.parse().map_err(|_| bad())?,
                "gmin" => s.gmin = value.parse().map_err(|_| bad())?,
                "pivot_tolerance" => s.pivot_tolerance = value.parse().map_err(|_| bad())?,
                "newton_max_iter" => s.newton_max_iter = value.parse().map_err(|_| bad())?,
                "source_steps" => s.source_steps = value.parse().map_err(|_| bad())?,
                "ic_conductance" => s.ic_conductance = value.parse().map_err(|_| bad())?,
                "min_step_fraction" => s.min_step_fraction = value.parse().map_err(|_| bad())?,
                "initial_step_fraction" => {
                    s.initial_step_fraction = value.parse().map_err(|_| bad())?
                }
                "step_safety" => s.step_safety = value.parse().map_err(|_| bad())?,
                "step_max_growth" => s.step_max_growth = value.parse().map_err(|_| bad())?,
                "step_min_shrink" => s.step_min_shrink = value.parse().map_err(|_| bad())?,
                "delay_step_fraction" => {
                    s.delay_step_fraction = value.parse().map_err(|_| bad())?
                }
                "floating_is_error" => s.floating_is_error = parse_bool(value).ok_or_else(bad)?,
                "netlist_ground" => s.netlist_ground = parse_names(value).ok_or_else(bad)?,
                "schematic_ground" => s.schematic_ground = parse_names(value).ok_or_else(bad)?,
                "diode_is" => s.diode_is = value.parse().map_err(|_| bad())?,
                "diode_n" => s.diode_n = value.parse().map_err(|_| bad())?,
                "write_retries" => s.write_retries = value.parse().map_err(|_| bad())?,
                "write_retry_ms" => s.write_retry_ms = value.parse().map_err(|_| bad())?,
                "coherence_points" => s.coherence_points = value.parse().map_err(|_| bad())?,
                "coherence_db_tolerance" => {
                    s.coherence_db_tolerance = value.parse().map_err(|_| bad())?
                }
                "coherence_phase_tolerance_deg" => {
                    s.coherence_phase_tolerance_deg = value.parse().map_err(|_| bad())?
                }
                "coherence_delay_tolerance" => {
                    s.coherence_delay_tolerance = value.parse().map_err(|_| bad())?
                }
                _ => {
                    return Err(SimError::at(
                        SETTINGS_INVALID,
                        line,
                        format!("unknown key {key}"),
                    ))
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges that the algorithms rely on.
    pub fn validate(&self) -> Result<(), SimError> {
        let fail = |what: &str| Err(SimError::new(SETTINGS_INVALID, what.to_string()));
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if self.max_unknowns < 2 || self.max_elements < 1 || self.max_lines < 2 {
            return fail("max_unknowns, max_elements and max_lines are too small");
        }
        if self.max_subckt_depth < 1 || self.max_expr_depth < 2 {
            return fail("max_subckt_depth must be at least 1 and max_expr_depth at least 2");
        }
        if self.max_ac_points < 1 || self.max_tran_steps < 1 || self.max_result_cells < 1 {
            return fail("max_ac_points, max_tran_steps and max_result_cells must be at least 1");
        }
        if !self.temperature_c.is_finite() || self.temperature_c <= -273.15 {
            return fail("temperature_c must be above absolute zero");
        }
        if !positive(self.reltol) || !positive(self.vntol) || !positive(self.abstol) {
            return fail("reltol, vntol and abstol must be positive");
        }
        if !self.gmin.is_finite() || self.gmin < 0.0 {
            return fail("gmin cannot be negative");
        }
        if !positive(self.pivot_tolerance) || self.pivot_tolerance >= 1.0 {
            return fail("pivot_tolerance must be between 0 and 1");
        }
        if self.newton_max_iter < 2 {
            return fail("newton_max_iter must be at least 2");
        }
        if self.source_steps < 1 {
            return fail("source_steps must be at least 1");
        }
        if !positive(self.ic_conductance) {
            return fail("ic_conductance must be positive");
        }
        if !positive(self.min_step_fraction) || self.min_step_fraction > 1.0 {
            return fail("min_step_fraction must be between 0 and 1");
        }
        if !positive(self.initial_step_fraction) || self.initial_step_fraction > 1.0 {
            return fail("initial_step_fraction must be between 0 and 1");
        }
        if !positive(self.step_safety) || self.step_safety > 1.0 {
            return fail("step_safety must be between 0 and 1");
        }
        if !self.step_max_growth.is_finite() || self.step_max_growth < 1.0 {
            return fail("step_max_growth must be at least 1");
        }
        if !positive(self.step_min_shrink) || self.step_min_shrink > 1.0 {
            return fail("step_min_shrink must be between 0 and 1");
        }
        if !positive(self.delay_step_fraction) || self.delay_step_fraction >= 0.5 {
            return fail("delay_step_fraction must be between 0 and 0.5");
        }
        if !positive(self.diode_is) || !positive(self.diode_n) {
            return fail("diode_is and diode_n must be positive");
        }
        if self.coherence_points < 2 {
            return fail("coherence_points must be at least 2");
        }
        if !positive(self.coherence_db_tolerance)
            || !positive(self.coherence_phase_tolerance_deg)
            || !positive(self.coherence_delay_tolerance)
        {
            return fail("the coherence tolerances must be positive");
        }
        Ok(())
    }

    /// The thermal voltage `kT/q` at the temperature of the settings, in volts.
    pub fn thermal_voltage(&self) -> f64 {
        BOLTZMANN * (self.temperature_c + 273.15) / CHARGE
    }

    /// True when a netlist node name means ground (the name is compared in lower case).
    pub fn is_netlist_ground(&self, name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        self.netlist_ground.iter().any(|g| *g == lower)
    }

    /// True when the name of a schematic net means ground (compared in lower case).
    pub fn is_schematic_ground(&self, name: &str) -> bool {
        let lower = name.trim().to_ascii_lowercase();
        self.schematic_ground.iter().any(|g| *g == lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        assert_eq!(
            SimSettings::parse("# nothing\n").unwrap(),
            SimSettings::default()
        );
        let s = SimSettings::parse(
            "reltol = 1e-4\nfloating_is_error = false\nnetlist_ground = 0, GND, Earth\nmax_unknowns = 50\n",
        )
        .unwrap();
        assert_eq!(s.reltol, 1e-4);
        assert!(!s.floating_is_error);
        assert!(s.is_netlist_ground("EARTH"));
        assert!(!s.is_netlist_ground("agnd"));
        assert_eq!(s.max_unknowns, 50);
        assert_eq!(s.vntol, SimSettings::default().vntol);
    }

    #[test]
    fn bad_lines_are_refused_with_their_number() {
        for (text, needle) in [
            ("reltol = high", "reltol"),
            ("\nreltol = 0", "reltol"),
            ("colour = red", "unknown key"),
            ("oops", "key = value"),
            ("floating_is_error = 1", "floating_is_error"),
            ("newton_max_iter = 1", "newton_max_iter"),
            ("netlist_ground = ,", "netlist_ground"),
            ("pivot_tolerance = 2", "pivot_tolerance"),
        ] {
            let e = SimSettings::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        let e = SimSettings::parse("\n\nreltol = x").unwrap_err();
        assert_eq!(e.line, Some(3));
    }

    #[test]
    fn the_thermal_voltage_at_room_temperature_is_about_26_millivolts() {
        let vt = SimSettings::default().thermal_voltage();
        assert!((vt - 0.025_865).abs() < 1e-5, "{vt}");
    }

    #[test]
    fn the_shipped_settings_file_is_the_defaults() {
        let text = include_str!("../../../config/sim.cfg");
        assert_eq!(SimSettings::parse(text).unwrap(), SimSettings::default());
    }
}
