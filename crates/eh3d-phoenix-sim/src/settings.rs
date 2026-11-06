// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the simulator. Every limit, tolerance and default is read from a plain
//! `key = value` text, `config/sim.cfg`, and not buried in the code. A missing line keeps the
//! default; an unknown key or a value out of range is refused with its line number.

use crate::codes::{SimError, SETTINGS_INVALID};

/// Boltzmann constant in J/K.
pub const BOLTZMANN: f64 = 1.380_649e-23;
/// Elementary charge in C.
pub const CHARGE: f64 = 1.602_176_634e-19;

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
    /// The steps of the gmin stepping that is tried when the plain iteration does not converge (zero turns it off).
    pub gmin_steps: usize,
    /// The conductance from every node to ground at the first step of the gmin stepping, in siemens.
    pub gmin_start: f64,
    /// The factor by which the gmin stepping lowers that conductance at each step.
    pub gmin_factor: f64,
    /// The most breakpoints (corners of pulses and piecewise-linear sources) of an adaptive transient analysis.
    pub max_breakpoints: usize,
    /// The current at the breakdown voltage of a diode model that does not give one, in amperes.
    pub diode_ibv: f64,
    /// The saturation current of a transistor model that does not give one, in amperes.
    pub bjt_is: f64,
    /// The forward current gain of a transistor model that does not give one.
    pub bjt_bf: f64,
    /// The reverse current gain of a transistor model that does not give one.
    pub bjt_br: f64,
    /// The threshold voltage of a MOSFET model that does not give one, in volts.
    pub mos_vto: f64,
    /// The transconductance parameter of a MOSFET model that does not give one, in A/V^2.
    pub mos_kp: f64,
    /// The channel-length modulation of a MOSFET model that does not give one, in 1/V.
    pub mos_lambda: f64,
    /// The channel width when neither the model nor the instance give one, in metres.
    pub mos_w: f64,
    /// The channel length when neither the model nor the instance give one, in metres.
    pub mos_l: f64,
    /// The open-loop DC gain of an op-amp model that does not give one.
    pub opamp_gain: f64,
    /// The gain-bandwidth product of an op-amp model that does not give one, in hertz.
    pub opamp_gbw: f64,
    /// The output resistance of an op-amp or comparator model that does not give one, in ohms.
    pub opamp_rout: f64,
    /// The resistance of the node that holds the pole of the op-amp macromodel, in ohms.
    pub opamp_pole_ohms: f64,
    /// The high output level of a comparator model that does not give one, in volts.
    pub comparator_high: f64,
    /// The low output level of a comparator model that does not give one, in volts.
    pub comparator_low: f64,
    /// The slope at zero of a comparator model that does not give one, in V/V.
    pub comparator_gain: f64,
    /// The most points of a DC sweep.
    pub max_dc_points: usize,
    /// The deepest nesting of `.include` files.
    pub max_include_depth: usize,
    /// The most files read through `.include` (and the model file) for one netlist.
    pub max_include_files: usize,
    /// The on resistance of a switch model that does not give one, in ohms.
    pub sw_ron: f64,
    /// The off resistance of a switch model that does not give one, in ohms.
    pub sw_roff: f64,
    /// The control voltage at which a switch model that does not give one is half on, in volts.
    pub sw_vt: f64,
    /// The half width of the transition of a switch model that does not give one, in volts.
    pub sw_vh: f64,
    /// The saturation current of an LED model that does not give one, in amperes.
    pub led_is: f64,
    /// The emission coefficient of an LED model that does not give one.
    pub led_n: f64,
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
    /// The most values of the grid of one `.step`.
    pub max_step_points: usize,
    /// The most combinations of all the `.step` lines of one study.
    pub max_step_runs: usize,
    /// The most runs of one Monte Carlo study (per combination of steps).
    pub max_mc_runs: usize,
    /// The number of Monte Carlo runs when the request does not give one.
    pub mc_runs: usize,
    /// The seed of the pseudo-random generator when the request does not give one.
    pub mc_seed: u64,
    /// How many standard deviations the tolerance band of a normal distribution is; the draws
    /// are cut at the band.
    pub mc_sigmas: f64,
    /// True when a tolerance follows a normal distribution unless it says otherwise; false is a
    /// uniform distribution over the band.
    pub mc_normal: bool,
    /// The percentiles (between 0 and 100) listed in the summary of a Monte Carlo study.
    pub mc_percentiles: Vec<f64>,
    /// The most measures of one study.
    pub max_measures: usize,
    /// The most values (runs times measures) of the table of one study.
    pub max_study_cells: usize,
    /// The fraction of the swing where a rise or fall time starts (10 % by default).
    pub measure_low: f64,
    /// The fraction of the swing where a rise or fall time ends (90 % by default).
    pub measure_high: f64,
    /// The relative change of a parameter for the numerical derivative of a sensitivity.
    pub sens_rel_step: f64,
    /// The highest order of the rational fit that looks for the poles and zeros.
    pub max_pole_order: usize,
    /// The reweighting passes of the rational fit.
    pub pole_fit_iterations: usize,
    /// The largest relative error of the fitted response that is accepted; above it the poles
    /// are refused.
    pub pole_fit_tolerance: f64,
    /// The most iterations of the root finder of a polynomial.
    pub pole_root_iterations: usize,
    /// The relative change at which the root finder stops.
    pub pole_root_tolerance: f64,
    /// A term of the numerator of the fit of the highest degrees that adds less than this
    /// fraction of the largest term at the top of the band is taken as zero.
    pub pole_trim: f64,
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
            gmin_steps: 10,
            gmin_start: 1e-2,
            gmin_factor: 10.0,
            max_breakpoints: 100_000,
            diode_ibv: 1e-3,
            bjt_is: 1e-16,
            bjt_bf: 100.0,
            bjt_br: 1.0,
            mos_vto: 0.0,
            mos_kp: 2e-5,
            mos_lambda: 0.0,
            mos_w: 1e-4,
            mos_l: 1e-4,
            opamp_gain: 1e5,
            opamp_gbw: 1e6,
            opamp_rout: 1.0,
            opamp_pole_ohms: 1000.0,
            comparator_high: 5.0,
            comparator_low: 0.0,
            comparator_gain: 1e4,
            max_dc_points: 100_000,
            max_include_depth: 4,
            max_include_files: 16,
            sw_ron: 1.0,
            sw_roff: 1e9,
            sw_vt: 0.0,
            sw_vh: 0.1,
            led_is: 1e-17,
            led_n: 2.0,
            write_retries: 5,
            write_retry_ms: 50,
            coherence_points: 41,
            coherence_db_tolerance: 1e-6,
            coherence_phase_tolerance_deg: 1e-5,
            coherence_delay_tolerance: 1e-4,
            max_step_points: 1_000,
            max_step_runs: 5_000,
            max_mc_runs: 100_000,
            mc_runs: 100,
            mc_seed: 1,
            mc_sigmas: 3.0,
            mc_normal: false,
            mc_percentiles: vec![5.0, 50.0, 95.0],
            max_measures: 200,
            max_study_cells: 10_000_000,
            measure_low: 0.1,
            measure_high: 0.9,
            sens_rel_step: 1e-3,
            max_pole_order: 8,
            pole_fit_iterations: 10,
            pole_fit_tolerance: 1e-3,
            pole_root_iterations: 1_000,
            pole_root_tolerance: 1e-13,
            pole_trim: 1e-6,
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

fn parse_percentages(value: &str) -> Option<Vec<f64>> {
    let mut out: Vec<f64> = Vec::new();
    for part in value.split(',') {
        let number: f64 = part.trim().parse().ok()?;
        if !(number.is_finite() && number > 0.0 && number < 100.0) {
            return None;
        }
        out.push(number);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
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
                "gmin_steps" => s.gmin_steps = value.parse().map_err(|_| bad())?,
                "gmin_start" => s.gmin_start = value.parse().map_err(|_| bad())?,
                "gmin_factor" => s.gmin_factor = value.parse().map_err(|_| bad())?,
                "max_breakpoints" => s.max_breakpoints = value.parse().map_err(|_| bad())?,
                "diode_ibv" => s.diode_ibv = value.parse().map_err(|_| bad())?,
                "bjt_is" => s.bjt_is = value.parse().map_err(|_| bad())?,
                "bjt_bf" => s.bjt_bf = value.parse().map_err(|_| bad())?,
                "bjt_br" => s.bjt_br = value.parse().map_err(|_| bad())?,
                "mos_vto" => s.mos_vto = value.parse().map_err(|_| bad())?,
                "mos_kp" => s.mos_kp = value.parse().map_err(|_| bad())?,
                "mos_lambda" => s.mos_lambda = value.parse().map_err(|_| bad())?,
                "mos_w" => s.mos_w = value.parse().map_err(|_| bad())?,
                "mos_l" => s.mos_l = value.parse().map_err(|_| bad())?,
                "opamp_gain" => s.opamp_gain = value.parse().map_err(|_| bad())?,
                "opamp_gbw" => s.opamp_gbw = value.parse().map_err(|_| bad())?,
                "opamp_rout" => s.opamp_rout = value.parse().map_err(|_| bad())?,
                "opamp_pole_ohms" => s.opamp_pole_ohms = value.parse().map_err(|_| bad())?,
                "comparator_high" => s.comparator_high = value.parse().map_err(|_| bad())?,
                "comparator_low" => s.comparator_low = value.parse().map_err(|_| bad())?,
                "comparator_gain" => s.comparator_gain = value.parse().map_err(|_| bad())?,
                "max_dc_points" => s.max_dc_points = value.parse().map_err(|_| bad())?,
                "max_include_depth" => s.max_include_depth = value.parse().map_err(|_| bad())?,
                "max_include_files" => s.max_include_files = value.parse().map_err(|_| bad())?,
                "sw_ron" => s.sw_ron = value.parse().map_err(|_| bad())?,
                "sw_roff" => s.sw_roff = value.parse().map_err(|_| bad())?,
                "sw_vt" => s.sw_vt = value.parse().map_err(|_| bad())?,
                "sw_vh" => s.sw_vh = value.parse().map_err(|_| bad())?,
                "led_is" => s.led_is = value.parse().map_err(|_| bad())?,
                "led_n" => s.led_n = value.parse().map_err(|_| bad())?,
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
                "max_step_points" => s.max_step_points = value.parse().map_err(|_| bad())?,
                "max_step_runs" => s.max_step_runs = value.parse().map_err(|_| bad())?,
                "max_mc_runs" => s.max_mc_runs = value.parse().map_err(|_| bad())?,
                "mc_runs" => s.mc_runs = value.parse().map_err(|_| bad())?,
                "mc_seed" => s.mc_seed = value.parse().map_err(|_| bad())?,
                "mc_sigmas" => s.mc_sigmas = value.parse().map_err(|_| bad())?,
                "mc_normal" => s.mc_normal = parse_bool(value).ok_or_else(bad)?,
                "mc_percentiles" => s.mc_percentiles = parse_percentages(value).ok_or_else(bad)?,
                "max_measures" => s.max_measures = value.parse().map_err(|_| bad())?,
                "max_study_cells" => s.max_study_cells = value.parse().map_err(|_| bad())?,
                "measure_low" => s.measure_low = value.parse().map_err(|_| bad())?,
                "measure_high" => s.measure_high = value.parse().map_err(|_| bad())?,
                "sens_rel_step" => s.sens_rel_step = value.parse().map_err(|_| bad())?,
                "max_pole_order" => s.max_pole_order = value.parse().map_err(|_| bad())?,
                "pole_fit_iterations" => {
                    s.pole_fit_iterations = value.parse().map_err(|_| bad())?
                }
                "pole_fit_tolerance" => s.pole_fit_tolerance = value.parse().map_err(|_| bad())?,
                "pole_root_iterations" => {
                    s.pole_root_iterations = value.parse().map_err(|_| bad())?
                }
                "pole_root_tolerance" => {
                    s.pole_root_tolerance = value.parse().map_err(|_| bad())?
                }
                "pole_trim" => s.pole_trim = value.parse().map_err(|_| bad())?,
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
        if !positive(self.gmin_start) || !self.gmin_factor.is_finite() || self.gmin_factor <= 1.0 {
            return fail("gmin_start must be positive and gmin_factor above 1");
        }
        if self.max_breakpoints < 1 {
            return fail("max_breakpoints must be at least 1");
        }
        if !positive(self.diode_ibv)
            || !positive(self.bjt_is)
            || !positive(self.bjt_bf)
            || !positive(self.bjt_br)
        {
            return fail("diode_ibv, bjt_is, bjt_bf and bjt_br must be positive");
        }
        if !self.mos_vto.is_finite() || !self.mos_lambda.is_finite() || self.mos_lambda < 0.0 {
            return fail("mos_vto must be a number and mos_lambda cannot be negative");
        }
        if !positive(self.mos_kp) || !positive(self.mos_w) || !positive(self.mos_l) {
            return fail("mos_kp, mos_w and mos_l must be positive");
        }
        if !positive(self.opamp_gain)
            || !self.opamp_gbw.is_finite()
            || self.opamp_gbw < 0.0
            || !positive(self.opamp_rout)
            || !positive(self.opamp_pole_ohms)
        {
            return fail("opamp_gain, opamp_rout and opamp_pole_ohms must be positive and opamp_gbw cannot be negative");
        }
        if !self.comparator_high.is_finite()
            || !self.comparator_low.is_finite()
            || self.comparator_high <= self.comparator_low
            || !positive(self.comparator_gain)
        {
            return fail(
                "comparator_high must be above comparator_low and comparator_gain positive",
            );
        }
        if self.max_dc_points < 1 || self.max_include_depth < 1 || self.max_include_files < 1 {
            return fail(
                "max_dc_points, max_include_depth and max_include_files must be at least 1",
            );
        }
        if !positive(self.sw_ron)
            || !positive(self.sw_roff)
            || self.sw_roff <= self.sw_ron
            || !self.sw_vt.is_finite()
            || !positive(self.sw_vh)
        {
            return fail(
                "sw_ron must be positive, sw_roff above sw_ron, sw_vt a number and sw_vh positive",
            );
        }
        if !positive(self.led_is) || !positive(self.led_n) {
            return fail("led_is and led_n must be positive");
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
        if self.max_step_points < 1 || self.max_step_runs < 1 || self.max_mc_runs < 1 {
            return fail("max_step_points, max_step_runs and max_mc_runs must be at least 1");
        }
        if self.mc_runs < 1 || self.mc_runs > self.max_mc_runs {
            return fail("mc_runs must be between 1 and max_mc_runs");
        }
        if !self.mc_sigmas.is_finite() || self.mc_sigmas < 1.0 {
            return fail("mc_sigmas must be at least 1");
        }
        if self.mc_percentiles.is_empty() {
            return fail("mc_percentiles needs at least one value");
        }
        if self.max_measures < 1 || self.max_study_cells < 1 {
            return fail("max_measures and max_study_cells must be at least 1");
        }
        if !(self.measure_low > 0.0
            && self.measure_low < self.measure_high
            && self.measure_high < 1.0)
        {
            return fail("measure_low and measure_high must satisfy 0 < low < high < 1");
        }
        if !positive(self.sens_rel_step) || self.sens_rel_step >= 0.5 {
            return fail("sens_rel_step must be between 0 and 0.5");
        }
        if !(1..=20).contains(&self.max_pole_order) {
            return fail("max_pole_order must be between 1 and 20");
        }
        if self.pole_fit_iterations < 1 || self.pole_root_iterations < 1 {
            return fail("pole_fit_iterations and pole_root_iterations must be at least 1");
        }
        if !positive(self.pole_fit_tolerance)
            || !positive(self.pole_root_tolerance)
            || !positive(self.pole_trim)
            || self.pole_trim >= 1.0
        {
            return fail("pole_fit_tolerance and pole_root_tolerance must be positive and pole_trim between 0 and 1");
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
        self.netlist_ground.contains(&lower)
    }

    /// True when the name of a schematic net means ground (compared in lower case).
    pub fn is_schematic_ground(&self, name: &str) -> bool {
        let lower = name.trim().to_ascii_lowercase();
        self.schematic_ground.contains(&lower)
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
        let t = SimSettings::parse("gmin_steps = 0\nopamp_gbw = 0\nmos_w = 2e-5\n").unwrap();
        assert_eq!(t.gmin_steps, 0);
        assert_eq!(t.opamp_gbw, 0.0);
        assert_eq!(t.mos_w, 2e-5);
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
            ("gmin_factor = 1", "gmin_factor"),
            ("gmin_start = 0", "gmin_start"),
            ("max_breakpoints = 0", "max_breakpoints"),
            ("bjt_bf = 0", "bjt_bf"),
            ("mos_kp = -1", "mos_kp"),
            ("mos_lambda = -1", "mos_lambda"),
            ("opamp_gain = 0", "opamp_gain"),
            ("opamp_gbw = -1", "opamp_gbw"),
            ("comparator_low = 6", "comparator_high"),
            ("max_dc_points = 0", "max_dc_points"),
            ("sw_roff = 0.5", "sw_roff"),
            ("sw_vh = 0", "sw_vh"),
            ("led_n = 0", "led_n"),
            ("mc_runs = 0", "mc_runs"),
            ("mc_runs = 200000", "mc_runs"),
            ("mc_sigmas = 0.5", "mc_sigmas"),
            ("mc_percentiles = 120", "mc_percentiles"),
            ("measure_low = 0.95", "measure_low"),
            ("sens_rel_step = 0.9", "sens_rel_step"),
            ("max_pole_order = 40", "max_pole_order"),
            ("pole_trim = 2", "pole_trim"),
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
