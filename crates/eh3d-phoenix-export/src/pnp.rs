//! The pick and place file: where every part goes, for each face of the board.
//!
//! [`plan`] works out the centre, the rotation and the face of every component; [`pnp_csv`] and
//! [`pnp_json`] (format `phoenix-pnp/1`) write them. The settings come from
//! `config/pnp_rotations.cfg` ([`PnpSettings::parse`]):
//!
//! * the origin of the coordinates: the lower left corner of the board outline, an auxiliary origin
//!   or the origin of a panel in which the lower left corner of the board sits at a given place;
//! * the unit (millimetres or mils) and the digits;
//! * what the centre of a part is: the middle of the box of its pads (`body`), the mean of its pads
//!   (`pads`) or the origin of its footprint (`origin`);
//! * the table of extra rotations: a pattern of the footprint name and the degrees to add, for the
//!   heads of a machine that hold a part turned (`rotate = SOT23* 180`), apart or in a section
//!   `[maker]` chosen with [`PnpSettings::select_maker`];
//! * whether the parts that are not fitted are left out or marked.
//!
//! The same board and settings always give the same bytes. Lengths are integer nanometres and
//! angles integer micro-degrees; a right angle is exact and any other angle uses floating point
//! once for the box of a pad.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Component, Footprint, Id, Pad, PadRef, PadShape, Side};

use crate::assembly::{csv_line, format_micro, natural_key, written_rotation, FULL_TURN};
use crate::fabcodes::{
    FabError, Finding, Severity, PNP_DUPLICATE, PNP_NOT_FITTED, PNP_NO_COMPONENTS,
    PNP_NO_FOOTPRINT, PNP_NO_PADS, PNP_NO_REFDES, PNP_OUTSIDE, PNP_ROTATION_CORRECTED,
    PNP_SETTINGS_INVALID,
};
use crate::fabutil::{glob_match, json_string, parse_micro, settings_line, split_pair};
use crate::profile::BottomRotation;

const HALF_TURN: i64 = 180_000_000;

/// Where the coordinates start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PnpOrigin {
    /// The lower left corner of the box of the board outline.
    LowerLeft,
    /// The auxiliary origin of the settings, in the coordinates of the board.
    Aux,
    /// The origin of a panel in which the lower left corner of the board is at the place of the
    /// settings.
    Panel,
}

/// The unit of the coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Millimetres.
    Mm,
    /// Thousandths of an inch.
    Mil,
}

impl Unit {
    /// The name written in the files.
    pub fn name(self) -> &'static str {
        match self {
            Unit::Mm => "mm",
            Unit::Mil => "mil",
        }
    }
}

/// What the centre of a part is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Centre {
    /// The middle of the box of the pads.
    Body,
    /// The mean of the centres of the pads.
    Pads,
    /// The origin of the footprint.
    Origin,
}

/// What is done with the parts that are not fitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotFitted {
    /// They are left out of the lines and listed apart.
    Exclude,
    /// They stay in the lines, marked.
    Mark,
}

/// One line of the table of extra rotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationRule {
    /// The section (the maker) the line is in; `None` before the first section.
    pub section: Option<String>,
    /// The pattern of the footprint name (`*` and `?`, any case).
    pub pattern: String,
    /// The degrees to add, in micro-degrees.
    pub micro_degrees: i64,
}

/// The settings of the pick and place file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PnpSettings {
    /// Where the coordinates start.
    pub origin: PnpOrigin,
    /// The auxiliary origin, in nanometres.
    pub aux: (i64, i64),
    /// The place of the lower left corner of the board in the panel, in nanometres.
    pub panel: (i64, i64),
    /// The unit.
    pub unit: Unit,
    /// Digits after the point of X and Y.
    pub decimals: u32,
    /// Digits after the point of the rotation.
    pub rotation_decimals: u32,
    /// What the centre of a part is.
    pub centre: Centre,
    /// How the rotation of a part on the bottom side is written.
    pub bottom_rotation: BottomRotation,
    /// What is done with the parts that are not fitted.
    pub not_fitted: NotFitted,
    /// The separator of the fields of the CSV file.
    pub delimiter: char,
    /// The table of extra rotations, in the order of the file.
    pub rules: Vec<RotationRule>,
    /// The sections of the file.
    pub makers: Vec<String>,
    /// The section that is in force, when one was chosen.
    pub maker: Option<String>,
}

impl Default for PnpSettings {
    fn default() -> Self {
        PnpSettings {
            origin: PnpOrigin::LowerLeft,
            aux: (0, 0),
            panel: (0, 0),
            unit: Unit::Mm,
            decimals: 4,
            rotation_decimals: 2,
            centre: Centre::Body,
            bottom_rotation: BottomRotation::Keep,
            not_fitted: NotFitted::Exclude,
            delimiter: ',',
            rules: Vec::new(),
            makers: Vec::new(),
            maker: None,
        }
    }
}

impl PnpSettings {
    /// Reads a settings text. A missing line keeps the default; an unknown key or a value out of
    /// range is refused with its line number.
    pub fn parse(text: &str) -> Result<PnpSettings, FabError> {
        let mut s = PnpSettings::default();
        let mut section: Option<String> = None;
        for (index, raw) in text.lines().enumerate() {
            let number = index + 1;
            let Some(line) = settings_line(raw) else {
                continue;
            };
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                let name = name.trim();
                if name.is_empty() || s.makers.iter().any(|m| m == name) {
                    return Err(bad(format!(
                        "line {number}: the section is empty or given twice"
                    )));
                }
                s.makers.push(name.to_owned());
                section = Some(name.to_owned());
                continue;
            }
            let (key, value) = split_pair(line, number, PNP_SETTINGS_INVALID)?;
            let invalid = || bad(format!("line {number}: the value of {key} is not valid"));
            if key == "rotate" {
                let mut words = value.split_whitespace();
                let (Some(pattern), Some(degrees), None) =
                    (words.next(), words.next(), words.next())
                else {
                    return Err(invalid());
                };
                let micro = parse_micro(degrees)
                    .filter(|d| d.abs() < 1_000 * 1_000_000)
                    .ok_or_else(invalid)?;
                s.rules.push(RotationRule {
                    section: section.clone(),
                    pattern: pattern.to_owned(),
                    micro_degrees: micro,
                });
                continue;
            }
            if section.is_some() {
                return Err(bad(format!(
                    "line {number}: only rotate lines belong in a section"
                )));
            }
            let length = || parse_micro(value).ok_or_else(invalid);
            match key {
                "origin" => {
                    s.origin = match value {
                        "lower_left" => PnpOrigin::LowerLeft,
                        "aux" => PnpOrigin::Aux,
                        "panel" => PnpOrigin::Panel,
                        _ => return Err(invalid()),
                    }
                }
                "aux_x_mm" => s.aux.0 = length()?,
                "aux_y_mm" => s.aux.1 = length()?,
                "panel_x_mm" => s.panel.0 = length()?,
                "panel_y_mm" => s.panel.1 = length()?,
                "units" => {
                    s.unit = match value {
                        "mm" => Unit::Mm,
                        "mil" => Unit::Mil,
                        _ => return Err(invalid()),
                    }
                }
                "decimals" => s.decimals = value.parse().map_err(|_| invalid())?,
                "rotation_decimals" => {
                    s.rotation_decimals = value.parse().map_err(|_| invalid())?
                }
                "centre" => {
                    s.centre = match value {
                        "body" => Centre::Body,
                        "pads" => Centre::Pads,
                        "origin" => Centre::Origin,
                        _ => return Err(invalid()),
                    }
                }
                "bottom_rotation" => {
                    s.bottom_rotation = match value {
                        "keep" => BottomRotation::Keep,
                        "mirror" => BottomRotation::Mirror,
                        "half_turn" => BottomRotation::HalfTurn,
                        _ => return Err(invalid()),
                    }
                }
                "not_fitted" => {
                    s.not_fitted = match value {
                        "exclude" => NotFitted::Exclude,
                        "mark" => NotFitted::Mark,
                        _ => return Err(invalid()),
                    }
                }
                "delimiter" => {
                    s.delimiter = match value {
                        "tab" => '\t',
                        "comma" | "," => ',',
                        "semicolon" | ";" => ';',
                        _ => return Err(invalid()),
                    }
                }
                _ => {
                    return Err(bad(format!("line {number}: unknown key {key}")));
                }
            }
        }
        s.validate()?;
        Ok(s)
    }

    /// Checks the ranges.
    pub fn validate(&self) -> Result<(), FabError> {
        if self.decimals > 6 || self.rotation_decimals > 6 {
            return Err(bad(
                "decimals and rotation_decimals are at most 6".to_owned()
            ));
        }
        for (name, value) in [
            ("aux_x_mm", self.aux.0),
            ("aux_y_mm", self.aux.1),
            ("panel_x_mm", self.panel.0),
            ("panel_y_mm", self.panel.1),
        ] {
            if value.abs() >= 1_000_000_000 {
                return Err(bad(format!("{name} is out of range")));
            }
        }
        Ok(())
    }

    /// Puts the table of a maker in force. The lines of its section are tried before the lines
    /// that come before the first section.
    pub fn select_maker(&mut self, name: &str) -> Result<(), FabError> {
        if !self.makers.iter().any(|m| m == name) {
            return Err(bad(format!("the settings have no section [{name}]")));
        }
        self.maker = Some(name.to_owned());
        Ok(())
    }

    /// The extra rotation for a footprint, in micro-degrees (0 when no line matches).
    pub fn extra_rotation(&self, footprint: &str) -> i64 {
        let find = |section: Option<&str>| {
            self.rules
                .iter()
                .find(|r| r.section.as_deref() == section && glob_match(&r.pattern, footprint))
                .map(|r| r.micro_degrees)
        };
        self.maker
            .as_deref()
            .and_then(|m| find(Some(m)))
            .or_else(|| find(None))
            .unwrap_or(0)
    }
}

fn bad(detail: String) -> FabError {
    FabError::new(PNP_SETTINGS_INVALID, detail)
}

/// One part of the placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PnpRow {
    /// The reference designator.
    pub refdes: String,
    /// The value.
    pub value: String,
    /// The name of the footprint (empty when the board does not have it).
    pub footprint: String,
    /// X of the centre, in nanometres from the origin of the settings.
    pub x_nm: i64,
    /// Y of the centre, in nanometres from the origin of the settings.
    pub y_nm: i64,
    /// The rotation written, in micro-degrees from 0 up to a full turn (not included).
    pub rotation: i64,
    /// The face.
    pub side: Side,
    /// False for a part that is not fitted.
    pub fitted: bool,
}

/// The placement of a board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PnpPlan {
    /// The name of the board.
    pub name: String,
    /// The parts on the top face, in the natural order of their designators.
    pub top: Vec<PnpRow>,
    /// The parts on the bottom face.
    pub bottom: Vec<PnpRow>,
    /// The parts that are not fitted and were left out of the lines.
    pub excluded: Vec<PnpRow>,
    /// What was found.
    pub findings: Vec<Finding>,
}

impl PnpPlan {
    /// The lines of a face.
    pub fn rows(&self, side: Side) -> &[PnpRow] {
        match side {
            Side::Top => &self.top,
            Side::Bottom => &self.bottom,
        }
    }
}

fn outline_box(board: &Board) -> Option<(i64, i64, i64, i64)> {
    let first = board.outline.first()?;
    let mut b = (
        first.x_nm.get(),
        first.y_nm.get(),
        first.x_nm.get(),
        first.y_nm.get(),
    );
    for p in &board.outline {
        b.0 = b.0.min(p.x_nm.get());
        b.1 = b.1.min(p.y_nm.get());
        b.2 = b.2.max(p.x_nm.get());
        b.3 = b.3.max(p.y_nm.get());
    }
    Some(b)
}

/// The direction of the pad, in micro-degrees from 0 up to a full turn (not included): a part on
/// the bottom side is mirrored, so the turn of the pad is taken off the turn of the part.
pub(crate) fn orientation(component: i64, pad: i64, side: Side) -> i64 {
    let turn = match side {
        Side::Top => component + pad,
        Side::Bottom => component - pad,
    };
    turn.rem_euclid(FULL_TURN)
}

/// The half sides of the box of a pad turned by `angle`, in nanometres.
fn half_extent(shape: PadShape, angle: i64) -> (i64, i64) {
    let (w, h) = match shape {
        PadShape::Circle { diameter } => (diameter.get(), diameter.get()),
        PadShape::Rect { width, height } | PadShape::Oblong { width, height } => {
            (width.get(), height.get())
        }
    };
    match angle.rem_euclid(HALF_TURN) {
        0 => (w / 2, h / 2),
        90_000_000 => (h / 2, w / 2),
        a => {
            let rad = (a as f64) / 1_000_000.0 * std::f64::consts::PI / 180.0;
            let (s, c) = (rad.sin().abs(), rad.cos().abs());
            let (wf, hf) = (w as f64, h as f64);
            (
                ((wf * c + hf * s) / 2.0).round() as i64,
                ((wf * s + hf * c) / 2.0).round() as i64,
            )
        }
    }
}

fn pad_box(board: &Board, cid: Id, pid: Id, pad: &Pad, component: &Component) -> Option<[i64; 4]> {
    let p = pad_position(
        board,
        PadRef {
            component: cid,
            pad: pid,
        },
    )?;
    let angle = orientation(component.rotation.get(), pad.rotation.get(), component.side);
    let (hx, hy) = half_extent(pad.shape, angle);
    let (x, y) = (p.x_nm.get(), p.y_nm.get());
    Some([x - hx, y - hy, x + hx, y + hy])
}

/// The centre of a component in the coordinates of the board.
fn centre_of(
    board: &Board,
    cid: Id,
    component: &Component,
    footprint: &Footprint,
    mode: Centre,
) -> (i64, i64) {
    let origin = (component.at.x_nm.get(), component.at.y_nm.get());
    if mode == Centre::Origin {
        return origin;
    }
    let mut boxes: Vec<[i64; 4]> = Vec::new();
    for (pid, pad) in &footprint.pads {
        if let Some(b) = pad_box(board, cid, *pid, pad, component) {
            boxes.push(b);
        }
    }
    if boxes.is_empty() {
        return origin;
    }
    if mode == Centre::Pads {
        let n = boxes.len() as i128;
        let sx: i128 = boxes
            .iter()
            .map(|b| i128::from(b[0]) + i128::from(b[2]))
            .sum();
        let sy: i128 = boxes
            .iter()
            .map(|b| i128::from(b[1]) + i128::from(b[3]))
            .sum();
        // the sums count each centre twice (the two sides of its box)
        let share = |s: i128| (s + n).div_euclid(2 * n) as i64;
        return (share(sx), share(sy));
    }
    let min_x = boxes.iter().map(|b| b[0]).min().unwrap_or(origin.0);
    let min_y = boxes.iter().map(|b| b[1]).min().unwrap_or(origin.1);
    let max_x = boxes.iter().map(|b| b[2]).max().unwrap_or(origin.0);
    let max_y = boxes.iter().map(|b| b[3]).max().unwrap_or(origin.1);
    (
        min_x + (max_x - min_x).div_euclid(2),
        min_y + (max_y - min_y).div_euclid(2),
    )
}

/// Works out the placement of every component. `not_fitted` holds the designators of the parts
/// that are not fitted.
pub fn plan(board: &Board, settings: &PnpSettings, not_fitted: &BTreeSet<String>) -> PnpPlan {
    let mut findings: Vec<Finding> = Vec::new();
    if board.components.is_empty() {
        findings.push(Finding::new(PNP_NO_COMPONENTS, Severity::Warning, "", ""));
    }
    let outline = outline_box(board);
    let (min_x, min_y) = outline.map_or((0, 0), |b| (b.0, b.1));
    let (origin_x, origin_y) = match settings.origin {
        PnpOrigin::LowerLeft => (min_x, min_y),
        PnpOrigin::Aux => settings.aux,
        PnpOrigin::Panel => (min_x - settings.panel.0, min_y - settings.panel.1),
    };
    let mut parts: Vec<(&Id, &Component)> = board.components.iter().collect();
    parts.sort_by_cached_key(|(id, c)| (natural_key(&c.refdes), **id));
    let mut uses: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, c) in &parts {
        *uses.entry(c.refdes.as_str()).or_default() += 1;
    }
    let mut reported: BTreeSet<&str> = BTreeSet::new();
    let (mut top, mut bottom, mut excluded) = (Vec::new(), Vec::new(), Vec::new());
    let mut corrected = 0usize;
    for (cid, c) in parts {
        let refdes = c.refdes.as_str();
        if refdes.is_empty() {
            findings.push(Finding::new(PNP_NO_REFDES, Severity::Warning, "", ""));
        } else if uses.get(refdes).copied().unwrap_or(0) > 1 && reported.insert(refdes) {
            findings.push(Finding::new(PNP_DUPLICATE, Severity::Warning, refdes, ""));
        }
        let footprint = board.footprints.get(&c.footprint);
        let (cx, cy) = match footprint {
            None => {
                findings.push(Finding::new(
                    PNP_NO_FOOTPRINT,
                    Severity::Warning,
                    refdes,
                    "",
                ));
                (c.at.x_nm.get(), c.at.y_nm.get())
            }
            Some(f) => {
                if f.pads.is_empty() {
                    findings.push(Finding::new(PNP_NO_PADS, Severity::Info, refdes, ""));
                }
                centre_of(board, *cid, c, f, settings.centre)
            }
        };
        let footprint_name = footprint.map_or_else(String::new, |f| f.name.clone());
        let extra = settings.extra_rotation(&footprint_name);
        if extra != 0 {
            corrected += 1;
        }
        if let Some((a, b, d, e)) = outline {
            if !(a..=d).contains(&cx) || !(b..=e).contains(&cy) {
                findings.push(Finding::new(PNP_OUTSIDE, Severity::Warning, refdes, ""));
            }
        }
        let fitted = !not_fitted.contains(refdes);
        let row = PnpRow {
            refdes: c.refdes.clone(),
            value: c.value.clone(),
            footprint: footprint_name,
            x_nm: cx - origin_x,
            y_nm: cy - origin_y,
            rotation: written_rotation(c.rotation.get() + extra, c.side, settings.bottom_rotation),
            side: c.side,
            fitted,
        };
        if !fitted {
            findings.push(Finding::new(
                PNP_NOT_FITTED,
                Severity::Info,
                refdes,
                match settings.not_fitted {
                    NotFitted::Exclude => "left out",
                    NotFitted::Mark => "marked",
                },
            ));
            if settings.not_fitted == NotFitted::Exclude {
                excluded.push(row);
                continue;
            }
        }
        match c.side {
            Side::Top => top.push(row),
            Side::Bottom => bottom.push(row),
        }
    }
    if corrected > 0 {
        findings.push(Finding::new(
            PNP_ROTATION_CORRECTED,
            Severity::Info,
            "",
            format!("{corrected} part(s)"),
        ));
    }
    PnpPlan {
        name: board.name.clone(),
        top,
        bottom,
        excluded,
        findings,
    }
}

/// A length in the unit of the settings, rounded half away from zero to `decimals` digits.
pub fn length_text(nm: i64, unit: Unit, decimals: u32) -> String {
    match unit {
        Unit::Mm => format_micro(nm, decimals),
        Unit::Mil => {
            // one mil is 25 400 nm: millionths of a mil are nm * 10^6 / 25 400
            let n = i128::from(nm) * 1_000_000;
            let q = (n.abs() + 12_700) / 25_400;
            let signed = if n < 0 { -q } else { q };
            format_micro(i64::try_from(signed).unwrap_or(0), decimals)
        }
    }
}

fn side_word(side: Side) -> &'static str {
    match side {
        Side::Top => "top",
        Side::Bottom => "bottom",
    }
}

/// The CSV file of one face: a header line and one line for each part.
pub fn pnp_csv(rows: &[PnpRow], settings: &PnpSettings) -> String {
    let status = rows.iter().any(|r| !r.fitted);
    let unit = settings.unit.name();
    let mut header = vec![
        "Ref".to_owned(),
        "Value".to_owned(),
        "Package".to_owned(),
        format!("PosX({unit})"),
        format!("PosY({unit})"),
        "Rot(deg)".to_owned(),
        "Side".to_owned(),
    ];
    if status {
        header.push("Status".to_owned());
    }
    let mut text = csv_line(&header, settings.delimiter);
    for r in rows {
        let mut fields = vec![
            r.refdes.clone(),
            r.value.clone(),
            r.footprint.clone(),
            length_text(r.x_nm, settings.unit, settings.decimals),
            length_text(r.y_nm, settings.unit, settings.decimals),
            format_micro(r.rotation, settings.rotation_decimals),
            side_word(r.side).to_owned(),
        ];
        if status {
            let word = if r.fitted { "fitted" } else { "not_fitted" };
            fields.push(word.to_owned());
        }
        text.push_str(&csv_line(&fields, settings.delimiter));
    }
    text
}

fn origin_word(origin: PnpOrigin) -> &'static str {
    match origin {
        PnpOrigin::LowerLeft => "lower_left",
        PnpOrigin::Aux => "aux",
        PnpOrigin::Panel => "panel",
    }
}

fn centre_word(centre: Centre) -> &'static str {
    match centre {
        Centre::Body => "body",
        Centre::Pads => "pads",
        Centre::Origin => "origin",
    }
}

/// The JSON file of one face, format `phoenix-pnp/1`.
pub fn pnp_json(plan: &PnpPlan, side: Side, settings: &PnpSettings) -> String {
    let mut text = String::from("{\n");
    text.push_str("  \"format\": \"phoenix-pnp/1\",\n");
    text.push_str(&format!("  \"board\": {},\n", json_string(&plan.name)));
    text.push_str(&format!("  \"side\": \"{}\",\n", side_word(side)));
    text.push_str(&format!("  \"units\": \"{}\",\n", settings.unit.name()));
    text.push_str(&format!(
        "  \"origin\": \"{}\",\n",
        origin_word(settings.origin)
    ));
    text.push_str(&format!(
        "  \"centre\": \"{}\",\n",
        centre_word(settings.centre)
    ));
    let rows = plan.rows(side);
    if rows.is_empty() {
        text.push_str("  \"parts\": [],\n");
    } else {
        text.push_str("  \"parts\": [\n");
        for (i, r) in rows.iter().enumerate() {
            text.push_str(&format!(
                "    {{\"ref\": {}, \"value\": {}, \"footprint\": {}, \"x\": {}, \"y\": {}, \"rotation\": {}, \"side\": \"{}\", \"fitted\": {}}}{}\n",
                json_string(&r.refdes),
                json_string(&r.value),
                json_string(&r.footprint),
                length_text(r.x_nm, settings.unit, settings.decimals),
                length_text(r.y_nm, settings.unit, settings.decimals),
                format_micro(r.rotation, settings.rotation_decimals),
                side_word(r.side),
                r.fitted,
                if i + 1 == rows.len() { "" } else { "," }
            ));
        }
        text.push_str("  ],\n");
    }
    let left_out: Vec<String> = plan
        .excluded
        .iter()
        .filter(|r| r.side == side)
        .map(|r| json_string(&r.refdes))
        .collect();
    text.push_str(&format!("  \"not_fitted\": [{}]\n", left_out.join(", ")));
    text.push_str("}\n");
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::Layer;
    use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

    const MM: i64 = 1_000_000;

    fn id(n: u32) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    fn pad(number: &str, at: (i64, i64), w: i64, h: i64) -> Pad {
        Pad {
            number: number.to_owned(),
            shape: PadShape::Rect {
                width: Nm::new(w).unwrap(),
                height: Nm::new(h).unwrap(),
            },
            at: pt(at.0, at.1),
            rotation: Udeg::new(0).unwrap(),
            drill: None,
            layers: vec![id(1)],
            roundness: 0,
            roundness: 0,
            plated: true,
            plated: true,
        }
    }

    fn part(refdes: &str, footprint: u32, at: (i64, i64), rot: i64, side: Side) -> Component {
        Component {
            refdes: refdes.to_owned(),
            value: "10k".to_owned(),
            symbol: None,
            footprint: id(footprint),
            at: pt(at.0, at.1),
            rotation: Udeg::new(rot).unwrap(),
            side,
        }
    }

    /// A board from (5, 5) to (25, 15) mm. The footprint `R0603` has two pads 1.6 mm apart, so the
    /// box of its pads is off the origin of the footprint by 0.1 mm; `SOT23-5` has one pad.
    fn board() -> Board {
        let mut b = Board {
            name: "pnp".to_owned(),
            outline: vec![
                pt(5 * MM, 5 * MM),
                pt(25 * MM, 5 * MM),
                pt(25 * MM, 15 * MM),
                pt(5 * MM, 15 * MM),
            ],
            ..Board::default()
        };
        b.layers.insert(
            id(1),
            Layer {
                name: "F.Cu".to_owned(),
                kind: eh3d_phoenix_board::LayerKind::Copper,
                side: None,
            },
        );
        let mut r = Footprint {
            name: "R0603".to_owned(),
            pads: Default::default(),
        };
        r.pads
            .insert(id(100), pad("1", (-800_000, 0), 800_000, 1_000_000));
        r.pads
            .insert(id(101), pad("2", (900_000, 0), 800_000, 1_000_000));
        b.footprints.insert(id(10), r);
        let mut s = Footprint {
            name: "SOT23-5".to_owned(),
            pads: Default::default(),
        };
        s.pads.insert(id(110), pad("1", (0, 0), MM, MM));
        b.footprints.insert(id(11), s);
        b.components.insert(
            id(20),
            part("R2", 10, (10 * MM, 10 * MM), 90_000_000, Side::Top),
        );
        b.components
            .insert(id(21), part("R10", 10, (12 * MM, 10 * MM), 0, Side::Bottom));
        b.components.insert(
            id(22),
            part("U1", 11, (20 * MM, 8 * MM), 30_000_000, Side::Top),
        );
        b
    }

    fn none() -> BTreeSet<String> {
        BTreeSet::new()
    }

    #[test]
    fn the_settings_are_read_and_the_wrong_ones_refused() {
        let s = PnpSettings::parse(
            "origin = aux\naux_x_mm = 1.5\nunits = mil\ndecimals = 2\ncentre = pads\n\
             bottom_rotation = mirror\nnot_fitted = mark\ndelimiter = semicolon\n\
             rotate = SOT23* 180\n[acme]\nrotate = R0603 90\n",
        )
        .unwrap();
        assert_eq!(s.origin, PnpOrigin::Aux);
        assert_eq!(s.aux, (1_500_000, 0));
        assert_eq!(s.unit, Unit::Mil);
        assert_eq!(s.centre, Centre::Pads);
        assert_eq!(s.delimiter, ';');
        assert_eq!(s.makers, vec!["acme".to_owned()]);
        assert_eq!(s.rules.len(), 2);
        for text in [
            "nope = 1\n",
            "origin = middle\n",
            "decimals = 7\n",
            "rotate = A\n",
            "rotate = A 1 2\n",
            "rotate = A x\n",
            "units\n",
            "[a]\n[a]\n",
            "[]\n",
            "[a]\ndecimals = 2\n",
            "aux_x_mm = 5000\n",
        ] {
            let e = PnpSettings::parse(text).unwrap_err();
            assert_eq!(e.code, PNP_SETTINGS_INVALID, "{text}");
        }
        assert_eq!(PnpSettings::parse("").unwrap(), PnpSettings::default());
    }

    #[test]
    fn the_settings_that_come_with_the_program_are_the_defaults() {
        let shipped = include_str!("../../../config/pnp_rotations.cfg");
        assert_eq!(PnpSettings::parse(shipped).unwrap(), PnpSettings::default());
    }

    #[test]
    fn the_table_of_rotations_tries_the_maker_first() {
        let mut s = PnpSettings::parse(
            "rotate = SOT23* 180\nrotate = R0603 90\n[acme]\nrotate = R0603 270\n",
        )
        .unwrap();
        assert_eq!(s.extra_rotation("SOT23-5"), 180_000_000);
        assert_eq!(s.extra_rotation("R0603"), 90_000_000);
        assert_eq!(s.extra_rotation("C0805"), 0);
        s.select_maker("acme").unwrap();
        assert_eq!(s.extra_rotation("R0603"), 270_000_000);
        assert_eq!(s.extra_rotation("SOT23-5"), 180_000_000);
        assert!(s.select_maker("other").is_err());
    }

    #[test]
    fn the_centre_of_a_part_is_the_middle_of_its_pads() {
        let b = board();
        let mut s = PnpSettings::default();
        // R2 is turned 90 degrees at (10, 10) mm: the pads are at y = 10 - 0.8 and 10 + 0.9; the
        // box of the pads runs from 10 - 0.8 - 0.4 = 8.8 (their sides are 0.8 mm along x, turned
        // to y) to 10 + 0.9 + 0.4 = 11.3, so the middle is 10.05
        let p = plan(&b, &s, &none());
        let r2 = p.top.iter().find(|r| r.refdes == "R2").unwrap();
        assert_eq!(r2.x_nm, 5 * MM);
        assert_eq!(r2.y_nm, 5 * MM + 50_000);
        s.centre = Centre::Origin;
        let p = plan(&b, &s, &none());
        let r2 = p.top.iter().find(|r| r.refdes == "R2").unwrap();
        assert_eq!((r2.x_nm, r2.y_nm), (5 * MM, 5 * MM));
        s.centre = Centre::Pads;
        let p = plan(&b, &s, &none());
        let r2 = p.top.iter().find(|r| r.refdes == "R2").unwrap();
        assert_eq!(r2.y_nm, 5 * MM + 50_000);
    }

    #[test]
    fn the_origin_and_the_faces_and_the_order_follow_the_settings() {
        let b = board();
        let mut s = PnpSettings::default();
        let p = plan(&b, &s, &none());
        assert_eq!(
            p.top.iter().map(|r| r.refdes.as_str()).collect::<Vec<_>>(),
            vec!["R2", "U1"]
        );
        assert_eq!(p.bottom.len(), 1);
        assert_eq!(p.bottom[0].refdes, "R10");
        // the bottom part is mirrored about the vertical axis: its pads run the other way, but a
        // symmetric pair about the box keeps the middle at x = 12 - 0.05
        assert_eq!(p.bottom[0].x_nm, 7 * MM - 50_000);
        s.origin = PnpOrigin::Aux;
        s.aux = (MM, MM);
        let p = plan(&b, &s, &none());
        let u1 = p.top.iter().find(|r| r.refdes == "U1").unwrap();
        assert_eq!((u1.x_nm, u1.y_nm), (19 * MM, 7 * MM));
        s.origin = PnpOrigin::Panel;
        s.panel = (10 * MM, 20 * MM);
        let p = plan(&b, &s, &none());
        let u1 = p.top.iter().find(|r| r.refdes == "U1").unwrap();
        assert_eq!((u1.x_nm, u1.y_nm), (25 * MM, 23 * MM));
    }

    #[test]
    fn the_extra_rotation_and_the_bottom_rule_are_applied() {
        let b = board();
        let mut s = PnpSettings::parse("rotate = SOT23* 180\n").unwrap();
        let p = plan(&b, &s, &none());
        let u1 = p.top.iter().find(|r| r.refdes == "U1").unwrap();
        assert_eq!(u1.rotation, 210_000_000);
        assert!(p
            .findings
            .iter()
            .any(|f| f.code == PNP_ROTATION_CORRECTED && f.detail == "1 part(s)"));
        s.bottom_rotation = BottomRotation::HalfTurn;
        let p = plan(&b, &s, &none());
        assert_eq!(p.bottom[0].rotation, 180_000_000);
    }

    #[test]
    fn parts_that_are_not_fitted_are_left_out_or_marked() {
        let b = board();
        let mut off = BTreeSet::new();
        off.insert("U1".to_owned());
        let mut s = PnpSettings::default();
        let p = plan(&b, &s, &off);
        assert_eq!(p.top.len(), 1);
        assert_eq!(p.excluded.len(), 1);
        assert_eq!(p.excluded[0].refdes, "U1");
        assert!(pnp_json(&p, Side::Top, &s).contains("\"not_fitted\": [\"U1\"]"));
        s.not_fitted = NotFitted::Mark;
        let p = plan(&b, &s, &off);
        assert_eq!(p.top.len(), 2);
        assert!(p.excluded.is_empty());
        let csv = pnp_csv(&p.top, &s);
        assert!(csv.lines().next().unwrap().ends_with(",Status"));
        assert!(csv.contains(",not_fitted\n"));
        assert!(csv.contains(",fitted\n"));
    }

    #[test]
    fn what_is_wrong_with_the_board_is_reported() {
        let mut b = board();
        b.components
            .insert(id(23), part("R2", 99, (50 * MM, 50 * MM), 0, Side::Top));
        b.components
            .insert(id(24), part("", 11, (10 * MM, 10 * MM), 0, Side::Top));
        let empty = Footprint {
            name: "EMPTY".to_owned(),
            pads: Default::default(),
        };
        b.footprints.insert(id(12), empty);
        b.components
            .insert(id(25), part("X1", 12, (10 * MM, 10 * MM), 0, Side::Top));
        let p = plan(&b, &PnpSettings::default(), &none());
        let codes: Vec<&str> = p.findings.iter().map(|f| f.code).collect();
        for code in [
            PNP_DUPLICATE,
            PNP_NO_FOOTPRINT,
            PNP_NO_REFDES,
            PNP_OUTSIDE,
            PNP_NO_PADS,
        ] {
            assert!(codes.contains(&code), "{code}");
        }
        assert_eq!(codes.iter().filter(|c| **c == PNP_DUPLICATE).count(), 1);
        let empty_board = plan(&Board::default(), &PnpSettings::default(), &none());
        assert_eq!(empty_board.findings[0].code, PNP_NO_COMPONENTS);
    }

    #[test]
    fn lengths_are_written_in_the_unit_with_rounding_half_away_from_zero() {
        assert_eq!(length_text(1_234_567, Unit::Mm, 3), "1.235");
        assert_eq!(length_text(-1_234_567, Unit::Mm, 3), "-1.235");
        assert_eq!(length_text(25_400, Unit::Mil, 2), "1.00");
        assert_eq!(length_text(-25_400, Unit::Mil, 0), "-1");
        assert_eq!(length_text(MM, Unit::Mil, 3), "39.370");
        assert_eq!(length_text(0, Unit::Mil, 1), "0.0");
    }

    #[test]
    fn the_csv_and_the_json_are_written_the_same_every_time() {
        let b = board();
        let s = PnpSettings::default();
        let p = plan(&b, &s, &none());
        let csv = pnp_csv(&p.top, &s);
        assert_eq!(
            csv,
            "Ref,Value,Package,PosX(mm),PosY(mm),Rot(deg),Side\n\
             R2,10k,R0603,5.0000,5.0500,90.00,top\n\
             U1,10k,SOT23-5,15.0000,3.0000,30.00,top\n"
        );
        assert_eq!(csv, pnp_csv(&plan(&b, &s, &none()).top, &s));
        let json = pnp_json(&p, Side::Bottom, &s);
        assert!(json.starts_with("{\n  \"format\": \"phoenix-pnp/1\",\n"));
        assert!(json.contains("\"side\": \"bottom\""));
        assert!(json.contains("\"ref\": \"R10\""));
        assert!(json.ends_with("  \"not_fitted\": []\n}\n"));
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["parts"][0]["x"], 6.95);
        let top = pnp_json(&p, Side::Top, &s);
        assert!(serde_json::from_str::<serde_json::Value>(&top).is_ok());
    }
}
