// SPDX-License-Identifier: GPL-3.0-or-later
//! The run: the connections of the ratsnest, in the order of the settings, each searched on the
//! grid; a connection that finds no way is searched again through the copper of the other nets,
//! and the nets it crosses are torn up and put back in the queue (rip-up and reroute), within the
//! limits of the settings. Then the result goes to [`crate::plan`] to be judged by the rules.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

mod couple;
#[cfg(test)]
mod snap_tests;

pub use couple::PairSpec;

use eh3d_phoenix_board::connectivity::Item;
use eh3d_phoenix_board::{ArcSettings, Board, Id, PadShape};
use eh3d_phoenix_model::Point;
use eh3d_phoenix_route::{copper_layers, item_label, ratsnest, track_width, via_size};
use eh3d_phoenix_route::{RouteSettings, RuleContext};

use crate::codes::{
    AutorouteError, BOARD_NOT_ROUTABLE, EXPANSION_LIMIT, GRID_TOO_LARGE, LAYER_UNKNOWN, NET_LIMIT,
    NET_SETTING_UNUSED, NET_UNKNOWN, NO_PATH, NO_TERMINAL, PAIR_MANY_LAYERS, PAIR_UNCOUPLED,
    PAIR_WITH_VIA, RIPUP_LIMIT, TOO_MANY_CONNECTIONS, VIA_SNAPPED,
};
use crate::config::{AutorouteConfig, NetOrder, DEFAULT_WEIGHT};
use crate::geom::{half_up, isqrt_ceil, pair, turn, P};
use crate::obstacles::{build, BuildParams, Obs, Spec, ViaSpec, World};
use crate::plan::{
    finish, AutoroutePlan, NetOutcome, NetStats, PlannedTrack, PlannedVia, Totals, Unrouted,
};
use crate::search::{search, terminals, Endpoint, Found, Grid, Job, Miss, Terminal};

/// What to route: the nets and the layers, by name. Empty means all of them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// The names of the nets; empty for every net with an open connection.
    pub nets: Vec<String>,
    /// The names of the copper layers the tracks may use; empty for all of them.
    pub layers: Vec<String>,
}

struct NetInfo {
    id: Id,
    name: String,
    width: i64,
    gap: i64,
    drill: i64,
    diameter: i64,
}

struct Conn {
    net: usize,
    a: Endpoint,
    b: Endpoint,
    a_label: String,
    b_label: String,
    len: i64,
}

struct Run {
    layer: usize,
    pts: Vec<P>,
}

struct ViaItem {
    at: P,
    lo: usize,
    hi: usize,
}

struct Assembled {
    runs: Vec<Run>,
    vias: Vec<ViaItem>,
    nodes: Vec<(usize, i64, i64)>,
    /// How many existing vias off the grid the route joins by a short piece.
    snapped: usize,
}

/// An existing via of a net that is within reach of a node of the grid.
struct NearVia {
    dist2: i128,
    id: Id,
    net: usize,
    node: (i64, i64),
    at: P,
    span: (usize, usize),
}

/// For each net, the vias of it by node of the grid, with the first and the last place in the
/// copper stack they join.
type ViaSpans = Vec<HashMap<(i64, i64), (usize, usize)>>;

/// For each net, where a via really is when it is not on its node of the grid.
type ViaOffsets = Vec<HashMap<(i64, i64), P>>;

/// The vias of each net (`ids`, sorted) that are already on the board and can be used as a free
/// change of layer: the ones on a node of the grid and, when `reach` is more than 0, the ones at
/// most `reach` nanometres from the nearest node. Of several vias for one node the nearest to it
/// is kept, then the one with the lowest id.
fn index_vias(
    board: &Board,
    ids: &[Id],
    grid: &Grid,
    copper: &[Id],
    reach: i64,
) -> (ViaSpans, ViaOffsets) {
    let mut free_vias: ViaSpans = vec![HashMap::new(); ids.len()];
    let mut via_off: ViaOffsets = vec![HashMap::new(); ids.len()];
    let limit = i128::from(reach) * i128::from(reach);
    let mut near: Vec<NearVia> = Vec::new();
    for (id, via) in &board.vias {
        let Ok(n) = ids.binary_search(&via.net) else {
            continue;
        };
        let at = pair(via.at);
        let (dx, dy) = (at.0 - grid.ox, at.1 - grid.oy);
        let ix = (dx + grid.pitch / 2).div_euclid(grid.pitch);
        let iy = (dy + grid.pitch / 2).div_euclid(grid.pitch);
        if ix < 0 || iy < 0 || ix >= grid.nx || iy >= grid.ny {
            continue;
        }
        let node = grid.point(ix, iy);
        let (ex, ey) = (i128::from(at.0 - node.0), i128::from(at.1 - node.1));
        let dist2 = ex * ex + ey * ey;
        if dist2 > limit {
            continue;
        }
        let ends = [via.from_layer, via.to_layer].map(|l| copper.iter().position(|c| *c == l));
        if let [Some(a), Some(b)] = ends {
            near.push(NearVia {
                dist2,
                id: *id,
                net: n,
                node: (ix, iy),
                at,
                span: (a.min(b), a.max(b)),
            });
        }
    }
    near.sort_by_key(|v| (v.dist2, v.id));
    for v in &near {
        if free_vias[v.net].contains_key(&v.node) {
            continue;
        }
        free_vias[v.net].insert(v.node, v.span);
        if v.dist2 > 0 {
            via_off[v.net].insert(v.node, v.at);
        }
    }
    (free_vias, via_off)
}

/// The way from a node of the grid to a via near it, the node left out: one piece when they share
/// a coordinate, else a step along x and a step along y (both along an axis).
fn dogleg(node: P, via: P) -> Vec<P> {
    if node.0 == via.0 || node.1 == via.1 {
        vec![via]
    } else {
        vec![(via.0, node.1), via]
    }
}

struct Routed {
    assembled: Assembled,
    handles: Vec<(usize, usize)>,
}

enum Attempt {
    Found(Found, Vec<Terminal>, Vec<Terminal>),
    Missed(&'static str, String),
}

struct Engine<'a> {
    cfg: &'a AutorouteConfig,
    grid: Grid,
    slots: Vec<usize>,
    world: World,
    nets: Vec<NetInfo>,
    conns: Vec<Conn>,
    routed: Vec<Option<Routed>>,
    history: HashMap<(usize, i64, i64), u32>,
    /// For each net, the vias of it that are on the board and sit on a node of the grid.
    free_vias: ViaSpans,
    /// For each net, where those vias really are when that is not the node itself.
    via_off: ViaOffsets,
    drill_gap: i64,
    expansions: u64,
    ripups: u32,
    /// The weight of each net in percent.
    weights: Vec<i64>,
    /// The nodes the searches for each net have expanded.
    net_exp: Vec<u64>,
    /// The limit of expanded nodes of each net (0: none).
    net_limit: Vec<u64>,
}

/// Removes the middle point of three that are on one line and the repeated points.
fn simplify(points: &[P]) -> Vec<P> {
    let mut out: Vec<P> = Vec::new();
    for p in points {
        if out.last() == Some(p) {
            continue;
        }
        while out.len() >= 2 && turn(out[out.len() - 2], out[out.len() - 1], *p) == 0 {
            out.pop();
        }
        // a piece that goes out and comes back leaves the point it came from, once
        if out.last() == Some(p) {
            continue;
        }
        out.push(*p);
    }
    out
}

fn assemble(
    found: &Found,
    sources: &[Terminal],
    targets: &[Terminal],
    slots: &[usize],
    grid: &Grid,
    free_vias: &HashMap<(i64, i64), (usize, usize)>,
    off_grid: &HashMap<(i64, i64), P>,
) -> Assembled {
    let first = found.nodes[0];
    let mut runs: Vec<Run> = Vec::new();
    let mut vias: Vec<ViaItem> = Vec::new();
    let mut snapped = 0_usize;
    let mut current = Run {
        layer: slots[first.slot],
        pts: sources[found.src].stub.clone(),
    };
    for w in found.nodes.windows(2) {
        let (a, b) = (w[0], w[1]);
        let at = grid.point(b.ix, b.iy);
        if a.slot == b.slot {
            current.pts.push(at);
        } else {
            let (x, y) = (slots[a.slot], slots[b.slot]);
            // a via of the net that is already there is not put down again
            let there = free_vias
                .get(&(a.ix, a.iy))
                .is_some_and(|&(lo, hi)| (lo..=hi).contains(&x) && (lo..=hi).contains(&y));
            if !there {
                vias.push(ViaItem {
                    at: grid.point(a.ix, a.iy),
                    lo: x.min(y),
                    hi: x.max(y),
                });
            }
            // a via that is not on the node is joined to it by a short piece on both layers
            let mut start = vec![at];
            if let Some(&via_at) = off_grid.get(&(a.ix, a.iy)).filter(|_| there) {
                let way = dogleg(at, via_at);
                current.pts.extend_from_slice(&way);
                start = way.iter().rev().copied().collect();
                start.push(at);
                snapped += 1;
            }
            runs.push(current);
            current = Run {
                layer: slots[b.slot],
                pts: start,
            };
        }
    }
    let tail: Vec<P> = targets[found.dst]
        .stub
        .iter()
        .rev()
        .skip(1)
        .copied()
        .collect();
    current.pts.extend(tail);
    runs.push(current);
    let runs: Vec<Run> = runs
        .into_iter()
        .map(|r| Run {
            layer: r.layer,
            pts: simplify(&r.pts),
        })
        .filter(|r| r.pts.len() >= 2)
        .collect();
    Assembled {
        runs,
        vias,
        nodes: found.nodes.iter().map(|n| (n.slot, n.ix, n.iy)).collect(),
        snapped,
    }
}

impl Engine<'_> {
    fn spec(&self, net: usize) -> Spec {
        let info = &self.nets[net];
        Spec {
            net: info.id,
            half: half_up(info.width),
            gap: info.gap,
        }
    }

    fn via_spec(&self, net: usize) -> ViaSpec {
        let info = &self.nets[net];
        ViaSpec {
            half: half_up(info.diameter),
            drill_half: half_up(info.drill),
            drill_gap: self.drill_gap,
            in_pad: self.cfg.via_in_pad,
        }
    }

    /// One search for a connection, through the copper of the other nets or not.
    fn attempt(&mut self, c: usize, soft: bool) -> Attempt {
        let net = self.conns[c].net;
        let spec = self.spec(net);
        let via = self.via_spec(net);
        let (a, b) = (self.conns[c].a.clone(), self.conns[c].b.clone());
        let labels = (self.conns[c].a_label.clone(), self.conns[c].b_label.clone());
        for (end, label) in [(&a, &labels.0), (&b, &labels.1)] {
            if end.slots.is_empty() {
                return Attempt::Missed(
                    NO_TERMINAL,
                    format!("{label} is on no layer that may be used"),
                );
            }
        }
        let sources = terminals(
            &self.world,
            &self.grid,
            self.cfg,
            &self.slots,
            &a,
            &spec,
            soft,
        );
        if sources.is_empty() {
            return Attempt::Missed(NO_TERMINAL, format!("no free place near {}", labels.0));
        }
        let targets = terminals(
            &self.world,
            &self.grid,
            self.cfg,
            &self.slots,
            &b,
            &spec,
            soft,
        );
        if targets.is_empty() {
            return Attempt::Missed(NO_TERMINAL, format!("no free place near {}", labels.1));
        }
        let remaining = self
            .cfg
            .max_expansions_total
            .saturating_sub(self.expansions);
        // the vias and the corners of a net of more weight cost more
        let base: &AutorouteConfig = self.cfg;
        let weight = self.weights[net];
        let scaled;
        let cfg: &AutorouteConfig = if weight == DEFAULT_WEIGHT {
            base
        } else {
            scaled = AutorouteConfig {
                via_penalty_nm: base.via_penalty_nm * weight / DEFAULT_WEIGHT,
                corner_penalty_nm: base.corner_penalty_nm * weight / DEFAULT_WEIGHT,
                ..base.clone()
            };
            &scaled
        };
        let shared = base.max_expansions_per_search.min(remaining);
        let net_left = match self.net_limit[net] {
            0 => u64::MAX,
            limit => limit.saturating_sub(self.net_exp[net]),
        };
        let result = {
            let job = Job {
                world: &self.world,
                grid: &self.grid,
                cfg,
                spec,
                via,
                slots: &self.slots,
                sources: &sources,
                targets: &targets,
                soft,
                history: &self.history,
                limit: shared.min(net_left),
                free_vias: &self.free_vias[net],
            };
            search(&job)
        };
        match result {
            Ok(found) => {
                self.expansions += found.expansions;
                self.net_exp[net] += found.expansions;
                Attempt::Found(found, sources, targets)
            }
            Err(Miss::Exhausted(n)) => {
                self.expansions += n;
                self.net_exp[net] += n;
                Attempt::Missed(NO_PATH, format!("no way after {n} expanded nodes"))
            }
            Err(Miss::Limit(n)) => {
                self.expansions += n;
                self.net_exp[net] += n;
                if net_left < shared {
                    let limit = self.net_limit[net];
                    return Attempt::Missed(
                        NET_LIMIT,
                        format!("the net reached its limit of {limit} expanded nodes"),
                    );
                }
                Attempt::Missed(EXPANSION_LIMIT, format!("stopped after {n} expanded nodes"))
            }
        }
    }

    /// The nets of the run that an assembled route would cross.
    fn crossing(&self, c: usize, route: &Assembled) -> BTreeSet<Id> {
        let net = self.conns[c].net;
        let spec = self.spec(net);
        let via = self.via_spec(net);
        let mut out = BTreeSet::new();
        for run in &route.runs {
            for w in run.pts.windows(2) {
                self.world.blockers(run.layer, w[0], w[1], &spec, &mut out);
            }
        }
        let on_copper = Spec {
            half: via.half,
            ..spec
        };
        let hole = Spec {
            half: via.drill_half,
            gap: via.drill_gap,
            ..spec
        };
        for v in &route.vias {
            for layer in v.lo..=v.hi {
                self.world.blockers(layer, v.at, v.at, &on_copper, &mut out);
            }
            self.world
                .blockers(self.world.copper_layers(), v.at, v.at, &hole, &mut out);
        }
        out
    }

    /// Puts a route down as obstacles.
    fn commit(&mut self, c: usize, assembled: Assembled) {
        let net = self.conns[c].net;
        let spec = self.spec(net);
        let via = self.via_spec(net);
        let mut handles: Vec<(usize, usize)> = Vec::new();
        for run in &assembled.runs {
            for w in run.pts.windows(2) {
                let obs = Obs::piece(spec.net, w[0], w[1], spec.half, spec.gap);
                handles.push((run.layer, self.world.put(run.layer, obs)));
            }
        }
        let holes = self.world.copper_layers();
        for v in &assembled.vias {
            for layer in v.lo..=v.hi {
                let obs = Obs::round(Some(spec.net), v.at, via.half, spec.gap);
                handles.push((layer, self.world.put(layer, obs)));
            }
            let mut hole = Obs::round(Some(spec.net), v.at, via.drill_half, via.drill_gap);
            hole.hole = true;
            handles.push((holes, self.world.put(holes, hole)));
        }
        self.routed[c] = Some(Routed { assembled, handles });
    }

    /// Takes a route away and remembers where it was.
    fn rip(&mut self, c: usize) {
        let Some(routed) = self.routed[c].take() else {
            return;
        };
        for (layer, id) in routed.handles {
            self.world.take(layer, id);
        }
        for (slot, ix, iy) in routed.assembled.nodes {
            *self.history.entry((slot, ix, iy)).or_insert(0) += 1;
        }
    }
}

fn find_net(board: &Board, name: &str) -> Option<Id> {
    board
        .nets
        .iter()
        .find(|(_, net)| net.name == name)
        .map(|(id, _)| *id)
}

/// The routing slots of the end of a connection: the layers it is on that the run may use.
fn end_of(board: &Board, item: Item, at: Point, slots: &[usize], copper: &[Id]) -> Endpoint {
    let index_of = |layer: Id| copper.iter().position(|l| *l == layer);
    let slot_of = |stack: usize| slots.iter().position(|s| *s == stack);
    let mut on: Vec<usize> = Vec::new();
    let mut extent = 0_i64;
    match item {
        Item::Pad(pad_ref) => {
            let pad = board
                .components
                .get(&pad_ref.component)
                .and_then(|c| board.footprints.get(&c.footprint))
                .and_then(|f| f.pads.get(&pad_ref.pad));
            if let Some(pad) = pad {
                if pad.drill.is_some() {
                    on = (0..slots.len()).collect();
                } else {
                    on = pad
                        .layers
                        .iter()
                        .filter_map(|l| index_of(*l))
                        .filter_map(slot_of)
                        .collect();
                }
                extent = match pad.shape {
                    PadShape::Circle { diameter } => half_up(diameter.get()),
                    PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
                        let (w, h) = (i128::from(width.get()), i128::from(height.get()));
                        i64::try_from(isqrt_ceil(w * w + h * h) / 2 + 1).unwrap_or(0)
                    }
                };
            }
        }
        Item::Via(id) => {
            if let Some(via) = board.vias.get(&id) {
                for layer in [via.from_layer, via.to_layer] {
                    if let Some(slot) = index_of(layer).and_then(slot_of) {
                        on.push(slot);
                    }
                }
            }
        }
        Item::Track(id) => {
            if let Some(track) = board.tracks.get(&id) {
                on.extend(index_of(track.layer).and_then(slot_of));
            }
        }
        Item::Zone(id) => {
            if let Some(zone) = board.zones.get(&id) {
                on.extend(index_of(zone.layer).and_then(slot_of));
            }
        }
    }
    on.sort_unstable();
    on.dedup();
    Endpoint {
        at: pair(at),
        slots: on,
        extent,
    }
}

fn failure(outcome_net: &NetInfo, conn: &Conn, code: &'static str, detail: String) -> Unrouted {
    Unrouted {
        net: outcome_net.name.clone(),
        from: conn.a_label.clone(),
        to: conn.b_label.clone(),
        code,
        detail,
    }
}

/// Routes the open connections of a board and returns the plan.
pub fn autoroute(
    board: &Board,
    ctx: &RuleContext,
    route: &RouteSettings,
    cfg: &AutorouteConfig,
    request: &Request,
) -> Result<AutoroutePlan, AutorouteError> {
    autoroute_with_pairs(board, ctx, route, cfg, request, &[])
}

/// Routes the open connections of a board like [`autoroute`], and each differential pair of
/// `pairs` as two coupled tracks side by side when there is room for them (one connection for
/// each net, a layer that the four pads share, no change of layer); a pair that cannot be coupled
/// is routed as two ordinary nets and the report says why (`PHOENIX-AROUTE-017`).
pub fn autoroute_with_pairs(
    board: &Board,
    ctx: &RuleContext,
    route: &RouteSettings,
    cfg: &AutorouteConfig,
    request: &Request,
    pairs: &[PairSpec],
) -> Result<AutoroutePlan, AutorouteError> {
    let copper = copper_layers(board);
    if board.outline.len() < 3 || copper.is_empty() {
        return Err(AutorouteError::new(
            BOARD_NOT_ROUTABLE,
            "the board needs a closed outline and a copper layer",
        ));
    }
    let mut slots: Vec<usize> = Vec::new();
    if request.layers.is_empty() {
        slots = (0..copper.len()).collect();
    } else {
        for name in &request.layers {
            let found = copper
                .iter()
                .position(|l| board.layers.get(l).is_some_and(|layer| layer.name == *name));
            match found {
                Some(index) => slots.push(index),
                None => {
                    return Err(AutorouteError::new(
                        LAYER_UNKNOWN,
                        format!("no copper layer named {name}"),
                    ));
                }
            }
        }
        slots.sort_unstable();
        slots.dedup();
    }
    let wanted: Option<BTreeSet<Id>> = if request.nets.is_empty() {
        None
    } else {
        let mut set = BTreeSet::new();
        for name in &request.nets {
            match find_net(board, name) {
                Some(id) => {
                    set.insert(id);
                }
                None => {
                    return Err(AutorouteError::new(
                        NET_UNKNOWN,
                        format!("no net named {name}"),
                    ));
                }
            }
        }
        Some(set)
    };
    let lines =
        ratsnest(board, route).map_err(|e| AutorouteError::new(TOO_MANY_CONNECTIONS, e.detail))?;
    let lines: Vec<_> = lines
        .into_iter()
        .filter(|l| wanted.as_ref().is_none_or(|set| set.contains(&l.net)))
        .collect();
    if lines.len() > cfg.max_connections {
        return Err(AutorouteError::new(
            TOO_MANY_CONNECTIONS,
            format!(
                "{} connections, the limit max_connections is {}",
                lines.len(),
                cfg.max_connections
            ),
        ));
    }
    // the grid
    let polygon = board.outline_polygon(&ArcSettings::current());
    let xs: Vec<i64> = polygon.iter().map(|p| p.x_nm.get()).collect();
    let ys: Vec<i64> = polygon.iter().map(|p| p.y_nm.get()).collect();
    let (min_x, max_x) = (
        xs.iter().copied().min().unwrap_or(0),
        xs.iter().copied().max().unwrap_or(0),
    );
    let (min_y, max_y) = (
        ys.iter().copied().min().unwrap_or(0),
        ys.iter().copied().max().unwrap_or(0),
    );
    let pitch = cfg.grid_nm;
    let ox = min_x.div_euclid(pitch) * pitch;
    let oy = min_y.div_euclid(pitch) * pitch;
    let grid = Grid {
        ox,
        oy,
        pitch,
        nx: (max_x - ox) / pitch + 1,
        ny: (max_y - oy) / pitch + 1,
    };
    let nodes = (grid.nx as u64)
        .saturating_mul(grid.ny as u64)
        .saturating_mul(slots.len() as u64);
    if nodes > cfg.max_nodes {
        return Err(AutorouteError::new(
            GRID_TOO_LARGE,
            format!(
                "{nodes} nodes at {pitch} nm, the limit max_nodes is {}",
                cfg.max_nodes
            ),
        ));
    }
    // the nets
    let rules = &ctx.rules;
    let base_gap = rules
        .min_clearance
        .get()
        .max(rules.connect_tolerance.get() + 1)
        + route.clearance_margin_nm;
    let edge_gap = rules.min_edge_clearance.get().max(base_gap);
    let mut net_gap: BTreeMap<Id, i64> = BTreeMap::new();
    for (id, net) in &board.nets {
        let class = ctx
            .classes
            .class_of(&net.name)
            .map_or(0, |c| c.min_clearance.get());
        if class > base_gap {
            net_gap.insert(*id, class);
        }
    }
    let params = BuildParams {
        base_gap,
        edge_gap,
        zones_block: cfg.zones_block,
        net_gap: &net_gap,
        drill_gap: rules.min_drill_spacing.get(),
    };
    let world = build(board, &copper, &params, pitch * 8)?;
    let mut ids: Vec<Id> = lines.iter().map(|l| l.net).collect();
    ids.sort_unstable();
    ids.dedup();
    let mut nets: Vec<NetInfo> = Vec::new();
    for id in &ids {
        let name = board
            .nets
            .get(id)
            .map(|n| n.name.clone())
            .unwrap_or_default();
        let (drill, diameter) = via_size(ctx, route, &name);
        nets.push(NetInfo {
            id: *id,
            width: track_width(ctx, route, &name),
            gap: net_gap.get(id).copied().unwrap_or(base_gap),
            drill,
            diameter,
            name,
        });
    }
    // the vias of each net that are already on the board and sit on a node of the grid, or within
    // `via_snap_nm` of the nearest node
    let (free_vias, via_off) = index_vias(board, &ids, &grid, &copper, cfg.via_snap_nm);
    // the weight of each net: its own, else the one of its class, else the default
    let weights: Vec<i64> = nets
        .iter()
        .map(|n| {
            cfg.net_weight
                .get(&n.name)
                .copied()
                .or_else(|| {
                    let class = ctx.classes.class_of(&n.name)?;
                    cfg.class_weight.get(&class.name).copied()
                })
                .unwrap_or(DEFAULT_WEIGHT)
        })
        .collect();
    let net_limit: Vec<u64> = nets
        .iter()
        .map(|n| {
            cfg.net_max_expansions
                .get(&n.name)
                .copied()
                .unwrap_or(cfg.max_expansions_per_net)
        })
        .collect();
    let mut conns: Vec<Conn> = Vec::new();
    for line in &lines {
        let net = ids.binary_search(&line.net).unwrap_or(0);
        conns.push(Conn {
            net,
            a: end_of(board, line.from, line.from_at, &slots, &copper),
            b: end_of(board, line.to, line.to_at, &slots, &copper),
            a_label: item_label(board, line.from),
            b_label: item_label(board, line.to),
            len: line.length_nm,
        });
    }
    // the order
    let mut total_len: Vec<i64> = vec![0; nets.len()];
    for conn in &conns {
        total_len[conn.net] += conn.len;
    }
    let key = |net: usize| -> (i64, i64, i64, usize) {
        let info = &nets[net];
        // the nets of more weight go first, whatever the order
        let heavy = -weights[net];
        match cfg.order {
            NetOrder::Shortest => (heavy, 0, total_len[net], net),
            NetOrder::Widest => (heavy, -info.width, total_len[net], net),
            NetOrder::Critical => {
                let first = cfg.critical_nets.contains(&info.name);
                (heavy, i64::from(!first), total_len[net], net)
            }
        }
    };
    let mut order: Vec<usize> = (0..conns.len()).collect();
    order.sort_by_key(|c| {
        let conn = &conns[*c];
        (key(conn.net), conn.len, conn.a.at, conn.b.at, *c)
    });
    // the place of each net in the order, from 1
    let mut net_rank: Vec<usize> = vec![0; nets.len()];
    let mut net_order: Vec<String> = Vec::new();
    for c in &order {
        let n = conns[*c].net;
        if net_rank[n] == 0 {
            net_order.push(nets[n].name.clone());
            net_rank[n] = net_order.len();
        }
    }
    let mut engine = Engine {
        cfg,
        grid,
        slots,
        world,
        nets,
        routed: (0..conns.len()).map(|_| None).collect(),
        conns,
        history: HashMap::new(),
        free_vias,
        via_off,
        drill_gap: rules.min_drill_spacing.get(),
        expansions: 0,
        ripups: 0,
        net_exp: vec![0; weights.len()],
        weights,
        net_limit,
    };
    let mut notes: Vec<String> = Vec::new();
    // a weight or a limit that names a net or a class that is not in this run
    let in_run = |name: &str| engine.nets.iter().any(|n| n.name == name);
    let unused = |what: String| format!("{NET_SETTING_UNUSED}: {what}: no connection to route");
    for name in cfg.net_weight.keys().filter(|k| !in_run(k.as_str())) {
        notes.push(unused(format!("net_weight.{name}")));
    }
    for name in cfg
        .net_max_expansions
        .keys()
        .filter(|k| !in_run(k.as_str()))
    {
        notes.push(unused(format!("net_max_expansions.{name}")));
    }
    for class in cfg.class_weight.keys() {
        let used = engine.nets.iter().any(|n| {
            ctx.classes
                .class_of(&n.name)
                .is_some_and(|c| c.name == *class)
        });
        if !used {
            notes.push(unused(format!("net_class_weight.{class}")));
        }
    }
    // the differential pairs, coupled when there is room
    let mut coupled: Vec<String> = Vec::new();
    for spec in pairs {
        let (Some(p), Some(n)) = (
            find_net(board, &spec.positive),
            find_net(board, &spec.negative),
        ) else {
            return Err(AutorouteError::new(
                NET_UNKNOWN,
                format!("a net of the pair {} does not exist", spec.name),
            ));
        };
        let (Ok(np), Ok(nn)) = (ids.binary_search(&p), ids.binary_search(&n)) else {
            continue;
        };
        let of = |net: usize| -> Vec<usize> {
            (0..engine.conns.len())
                .filter(|c| engine.conns[*c].net == net)
                .collect()
        };
        let (cp, cn) = (of(np), of(nn));
        let uncoupled = |why: &str| format!("{PAIR_UNCOUPLED}: pair {}: {why}", spec.name);
        if cp.len() != 1 || cn.len() != 1 {
            notes.push(uncoupled("each net needs exactly one open connection"));
            continue;
        }
        let before = engine.expansions;
        let verdict = engine.couple(cp[0], cn[0], spec.gap_nm);
        // the nodes the pair took are shared between its two nets
        let spent = engine.expansions - before;
        engine.net_exp[np] += spent - spent / 2;
        engine.net_exp[nn] += spent / 2;
        match verdict {
            Ok(0) => coupled.push(spec.name.clone()),
            Ok(1) => {
                coupled.push(spec.name.clone());
                notes.push(format!(
                    "{PAIR_WITH_VIA}: pair {}: one via for each track",
                    spec.name
                ));
            }
            Ok(changes) => {
                coupled.push(spec.name.clone());
                notes.push(format!(
                    "{PAIR_MANY_LAYERS}: pair {}: {changes} changes of layer, one via for each track at each",
                    spec.name
                ));
            }
            Err(why) => notes.push(uncoupled(&why)),
        }
    }
    let mut failures: BTreeMap<usize, (&'static str, String)> = BTreeMap::new();
    let mut rips: Vec<u32> = vec![0; engine.conns.len()];
    let mut queue: VecDeque<usize> = order.iter().copied().collect();
    while let Some(c) = queue.pop_front() {
        if engine.routed[c].is_some() || failures.contains_key(&c) {
            continue;
        }
        if engine.expansions >= cfg.max_expansions_total {
            failures.insert(
                c,
                (
                    EXPANSION_LIMIT,
                    "the limit of expanded nodes of the run was reached".to_owned(),
                ),
            );
            continue;
        }
        let owner = engine.conns[c].net;
        if engine.net_limit[owner] > 0 && engine.net_exp[owner] >= engine.net_limit[owner] {
            let limit = engine.net_limit[owner];
            failures.insert(
                c,
                (
                    NET_LIMIT,
                    format!("the net reached its limit of {limit} expanded nodes"),
                ),
            );
            continue;
        }
        match engine.attempt(c, false) {
            Attempt::Found(found, sources, targets) => {
                let assembled = assemble(
                    &found,
                    &sources,
                    &targets,
                    &engine.slots,
                    &engine.grid,
                    &engine.free_vias[engine.conns[c].net],
                    &engine.via_off[engine.conns[c].net],
                );
                engine.commit(c, assembled);
            }
            Attempt::Missed(code, detail) => {
                let may_rip = code != NO_TERMINAL
                    && code != NET_LIMIT
                    && engine.ripups < cfg.max_ripup_iterations
                    && rips[c] < cfg.max_rips_per_connection;
                if !may_rip {
                    let (code, detail) = if code != NO_TERMINAL
                        && code != NET_LIMIT
                        && (engine.ripups >= cfg.max_ripup_iterations
                            || rips[c] >= cfg.max_rips_per_connection)
                    {
                        (RIPUP_LIMIT, format!("{detail}; rip-up is at its limit"))
                    } else {
                        (code, detail)
                    };
                    failures.insert(c, (code, detail));
                    continue;
                }
                rips[c] += 1;
                match engine.attempt(c, true) {
                    Attempt::Found(found, sources, targets) => {
                        let assembled = assemble(
                            &found,
                            &sources,
                            &targets,
                            &engine.slots,
                            &engine.grid,
                            &engine.free_vias[engine.conns[c].net],
                            &engine.via_off[engine.conns[c].net],
                        );
                        let crossed = engine.crossing(c, &assembled);
                        if !crossed.is_empty() {
                            engine.ripups += 1;
                            let victims: Vec<usize> = (0..engine.conns.len())
                                .filter(|k| {
                                    engine.routed[*k].is_some()
                                        && *k != c
                                        && crossed.contains(&engine.nets[engine.conns[*k].net].id)
                                })
                                .collect();
                            for k in victims {
                                engine.rip(k);
                                queue.push_back(k);
                            }
                        }
                        engine.commit(c, assembled);
                    }
                    Attempt::Missed(code, detail) => {
                        failures.insert(c, (code, detail));
                    }
                }
            }
        }
    }
    // the result, net by net
    let mut outcomes: Vec<NetOutcome> = Vec::new();
    let mut unrouted: Vec<Unrouted> = Vec::new();
    for (n, info) in engine.nets.iter().enumerate() {
        let mut outcome = NetOutcome {
            name: info.name.clone(),
            total: 0,
            routed: Vec::new(),
            tracks: Vec::new(),
            vias: Vec::new(),
        };
        let mut seen: BTreeSet<(i64, i64, usize, usize)> = BTreeSet::new();
        let net_order: Vec<usize> = order
            .iter()
            .copied()
            .filter(|c| engine.conns[*c].net == n)
            .collect();
        for c in net_order {
            let conn = &engine.conns[c];
            outcome.total += 1;
            let Some(routed) = &engine.routed[c] else {
                let (code, detail) = failures
                    .get(&c)
                    .cloned()
                    .unwrap_or((NO_PATH, "not routed".to_owned()));
                unrouted.push(failure(info, conn, code, detail));
                continue;
            };
            outcome
                .routed
                .push((conn.a_label.clone(), conn.b_label.clone()));
            for run in &routed.assembled.runs {
                let points: Vec<Point> = run
                    .pts
                    .iter()
                    .filter_map(|p| Point::from_nm(p.0, p.1).ok())
                    .collect();
                if points.len() >= 2 {
                    outcome.tracks.push(PlannedTrack {
                        net: info.id,
                        layer: copper[run.layer],
                        width_nm: info.width,
                        points,
                    });
                }
            }
            for v in &routed.assembled.vias {
                if !seen.insert((v.at.0, v.at.1, v.lo, v.hi)) {
                    continue;
                }
                if let Ok(at) = Point::from_nm(v.at.0, v.at.1) {
                    outcome.vias.push(PlannedVia {
                        net: info.id,
                        at,
                        from_layer: copper[v.lo],
                        to_layer: copper[v.hi],
                        drill_nm: info.drill,
                        diameter_nm: info.diameter,
                    });
                }
            }
        }
        outcomes.push(outcome);
    }
    let limited: BTreeSet<usize> = failures
        .iter()
        .filter(|(_, (code, _))| *code == NET_LIMIT)
        .map(|(c, _)| engine.conns[*c].net)
        .collect();
    for n in limited {
        notes.push(format!(
            "{NET_LIMIT}: net {}: the limit of {} expanded nodes was reached",
            engine.nets[n].name, engine.net_limit[n]
        ));
    }
    let stats: Vec<NetStats> = engine
        .nets
        .iter()
        .enumerate()
        .map(|(n, info)| NetStats {
            name: info.name.clone(),
            weight_percent: engine.weights[n],
            order: net_rank[n],
            expansions: engine.net_exp[n],
            rips: (0..engine.conns.len())
                .filter(|c| engine.conns[*c].net == n)
                .map(|c| rips[c])
                .sum(),
            corners: 0,
            max_expansions: engine.net_limit[n],
        })
        .collect();
    let snapped: usize = engine
        .routed
        .iter()
        .flatten()
        .map(|r| r.assembled.snapped)
        .sum();
    if snapped > 0 {
        notes.push(format!(
            "{VIA_SNAPPED}: {snapped} existing via(s) off the grid joined to a node"
        ));
    }
    finish(
        board,
        ctx,
        route,
        cfg.diagonal,
        outcomes,
        unrouted,
        &Totals {
            ripups: engine.ripups,
            expansions: engine.expansions,
            notes,
            pairs_coupled: coupled,
            stats,
            net_order,
        },
    )
}
