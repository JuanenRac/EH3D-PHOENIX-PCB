// SPDX-License-Identifier: GPL-3.0-or-later
//! Differential pairs routed as two coupled tracks.
//!
//! The pair is searched as one fat track on the grid, with only horizontal and vertical steps, on
//! a single layer: its width is the two tracks and the gap between them. The path of the fat
//! track is its centre line; the positive track and the negative track are that line moved
//! sideways by half the distance between the two tracks, one to each side, with the corners
//! where the moved lines cross (all integer, as the pieces are at right angles). Each track then
//! leaves its pad by a stub of two pieces, tried in both orders, and every piece of both tracks is
//! checked against the copper of the board and of the run with the width and the clearance of its
//! own net before anything is kept.
//!
//! When there is no room for that (the ends have no layer in common, the nets cross between their
//! ends, a stub or a track is blocked) the pair is not coupled: both nets are routed as ordinary
//! nets by the search of the run, and the report says why. A pair that needs a via is also routed
//! as two ordinary nets, unless `couple_via` is on: then the pair may cross to another layer once,
//! straight through, with one via for each track side by side.

use std::collections::HashMap;

use super::{simplify, Assembled, Engine, Run, ViaItem};
use crate::geom::{half_up, seg_len, P};
use crate::obstacles::{Edge, Spec, ViaSpec};
use crate::search::{search, Endpoint, Job, Miss, Node, Terminal};

/// A differential pair to route as two coupled tracks: the names of its two nets and the gap
/// between the edges of the two tracks. The width of the tracks is the width of the nets, which
/// must be the same for both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairSpec {
    /// The name of the pair, for the report.
    pub name: String,
    /// The name of the positive net.
    pub positive: String,
    /// The name of the negative net.
    pub negative: String,
    /// The gap between the edges of the two tracks, in nanometres.
    pub gap_nm: i64,
}

/// The distance between the centre lines of two tracks that run side by side, as the pair asks
/// for it: rounded up to an even number of nanometres, so the gap may be larger by one.
fn centre_distance(width: i64, gap: i64) -> i64 {
    2 * half_up(width + gap)
}

fn dist2(a: P, b: P) -> i128 {
    let dx = i128::from(a.0) - i128::from(b.0);
    let dy = i128::from(a.1) - i128::from(b.1);
    dx * dx + dy * dy
}

fn sign(v: i64) -> i64 {
    v.signum()
}

/// The left normal of a piece that goes in the direction `d` (each component -1, 0 or 1).
fn left(d: (i64, i64)) -> (i64, i64) {
    (-d.1, d.0)
}

fn direction(a: P, b: P) -> (i64, i64) {
    (sign(b.0 - a.0), sign(b.1 - a.1))
}

/// The line moved sideways by `h`, to the left for `side` = 1 and to the right for -1. The corners
/// are where the moved lines cross.
fn offset(lane: &[P], side: i64, h: i64) -> Vec<P> {
    let n = lane.len();
    (0..n)
        .map(|i| {
            let mut v = (0_i64, 0_i64);
            if i > 0 {
                let l = left(direction(lane[i - 1], lane[i]));
                v = (v.0 + l.0, v.1 + l.1);
            }
            if i + 1 < n {
                let l = left(direction(lane[i], lane[i + 1]));
                v = (v.0 + l.0, v.1 + l.1);
            }
            (lane[i].0 + side * h * v.0, lane[i].1 + side * h * v.1)
        })
        .collect()
}

/// The ways from a pad to a point: straight when they share a coordinate, else two pieces in
/// either order.
fn stubs(at: P, to: P) -> Vec<Vec<P>> {
    if at == to {
        return vec![vec![at]];
    }
    if at.0 == to.0 || at.1 == to.1 {
        return vec![vec![at, to]];
    }
    vec![vec![at, (to.0, at.1), to], vec![at, (at.0, to.1), to]]
}

/// A track: the stub from the first pad, the line and the stub to the second pad.
fn track(first: &[P], line: &[P], last: &[P]) -> Vec<P> {
    let mut pts: Vec<P> = first.to_vec();
    pts.extend(line.iter().skip(1).copied());
    pts.extend(last.iter().rev().skip(1).copied());
    simplify(&pts)
}

impl Engine<'_> {
    /// Every piece of a track is free of other copper for a net.
    fn track_is_clear(&self, layer: usize, pts: &[P], spec: &Spec) -> bool {
        pts.len() >= 2
            && pts
                .windows(2)
                .all(|w| self.world.edge(layer, w[0], w[1], spec, false, false) == Edge::Free)
    }

    /// Takes a route away without remembering where it was.
    fn uncommit(&mut self, c: usize) {
        if let Some(routed) = self.routed[c].take() {
            for (layer, id) in routed.handles {
                self.world.take(layer, id);
            }
        }
    }

    /// The nodes of the grid near the middle of a pair of pads where the fat track may start or
    /// end, nearest to the middle first.
    fn pair_terminals(
        &self,
        cfg: &crate::config::AutorouteConfig,
        layer: usize,
        pads: (&Endpoint, &Endpoint),
        fat: &Spec,
    ) -> Vec<Terminal> {
        let (p, n) = pads;
        let mid = ((p.at.0 + n.at.0) / 2, (p.at.1 + n.at.1) / 2);
        let apart = seg_len(p.at, n.at);
        let ext = p.extent.max(n.extent);
        let pitch = self.grid.pitch;
        let reach =
            cfg.terminal_radius_cells + (ext + fat.half + fat.gap + apart + pitch - 1) / pitch;
        let cx = (mid.0 - self.grid.ox).div_euclid(pitch);
        let cy = (mid.1 - self.grid.oy).div_euclid(pitch);
        let mut cells: Vec<(i128, i64, i64)> = Vec::new();
        for ix in (cx - reach).max(0)..=(cx + reach + 1).min(self.grid.nx - 1) {
            for iy in (cy - reach).max(0)..=(cy + reach + 1).min(self.grid.ny - 1) {
                cells.push((dist2(self.grid.point(ix, iy), mid), ix, iy));
            }
        }
        cells.sort_unstable();
        let keep_away = |e: &Endpoint| {
            let r = i128::from(e.extent) + i128::from(fat.half) + i128::from(fat.gap);
            r * r
        };
        let mut out: Vec<Terminal> = Vec::new();
        for (_, ix, iy) in cells {
            if out.len() >= cfg.terminal_candidates {
                break;
            }
            let node = self.grid.point(ix, iy);
            if dist2(node, p.at) < keep_away(p) || dist2(node, n.at) < keep_away(n) {
                continue;
            }
            if self.world.edge(layer, node, node, fat, false, false) != Edge::Free {
                continue;
            }
            let stub = if node == mid {
                vec![node]
            } else {
                vec![mid, node]
            };
            out.push(Terminal {
                slot: 0,
                ix,
                iy,
                stub,
            });
        }
        out
    }

    /// Tries to route the two connections of a pair as coupled tracks. On success both are
    /// routed and put down; on failure nothing is changed (but the expanded nodes are counted)
    /// and the reason comes back.
    pub(super) fn couple(&mut self, cp: usize, cn: usize, gap: i64) -> Result<usize, String> {
        let (np, nn) = (self.conns[cp].net, self.conns[cn].net);
        if self.nets[np].width != self.nets[nn].width {
            return Err("the two nets have tracks of different widths".to_owned());
        }
        let (pa, pb) = (self.conns[cp].a.clone(), self.conns[cp].b.clone());
        let (mut na, mut nb) = (self.conns[cn].a.clone(), self.conns[cn].b.clone());
        if dist2(pa.at, na.at) + dist2(pb.at, nb.at) > dist2(pa.at, nb.at) + dist2(pb.at, na.at) {
            std::mem::swap(&mut na, &mut nb);
        }
        let common: Vec<usize> = (0..self.slots.len())
            .filter(|k| [&pa, &pb, &na, &nb].iter().all(|e| e.slots.contains(k)))
            .collect();
        let ends = [&pa, &pb, &na, &nb];
        let shared = !common.is_empty();
        let mut reason = "the four ends have no layer in common".to_owned();
        for k in common {
            match self.couple_on(cp, cn, k, gap, ends) {
                Ok(()) => return Ok(0),
                Err(why) => reason = why,
            }
        }
        if self.cfg.couple_via {
            match self.couple_through_via((cp, cn), gap, ends) {
                Ok(()) => return Ok(1),
                // the reason of the search on one layer is the better one when there was one
                Err(why) if !shared => reason = why,
                Err(_) => {}
            }
            if self.cfg.couple_layers_max > 1 {
                match self.couple_through_many((cp, cn), gap, ends) {
                    Ok(changes) => return Ok(changes),
                    Err(why) if !shared => reason = why,
                    Err(_) => {}
                }
            }
        }
        Err(reason)
    }

    fn couple_on(
        &mut self,
        cp: usize,
        cn: usize,
        k: usize,
        gap: i64,
        ends: [&Endpoint; 4],
    ) -> Result<(), String> {
        let [pa, pb, na, nb] = ends;
        let layer = self.slots[k];
        let (np, nn) = (self.conns[cp].net, self.conns[cn].net);
        let width = self.nets[np].width;
        let apart = centre_distance(width, gap);
        let h = apart / 2;
        let fat = Spec {
            net: self.nets[np].id,
            half: half_up(apart + width),
            gap: self.nets[np].gap.max(self.nets[nn].gap),
        };
        let mut flat = self.cfg.clone();
        flat.diagonal = false;
        let sources = self.pair_terminals(&flat, layer, (pa, na), &fat);
        if sources.is_empty() {
            return Err("no free place near the first pair of pads".to_owned());
        }
        let targets = self.pair_terminals(&flat, layer, (pb, nb), &fat);
        if targets.is_empty() {
            return Err("no free place near the second pair of pads".to_owned());
        }
        let remaining = self
            .cfg
            .max_expansions_total
            .saturating_sub(self.expansions);
        let one = [layer];
        let history = HashMap::new();
        let no_vias = HashMap::new();
        let result = search(&Job {
            world: &self.world,
            grid: &self.grid,
            cfg: &flat,
            spec: fat,
            via: self.via_spec(np),
            slots: &one,
            sources: &sources,
            targets: &targets,
            soft: false,
            history: &history,
            limit: self.cfg.max_expansions_per_search.min(remaining),
            free_vias: &no_vias,
        });
        let found = match result {
            Ok(found) => {
                self.expansions += found.expansions;
                found
            }
            Err(Miss::Exhausted(n)) | Err(Miss::Limit(n)) => {
                self.expansions += n;
                return Err(format!("no way for the pair after {n} expanded nodes"));
            }
        };
        let points: Vec<P> = found
            .nodes
            .iter()
            .map(|n| self.grid.point(n.ix, n.iy))
            .collect();
        let lane = simplify(&points);
        if lane.len() < 2 {
            return Err("the ends of the pair are too close for two tracks".to_owned());
        }
        // on which side of the line the negative net lies, at both ends
        let first = left(direction(lane[0], lane[1]));
        let last = left(direction(lane[lane.len() - 2], lane[lane.len() - 1]));
        let on = |n: P, p: P, l: (i64, i64)| sign((n.0 - p.0) * l.0 + (n.1 - p.1) * l.1);
        let (s_first, s_last) = (on(na.at, pa.at, first), on(nb.at, pb.at, last));
        if s_first != 0 && s_last != 0 && s_first != s_last {
            return Err("the two nets change sides between their ends".to_owned());
        }
        let side = if s_first != 0 {
            s_first
        } else if s_last != 0 {
            s_last
        } else {
            1
        };
        let lane_p = offset(&lane, -side, h);
        let lane_n = offset(&lane, side, h);
        let spec_p = self.spec(np);
        let spec_n = self.spec(nn);
        let nodes: Vec<(usize, i64, i64)> = found.nodes.iter().map(|n| (k, n.ix, n.iy)).collect();
        for first_p in stubs(pa.at, lane_p[0]) {
            for last_p in stubs(pb.at, lane_p[lane_p.len() - 1]) {
                let pts_p = track(&first_p, &lane_p, &last_p);
                if !self.track_is_clear(layer, &pts_p, &spec_p) {
                    continue;
                }
                self.commit(
                    cp,
                    Assembled {
                        runs: vec![Run {
                            layer,
                            pts: pts_p.clone(),
                        }],
                        vias: Vec::new(),
                        nodes: nodes.clone(),
                        snapped: 0,
                    },
                );
                for first_n in stubs(na.at, lane_n[0]) {
                    for last_n in stubs(nb.at, lane_n[lane_n.len() - 1]) {
                        let pts_n = track(&first_n, &lane_n, &last_n);
                        if !self.track_is_clear(layer, &pts_n, &spec_n) {
                            continue;
                        }
                        self.commit(
                            cn,
                            Assembled {
                                runs: vec![Run { layer, pts: pts_n }],
                                vias: Vec::new(),
                                nodes: Vec::new(),
                                snapped: 0,
                            },
                        );
                        return Ok(());
                    }
                }
                self.uncommit(cp);
            }
        }
        Err("a track of the pair is blocked".to_owned())
    }

    /// Tries to route the pair across two layers: the pair is searched as one fat track that
    /// changes layer once, straight through (not at a corner), and each track then has its own
    /// via, the two vias side by side at the distance of the two tracks. Used when the pair cannot
    /// be coupled on one layer and `couple_via` is on. On failure nothing is changed (but the
    /// expanded nodes are counted).
    fn couple_through_via(
        &mut self,
        conns: (usize, usize),
        gap: i64,
        ends: [&Endpoint; 4],
    ) -> Result<(), String> {
        let [pa, pb, na, nb] = ends;
        let (np, nn) = (self.conns[conns.0].net, self.conns[conns.1].net);
        let apart = centre_distance(self.nets[np].width, gap);
        let (vp, vn) = (self.via_spec(np), self.via_spec(nn));
        let between = self.nets[np].gap.max(self.nets[nn].gap);
        let copper_ok = apart >= vp.half + vn.half + between;
        let holes_ok = apart >= vp.drill_half + vn.drill_half + vp.drill_gap;
        if !copper_ok || !holes_ok {
            return Err("the two vias of the pair would be too close to each other".to_owned());
        }
        let both = |a: &Endpoint, b: &Endpoint| -> Vec<usize> {
            (0..self.slots.len())
                .filter(|k| a.slots.contains(k) && b.slots.contains(k))
                .collect()
        };
        let (first, last) = (both(pa, na), both(pb, nb));
        if first.is_empty() || last.is_empty() {
            return Err("an end of the pair has no layer that both its pads are on".to_owned());
        }
        let mut reason = "the pair has no two layers to cross".to_owned();
        for &ka in &first {
            for &kb in &last {
                if ka == kb {
                    continue;
                }
                match self.couple_across(conns, gap, (ka, kb), ends) {
                    Ok(()) => return Ok(()),
                    Err(why) => reason = why,
                }
            }
        }
        Err(reason)
    }

    /// The pair from layer `ka` (the first ends) to layer `kb` (the last ends), by one via for
    /// each track.
    fn couple_across(
        &mut self,
        conns: (usize, usize),
        gap: i64,
        layers: (usize, usize),
        ends: [&Endpoint; 4],
    ) -> Result<(), String> {
        let [pa, pb, na, nb] = ends;
        let (cp, cn) = conns;
        let (ka, kb) = layers;
        let (np, nn) = (self.conns[cp].net, self.conns[cn].net);
        let width = self.nets[np].width;
        let apart = centre_distance(width, gap);
        let h = apart / 2;
        let (vp, vn) = (self.via_spec(np), self.via_spec(nn));
        let fat = Spec {
            net: self.nets[np].id,
            half: half_up(apart + width),
            gap: self.nets[np].gap.max(self.nets[nn].gap),
        };
        // a circle around both vias, for the search
        let fat_via = ViaSpec {
            half: h + vp.half.max(vn.half),
            drill_half: h + vp.drill_half.max(vn.drill_half),
            drill_gap: vp.drill_gap,
            in_pad: vp.in_pad,
        };
        let ks = [ka.min(kb), ka.max(kb)];
        let stack = [self.slots[ks[0]], self.slots[ks[1]]];
        let (la, lb) = (self.slots[ka], self.slots[kb]);
        let (from_slot, to_slot) = (usize::from(ka > kb), usize::from(kb > ka));
        let mut flat = self.cfg.clone();
        flat.diagonal = false;
        let mut sources = self.pair_terminals(&flat, la, (pa, na), &fat);
        if sources.is_empty() {
            return Err("no free place near the first pair of pads".to_owned());
        }
        let mut targets = self.pair_terminals(&flat, lb, (pb, nb), &fat);
        if targets.is_empty() {
            return Err("no free place near the second pair of pads".to_owned());
        }
        for t in &mut sources {
            t.slot = from_slot;
        }
        for t in &mut targets {
            t.slot = to_slot;
        }
        let remaining = self
            .cfg
            .max_expansions_total
            .saturating_sub(self.expansions);
        let history = HashMap::new();
        let no_vias = HashMap::new();
        let result = search(&Job {
            world: &self.world,
            grid: &self.grid,
            cfg: &flat,
            spec: fat,
            via: fat_via,
            slots: &stack,
            sources: &sources,
            targets: &targets,
            soft: false,
            history: &history,
            limit: self.cfg.max_expansions_per_search.min(remaining),
            free_vias: &no_vias,
        });
        let found = match result {
            Ok(found) => {
                self.expansions += found.expansions;
                found
            }
            Err(Miss::Exhausted(n)) | Err(Miss::Limit(n)) => {
                self.expansions += n;
                return Err(format!("no way for the pair after {n} expanded nodes"));
            }
        };
        let first_slot = found.nodes[0].slot;
        let changes = found
            .nodes
            .windows(2)
            .filter(|w| w[0].slot != w[1].slot)
            .count();
        let cut = found.nodes.iter().position(|n| n.slot != first_slot);
        let Some(cut) = cut.filter(|_| changes == 1) else {
            return Err("the pair would need more than one change of layer".to_owned());
        };
        let point = |n: &Node| self.grid.point(n.ix, n.iy);
        let pts_a: Vec<P> = found.nodes[..cut].iter().map(point).collect();
        let pts_b: Vec<P> = found.nodes[cut..].iter().map(point).collect();
        let whole: Vec<P> = found.nodes.iter().map(point).collect();
        let (lane_a, lane_b, lane) = (simplify(&pts_a), simplify(&pts_b), simplify(&whole));
        let centre = pts_b[0];
        if lane.len() < 2 {
            return Err("the ends of the pair are too close for two tracks".to_owned());
        }
        // the direction the pair crosses in: the same on both layers
        let along_a = (lane_a.len() >= 2).then(|| {
            let n = lane_a.len();
            direction(lane_a[n - 2], lane_a[n - 1])
        });
        let along_b = (lane_b.len() >= 2).then(|| direction(lane_b[0], lane_b[1]));
        let across = match (along_a, along_b) {
            (Some(a), Some(b)) if a != b => return Err("the pair turns at its via".to_owned()),
            (Some(a), _) | (None, Some(a)) => a,
            (None, None) => {
                return Err("the ends of the pair are too close for two tracks".to_owned());
            }
        };
        // on which side of the line the negative net lies, at both ends
        let first = left(direction(lane[0], lane[1]));
        let last = left(direction(lane[lane.len() - 2], lane[lane.len() - 1]));
        let on = |n: P, p: P, l: (i64, i64)| sign((n.0 - p.0) * l.0 + (n.1 - p.1) * l.1);
        let (s_first, s_last) = (on(na.at, pa.at, first), on(nb.at, pb.at, last));
        if s_first != 0 && s_last != 0 && s_first != s_last {
            return Err("the two nets change sides between their ends".to_owned());
        }
        let side = if s_first != 0 {
            s_first
        } else if s_last != 0 {
            s_last
        } else {
            1
        };
        let normal = left(across);
        let via_p = (
            centre.0 - side * h * normal.0,
            centre.1 - side * h * normal.1,
        );
        let via_n = (
            centre.0 + side * h * normal.0,
            centre.1 + side * h * normal.1,
        );
        let shifted = |lane: &[P], s: i64, via: P| -> Vec<P> {
            if lane.len() >= 2 {
                offset(lane, s, h)
            } else {
                vec![via]
            }
        };
        let (a_p, b_p) = (
            shifted(&lane_a, -side, via_p),
            shifted(&lane_b, -side, via_p),
        );
        let (a_n, b_n) = (shifted(&lane_a, side, via_n), shifted(&lane_b, side, via_n));
        let (spec_p, spec_n) = (self.spec(np), self.spec(nn));
        let (lo, hi) = (la.min(lb), la.max(lb));
        for (at, via, spec) in [(via_p, &vp, &spec_p), (via_n, &vn, &spec_n)] {
            if self.world.via(at, lo, hi, via, spec, false) != Edge::Free {
                return Err("a via of the pair is blocked".to_owned());
            }
        }
        let nodes: Vec<(usize, i64, i64)> = found
            .nodes
            .iter()
            .map(|n| (ks[n.slot], n.ix, n.iy))
            .collect();
        for first_p in stubs(pa.at, a_p[0]) {
            for last_p in stubs(pb.at, b_p[b_p.len() - 1]) {
                let run_a = track(&first_p, &a_p, &[via_p]);
                let run_b = track(&[via_p], &b_p, &last_p);
                if !self.track_is_clear(la, &run_a, &spec_p)
                    || !self.track_is_clear(lb, &run_b, &spec_p)
                {
                    continue;
                }
                let routes = vec![
                    Run {
                        layer: la,
                        pts: run_a,
                    },
                    Run {
                        layer: lb,
                        pts: run_b,
                    },
                ];
                let assembled = Assembled {
                    runs: routes,
                    vias: vec![ViaItem { at: via_p, lo, hi }],
                    nodes: nodes.clone(),
                    snapped: 0,
                };
                self.commit(cp, assembled);
                for first_n in stubs(na.at, a_n[0]) {
                    for last_n in stubs(nb.at, b_n[b_n.len() - 1]) {
                        let run_a = track(&first_n, &a_n, &[via_n]);
                        let run_b = track(&[via_n], &b_n, &last_n);
                        if !self.track_is_clear(la, &run_a, &spec_n)
                            || !self.track_is_clear(lb, &run_b, &spec_n)
                        {
                            continue;
                        }
                        let routes = vec![
                            Run {
                                layer: la,
                                pts: run_a,
                            },
                            Run {
                                layer: lb,
                                pts: run_b,
                            },
                        ];
                        let assembled = Assembled {
                            runs: routes,
                            vias: vec![ViaItem { at: via_n, lo, hi }],
                            nodes: Vec::new(),
                            snapped: 0,
                        };
                        self.commit(cn, assembled);
                        return Ok(());
                    }
                }
                self.uncommit(cp);
            }
        }
        Err("a track of the pair is blocked".to_owned())
    }
}

impl Engine<'_> {
    /// Tries to route the pair across several layers, with up to `couple_layers_max` changes of
    /// layer: the pair is searched as one fat track over all the layers the ends allow, and every
    /// change of layer is made straight through (the same direction before and after), with one
    /// via for each track side by side at the distance of the two tracks. The layers in between
    /// need at least two nodes of the grid each. Used when the pair cannot be coupled on one layer
    /// or across two, and `couple_via` is on. On success the number of changes of layer comes
    /// back. On failure nothing is changed (but the expanded nodes are counted).
    fn couple_through_many(
        &mut self,
        conns: (usize, usize),
        gap: i64,
        ends: [&Endpoint; 4],
    ) -> Result<usize, String> {
        let [pa, pb, na, nb] = ends;
        let (cp, cn) = conns;
        let (np, nn) = (self.conns[cp].net, self.conns[cn].net);
        let width = self.nets[np].width;
        let apart = centre_distance(width, gap);
        let h = apart / 2;
        let (vp, vn) = (self.via_spec(np), self.via_spec(nn));
        let between = self.nets[np].gap.max(self.nets[nn].gap);
        let need_copper = vp.half + vn.half + between;
        let need_hole = vp.drill_half + vn.drill_half + vp.drill_gap;
        if apart < need_copper || apart < need_hole {
            return Err("the two vias of the pair would be too close to each other".to_owned());
        }
        let both = |a: &Endpoint, b: &Endpoint| -> Vec<usize> {
            (0..self.slots.len())
                .filter(|k| a.slots.contains(k) && b.slots.contains(k))
                .collect()
        };
        let (first, last) = (both(pa, na), both(pb, nb));
        if first.is_empty() || last.is_empty() {
            return Err("an end of the pair has no layer that both its pads are on".to_owned());
        }
        let fat = Spec {
            net: self.nets[np].id,
            half: half_up(apart + width),
            gap: between,
        };
        let fat_via = ViaSpec {
            half: h + vp.half.max(vn.half),
            drill_half: h + vp.drill_half.max(vn.drill_half),
            drill_gap: vp.drill_gap,
            in_pad: vp.in_pad,
        };
        let mut flat = self.cfg.clone();
        flat.diagonal = false;
        let mut sources: Vec<Terminal> = Vec::new();
        for &k in &first {
            let mut found = self.pair_terminals(&flat, self.slots[k], (pa, na), &fat);
            found.iter_mut().for_each(|t| t.slot = k);
            sources.extend(found);
        }
        if sources.is_empty() {
            return Err("no free place near the first pair of pads".to_owned());
        }
        let mut targets: Vec<Terminal> = Vec::new();
        for &k in &last {
            let mut found = self.pair_terminals(&flat, self.slots[k], (pb, nb), &fat);
            found.iter_mut().for_each(|t| t.slot = k);
            targets.extend(found);
        }
        if targets.is_empty() {
            return Err("no free place near the second pair of pads".to_owned());
        }
        let remaining = self
            .cfg
            .max_expansions_total
            .saturating_sub(self.expansions);
        let history = HashMap::new();
        let no_vias = HashMap::new();
        let result = search(&Job {
            world: &self.world,
            grid: &self.grid,
            cfg: &flat,
            spec: fat,
            via: fat_via,
            slots: &self.slots,
            sources: &sources,
            targets: &targets,
            soft: false,
            history: &history,
            limit: self.cfg.max_expansions_per_search.min(remaining),
            free_vias: &no_vias,
        });
        let found = match result {
            Ok(found) => {
                self.expansions += found.expansions;
                found
            }
            Err(Miss::Exhausted(n)) | Err(Miss::Limit(n)) => {
                self.expansions += n;
                return Err(format!("no way for the pair after {n} expanded nodes"));
            }
        };
        // the stretches of the path on one layer, in order
        let mut groups: Vec<(usize, Vec<P>)> = Vec::new();
        for n in &found.nodes {
            let at = self.grid.point(n.ix, n.iy);
            match groups.last_mut() {
                Some((slot, pts)) if *slot == n.slot => pts.push(at),
                _ => groups.push((n.slot, vec![at])),
            }
        }
        let changes = groups.len() - 1;
        if changes == 0 {
            return Err("the pair does not need to change layer".to_owned());
        }
        if changes > self.cfg.couple_layers_max as usize {
            return Err(format!(
                "the pair would need {changes} changes of layer, the limit is {}",
                self.cfg.couple_layers_max
            ));
        }
        let lanes: Vec<Vec<P>> = groups.iter().map(|(_, pts)| simplify(pts)).collect();
        if lanes[1..changes].iter().any(|lane| lane.len() < 2) {
            return Err("two changes of layer of the pair would be on one place".to_owned());
        }
        // the direction the pair crosses in at each change: the same on both layers
        let mut across: Vec<(i64, i64)> = Vec::new();
        for (before, after) in lanes.iter().zip(lanes.iter().skip(1)) {
            let along_a = (before.len() >= 2)
                .then(|| direction(before[before.len() - 2], before[before.len() - 1]));
            let along_b = (after.len() >= 2).then(|| direction(after[0], after[1]));
            match (along_a, along_b) {
                (Some(a), Some(b)) if a != b => return Err("the pair turns at its via".to_owned()),
                (Some(a), _) | (None, Some(a)) => across.push(a),
                (None, None) => {
                    return Err("the ends of the pair are too close for two tracks".to_owned());
                }
            }
        }
        // on which side of the line the negative net lies, at both ends
        let whole: Vec<P> = found
            .nodes
            .iter()
            .map(|n| self.grid.point(n.ix, n.iy))
            .collect();
        let lane = simplify(&whole);
        if lane.len() < 2 {
            return Err("the ends of the pair are too close for two tracks".to_owned());
        }
        let first_side = left(direction(lane[0], lane[1]));
        let last_side = left(direction(lane[lane.len() - 2], lane[lane.len() - 1]));
        let on = |n: P, p: P, l: (i64, i64)| sign((n.0 - p.0) * l.0 + (n.1 - p.1) * l.1);
        let (s_first, s_last) = (on(na.at, pa.at, first_side), on(nb.at, pb.at, last_side));
        if s_first != 0 && s_last != 0 && s_first != s_last {
            return Err("the two nets change sides between their ends".to_owned());
        }
        let side = if s_first != 0 {
            s_first
        } else if s_last != 0 {
            s_last
        } else {
            1
        };
        let vias_at = |sign_of_net: i64| -> Vec<P> {
            (0..changes)
                .map(|i| {
                    let normal = left(across[i]);
                    let centre = groups[i + 1].1[0];
                    (
                        centre.0 + sign_of_net * h * normal.0,
                        centre.1 + sign_of_net * h * normal.1,
                    )
                })
                .collect()
        };
        let (via_p, via_n) = (vias_at(-side), vias_at(side));
        let all: Vec<P> = via_p.iter().chain(via_n.iter()).copied().collect();
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                let d = dist2(*a, *b);
                let (c, o) = (i128::from(need_copper), i128::from(need_hole));
                if d < c * c || d < o * o {
                    return Err("two vias of the pair would be too close to each other".to_owned());
                }
            }
        }
        let span = |i: usize| {
            let (x, y) = (self.slots[groups[i].0], self.slots[groups[i + 1].0]);
            (x.min(y), x.max(y))
        };
        let (spec_p, spec_n) = (self.spec(np), self.spec(nn));
        for (i, (at_p, at_n)) in via_p.iter().zip(via_n.iter()).enumerate() {
            let (lo, hi) = span(i);
            for (at, via, spec) in [(*at_p, &vp, &spec_p), (*at_n, &vn, &spec_n)] {
                if self.world.via(at, lo, hi, via, spec, false) != Edge::Free {
                    return Err("a via of the pair is blocked".to_owned());
                }
            }
        }
        // the lane of each stretch, moved to the side of each net; a single node at an end is the
        // via beside it
        let moved = |s: i64, vias: &[P]| -> Vec<Vec<P>> {
            (0..lanes.len())
                .map(|g| {
                    if lanes[g].len() >= 2 {
                        offset(&lanes[g], s, h)
                    } else if g == 0 {
                        vec![vias[0]]
                    } else {
                        vec![vias[changes - 1]]
                    }
                })
                .collect()
        };
        let (moved_p, moved_n) = (moved(-side, &via_p), moved(side, &via_n));
        // the tracks of one net: every stretch, the first and the last with the stub to the pad
        let stretches = |first: &[P], last: &[P], shifted: &[Vec<P>], vias: &[P]| {
            let m = shifted.len();
            (0..m)
                .map(|g| {
                    if g == 0 {
                        track(first, &shifted[0], &[vias[0]])
                    } else if g == m - 1 {
                        track(&[vias[g - 1]], &shifted[g], last)
                    } else {
                        simplify(&shifted[g])
                    }
                })
                .collect::<Vec<Vec<P>>>()
        };
        let layers: Vec<usize> = groups.iter().map(|(slot, _)| self.slots[*slot]).collect();
        let clear = |this: &Self, runs: &[Vec<P>], spec: &Spec| {
            runs.iter()
                .enumerate()
                .all(|(g, pts)| this.track_is_clear(layers[g], pts, spec))
        };
        let assemble_for =
            |runs: Vec<Vec<P>>, vias: &[P], nodes: Vec<(usize, i64, i64)>| Assembled {
                runs: runs
                    .into_iter()
                    .enumerate()
                    .map(|(g, pts)| Run {
                        layer: layers[g],
                        pts,
                    })
                    .collect(),
                vias: vias
                    .iter()
                    .enumerate()
                    .map(|(i, at)| {
                        let (x, y) = (layers[i], layers[i + 1]);
                        ViaItem {
                            at: *at,
                            lo: x.min(y),
                            hi: x.max(y),
                        }
                    })
                    .collect(),
                nodes,
                snapped: 0,
            };
        let nodes: Vec<(usize, i64, i64)> =
            found.nodes.iter().map(|n| (n.slot, n.ix, n.iy)).collect();
        let m = moved_p.len();
        for first_p in stubs(pa.at, moved_p[0][0]) {
            for last_p in stubs(pb.at, moved_p[m - 1][moved_p[m - 1].len() - 1]) {
                let runs_p = stretches(&first_p, &last_p, &moved_p, &via_p);
                if !clear(self, &runs_p, &spec_p) {
                    continue;
                }
                let routed_p = assemble_for(runs_p, &via_p, nodes.clone());
                self.commit(cp, routed_p);
                for first_n in stubs(na.at, moved_n[0][0]) {
                    for last_n in stubs(nb.at, moved_n[m - 1][moved_n[m - 1].len() - 1]) {
                        let runs_n = stretches(&first_n, &last_n, &moved_n, &via_n);
                        if !clear(self, &runs_n, &spec_n) {
                            continue;
                        }
                        let routed_n = assemble_for(runs_n, &via_n, Vec::new());
                        self.commit(cn, routed_n);
                        return Ok(changes);
                    }
                }
                self.uncommit(cp);
            }
        }
        Err("a track of the pair is blocked".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_is_moved_sideways_and_the_corners_are_where_the_moved_lines_cross() {
        // east, then north: the left side is north of the first piece and west of the second
        let line = [(0, 0), (10, 0), (10, 10)];
        assert_eq!(offset(&line, 1, 2), vec![(0, 2), (8, 2), (8, 10)]);
        assert_eq!(offset(&line, -1, 2), vec![(0, -2), (12, -2), (12, 10)]);
        // the moved lines keep the distance 2h from each other
        let a = offset(&line, 1, 2);
        let b = offset(&line, -1, 2);
        assert_eq!(b[0].1 - a[0].1, -4);
        assert_eq!(b[1].0 - a[1].0, 4);
    }

    #[test]
    fn a_stub_goes_straight_or_in_two_pieces_in_either_order() {
        assert_eq!(stubs((1, 1), (1, 1)), vec![vec![(1, 1)]]);
        assert_eq!(stubs((1, 1), (5, 1)), vec![vec![(1, 1), (5, 1)]]);
        assert_eq!(
            stubs((1, 1), (5, 3)),
            vec![vec![(1, 1), (5, 1), (5, 3)], vec![(1, 1), (1, 3), (5, 3)]]
        );
    }

    #[test]
    fn a_track_is_the_two_stubs_and_the_line_without_repeated_or_aligned_points() {
        let line = [(3, 5), (17, 5)];
        let t = track(&[(2, 5), (3, 5)], &line, &[(18, 5), (17, 5)]);
        assert_eq!(t, vec![(2, 5), (18, 5)]);
        let t = track(&[(2, 6), (3, 6), (3, 5)], &line, &[(18, 5), (17, 5)]);
        assert_eq!(t, vec![(2, 6), (3, 6), (3, 5), (18, 5)]);
    }

    #[test]
    fn the_distance_between_the_lines_is_even_and_never_less_than_asked() {
        assert_eq!(centre_distance(250_000, 150_000), 400_000);
        assert_eq!(centre_distance(250_001, 150_000), 400_002);
    }
}
