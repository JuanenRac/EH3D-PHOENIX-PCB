// SPDX-License-Identifier: GPL-3.0-or-later
//! The types of structure the generator draws, with the parameters each one takes, and the
//! helpers they share. Every generator is a function from the environment and the parameters to a
//! [`Layout`] in a local frame and the [`Report`] of what it calculated.

mod antennas;
mod couplers;
mod filters;
mod launch;
mod lines;
mod stubs;

use crate::codes::{RfgenError, KIND_UNKNOWN, WARN_APPROXIMATE, WARN_FEATURE};
use crate::env::{Environment, Line};
use crate::formulas::open_end_range;
use crate::layout::{rect_points, Layout};
use crate::params::{ParamSpec, Params};
use crate::report::Report;

/// The names of the nets of a structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetNames {
    /// The signal net; the other signal nets of a structure are named from it.
    pub signal: String,
    /// The ground net.
    pub ground: String,
}

impl Default for NetNames {
    fn default() -> Self {
        NetNames {
            signal: "RF".to_owned(),
            ground: "GND".to_owned(),
        }
    }
}

/// A type of structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A microstrip line of an electrical length.
    Line,
    /// A right-angle bend with the optimum miter.
    Bend,
    /// A right-angle bend as an arc.
    Curve,
    /// A transition between two widths.
    Taper,
    /// A shunt stub with an open end.
    StubOpen,
    /// A shunt stub short-circuited by a via.
    StubShort,
    /// The single shunt stub that matches a load.
    StubMatch,
    /// A quarter-wave transformer.
    QuarterWave,
    /// An L network made of a line and a stub.
    LMatch,
    /// A stepped-impedance low-pass filter.
    Lowpass,
    /// A coupled-line band-pass filter.
    Bandpass,
    /// A Wilkinson power divider.
    Wilkinson,
    /// A branch-line coupler.
    BranchLine,
    /// A rectangular patch antenna with an inset feed.
    Patch,
    /// A printed half-wave dipole.
    Dipole,
    /// An edge SMA launch with a via fence.
    Sma,
}

const Z0: ParamSpec = ParamSpec {
    name: "z0",
    unit: "ohm",
    default: "default_z0_ohm",
    help: "the impedance of the line",
};

const TURN: ParamSpec = ParamSpec {
    name: "turn",
    unit: "",
    default: "left",
    help: "left or right",
};

const ARM: ParamSpec = ParamSpec {
    name: "arm_mm",
    unit: "mm",
    default: "lead_length_nm, or the minimum arm of a bend",
    help: "the length of each arm",
};

const LINE_PARAMS: &[ParamSpec] = &[
    Z0,
    ParamSpec {
        name: "degrees",
        unit: "deg",
        default: "default_line_degrees",
        help: "the electrical length at the guided wavelength",
    },
    ParamSpec {
        name: "length_mm",
        unit: "mm",
        default: "",
        help: "the physical length, instead of degrees",
    },
];

const BEND_PARAMS: &[ParamSpec] = &[Z0, TURN, ARM];

const CURVE_PARAMS: &[ParamSpec] = &[
    Z0,
    TURN,
    ARM,
    ParamSpec {
        name: "radius_mm",
        unit: "mm",
        default: "curve_radius_over_width",
        help: "the radius of the centre line",
    },
];

const TAPER_PARAMS: &[ParamSpec] = &[
    Z0,
    ParamSpec {
        name: "z0_b",
        unit: "ohm",
        default: "",
        help: "the impedance at the other end (needed)",
    },
    ParamSpec {
        name: "degrees",
        unit: "deg",
        default: "taper_length_degrees",
        help: "the electrical length of the transition",
    },
];

const STUB_PARAMS: &[ParamSpec] = &[
    Z0,
    ParamSpec {
        name: "zs",
        unit: "ohm",
        default: "z0",
        help: "the impedance of the stub",
    },
    ParamSpec {
        name: "degrees",
        unit: "deg",
        default: "90",
        help: "the electrical length of the stub",
    },
    ParamSpec {
        name: "main_degrees",
        unit: "deg",
        default: "stub_main_degrees",
        help: "the electrical length of the main line",
    },
];

const STUB_MATCH_PARAMS: &[ParamSpec] = &[
    Z0,
    ParamSpec {
        name: "load_re",
        unit: "ohm",
        default: "",
        help: "the real part of the load (needed)",
    },
    ParamSpec {
        name: "load_im",
        unit: "ohm",
        default: "0",
        help: "the imaginary part of the load",
    },
    ParamSpec {
        name: "termination",
        unit: "",
        default: "open",
        help: "open or short",
    },
    ParamSpec {
        name: "solution",
        unit: "",
        default: "1",
        help: "which match, the nearest to the load first",
    },
];

const QUARTER_WAVE_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "z1",
        unit: "ohm",
        default: "default_z0_ohm",
        help: "the impedance of the first line",
    },
    ParamSpec {
        name: "z2",
        unit: "ohm",
        default: "",
        help: "the impedance of the second line (needed)",
    },
];

const LMATCH_PARAMS: &[ParamSpec] = &[
    Z0,
    ParamSpec {
        name: "load_re",
        unit: "ohm",
        default: "",
        help: "the real part of the load (needed)",
    },
    ParamSpec {
        name: "load_im",
        unit: "ohm",
        default: "0",
        help: "the imaginary part of the load",
    },
];

const LOWPASS_PARAMS: &[ParamSpec] = &[
    Z0,
    ParamSpec {
        name: "family",
        unit: "",
        default: "butterworth",
        help: "butterworth or chebyshev",
    },
    ParamSpec {
        name: "order",
        unit: "",
        default: "5",
        help: "the number of sections",
    },
    ParamSpec {
        name: "ripple_db",
        unit: "dB",
        default: "0.5",
        help: "the ripple of a Chebyshev filter",
    },
];

const BANDPASS_PARAMS: &[ParamSpec] = &[
    Z0,
    ParamSpec {
        name: "family",
        unit: "",
        default: "chebyshev",
        help: "butterworth or chebyshev",
    },
    ParamSpec {
        name: "order",
        unit: "",
        default: "3",
        help: "the number of resonators",
    },
    ParamSpec {
        name: "ripple_db",
        unit: "dB",
        default: "0.5",
        help: "the ripple of a Chebyshev filter",
    },
    ParamSpec {
        name: "bandwidth_percent",
        unit: "%",
        default: "bandpass_bandwidth_percent",
        help: "the fractional bandwidth",
    },
];

const COUPLER_PARAMS: &[ParamSpec] = &[Z0];

const PATCH_PARAMS: &[ParamSpec] = &[ParamSpec {
    name: "z0",
    unit: "ohm",
    default: "default_z0_ohm",
    help: "the impedance of the feed line",
}];

const DIPOLE_PARAMS: &[ParamSpec] = &[];

const SMA_PARAMS: &[ParamSpec] = &[
    Z0,
    ParamSpec {
        name: "length_mm",
        unit: "mm",
        default: "10",
        help: "the length of the line and of the via fence",
    },
];

impl Kind {
    /// Every type, in the order of `phoenix rfgen --list`.
    pub const ALL: [Kind; 16] = [
        Kind::Line,
        Kind::Bend,
        Kind::Curve,
        Kind::Taper,
        Kind::StubOpen,
        Kind::StubShort,
        Kind::StubMatch,
        Kind::QuarterWave,
        Kind::LMatch,
        Kind::Lowpass,
        Kind::Bandpass,
        Kind::Wilkinson,
        Kind::BranchLine,
        Kind::Patch,
        Kind::Dipole,
        Kind::Sma,
    ];

    /// The name used on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Line => "line",
            Kind::Bend => "bend",
            Kind::Curve => "curve",
            Kind::Taper => "taper",
            Kind::StubOpen => "stub-open",
            Kind::StubShort => "stub-short",
            Kind::StubMatch => "stub-match",
            Kind::QuarterWave => "quarter-wave",
            Kind::LMatch => "l-match",
            Kind::Lowpass => "lowpass",
            Kind::Bandpass => "bandpass",
            Kind::Wilkinson => "wilkinson",
            Kind::BranchLine => "branch-line",
            Kind::Patch => "patch",
            Kind::Dipole => "dipole",
            Kind::Sma => "sma",
        }
    }

    /// The type with a name.
    pub fn from_name(name: &str) -> Result<Kind, RfgenError> {
        Kind::ALL
            .iter()
            .copied()
            .find(|k| k.name() == name)
            .ok_or_else(|| {
                RfgenError::new(
                    KIND_UNKNOWN,
                    format!("{name:?} is not one of the types of `phoenix rfgen --list`"),
                )
            })
    }

    /// One line about the type.
    pub fn summary(self) -> &'static str {
        match self {
            Kind::Line => "microstrip line of an electrical length, 50 ohm by default",
            Kind::Bend => "right-angle bend with the optimum miter (Douville and James)",
            Kind::Curve => "right-angle bend as an arc of a given radius",
            Kind::Taper => "linear transition between the widths of two impedances",
            Kind::StubOpen => "shunt stub with an open end, corrected for the fringing field",
            Kind::StubShort => "shunt stub short-circuited by a via",
            Kind::StubMatch => "single shunt stub that matches a complex load (from the RF tools)",
            Kind::QuarterWave => "quarter-wave transformer between two real impedances",
            Kind::LMatch => "L network of a series line and a shunt stub for a complex load",
            Kind::Lowpass => "stepped-impedance low-pass filter (Richards, Kuroda)",
            Kind::Bandpass => "coupled-line band-pass filter with open ends (inverters)",
            Kind::Wilkinson => "Wilkinson power divider with quarter-wave arms",
            Kind::BranchLine => "branch-line (quadrature hybrid) coupler as a rectangular ring",
            Kind::Patch => "rectangular patch antenna with an inset feed",
            Kind::Dipole => "printed half-wave dipole with a balanced feed",
            Kind::Sma => "edge SMA launch with a line and a via fence",
        }
    }

    /// The parameters the type takes.
    pub fn params(self) -> &'static [ParamSpec] {
        match self {
            Kind::Line => LINE_PARAMS,
            Kind::Bend => BEND_PARAMS,
            Kind::Curve => CURVE_PARAMS,
            Kind::Taper => TAPER_PARAMS,
            Kind::StubOpen | Kind::StubShort => STUB_PARAMS,
            Kind::StubMatch => STUB_MATCH_PARAMS,
            Kind::QuarterWave => QUARTER_WAVE_PARAMS,
            Kind::LMatch => LMATCH_PARAMS,
            Kind::Lowpass => LOWPASS_PARAMS,
            Kind::Bandpass => BANDPASS_PARAMS,
            Kind::Wilkinson | Kind::BranchLine => COUPLER_PARAMS,
            Kind::Patch => PATCH_PARAMS,
            Kind::Dipole => DIPOLE_PARAMS,
            Kind::Sma => SMA_PARAMS,
        }
    }
}

/// Draws a structure in its local frame.
pub fn build(
    kind: Kind,
    env: &Environment,
    params: &Params,
    nets: &NetNames,
) -> Result<(Layout, Report), RfgenError> {
    params.check_known(kind.params())?;
    let mut report = Report::new(kind.name(), env.freq_ghz, &env.layer);
    let mut layout = Layout::new();
    match kind {
        Kind::Line => lines::line(env, params, nets, &mut layout, &mut report)?,
        Kind::Bend => lines::bend(env, params, nets, &mut layout, &mut report)?,
        Kind::Curve => lines::curve(env, params, nets, &mut layout, &mut report)?,
        Kind::Taper => lines::taper(env, params, nets, &mut layout, &mut report)?,
        Kind::StubOpen => stubs::stub(env, params, nets, &mut layout, &mut report, false)?,
        Kind::StubShort => stubs::stub(env, params, nets, &mut layout, &mut report, true)?,
        Kind::StubMatch => stubs::stub_match(env, params, nets, &mut layout, &mut report)?,
        Kind::QuarterWave => stubs::quarter_wave(env, params, nets, &mut layout, &mut report)?,
        Kind::LMatch => stubs::l_match(env, params, nets, &mut layout, &mut report)?,
        Kind::Lowpass => filters::lowpass(env, params, nets, &mut layout, &mut report)?,
        Kind::Bandpass => filters::bandpass(env, params, nets, &mut layout, &mut report)?,
        Kind::Wilkinson => couplers::wilkinson(env, params, nets, &mut layout, &mut report)?,
        Kind::BranchLine => couplers::branch_line(env, params, nets, &mut layout, &mut report)?,
        Kind::Patch => antennas::patch(env, params, nets, &mut layout, &mut report)?,
        Kind::Dipole => antennas::dipole(env, params, nets, &mut layout, &mut report)?,
        Kind::Sma => launch::sma(env, params, nets, &mut layout, &mut report)?,
    }
    finish(kind, env, nets, &mut layout, &mut report);
    Ok((layout, report))
}

/// The notes that every structure carries: a shape narrower than the minimum, the model of the
/// microstrip and the ground pour.
fn finish(
    kind: Kind,
    env: &Environment,
    nets: &NetNames,
    layout: &mut Layout,
    report: &mut Report,
) {
    let settings = &env.settings;
    if layout.narrowest_nm > 0 && layout.narrowest_nm < settings.min_feature_nm {
        report.warn(
            WARN_FEATURE,
            &format!(
                "the narrowest shape is {} nm and min_feature_nm is {} nm",
                layout.narrowest_nm, settings.min_feature_nm
            ),
        );
    }
    report.formula(
        "microstrip impedance, effective permittivity, guided wavelength",
        "Hammerstad and Jensen 1980 and Wheeler, through eh3d-phoenix-stackup (quasi-static)",
        "0.01 <= W/h <= 100, 1 <= er <= 128; no dispersion",
        &[],
    );
    if settings.ground_pour && kind != Kind::Dipole {
        if let Some(((x0, y0), (x1, y1))) = layout.bbox() {
            let margin = settings.ground_margin_nm;
            let ground = layout.net(&nets.ground);
            layout.plane_rect(ground, x0 - margin, y0 - margin, x1 + margin, y1 + margin);
        }
    }
    report.warn(
        WARN_APPROXIMATE,
        "prototype: not compared with an electromagnetic simulator or a measurement; \
         discontinuities (steps, T junctions, open ends of coupled lines) are not modelled",
    );
}

/// Adds the width, impedance, effective permittivity and guided wavelength of a line to the
/// report under a name, and the warnings of the stack-up calculation.
pub(crate) fn describe_line(report: &mut Report, name: &str, line: &Line) {
    report.dim_mm(&format!("{name} width"), line.width_m());
    report.dim(&format!("{name} impedance"), line.z0_ohm, "ohm");
    report.dim(&format!("{name} eps_eff"), line.eps_eff, "");
    report.dim_mm(&format!("{name} guided wavelength"), line.lambda_g_m);
    for (code, detail) in &line.warnings {
        report.warn(code, detail);
    }
}

/// The ranges of the open-end extension for a line of this environment.
pub(crate) fn open_end_problems(env: &Environment, width_m: f64) -> Vec<String> {
    open_end_range(width_m / env.substrate.h_m, env.substrate.er)
}

/// The source and the range of the open-end extension, for the report.
pub(crate) const OPEN_END_SOURCE: &str = "Hammerstad 1975 (5th European Microwave Conference)";
/// The range text of the open-end extension.
pub(crate) const OPEN_END_RANGE: &str = "0.1 <= W/h <= 100, 1 <= er <= 20";

/// A strip along x between `x0` and `x1`, centred on `yc`, of an even width.
pub(crate) fn h_strip(layout: &mut Layout, net: usize, x0: i64, x1: i64, yc: i64, w: i64) {
    let half = w / 2;
    layout.poly_with_width(net, rect_points(x0, yc - half, x1, yc + half), w);
}

/// A strip along y between `y0` and `y1`, centred on `xc`, of an even width.
pub(crate) fn v_strip(layout: &mut Layout, net: usize, xc: i64, y0: i64, y1: i64, w: i64) {
    let half = w / 2;
    layout.poly_with_width(net, rect_points(xc - half, y0, xc + half, y1), w);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_has_a_unique_name_that_reads_back() {
        let mut names: Vec<&str> = Kind::ALL.iter().map(|k| k.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 16);
        for kind in Kind::ALL {
            assert_eq!(Kind::from_name(kind.name()).unwrap(), kind);
            assert!(!kind.summary().is_empty());
        }
        assert_eq!(Kind::from_name("amplifier").unwrap_err().code, KIND_UNKNOWN);
    }

    #[test]
    fn the_parameters_of_every_type_are_documented_and_unique() {
        for kind in Kind::ALL {
            let specs = kind.params();
            let mut names: Vec<&str> = specs.iter().map(|s| s.name).collect();
            let all = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), all, "{kind:?}");
            for spec in specs {
                assert!(!spec.name.is_empty() && !spec.help.is_empty(), "{kind:?}");
            }
        }
    }

    #[test]
    fn the_default_names_are_the_signal_and_ground_nets() {
        let nets = NetNames::default();
        assert_eq!((nets.signal.as_str(), nets.ground.as_str()), ("RF", "GND"));
    }
}
