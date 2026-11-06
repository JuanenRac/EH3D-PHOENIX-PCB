// SPDX-License-Identifier: GPL-3.0-or-later
//! Crosstalk between the parallel runs of tracks of different nets on the same layer.
//!
//! The straight pieces of two nets that run side by side (within `parallel_tolerance_deg` and
//! `coupling_range_nm`) are added up by net pair and layer: the length that runs parallel, the
//! distance between centres and the narrowest gap. From the line under them (the stack-up, or the
//! defaults) the coupling is estimated with `phys::coupling` and the near-end and far-end
//! crosstalk for the rise time of the aggressor with `phys::crosstalk`. Pieces on different layers
//! (broadside coupling) and arcs are not looked at. A quasi-static estimate for weakly coupled
//! lines, not a field solution.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Id};
use serde::Serialize;

use crate::codes::{
    Finding, Severity, SiError, COUPLING_STRONG, FEXT_HIGH, LIMIT_EXCEEDED, NEXT_HIGH,
};
use crate::env::Env;
use crate::netdata::{layer_name, net_name, straight_pieces, Piece};
use crate::phys::{coupling, crosstalk, round_to};
use crate::settings::SiSettings;

/// One victim of one aggressor on one layer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CoupledRow {
    /// The net whose edge is considered.
    pub aggressor: String,
    /// The net that receives the crosstalk.
    pub victim: String,
    /// The layer they run on.
    pub layer: String,
    /// The length that runs parallel, in millimetres.
    pub parallel_mm: f64,
    /// The distance between the centre lines, averaged over the length, in micrometres.
    pub distance_um: f64,
    /// The narrowest gap between the edges, in micrometres.
    pub gap_um: f64,
    /// The ratio of the mutual to the self inductance.
    pub lm_over_l: f64,
    /// The ratio of the mutual to the self capacitance.
    pub cm_over_c: f64,
    /// The rise time of the aggressor, in picoseconds.
    pub rise_ps: f64,
    /// The near-end crosstalk, in percent of the step of the aggressor.
    pub next_pct: f64,
    /// The far-end crosstalk, in percent of the step of the aggressor.
    pub fext_pct: f64,
    /// Whether the coupling is too strong for the estimate.
    pub strong: bool,
}

/// The crosstalk that all the aggressors together can put on one victim, added without sign.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VictimTotal {
    /// The victim.
    pub victim: String,
    /// The number of aggressors.
    pub aggressors: usize,
    /// The sum of the near-end crosstalk, in percent.
    pub next_sum_pct: f64,
    /// The sum of the absolute far-end crosstalk, in percent.
    pub fext_sum_pct: f64,
}

/// The result for a board.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CrosstalkReport {
    /// The victims of each aggressor, the worst first.
    pub rows: Vec<CoupledRow>,
    /// The sum on each victim, the worst first.
    pub victims: Vec<VictimTotal>,
    /// The arcs that were not looked at.
    pub arcs_skipped: usize,
}

/// What is added up for a pair of nets on a layer.
struct Acc {
    length: f64,
    centre_weighted: f64,
    width_weighted: f64,
    gap_min: f64,
}

/// The parallel run of two pieces: its length, the distance between centres averaged over it and
/// the least distance, all in nanometres; `None` if they are not parallel or do not overlap.
fn parallel_run(p: &Piece, q: &Piece, sin_tolerance: f64) -> Option<(f64, f64, f64)> {
    let (ax, ay) = (p.a.0 as f64, p.a.1 as f64);
    let (ux, uy) = (p.b.0 as f64 - ax, p.b.1 as f64 - ay);
    let (vx, vy) = ((q.b.0 - q.a.0) as f64, (q.b.1 - q.a.1) as f64);
    let (lp, lq) = (ux.hypot(uy), vx.hypot(vy));
    if lp < 1.0 || lq < 1.0 {
        return None;
    }
    if (ux * vy - uy * vx).abs() / (lp * lq) > sin_tolerance {
        return None;
    }
    let (ex, ey) = (ux / lp, uy / lp);
    let (nx, ny) = (-ey, ex);
    let along = |x: i64, y: i64| (x as f64 - ax) * ex + (y as f64 - ay) * ey;
    let across = |x: i64, y: i64| (x as f64 - ax) * nx + (y as f64 - ay) * ny;
    let (t0, t1) = (along(q.a.0, q.a.1), along(q.b.0, q.b.1));
    let (s0, s1) = (across(q.a.0, q.a.1), across(q.b.0, q.b.1));
    let lo = t0.min(t1).max(0.0);
    let hi = t0.max(t1).min(lp);
    if hi - lo <= 0.0 {
        return None;
    }
    let offset = |t: f64| {
        if (t1 - t0).abs() < 1.0e-9 {
            s0
        } else {
            s0 + (s1 - s0) * (t - t0) / (t1 - t0)
        }
    };
    let mid = offset((lo + hi) / 2.0).abs();
    let least = offset(lo).abs().min(offset(hi).abs()).min(mid);
    Some((hi - lo, mid, least))
}

/// The near-end and far-end crosstalk of every pair of nets that runs side by side on a layer.
/// With `only`, the rows of the aggressor of that name and the total of that victim.
pub(crate) fn analyse(
    board: &Board,
    settings: &SiSettings,
    env: &mut Env,
    only: Option<&str>,
) -> Result<(CrosstalkReport, Vec<Finding>), SiError> {
    let (pieces, arcs_skipped) = straight_pieces(board);
    if pieces.len() > settings.max_items {
        return Err(SiError::new(
            LIMIT_EXCEEDED,
            "the board has more pieces of track than max_items",
        ));
    }
    let sin_tolerance = settings.parallel_tolerance_deg.to_radians().sin();
    let range = settings.coupling_range_nm as f64;
    let mut by_layer: BTreeMap<Id, Vec<&Piece>> = BTreeMap::new();
    for piece in &pieces {
        by_layer.entry(piece.layer).or_default().push(piece);
    }
    let mut pairs: BTreeMap<(Id, Id, Id), Acc> = BTreeMap::new();
    for (layer, list) in &by_layer {
        for (i, p) in list.iter().enumerate() {
            for q in &list[i + 1..] {
                if p.net == q.net {
                    continue;
                }
                let Some((length, mid, least)) = parallel_run(p, q, sin_tolerance) else {
                    continue;
                };
                let half_widths = (p.width + q.width) as f64 / 2.0;
                if mid - half_widths > range {
                    continue;
                }
                let key = (p.net.min(q.net), p.net.max(q.net), *layer);
                let acc = pairs.entry(key).or_insert(Acc {
                    length: 0.0,
                    centre_weighted: 0.0,
                    width_weighted: 0.0,
                    gap_min: f64::MAX,
                });
                acc.length += length;
                acc.centre_weighted += mid * length;
                acc.width_weighted += half_widths * length;
                acc.gap_min = acc.gap_min.min((least - half_widths).max(0.0));
            }
        }
    }
    let mut rows = Vec::new();
    let mut findings = Vec::new();
    for ((first, second, layer), acc) in &pairs {
        if acc.length < settings.min_parallel_nm as f64 {
            continue;
        }
        let layer_label = layer_name(board, *layer);
        let centre = acc.centre_weighted / acc.length;
        let half_widths = acc.width_weighted / acc.length;
        let width_nm = (half_widths.round() as i64).max(1);
        let line = env.line(&layer_label, width_nm, 1.0);
        let c = coupling(
            line.h_m,
            centre.max(half_widths) * 1.0e-9,
            line.z0,
            line.eps_eff,
            line.er,
            line.microstrip,
            settings.weak_coupling_limit,
        );
        let td_ps = acc.length / 1.0e6 * line.delay_ps_per_mm;
        let names = [net_name(board, *first), net_name(board, *second)];
        for (aggressor, victim) in [(0, 1), (1, 0)] {
            let rise_ps = settings.rise_for(&names[aggressor]);
            let x = crosstalk(&c, td_ps, rise_ps);
            rows.push(CoupledRow {
                aggressor: names[aggressor].clone(),
                victim: names[victim].clone(),
                layer: layer_label.clone(),
                parallel_mm: round_to(acc.length / 1.0e6, 3),
                distance_um: round_to(centre / 1.0e3, 1),
                gap_um: round_to(acc.gap_min / 1.0e3, 1),
                lm_over_l: round_to(c.lm_over_l, 4),
                cm_over_c: round_to(c.cm_over_c, 4),
                rise_ps,
                next_pct: round_to(x.next_pct, 3),
                fext_pct: round_to(x.fext_pct, 3),
                strong: c.limited,
            });
        }
    }
    for row in &rows {
        judge(row, settings, &mut findings);
    }
    rows.sort_by(|a, b| {
        a.aggressor
            .cmp(&b.aggressor)
            .then(b.next_pct.total_cmp(&a.next_pct))
            .then(a.victim.cmp(&b.victim))
            .then(a.layer.cmp(&b.layer))
    });
    let mut totals: BTreeMap<String, (usize, f64, f64)> = BTreeMap::new();
    for row in &rows {
        let entry = totals.entry(row.victim.clone()).or_insert((0, 0.0, 0.0));
        entry.0 += 1;
        entry.1 += row.next_pct;
        entry.2 += row.fext_pct.abs();
    }
    let mut victims: Vec<VictimTotal> = totals
        .into_iter()
        .map(|(victim, (aggressors, next, fext))| VictimTotal {
            victim,
            aggressors,
            next_sum_pct: round_to(next, 3),
            fext_sum_pct: round_to(fext, 3),
        })
        .collect();
    victims.sort_by(|a, b| {
        b.next_sum_pct
            .total_cmp(&a.next_sum_pct)
            .then(a.victim.cmp(&b.victim))
    });
    if let Some(name) = only {
        rows.retain(|r| r.aggressor == name);
        victims.retain(|v| v.victim == name);
        let prefix = format!("{name}/");
        findings.retain(|f| {
            f.subject
                .as_deref()
                .is_some_and(|s| s.starts_with(prefix.as_str()))
        });
    }
    Ok((
        CrosstalkReport {
            rows,
            victims,
            arcs_skipped,
        },
        findings,
    ))
}

/// The findings of one row against the thresholds of the settings.
fn judge(row: &CoupledRow, s: &SiSettings, findings: &mut Vec<Finding>) {
    let subject = format!("{}/{}", row.aggressor, row.victim);
    let subject = Some(subject.as_str());
    let next = row.next_pct;
    if next >= s.next_warn_pct {
        let severity = if next >= s.next_error_pct {
            Severity::Error
        } else {
            Severity::Warning
        };
        findings.push(Finding::new(
            NEXT_HIGH,
            severity,
            subject,
            format!(
                "{next:.2} % over {:.2} mm on {} (limit {} %)",
                row.parallel_mm,
                row.layer,
                if severity == Severity::Error {
                    s.next_error_pct
                } else {
                    s.next_warn_pct
                }
            ),
        ));
    }
    let fext = row.fext_pct.abs();
    if fext >= s.fext_warn_pct {
        let severity = if fext >= s.fext_error_pct {
            Severity::Error
        } else {
            Severity::Warning
        };
        findings.push(Finding::new(
            FEXT_HIGH,
            severity,
            subject,
            format!(
                "{:.2} % over {:.2} mm on {} (limit {} %)",
                row.fext_pct,
                row.parallel_mm,
                row.layer,
                if severity == Severity::Error {
                    s.fext_error_pct
                } else {
                    s.fext_warn_pct
                }
            ),
        ));
    }
    if row.strong && row.aggressor < row.victim {
        findings.push(Finding::new(
            COUPLING_STRONG,
            Severity::Warning,
            subject,
            format!(
                "the mutual inductance reaches more than {} of the self inductance at {:.0} um between centres",
                s.weak_coupling_limit, row.distance_um
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(net: u8, y: i64, x0: i64, x1: i64) -> Piece {
        let id = |n: u8| -> Id {
            format!("01890a5d-ac96-7ab2-80e2-4536629c13{n:02x}")
                .parse()
                .unwrap()
        };
        Piece {
            net: id(net),
            layer: id(1),
            width: 200_000,
            a: (x0, y),
            b: (x1, y),
        }
    }

    #[test]
    fn two_pieces_side_by_side_run_parallel_for_their_common_length() {
        let a = piece(3, 0, 0, 10_000_000);
        let b = piece(4, 400_000, 4_000_000, 20_000_000);
        let (length, mid, least) = parallel_run(&a, &b, 0.02).unwrap();
        assert!((length - 6_000_000.0).abs() < 1e-6);
        assert!((mid - 400_000.0).abs() < 1e-6);
        assert!((least - 400_000.0).abs() < 1e-6);
        // the other way round the answer is the same
        let (length2, mid2, _) = parallel_run(&b, &a, 0.02).unwrap();
        assert!((length2 - length).abs() < 1e-6 && (mid2 - mid).abs() < 1e-6);
    }

    #[test]
    fn pieces_that_do_not_overlap_or_are_not_parallel_are_left_out() {
        let a = piece(3, 0, 0, 1_000_000);
        let far = piece(4, 400_000, 2_000_000, 3_000_000);
        assert!(parallel_run(&a, &far, 0.02).is_none());
        let slanted = Piece {
            b: (1_000_000, 500_000),
            ..piece(4, 400_000, 0, 0)
        };
        assert!(parallel_run(&a, &slanted, 0.02).is_none());
        let point = piece(4, 400_000, 5, 5);
        assert!(parallel_run(&a, &point, 0.02).is_none());
    }

    #[test]
    fn a_slightly_slanted_piece_is_parallel_inside_the_tolerance() {
        let a = piece(3, 0, 0, 10_000_000);
        // 10 um over 10 mm is 0.057 degrees
        let b = Piece {
            b: (10_000_000, 410_000),
            ..piece(4, 400_000, 0, 0)
        };
        let (length, mid, least) = parallel_run(&a, &b, 0.002).unwrap();
        assert!((length - 10_000_000.0).abs() < 1.0);
        assert!((mid - 405_000.0).abs() < 1.0);
        assert!((least - 400_000.0).abs() < 1.0);
    }
}
