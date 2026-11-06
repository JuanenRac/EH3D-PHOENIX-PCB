// SPDX-License-Identifier: GPL-3.0-or-later
//! The search: A* over the nodes of the routing grid, layer by layer, with vias between the
//! layers. A state is a node, the layer and the direction the search arrived in (so that a corner
//! can cost something and a turn sharper than 90 degrees can be refused). The cost is in
//! nanometres of track: the length of the steps, a penalty for a corner, for a via, for a step
//! against the preferred direction of its layer, for a node that was torn up before and, when
//! rip-up looks for the nets in the way, for a step through the copper of another net.
//!
//! The order of expansion is fixed: the queue is ordered by (estimate, cost so far, state), so the
//! same input always gives the same path. Everything is integer arithmetic.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::config::AutorouteConfig;
use crate::geom::{seg_len, P};
use crate::obstacles::{Edge, Spec, ViaSpec, World};

/// The routing grid: nodes at a fixed pitch, anchored at multiples of the pitch.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Grid {
    pub ox: i64,
    pub oy: i64,
    pub pitch: i64,
    pub nx: i64,
    pub ny: i64,
}

impl Grid {
    /// Where a node is.
    pub(crate) fn point(&self, ix: i64, iy: i64) -> P {
        (self.ox + ix * self.pitch, self.oy + iy * self.pitch)
    }
}

/// A place where a track may start or end: a node of a layer and the stub from the pad.
#[derive(Debug, Clone)]
pub(crate) struct Terminal {
    pub slot: usize,
    pub ix: i64,
    pub iy: i64,
    /// From the end of the connection (the centre of a pad) to the node, corners included.
    pub stub: Vec<P>,
}

/// A node of a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Node {
    pub ix: i64,
    pub iy: i64,
    pub slot: usize,
}

/// What a search found.
#[derive(Debug, Clone)]
pub(crate) struct Found {
    pub nodes: Vec<Node>,
    pub src: usize,
    pub dst: usize,
    pub expansions: u64,
}

/// Why a search found nothing, with the nodes it expanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Miss {
    /// Every reachable node was expanded and none was an end.
    Exhausted(u64),
    /// The limit of nodes was reached.
    Limit(u64),
}

/// Everything a search needs.
pub(crate) struct Job<'a> {
    pub world: &'a World,
    pub grid: &'a Grid,
    pub cfg: &'a AutorouteConfig,
    pub spec: Spec,
    pub via: ViaSpec,
    /// The place in the copper stack of each layer the search may use, in the order of the stack.
    pub slots: &'a [usize],
    pub sources: &'a [Terminal],
    pub targets: &'a [Terminal],
    pub soft: bool,
    pub history: &'a HashMap<(usize, i64, i64), u32>,
    pub limit: u64,
    /// The vias of this net that are already on the board and sit exactly on a node of the grid,
    /// by node, with the first and the last place in the copper stack they join. A change of
    /// layer there costs nothing and puts no new via.
    pub free_vias: &'a HashMap<(i64, i64), (usize, usize)>,
}

const DIRS: [(i64, i64); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];
const NO_DIR: u64 = 8;
const AFTER_VIA: u64 = 9;

fn encode(g: &Grid, slot: usize, ix: i64, iy: i64, dir: u64) -> u64 {
    (((slot as u64 * g.ny as u64 + iy as u64) * g.nx as u64 + ix as u64) * 10) + dir
}

fn decode(g: &Grid, key: u64) -> (usize, i64, i64, u64) {
    let dir = key % 10;
    let rest = key / 10;
    let ix = (rest % g.nx as u64) as i64;
    let rest = rest / g.nx as u64;
    let iy = (rest % g.ny as u64) as i64;
    let slot = (rest / g.ny as u64) as usize;
    (slot, ix, iy, dir)
}

/// The length of a stub, rounded up piece by piece.
pub(crate) fn stub_len(stub: &[P]) -> i64 {
    stub.windows(2).map(|w| seg_len(w[0], w[1])).sum()
}

type Best = HashMap<u64, (i64, u64)>;
type Queue = BinaryHeap<Reverse<(i64, i64, u64)>>;

fn relax(best: &mut Best, queue: &mut Queue, key: u64, g: i64, prev: u64, h: i64) {
    if best.get(&key).is_none_or(|r| g < r.0) {
        best.insert(key, (g, prev));
        queue.push(Reverse((g + h, g, key)));
    }
}

/// Runs the search.
pub(crate) fn search(job: &Job<'_>) -> Result<Found, Miss> {
    let g = job.grid;
    let ortho = g.pitch;
    let diag = (g.pitch * 1414 + 500) / 1000;
    let mut goals: HashMap<(usize, i64, i64), usize> = HashMap::new();
    for (i, t) in job.targets.iter().enumerate() {
        goals.insert((t.slot, t.ix, t.iy), i);
    }
    let heuristic = |ix: i64, iy: i64| -> i64 {
        let mut best = i64::MAX;
        for t in job.targets {
            let dx = (ix - t.ix).abs();
            let dy = (iy - t.iy).abs();
            let h = if job.cfg.diagonal {
                diag * dx.min(dy) + ortho * (dx.max(dy) - dx.min(dy))
            } else {
                ortho * (dx + dy)
            };
            best = best.min(h);
        }
        if best == i64::MAX {
            0
        } else {
            best
        }
    };
    let mut best: Best = HashMap::new();
    let mut queue: Queue = BinaryHeap::new();
    for t in job.sources {
        let key = encode(g, t.slot, t.ix, t.iy, NO_DIR);
        relax(
            &mut best,
            &mut queue,
            key,
            stub_len(&t.stub),
            u64::MAX,
            heuristic(t.ix, t.iy),
        );
    }
    let mut via_cache: HashMap<(i64, i64, usize, usize), Edge> = HashMap::new();
    let mut expansions: u64 = 0;
    while let Some(Reverse((_, cost, key))) = queue.pop() {
        if best.get(&key).is_some_and(|r| r.0 < cost) {
            continue;
        }
        let (slot, ix, iy, dir) = decode(g, key);
        if let Some(&dst) = goals.get(&(slot, ix, iy)) {
            return Ok(rebuild(g, &best, key, job.sources, dst, expansions));
        }
        expansions += 1;
        if expansions > job.limit {
            return Err(Miss::Limit(expansions));
        }
        let layer = job.slots[slot];
        let here = g.point(ix, iy);
        for (d, (dx, dy)) in DIRS.iter().enumerate() {
            let odd = d & 1 == 1;
            if odd && !job.cfg.diagonal {
                continue;
            }
            if dir < NO_DIR {
                let turn = (d as u64 + 8 - dir) % 8;
                if (3..=5).contains(&turn) {
                    continue;
                }
            }
            let (jx, jy) = (ix + dx, iy + dy);
            if jx < 0 || jy < 0 || jx >= g.nx || jy >= g.ny {
                continue;
            }
            let there = g.point(jx, jy);
            let extra = match job
                .world
                .edge(layer, here, there, &job.spec, job.soft, false)
            {
                Edge::Blocked => continue,
                Edge::Soft => job.cfg.ripup_penalty_nm,
                Edge::Free => 0,
            };
            let base = if odd { diag } else { ortho };
            let against = if (slot & 1) == 0 {
                d & 3 == 2
            } else {
                d & 3 == 0
            };
            let wrong = if odd {
                base * job.cfg.wrong_way_percent / 200
            } else if against {
                base * job.cfg.wrong_way_percent / 100
            } else {
                0
            };
            let corner = if dir < NO_DIR && dir != d as u64 {
                job.cfg.corner_penalty_nm
            } else {
                0
            };
            let torn = i64::from(job.history.get(&(slot, jx, jy)).copied().unwrap_or(0))
                * job.cfg.history_penalty_nm;
            let next = encode(g, slot, jx, jy, d as u64);
            relax(
                &mut best,
                &mut queue,
                next,
                cost + base + wrong + corner + extra + torn,
                key,
                heuristic(jx, jy),
            );
        }
        if dir == AFTER_VIA {
            continue;
        }
        for other in 0..job.slots.len() {
            if other == slot {
                continue;
            }
            let reuse = job.free_vias.get(&(ix, iy)).is_some_and(|&(lo, hi)| {
                (lo..=hi).contains(&job.slots[slot]) && (lo..=hi).contains(&job.slots[other])
            });
            if reuse {
                let next = encode(g, other, ix, iy, AFTER_VIA);
                relax(&mut best, &mut queue, next, cost, key, heuristic(ix, iy));
                continue;
            }
            let (lo_slot, hi_slot) = (slot.min(other), slot.max(other));
            let verdict = *via_cache
                .entry((ix, iy, lo_slot, hi_slot))
                .or_insert_with(|| {
                    job.world.via(
                        here,
                        job.slots[lo_slot],
                        job.slots[hi_slot],
                        &job.via,
                        &job.spec,
                        job.soft,
                    )
                });
            let extra = match verdict {
                Edge::Blocked => continue,
                Edge::Soft => job.cfg.ripup_penalty_nm,
                Edge::Free => 0,
            };
            let torn = i64::from(job.history.get(&(other, ix, iy)).copied().unwrap_or(0))
                * job.cfg.history_penalty_nm;
            let next = encode(g, other, ix, iy, AFTER_VIA);
            relax(
                &mut best,
                &mut queue,
                next,
                cost + job.cfg.via_penalty_nm + extra + torn,
                key,
                heuristic(ix, iy),
            );
        }
    }
    Err(Miss::Exhausted(expansions))
}

fn rebuild(
    g: &Grid,
    best: &Best,
    last: u64,
    sources: &[Terminal],
    dst: usize,
    expansions: u64,
) -> Found {
    let mut nodes: Vec<Node> = Vec::new();
    let mut key = last;
    loop {
        let (slot, ix, iy, _) = decode(g, key);
        nodes.push(Node { ix, iy, slot });
        match best.get(&key) {
            Some(&(_, prev)) if prev != u64::MAX => key = prev,
            _ => break,
        }
    }
    nodes.reverse();
    let first = nodes[0];
    let src = sources
        .iter()
        .position(|t| t.slot == first.slot && t.ix == first.ix && t.iy == first.iy)
        .unwrap_or(0);
    Found {
        nodes,
        src,
        dst,
        expansions,
    }
}

/// The way from the end of a connection to a node: along an axis and then at 45 degrees (or
/// along both axes when the diagonal is off). Every piece is horizontal, vertical or diagonal.
pub(crate) fn dogleg(at: P, node: P, diagonal: bool) -> Vec<P> {
    if at == node {
        return vec![at];
    }
    let dx = node.0 - at.0;
    let dy = node.1 - at.1;
    let (adx, ady) = (dx.abs(), dy.abs());
    if dx == 0 || dy == 0 || (diagonal && adx == ady) {
        return vec![at, node];
    }
    let corner = if !diagonal {
        (node.0, at.1)
    } else if adx > ady {
        (at.0 + dx.signum() * (adx - ady), at.1)
    } else {
        (at.0, at.1 + dy.signum() * (ady - adx))
    };
    vec![at, corner, node]
}

/// A connection end: where it is, the layers it is on and how far its copper reaches.
#[derive(Debug, Clone)]
pub(crate) struct Endpoint {
    pub at: P,
    pub slots: Vec<usize>,
    pub extent: i64,
}

/// The places near an end where a track may start or end, nearest first: for every layer of the
/// end, up to `terminal_candidates` nodes that a stub from the centre of the pad reaches.
pub(crate) fn terminals(
    world: &World,
    grid: &Grid,
    cfg: &AutorouteConfig,
    slots: &[usize],
    end: &Endpoint,
    spec: &Spec,
    soft: bool,
) -> Vec<Terminal> {
    let reach = cfg.terminal_radius_cells + (end.extent + grid.pitch - 1) / grid.pitch;
    let cx = (end.at.0 - grid.ox).div_euclid(grid.pitch);
    let cy = (end.at.1 - grid.oy).div_euclid(grid.pitch);
    let mut cells: Vec<(i128, i64, i64)> = Vec::new();
    for ix in (cx - reach).max(0)..=(cx + reach + 1).min(grid.nx - 1) {
        for iy in (cy - reach).max(0)..=(cy + reach + 1).min(grid.ny - 1) {
            let p = grid.point(ix, iy);
            let dx = i128::from(p.0) - i128::from(end.at.0);
            let dy = i128::from(p.1) - i128::from(end.at.1);
            cells.push((dx * dx + dy * dy, ix, iy));
        }
    }
    cells.sort_unstable();
    let mut taken = vec![0_usize; end.slots.len()];
    let mut out: Vec<Terminal> = Vec::new();
    for (_, ix, iy) in cells {
        if taken.iter().all(|n| *n >= cfg.terminal_candidates) {
            break;
        }
        let node = grid.point(ix, iy);
        let stub = dogleg(end.at, node, cfg.diagonal);
        for (k, slot) in end.slots.iter().enumerate() {
            if taken[k] >= cfg.terminal_candidates {
                continue;
            }
            let layer = slots[*slot];
            let pieces: Vec<(P, P)> = if stub.len() == 1 {
                vec![(stub[0], stub[0])]
            } else {
                stub.windows(2).map(|w| (w[0], w[1])).collect()
            };
            let clear =
                pieces.iter().all(
                    |(a, b)| match world.edge(layer, *a, *b, spec, soft, false) {
                        Edge::Free => true,
                        Edge::Soft => soft,
                        Edge::Blocked => false,
                    },
                );
            if clear {
                taken[k] += 1;
                out.push(Terminal {
                    slot: *slot,
                    ix,
                    iy,
                    stub: stub.clone(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_is_written_and_read_back() {
        let g = Grid {
            ox: 0,
            oy: 0,
            pitch: 10,
            nx: 7,
            ny: 5,
        };
        for slot in 0..3 {
            for (ix, iy) in [(0, 0), (6, 4), (3, 2)] {
                for dir in 0..10 {
                    let key = encode(&g, slot, ix, iy, dir);
                    assert_eq!(decode(&g, key), (slot, ix, iy, dir));
                }
            }
        }
        assert_eq!(g.point(2, 3), (20, 30));
    }

    #[test]
    fn the_way_from_a_pad_to_a_node_is_made_of_axis_and_diagonal_pieces() {
        assert_eq!(dogleg((5, 5), (5, 5), true), vec![(5, 5)]);
        assert_eq!(dogleg((0, 0), (10, 0), true), vec![(0, 0), (10, 0)]);
        assert_eq!(dogleg((0, 0), (7, 7), true), vec![(0, 0), (7, 7)]);
        assert_eq!(dogleg((0, 0), (10, 4), true), vec![(0, 0), (6, 0), (10, 4)]);
        assert_eq!(
            dogleg((0, 0), (-4, 10), true),
            vec![(0, 0), (0, 6), (-4, 10)]
        );
        assert_eq!(
            dogleg((0, 0), (10, 4), false),
            vec![(0, 0), (10, 0), (10, 4)]
        );
        assert_eq!(stub_len(&[(0, 0), (6, 0), (10, 4)]), 6 + 6);
    }

    fn id(n: u16) -> eh3d_phoenix_board::Id {
        eh3d_phoenix_model::ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    /// A square of 10 000 nm with a grid of 1000 and a wall of another net, put down by the run,
    /// across the whole of it at x = 5000.
    fn walled() -> (World, Grid, Spec) {
        use crate::obstacles::{Obs, StaticLayer};
        let outline = vec![(0, 0), (10_000, 0), (10_000, 10_000), (0, 10_000)];
        let copper = StaticLayer::new(Vec::new()).unwrap();
        let holes = StaticLayer::new(Vec::new()).unwrap();
        let mut world = World::new(outline, vec![copper, holes], 4000);
        world.put(0, Obs::piece(id(2), (5000, -1000), (5000, 11_000), 10, 10));
        let grid = Grid {
            ox: 0,
            oy: 0,
            pitch: 1000,
            nx: 11,
            ny: 11,
        };
        let spec = Spec {
            net: id(1),
            half: 10,
            gap: 10,
        };
        (world, grid, spec)
    }

    #[test]
    fn a_wall_of_the_run_stops_the_search_and_rip_up_goes_through_it() {
        let (world, grid, spec) = walled();
        let cfg = AutorouteConfig::default();
        let sources = [Terminal {
            slot: 0,
            ix: 1,
            iy: 5,
            stub: vec![(1000, 5000)],
        }];
        let targets = [Terminal {
            slot: 0,
            ix: 9,
            iy: 5,
            stub: vec![(9000, 5000)],
        }];
        let history = HashMap::new();
        let free_vias = HashMap::new();
        let via = ViaSpec {
            half: 300,
            drill_half: 150,
            drill_gap: 0,
            in_pad: false,
        };
        let job = |soft: bool| Job {
            world: &world,
            grid: &grid,
            cfg: &cfg,
            spec,
            via,
            slots: &[0],
            sources: &sources,
            targets: &targets,
            soft,
            history: &history,
            limit: 1_000_000,
            free_vias: &free_vias,
        };
        let strict = search(&job(false));
        assert!(
            matches!(strict, Err(Miss::Exhausted(n)) if n > 0),
            "{strict:?}"
        );
        let found = search(&job(true)).unwrap();
        assert_eq!(found.nodes.first().map(|n| (n.ix, n.iy)), Some((1, 5)));
        assert_eq!(found.nodes.last().map(|n| (n.ix, n.iy)), Some((9, 5)));
        assert!(found.nodes.iter().any(|n| n.ix == 5));
        // the same input gives the same path and the same count
        let again = search(&job(true)).unwrap();
        assert_eq!(found.nodes, again.nodes);
        assert_eq!(found.expansions, again.expansions);
        // a limit of one node stops it
        let mut tight = job(true);
        tight.limit = 1;
        assert!(matches!(search(&tight), Err(Miss::Limit(2))));
    }

    #[test]
    fn the_straight_way_is_found_and_costs_nothing_but_length() {
        let (world, grid, spec) = walled();
        let cfg = AutorouteConfig::default();
        let sources = [Terminal {
            slot: 0,
            ix: 1,
            iy: 5,
            stub: vec![(1000, 5000)],
        }];
        let targets = [Terminal {
            slot: 0,
            ix: 4,
            iy: 5,
            stub: vec![(4000, 5000)],
        }];
        let history = HashMap::new();
        let free_vias = HashMap::new();
        let via = ViaSpec {
            half: 300,
            drill_half: 150,
            drill_gap: 0,
            in_pad: false,
        };
        let job = Job {
            world: &world,
            grid: &grid,
            cfg: &cfg,
            spec,
            via,
            slots: &[0],
            sources: &sources,
            targets: &targets,
            soft: false,
            history: &history,
            limit: 1_000_000,
            free_vias: &free_vias,
        };
        let found = search(&job).unwrap();
        let ixs: Vec<i64> = found.nodes.iter().map(|n| n.ix).collect();
        assert_eq!(ixs, vec![1, 2, 3, 4]);
        assert!(found.nodes.iter().all(|n| n.iy == 5 && n.slot == 0));
        assert_eq!((found.src, found.dst), (0, 0));
    }

    /// Where the path changes layer: the node it leaves.
    fn change_of_layer(found: &Found) -> Option<(i64, i64)> {
        found
            .nodes
            .windows(2)
            .find(|w| w[0].slot != w[1].slot)
            .map(|w| (w[0].ix, w[0].iy))
    }

    #[test]
    fn a_via_that_is_already_there_is_a_free_change_of_layer() {
        use crate::obstacles::StaticLayer;
        let outline = vec![(0, 0), (10_000, 0), (10_000, 10_000), (0, 10_000)];
        let layers = vec![
            StaticLayer::new(Vec::new()).unwrap(),
            StaticLayer::new(Vec::new()).unwrap(),
            StaticLayer::new(Vec::new()).unwrap(),
        ];
        let world = World::new(outline, layers, 4000);
        let grid = Grid {
            ox: 0,
            oy: 0,
            pitch: 1000,
            nx: 11,
            ny: 11,
        };
        let spec = Spec {
            net: id(1),
            half: 10,
            gap: 10,
        };
        let cfg = AutorouteConfig::default();
        let sources = [Terminal {
            slot: 0,
            ix: 1,
            iy: 5,
            stub: vec![(1000, 5000)],
        }];
        let targets = [Terminal {
            slot: 1,
            ix: 9,
            iy: 5,
            stub: vec![(9000, 5000)],
        }];
        let history = HashMap::new();
        let via = ViaSpec {
            half: 300,
            drill_half: 150,
            drill_gap: 0,
            in_pad: false,
        };
        let none = HashMap::new();
        let job = |free: &HashMap<(i64, i64), (usize, usize)>| {
            search(&Job {
                world: &world,
                grid: &grid,
                cfg: &cfg,
                spec,
                via,
                slots: &[0, 1],
                sources: &sources,
                targets: &targets,
                soft: false,
                history: &history,
                limit: 1_000_000,
                free_vias: free,
            })
            .unwrap()
        };
        // with no via of the net on the board the change is made where it is cheapest to walk
        let plain = job(&none);
        assert_ne!(change_of_layer(&plain), Some((8, 2)));
        // a via of the net at the node (8, 2) that joins the two layers: the walk to it costs far
        // less than a new via, so the path goes there and changes layer for nothing
        let free = HashMap::from([((8, 2), (0, 1))]);
        let reused = job(&free);
        assert_eq!(change_of_layer(&reused), Some((8, 2)));
        assert_eq!(reused.nodes.last().map(|n| (n.ix, n.iy)), Some((9, 5)));
        // the same input gives the same path
        assert_eq!(job(&free).nodes, reused.nodes);
        // a via that does not span the layer of the end is not used
        let short = HashMap::from([((8, 2), (0, 0))]);
        assert_ne!(change_of_layer(&job(&short)), Some((8, 2)));
    }
}
