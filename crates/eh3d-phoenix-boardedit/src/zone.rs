// SPDX-License-Identifier: GPL-3.0-or-later
//! The copper zones in the board editor: the properties of a zone that the board model does not
//! hold (priority, minimum width and island, connection and thermal relief of the pads, cutouts),
//! the fills that `eh3d-phoenix-zones` makes of them, and the planning of the operations
//! `zone_add`, `zone_edit`, `zone_remove`, `zone_fill`, `zone_priority` and `zone_cutout`.
//!
//! The model of the board has a zone with a net, a layer, an outline and a clearance, and no room
//! for the rest. The rest lives in [`ZoneData`], which belongs to the editor, goes through undo
//! and redo with the board and is written as a file of its own (`phoenix-zone-data/1`). An
//! operation is a change of the board (an add or a remove of a zone, or nothing) together with a
//! new [`ZoneData`]; the editor keeps both or neither.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Id, Operation, Zone};
use eh3d_phoenix_model::{Nm, Point};
use eh3d_phoenix_route::RuleContext;
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_zones::codes::{
    CLEARANCE_VIOLATED, FILL_EMPTY, ISLAND_NARROW, ISLAND_SMALL, ISLAND_UNCONNECTED,
    OUTLINE_UNUSABLE, OUTSIDE_OUTLINE, PRIORITY_TIE,
};
use eh3d_phoenix_zones::{
    check_fill, fill_board, Connection, FillFile, FillReport, FilledZone, Finding as ZoneFinding,
    Severity as ZoneSeverity, ZoneEntry, ZoneFile, ZoneRules, ZoneSettings, FILL_FORMAT,
    ZONES_FORMAT,
};
use serde::{Deserialize, Serialize};

use crate::codes::{BoardEditError, OPERATION_REFUSED};
use crate::ids::IdSupply;
use crate::ops::EditOp;
use crate::plan::copper_layer;
use crate::zone_codes::{
    FILL_BREAKS_RULES, FILL_ISLAND, FILL_MISSING, NOT_REPRESENTABLE, ZONE_CUTOUT,
    ZONE_DATA_INVALID, ZONE_NOT_FOUND, ZONE_OUTLINE, ZONE_PROPERTY,
};

/// The name of the format of the file of zone data.
pub const DATA_FORMAT: &str = "phoenix-zone-data/1";

/// The largest length a property may take, in nanometres (the range of the coordinates).
const LENGTH_LIMIT_NM: i64 = 1 << 30;

/// The most points for which every pair of sides of an outline is compared.
const SIMPLE_CHECK_LIMIT: usize = 2000;

/// The properties of a zone that the board model does not hold. A property left out takes the
/// value of `config/zones.cfg`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneProps {
    /// The priority; a zone of a higher one is filled first and the others keep away from it.
    #[serde(default)]
    pub priority: i32,
    /// The narrowest island of copper that is kept, in nanometres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_width_nm: Option<i64>,
    /// The smallest island of copper that is kept, in square nanometres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_island_area_nm2: Option<i64>,
    /// How the pads of the net are joined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection: Option<Connection>,
    /// The gap of the thermal reliefs, in nanometres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thermal_gap_nm: Option<i64>,
    /// The width of the spokes of the thermal reliefs, in nanometres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thermal_spoke_width_nm: Option<i64>,
    /// The number of spokes of the thermal reliefs (2 or 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thermal_spokes: Option<u32>,
    /// The holes of the zone, each a closed polygon inside the outline.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cutouts: Vec<Vec<Point>>,
}

impl ZoneProps {
    /// The entry of the file of zone rules of `eh3d-phoenix-zones` for the zone `zone`.
    pub fn entry(&self, zone: Id) -> ZoneEntry {
        ZoneEntry {
            zone,
            priority: self.priority,
            clearance_nm: None,
            min_width_nm: self.min_width_nm,
            min_island_area_nm2: self.min_island_area_nm2,
            connection: self.connection,
            thermal_gap_nm: self.thermal_gap_nm,
            thermal_spoke_width_nm: self.thermal_spoke_width_nm,
            thermal_spokes: self.thermal_spokes,
            cutouts: self
                .cutouts
                .iter()
                .map(|hole| {
                    hole.iter()
                        .map(|p| [p.x_nm.get(), p.y_nm.get()])
                        .collect::<Vec<[i64; 2]>>()
                })
                .collect(),
        }
    }

    /// True when the zone is as a zone with no properties is (nothing but the board model).
    pub fn is_plain(&self) -> bool {
        *self == ZoneProps::default()
    }
}

/// The changes `zone_edit` makes to the properties of a zone; what is left out stays.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneChanges {
    /// The new priority.
    #[serde(default)]
    pub priority: Option<i32>,
    /// The new narrowest island, in nanometres.
    #[serde(default)]
    pub min_width_nm: Option<i64>,
    /// The new smallest island, in square nanometres.
    #[serde(default)]
    pub min_island_area_nm2: Option<i64>,
    /// The new connection of the pads.
    #[serde(default)]
    pub connection: Option<Connection>,
    /// The new gap of the thermal reliefs, in nanometres.
    #[serde(default)]
    pub thermal_gap_nm: Option<i64>,
    /// The new width of the spokes, in nanometres.
    #[serde(default)]
    pub thermal_spoke_width_nm: Option<i64>,
    /// The new number of spokes (2 or 4).
    #[serde(default)]
    pub thermal_spokes: Option<u32>,
    /// The cutout to remove, from 0.
    #[serde(default)]
    pub remove_cutout: Option<usize>,
}

/// What the editor keeps of the zones besides the board: the properties and the fill of each.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ZoneData {
    /// The properties, by zone. A zone with none has the defaults.
    pub props: BTreeMap<Id, ZoneProps>,
    /// The fill of each zone that was filled.
    pub fills: BTreeMap<Id, FilledZone>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DataFile {
    format: String,
    #[serde(default)]
    props: BTreeMap<Id, ZoneProps>,
    #[serde(default)]
    fills: BTreeMap<Id, FilledZone>,
}

impl ZoneData {
    /// The properties of a zone (the defaults when it has none).
    pub fn props_of(&self, zone: Id) -> ZoneProps {
        self.props.get(&zone).cloned().unwrap_or_default()
    }

    /// The fill of a zone, when it has one.
    pub fn fill_of(&self, zone: Id) -> Option<&FilledZone> {
        self.fills.get(&zone)
    }

    /// The file of zone rules of `eh3d-phoenix-zones` for the zones the board has.
    pub fn zone_file(&self, board: &Board) -> ZoneFile {
        ZoneFile {
            format: ZONES_FORMAT.to_owned(),
            zones: self
                .props
                .iter()
                .filter(|(id, _)| board.zones.contains_key(id))
                .map(|(id, props)| props.entry(*id))
                .collect(),
        }
    }

    /// The zones of the board in the order of the fill: the higher priority first, then by id.
    pub fn fill_order(&self, board: &Board) -> Vec<Id> {
        let mut order: Vec<(std::cmp::Reverse<i32>, Id)> = board
            .zones
            .keys()
            .map(|id| (std::cmp::Reverse(self.props_of(*id).priority), *id))
            .collect();
        order.sort();
        order.into_iter().map(|(_, id)| id).collect()
    }

    /// The texts `zones <a> and <b>` that the rules use for an overlap of zones of different nets
    /// on one layer, for the pairs whose priorities differ: the fill of the higher one makes room
    /// for the other, so the overlap is wanted.
    pub fn tolerated_overlaps(&self, board: &Board) -> Vec<String> {
        let zones: Vec<(Id, &Zone)> = board.zones.iter().map(|(id, z)| (*id, z)).collect();
        let mut out: Vec<String> = Vec::new();
        for (i, (ida, a)) in zones.iter().enumerate() {
            for (idb, b) in &zones[i + 1..] {
                if a.net != b.net
                    && a.layer == b.layer
                    && self.props_of(*ida).priority != self.props_of(*idb).priority
                {
                    out.push(format!("zones {ida} and {idb}"));
                }
            }
        }
        out
    }

    /// The data as the text of a file of zone data (`phoenix-zone-data/1`).
    pub fn to_json(&self) -> String {
        let file = DataFile {
            format: DATA_FORMAT.to_owned(),
            props: self.props.clone(),
            fills: self.fills.clone(),
        };
        serde_json::to_string_pretty(&file).unwrap_or_default()
    }

    /// Reads a file of zone data. Unknown fields and another format are refused.
    pub fn from_json(text: &str) -> Result<ZoneData, BoardEditError> {
        let file: DataFile = serde_json::from_str(text).map_err(|e| {
            BoardEditError::new(ZONE_DATA_INVALID, format!("the file cannot be read: {e}"))
        })?;
        if file.format != DATA_FORMAT {
            return Err(BoardEditError::new(
                ZONE_DATA_INVALID,
                format!("the format must be {DATA_FORMAT}"),
            ));
        }
        for props in file.props.values() {
            validate_props(props)?;
        }
        Ok(ZoneData {
            props: file.props,
            fills: file.fills,
        })
    }
}

/// Something about a zone that the person should know, with a stable code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneIssue {
    /// The code, `PHOENIX-BZONE-<NNN>` or a `PHOENIX-ZONE-<NNN>` of the fill.
    pub code: &'static str,
    /// How serious it is.
    pub severity: ZoneSeverity,
    /// The zone it is about, when it is about one.
    pub zone: Option<Id>,
    /// What is specific to the case, in English.
    pub detail: String,
}

impl ZoneIssue {
    /// The issue as a note of an operation (English).
    pub fn note(&self) -> String {
        match self.zone {
            Some(z) => format!("{}: zone {}: {}", self.code, z, self.detail),
            None => format!("{}: {}", self.code, self.detail),
        }
    }

    /// The issue as a line in a language: severity, code, title and detail.
    pub fn line(&self, lang: Lang) -> String {
        let mut title = crate::zone_codes::title(self.code, lang);
        if title.is_empty() {
            title = eh3d_phoenix_zones::title(self.code, lang);
        }
        let mut out = format!("{}: {} {}", self.severity.as_str(), self.code, title);
        if let Some(z) = self.zone {
            out.push_str(&format!(" [zone {z}]"));
        }
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

/// The issue that a finding of the fill or of its check is, with the codes of the editor for the
/// ones the editor names itself.
fn issue_of(f: &ZoneFinding) -> ZoneIssue {
    let code = match f.code {
        c if c == ISLAND_NARROW || c == ISLAND_SMALL || c == ISLAND_UNCONNECTED => FILL_ISLAND,
        c if c == FILL_EMPTY => FILL_ISLAND,
        c if c == CLEARANCE_VIOLATED || c == OUTSIDE_OUTLINE || c == PRIORITY_TIE => {
            FILL_BREAKS_RULES
        }
        c => c,
    };
    ZoneIssue {
        code,
        severity: f.severity,
        zone: f.zone,
        detail: if code == f.code {
            f.detail.clone()
        } else {
            format!("{}: {}", f.code, f.detail)
        },
    }
}

/// What the editor needs to plan an operation on zones besides the board.
pub struct ZoneEnv<'a> {
    /// The rules and the net classes.
    pub ctx: &'a RuleContext,
    /// The settings of the fill (`config/zones.cfg`).
    pub settings: &'a ZoneSettings,
    /// The properties and the fills now.
    pub data: &'a ZoneData,
}

/// What an operation on zones will do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZonePlan {
    /// The change of the board; an empty batch when the operation changes only the zone data.
    pub board_op: Operation,
    /// The zone data after the operation.
    pub data: ZoneData,
    /// The ids of the parts the operation makes.
    pub created: Vec<Id>,
    /// Things to tell the person.
    pub notes: Vec<String>,
    /// True when the design rules must not find a new error, whatever the settings say.
    pub strict: bool,
}

fn empty_batch() -> Operation {
    Operation::Batch { ops: Vec::new() }
}

pub(crate) fn error(code: &'static str, detail: impl Into<String>) -> BoardEditError {
    BoardEditError::new(code, detail)
}

fn not_found(zone: Id) -> BoardEditError {
    error(ZONE_NOT_FOUND, format!("no zone {zone}"))
}

fn length(code: &'static str, what: &str, value: i64) -> Result<Nm, BoardEditError> {
    if !(0..=LENGTH_LIMIT_NM).contains(&value) {
        return Err(error(
            code,
            format!("{what} must be between 0 and 2^30 nanometres"),
        ));
    }
    Nm::new(value).map_err(|e| error(code, e.to_string()))
}

/// Checks the properties of a zone.
pub fn validate_props(props: &ZoneProps) -> Result<(), BoardEditError> {
    for (name, value) in [
        ("min_width_nm", props.min_width_nm),
        ("min_island_area_nm2", props.min_island_area_nm2),
        ("thermal_gap_nm", props.thermal_gap_nm),
        ("thermal_spoke_width_nm", props.thermal_spoke_width_nm),
    ] {
        if let Some(v) = value {
            if v < 0 {
                return Err(error(ZONE_PROPERTY, format!("{name} must not be negative")));
            }
        }
    }
    if props.thermal_spokes.is_some_and(|n| n != 2 && n != 4) {
        return Err(error(ZONE_PROPERTY, "thermal_spokes must be 2 or 4"));
    }
    Ok(())
}

fn apply_changes(props: &mut ZoneProps, ch: &ZoneChanges) -> Result<(), BoardEditError> {
    if let Some(v) = ch.priority {
        props.priority = v;
    }
    if ch.min_width_nm.is_some() {
        props.min_width_nm = ch.min_width_nm;
    }
    if ch.min_island_area_nm2.is_some() {
        props.min_island_area_nm2 = ch.min_island_area_nm2;
    }
    if ch.connection.is_some() {
        props.connection = ch.connection;
    }
    if ch.thermal_gap_nm.is_some() {
        props.thermal_gap_nm = ch.thermal_gap_nm;
    }
    if ch.thermal_spoke_width_nm.is_some() {
        props.thermal_spoke_width_nm = ch.thermal_spoke_width_nm;
    }
    if ch.thermal_spokes.is_some() {
        props.thermal_spokes = ch.thermal_spokes;
    }
    if let Some(index) = ch.remove_cutout {
        if index >= props.cutouts.len() {
            return Err(error(
                ZONE_CUTOUT,
                format!("the zone has no cutout {index}"),
            ));
        }
        props.cutouts.remove(index);
    }
    validate_props(props)
}

// -- exact geometry on integer nanometres ---------------------------------------------------

type P = (i64, i64);

fn xy(p: &Point) -> P {
    (p.x_nm.get(), p.y_nm.get())
}

fn orient(a: P, b: P, c: P) -> i128 {
    (i128::from(b.0) - i128::from(a.0)) * (i128::from(c.1) - i128::from(a.1))
        - (i128::from(b.1) - i128::from(a.1)) * (i128::from(c.0) - i128::from(a.0))
}

fn area2(poly: &[P]) -> i128 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1)
        })
        .sum()
}

fn on_segment(a: P, b: P, p: P) -> bool {
    orient(a, b, p) == 0
        && p.0 >= a.0.min(b.0)
        && p.0 <= a.0.max(b.0)
        && p.1 >= a.1.min(b.1)
        && p.1 <= a.1.max(b.1)
}

fn segments_touch(a: P, b: P, c: P, d: P) -> bool {
    let (o1, o2) = (orient(a, b, c).signum(), orient(a, b, d).signum());
    let (o3, o4) = (orient(c, d, a).signum(), orient(c, d, b).signum());
    (o1 != o2 && o3 != o4)
        || on_segment(a, b, c)
        || on_segment(a, b, d)
        || on_segment(c, d, a)
        || on_segment(c, d, b)
}

fn proper_cross(a: P, b: P, c: P, d: P) -> bool {
    let (o1, o2) = (orient(a, b, c).signum(), orient(a, b, d).signum());
    let (o3, o4) = (orient(c, d, a).signum(), orient(c, d, b).signum());
    o1 * o2 < 0 && o3 * o4 < 0
}

/// True when the point is inside the polygon or on its border.
fn inside_or_on(p: P, poly: &[P]) -> bool {
    let n = poly.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if on_segment(a, b, p) {
            return true;
        }
        if (a.1 > p.1) != (b.1 > p.1) {
            let side = orient(a, b, p);
            let crosses = if b.1 > a.1 { side > 0 } else { side < 0 };
            if crosses {
                inside = !inside;
            }
        }
    }
    inside
}

/// Checks that the points are a polygon with an area whose sides do not touch each other.
fn simple_polygon(points: &[Point], max_points: usize) -> Result<Vec<P>, String> {
    let n = points.len();
    if n < 3 {
        return Err("fewer than three points".to_owned());
    }
    if n > max_points {
        return Err(format!("more than {max_points} points"));
    }
    let v: Vec<P> = points.iter().map(xy).collect();
    for i in 0..n {
        if v[i] == v[(i + 1) % n] {
            return Err("two neighbour points are the same".to_owned());
        }
    }
    if area2(&v) == 0 {
        return Err("the polygon has no area".to_owned());
    }
    for i in 0..n {
        let (a, b, c) = (v[i], v[(i + 1) % n], v[(i + 2) % n]);
        if orient(a, b, c) == 0 {
            let dot = (i128::from(b.0) - i128::from(a.0)) * (i128::from(c.0) - i128::from(b.0))
                + (i128::from(b.1) - i128::from(a.1)) * (i128::from(c.1) - i128::from(b.1));
            if dot < 0 {
                return Err("a side folds back on the one before".to_owned());
            }
        }
    }
    if n <= SIMPLE_CHECK_LIMIT {
        for i in 0..n {
            for j in (i + 1)..n {
                if j == i + 1 || (i == 0 && j == n - 1) {
                    continue;
                }
                if segments_touch(v[i], v[(i + 1) % n], v[j], v[(j + 1) % n]) {
                    return Err("two sides of the polygon touch".to_owned());
                }
            }
        }
    }
    Ok(v)
}

fn check_outline(points: &[Point], settings: &ZoneSettings) -> Result<(), BoardEditError> {
    simple_polygon(points, settings.max_outline_points)
        .map(|_| ())
        .map_err(|m| error(ZONE_OUTLINE, m))
}

/// Checks that a hole is a polygon that lies inside the outline and crosses none of its sides.
fn check_cutout(
    hole: &[Point],
    outline: &[Point],
    settings: &ZoneSettings,
) -> Result<(), BoardEditError> {
    let h = simple_polygon(hole, settings.max_outline_points)
        .map_err(|m| error(ZONE_CUTOUT, format!("the cutout: {m}")))?;
    let o: Vec<P> = outline.iter().map(xy).collect();
    if let Some(p) = h.iter().find(|p| !inside_or_on(**p, &o)) {
        return Err(error(
            ZONE_CUTOUT,
            format!(
                "the cutout has a point outside the zone: ({}, {})",
                p.0, p.1
            ),
        ));
    }
    let (hn, on) = (h.len(), o.len());
    for i in 0..hn {
        for j in 0..on {
            if proper_cross(h[i], h[(i + 1) % hn], o[j], o[(j + 1) % on]) {
                return Err(error(ZONE_CUTOUT, "the cutout crosses a side of the zone"));
            }
        }
    }
    Ok(())
}

// -- the fill -----------------------------------------------------------------------------

/// Fills all the zones of the board with the properties in `data` and returns the report,
/// without keeping anything.
pub fn run_fill(
    board: &Board,
    ctx: &RuleContext,
    settings: &ZoneSettings,
    data: &ZoneData,
) -> Result<(FillReport, ZoneRules), BoardEditError> {
    let rules = ZoneRules::from_rules(board, &ctx.rules, &ctx.classes);
    let file = data.zone_file(board);
    let report = fill_board(board, &rules, settings, Some(&file))
        .map_err(|e| error(ZONE_OUTLINE, e.message(Lang::En)))?;
    Ok((report, rules))
}

/// What the zones say about the board as it is: the zones with no fill, and every proven break of
/// the clearance, the outline or the board by the fills that are kept (the copper changed after
/// the fill), with the islands that join nothing.
pub fn zone_issues(
    board: &Board,
    ctx: &RuleContext,
    settings: &ZoneSettings,
    data: &ZoneData,
) -> Vec<ZoneIssue> {
    let mut out: Vec<ZoneIssue> = Vec::new();
    for id in board.zones.keys() {
        if !data.fills.contains_key(id) {
            out.push(ZoneIssue {
                code: FILL_MISSING,
                severity: ZoneSeverity::Info,
                zone: Some(*id),
                detail: "the zone has no fill; the files carry its outline".to_owned(),
            });
        }
    }
    let rules = ZoneRules::from_rules(board, &ctx.rules, &ctx.classes);
    let file = FillFile {
        format: FILL_FORMAT.to_owned(),
        board: board.name.clone(),
        snap_tolerance_nm: settings.snap_tolerance_nm,
        zones: data
            .fills
            .values()
            .filter(|f| board.zones.contains_key(&f.zone))
            .cloned()
            .collect(),
    };
    match check_fill(board, &rules, settings, &file) {
        Ok(found) => out.extend(found.iter().map(issue_of)),
        Err(e) => out.push(ZoneIssue {
            code: FILL_BREAKS_RULES,
            severity: ZoneSeverity::Error,
            zone: None,
            detail: e.message(Lang::En),
        }),
    }
    out
}

/// The board with each filled zone replaced by one zone for each polygon of its copper, so that
/// a writer that knows only outlines (the Gerber export, the DSN) writes the poured copper. A
/// zone with no fill stays as it is and gets an issue. The new ids are deterministic.
pub fn poured_board(
    board: &Board,
    data: &ZoneData,
) -> Result<(Board, Vec<ZoneIssue>), BoardEditError> {
    let mut out = board.clone();
    let mut ids = IdSupply::new()?;
    let mut issues: Vec<ZoneIssue> = Vec::new();
    for (id, zone) in &board.zones {
        let Some(fill) = data.fills.get(id) else {
            issues.push(ZoneIssue {
                code: FILL_MISSING,
                severity: ZoneSeverity::Warning,
                zone: Some(*id),
                detail: "the zone has no fill and is written as its outline".to_owned(),
            });
            continue;
        };
        out.zones.remove(id);
        for polygon in &fill.polygons {
            let outline: Result<Vec<Point>, _> =
                polygon.iter().map(|p| Point::from_nm(p[0], p[1])).collect();
            let outline = outline.map_err(|e| error(ZONE_DATA_INVALID, e.to_string()))?;
            let new_id = ids.next_for(&out);
            out.zones.insert(
                new_id,
                Zone {
                    net: zone.net,
                    layer: zone.layer,
                    outline,
                    clearance: zone.clearance,
                    cutouts: Vec::new(),
                },
            );
        }
    }
    Ok((out, issues))
}

/// The file formats that the zone data is written to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The Gerber files of the copper layers (through the poured board).
    Gerber,
    /// The Specctra DSN file for the router.
    Dsn,
    /// The KiCad board.
    Kicad,
}

/// What the files of `target` cannot say about the zones, one issue for each zone and thing, so
/// that nothing is dropped without a word.
pub fn export_issues(board: &Board, data: &ZoneData, target: Target) -> Vec<ZoneIssue> {
    let mut out: Vec<ZoneIssue> = Vec::new();
    let mut say = |zone: Id, severity: ZoneSeverity, detail: String| {
        out.push(ZoneIssue {
            code: NOT_REPRESENTABLE,
            severity,
            zone: Some(zone),
            detail,
        });
    };
    for id in board.zones.keys() {
        let props = data.props_of(*id);
        let filled = data.fills.contains_key(id);
        match target {
            Target::Gerber => {
                if !filled {
                    say(
                        *id,
                        ZoneSeverity::Warning,
                        "no fill: the layer file carries the outline of the zone".to_owned(),
                    );
                }
            }
            Target::Dsn => {
                say(
                    *id,
                    ZoneSeverity::Info,
                    "the router gets the outline as a plane of the net; the fill, the cutouts, \
                     the priority and the thermal relief are not written"
                        .to_owned(),
                );
            }
            Target::Kicad => {
                if filled {
                    say(
                        *id,
                        ZoneSeverity::Info,
                        "the filled copper is not written; KiCad fills the zone itself".to_owned(),
                    );
                }
                if props.priority != 0 {
                    say(
                        *id,
                        ZoneSeverity::Warning,
                        "the priority is not written".to_owned(),
                    );
                }
                if !props.cutouts.is_empty() {
                    say(
                        *id,
                        ZoneSeverity::Warning,
                        "the cutouts are not written".to_owned(),
                    );
                }
                if props.thermal_gap_nm.is_some()
                    || props.thermal_spoke_width_nm.is_some()
                    || props.thermal_spokes.is_some()
                    || props.connection.is_some()
                {
                    say(
                        *id,
                        ZoneSeverity::Warning,
                        "the connection and the thermal relief of the zone are not written"
                            .to_owned(),
                    );
                }
            }
        }
    }
    out
}

// -- the operations -------------------------------------------------------------------------

/// Plans an operation on zones. `Ok(None)` when the operation is not one of them. The board and
/// the data are read and not changed.
pub fn plan_zone(
    op: &EditOp,
    board: &Board,
    env: &ZoneEnv,
    ids: &mut IdSupply,
) -> Result<Option<ZonePlan>, BoardEditError> {
    let plan = match op {
        EditOp::ZoneAdd {
            id,
            net,
            layer,
            outline,
            clearance_nm,
            props,
        } => add(
            board,
            env,
            ids,
            *id,
            *net,
            *layer,
            outline,
            *clearance_nm,
            props,
        )?,
        EditOp::ZoneEdit {
            zone,
            net,
            layer,
            outline,
            clearance_nm,
            props,
        } => edit(
            board,
            env,
            *zone,
            *net,
            *layer,
            outline.as_deref(),
            *clearance_nm,
            props,
        )?,
        EditOp::ZoneRemove { zone } => remove(board, env, *zone)?,
        EditOp::ZoneFill { zone } => fill(board, env, *zone)?,
        EditOp::ZonePriority { zone, priority } => priority_of(board, env, *zone, *priority)?,
        EditOp::ZoneCutout { zone, polygon } => cutout(board, env, *zone, polygon)?,
        EditOp::PasteGroup {
            clipboard,
            from_net,
            at,
            quarter_turns,
            mirror,
            net_map,
        } => crate::zone_clip::plan_paste(
            board,
            env,
            ids,
            &crate::zone_clip::PasteRequest {
                clipboard: clipboard.as_ref(),
                from_net: *from_net,
                at: *at,
                quarter_turns: *quarter_turns,
                mirror: *mirror,
                net_map,
            },
        )?,
        _ => return Ok(None),
    };
    Ok(Some(plan))
}

fn data_only(data: ZoneData, notes: Vec<String>) -> ZonePlan {
    ZonePlan {
        board_op: empty_batch(),
        data,
        created: Vec::new(),
        notes,
        strict: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn add(
    board: &Board,
    env: &ZoneEnv,
    ids: &mut IdSupply,
    id: Option<Id>,
    net: Id,
    layer: Id,
    outline: &[Point],
    clearance_nm: Option<i64>,
    props: &ZoneProps,
) -> Result<ZonePlan, BoardEditError> {
    if !board.nets.contains_key(&net) {
        return Err(error(OPERATION_REFUSED, format!("no net {net}")));
    }
    copper_layer(board, layer)?;
    check_outline(outline, env.settings)?;
    validate_props(props)?;
    for hole in &props.cutouts {
        check_cutout(hole, outline, env.settings)?;
    }
    let clearance = match clearance_nm {
        Some(c) => length(ZONE_PROPERTY, "clearance_nm", c)?,
        None => env.ctx.rules.min_clearance,
    };
    let id = match id {
        Some(given) => {
            if board.zones.contains_key(&given) {
                return Err(error(
                    OPERATION_REFUSED,
                    format!("the zone {given} exists already"),
                ));
            }
            given
        }
        None => ids.next_for(board),
    };
    let mut data = env.data.clone();
    data.props.insert(id, props.clone());
    Ok(ZonePlan {
        board_op: Operation::AddZone {
            id,
            zone: Zone {
                net,
                layer,
                outline: outline.to_vec(),
                clearance,
                cutouts: Vec::new(),
            },
        },
        data,
        created: vec![id],
        notes: vec![format!(
            "{FILL_MISSING}: zone {id}: the zone has no fill yet (zone_fill)"
        )],
        strict: false,
    })
}

#[allow(clippy::too_many_arguments)]
fn edit(
    board: &Board,
    env: &ZoneEnv,
    zone: Id,
    net: Option<Id>,
    layer: Option<Id>,
    outline: Option<&[Point]>,
    clearance_nm: Option<i64>,
    changes: &ZoneChanges,
) -> Result<ZonePlan, BoardEditError> {
    let old = board.zones.get(&zone).ok_or_else(|| not_found(zone))?;
    let mut new = old.clone();
    if let Some(n) = net {
        if !board.nets.contains_key(&n) {
            return Err(error(OPERATION_REFUSED, format!("no net {n}")));
        }
        new.net = n;
    }
    if let Some(l) = layer {
        copper_layer(board, l)?;
        new.layer = l;
    }
    if let Some(points) = outline {
        check_outline(points, env.settings)?;
        new.outline = points.to_vec();
    }
    if let Some(c) = clearance_nm {
        new.clearance = length(ZONE_PROPERTY, "clearance_nm", c)?;
    }
    let mut props = env.data.props_of(zone);
    apply_changes(&mut props, changes)?;
    for hole in &props.cutouts {
        check_cutout(hole, &new.outline, env.settings)?;
    }
    let board_op = if new == *old {
        empty_batch()
    } else {
        Operation::Batch {
            ops: vec![
                Operation::RemoveZone { id: zone },
                Operation::AddZone {
                    id: zone,
                    zone: new,
                },
            ],
        }
    };
    let mut data = env.data.clone();
    data.props.insert(zone, props);
    let mut notes: Vec<String> = Vec::new();
    if data.fills.remove(&zone).is_some() {
        notes.push(format!(
            "{FILL_MISSING}: zone {zone}: the fill was dropped; fill the zone again"
        ));
    }
    Ok(ZonePlan {
        board_op,
        data,
        created: Vec::new(),
        notes,
        strict: false,
    })
}

fn remove(board: &Board, env: &ZoneEnv, zone: Id) -> Result<ZonePlan, BoardEditError> {
    if !board.zones.contains_key(&zone) {
        return Err(not_found(zone));
    }
    let mut data = env.data.clone();
    data.props.remove(&zone);
    data.fills.remove(&zone);
    Ok(ZonePlan {
        board_op: Operation::RemoveZone { id: zone },
        data,
        created: Vec::new(),
        notes: Vec::new(),
        strict: false,
    })
}

fn priority_of(
    board: &Board,
    env: &ZoneEnv,
    zone: Id,
    priority: i32,
) -> Result<ZonePlan, BoardEditError> {
    if !board.zones.contains_key(&zone) {
        return Err(not_found(zone));
    }
    let mut data = env.data.clone();
    let mut props = data.props_of(zone);
    props.priority = priority;
    data.props.insert(zone, props);
    let mut notes: Vec<String> = Vec::new();
    if data.fills.remove(&zone).is_some() {
        notes.push(format!(
            "{FILL_MISSING}: zone {zone}: the fill was dropped; fill the zones again"
        ));
    }
    Ok(data_only(data, notes))
}

fn cutout(
    board: &Board,
    env: &ZoneEnv,
    zone: Id,
    polygon: &[Point],
) -> Result<ZonePlan, BoardEditError> {
    let z = board.zones.get(&zone).ok_or_else(|| not_found(zone))?;
    check_cutout(polygon, &z.outline, env.settings)?;
    let mut data = env.data.clone();
    let mut props = data.props_of(zone);
    props.cutouts.push(polygon.to_vec());
    data.props.insert(zone, props);
    let mut notes: Vec<String> = Vec::new();
    if data.fills.remove(&zone).is_some() {
        notes.push(format!(
            "{FILL_MISSING}: zone {zone}: the fill was dropped; fill the zone again"
        ));
    }
    Ok(data_only(data, notes))
}

fn fill(board: &Board, env: &ZoneEnv, target: Option<Id>) -> Result<ZonePlan, BoardEditError> {
    if let Some(z) = target {
        if !board.zones.contains_key(&z) {
            return Err(not_found(z));
        }
    }
    let (report, rules) = run_fill(board, env.ctx, env.settings, env.data)?;
    let relevant = |zone: Option<Id>| target.is_none() || zone.is_none() || zone == target;
    if let Some(bad) = report
        .findings
        .iter()
        .find(|f| f.severity == ZoneSeverity::Error && relevant(f.zone))
    {
        let code = if bad.code == OUTLINE_UNUSABLE {
            ZONE_OUTLINE
        } else {
            FILL_BREAKS_RULES
        };
        return Err(error(code, format!("{}: {}", bad.code, bad.detail)));
    }
    let keep = FillFile {
        format: FILL_FORMAT.to_owned(),
        board: report.fill.board.clone(),
        snap_tolerance_nm: report.fill.snap_tolerance_nm,
        zones: report
            .fill
            .zones
            .iter()
            .filter(|f| target.is_none() || Some(f.zone) == target)
            .cloned()
            .collect(),
    };
    let proven = check_fill(board, &rules, env.settings, &keep)
        .map_err(|e| error(FILL_BREAKS_RULES, e.message(Lang::En)))?;
    if let Some(bad) = proven.iter().find(|f| f.severity == ZoneSeverity::Error) {
        return Err(error(
            FILL_BREAKS_RULES,
            format!("{}: {}", bad.code, bad.detail),
        ));
    }
    let mut data = env.data.clone();
    match target {
        None => data.fills.clear(),
        Some(z) => {
            data.fills.remove(&z);
        }
    }
    data.fills.retain(|id, _| board.zones.contains_key(id));
    let mut notes: Vec<String> = Vec::new();
    for stat in report.stats.iter().filter(|s| relevant(Some(s.zone))) {
        notes.push(format!(
            "zone {}: {} polygons, {} islands kept, {} dropped, {} nm2 of copper",
            stat.zone, stat.polygons, stat.islands_kept, stat.islands_removed, stat.area_nm2
        ));
    }
    for finding in report
        .findings
        .iter()
        .chain(proven.iter())
        .filter(|f| relevant(f.zone))
    {
        notes.push(issue_of(finding).note());
    }
    for fz in keep.zones {
        data.fills.insert(fz.zone, fz);
    }
    Ok(data_only(data, notes))
}
