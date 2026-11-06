// SPDX-License-Identifier: GPL-3.0-or-later
//! The reader of DXF in ASCII. It understands `LINE`, `ARC`, `CIRCLE`, `POINT`, `LWPOLYLINE`,
//! `POLYLINE` with `VERTEX` (bulges too), `SPLINE` (cut into straight pieces within a tolerance,
//! with a finding), `INSERT` of blocks (scale, turn, nested blocks to a depth), the layers and the
//! unit of `$INSUNITS`. Whatever else a file holds is counted and listed in the report with its
//! layers; nothing is left out without a finding. The limits (bytes, entities, blocks, depth,
//! points of a spline) come from the settings, and a file that goes past one is refused.

use std::collections::{BTreeMap, BTreeSet};

use crate::codes::{
    DxfError, Finding, Report, Severity, BLOCK_UNUSABLE, DXF_INVALID, ENTITY_UNSUPPORTED,
    LIMIT_EXCEEDED, SPLINE_APPROXIMATED, UNIT_UNKNOWN,
};
use crate::geom::{bulge_to_sweep, Seg, P};
use crate::model::{Drawing, Entity, Shape};
use crate::settings::DxfSettings;
use crate::spline::{sample, SplineData};

/// What the reader makes of a file.
#[derive(Debug, Clone, PartialEq)]
pub struct Read {
    /// The entities that were understood.
    pub drawing: Drawing,
    /// What was left out, approximated or worth knowing.
    pub report: Report,
    /// How many entities of each type the file holds (the entities of the blocks are counted
    /// once, where the block is defined).
    pub counts: BTreeMap<String, usize>,
    /// Millimetres in one unit of the file.
    pub unit_mm: f64,
}

struct Pair {
    code: i32,
    value: String,
    line: usize,
}

struct Raw {
    kind: String,
    data: Vec<Pair>,
}

/// A shape in the units of the file, before the blocks are put in place.
enum Prim {
    /// Vertices with the bulge of the piece that starts at each.
    Path {
        pts: Vec<(f64, f64, f64)>,
        closed: bool,
    },
    Circle {
        centre: (f64, f64),
        radius: f64,
    },
    Point {
        at: (f64, f64),
    },
}

struct Insert {
    layer: String,
    block: String,
    at: (f64, f64),
    scale: (f64, f64),
    turn_deg: f64,
}

enum Node {
    Item { layer: String, prim: Prim },
    Insert(Insert),
}

struct Block {
    base: (f64, f64),
    nodes: Vec<Node>,
}

fn invalid(line: usize, what: &str) -> DxfError {
    DxfError::new(DXF_INVALID, format!("line {line}: {what}"))
}

fn parse_pairs(text: &str) -> Result<Vec<Pair>, DxfError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return Err(invalid(1, "the file is empty"));
    }
    if !lines.len().is_multiple_of(2) {
        return Err(invalid(
            lines.len(),
            "a group code has no value: the number of lines is odd",
        ));
    }
    let mut pairs = Vec::with_capacity(lines.len() / 2);
    for (n, (code_line, value)) in lines
        .iter()
        .step_by(2)
        .zip(lines.iter().skip(1).step_by(2))
        .enumerate()
    {
        let line = 2 * n + 1;
        let code: i32 = code_line
            .trim()
            .parse()
            .map_err(|_| invalid(line, "a group code is not a number"))?;
        pairs.push(Pair {
            code,
            value: value.trim().to_owned(),
            line: line + 1,
        });
    }
    Ok(pairs)
}

fn parse_real(pair: &Pair) -> Result<f64, DxfError> {
    match pair.value.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => Err(invalid(
            pair.line,
            &format!("`{}` is not a number", pair.value),
        )),
    }
}

fn real(data: &[Pair], code: i32) -> Result<Option<f64>, DxfError> {
    match data.iter().find(|p| p.code == code) {
        None => Ok(None),
        Some(pair) => parse_real(pair).map(Some),
    }
}

fn integer(data: &[Pair], code: i32) -> Result<Option<i64>, DxfError> {
    match data.iter().find(|p| p.code == code) {
        None => Ok(None),
        Some(pair) => pair
            .value
            .parse::<i64>()
            .map(Some)
            .map_err(|_| invalid(pair.line, &format!("`{}` is not an integer", pair.value))),
    }
}

fn text_of(data: &[Pair], code: i32) -> Option<&str> {
    data.iter()
        .find(|p| p.code == code)
        .map(|p| p.value.as_str())
}

/// The entities from `*pos`, up to one of the kinds in `ends` (which is not consumed).
fn collect_raws(pairs: &[Pair], pos: &mut usize, ends: &[&str]) -> Vec<Raw> {
    let mut raws = Vec::new();
    while *pos < pairs.len() {
        let head = &pairs[*pos];
        if head.code != 0 {
            *pos += 1;
            continue;
        }
        if ends.contains(&head.value.as_str()) {
            break;
        }
        let kind = head.value.clone();
        *pos += 1;
        let start = *pos;
        while *pos < pairs.len() && pairs[*pos].code != 0 {
            *pos += 1;
        }
        let data: Vec<Pair> = pairs[start..*pos]
            .iter()
            .map(|p| Pair {
                code: p.code,
                value: p.value.clone(),
                line: p.line,
            })
            .collect();
        raws.push(Raw { kind, data });
    }
    raws
}

/// Millimetres in a unit of `$INSUNITS`, when the number is one this reader knows.
fn unit_millimetres(code: i64) -> Option<f64> {
    Some(match code {
        1 => 25.4,
        2 => 304.8,
        3 => 1_609_344.0,
        4 => 1.0,
        5 => 10.0,
        6 => 1_000.0,
        7 => 1_000_000.0,
        8 => 0.000_025_4,
        9 => 0.025_4,
        10 => 914.4,
        13 => 0.001,
        14 => 100.0,
        _ => return None,
    })
}

/// The whole of what the conversion of entities counts.
#[derive(Default)]
struct Stats {
    counts: BTreeMap<String, usize>,
    unsupported: BTreeMap<String, (usize, BTreeSet<String>)>,
    splines: usize,
    splines_by_fit_points: usize,
}

impl Stats {
    fn skip(&mut self, kind: &str, layer: &str) {
        let entry = self.unsupported.entry(kind.to_owned()).or_default();
        entry.0 += 1;
        entry.1.insert(layer.to_owned());
    }
}

struct Converter<'a> {
    settings: &'a DxfSettings,
    unit_nm: f64,
    stats: Stats,
}

impl Converter<'_> {
    fn chain_nodes(&mut self, raws: &[Raw]) -> Result<Vec<Node>, DxfError> {
        let mut nodes = Vec::new();
        let mut i = 0;
        while i < raws.len() {
            let raw = &raws[i];
            i += 1;
            let kind = raw.kind.as_str();
            *self.stats.counts.entry(kind.to_owned()).or_default() += 1;
            let layer = text_of(&raw.data, 8).unwrap_or("0").to_owned();
            let prim = match kind {
                "LINE" => Some(line_prim(&raw.data)?),
                "CIRCLE" => Some(Prim::Circle {
                    centre: centre_of(&raw.data)?,
                    radius: real(&raw.data, 40)?.unwrap_or(0.0),
                }),
                "ARC" => Some(arc_prim(&raw.data)?),
                "POINT" => Some(Prim::Point {
                    at: centre_of(&raw.data)?,
                }),
                "LWPOLYLINE" => Some(lw_prim(&raw.data)?),
                "POLYLINE" => {
                    let mut vertices: Vec<&Raw> = Vec::new();
                    while i < raws.len() && raws[i].kind == "VERTEX" {
                        vertices.push(&raws[i]);
                        i += 1;
                    }
                    if i < raws.len() && raws[i].kind == "SEQEND" {
                        i += 1;
                    }
                    self.polyline_prim(&raw.data, &vertices, &layer)?
                }
                "SPLINE" => self.spline_prim(&raw.data, &layer)?,
                "INSERT" => {
                    nodes.push(Node::Insert(insert_of(&raw.data, layer)?));
                    continue;
                }
                other => {
                    self.stats.skip(other, &layer);
                    None
                }
            };
            if let Some(prim) = prim {
                nodes.push(Node::Item { layer, prim });
            }
        }
        Ok(nodes)
    }

    fn polyline_prim(
        &mut self,
        header: &[Pair],
        vertices: &[&Raw],
        layer: &str,
    ) -> Result<Option<Prim>, DxfError> {
        let flags = integer(header, 70)?.unwrap_or(0);
        if flags & (8 | 16 | 64) != 0 {
            let what = if flags & 8 != 0 {
                "POLYLINE (3D)"
            } else {
                "POLYLINE (mesh)"
            };
            self.stats.skip(what, layer);
            return Ok(None);
        }
        let mut pts = Vec::new();
        for vertex in vertices {
            // a vertex of the frame of a spline-fit polyline is not on the curve
            if integer(&vertex.data, 70)?.unwrap_or(0) & 16 != 0 {
                continue;
            }
            pts.push((
                real(&vertex.data, 10)?.unwrap_or(0.0),
                real(&vertex.data, 20)?.unwrap_or(0.0),
                real(&vertex.data, 42)?.unwrap_or(0.0),
            ));
        }
        Ok(Some(Prim::Path {
            pts,
            closed: flags & 1 != 0,
        }))
    }

    fn spline_prim(&mut self, data: &[Pair], layer: &str) -> Result<Option<Prim>, DxfError> {
        let flags = integer(data, 70)?.unwrap_or(0);
        let degree = integer(data, 71)?.unwrap_or(3);
        let mut knots = Vec::new();
        let mut weights = Vec::new();
        let mut control: Vec<(f64, f64)> = Vec::new();
        let mut fit: Vec<(f64, f64)> = Vec::new();
        for pair in data {
            match pair.code {
                40 => knots.push(parse_real(pair)?),
                41 => weights.push(parse_real(pair)?),
                10 => control.push((parse_real(pair)?, 0.0)),
                20 => {
                    if let Some(last) = control.last_mut() {
                        last.1 = parse_real(pair)?;
                    }
                }
                11 => fit.push((parse_real(pair)?, 0.0)),
                21 => {
                    if let Some(last) = fit.last_mut() {
                        last.1 = parse_real(pair)?;
                    }
                }
                _ => {}
            }
        }
        let closed = flags & 1 != 0;
        let tolerance = self.settings.spline_tolerance_nm as f64 / self.unit_nm;
        // the weights are used only when there is one for each control point
        let weights = if weights.len() == control.len() {
            weights
        } else {
            Vec::new()
        };
        let spline = SplineData {
            degree: usize::try_from(degree).unwrap_or(0),
            knots,
            control,
            weights,
        };
        if !spline.control.is_empty() {
            if let Some(points) = sample(&spline, tolerance, self.settings.max_spline_points) {
                self.stats.splines += 1;
                return Ok(Some(Prim::Path {
                    pts: points.into_iter().map(|p| (p.0, p.1, 0.0)).collect(),
                    closed,
                }));
            }
        }
        if fit.len() >= 2 {
            self.stats.splines_by_fit_points += 1;
            return Ok(Some(Prim::Path {
                pts: fit.into_iter().map(|p| (p.0, p.1, 0.0)).collect(),
                closed,
            }));
        }
        self.stats.skip("SPLINE (cannot be evaluated)", layer);
        Ok(None)
    }
}

fn centre_of(data: &[Pair]) -> Result<(f64, f64), DxfError> {
    Ok((
        real(data, 10)?.unwrap_or(0.0),
        real(data, 20)?.unwrap_or(0.0),
    ))
}

fn line_prim(data: &[Pair]) -> Result<Prim, DxfError> {
    Ok(Prim::Path {
        pts: vec![
            (
                real(data, 10)?.unwrap_or(0.0),
                real(data, 20)?.unwrap_or(0.0),
                0.0,
            ),
            (
                real(data, 11)?.unwrap_or(0.0),
                real(data, 21)?.unwrap_or(0.0),
                0.0,
            ),
        ],
        closed: false,
    })
}

fn arc_prim(data: &[Pair]) -> Result<Prim, DxfError> {
    let centre = centre_of(data)?;
    let radius = real(data, 40)?.unwrap_or(0.0);
    let start = real(data, 50)?.unwrap_or(0.0);
    let end = real(data, 51)?.unwrap_or(0.0);
    let sweep = (end - start).rem_euclid(360.0);
    if sweep == 0.0 {
        return Ok(Prim::Circle { centre, radius });
    }
    let at = |deg: f64| {
        let rad = deg.to_radians();
        (centre.0 + radius * rad.cos(), centre.1 + radius * rad.sin())
    };
    let (a, b) = (at(start), at(end));
    Ok(Prim::Path {
        pts: vec![
            (a.0, a.1, (sweep.to_radians() / 4.0).tan()),
            (b.0, b.1, 0.0),
        ],
        closed: false,
    })
}

fn lw_prim(data: &[Pair]) -> Result<Prim, DxfError> {
    let mut pts: Vec<(f64, f64, f64)> = Vec::new();
    for pair in data {
        match pair.code {
            10 => pts.push((parse_real(pair)?, 0.0, 0.0)),
            20 => {
                if let Some(last) = pts.last_mut() {
                    last.1 = parse_real(pair)?;
                }
            }
            42 => {
                if let Some(last) = pts.last_mut() {
                    last.2 = parse_real(pair)?;
                }
            }
            _ => {}
        }
    }
    Ok(Prim::Path {
        pts,
        closed: integer(data, 70)?.unwrap_or(0) & 1 != 0,
    })
}

fn insert_of(data: &[Pair], layer: String) -> Result<Insert, DxfError> {
    let rows = integer(data, 71)?.unwrap_or(1);
    let columns = integer(data, 70)?.unwrap_or(1);
    Ok(Insert {
        layer,
        block: text_of(data, 2).unwrap_or("").to_uppercase(),
        at: centre_of(data)?,
        scale: (
            real(data, 41)?.unwrap_or(1.0),
            real(data, 42)?.unwrap_or(1.0),
        ),
        turn_deg: if rows > 1 || columns > 1 {
            f64::NAN
        } else {
            real(data, 50)?.unwrap_or(0.0)
        },
    })
}

/// A transformation of the plane: `x' = a x + b y + e`, `y' = c x + d y + f`.
#[derive(Clone, Copy)]
struct Affine {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Affine {
    fn identity() -> Affine {
        Affine {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    fn apply(&self, p: (f64, f64)) -> (f64, f64) {
        (
            self.a * p.0 + self.b * p.1 + self.e,
            self.c * p.0 + self.d * p.1 + self.f,
        )
    }

    /// The transformation that does `inner` first and `self` after it.
    fn after(&self, inner: &Affine) -> Affine {
        Affine {
            a: self.a * inner.a + self.b * inner.c,
            b: self.a * inner.b + self.b * inner.d,
            c: self.c * inner.a + self.d * inner.c,
            d: self.c * inner.b + self.d * inner.d,
            e: self.a * inner.e + self.b * inner.f + self.e,
            f: self.c * inner.e + self.d * inner.f + self.f,
        }
    }

    fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// True for a turn, a mirror and one scale in both directions: circles stay circles.
    fn is_similarity(&self) -> bool {
        let sx = self.a.hypot(self.c);
        let sy = self.b.hypot(self.d);
        let top = sx.max(sy);
        top > 0.0
            && (sx - sy).abs() <= 1e-9 * top
            && (self.a * self.b + self.c * self.d).abs() <= 1e-9 * top * top
    }
}

struct Expander<'a> {
    blocks: &'a BTreeMap<String, Block>,
    settings: &'a DxfSettings,
    unit_nm: f64,
    entities: Vec<Entity>,
    inserts: usize,
    report: &'a mut Report,
    told: BTreeSet<String>,
}

fn is_head(pair: &Pair, kind: &str) -> bool {
    pair.code == 0 && pair.value == kind
}

fn limit(what: &str, value: usize) -> DxfError {
    DxfError::new(LIMIT_EXCEEDED, format!("{what}: the limit is {value}"))
}

impl Expander<'_> {
    fn tell(&mut self, subject: &str, detail: String) {
        if self.told.insert(format!("{subject}|{detail}")) {
            self.report.push(Finding::new(
                BLOCK_UNUSABLE,
                Severity::Warning,
                Some(subject),
                detail,
            ));
        }
    }

    fn nm(&self, value: f64) -> Option<i64> {
        let x = (value * self.unit_nm).round();
        (x.is_finite() && x.abs() < 1_073_741_823.0).then_some(x as i64)
    }

    fn point(&self, p: (f64, f64)) -> Option<P> {
        Some((self.nm(p.0)?, self.nm(p.1)?))
    }

    fn push(&mut self, layer: &str, shape: Shape) -> Result<(), DxfError> {
        if self.entities.len() >= self.settings.max_entities {
            return Err(limit("entities", self.settings.max_entities));
        }
        self.entities.push(Entity {
            layer: layer.to_owned(),
            shape,
        });
        Ok(())
    }

    fn out_of_range(&mut self, layer: &str) {
        self.report.push(Finding::new(
            ENTITY_UNSUPPORTED,
            Severity::Warning,
            Some(layer),
            "an entity with a coordinate out of the range of the design was left out",
        ));
    }

    fn emit(&mut self, layer: &str, prim: &Prim, m: &Affine) -> Result<(), DxfError> {
        match prim {
            Prim::Path { pts, closed } => {
                if pts.len() < 2 {
                    self.report.push(Finding::new(
                        ENTITY_UNSUPPORTED,
                        Severity::Warning,
                        Some(layer),
                        "a polyline with fewer than two vertices was left out",
                    ));
                    return Ok(());
                }
                if pts.iter().any(|p| p.2 != 0.0) && !m.is_similarity() {
                    self.tell(
                        layer,
                        "an arc inside a block that is scaled differently along x and y was left out"
                            .to_owned(),
                    );
                    return Ok(());
                }
                let flip = m.determinant() < 0.0;
                let mut mapped: Vec<(P, i64)> = Vec::with_capacity(pts.len());
                for p in pts {
                    let q = m.apply((p.0, p.1));
                    let Some(point) = self.point(q) else {
                        self.out_of_range(layer);
                        return Ok(());
                    };
                    let bulge = if flip { -p.2 } else { p.2 };
                    mapped.push((
                        point,
                        if bulge == 0.0 {
                            0
                        } else {
                            bulge_to_sweep(bulge)
                        },
                    ));
                }
                let count = if *closed {
                    mapped.len()
                } else {
                    mapped.len() - 1
                };
                let segs: Vec<Seg> = (0..count)
                    .map(|i| Seg {
                        a: mapped[i].0,
                        b: mapped[(i + 1) % mapped.len()].0,
                        sweep_udeg: mapped[i].1,
                    })
                    .collect();
                self.push(
                    layer,
                    Shape::Path {
                        segs,
                        closed: *closed,
                    },
                )
            }
            Prim::Circle { centre, radius } => {
                if !m.is_similarity() {
                    self.tell(
                        layer,
                        "a circle inside a block that is scaled differently along x and y was left out"
                            .to_owned(),
                    );
                    return Ok(());
                }
                let scale = m.determinant().abs().sqrt();
                let c = self.point(m.apply(*centre));
                let r = self.nm(radius.abs() * scale);
                match (c, r) {
                    (Some(centre), Some(radius)) if radius > 0 => {
                        self.push(layer, Shape::Circle { centre, radius })
                    }
                    (Some(_), Some(_)) => {
                        self.report.push(Finding::new(
                            ENTITY_UNSUPPORTED,
                            Severity::Warning,
                            Some(layer),
                            "a circle without a radius was left out",
                        ));
                        Ok(())
                    }
                    _ => {
                        self.out_of_range(layer);
                        Ok(())
                    }
                }
            }
            Prim::Point { at } => match self.point(m.apply(*at)) {
                Some(at) => self.push(layer, Shape::Point { at }),
                None => {
                    self.out_of_range(layer);
                    Ok(())
                }
            },
        }
    }

    fn expand(
        &mut self,
        nodes: &[Node],
        m: &Affine,
        layer: Option<&str>,
        depth: usize,
        stack: &mut Vec<String>,
    ) -> Result<(), DxfError> {
        for node in nodes {
            match node {
                Node::Item { layer: own, prim } => {
                    let chosen = match layer {
                        Some(outer) if own == "0" => outer,
                        _ => own.as_str(),
                    };
                    self.emit(chosen, prim, m)?;
                }
                Node::Insert(insert) => {
                    self.inserts += 1;
                    if self.inserts > self.settings.max_entities {
                        return Err(limit("inserts", self.settings.max_entities));
                    }
                    self.expand_insert(insert, m, layer, depth, stack)?;
                }
            }
        }
        Ok(())
    }

    fn expand_insert(
        &mut self,
        insert: &Insert,
        m: &Affine,
        layer: Option<&str>,
        depth: usize,
        stack: &mut Vec<String>,
    ) -> Result<(), DxfError> {
        let name = insert.block.as_str();
        if insert.turn_deg.is_nan() {
            self.tell(
                name,
                "an array of inserts (several rows or columns) was left out".to_owned(),
            );
            return Ok(());
        }
        let blocks = self.blocks;
        let Some(block) = blocks.get(name) else {
            self.tell(name, "the block is not defined".to_owned());
            return Ok(());
        };
        if stack.iter().any(|n| n == name) {
            self.tell(name, "the block holds itself".to_owned());
            return Ok(());
        }
        if depth + 1 > self.settings.max_block_depth {
            return Err(limit("depth of blocks", self.settings.max_block_depth));
        }
        let (sx, sy) = insert.scale;
        if sx == 0.0 || sy == 0.0 {
            self.tell(name, "the insert has a scale of 0".to_owned());
            return Ok(());
        }
        let (sin, cos) = insert.turn_deg.to_radians().sin_cos();
        let (a, b, c, d) = (cos * sx, -sin * sy, sin * sx, cos * sy);
        let placed = Affine {
            a,
            b,
            c,
            d,
            e: insert.at.0 - (a * block.base.0 + b * block.base.1),
            f: insert.at.1 - (c * block.base.0 + d * block.base.1),
        };
        let total = m.after(&placed);
        let own_layer = if insert.layer == "0" {
            layer.map(str::to_owned)
        } else {
            Some(insert.layer.clone())
        };
        stack.push(name.to_owned());
        let result = self.expand(&block.nodes, &total, own_layer.as_deref(), depth + 1, stack);
        stack.pop();
        result
    }
}

/// The unit of the file in nanometres, with the finding that says how it was chosen.
fn unit_of(pairs: &[Pair], settings: &DxfSettings, report: &mut Report) -> f64 {
    let default = settings.default_unit.millimetres();
    let found = pairs
        .iter()
        .enumerate()
        .find(|(_, p)| p.code == 9 && p.value.eq_ignore_ascii_case("$INSUNITS"))
        .and_then(|(i, _)| pairs.get(i + 1))
        .filter(|p| p.code == 70)
        .and_then(|p| p.value.parse::<i64>().ok());
    let millimetres = match found {
        Some(code) if code != 0 => match unit_millimetres(code) {
            Some(mm) => mm,
            None => {
                let subject = format!("$INSUNITS = {code}");
                report.push(Finding::new(
                    UNIT_UNKNOWN,
                    Severity::Warning,
                    Some(subject.as_str()),
                    format!("read in {default} mm"),
                ));
                default
            }
        },
        Some(_) => {
            report.push(Finding::new(
                UNIT_UNKNOWN,
                Severity::Info,
                Some("$INSUNITS = 0"),
                format!("the file says the drawing has no unit; read in {default} mm"),
            ));
            default
        }
        None => {
            report.push(Finding::new(
                UNIT_UNKNOWN,
                Severity::Info,
                None,
                format!("the file does not say its unit; read in {default} mm"),
            ));
            default
        }
    };
    millimetres * 1_000_000.0
}

/// Reads the text of a DXF file.
pub fn read_dxf(text: &str, settings: &DxfSettings) -> Result<Read, DxfError> {
    if text.len() as u64 > settings.max_file_bytes {
        return Err(DxfError::new(
            LIMIT_EXCEEDED,
            format!(
                "the file has {} bytes, the limit is {}",
                text.len(),
                settings.max_file_bytes
            ),
        ));
    }
    let pairs = parse_pairs(text)?;
    let mut report = Report::new(settings.max_findings);
    let unit_nm = unit_of(&pairs, settings, &mut report);
    let mut conv = Converter {
        settings,
        unit_nm,
        stats: Stats::default(),
    };

    let mut blocks: BTreeMap<String, Block> = BTreeMap::new();
    let mut top: Option<Vec<Node>> = None;
    let mut sections = 0;
    let mut i = 0;
    while i < pairs.len() {
        if !is_head(&pairs[i], "SECTION") {
            i += 1;
            continue;
        }
        sections += 1;
        let name = match pairs.get(i + 1) {
            Some(p) if p.code == 2 => p.value.to_uppercase(),
            _ => return Err(invalid(pairs[i].line, "a SECTION has no name")),
        };
        i += 2;
        match name.as_str() {
            "ENTITIES" => {
                let raws = collect_raws(&pairs, &mut i, &["ENDSEC"]);
                top = Some(conv.chain_nodes(&raws)?);
            }
            "BLOCKS" => {
                while i < pairs.len() && !is_head(&pairs[i], "ENDSEC") {
                    if !is_head(&pairs[i], "BLOCK") {
                        i += 1;
                        continue;
                    }
                    i += 1;
                    let start = i;
                    while i < pairs.len() && pairs[i].code != 0 {
                        i += 1;
                    }
                    let head: Vec<Pair> = pairs[start..i]
                        .iter()
                        .map(|p| Pair {
                            code: p.code,
                            value: p.value.clone(),
                            line: p.line,
                        })
                        .collect();
                    let raws = collect_raws(&pairs, &mut i, &["ENDBLK", "ENDSEC"]);
                    let block_name = text_of(&head, 2).unwrap_or("").to_uppercase();
                    if blocks.len() >= settings.max_blocks {
                        return Err(limit("blocks", settings.max_blocks));
                    }
                    let base = (
                        real(&head, 10)?.unwrap_or(0.0),
                        real(&head, 20)?.unwrap_or(0.0),
                    );
                    blocks.insert(
                        block_name,
                        Block {
                            base,
                            nodes: conv.chain_nodes(&raws)?,
                        },
                    );
                }
            }
            _ => {}
        }
    }
    if sections == 0 {
        return Err(invalid(1, "there is no SECTION: this is not a DXF file"));
    }
    let Some(top) = top else {
        return Err(DxfError::new(DXF_INVALID, "there is no ENTITIES section"));
    };

    let entities = {
        let mut expander = Expander {
            blocks: &blocks,
            settings,
            unit_nm,
            entities: Vec::new(),
            inserts: 0,
            report: &mut report,
            told: BTreeSet::new(),
        };
        expander.expand(&top, &Affine::identity(), None, 0, &mut Vec::new())?;
        expander.entities
    };

    let stats = conv.stats;
    for (kind, (count, layers)) in &stats.unsupported {
        let layers: Vec<&str> = layers.iter().map(String::as_str).collect();
        report.push(Finding::new(
            ENTITY_UNSUPPORTED,
            Severity::Warning,
            Some(kind),
            format!("{count} left out, on the layers {}", layers.join(", ")),
        ));
    }
    if stats.splines > 0 {
        report.push(Finding::new(
            SPLINE_APPROXIMATED,
            Severity::Warning,
            Some("SPLINE"),
            format!(
                "{} cut into straight pieces within {} nm",
                stats.splines, settings.spline_tolerance_nm
            ),
        ));
    }
    if stats.splines_by_fit_points > 0 {
        report.push(Finding::new(
            SPLINE_APPROXIMATED,
            Severity::Warning,
            Some("SPLINE"),
            format!(
                "{} joined through their fit points by straight pieces: the curve between the points is not the spline",
                stats.splines_by_fit_points
            ),
        ));
    }
    Ok(Read {
        drawing: Drawing { entities },
        report,
        counts: stats.counts,
        unit_mm: unit_nm / 1_000_000.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;

    /// The text of a file from groups: the header, the blocks and the entities.
    fn file(header: &[(i32, &str)], blocks: &[(i32, &str)], entities: &[(i32, &str)]) -> String {
        fn section(out: &mut String, name: &str, groups: &[(i32, &str)]) {
            out.push_str(&format!("  0\nSECTION\n  2\n{name}\n"));
            for (code, value) in groups {
                out.push_str(&format!("{code:>3}\n{value}\n"));
            }
            out.push_str("  0\nENDSEC\n");
        }
        let mut out = String::new();
        section(&mut out, "HEADER", header);
        section(&mut out, "BLOCKS", blocks);
        section(&mut out, "ENTITIES", entities);
        out.push_str("  0\nEOF\n");
        out
    }

    fn mm_header() -> Vec<(i32, &'static str)> {
        vec![(9, "$INSUNITS"), (70, "4")]
    }

    fn read(entities: &[(i32, &str)]) -> Read {
        read_dxf(&file(&mm_header(), &[], entities), &DxfSettings::default()).unwrap()
    }

    fn only_path(read: &Read) -> (Vec<Seg>, bool) {
        match &read.drawing.entities[0].shape {
            Shape::Path { segs, closed } => (segs.clone(), *closed),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_line_is_read_in_millimetres_on_its_layer() {
        let r = read(&[
            (0, "LINE"),
            (8, "EDGE"),
            (10, "1.5"),
            (20, "2"),
            (11, "10"),
            (21, "2"),
        ]);
        assert_eq!(r.drawing.entities[0].layer, "EDGE");
        let (segs, closed) = only_path(&r);
        assert!(!closed);
        assert_eq!(
            segs,
            vec![Seg::line((1_500_000, 2 * MM), (10 * MM, 2 * MM))]
        );
        assert_eq!(r.counts.get("LINE"), Some(&1));
        assert_eq!(r.unit_mm, 1.0);
        assert!(r.report.findings.is_empty());
    }

    #[test]
    fn an_arc_is_a_piece_with_its_sweep_and_a_full_turn_is_a_circle() {
        let r = read(&[
            (0, "ARC"),
            (8, "0"),
            (10, "0"),
            (20, "0"),
            (40, "10"),
            (50, "0"),
            (51, "90"),
        ]);
        let (segs, _) = only_path(&r);
        assert_eq!(
            segs,
            vec![Seg {
                a: (10 * MM, 0),
                b: (0, 10 * MM),
                sweep_udeg: 90_000_000
            }]
        );
        // from 270 to 90 degrees: a half turn through 0
        let r = read(&[
            (0, "ARC"),
            (10, "0"),
            (20, "0"),
            (40, "10"),
            (50, "270"),
            (51, "90"),
        ]);
        let (segs, _) = only_path(&r);
        assert_eq!(segs[0].sweep_udeg, 180_000_000);
        assert_eq!(segs[0].a, (0, -10 * MM));
        assert_eq!(segs[0].b, (0, 10 * MM));
        let r = read(&[
            (0, "ARC"),
            (10, "1"),
            (20, "2"),
            (40, "3"),
            (50, "10"),
            (51, "10"),
        ]);
        assert_eq!(
            r.drawing.entities[0].shape,
            Shape::Circle {
                centre: (MM, 2 * MM),
                radius: 3 * MM
            }
        );
    }

    #[test]
    fn a_circle_and_a_point_are_read() {
        let r = read(&[
            (0, "CIRCLE"),
            (8, "DRILLS"),
            (10, "5"),
            (20, "6"),
            (40, "0.5"),
            (0, "POINT"),
            (8, "MARK"),
            (10, "1"),
            (20, "2"),
        ]);
        assert_eq!(
            r.drawing.entities[0].shape,
            Shape::Circle {
                centre: (5 * MM, 6 * MM),
                radius: 500_000
            }
        );
        assert_eq!(
            r.drawing.entities[1].shape,
            Shape::Point { at: (MM, 2 * MM) }
        );
    }

    #[test]
    fn a_light_polyline_keeps_its_bulge_and_its_closed_flag() {
        let r = read(&[
            (0, "LWPOLYLINE"),
            (8, "OUTLINE"),
            (90, "3"),
            (70, "1"),
            (10, "0"),
            (20, "0"),
            (42, "1"),
            (10, "10"),
            (20, "0"),
            (10, "10"),
            (20, "10"),
        ]);
        let (segs, closed) = only_path(&r);
        assert!(closed);
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].sweep_udeg, 180_000_000);
        assert_eq!(segs[0].b, (10 * MM, 0));
        assert_eq!(segs[1], Seg::line((10 * MM, 0), (10 * MM, 10 * MM)));
        assert_eq!(segs[2], Seg::line((10 * MM, 10 * MM), (0, 0)));
    }

    #[test]
    fn an_old_polyline_is_read_with_its_vertices_and_skips_the_frame_of_a_spline_fit() {
        let r = read(&[
            (0, "POLYLINE"),
            (8, "A"),
            (66, "1"),
            (70, "0"),
            (0, "VERTEX"),
            (10, "0"),
            (20, "0"),
            (42, "-1"),
            (0, "VERTEX"),
            (10, "4"),
            (20, "0"),
            (0, "VERTEX"),
            (10, "99"),
            (20, "99"),
            (70, "16"),
            (0, "VERTEX"),
            (10, "4"),
            (20, "3"),
            (0, "SEQEND"),
        ]);
        let (segs, closed) = only_path(&r);
        assert!(!closed);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].sweep_udeg, -180_000_000);
        assert_eq!(segs[1], Seg::line((4 * MM, 0), (4 * MM, 3 * MM)));
        assert_eq!(r.counts.get("POLYLINE"), Some(&1));
        assert_eq!(r.counts.get("VERTEX"), None);
    }

    #[test]
    fn a_spline_is_cut_into_straight_pieces_with_a_finding() {
        let r = read(&[
            (0, "SPLINE"),
            (8, "S"),
            (70, "8"),
            (71, "1"),
            (72, "5"),
            (73, "3"),
            (40, "0"),
            (40, "0"),
            (40, "1"),
            (40, "2"),
            (40, "2"),
            (10, "0"),
            (20, "0"),
            (10, "10"),
            (20, "0"),
            (10, "10"),
            (20, "10"),
        ]);
        let (segs, closed) = only_path(&r);
        assert!(!closed);
        assert!(segs.len() >= 8);
        assert_eq!(segs[0].a, (0, 0));
        assert!(segs.iter().any(|s| s.b == (10 * MM, 0)));
        let finding = &r.report.findings[0];
        assert_eq!(finding.code, SPLINE_APPROXIMATED);
        assert!(finding.detail.starts_with("1 cut into straight pieces"));
    }

    #[test]
    fn a_spline_with_only_fit_points_is_joined_through_them_with_a_finding() {
        let r = read(&[
            (0, "SPLINE"),
            (70, "0"),
            (71, "3"),
            (74, "3"),
            (11, "0"),
            (21, "0"),
            (11, "5"),
            (21, "5"),
            (11, "10"),
            (21, "0"),
        ]);
        let (segs, _) = only_path(&r);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].b, (5 * MM, 5 * MM));
        assert!(r.report.findings[0].detail.contains("fit points"));
    }

    #[test]
    fn an_insert_scales_turns_and_moves_the_block_and_the_layer_zero_takes_the_layer_of_the_insert()
    {
        let blocks = [
            (0, "BLOCK"),
            (2, "Sq"),
            (10, "1"),
            (20, "0"),
            (0, "LINE"),
            (8, "0"),
            (10, "1"),
            (20, "0"),
            (11, "2"),
            (21, "0"),
            (0, "ENDBLK"),
        ];
        let entities = [
            (0, "INSERT"),
            (8, "PLACED"),
            (2, "SQ"),
            (10, "10"),
            (20, "0"),
            (41, "2"),
            (42, "2"),
            (50, "90"),
        ];
        let r = read_dxf(
            &file(&mm_header(), &blocks, &entities),
            &DxfSettings::default(),
        )
        .unwrap();
        assert_eq!(r.drawing.entities.len(), 1);
        assert_eq!(r.drawing.entities[0].layer, "PLACED");
        let (segs, _) = only_path(&r);
        // the base point (1, 0) goes to the insertion point (10, 0); the line of length 1 along x
        // becomes a line of length 2 along y
        assert_eq!(segs, vec![Seg::line((10 * MM, 0), (10 * MM, 2 * MM))]);
        assert_eq!(r.counts.get("INSERT"), Some(&1));
        assert_eq!(r.counts.get("LINE"), Some(&1));
    }

    #[test]
    fn a_mirror_turns_an_arc_the_other_way_and_blocks_may_hold_blocks() {
        let blocks = [
            (0, "BLOCK"),
            (2, "ARCB"),
            (0, "ARC"),
            (8, "A"),
            (10, "0"),
            (20, "0"),
            (40, "10"),
            (50, "0"),
            (51, "90"),
            (0, "ENDBLK"),
            (0, "BLOCK"),
            (2, "OUTER"),
            (0, "INSERT"),
            (8, "A"),
            (2, "ARCB"),
            (10, "0"),
            (20, "0"),
            (41, "-1"),
            (0, "ENDBLK"),
        ];
        let entities = [(0, "INSERT"), (8, "A"), (2, "OUTER"), (10, "0"), (20, "0")];
        let r = read_dxf(
            &file(&mm_header(), &blocks, &entities),
            &DxfSettings::default(),
        )
        .unwrap();
        let (segs, _) = only_path(&r);
        assert_eq!(
            segs,
            vec![Seg {
                a: (-10 * MM, 0),
                b: (0, 10 * MM),
                sweep_udeg: -90_000_000
            }]
        );
    }

    #[test]
    fn the_unit_of_the_file_decides_the_size() {
        let inch = [(9, "$INSUNITS"), (70, "1")];
        let entities = [(0, "LINE"), (10, "0"), (20, "0"), (11, "1"), (21, "0")];
        let r = read_dxf(&file(&inch, &[], &entities), &DxfSettings::default()).unwrap();
        assert_eq!(only_path(&r).0[0].b, (25_400_000, 0));
        assert_eq!(r.unit_mm, 25.4);
        // no unit: the default one, with a finding of information
        let r = read_dxf(&file(&[], &[], &entities), &DxfSettings::default()).unwrap();
        assert_eq!(only_path(&r).0[0].b, (MM, 0));
        assert_eq!(r.report.findings[0].code, UNIT_UNKNOWN);
        assert_eq!(r.report.findings[0].severity, Severity::Info);
        // a unit that is not known: a warning
        let odd = [(9, "$INSUNITS"), (70, "12")];
        let r = read_dxf(&file(&odd, &[], &entities), &DxfSettings::default()).unwrap();
        assert_eq!(r.report.findings[0].severity, Severity::Warning);
        // the default unit may be the inch
        let settings = DxfSettings::parse("default_unit = inch\n").unwrap();
        let r = read_dxf(&file(&[], &[], &entities), &settings).unwrap();
        assert_eq!(only_path(&r).0[0].b, (25_400_000, 0));
    }

    #[test]
    fn what_is_not_understood_is_listed_with_its_count_and_its_layers() {
        let r = read(&[
            (0, "TEXT"),
            (8, "NOTES"),
            (0, "TEXT"),
            (8, "TITLE"),
            (0, "ELLIPSE"),
            (8, "A"),
            (0, "LINE"),
            (10, "0"),
            (20, "0"),
            (11, "1"),
            (21, "0"),
        ]);
        assert_eq!(r.drawing.entities.len(), 1);
        let texts = r
            .report
            .findings
            .iter()
            .find(|f| f.subject.as_deref() == Some("TEXT"))
            .unwrap();
        assert_eq!(texts.code, ENTITY_UNSUPPORTED);
        assert_eq!(texts.detail, "2 left out, on the layers NOTES, TITLE");
        assert!(r
            .report
            .findings
            .iter()
            .any(|f| f.subject.as_deref() == Some("ELLIPSE")));
        assert_eq!(r.counts.get("TEXT"), Some(&2));
    }

    #[test]
    fn a_block_that_does_not_exist_or_holds_itself_is_a_finding_and_a_deep_nest_is_refused() {
        let entities = [(0, "INSERT"), (2, "NOPE"), (10, "0"), (20, "0")];
        let r = read_dxf(&file(&mm_header(), &[], &entities), &DxfSettings::default()).unwrap();
        assert_eq!(r.drawing.entities.len(), 0);
        assert!(r
            .report
            .findings
            .iter()
            .any(|f| f.code == BLOCK_UNUSABLE && f.detail.contains("not defined")));
        let selfish = [
            (0, "BLOCK"),
            (2, "LOOP"),
            (0, "INSERT"),
            (2, "LOOP"),
            (0, "ENDBLK"),
        ];
        let entities = [(0, "INSERT"), (2, "LOOP")];
        let r = read_dxf(
            &file(&mm_header(), &selfish, &entities),
            &DxfSettings::default(),
        )
        .unwrap();
        assert!(r
            .report
            .findings
            .iter()
            .any(|f| f.detail.contains("holds itself")));
        // a chain of blocks one deeper than the limit
        let mut blocks: Vec<(i32, String)> = Vec::new();
        for n in 0..4 {
            blocks.push((0, "BLOCK".to_owned()));
            blocks.push((2, format!("B{n}")));
            blocks.push((0, "INSERT".to_owned()));
            blocks.push((2, format!("B{}", n + 1)));
            blocks.push((0, "ENDBLK".to_owned()));
        }
        blocks.push((0, "BLOCK".to_owned()));
        blocks.push((2, "B4".to_owned()));
        blocks.push((0, "LINE".to_owned()));
        blocks.push((0, "ENDBLK".to_owned()));
        let refs: Vec<(i32, &str)> = blocks.iter().map(|(c, v)| (*c, v.as_str())).collect();
        let entities = [(0, "INSERT"), (2, "B0")];
        let tight = DxfSettings::parse("max_block_depth = 3\n").unwrap();
        let e = read_dxf(&file(&mm_header(), &refs, &entities), &tight).unwrap_err();
        assert_eq!(e.code, LIMIT_EXCEEDED);
        assert!(read_dxf(
            &file(&mm_header(), &refs, &entities),
            &DxfSettings::default()
        )
        .is_ok());
    }

    #[test]
    fn the_limits_refuse_a_file() {
        let four_lines: Vec<(i32, &str)> = (0..4)
            .flat_map(|_| [(0, "LINE"), (10, "0"), (20, "0"), (11, "1"), (21, "1")])
            .collect();
        let text = file(&mm_header(), &[], &four_lines);
        let few = DxfSettings::parse("max_entities = 3\n").unwrap();
        assert_eq!(read_dxf(&text, &few).unwrap_err().code, LIMIT_EXCEEDED);
        let small = DxfSettings::parse("max_file_bytes = 20\n").unwrap();
        assert_eq!(read_dxf(&text, &small).unwrap_err().code, LIMIT_EXCEEDED);
        let no_blocks = [(0, "BLOCK"), (2, "A"), (0, "ENDBLK")];
        let none = DxfSettings::parse("max_blocks = 1\n").unwrap();
        let two = [
            (0, "BLOCK"),
            (2, "A"),
            (0, "ENDBLK"),
            (0, "BLOCK"),
            (2, "B"),
            (0, "ENDBLK"),
        ];
        assert!(read_dxf(&file(&mm_header(), &no_blocks, &four_lines), &none).is_ok());
        assert_eq!(
            read_dxf(&file(&mm_header(), &two, &four_lines), &none)
                .unwrap_err()
                .code,
            LIMIT_EXCEEDED
        );
    }

    #[test]
    fn a_file_that_is_not_dxf_is_refused_with_its_line() {
        let s = DxfSettings::default();
        for (text, needle) in [
            ("", "empty"),
            ("0\nSECTION\n2\n", "odd"),
            ("zero\nSECTION\n", "line 1"),
            ("  0\nEOF\n", "no SECTION"),
            (
                "  0\nSECTION\n  2\nHEADER\n  0\nENDSEC\n  0\nEOF\n",
                "no ENTITIES",
            ),
            (
                "  0\nSECTION\n  2\nENTITIES\n  0\nLINE\n 10\nabc\n  0\nENDSEC\n",
                "line 8",
            ),
        ] {
            let e = read_dxf(text, &s).unwrap_err();
            assert_eq!(e.code, DXF_INVALID, "{text:?}");
            assert!(e.detail.contains(needle), "{text:?}: {}", e.detail);
        }
    }

    #[test]
    fn a_file_with_windows_line_ends_and_a_byte_order_mark_is_read() {
        let text = file(
            &mm_header(),
            &[],
            &[(0, "LINE"), (10, "0"), (20, "0"), (11, "1"), (21, "0")],
        );
        let windows = format!("\u{feff}{}", text.replace('\n', "\r\n"));
        let r = read_dxf(&windows, &DxfSettings::default()).unwrap();
        assert_eq!(r.drawing.entities.len(), 1);
    }
}
