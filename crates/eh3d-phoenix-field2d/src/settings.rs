// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings of the field solver, read from `config/field2d.cfg`. Every tuned value (the
//! solver, its tolerance, the limits of cells and iterations, the fineness of the mesh near the
//! edges of the tracks, the distance of the walls) is a line of that file and not a number in the
//! code. A missing line keeps the default, an unknown key is an error.

use crate::codes::{FieldError, SETTINGS_INVALID};

/// The method that solves the system of equations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Conjugate gradients with a diagonal preconditioner.
    ConjugateGradient,
    /// Successive over-relaxation in a fixed order of the nodes.
    Sor,
}

impl Method {
    /// The word of the settings file.
    pub fn as_str(self) -> &'static str {
        match self {
            Method::ConjugateGradient => "cg",
            Method::Sor => "sor",
        }
    }
}

/// The tuned values of the field solver.
#[derive(Debug, Clone, PartialEq)]
pub struct Field2dSettings {
    /// The method that solves the Laplace equation.
    pub method: Method,
    /// The solver stops when the residual is this fraction of the first residual.
    pub tolerance: f64,
    /// The most iterations of one solution.
    pub max_iterations: u32,
    /// The most cells of the mesh.
    pub max_cells: usize,
    /// The factor of over-relaxation of the method `sor`, between 1 and 2.
    pub sor_omega: f64,
    /// The size of a cell at the edge of a conductor, as a fraction of the smallest feature of
    /// the section (a thickness, a width, a gap, the height of a layer).
    pub edge_step_fraction: f64,
    /// The largest cell, as a fraction of the smaller side of the box.
    pub max_step_fraction: f64,
    /// How much a cell may be larger than the one before it when the mesh grows away from an edge.
    pub grading_ratio: f64,
    /// Every step of the mesh is divided by this number (at least 1); a convergence study
    /// raises it.
    pub refine: f64,
    /// The distance from the track to the walls of the box, as a multiple of the sum of the
    /// height of the section and the width taken by the conductors.
    pub box_margin_factor: f64,
    /// The frequency, in GHz, at which the permittivity and the loss tangent of the materials are
    /// taken when the command does not give one.
    pub freq_ghz: f64,
    /// A difference from the closed formula above this percentage is a warning.
    pub compare_warn_percent: f64,
    /// The most lines one run solves.
    pub max_cases: usize,
}

impl Default for Field2dSettings {
    fn default() -> Self {
        Field2dSettings {
            method: Method::ConjugateGradient,
            tolerance: 1.0e-10,
            max_iterations: 50_000,
            max_cells: 400_000,
            sor_omega: 1.9,
            edge_step_fraction: 0.25,
            max_step_fraction: 0.1,
            grading_ratio: 1.25,
            refine: 1.0,
            box_margin_factor: 8.0,
            freq_ghz: 1.0,
            compare_warn_percent: 5.0,
            max_cases: 200,
        }
    }
}

fn bad(line: usize, key: &str, reason: &str) -> FieldError {
    FieldError::new(SETTINGS_INVALID, format!("line {line}: {key}: {reason}"))
}

fn number(value: &str, line: usize, key: &str) -> Result<f64, FieldError> {
    let parsed: f64 = value.parse().map_err(|_| bad(line, key, "not a number"))?;
    if !parsed.is_finite() {
        return Err(bad(line, key, "not a finite number"));
    }
    Ok(parsed)
}

impl Field2dSettings {
    /// Parse a settings text of `key = value` lines. Blank lines and lines starting with `#` are
    /// ignored; a missing key keeps its default; an unknown key is an error.
    pub fn parse(text: &str) -> Result<Field2dSettings, FieldError> {
        let mut s = Field2dSettings::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed.split_once('=').ok_or_else(|| {
                FieldError::new(SETTINGS_INVALID, format!("line {line}: not key = value"))
            })?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "method" => {
                    s.method = match value {
                        "cg" => Method::ConjugateGradient,
                        "sor" => Method::Sor,
                        _ => return Err(bad(line, key, "must be cg or sor")),
                    }
                }
                "tolerance" => s.tolerance = number(value, line, key)?,
                "max_iterations" => {
                    s.max_iterations = value
                        .parse()
                        .map_err(|_| bad(line, key, "not a whole number"))?
                }
                "max_cells" => {
                    s.max_cells = value
                        .parse()
                        .map_err(|_| bad(line, key, "not a whole number"))?
                }
                "sor_omega" => s.sor_omega = number(value, line, key)?,
                "edge_step_fraction" => s.edge_step_fraction = number(value, line, key)?,
                "max_step_fraction" => s.max_step_fraction = number(value, line, key)?,
                "grading_ratio" => s.grading_ratio = number(value, line, key)?,
                "refine" => s.refine = number(value, line, key)?,
                "box_margin_factor" => s.box_margin_factor = number(value, line, key)?,
                "freq_ghz" => s.freq_ghz = number(value, line, key)?,
                "compare_warn_percent" => s.compare_warn_percent = number(value, line, key)?,
                "max_cases" => {
                    s.max_cases = value
                        .parse()
                        .map_err(|_| bad(line, key, "not a whole number"))?
                }
                _ => return Err(bad(line, key, "unknown key")),
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Check the ranges the algorithms rely on.
    pub fn validate(&self) -> Result<(), FieldError> {
        let fail = |what: &str| Err(FieldError::new(SETTINGS_INVALID, what.to_string()));
        if !(1.0e-15..=1.0e-2).contains(&self.tolerance) {
            return fail("tolerance must be between 1e-15 and 1e-2");
        }
        if !(1..=10_000_000).contains(&self.max_iterations) {
            return fail("max_iterations must be between 1 and 10000000");
        }
        if !(100..=50_000_000).contains(&self.max_cells) {
            return fail("max_cells must be between 100 and 50000000");
        }
        if !(1.0..2.0).contains(&self.sor_omega) {
            return fail("sor_omega must be at least 1 and below 2");
        }
        if !(0.001..=2.0).contains(&self.edge_step_fraction) {
            return fail("edge_step_fraction must be between 0.001 and 2");
        }
        if !(0.001..=1.0).contains(&self.max_step_fraction) {
            return fail("max_step_fraction must be between 0.001 and 1");
        }
        if !(1.0..=3.0).contains(&self.grading_ratio) {
            return fail("grading_ratio must be between 1 and 3");
        }
        if !(1.0..=64.0).contains(&self.refine) {
            return fail("refine must be between 1 and 64");
        }
        if !(1.0..=1000.0).contains(&self.box_margin_factor) {
            return fail("box_margin_factor must be between 1 and 1000");
        }
        if !(1.0e-6..=1000.0).contains(&self.freq_ghz) {
            return fail("freq_ghz must be between 1e-6 and 1000");
        }
        if !(0.0..=1000.0).contains(&self.compare_warn_percent) {
            return fail("compare_warn_percent must be between 0 and 1000");
        }
        if !(1..=100_000).contains(&self.max_cases) {
            return fail("max_cases must be between 1 and 100000");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_text_keeps_the_defaults() {
        assert_eq!(
            Field2dSettings::parse("").unwrap(),
            Field2dSettings::default()
        );
        assert_eq!(
            Field2dSettings::parse("# a comment\n\n").unwrap(),
            Field2dSettings::default()
        );
    }

    #[test]
    fn the_shipped_file_is_the_default() {
        let text = include_str!("../../../config/field2d.cfg");
        assert_eq!(
            Field2dSettings::parse(text).unwrap(),
            Field2dSettings::default()
        );
    }

    #[test]
    fn every_key_is_read() {
        let text = "method = sor\ntolerance = 1e-8\nmax_iterations = 10\nmax_cells = 1000\n\
                    sor_omega = 1.5\nedge_step_fraction = 0.5\nmax_step_fraction = 0.2\n\
                    grading_ratio = 1.5\nrefine = 2\nbox_margin_factor = 4\nfreq_ghz = 2\n\
                    compare_warn_percent = 3\nmax_cases = 7\n";
        let s = Field2dSettings::parse(text).unwrap();
        assert_eq!(s.method, Method::Sor);
        assert_eq!(s.tolerance, 1.0e-8);
        assert_eq!(s.max_iterations, 10);
        assert_eq!(s.max_cells, 1000);
        assert_eq!(s.sor_omega, 1.5);
        assert_eq!(s.edge_step_fraction, 0.5);
        assert_eq!(s.max_step_fraction, 0.2);
        assert_eq!(s.grading_ratio, 1.5);
        assert_eq!(s.refine, 2.0);
        assert_eq!(s.box_margin_factor, 4.0);
        assert_eq!(s.freq_ghz, 2.0);
        assert_eq!(s.compare_warn_percent, 3.0);
        assert_eq!(s.max_cases, 7);
        assert_eq!(s.method.as_str(), "sor");
    }

    #[test]
    fn bad_lines_are_refused_with_the_code_of_the_settings() {
        for text in [
            "nothing",
            "unknown = 1",
            "method = fast",
            "tolerance = abc",
            "tolerance = 0",
            "tolerance = nan",
            "max_iterations = 0",
            "max_iterations = -3",
            "max_cells = 5",
            "sor_omega = 2",
            "edge_step_fraction = 0",
            "max_step_fraction = 5",
            "grading_ratio = 0.5",
            "refine = 0.5",
            "box_margin_factor = 0",
            "freq_ghz = 0",
            "compare_warn_percent = -1",
            "max_cases = 0",
        ] {
            let error = Field2dSettings::parse(text).unwrap_err();
            assert_eq!(error.code, SETTINGS_INVALID, "{text}");
        }
    }
}
