// SPDX-License-Identifier: GPL-3.0-or-later
//! The advanced checks on silkscreen over holes, necks, stubs, isolation by voltage and copper
//! islands. The others are in `checks`.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::connectivity::Item as ConItem;
use eh3d_phoenix_board::{Board, Segment, Track};
use eh3d_phoenix_model::Point;

use super::checks::{
    both, box_of, judge, make_index, net_name, shares_layer, to_i64, via_is_through, Ctx, Kind,
    Verdict,
};
use super::grid::{inflate, overlap, Bx};
use super::{make, not_judged, R_ISLAND, R_ISOLATION, R_NECK, R_SILK_HOLE, R_STUB};
use crate::geometry::{contact, inside_polygon, Contact, Prim};
use crate::report::Report;

/// A piece of drawn silkscreen or a hole, with the box that holds it.
struct Piece {
    label: String,
    prim: Prim,
    bx: Bx,
}

fn piece(label: String, prim: Prim) -> Piece {
    let bx = box_of(&prim);
    Piece { label, prim, bx }
}

/// The silkscreen against the drill of the through holes of pads and vias.
pub(super) fn silk_holes(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.silk_hole_level else {
        return;
    };
    let limit_nm = ctx.cfg.silk_hole_min_nm;
    let limit = i128::from(limit_nm);
    let board = ctx.board;
    let mut holes: Vec<Piece> = Vec::new();
    for (id, via) in &board.vias {
        if via_is_through(board, via) {
            let d = i128::from(via.drill.get());
            if let Some(prim) = Prim::at_point(via.at, d, d) {
                holes.push(piece(format!("via {id}"), prim));
            }
        }
    }
    for pad in &ctx.pads {
        if let Some(drill) = pad.drill {
            let d = i128::from(drill.get());
            if let Some(prim) = Prim::at_point(pad.at, d, d) {
                holes.push(piece(pad.label.clone(), prim));
            }
        }
    }
    if holes.is_empty() {
        return;
    }
    let mut strokes: Vec<Piece> = Vec::new();
    for (sid, item) in &board.silkscreen {
        let label = format!("silk {sid}");
        let drawn = match item.drawn() {
            Ok(drawn) => drawn,
            Err(_) => {
                out.inconclusive.push(not_judged(
                    label,
                    "the silkscreen cannot be drawn, so its distance to the holes is not judged",
                ));
                continue;
            }
        };
        let width = i128::from(drawn.width.get());
        for segment in &drawn.pieces {
            let (parts, arc) = match segment {
                Segment::Line { start, end } => (vec![(*start, *end)], false),
                Segment::Arc { start, end, .. } => (vec![(*start, *start), (*end, *end)], true),
            };
            if let Some(prim) = Prim::new(parts, width, width, arc) {
                strokes.push(piece(label.clone(), prim));
            }
        }
    }
    let boxes: Vec<Bx> = strokes.iter().chain(holes.iter()).map(|p| p.bx).collect();
    let index = make_index(ctx.mode, &boxes, ctx.cfg);
    let mut proven: BTreeMap<(String, String), (i64, Point)> = BTreeMap::new();
    let mut unsure: BTreeSet<(String, String)> = BTreeSet::new();
    for (i, stroke) in strokes.iter().enumerate() {
        for j in index.candidates(&inflate(&boxes[i], limit)) {
            let Some(hole) = j.checked_sub(strokes.len()).and_then(|k| holes.get(k)) else {
                continue;
            };
            if !overlap(&boxes[i], &boxes[j], limit) {
                continue;
            }
            let key = (stroke.label.clone(), hole.label.clone());
            match judge(&stroke.prim, &hole.prim, limit) {
                Verdict::Violates(gap) => {
                    let at = hole.prim.first_point();
                    let entry = proven.entry(key).or_insert((gap, at));
                    if gap < entry.0 {
                        *entry = (gap, at);
                    }
                }
                Verdict::Maybe | Verdict::Unknown => {
                    unsure.insert(key);
                }
                Verdict::Clear => {}
            }
        }
    }
    for ((silk, hole), (gap, at)) in &proven {
        out.confirmed.push(
            make(&R_SILK_HOLE, severity, both(silk, hole))
                .at(at.x_nm.get(), at.y_nm.get())
                .values(*gap, limit_nm),
        );
    }
    for (silk, hole) in unsure.iter().filter(|k| !proven.contains_key(*k)) {
        out.inconclusive.push(
            not_judged(
                both(silk, hole),
                "the numbers are too large for the exact test",
            )
            .limit(limit_nm),
        );
    }
}

/// The first point of a track and the last.
fn ends(track: &Track) -> Option<(Point, Point)> {
    Some((track.path.first()?.start(), track.path.last()?.end()))
}

/// The items of the same net on a shared layer that a point touches.
struct Touch {
    joined: Vec<usize>,
    undecided: bool,
}

fn touching(ctx: &Ctx<'_>, i: usize, p: Point) -> Touch {
    let mut touch = Touch {
        joined: Vec::new(),
        undecided: false,
    };
    let me = &ctx.items[i];
    let Some(point) = Prim::at_point(p, 0, 0) else {
        touch.undecided = true;
        return touch;
    };
    let (x, y) = (i128::from(p.x_nm.get()), i128::from(p.y_nm.get()));
    for j in ctx.index.candidates(&[x - 1, y - 1, x + 1, y + 1]) {
        if j == i {
            continue;
        }
        let other = &ctx.items[j];
        if other.net != me.net || !shares_layer(me, other) {
            continue;
        }
        match contact(&point, &other.prim, 0) {
            Contact::Yes => touch.joined.push(j),
            Contact::Maybe | Contact::Unknown => touch.undecided = true,
            Contact::No => {}
        }
    }
    touch
}

/// A track narrower than the limit whose end joins wider copper of its net.
pub(super) fn necks(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.neck_level else {
        return;
    };
    let min = i128::from(ctx.cfg.neck_min_width_nm);
    for (i, item) in ctx.items.iter().enumerate() {
        if item.kind != Kind::Track || item.prim.lo >= min {
            continue;
        }
        let Some(track) = item.id.and_then(|id| ctx.board.tracks.get(&id)) else {
            continue;
        };
        let Some((first, last)) = ends(track) else {
            continue;
        };
        for end in [first, last] {
            let wider = touching(ctx, i, end)
                .joined
                .into_iter()
                .find(|j| ctx.items[*j].prim.lo > item.prim.lo);
            if let Some(j) = wider {
                out.confirmed.push(
                    make(&R_NECK, severity, item.label.clone())
                        .at(end.x_nm.get(), end.y_nm.get())
                        .values(to_i64(item.prim.lo), ctx.cfg.neck_min_width_nm)
                        .why(&format!(
                            "joins {} ({} nm wide)",
                            ctx.items[j].label, ctx.items[j].prim.lo
                        )),
                );
                break;
            }
        }
    }
}

/// How an end of a track is joined.
enum End {
    Joined,
    Free,
    Undecided,
}

fn end_state(ctx: &Ctx<'_>, i: usize, track: &Track, p: Point) -> End {
    let touch = touching(ctx, i, p);
    if !touch.joined.is_empty() {
        return End::Joined;
    }
    let in_zone = ctx
        .board
        .zones
        .values()
        .any(|z| z.net == track.net && z.layer == track.layer && inside_polygon(p, &z.outline));
    if in_zone {
        End::Joined
    } else if touch.undecided {
        End::Undecided
    } else {
        End::Free
    }
}

/// A track with one end that touches nothing of its net while the other end is joined.
pub(super) fn stubs(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.stub_level else {
        return;
    };
    for (i, item) in ctx.items.iter().enumerate() {
        if item.kind != Kind::Track {
            continue;
        }
        let Some(track) = item.id.and_then(|id| ctx.board.tracks.get(&id)) else {
            continue;
        };
        let Some((first, last)) = ends(track) else {
            continue;
        };
        if first == last {
            continue;
        }
        let states = (
            end_state(ctx, i, track, first),
            end_state(ctx, i, track, last),
        );
        let (free_end, sure) = match states {
            (End::Free, End::Joined) => (first, true),
            (End::Joined, End::Free) => (last, true),
            (End::Free, End::Undecided) => (first, false),
            (End::Undecided, End::Free) => (last, false),
            _ => continue,
        };
        let (x, y) = (free_end.x_nm.get(), free_end.y_nm.get());
        if sure {
            out.confirmed
                .push(make(&R_STUB, severity, item.label.clone()).at(x, y));
        } else {
            out.inconclusive.push(
                not_judged(
                    item.label.clone(),
                    "the shape of the copper at the other end is too coarse to tell if the track is a stub",
                )
                .at(x, y),
            );
        }
    }
}

/// The voltage the settings give to the net of an item.
fn volts_of(ctx: &Ctx<'_>, net: Option<eh3d_phoenix_board::Id>) -> Option<i64> {
    let name = net_name(ctx.board, net)?;
    ctx.cfg.voltages.get(name).copied()
}

/// Copper of two nets closer than the table of spacing asks for the voltage between them.
pub(super) fn isolation(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.isolation_level else {
        return;
    };
    if ctx.cfg.voltages.len() < 2 {
        return;
    }
    let cfg = ctx.cfg;
    let (low, high) = cfg
        .voltages
        .values()
        .fold((i64::MAX, i64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let column = cfg.iso_column;
    let table_max = cfg.iso_rows.iter().map(|r| r.nm[column]).max().unwrap_or(0);
    let margin = i128::from(table_max.max(cfg.iso_per_volt_nm[column].saturating_mul(high - low)));
    let volts: Vec<Option<i64>> = ctx.items.iter().map(|c| volts_of(ctx, c.net)).collect();
    for (i, j) in ctx.pairs(margin) {
        let (a, b) = (&ctx.items[i], &ctx.items[j]);
        let (Some(va), Some(vb)) = (volts[i], volts[j]) else {
            continue;
        };
        let dv = (va - vb).abs();
        if a.net == b.net || dv == 0 || !shares_layer(a, b) {
            continue;
        }
        let required = cfg.required_spacing_nm(dv);
        let location = both(&a.label, &b.label);
        let at = a.prim.first_point();
        match judge(&a.prim, &b.prim, i128::from(required)) {
            Verdict::Violates(gap) => out.confirmed.push(
                make(&R_ISOLATION, severity, location)
                    .at(at.x_nm.get(), at.y_nm.get())
                    .values(gap, required)
                    .why(&format!(
                        "{dv} V between the nets; IPC-2221 table 6-1 column {}, figures not verified against the standard",
                        cfg.iso_column_name()
                    )),
            ),
            Verdict::Maybe | Verdict::Unknown => out.inconclusive.push(
                not_judged(
                    location,
                    "the shape is too coarse, or the numbers too large, to judge the spacing",
                )
                .at(at.x_nm.get(), at.y_nm.get())
                .limit(required),
            ),
            Verdict::Clear => {}
        }
    }
}

fn item_label(item: &ConItem) -> String {
    match item {
        ConItem::Pad(pad) => format!("pad {}", pad.pad),
        ConItem::Track(id) => format!("track {id}"),
        ConItem::Via(id) => format!("via {id}"),
        ConItem::Zone(id) => format!("zone {id}"),
    }
}

fn item_point(board: &Board, item: &ConItem) -> Option<Point> {
    match item {
        ConItem::Track(id) => board
            .tracks
            .get(id)
            .and_then(|t| t.path.first())
            .map(Segment::start),
        ConItem::Via(id) => board.vias.get(id).map(|v| v.at),
        ConItem::Zone(id) => board.zones.get(id).and_then(|z| z.outline.first().copied()),
        ConItem::Pad(_) => None,
    }
}

/// Copper (tracks, vias, zones) of a net that is joined together but to no pad.
pub(super) fn islands(ctx: &Ctx<'_>, out: &mut Report) {
    let Some(severity) = ctx.cfg.island_level else {
        return;
    };
    let report = ctx.board.connectivity();
    for net in &report.nets {
        let name = net_name(ctx.board, Some(net.net)).unwrap_or("?");
        for island in &net.islands {
            let Some(first) = island.items.first() else {
                continue;
            };
            if island.items.iter().any(|i| matches!(i, ConItem::Pad(_))) {
                continue;
            }
            let labels: Vec<String> = island.items.iter().map(item_label).collect();
            let shown: Vec<&str> = labels.iter().take(5).map(String::as_str).collect();
            let more = if labels.len() > 5 { ", ..." } else { "" };
            let mut finding = make(&R_ISLAND, severity, format!("net {name}: {}", labels[0])).why(
                &format!("{} items: {}{more}", labels.len(), shown.join(", ")),
            );
            if let Some(p) = item_point(ctx.board, first) {
                finding = finding.at(p.x_nm.get(), p.y_nm.get());
            }
            out.confirmed.push(finding);
        }
    }
}
