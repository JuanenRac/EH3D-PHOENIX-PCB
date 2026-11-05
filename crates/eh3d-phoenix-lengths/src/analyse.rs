// SPDX-License-Identifier: GPL-3.0-or-later
//! Groups of nets that must match in length, differential pairs, and the report that judges them.
//!
//! A **group** names nets and a tolerance: the longest and the shortest net may differ by at most
//! the tolerance; with a target length every net must be within the tolerance of the target
//! instead. A **pair** names two nets and says how much their lengths may differ, how wide their
//! tracks must be, how far apart they must run and how much of the pair may run uncoupled. The
//! separation is the gap between the edges of the two tracks, measured on every straight piece of
//! the positive net against the pieces of the negative net on the same layer; a piece within the
//! coupling range of the other net counts as coupled. Arcs are counted in the lengths but not in
//! the separation, and a piece of arc is never coupled.
//!
//! The report is the same every time: the nets come in the order of their names, the groups and
//! pairs in the order of the file, and every number is an integer.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{Board, Id, Segment};
use eh3d_phoenix_stackup::StackupJob;
use serde::{Deserialize, Serialize};

use crate::codes::{
    Finding, LengthError, Severity, FILE_INVALID, GROUP_MISMATCH, GROUP_TARGET, LIMIT_EXCEEDED,
    NET_NOT_FOUND, NET_NO_COPPER, PAIR_GAP_LARGE, PAIR_GAP_SMALL, PAIR_NOT_COUPLED, PAIR_SKEW,
    PAIR_UNCOUPLED, PAIR_WIDTH,
};
use crate::geom::{seg_dist, seg_len, P};
use crate::measure::{measure_nets, NetLength};
use crate::settings::LengthSettings;

/// The name of the format of the file of groups and pairs.
pub const FILE_FORMAT: &str = "phoenix-lengths/1";

/// A group of nets that must match in length.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupSpec {
    /// The name of the group.
    pub name: String,
    /// The names of the nets.
    pub nets: Vec<String>,
    /// How much the lengths may differ, in nanometres; the default of the settings when missing.
    #[serde(default)]
    pub tolerance_nm: Option<i64>,
    /// The length every net must have, in nanometres, within the tolerance.
    #[serde(default)]
    pub target_nm: Option<i64>,
}

/// A differential pair.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairSpec {
    /// The name of the pair.
    pub name: String,
    /// The name of the positive net.
    pub positive: String,
    /// The name of the negative net.
    pub negative: String,
    /// The most the two lengths may differ, in nanometres; the default tolerance of the
    /// settings when missing.
    #[serde(default)]
    pub max_skew_nm: Option<i64>,
    /// The gap asked for between the edges of the two tracks, in nanometres.
    #[serde(default)]
    pub gap_nm: Option<i64>,
    /// How far the gap may be from the one asked for; the settings when missing.
    #[serde(default)]
    pub gap_tolerance_nm: Option<i64>,
    /// The width asked for in the tracks of both nets, in nanometres.
    #[serde(default)]
    pub width_nm: Option<i64>,
    /// How far a width may be from the one asked for; the settings when missing.
    #[serde(default)]
    pub width_tolerance_nm: Option<i64>,
    /// The most track of the positive net that may run uncoupled, in nanometres.
    #[serde(default)]
    pub max_uncoupled_nm: Option<i64>,
    /// The widest gap at which two pieces count as coupled; the settings when missing.
    #[serde(default)]
    pub coupling_range_nm: Option<i64>,
}

/// The file of groups and pairs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LengthFile {
    /// Must be [`FILE_FORMAT`].
    pub format: String,
    /// The groups.
    #[serde(default)]
    pub groups: Vec<GroupSpec>,
    /// The pairs.
    #[serde(default)]
    pub pairs: Vec<PairSpec>,
}

impl LengthFile {
    /// Reads and checks the text of a file of groups and pairs.
    pub fn from_json(text: &str) -> Result<LengthFile, LengthError> {
        let file: LengthFile = serde_json::from_str(text)
            .map_err(|e| LengthError::new(FILE_INVALID, e.to_string()))?;
        let bad = |what: String| Err(LengthError::new(FILE_INVALID, what));
        if file.format != FILE_FORMAT {
            return bad(format!("the format must be {FILE_FORMAT}"));
        }
        let mut names: BTreeSet<&str> = BTreeSet::new();
        for g in &file.groups {
            if g.name.trim().is_empty() || !names.insert(g.name.as_str()) {
                return bad(format!(
                    "the group name {:?} is empty or used twice",
                    g.name
                ));
            }
            let needed = if g.target_nm.is_some() { 1 } else { 2 };
            if g.nets.len() < needed {
                return bad(format!("the group {} needs at least {needed} nets", g.name));
            }
            if [g.tolerance_nm, g.target_nm]
                .iter()
                .any(|v| v.is_some_and(|x| x < 0))
            {
                return bad(format!("the group {} has a negative length", g.name));
            }
        }
        let mut pair_names: BTreeSet<&str> = BTreeSet::new();
        for p in &file.pairs {
            if p.name.trim().is_empty() || !pair_names.insert(p.name.as_str()) {
                return bad(format!("the pair name {:?} is empty or used twice", p.name));
            }
            if p.positive == p.negative {
                return bad(format!("the pair {} has the same net twice", p.name));
            }
            let values = [
                p.max_skew_nm,
                p.gap_nm,
                p.gap_tolerance_nm,
                p.width_nm,
                p.width_tolerance_nm,
                p.max_uncoupled_nm,
                p.coupling_range_nm,
            ];
            if values.iter().any(|v| v.is_some_and(|x| x < 0)) {
                return bad(format!("the pair {} has a negative length", p.name));
            }
        }
        Ok(file)
    }
}

/// A net of a group with its length and delay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GroupNet {
    /// The name of the net.
    pub name: String,
    /// Its length, in nanometres.
    pub length_nm: i64,
    /// Its delay, in femtoseconds.
    pub delay_fs: i64,
}

/// The judgement of a group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GroupResult {
    /// The name of the group.
    pub name: String,
    /// Its nets, in the order of the file.
    pub nets: Vec<GroupNet>,
    /// The shortest length, in nanometres.
    pub min_nm: i64,
    /// The longest length, in nanometres.
    pub max_nm: i64,
    /// The longest minus the shortest, in nanometres.
    pub spread_nm: i64,
    /// The tolerance that was applied, in nanometres.
    pub tolerance_nm: i64,
    /// The target length, when the group has one.
    pub target_nm: Option<i64>,
    /// True when the group is within its tolerance.
    pub ok: bool,
}

/// The judgement of a pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PairResult {
    /// The name of the pair.
    pub name: String,
    /// The positive net.
    pub positive: String,
    /// The negative net.
    pub negative: String,
    /// The length of the positive net, in nanometres.
    pub positive_nm: i64,
    /// The length of the negative net, in nanometres.
    pub negative_nm: i64,
    /// The difference of the lengths, in nanometres.
    pub skew_nm: i64,
    /// The difference that was allowed, in nanometres.
    pub max_skew_nm: i64,
    /// The difference of the delays, in femtoseconds.
    pub delay_skew_fs: i64,
    /// The length of the positive net that runs within the coupling range of the other.
    pub coupled_nm: i64,
    /// The rest of the length of the positive net.
    pub uncoupled_nm: i64,
    /// The smallest gap between the edges of the coupled pieces, in nanometres.
    pub min_gap_nm: Option<i64>,
    /// The largest of the gaps of the coupled pieces, in nanometres.
    pub max_gap_nm: Option<i64>,
    /// True when no rule of the pair is broken.
    pub ok: bool,
}

/// The whole report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LengthReport {
    /// Every net that has copper, in the order of the names.
    pub nets: Vec<NetLength>,
    /// The groups of the file.
    pub groups: Vec<GroupResult>,
    /// The pairs of the file.
    pub pairs: Vec<PairResult>,
    /// What to know about the report.
    pub findings: Vec<Finding>,
}

impl LengthReport {
    /// True when a finding is a proven break of a rule.
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }
}

/// A straight or curved piece of a track.
struct Piece {
    layer: Id,
    width: i64,
    a: P,
    b: P,
    arc: bool,
}

fn pieces_of(board: &Board, net: Id) -> Vec<Piece> {
    let mut out = Vec::new();
    for track in board.tracks.values().filter(|t| t.net == net) {
        for segment in &track.path {
            let (a, b) = (segment.start(), segment.end());
            out.push(Piece {
                layer: track.layer,
                width: track.width.get(),
                a: (a.x_nm.get(), a.y_nm.get()),
                b: (b.x_nm.get(), b.y_nm.get()),
                arc: matches!(segment, Segment::Arc { .. }),
            });
        }
    }
    out
}

/// The coupled length of the pieces of `p` against the pieces of `n`, and the smallest and the
/// largest gap between the edges of the pieces that are coupled.
fn coupling(p: &[Piece], n: &[Piece], range_nm: i64) -> (i64, Option<i64>, Option<i64>) {
    let mut coupled: i64 = 0;
    let mut min_gap: Option<i64> = None;
    let mut max_gap: Option<i64> = None;
    for pp in p.iter().filter(|x| !x.arc) {
        let mut best: Option<f64> = None;
        for nn in n.iter().filter(|x| !x.arc && x.layer == pp.layer) {
            let gap = seg_dist(pp.a, pp.b, nn.a, nn.b) - (pp.width + nn.width) as f64 / 2.0;
            best = Some(best.map_or(gap, |b| b.min(gap)));
        }
        let Some(gap) = best else {
            continue;
        };
        let gap = gap.max(0.0);
        if gap <= range_nm as f64 {
            coupled = coupled.saturating_add(seg_len(pp.a, pp.b));
            let g = gap.round() as i64;
            min_gap = Some(min_gap.map_or(g, |m| m.min(g)));
            max_gap = Some(max_gap.map_or(g, |m| m.max(g)));
        }
    }
    (coupled, min_gap, max_gap)
}

/// What the names of the nets stand for.
struct Resolver<'a> {
    ids: BTreeMap<&'a str, Id>,
    by_net: BTreeMap<Id, &'a NetLength>,
    told: BTreeSet<String>,
}

impl<'a> Resolver<'a> {
    fn new(board: &'a Board, nets: &'a [NetLength]) -> Resolver<'a> {
        let mut ids: BTreeMap<&str, Id> = BTreeMap::new();
        for (id, net) in &board.nets {
            ids.entry(net.name.as_str()).or_insert(*id);
        }
        Resolver {
            ids,
            by_net: nets.iter().map(|n| (n.net, n)).collect(),
            told: BTreeSet::new(),
        }
    }

    /// The id, the length and the delay of a net by its name; `None` when the board has no such
    /// net. A net without copper has length zero.
    fn get(&mut self, name: &str, findings: &mut Vec<Finding>) -> Option<(Id, i64, i64)> {
        let id = *self.ids.get(name)?;
        match self.by_net.get(&id) {
            Some(n) => Some((id, n.total_nm, n.delay_fs)),
            None => {
                if self.told.insert(name.to_owned()) {
                    findings.push(Finding::new(
                        NET_NO_COPPER,
                        Severity::Warning,
                        Some(name),
                        "its length is zero",
                    ));
                }
                Some((id, 0, 0))
            }
        }
    }
}

fn judge_group(
    g: &GroupSpec,
    settings: &LengthSettings,
    resolver: &mut Resolver<'_>,
    findings: &mut Vec<Finding>,
) -> Option<GroupResult> {
    let mut nets: Vec<GroupNet> = Vec::new();
    let mut missing = false;
    for name in &g.nets {
        match resolver.get(name, findings) {
            Some((_, length_nm, delay_fs)) => nets.push(GroupNet {
                name: name.clone(),
                length_nm,
                delay_fs,
            }),
            None => {
                missing = true;
                findings.push(Finding::new(
                    NET_NOT_FOUND,
                    Severity::Error,
                    Some(g.name.as_str()),
                    format!("the net {name} is not on the board"),
                ));
            }
        }
    }
    if missing {
        return None;
    }
    let tolerance_nm = g
        .tolerance_nm
        .unwrap_or(settings.default_group_tolerance_nm);
    let min_nm = nets.iter().map(|n| n.length_nm).min().unwrap_or(0);
    let max_nm = nets.iter().map(|n| n.length_nm).max().unwrap_or(0);
    let spread_nm = max_nm - min_nm;
    let mut ok = true;
    match g.target_nm {
        Some(target) => {
            for n in &nets {
                let off = (n.length_nm - target).abs();
                if off > tolerance_nm {
                    ok = false;
                    findings.push(Finding::new(
                        GROUP_TARGET,
                        Severity::Error,
                        Some(g.name.as_str()),
                        format!(
                            "{} is {} nm from the target, the tolerance is {} nm",
                            n.name, off, tolerance_nm
                        ),
                    ));
                }
            }
        }
        None => {
            if spread_nm > tolerance_nm {
                ok = false;
                findings.push(Finding::new(
                    GROUP_MISMATCH,
                    Severity::Error,
                    Some(g.name.as_str()),
                    format!(
                        "the nets differ by {spread_nm} nm, the tolerance is {tolerance_nm} nm"
                    ),
                ));
            }
        }
    }
    Some(GroupResult {
        name: g.name.clone(),
        nets,
        min_nm,
        max_nm,
        spread_nm,
        tolerance_nm,
        target_nm: g.target_nm,
        ok,
    })
}

fn judge_pair(
    board: &Board,
    p: &PairSpec,
    settings: &LengthSettings,
    resolver: &mut Resolver<'_>,
    findings: &mut Vec<Finding>,
) -> Option<PairResult> {
    let positive = resolver.get(&p.positive, findings);
    let negative = resolver.get(&p.negative, findings);
    let (Some((pid, p_nm, p_fs)), Some((nid, n_nm, n_fs))) = (positive, negative) else {
        for (name, found) in [(&p.positive, positive), (&p.negative, negative)] {
            if found.is_none() {
                findings.push(Finding::new(
                    NET_NOT_FOUND,
                    Severity::Error,
                    Some(p.name.as_str()),
                    format!("the net {name} is not on the board"),
                ));
            }
        }
        return None;
    };
    let mut ok = true;
    let subject = Some(p.name.as_str());
    let skew_nm = (p_nm - n_nm).abs();
    let max_skew_nm = p.max_skew_nm.unwrap_or(settings.default_group_tolerance_nm);
    if skew_nm > max_skew_nm {
        ok = false;
        findings.push(Finding::new(
            PAIR_SKEW,
            Severity::Error,
            subject,
            format!("the nets differ by {skew_nm} nm, at most {max_skew_nm} nm is allowed"),
        ));
    }
    let p_pieces = pieces_of(board, pid);
    let n_pieces = pieces_of(board, nid);
    if let Some(want) = p.width_nm {
        let tol = p
            .width_tolerance_nm
            .unwrap_or(settings.pair_width_tolerance_nm);
        for (name, pieces) in [(&p.positive, &p_pieces), (&p.negative, &n_pieces)] {
            let off = pieces
                .iter()
                .filter(|x| (x.width - want).abs() > tol)
                .count();
            if off > 0 {
                ok = false;
                findings.push(Finding::new(
                    PAIR_WIDTH,
                    Severity::Error,
                    subject,
                    format!("{off} pieces of {name} are not {want} nm wide (tolerance {tol} nm)"),
                ));
            }
        }
    }
    let range = p
        .coupling_range_nm
        .unwrap_or(settings.pair_coupling_range_nm);
    let (coupled_nm, min_gap_nm, max_gap_nm) = coupling(&p_pieces, &n_pieces, range);
    let positive_tracks_nm = resolver.by_net.get(&pid).map_or(0, |n| n.tracks_nm);
    let uncoupled_nm = (positive_tracks_nm - coupled_nm).max(0);
    if coupled_nm == 0 {
        findings.push(Finding::new(
            PAIR_NOT_COUPLED,
            Severity::Warning,
            subject,
            format!(
                "no piece of {} runs within {range} nm of {}",
                p.positive, p.negative
            ),
        ));
    } else if let (Some(gap), Some(lo), Some(hi)) = (p.gap_nm, min_gap_nm, max_gap_nm) {
        let tol = p.gap_tolerance_nm.unwrap_or(settings.pair_gap_tolerance_nm);
        if lo < gap - tol {
            ok = false;
            findings.push(Finding::new(
                PAIR_GAP_SMALL,
                Severity::Error,
                subject,
                format!("the gap is {lo} nm at its smallest, {gap} nm minus {tol} nm was asked"),
            ));
        }
        if hi > gap + tol {
            ok = false;
            findings.push(Finding::new(
                PAIR_GAP_LARGE,
                Severity::Error,
                subject,
                format!("the gap is {hi} nm at its largest, {gap} nm plus {tol} nm was asked"),
            ));
        }
    }
    if let Some(most) = p.max_uncoupled_nm {
        if uncoupled_nm > most {
            ok = false;
            findings.push(Finding::new(
                PAIR_UNCOUPLED,
                Severity::Error,
                subject,
                format!("{uncoupled_nm} nm run uncoupled, at most {most} nm is allowed"),
            ));
        }
    }
    Some(PairResult {
        name: p.name.clone(),
        positive: p.positive.clone(),
        negative: p.negative.clone(),
        positive_nm: p_nm,
        negative_nm: n_nm,
        skew_nm,
        max_skew_nm,
        delay_skew_fs: (p_fs - n_fs).abs(),
        coupled_nm,
        uncoupled_nm,
        min_gap_nm,
        max_gap_nm,
        ok,
    })
}

/// Measures the nets of the board and judges the groups and pairs of the file. A net that the
/// board does not have is an error finding and its group or pair is left out of the results.
pub fn analyse(
    board: &Board,
    settings: &LengthSettings,
    job: Option<&StackupJob>,
    file: Option<&LengthFile>,
) -> Result<LengthReport, LengthError> {
    let (nets, mut findings) = measure_nets(board, settings, job)?;
    let mut groups: Vec<GroupResult> = Vec::new();
    let mut pairs: Vec<PairResult> = Vec::new();
    if let Some(file) = file {
        if file.groups.len() + file.pairs.len() > settings.max_items {
            return Err(LengthError::new(
                LIMIT_EXCEEDED,
                "the file has more groups and pairs than max_items",
            ));
        }
        let mut resolver = Resolver::new(board, &nets);
        for g in &file.groups {
            if let Some(result) = judge_group(g, settings, &mut resolver, &mut findings) {
                groups.push(result);
            }
        }
        for p in &file.pairs {
            if let Some(result) = judge_pair(board, p, settings, &mut resolver, &mut findings) {
                pairs.push(result);
            }
        }
    }
    Ok(LengthReport {
        nets,
        groups,
        pairs,
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_of_groups_and_pairs_is_read_and_checked() {
        let text = r#"{"format":"phoenix-lengths/1",
            "groups":[{"name":"data","nets":["D0","D1"],"tolerance_nm":500000}],
            "pairs":[{"name":"usb","positive":"P","negative":"N","gap_nm":200000}]}"#;
        let file = LengthFile::from_json(text).unwrap();
        assert_eq!(file.groups.len(), 1);
        assert_eq!(file.pairs[0].gap_nm, Some(200_000));
        assert_eq!(file.pairs[0].max_skew_nm, None);
    }

    #[test]
    fn a_bad_file_is_refused() {
        let wrong = r#"{"format":"x/1"}"#;
        assert!(LengthFile::from_json(wrong).is_err());
        let one = r#"{"format":"phoenix-lengths/1","groups":[{"name":"g","nets":["A"]}]}"#;
        assert!(LengthFile::from_json(one)
            .unwrap_err()
            .detail
            .contains("at least 2"));
        let target =
            r#"{"format":"phoenix-lengths/1","groups":[{"name":"g","nets":["A"],"target_nm":5}]}"#;
        assert!(LengthFile::from_json(target).is_ok());
        let same = r#"{"format":"phoenix-lengths/1","pairs":[{"name":"p","positive":"A","negative":"A"}]}"#;
        assert!(LengthFile::from_json(same).is_err());
        let negative = r#"{"format":"phoenix-lengths/1","pairs":[{"name":"p","positive":"A","negative":"B","gap_nm":-1}]}"#;
        assert!(LengthFile::from_json(negative).is_err());
        let unknown = r#"{"format":"phoenix-lengths/1","pairs":[{"name":"p","positive":"A","negative":"B","colour":1}]}"#;
        assert!(LengthFile::from_json(unknown).is_err());
        let twice = r#"{"format":"phoenix-lengths/1","groups":[{"name":"g","nets":["A","B"]},{"name":"g","nets":["A","B"]}]}"#;
        assert!(LengthFile::from_json(twice).is_err());
        assert!(LengthFile::from_json("not json").is_err());
    }

    #[test]
    fn two_parallel_pieces_are_coupled_and_their_gap_is_measured_edge_to_edge() {
        let layer: Id = "01890a5d-ac96-7ab2-80e2-4536629c1301".parse().unwrap();
        let piece = |y: i64| Piece {
            layer,
            width: 100_000,
            a: (0, y),
            b: (10_000_000, y),
            arc: false,
        };
        // centre lines 300 000 nm apart, tracks 100 000 nm wide: 200 000 nm between the edges
        let (coupled, lo, hi) = coupling(&[piece(0)], &[piece(300_000)], 1_000_000);
        assert_eq!(coupled, 10_000_000);
        assert_eq!(lo, Some(200_000));
        assert_eq!(hi, Some(200_000));
        // too far to count as coupled
        let (far, lo_far, _) = coupling(&[piece(0)], &[piece(5_000_000)], 1_000_000);
        assert_eq!(far, 0);
        assert_eq!(lo_far, None);
        // on another layer nothing is coupled
        let other: Id = "01890a5d-ac96-7ab2-80e2-4536629c1302".parse().unwrap();
        let elsewhere = Piece {
            layer: other,
            ..piece(300_000)
        };
        assert_eq!(coupling(&[piece(0)], &[elsewhere], 1_000_000).0, 0);
    }
}
