// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `thermal`: the temperature rise that the current of each net gives to its tracks and
//! vias, by the fit of IPC-2221 and by the convection model of EH3D-THERMAL-LAB.
//!
//! Exit codes: 0 done, 1 a track or a via is above the limit of the settings, 2 the arguments or
//! the input are unusable. Nothing is written unless `--out` names a new file; an existing file is
//! never replaced.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_stackup::heating::{
    heat_title, trace_heating, via_heating, HeatError, HeatSettings, LimitBasis, Placement,
    TraceHeating, HEAT_CURRENTS_INVALID, HEAT_INPUT_UNREADABLE, HEAT_LAYER_DEFAULTED,
    HEAT_NET_UNKNOWN, HEAT_NO_IPC_2152, HEAT_OPTION_INVALID, HEAT_OUTPUT_FAILED,
    HEAT_OUTSIDE_CHART, HEAT_OUT_OF_VALIDITY, HEAT_OVER_LIMIT,
};
use eh3d_phoenix_stackup::{Language, Stackup, StackupJob};
use serde::Serialize;
use serde_json::json;

use crate::check::read_text;
use crate::export::read_board_file;

/// The settings file that is read when `--settings` is not given and the file exists.
const DEFAULT_SETTINGS: &str = "config/heating.cfg";

pub(crate) const USAGE: &str = "\
phoenix thermal --currents FILE [--stackup FILE] [--net NAME] [--settings FILE]
                [--out FILE] [--json] [--lang CODE] <board>

  --currents    the current of each net in ampere, one line `net = amps` (a line that starts with
                # is a comment); only the nets of the file are looked at
  --stackup     a stack-up job file (the file of `phoenix stackup`) for the copper thickness of each
                layer and to tell an outer layer from an inner one (without it the defaults of the
                settings)
  --net         only this net
  --settings    ambient temperature, copper, convection, limit and the constants of the fit
                (default: config/heating.cfg when it exists)
  --out         write the JSON report to a new file (an existing file is never replaced)
  --lang CODE   the titles of the codes in en, es, de, fr, it, ja, zh or pt

For each net: the rise of each track (a layer and a width) and of its vias by the fit of IPC-2221
and by the convection model, and a warning when it is above the limit. The whole current of the net
is taken through every track and every via, so a net that splits its current is judged on the safe
side. A prototype of quasi-static estimates, never compared with a measurement. A dry run: nothing is
written without --out.
";

struct Options {
    json: bool,
    lang: Language,
    currents: Option<String>,
    stackup: Option<String>,
    settings: Option<String>,
    net: Option<String>,
    out: Option<String>,
    files: Vec<String>,
}

fn bad(what: impl Into<String>) -> HeatError {
    HeatError::new(HEAT_OPTION_INVALID, what)
}

fn parse_options(args: &[&str]) -> Result<Options, HeatError> {
    let mut options = Options {
        json: false,
        lang: Language::En,
        currents: None,
        stackup: None,
        settings: None,
        net: None,
        out: None,
        files: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, HeatError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => options.json = true,
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                options.lang =
                    Language::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--currents" => options.currents = Some(value("--currents needs a file")?),
            "--stackup" => options.stackup = Some(value("--stackup needs a file")?),
            "--settings" => options.settings = Some(value("--settings needs a file")?),
            "--net" => options.net = Some(value("--net needs a net name")?),
            "--out" => options.out = Some(value("--out needs a file")?),
            other if other.starts_with("--") => {
                return Err(bad(format!("unknown option {other}")));
            }
            other => options.files.push(other.to_owned()),
        }
    }
    Ok(options)
}

fn unreadable(what: String) -> HeatError {
    HeatError::new(HEAT_INPUT_UNREADABLE, what)
}

/// Read a file of currents: `net = amps` lines, `#` comments. A net twice, a current that is not a
/// number between 0 and 100000, a line without `=` or an empty file are refused.
pub(crate) fn parse_currents(
    text: &str,
    max_nets: usize,
) -> Result<BTreeMap<String, f64>, HeatError> {
    let mut currents: BTreeMap<String, f64> = BTreeMap::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let invalid =
            |why: &str| HeatError::new(HEAT_CURRENTS_INVALID, format!("line {line}: {why}"));
        let (net, value) = trimmed
            .split_once('=')
            .ok_or_else(|| invalid("not net = amps"))?;
        let net = net.trim();
        if net.is_empty() {
            return Err(invalid("the name of the net is empty"));
        }
        let amps: f64 = value
            .trim()
            .parse()
            .map_err(|_| invalid("the current is not a number"))?;
        if !amps.is_finite() || !(0.0..=100_000.0).contains(&amps) {
            return Err(invalid("the current must be between 0 and 100000 ampere"));
        }
        if currents.insert(net.to_owned(), amps).is_some() {
            return Err(invalid("the net is given twice"));
        }
        if currents.len() > max_nets {
            return Err(invalid("more nets than max_nets"));
        }
    }
    if currents.is_empty() {
        return Err(HeatError::new(HEAT_CURRENTS_INVALID, "the file has no net"));
    }
    Ok(currents)
}

fn load_settings(path: Option<&str>) -> Result<HeatSettings, HeatError> {
    let path = path.or_else(|| {
        Path::new(DEFAULT_SETTINGS)
            .is_file()
            .then_some(DEFAULT_SETTINGS)
    });
    match path {
        None => Ok(HeatSettings::default()),
        Some(path) => {
            let text = read_text(path).map_err(unreadable)?;
            HeatSettings::parse(&text)
        }
    }
}

/// Writes a new file. An existing file is never replaced.
fn write_new(path: &str, text: &str) -> Result<(), HeatError> {
    let failed = |e: std::io::Error| HeatError::new(HEAT_OUTPUT_FAILED, format!("{path}: {e}"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct Finding {
    code: &'static str,
    severity: &'static str,
    net: Option<String>,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct TrackHeat {
    layer: String,
    width_nm: i64,
    thickness_nm: i64,
    length_mm: f64,
    heating: TraceHeating,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct ViaHeat {
    count: usize,
    drill_nm: i64,
    heating: TraceHeating,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct NetHeat {
    net: String,
    current_a: f64,
    tracks: Vec<TrackHeat>,
    vias: Option<ViaHeat>,
    over_limit: bool,
}

struct Layers {
    /// The thickness in nanometres and the placement of each copper layer of the stack-up.
    known: BTreeMap<String, (i64, Placement)>,
}

fn layers_of(stackup: Option<&Stackup>) -> Layers {
    let mut known = BTreeMap::new();
    if let Some(stackup) = stackup {
        for (index, layer) in stackup.layers.iter().enumerate() {
            if !layer.is_copper() {
                continue;
            }
            let Ok(thickness) = layer.thickness_nm() else {
                continue;
            };
            let above = stackup.layers.iter().take(index).any(|l| l.is_copper());
            let below = stackup.layers.iter().skip(index + 1).any(|l| l.is_copper());
            let placement = if above && below {
                Placement::Internal
            } else {
                Placement::External
            };
            known.insert(layer.name().to_owned(), (thickness, placement));
        }
    }
    Layers { known }
}

fn chord(start: (i64, i64), end: (i64, i64)) -> f64 {
    let dx = (start.0 - end.0) as f64;
    let dy = (start.1 - end.1) as f64;
    dx.hypot(dy)
}

struct Report {
    nets: Vec<NetHeat>,
    findings: Vec<Finding>,
}

fn analyse(
    board: &Board,
    currents: &BTreeMap<String, f64>,
    layers: &Layers,
    stack_given: bool,
    only: Option<&str>,
    settings: &HeatSettings,
) -> Result<Report, HeatError> {
    let mut lengths: BTreeMap<(Id, Id, i64), f64> = BTreeMap::new();
    for track in board.tracks.values() {
        let mut sum = 0.0;
        for segment in &track.path {
            let (a, b) = (segment.start(), segment.end());
            sum += chord((a.x_nm.get(), a.y_nm.get()), (b.x_nm.get(), b.y_nm.get()));
        }
        *lengths
            .entry((track.net, track.layer, track.width.get()))
            .or_insert(0.0) += sum;
    }
    let mut vias: BTreeMap<Id, (usize, i64)> = BTreeMap::new();
    for via in board.vias.values() {
        let entry = vias.entry(via.net).or_insert((0, i64::MAX));
        entry.0 += 1;
        entry.1 = entry.1.min(via.drill.get());
    }
    let mut report = Report {
        nets: Vec::new(),
        findings: Vec::new(),
    };
    let mut defaulted: BTreeSet<String> = BTreeSet::new();
    for (name, amps) in currents {
        if only.is_some_and(|wanted| wanted != name) {
            continue;
        }
        let Some(id) = board
            .nets
            .iter()
            .find(|(_, net)| &net.name == name)
            .map(|(id, _)| *id)
        else {
            report.findings.push(Finding {
                code: HEAT_NET_UNKNOWN,
                severity: "warning",
                net: Some(name.clone()),
                detail: format!("the net {name} has no entry on the board"),
            });
            continue;
        };
        let mut tracks: Vec<TrackHeat> = Vec::new();
        for ((net, layer, width), length) in &lengths {
            if *net != id {
                continue;
            }
            let layer_name = board
                .layers
                .get(layer)
                .map_or_else(|| layer.to_string(), |l| l.name.clone());
            let (thickness_nm, placement) = match layers.known.get(&layer_name) {
                Some(found) => *found,
                None => {
                    if defaulted.insert(layer_name.clone()) {
                        let why = if stack_given {
                            "the stack-up has no such copper layer"
                        } else {
                            "no stack-up was given"
                        };
                        report.findings.push(Finding {
                            code: HEAT_LAYER_DEFAULTED,
                            severity: "warning",
                            net: None,
                            detail: format!("layer {layer_name}: {why}"),
                        });
                    }
                    (
                        (settings.default_copper_um * 1000.0).round() as i64,
                        settings.default_placement,
                    )
                }
            };
            let heating = trace_heating(*width, thickness_nm, *amps, placement, settings)?;
            tracks.push(TrackHeat {
                layer: layer_name,
                width_nm: *width,
                thickness_nm,
                length_mm: length * 1.0e-6,
                heating,
            });
        }
        tracks.sort_by_key(|t| (t.layer.clone(), t.width_nm));
        let via_heat = match vias.get(&id) {
            Some((count, drill)) => Some(ViaHeat {
                count: *count,
                drill_nm: *drill,
                heating: via_heating(*drill, *amps, settings)?,
            }),
            None => None,
        };
        for track in &tracks {
            let what = format!(
                "{name}: track on {} of {:.3} mm",
                track.layer,
                track.width_nm as f64 * 1.0e-6
            );
            judge(
                &what,
                name,
                &track.heating,
                *amps,
                settings,
                &mut report.findings,
            );
        }
        if let Some(via) = &via_heat {
            let what = format!(
                "{name}: via of {:.3} mm drill",
                via.drill_nm as f64 * 1.0e-6
            );
            judge(
                &what,
                name,
                &via.heating,
                *amps,
                settings,
                &mut report.findings,
            );
        }
        let over_limit = tracks.iter().any(|t| t.heating.over_limit)
            || via_heat.as_ref().is_some_and(|v| v.heating.over_limit);
        report.nets.push(NetHeat {
            net: name.clone(),
            current_a: *amps,
            tracks,
            vias: via_heat,
            over_limit,
        });
    }
    if let Some(wanted) = only {
        if !currents.contains_key(wanted) {
            return Err(bad(format!(
                "the net {wanted} is not in the file of currents"
            )));
        }
    }
    report.findings.push(Finding {
        code: HEAT_NO_IPC_2152,
        severity: "info",
        net: None,
        detail:
            "the charts of IPC-2152 and its modifiers for the planes and the board are not applied"
                .to_owned(),
    });
    Ok(report)
}

fn judge(
    what: &str,
    net: &str,
    heating: &TraceHeating,
    amps: f64,
    settings: &HeatSettings,
    findings: &mut Vec<Finding>,
) {
    if heating.over_limit {
        let judged = heating
            .judged_rise_k
            .map_or_else(|| "no number".to_owned(), |r| format!("{r:.1} K"));
        findings.push(Finding {
            code: HEAT_OVER_LIMIT,
            severity: "warning",
            net: Some(net.to_owned()),
            detail: format!(
                "{what}: {judged} against a limit of {} K at {amps} A (the fit allows {:.2} A at the limit)",
                settings.max_rise_k, heating.capacity_at_limit_a
            ),
        });
    }
    if !heating.ipc2221_in_range {
        findings.push(Finding {
            code: HEAT_OUTSIDE_CHART,
            severity: "info",
            net: Some(net.to_owned()),
            detail: format!(
                "{what}: {:.1} K by the fit is outside the range of the chart",
                heating.ipc2221_rise_k
            ),
        });
    }
    if let Some(note) = &heating.convection_note {
        if heating.convection_rise_k.is_none() && amps > 0.0 {
            findings.push(Finding {
                code: HEAT_OUT_OF_VALIDITY,
                severity: "info",
                net: Some(net.to_owned()),
                detail: format!("{what}: {note}"),
            });
        }
    }
}

fn text_of(report: &Report, settings: &HeatSettings, language: Language) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "heating of tracks and vias (prototype): ambient {} C, limit {} K by {}",
        settings.ambient_c,
        settings.max_rise_k,
        match settings.limit_basis {
            LimitBasis::Ipc2221 => "the fit of IPC-2221",
            LimitBasis::Convection => "the convection model",
            LimitBasis::Larger => "the larger of the two",
        }
    );
    for net in &report.nets {
        let _ = writeln!(out, "net {} {} A", net.net, net.current_a);
        for track in &net.tracks {
            let h = &track.heating;
            let convection = h
                .convection_rise_k
                .map_or_else(|| "no number".to_owned(), |r| format!("{r:.2} K"));
            let _ =
                writeln!(
                out,
                "  track on {} {:.3} mm by {:.1} um, {}, {:.2} mm: fit {:.2} K, convection {}{}",
                track.layer,
                track.width_nm as f64 * 1.0e-6,
                track.thickness_nm as f64 * 1.0e-3,
                h.placement.as_str(),
                track.length_mm,
                h.ipc2221_rise_k,
                convection,
                if h.over_limit { "  [above the limit]" } else { "" }
            );
        }
        if let Some(via) = &net.vias {
            let h = &via.heating;
            let _ = writeln!(
                out,
                "  {} via(s) of {:.3} mm drill: fit {:.2} K{}",
                via.count,
                via.drill_nm as f64 * 1.0e-6,
                h.ipc2221_rise_k,
                if h.over_limit {
                    "  [above the limit]"
                } else {
                    ""
                }
            );
        }
    }
    for finding in &report.findings {
        let head = heat_title(finding.code, language).unwrap_or("");
        let _ = writeln!(
            out,
            "{} {}: {head}: {}",
            finding.severity, finding.code, finding.detail
        );
    }
    out
}

fn json_of(report: &Report, settings: &HeatSettings) -> String {
    let value = json!({
        "format": "phoenix-thermal/1",
        "state": "prototype",
        "ambient_c": settings.ambient_c,
        "max_rise_k": settings.max_rise_k,
        "convection_w_m2k": settings.convection_w_m2k,
        "nets": report.nets,
        "findings": report.findings,
    });
    let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
    text.push('\n');
    text
}

fn run_thermal(options: &Options, out: &mut dyn Write) -> Result<i32, HeatError> {
    let [board_path] = options.files.as_slice() else {
        return Err(bad("give exactly one board file"));
    };
    let Some(currents_path) = &options.currents else {
        return Err(bad("--currents FILE is needed"));
    };
    let settings = load_settings(options.settings.as_deref())?;
    let currents_text = read_text(currents_path).map_err(unreadable)?;
    let currents = parse_currents(&currents_text, settings.max_nets)?;
    let job = match &options.stackup {
        Some(path) => {
            let text = read_text(path).map_err(unreadable)?;
            Some(StackupJob::parse(&text).map_err(|e| unreadable(format!("{path}: {e}")))?)
        }
        None => None,
    };
    let board = read_board_file(board_path).map_err(unreadable)?;
    let layers = layers_of(job.as_ref().map(|j| &j.stackup));
    let report = analyse(
        &board,
        &currents,
        &layers,
        job.is_some(),
        options.net.as_deref(),
        &settings,
    )?;
    let json = json_of(&report, &settings);
    if let Some(target) = &options.out {
        write_new(target, &json)?;
    }
    let shown = if options.json {
        json
    } else {
        let mut shown = text_of(&report, &settings, options.lang);
        if let Some(target) = &options.out {
            let _ = writeln!(shown, "written: {target}");
        }
        shown
    };
    let _ = out.write_all(shown.as_bytes());
    Ok(i32::from(report.nets.iter().any(|n| n.over_limit)))
}

pub(crate) fn thermal_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            let _ = writeln!(err, "error: {}", error.message(Language::En));
            return 2;
        }
    };
    match run_thermal(&options, out) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(err, "error: {}", error.message(options.lang));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_of_currents_is_read_by_net_and_refuses_what_it_cannot_use() {
        let text = "# currents\nVBUS = 3\n GND =3.5 \n\nSIG=0\n";
        let currents = parse_currents(text, 100).unwrap();
        assert_eq!(currents.len(), 3);
        assert_eq!(currents["VBUS"], 3.0);
        assert_eq!(currents["GND"], 3.5);
        assert_eq!(currents["SIG"], 0.0);
        for bad in [
            "",
            "# only a comment\n",
            "VBUS",
            "= 3",
            "VBUS = three",
            "VBUS = -1",
            "VBUS = 1e9",
            "VBUS = nan",
            "VBUS = 1\nVBUS = 2",
        ] {
            let error = parse_currents(bad, 100).unwrap_err();
            assert_eq!(error.code, HEAT_CURRENTS_INVALID, "{bad:?}");
        }
        let error = parse_currents("A = 1\nB = 1\n", 1).unwrap_err();
        assert_eq!(error.code, HEAT_CURRENTS_INVALID);
    }

    #[test]
    fn the_layers_of_a_stackup_are_outer_or_inner() {
        let stackup: Stackup = serde_json::from_str(
            r#"{"name": "four", "layers": [
              {"kind": "copper", "name": "top", "thickness": {"um": 35.0}},
              {"kind": "core", "name": "p1", "thickness_nm": 200000, "material": "m"},
              {"kind": "copper", "name": "in1", "thickness": {"um": 17.5}},
              {"kind": "core", "name": "c", "thickness_nm": 800000, "material": "m"},
              {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}]}"#,
        )
        .unwrap();
        let layers = layers_of(Some(&stackup));
        assert_eq!(layers.known["top"], (35_000, Placement::External));
        assert_eq!(layers.known["in1"], (17_500, Placement::Internal));
        assert_eq!(layers.known["bottom"], (35_000, Placement::External));
        assert!(layers_of(None).known.is_empty());
    }

    #[test]
    fn the_options_are_read_and_a_wrong_one_is_refused() {
        let options = parse_options(&[
            "--currents",
            "c.cfg",
            "--net",
            "VBUS",
            "--lang",
            "pt",
            "--json",
            "board.json",
        ])
        .unwrap();
        assert_eq!(options.currents.as_deref(), Some("c.cfg"));
        assert_eq!(options.net.as_deref(), Some("VBUS"));
        assert_eq!(options.lang, Language::Pt);
        assert!(options.json);
        assert_eq!(options.files, vec!["board.json".to_owned()]);
        for bad in [&["--lang", "xx"][..], &["--currents"], &["--nothing"]] {
            assert_eq!(parse_options(bad).unwrap_err().code, HEAT_OPTION_INVALID);
        }
    }
}
