// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks of the differential pairs of a board: the gap between the two tracks, the symmetry of
//! the coupling, the stretch that runs uncoupled, the difference of the lengths, the vias that have
//! no partner, the width of the tracks and, when a stack-up is given and the pair has a target, the
//! differential impedance. Every finding has a code `PHOENIX-PAIR-<NNN>`. The report is the same
//! every time: the pairs come in the order of their names, the findings in the order of the checks
//! and every number is an integer.
//!
//! The lengths, the coupled stretches and the gaps are measured by `eh3d-phoenix-lengths`, once for
//! each net of the pair so that the two sides can be compared.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_lengths::{
    analyse, millimetres, LengthFile, LengthSettings, PairResult, PairSpec,
    FILE_FORMAT as LENGTH_FORMAT,
};
use eh3d_phoenix_rules::{Lang, Severity};
use eh3d_phoenix_stackup::{analyze, LineKind, LineSpec, StackupJob, StackupSettings};
use serde_json::{json, Value};

use crate::codes::{BoardEditError, OPERATION_REFUSED};
use crate::pair_codes::{
    title, ASYMMETRIC, GAP_OUT, IMPEDANCE_OFF, LENGTH_MISMATCH, NOT_COUPLED, UNCOUPLED_LONG,
    VIAS_UNPAIRED, WIDTH_WRONG,
};
use crate::pairs::{resolve, PairConfig, PairSettings, ResolvedPair};

/// The name of the format of the JSON report.
pub const REPORT_FORMAT: &str = "phoenix-pairs/1";

/// One finding of the checks of a pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairFinding {
    /// The stable code `PHOENIX-PAIR-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The name of the pair.
    pub pair: String,
    /// What is wrong, in English.
    pub detail: String,
}

/// What was measured on one pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairMeasure {
    /// The name of the pair.
    pub name: String,
    /// The name of the positive net.
    pub positive: String,
    /// The name of the negative net.
    pub negative: String,
    /// The copper length of the positive net, in nanometres.
    pub positive_nm: i64,
    /// The copper length of the negative net, in nanometres.
    pub negative_nm: i64,
    /// The difference of the two lengths, in nanometres.
    pub skew_nm: i64,
    /// The length of the positive net that runs coupled, in nanometres.
    pub coupled_positive_nm: i64,
    /// The length of the negative net that runs coupled, in nanometres.
    pub coupled_negative_nm: i64,
    /// The larger of the two uncoupled lengths, in nanometres.
    pub uncoupled_nm: i64,
    /// The smallest gap between the edges of the coupled pieces, in nanometres.
    pub min_gap_nm: Option<i64>,
    /// The largest gap between the edges of the coupled pieces, in nanometres.
    pub max_gap_nm: Option<i64>,
    /// The vias of the positive net.
    pub vias_positive: usize,
    /// The vias of the negative net.
    pub vias_negative: usize,
    /// The vias of both nets that have no partner on the other net.
    pub unpaired_vias: usize,
    /// The differential impedance worked out from the stack-up, in milli-ohm.
    pub impedance_mohm: Option<i64>,
    /// True when no finding of the pair is an error.
    pub ok: bool,
}

/// The report of the checks of the pairs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PairReport {
    /// The pairs that could be measured, in the order of their names.
    pub pairs: Vec<PairMeasure>,
    /// The findings.
    pub findings: Vec<PairFinding>,
}

/// The number of vias of each net and of the vias that have no partner: a via of one net is paired
/// with an unused via of the other net on the same layers within `range_nm`, the nearest first.
fn via_pairing(board: &Board, positive: Id, negative: Id, range_nm: i64) -> (usize, usize, usize) {
    let of = |net: Id| -> Vec<_> { board.vias.values().filter(|v| v.net == net).collect() };
    let (p, n) = (of(positive), of(negative));
    let mut used = vec![false; n.len()];
    let mut matched = 0usize;
    let limit = i128::from(range_nm) * i128::from(range_nm);
    for a in &p {
        let mut best: Option<(i128, usize)> = None;
        for (k, b) in n.iter().enumerate() {
            if used[k] || a.from_layer != b.from_layer || a.to_layer != b.to_layer {
                continue;
            }
            let dx = i128::from(a.at.x_nm.get()) - i128::from(b.at.x_nm.get());
            let dy = i128::from(a.at.y_nm.get()) - i128::from(b.at.y_nm.get());
            let d = dx * dx + dy * dy;
            if d <= limit && best.is_none_or(|(least, _)| d < least) {
                best = Some((d, k));
            }
        }
        if let Some((_, k)) = best {
            used[k] = true;
            matched += 1;
        }
    }
    (p.len(), n.len(), p.len() + n.len() - 2 * matched)
}

/// The number of pieces of the tracks of a net that are not of the width asked for.
fn pieces_of_wrong_width(board: &Board, net: Id, want_nm: i64, tolerance_nm: i64) -> usize {
    board
        .tracks
        .values()
        .filter(|t| t.net == net && (t.width.get() - want_nm).abs() > tolerance_nm)
        .map(|t| t.path.len())
        .sum()
}

/// The differential impedance of a pair, in ohm, worked out from the stack-up for the layer where
/// the positive net has most copper, or why it could not be worked out.
fn differential_impedance_ohm(
    board: &Board,
    pair: &ResolvedPair,
    job: &StackupJob,
    gap_nm: i64,
    settings: &PairSettings,
) -> Result<f64, String> {
    let mut by_layer: BTreeMap<Id, i64> = BTreeMap::new();
    let mut first_width: BTreeMap<Id, i64> = BTreeMap::new();
    for track in board.tracks.values().filter(|t| t.net == pair.positive) {
        let run: i64 = track
            .path
            .iter()
            .map(|s| {
                let (a, b) = (s.start(), s.end());
                (b.x_nm.get() - a.x_nm.get())
                    .abs()
                    .max((b.y_nm.get() - a.y_nm.get()).abs())
            })
            .sum();
        *by_layer.entry(track.layer).or_insert(0) += run;
        first_width.entry(track.layer).or_insert(track.width.get());
    }
    let mut chosen: Option<(Id, i64)> = None;
    for (layer, run) in &by_layer {
        if chosen.is_none_or(|(_, most)| *run > most) {
            chosen = Some((*layer, *run));
        }
    }
    let (layer, _) = chosen.ok_or_else(|| "the positive net has no copper".to_owned())?;
    let width_nm = first_width.get(&layer).copied().unwrap_or(pair.width_nm);
    let name = board
        .layers
        .get(&layer)
        .map(|l| l.name.clone())
        .ok_or_else(|| "the layer of the pair is not on the board".to_owned())?;
    let stackup = &job.stackup;
    let at = stackup
        .find(&name)
        .ok_or_else(|| format!("the stack-up has no layer named {name}"))?;
    let coppers: Vec<usize> = stackup
        .layers
        .iter()
        .enumerate()
        .filter(|(_, l)| l.is_copper())
        .map(|(i, _)| i)
        .collect();
    let outer = coppers.first() == Some(&at) || coppers.last() == Some(&at);
    let spec = LineSpec {
        name: pair.name.clone(),
        kind: if outer {
            LineKind::DifferentialMicrostrip
        } else {
            LineKind::DifferentialStripline
        },
        layer: name,
        width_nm: Some(width_nm),
        target_ohm: None,
        gap_nm: Some(gap_nm),
    };
    let frequency_ghz = settings.impedance_frequency_mhz as f64 / 1000.0;
    analyze(
        stackup,
        &job.materials,
        &spec,
        width_nm,
        frequency_ghz,
        &StackupSettings::default(),
    )
    .map(|r| r.impedance_ohm)
    .map_err(|e| e.to_string())
}

fn length_spec(pair: &ResolvedPair, settings: &PairSettings, swapped: bool) -> PairSpec {
    let (positive, negative) = if swapped {
        (&pair.negative_name, &pair.positive_name)
    } else {
        (&pair.positive_name, &pair.negative_name)
    };
    PairSpec {
        name: pair.name.clone(),
        positive: positive.clone(),
        negative: negative.clone(),
        max_skew_nm: Some(pair.length_tolerance_nm),
        gap_nm: Some(pair.gap_nm),
        gap_tolerance_nm: Some(settings.gap_tolerance_nm),
        width_nm: Some(pair.width_nm),
        width_tolerance_nm: Some(settings.width_tolerance_nm),
        max_uncoupled_nm: Some(pair.max_uncoupled_nm),
        coupling_range_nm: Some(settings.coupling_range_nm),
    }
}

/// Checks every pair of the configuration on a board. A pair whose nets the board does not have is
/// a finding (`PHOENIX-PAIR-002`) and is left out of the measures. The stack-up is only needed for
/// the impedance of a pair that has a target.
pub fn check_pairs(
    board: &Board,
    config: &PairConfig,
    job: Option<&StackupJob>,
) -> Result<PairReport, BoardEditError> {
    let settings = &config.settings;
    let mut report = PairReport::default();
    let mut resolved: Vec<ResolvedPair> = Vec::new();
    for def in &config.pairs {
        match resolve(board, def, settings) {
            Ok(pair) => resolved.push(pair),
            Err(e) => report.findings.push(PairFinding {
                code: e.code,
                severity: Severity::Error,
                pair: def.name.clone(),
                detail: e.detail,
            }),
        }
    }
    let mut specs: Vec<PairSpec> = Vec::with_capacity(resolved.len() * 2);
    for pair in &resolved {
        specs.push(length_spec(pair, settings, false));
        specs.push(length_spec(pair, settings, true));
    }
    let file = LengthFile {
        format: LENGTH_FORMAT.to_owned(),
        groups: Vec::new(),
        pairs: specs,
    };
    let lengths = analyse(board, &LengthSettings::default(), None, Some(&file))
        .map_err(|e| BoardEditError::new(OPERATION_REFUSED, e.message(Lang::En)))?;
    for (pair, couple) in resolved.iter().zip(lengths.pairs.chunks(2)) {
        let [p, n] = couple else {
            continue;
        };
        judge(board, config, job, pair, (p, n), &mut report);
    }
    Ok(report)
}

/// Judges one pair from the two measures of the lengths crate (the pair as it is written and the
/// pair with its nets swapped).
fn judge(
    board: &Board,
    config: &PairConfig,
    job: Option<&StackupJob>,
    pair: &ResolvedPair,
    measures: (&PairResult, &PairResult),
    report: &mut PairReport,
) {
    let settings = &config.settings;
    let (p, n) = measures;
    let mut found: Vec<PairFinding> = Vec::new();
    let mut add = |code: &'static str, severity: Severity, detail: String| {
        found.push(PairFinding {
            code,
            severity,
            pair: pair.name.clone(),
            detail,
        });
    };
    let coupled = p.coupled_nm > 0 || n.coupled_nm > 0;
    let gaps: Vec<i64> = [p.min_gap_nm, n.min_gap_nm].into_iter().flatten().collect();
    let widest: Vec<i64> = [p.max_gap_nm, n.max_gap_nm].into_iter().flatten().collect();
    let min_gap = gaps.iter().copied().min();
    let max_gap = widest.iter().copied().max();
    if !coupled {
        add(
            NOT_COUPLED,
            Severity::Warning,
            format!(
                "no piece of the pair runs within {} nm of the other net",
                settings.coupling_range_nm
            ),
        );
    } else {
        let tol = settings.gap_tolerance_nm;
        if let Some(lo) = min_gap.filter(|lo| *lo < pair.gap_nm - tol) {
            add(
                GAP_OUT,
                Severity::Error,
                format!(
                    "the gap is {lo} nm at its smallest, {} nm minus {tol} nm was asked",
                    pair.gap_nm
                ),
            );
        }
        if let Some(hi) = max_gap.filter(|hi| *hi > pair.gap_nm + tol) {
            add(
                GAP_OUT,
                Severity::Error,
                format!(
                    "the gap is {hi} nm at its largest, {} nm plus {tol} nm was asked",
                    pair.gap_nm
                ),
            );
        }
        let off = (p.coupled_nm - n.coupled_nm).abs();
        if off > settings.symmetry_tolerance_nm {
            add(
                ASYMMETRIC,
                Severity::Error,
                format!(
                    "{} runs coupled for {} nm and {} for {} nm, at most {} nm of difference is allowed",
                    pair.positive_name,
                    p.coupled_nm,
                    pair.negative_name,
                    n.coupled_nm,
                    settings.symmetry_tolerance_nm
                ),
            );
        }
    }
    let uncoupled = p.uncoupled_nm.max(n.uncoupled_nm);
    if uncoupled > pair.max_uncoupled_nm {
        add(
            UNCOUPLED_LONG,
            Severity::Error,
            format!(
                "{uncoupled} nm run uncoupled, at most {} nm is allowed",
                pair.max_uncoupled_nm
            ),
        );
    }
    if p.skew_nm > pair.length_tolerance_nm {
        add(
            LENGTH_MISMATCH,
            Severity::Error,
            format!(
                "the nets differ by {} nm, at most {} nm is allowed",
                p.skew_nm, pair.length_tolerance_nm
            ),
        );
    }
    let (vias_positive, vias_negative, unpaired) = via_pairing(
        board,
        pair.positive,
        pair.negative,
        settings.via_pair_range_nm,
    );
    if unpaired > 0 {
        add(
            VIAS_UNPAIRED,
            Severity::Error,
            format!(
                "{unpaired} vias have no partner on the other net ({vias_positive} and {vias_negative} vias)"
            ),
        );
    }
    let wrong = pieces_of_wrong_width(
        board,
        pair.positive,
        pair.width_nm,
        settings.width_tolerance_nm,
    ) + pieces_of_wrong_width(
        board,
        pair.negative,
        pair.width_nm,
        settings.width_tolerance_nm,
    );
    if wrong > 0 {
        add(
            WIDTH_WRONG,
            Severity::Error,
            format!(
                "{wrong} pieces are not {} nm wide (tolerance {} nm)",
                pair.width_nm, settings.width_tolerance_nm
            ),
        );
    }
    let mut impedance_mohm = None;
    if let (Some(target), Some(job)) = (pair.target_diff_ohm, job) {
        let gap = match (min_gap, max_gap) {
            (Some(lo), Some(hi)) => (lo + hi) / 2,
            _ => pair.gap_nm,
        };
        match differential_impedance_ohm(board, pair, job, gap, settings) {
            Ok(ohm) => {
                impedance_mohm = Some((ohm * 1000.0).round() as i64);
                let allowed = target * settings.impedance_tolerance_permille as f64 / 1000.0;
                if (ohm - target).abs() > allowed {
                    add(
                        IMPEDANCE_OFF,
                        Severity::Warning,
                        format!(
                            "{ohm:.2} ohm worked out from the stack-up, the target is {target:.2} ohm (tolerance {allowed:.2} ohm)"
                        ),
                    );
                }
            }
            Err(why) => add(
                IMPEDANCE_OFF,
                Severity::Warning,
                format!("the impedance could not be worked out: {why}"),
            ),
        }
    }
    let ok = !found.iter().any(|f| f.severity == Severity::Error);
    report.pairs.push(PairMeasure {
        name: pair.name.clone(),
        positive: pair.positive_name.clone(),
        negative: pair.negative_name.clone(),
        positive_nm: p.positive_nm,
        negative_nm: p.negative_nm,
        skew_nm: p.skew_nm,
        coupled_positive_nm: p.coupled_nm,
        coupled_negative_nm: n.coupled_nm,
        uncoupled_nm: uncoupled,
        min_gap_nm: min_gap,
        max_gap_nm: max_gap,
        vias_positive,
        vias_negative,
        unpaired_vias: unpaired,
        impedance_mohm,
        ok,
    });
    report.findings.extend(found);
}

#[derive(Clone, Copy)]
enum Label {
    Pairs,
    Errors,
    Warnings,
    Pair,
    Fine,
    NoPairs,
}

/// The words of the text report in the eight languages.
fn label(which: Label, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    let row: [&str; 8] = match which {
        Label::Pairs => [
            "pairs",
            "pares",
            "Paare",
            "paires",
            "coppie",
            "ペア",
            "差分对",
            "pares",
        ],
        Label::Errors => [
            "errors",
            "errores",
            "Fehler",
            "erreurs",
            "errori",
            "エラー",
            "错误",
            "erros",
        ],
        Label::Warnings => [
            "warnings",
            "avisos",
            "Warnungen",
            "avertissements",
            "avvisi",
            "警告",
            "警告",
            "avisos",
        ],
        Label::Pair => [
            "pair",
            "par",
            "Paar",
            "paire",
            "coppia",
            "ペア",
            "差分对",
            "par",
        ],
        Label::Fine => [
            "ok",
            "correcto",
            "in Ordnung",
            "correct",
            "corretto",
            "正常",
            "通过",
            "correto",
        ],
        Label::NoPairs => [
            "no pairs are defined",
            "no hay pares definidos",
            "keine Paare definiert",
            "aucune paire définie",
            "nessuna coppia definita",
            "ペアが定義されていません",
            "未定义差分对",
            "não há pares definidos",
        ],
    };
    row[column]
}

fn mm_range(lo: Option<i64>, hi: Option<i64>) -> String {
    match (lo, hi) {
        (Some(lo), Some(hi)) => format!("{}..{}", millimetres(lo), millimetres(hi)),
        _ => "-".to_owned(),
    }
}

impl PairReport {
    /// The number of findings that are errors and the number that are warnings.
    pub fn counts(&self) -> (usize, usize) {
        let of = |s: Severity| self.findings.iter().filter(|f| f.severity == s).count();
        (of(Severity::Error), of(Severity::Warning))
    }

    /// True when a finding is an error.
    pub fn has_errors(&self) -> bool {
        self.counts().0 > 0
    }

    /// The report as text, with the titles of the codes in `lang`. The lengths are in millimetres.
    pub fn to_text(&self, lang: Lang) -> String {
        let (errors, warnings) = self.counts();
        let mut out = format!(
            "{}: {}, {}: {errors}, {}: {warnings}\n",
            label(Label::Pairs, lang),
            self.pairs.len(),
            label(Label::Errors, lang),
            label(Label::Warnings, lang)
        );
        if self.pairs.is_empty() && self.findings.is_empty() {
            out.push_str(label(Label::NoPairs, lang));
            out.push('\n');
        }
        for m in &self.pairs {
            out.push_str(&format!(
                "{} {} ({} / {}): {} mm / {} mm, skew {} mm, coupled {} mm / {} mm, uncoupled {} mm, gap {} mm, vias {}/{} ({} unpaired){} {}\n",
                label(Label::Pair, lang),
                m.name,
                m.positive,
                m.negative,
                millimetres(m.positive_nm),
                millimetres(m.negative_nm),
                millimetres(m.skew_nm),
                millimetres(m.coupled_positive_nm),
                millimetres(m.coupled_negative_nm),
                millimetres(m.uncoupled_nm),
                mm_range(m.min_gap_nm, m.max_gap_nm),
                m.vias_positive,
                m.vias_negative,
                m.unpaired_vias,
                m.impedance_mohm
                    .map_or(String::new(), |z| format!(", {}.{:03} ohm", z / 1000, z % 1000)),
                if m.ok { label(Label::Fine, lang) } else { "!" },
            ));
        }
        for f in &self.findings {
            out.push_str(&format!(
                "  [{}] {} {}: {} | {}\n",
                f.severity.as_str(),
                f.code,
                title(f.code, lang),
                f.pair,
                f.detail
            ));
        }
        out
    }

    /// The report as a JSON value.
    pub fn to_value(&self) -> Value {
        let (errors, warnings) = self.counts();
        let pairs: Vec<Value> = self
            .pairs
            .iter()
            .map(|m| {
                json!({
                    "name": m.name,
                    "positive": m.positive,
                    "negative": m.negative,
                    "positive_nm": m.positive_nm,
                    "negative_nm": m.negative_nm,
                    "skew_nm": m.skew_nm,
                    "coupled_positive_nm": m.coupled_positive_nm,
                    "coupled_negative_nm": m.coupled_negative_nm,
                    "uncoupled_nm": m.uncoupled_nm,
                    "min_gap_nm": m.min_gap_nm,
                    "max_gap_nm": m.max_gap_nm,
                    "vias": [m.vias_positive, m.vias_negative],
                    "unpaired_vias": m.unpaired_vias,
                    "impedance_mohm": m.impedance_mohm,
                    "ok": m.ok,
                })
            })
            .collect();
        let findings: Vec<Value> = self
            .findings
            .iter()
            .map(|f| {
                json!({
                    "code": f.code,
                    "severity": f.severity.as_str(),
                    "title": title(f.code, Lang::En),
                    "pair": f.pair,
                    "detail": f.detail,
                })
            })
            .collect();
        json!({
            "schema": REPORT_FORMAT,
            "pairs": pairs,
            "findings": findings,
            "errors": errors,
            "warnings": warnings,
        })
    }

    /// The report as JSON text, the same every time.
    pub fn to_json(&self) -> String {
        self.to_value().to_string()
    }
}
