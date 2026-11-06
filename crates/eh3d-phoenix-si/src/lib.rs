// SPDX-License-Identifier: GPL-3.0-or-later
//! Signal integrity estimates of a board: crosstalk, terminations and reflections, rise time and
//! loss, a simplified eye diagram and the return path of the tracks.
//!
//! * [`phys`]: the closed formulas (bandwidth from the rise time, the reflection coefficient, the
//!   coupling of two lines and the near-end and far-end crosstalk, the cutoff and the rise time of
//!   a low-pass stage, the loss of a line without a stack-up), each with its source or the
//!   statement that it is an estimate made here;
//! * [`bergeron`]: the reflections of a lossless line between a driver and a load by the method of
//!   Bergeron, with the diagram of bounces and the time stepping with capacitors, from which the
//!   overshoot, the ringback and the settling are measured;
//! * [`eye`]: a pseudo-random sequence with a generator of its own and a fixed seed through the
//!   rise time and a low-pass filter, folded into an eye whose height, width and jitter are
//!   measured;
//! * [`crosstalk`], [`reflect`], [`eyenet`] and [`returnpath`]: the analyses of a board, which take
//!   the geometry from the tracks, the zones and the vias, and the line from the stack-up;
//! * [`settings`]: every threshold, driver, load and limit, read from `config/si.cfg`;
//! * [`report`]: the text and JSON of a report, the same every time;
//! * [`codes`]: the codes `PHOENIX-SI-001` to `PHOENIX-SI-020` with their titles in the eight
//!   languages.
//!
//! **State: prototype, quasi-static.** Every number is an estimate from a closed formula applied to
//! the geometry of the board and the stack-up: it has not been compared with a field solver, a
//! circuit simulator, another tool or a measurement of a real board. The lines are lossless in the
//! reflections and lossy only in the rise time and the eye; branches, the pads of the parts, the
//! package of the driver and the receivers are not in the model.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bergeron;
pub mod codes;
pub mod crosstalk;
mod env;
pub mod eye;
pub mod eyenet;
mod netdata;
pub mod phys;
pub mod reflect;
pub mod report;
pub mod returnpath;
pub mod settings;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_stackup::StackupJob;

pub use codes::{title, Finding, Severity, SiError};
pub use report::{findings_text, report_json, report_text, SiReport, STATE_NOTE};
pub use settings::SiSettings;

use codes::{LIMIT_EXCEEDED, NET_NOT_FOUND, NO_REFERENCE, SETTINGS_INVALID};
use env::Env;

/// What to work out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Analysis {
    /// The crosstalk between parallel runs of tracks.
    Crosstalk,
    /// The terminations and reflections of the nets, with the rise time and the loss.
    Reflect,
    /// The eye diagram of the nets.
    Eye,
    /// All of them and the return path of the tracks.
    Check,
}

impl Analysis {
    /// The name of the command.
    pub fn name(self) -> &'static str {
        match self {
            Analysis::Crosstalk => "crosstalk",
            Analysis::Reflect => "reflect",
            Analysis::Eye => "eye",
            Analysis::Check => "check",
        }
    }
}

/// Works out an analysis of a board. With `net`, only that net is looked at (as the aggressor in
/// the crosstalk, as the line in the reflections and the eye, and in the return path). Nothing is
/// written.
pub fn analyse(
    board: &Board,
    settings: &SiSettings,
    job: Option<&StackupJob>,
    what: Analysis,
    net: Option<&str>,
) -> Result<SiReport, SiError> {
    settings
        .validate()
        .map_err(|name| SiError::new(SETTINGS_INVALID, name))?;
    if let Some(name) = net {
        if !board.nets.values().any(|n| n.name == name) {
            return Err(SiError::new(NET_NOT_FOUND, name));
        }
    }
    let mut env = Env::new(job, settings);
    let mut findings: Vec<Finding> = Vec::new();
    let wants = |a: Analysis| what == a || what == Analysis::Check;
    let mut nets = Vec::new();
    if what != Analysis::Crosstalk {
        nets = netdata::collect(board, settings, job, &mut env)?;
        nets.retain(|n| net.is_none_or(|name| n.name == name));
        if nets.len() > settings.max_nets {
            return Err(SiError::new(
                LIMIT_EXCEEDED,
                "the board has more nets with copper than max_nets",
            ));
        }
        if let (Some(name), true) = (net, nets.is_empty()) {
            findings.push(Finding::new(
                NET_NOT_FOUND,
                Severity::Warning,
                Some(name),
                "the net has no copper",
            ));
        }
    }
    let mut report = SiReport {
        command: what.name(),
        state: STATE_NOTE,
        crosstalk: None,
        nets: Vec::new(),
        eyes: Vec::new(),
        returns: None,
        findings: Vec::new(),
    };
    if wants(Analysis::Crosstalk) {
        let (found, more) = crosstalk::analyse(board, settings, &mut env, net)?;
        report.crosstalk = Some(found);
        findings.extend(more);
    }
    if wants(Analysis::Reflect) {
        let (found, more) = reflect::analyse(&nets, settings, &mut env);
        report.nets = found;
        findings.extend(more);
    }
    if wants(Analysis::Eye) {
        let (found, more) = eyenet::analyse(&nets, settings, &mut env);
        report.eyes = found;
        findings.extend(more);
    }
    if what == Analysis::Check {
        let mut more = Vec::new();
        let (tracks_checked, mut crossings) = returnpath::planes(board, settings, &mut more);
        let mut stubs = returnpath::stubs(board, settings, job, &mut more);
        if let Some(name) = net {
            crossings.retain(|c| c.net == name);
            stubs.retain(|s| s.net == name);
            more.retain(|f| f.code == NO_REFERENCE || f.subject.as_deref() == Some(name));
        }
        report.returns = Some(returnpath::ReturnReport {
            tracks_checked,
            crossings,
            stubs,
        });
        findings.extend(more);
    }
    findings.extend(env.findings());
    report.findings = findings;
    Ok(report)
}
