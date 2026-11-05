// SPDX-License-Identifier: GPL-3.0-or-later
//! Generators of footprints from parameters: chip components from 0201 to 2512, SOIC, TSSOP, QFN,
//! QFP, DIP and pin headers in one row or two.
//!
//! The land of a lead follows the method of IPC-7351B: the dimensions of the lead plus a fillet
//! goal at the toe, the heel and the side, with the tolerances of the component, of the
//! fabrication and of the placement added in quadrature. Every dimension, tolerance and goal is a
//! value of `config/footprints.cfg`, per family and variant, and the file says that they are a
//! prototype and have not been checked against the standard or a datasheet. The footprints are
//! drawn on the layers of EAGLE: copper, silkscreen (21), the texts of name and value (25 and 27),
//! the keep-out used as courtyard (39) and the documentation layer used as assembly outline (51).

use std::collections::BTreeMap;

use crate::codes::{LibError, PARAM_INVALID, SETTINGS_INVALID};
use crate::geom::items_bounds;
use crate::model::{
    Circle, Item, Package, Pad, PadShape, Smd, Text, Wire, LAYER_TOP, LAYER_T_DOCU,
    LAYER_T_KEEPOUT, LAYER_T_NAMES, LAYER_T_PLACE, LAYER_T_VALUES,
};
use crate::write::fmt_mm;

/// The text of `config/footprints.cfg`, the values the generators start with.
pub const DEFAULT_CONFIG: &str = include_str!("../../../config/footprints.cfg");

/// The families of footprints that can be generated.
pub const FAMILIES: [&str; 7] = ["chip", "soic", "tssop", "qfn", "qfp", "dip", "header"];

const AREAS: [&str; 6] = ["meta", "ipc", "silk", "courtyard", "assembly", "text"];

/// The values of the generators: numbers by key, and the texts of the `meta` lines.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FootprintConfig {
    numbers: BTreeMap<String, f64>,
    texts: BTreeMap<String, String>,
}

impl FootprintConfig {
    /// Parses a settings text. Blank lines and lines starting with `#` are ignored; an unknown
    /// family or area, a value that is not a finite number or a negative value (except for the
    /// fillet goals) is refused with its line number.
    pub fn parse(text: &str) -> Result<FootprintConfig, LibError> {
        let mut cfg = FootprintConfig::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let bad = |what: &str| LibError::new(SETTINGS_INVALID, format!("line {line}: {what}"));
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| bad("not key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            let area = key.split('.').next().unwrap_or("");
            if !AREAS.contains(&area) && !FAMILIES.contains(&area) {
                return Err(bad(&format!("unknown family or area in {key}")));
            }
            if area == "meta" {
                cfg.texts.insert(key.to_owned(), value.to_owned());
                continue;
            }
            let number: f64 = value
                .parse()
                .map_err(|_| bad(&format!("the value of {key} is not a number")))?;
            if !number.is_finite() {
                return Err(bad(&format!("the value of {key} is not finite")));
            }
            let goal = key.ends_with(".toe") || key.ends_with(".heel") || key.ends_with(".side");
            if number < 0.0 && !goal {
                return Err(bad(&format!("the value of {key} is negative")));
            }
            cfg.numbers.insert(key.to_owned(), number);
        }
        Ok(cfg)
    }

    /// The values of `config/footprints.cfg` as they were when the program was built.
    pub fn builtin() -> FootprintConfig {
        FootprintConfig::parse(DEFAULT_CONFIG).unwrap_or_default()
    }

    /// A `meta` text such as `meta.status`.
    pub fn text(&self, key: &str) -> Option<&str> {
        self.texts.get(key).map(String::as_str)
    }

    /// The value of `family.variant.name`, or else of `family.name`.
    pub fn number(&self, family: &str, variant: &str, name: &str) -> Option<f64> {
        self.numbers
            .get(&format!("{family}.{variant}.{name}"))
            .or_else(|| self.numbers.get(&format!("{family}.{name}")))
            .copied()
    }

    fn need(&self, family: &str, variant: &str, name: &str) -> Result<f64, LibError> {
        self.number(family, variant, name).ok_or_else(|| {
            LibError::new(
                SETTINGS_INVALID,
                format!("no value for {family}.{variant}.{name}"),
            )
        })
    }

    fn flat(&self, key: &str) -> Result<f64, LibError> {
        self.numbers
            .get(key)
            .copied()
            .ok_or_else(|| LibError::new(SETTINGS_INVALID, format!("no value for {key}")))
    }

    /// The variants a family has, as they are written in the file, sorted.
    pub fn variants(&self, family: &str) -> Vec<String> {
        let prefix = format!("{family}.");
        let mut out: Vec<String> = self
            .numbers
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix))
            .filter_map(|rest| rest.split_once('.'))
            .map(|(variant, _)| variant.to_owned())
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// A footprint made by a generator, with the notes to show to the person who asked for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Generated {
    /// The footprint.
    pub package: Package,
    /// The first note says that the values are a prototype; the others give the dimensions.
    pub notes: Vec<String>,
}

fn bad(what: impl Into<String>) -> LibError {
    LibError::new(PARAM_INVALID, what)
}

/// Millimetres as nanometres.
fn nm(mm: f64) -> i64 {
    (mm * 1_000_000.0).round() as i64
}

/// A value rounded to the nearest multiple of the grid.
fn snap(value: i64, grid: i64) -> i64 {
    if grid <= 0 {
        return value;
    }
    ((value as f64 / grid as f64).round() as i64) * grid
}

/// The drawing values shared by every family.
struct Common {
    grid: i64,
    silk_w: i64,
    silk_gap: i64,
    bracket: i64,
    dot_r: i64,
    court_excess: i64,
    court_w: i64,
    asm_w: i64,
    text_size: i64,
    text_gap: i64,
    fab: f64,
    place: f64,
}

fn common(cfg: &FootprintConfig) -> Result<Common, LibError> {
    let grid = nm(cfg.flat("ipc.round")?);
    if grid <= 0 {
        return Err(LibError::new(
            SETTINGS_INVALID,
            "ipc.round must be above zero",
        ));
    }
    Ok(Common {
        grid,
        silk_w: nm(cfg.flat("silk.width")?),
        silk_gap: nm(cfg.flat("silk.gap")?),
        bracket: nm(cfg.flat("silk.bracket")?),
        dot_r: nm(cfg.flat("silk.dot_r")?),
        court_excess: nm(cfg.flat("courtyard.excess")?),
        court_w: nm(cfg.flat("courtyard.width")?),
        asm_w: nm(cfg.flat("assembly.width")?),
        text_size: nm(cfg.flat("text.size")?),
        text_gap: nm(cfg.flat("text.gap")?),
        fab: cfg.flat("ipc.fab_tol")?,
        place: cfg.flat("ipc.place_tol")?,
    })
}

/// The dimensions of a lead and the fillet goals, in millimetres.
struct Dims {
    l: f64,
    l_tol: f64,
    t: f64,
    t_tol: f64,
    w: f64,
    w_tol: f64,
    toe: f64,
    heel: f64,
    side: f64,
}

/// A land: its length, its width and the distance of its centre from the axis, in nanometres.
struct Land {
    len: i64,
    width: i64,
    centre: i64,
}

/// The land of a lead by the method of IPC-7351B.
fn land(d: &Dims, c: &Common) -> Result<Land, LibError> {
    let lmin = d.l - d.l_tol;
    let lmax = d.l + d.l_tol;
    let cl = lmax - lmin;
    let tmin = d.t - d.t_tol;
    let ct = 2.0 * d.t_tol;
    let smax = lmax - 2.0 * tmin;
    let cs = (cl * cl + 2.0 * ct * ct).sqrt();
    let wmin = d.w - d.w_tol;
    let cw = 2.0 * d.w_tol;
    let fp = c.fab * c.fab + c.place * c.place;
    let zmax = lmin + 2.0 * d.toe + (cl * cl + fp).sqrt();
    let gmin = smax - 2.0 * d.heel - (cs * cs + fp).sqrt();
    let xmax = wmin + 2.0 * d.side + (cw * cw + fp).sqrt();
    if gmin <= 0.0 || zmax <= gmin || xmax <= 0.0 {
        return Err(bad(
            "the dimensions give lands that touch or have no size; check the values of the family",
        ));
    }
    let len = snap(nm((zmax - gmin) / 2.0), c.grid).max(c.grid);
    let width = snap(nm(xmax), c.grid).max(c.grid);
    let centre = snap(nm((zmax + gmin) / 4.0), c.grid);
    if centre <= len / 2 {
        return Err(bad("the lands of the two sides would cross the axis"));
    }
    Ok(Land { len, width, centre })
}

fn smd_pad(name: &str, x: i64, y: i64, dx: i64, dy: i64) -> Item {
    Item::Smd(Smd {
        name: name.to_owned(),
        x,
        y,
        dx,
        dy,
        layer: LAYER_TOP,
        ..Smd::default()
    })
}

fn line(layer: u32, x1: i64, y1: i64, x2: i64, y2: i64, width: i64) -> Item {
    Item::Wire(Wire {
        layer,
        x1,
        y1,
        x2,
        y2,
        width,
        ..Wire::default()
    })
}

fn box_lines(layer: u32, x1: i64, y1: i64, x2: i64, y2: i64, width: i64) -> Vec<Item> {
    vec![
        line(layer, x1, y1, x2, y1, width),
        line(layer, x2, y1, x2, y2, width),
        line(layer, x2, y2, x1, y2, width),
        line(layer, x1, y2, x1, y1, width),
    ]
}

fn dot(layer: u32, x: i64, y: i64, radius: i64) -> Item {
    Item::Circle(Circle {
        layer,
        x,
        y,
        radius,
        width: 0,
        extra: Vec::new(),
    })
}

fn text(layer: u32, x: i64, y: i64, size: i64, content: &str) -> Item {
    Item::Text(Text {
        layer,
        x,
        y,
        size,
        content: content.to_owned(),
        ..Text::default()
    })
}

/// Adds to the pads and the silkscreen the assembly outline, the courtyard and the texts, and
/// makes the footprint.
fn finish(
    name: String,
    mut items: Vec<Item>,
    body: (i64, i64, i64, i64),
    c: &Common,
    notes: Vec<String>,
) -> Result<Generated, LibError> {
    let pads: Vec<Item> = items.iter().filter(|i| i.is_pad()).cloned().collect();
    let Some(b) = items_bounds(&pads) else {
        return Err(bad("the footprint has no pads"));
    };
    let (x1, y1) = (b.min_x - c.court_excess, b.min_y - c.court_excess);
    let (x2, y2) = (b.max_x + c.court_excess, b.max_y + c.court_excess);
    items.extend(box_lines(
        LAYER_T_DOCU,
        body.0,
        body.1,
        body.2,
        body.3,
        c.asm_w,
    ));
    items.extend(box_lines(LAYER_T_KEEPOUT, x1, y1, x2, y2, c.court_w));
    items.push(text(
        LAYER_T_NAMES,
        x1,
        y2 + c.text_gap,
        c.text_size,
        ">NAME",
    ));
    items.push(text(
        LAYER_T_VALUES,
        x1,
        y1 - c.text_gap - c.text_size,
        c.text_size,
        ">VALUE",
    ));
    Ok(Generated {
        package: Package {
            name,
            description: Vec::new(),
            items,
            extra: Vec::new(),
        },
        notes,
    })
}

fn known(
    params: &BTreeMap<String, String>,
    family: &str,
    allowed: &[&str],
) -> Result<(), LibError> {
    for key in params.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(bad(format!(
                "{key} is not a parameter of {family} (it has {})",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

fn word<'a>(params: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    params.get(key).map(String::as_str)
}

fn need_word<'a>(params: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, LibError> {
    word(params, key).ok_or_else(|| bad(format!("the parameter {key} is needed")))
}

fn clean_word(text: &str) -> Result<String, LibError> {
    let lower = text.trim().to_ascii_lowercase().replace('.', "_");
    if lower.is_empty()
        || !lower
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(bad(format!("{text:?} is not a valid name of a variant")));
    }
    Ok(lower)
}

fn count(
    params: &BTreeMap<String, String>,
    key: &str,
    min: usize,
    max: usize,
    multiple: usize,
) -> Result<usize, LibError> {
    let n: usize = need_word(params, key)?
        .trim()
        .parse()
        .map_err(|_| bad(format!("{key} is not a whole number")))?;
    if !(min..=max).contains(&n) || (n / multiple) * multiple != n {
        return Err(bad(format!(
            "{key} must be a multiple of {multiple} from {min} to {max}"
        )));
    }
    Ok(n)
}

fn millimetres(text: &str, key: &str, max: f64) -> Result<f64, LibError> {
    let v: f64 = text
        .trim()
        .parse()
        .map_err(|_| bad(format!("{key} is not a number")))?;
    if !v.is_finite() || v <= 0.0 || v > max {
        return Err(bad(format!(
            "{key} must be above zero and not above {max} mm"
        )));
    }
    Ok(v)
}

/// The text of a length in nanometres as it goes in a name: `5`, `2_54`.
fn in_name(value: i64) -> String {
    fmt_mm(value).replace('.', "_")
}

fn chip(
    cfg: &FootprintConfig,
    params: &BTreeMap<String, String>,
    c: &Common,
) -> Result<Generated, LibError> {
    known(params, "chip", &["size"])?;
    let size = clean_word(need_word(params, "size")?)?;
    if cfg.number("chip", &size, "length").is_none() {
        return Err(bad(format!(
            "no chip size {size}; the file has {}",
            cfg.variants("chip").join(", ")
        )));
    }
    let n = |name: &str| cfg.need("chip", &size, name);
    let dims = Dims {
        l: n("length")?,
        l_tol: n("length_tol")?,
        t: n("term")?,
        t_tol: n("term_tol")?,
        w: n("width")?,
        w_tol: n("width_tol")?,
        toe: n("toe")?,
        heel: n("heel")?,
        side: n("side")?,
    };
    let land = land(&dims, c)?;
    let reach = land.centre + land.len / 2;
    let silk_y = land.width / 2 + c.silk_gap + c.silk_w / 2;
    let items = vec![
        smd_pad("1", -land.centre, 0, land.len, land.width),
        smd_pad("2", land.centre, 0, land.len, land.width),
        line(LAYER_T_PLACE, -reach, silk_y, reach, silk_y, c.silk_w),
        line(LAYER_T_PLACE, -reach, -silk_y, reach, -silk_y, c.silk_w),
    ];
    let (bx, by) = (nm(dims.l) / 2, nm(dims.w) / 2);
    let notes = vec![format!(
        "pads {} x {} mm, centres {} mm apart",
        fmt_mm(land.len),
        fmt_mm(land.width),
        fmt_mm(2 * land.centre)
    )];
    finish(
        format!("CHIP_{}", size.to_ascii_uppercase()),
        items,
        (-bx, -by, bx, by),
        c,
        notes,
    )
}

/// The variant of a family: the `variant` parameter, or the pitch, or the default.
fn variant_of(
    params: &BTreeMap<String, String>,
    default: Option<&str>,
    family: &str,
) -> Result<String, LibError> {
    if let Some(v) = word(params, "variant") {
        return clean_word(v);
    }
    if let Some(p) = word(params, "pitch") {
        let mm = millimetres(p, "pitch", 10.0)?;
        return Ok(format!("p{}", in_name(nm(mm))));
    }
    match default {
        Some(d) => Ok(d.to_owned()),
        None => Err(bad(format!(
            "{family} needs the parameter variant or pitch"
        ))),
    }
}

fn dual(
    family: &str,
    default_variant: &str,
    cfg: &FootprintConfig,
    params: &BTreeMap<String, String>,
    c: &Common,
) -> Result<Generated, LibError> {
    known(params, family, &["pins", "variant"])?;
    let pins = count(params, "pins", 4, 64, 2)?;
    let variant = variant_of(params, Some(default_variant), family)?;
    if cfg.number(family, &variant, "span").is_none() {
        return Err(bad(format!(
            "no variant {variant} of {family}; the file has {}",
            cfg.variants(family).join(", ")
        )));
    }
    let n = |name: &str| cfg.need(family, &variant, name);
    let dims = Dims {
        l: n("span")?,
        l_tol: n("span_tol")?,
        t: n("lead")?,
        t_tol: n("lead_tol")?,
        w: n("lead_w")?,
        w_tol: n("lead_w_tol")?,
        toe: n("toe")?,
        heel: n("heel")?,
        side: n("side")?,
    };
    let land = land(&dims, c)?;
    let pitch = nm(n("pitch")?);
    let body_w = nm(n("body_w")?);
    let trim = nm(n("body_trim")?);
    if pitch <= land.width {
        return Err(bad("the pitch is not larger than the width of the lands"));
    }
    let m = pins / 2;
    let ytop = (m as i64 - 1) * pitch / 2;
    let mut items = Vec::new();
    for i in 0..m {
        let y = ytop - i as i64 * pitch;
        items.push(smd_pad(
            &(i + 1).to_string(),
            -land.centre,
            y,
            land.len,
            land.width,
        ));
    }
    for i in 0..m {
        let y = ytop - i as i64 * pitch;
        items.push(smd_pad(
            &(m + 1 + i).to_string(),
            land.centre,
            -y,
            land.len,
            land.width,
        ));
    }
    let silk_y = ytop + land.width / 2 + c.silk_gap + c.silk_w / 2;
    items.push(line(
        LAYER_T_PLACE,
        -body_w / 2,
        silk_y,
        body_w / 2,
        silk_y,
        c.silk_w,
    ));
    items.push(line(
        LAYER_T_PLACE,
        -body_w / 2,
        -silk_y,
        body_w / 2,
        -silk_y,
        c.silk_w,
    ));
    items.push(dot(
        LAYER_T_PLACE,
        -(land.centre + land.len / 2 + c.silk_gap + c.dot_r),
        ytop,
        c.dot_r,
    ));
    let half_len = (m as i64 * pitch - trim) / 2;
    let label = match variant.as_str() {
        "narrow" => "N".to_owned(),
        "wide" => "W".to_owned(),
        other => other.to_ascii_uppercase(),
    };
    let notes = vec![format!(
        "pads {} x {} mm, columns {} mm apart, pitch {} mm",
        fmt_mm(land.len),
        fmt_mm(land.width),
        fmt_mm(2 * land.centre),
        fmt_mm(pitch)
    )];
    finish(
        format!("{}{}_{}", family.to_ascii_uppercase(), pins, label),
        items,
        (-body_w / 2, -half_len, body_w / 2, half_len),
        c,
        notes,
    )
}

fn quad(
    is_qfn: bool,
    cfg: &FootprintConfig,
    params: &BTreeMap<String, String>,
    c: &Common,
) -> Result<Generated, LibError> {
    let family = if is_qfn { "qfn" } else { "qfp" };
    if is_qfn {
        known(params, family, &["pins", "body", "variant", "pitch", "ep"])?;
    } else {
        known(params, family, &["pins", "body", "variant", "pitch"])?;
    }
    let pins = count(params, "pins", 8, 256, 4)?;
    let body_mm = millimetres(need_word(params, "body")?, "body", 50.0)?;
    let variant = variant_of(params, None, family)?;
    if cfg.number(family, &variant, "pitch").is_none() {
        return Err(bad(format!(
            "no variant {variant} of {family}; the file has {}",
            cfg.variants(family).join(", ")
        )));
    }
    let n = |name: &str| cfg.need(family, &variant, name);
    let (l, l_tol) = if is_qfn {
        (body_mm, n("body_tol")?)
    } else {
        (body_mm + n("span_extra")?, n("span_tol")?)
    };
    let dims = Dims {
        l,
        l_tol,
        t: n("lead")?,
        t_tol: n("lead_tol")?,
        w: n("lead_w")?,
        w_tol: n("lead_w_tol")?,
        toe: n("toe")?,
        heel: n("heel")?,
        side: n("side")?,
    };
    let land = land(&dims, c)?;
    let pitch = nm(n("pitch")?);
    if pitch <= land.width {
        return Err(bad("the pitch is not larger than the width of the lands"));
    }
    let m = pins / 4;
    let ytop = (m as i64 - 1) * pitch / 2;
    if ytop + land.width / 2 >= land.centre - land.len / 2 {
        return Err(bad(
            "the pads of neighbouring sides would touch: too many pins for the body",
        ));
    }
    let along = |i: usize| ytop - i as i64 * pitch;
    let mut items = Vec::new();
    // left side from the top down, bottom side from the left to the right, right side from the
    // bottom up and top side from the right to the left
    for i in 0..m {
        items.push(smd_pad(
            &(i + 1).to_string(),
            -land.centre,
            along(i),
            land.len,
            land.width,
        ));
    }
    for i in 0..m {
        items.push(smd_pad(
            &(m + 1 + i).to_string(),
            -along(i),
            -land.centre,
            land.width,
            land.len,
        ));
    }
    for i in 0..m {
        items.push(smd_pad(
            &(2 * m + 1 + i).to_string(),
            land.centre,
            -along(i),
            land.len,
            land.width,
        ));
    }
    for i in 0..m {
        items.push(smd_pad(
            &(3 * m + 1 + i).to_string(),
            along(i),
            land.centre,
            land.width,
            land.len,
        ));
    }
    let mut notes = vec![format!(
        "pads {} x {} mm, rows {} mm apart, pitch {} mm",
        fmt_mm(land.len),
        fmt_mm(land.width),
        fmt_mm(2 * land.centre),
        fmt_mm(pitch)
    )];
    let mut name = format!(
        "{}{}_{}X{}_{}",
        family.to_ascii_uppercase(),
        pins,
        in_name(nm(body_mm)),
        in_name(nm(body_mm)),
        variant.to_ascii_uppercase()
    );
    if is_qfn {
        if let Some(ep) = word(params, "ep") {
            let ep_nm = nm(millimetres(ep, "ep", 50.0)?);
            if ep_nm / 2 + c.silk_gap > land.centre - land.len / 2 {
                return Err(bad("the exposed pad would touch the lands"));
            }
            items.push(smd_pad("EP", 0, 0, ep_nm, ep_nm));
            name.push_str("_EP");
            notes.push(format!("exposed pad {} mm square", fmt_mm(ep_nm)));
        }
    }
    let reach = land.centre + land.len / 2 + c.silk_gap + c.silk_w / 2;
    if c.bracket >= reach {
        return Err(bad("the corner marks are longer than the package"));
    }
    for (sx, sy) in [(-1i64, -1i64), (-1, 1), (1, -1), (1, 1)] {
        items.push(line(
            LAYER_T_PLACE,
            sx * reach,
            sy * reach,
            sx * (reach - c.bracket),
            sy * reach,
            c.silk_w,
        ));
        items.push(line(
            LAYER_T_PLACE,
            sx * reach,
            sy * reach,
            sx * reach,
            sy * (reach - c.bracket),
            c.silk_w,
        ));
    }
    items.push(dot(
        LAYER_T_PLACE,
        -(land.centre + land.len / 2 + c.silk_gap + c.dot_r),
        ytop,
        c.dot_r,
    ));
    let half = nm(body_mm) / 2;
    finish(name, items, (-half, -half, half, half), c, notes)
}

fn dip(
    cfg: &FootprintConfig,
    params: &BTreeMap<String, String>,
    c: &Common,
) -> Result<Generated, LibError> {
    known(params, "dip", &["pins", "row"])?;
    let pins = count(params, "pins", 4, 64, 2)?;
    let row = match word(params, "row").map(str::trim) {
        None | Some("narrow") => nm(cfg.flat("dip.row_narrow")?),
        Some("wide") => nm(cfg.flat("dip.row_wide")?),
        Some(other) => nm(millimetres(other, "row", 50.0)?),
    };
    let pitch = nm(cfg.flat("dip.pitch")?);
    let drill = nm(cfg.flat("dip.drill")?);
    let pad_d = nm(cfg.flat("dip.pad_d")?);
    if pad_d <= drill || pitch <= pad_d {
        return Err(bad(
            "the pad must be larger than the drill and smaller than the pitch",
        ));
    }
    let side = row / 2 - pad_d / 2 - c.silk_gap - c.silk_w / 2;
    if side <= 0 {
        return Err(bad(
            "the rows are too close for the silkscreen between them",
        ));
    }
    let m = pins / 2;
    let ytop = (m as i64 - 1) * pitch / 2;
    let mut items = Vec::new();
    let pad = |name: String, x: i64, y: i64| {
        Item::Pad(Pad {
            name,
            x,
            y,
            drill,
            diameter: pad_d,
            shape: PadShape::Round,
            ..Pad::default()
        })
    };
    for i in 0..m {
        let y = ytop - i as i64 * pitch;
        let mut left = pad((i + 1).to_string(), -row / 2, y);
        if i == 0 {
            if let Item::Pad(p) = &mut left {
                p.shape = PadShape::Square;
            }
        }
        items.push(left);
    }
    for i in 0..m {
        let y = ytop - i as i64 * pitch;
        items.push(pad((m + 1 + i).to_string(), row / 2, -y));
    }
    let top = m as i64 * pitch / 2;
    items.push(line(LAYER_T_PLACE, -side, -top, -side, top, c.silk_w));
    items.push(line(LAYER_T_PLACE, side, -top, side, top, c.silk_w));
    items.push(line(LAYER_T_PLACE, -side, top, side, top, c.silk_w));
    items.push(line(LAYER_T_PLACE, -side, -top, side, -top, c.silk_w));
    items.push(dot(
        LAYER_T_PLACE,
        -(row / 2 + pad_d / 2 + c.silk_gap + c.dot_r),
        ytop,
        c.dot_r,
    ));
    let notes = vec![format!(
        "pads {} mm over a drill of {} mm, rows {} mm apart, pitch {} mm",
        fmt_mm(pad_d),
        fmt_mm(drill),
        fmt_mm(row),
        fmt_mm(pitch)
    )];
    finish(
        format!("DIP{}_W{}", pins, in_name(row)),
        items,
        (-side, -top, side, top),
        c,
        notes,
    )
}

fn header(
    cfg: &FootprintConfig,
    params: &BTreeMap<String, String>,
    c: &Common,
) -> Result<Generated, LibError> {
    known(params, "header", &["pins", "rows", "pitch"])?;
    let per_row = count(params, "pins", 1, 64, 1)?;
    let rows = match word(params, "rows").map(str::trim) {
        None | Some("1") => 1usize,
        Some("2") => 2,
        Some(_) => return Err(bad("rows is 1 or 2")),
    };
    let pitch = match word(params, "pitch") {
        Some(text) => nm(millimetres(text, "pitch", 10.0)?),
        None => nm(cfg.flat("header.pitch")?),
    };
    let drill = nm(cfg.flat("header.drill")?);
    let pad_d = nm(cfg.flat("header.pad_d")?);
    if pad_d <= drill || pitch <= pad_d {
        return Err(bad(
            "the pad must be larger than the drill and smaller than the pitch",
        ));
    }
    let mut items = Vec::new();
    for i in 0..per_row {
        for r in 0..rows {
            let number = if rows == 1 { i + 1 } else { 2 * i + 1 + r };
            let shape = if number == 1 {
                PadShape::Square
            } else {
                PadShape::Round
            };
            items.push(Item::Pad(Pad {
                name: number.to_string(),
                x: i as i64 * pitch,
                y: -(r as i64) * pitch,
                drill,
                diameter: pad_d,
                shape,
                ..Pad::default()
            }));
        }
    }
    let off = (pitch / 2).max(pad_d / 2 + c.silk_gap + c.silk_w / 2);
    let (x1, x2) = (-off, (per_row as i64 - 1) * pitch + off);
    let (y1, y2) = (-((rows as i64 - 1) * pitch) - off, off);
    items.extend(box_lines(LAYER_T_PLACE, x1, y1, x2, y2, c.silk_w));
    let notes = vec![format!(
        "pads {} mm over a drill of {} mm, pitch {} mm",
        fmt_mm(pad_d),
        fmt_mm(drill),
        fmt_mm(pitch)
    )];
    finish(
        format!("HDR{}X{}_P{}", rows, per_row, in_name(pitch)),
        items,
        (x1, y1, x2, y2),
        c,
        notes,
    )
}

/// Makes a footprint of a family from parameters. The same inputs always give the same footprint.
pub fn generate(
    family: &str,
    params: &BTreeMap<String, String>,
    cfg: &FootprintConfig,
) -> Result<Generated, LibError> {
    let c = common(cfg)?;
    let mut made = match family {
        "chip" => chip(cfg, params, &c)?,
        "soic" => dual("soic", "narrow", cfg, params, &c)?,
        "tssop" => dual("tssop", "4_4", cfg, params, &c)?,
        "qfn" => quad(true, cfg, params, &c)?,
        "qfp" => quad(false, cfg, params, &c)?,
        "dip" => dip(cfg, params, &c)?,
        "header" => header(cfg, params, &c)?,
        other => {
            return Err(bad(format!(
                "{other} is not a family; the families are {}",
                FAMILIES.join(", ")
            )))
        }
    };
    let standard = cfg.text("meta.standard").unwrap_or("IPC-7351B");
    let status = cfg
        .text("meta.status")
        .unwrap_or("prototype, not verified against a datasheet");
    made.notes
        .insert(0, format!("method of {standard}; values: {status}"));
    Ok(made)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::check_library;
    use crate::model::Library;
    use crate::settings::LibSettings;
    use crate::write::verify_round_trip;
    use std::sync::Arc;

    fn params(list: &[(&str, &str)]) -> BTreeMap<String, String> {
        list.iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn make(family: &str, list: &[(&str, &str)]) -> Generated {
        generate(family, &params(list), &FootprintConfig::builtin()).unwrap()
    }

    fn pad_at(p: &Package, name: &str) -> (i64, i64, i64, i64) {
        match p
            .items
            .iter()
            .find(|i| i.is_pad() && i.name() == Some(name))
        {
            Some(Item::Smd(s)) => (s.x, s.y, s.dx, s.dy),
            Some(Item::Pad(t)) => (t.x, t.y, t.drill, t.diameter),
            other => panic!("no pad {name}: {other:?}"),
        }
    }

    fn pad_count(p: &Package) -> usize {
        p.items.iter().filter(|i| i.is_pad()).count()
    }

    #[test]
    fn the_file_in_the_repository_parses_and_says_it_is_a_prototype() {
        let cfg = FootprintConfig::parse(DEFAULT_CONFIG).unwrap();
        assert_eq!(cfg, FootprintConfig::builtin());
        assert_eq!(cfg.text("meta.standard"), Some("IPC-7351B"));
        assert!(cfg.text("meta.status").unwrap().contains("prototype"));
        assert!(DEFAULT_CONFIG.contains("NOT been checked"));
        assert_eq!(cfg.number("chip", "0603", "length"), Some(1.6));
        // a family value is the fallback of a variant
        assert_eq!(cfg.number("chip", "0603", "heel"), Some(0.0));
        assert_eq!(cfg.number("chip", "0201", "toe"), Some(0.2));
        assert!(cfg.variants("chip").contains(&"2512".to_owned()));
        assert_eq!(
            cfg.variants("soic"),
            vec!["narrow".to_owned(), "wide".to_owned()]
        );
    }

    #[test]
    fn a_bad_line_of_the_file_is_refused_with_its_number() {
        for (text, needle) in [
            ("silk.width\n", "line 1"),
            ("fish.size = 3\n", "unknown family"),
            ("silk.width = x\n", "not a number"),
            ("silk.width = -1\n", "negative"),
            ("silk.width = inf\n", "not finite"),
        ] {
            let e = FootprintConfig::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        assert!(FootprintConfig::parse("chip.0603.toe = -0.05\n").is_ok());
    }

    #[test]
    fn a_chip_has_two_pads_that_follow_the_method() {
        let g = make("chip", &[("size", "0603")]);
        assert_eq!(g.package.name, "CHIP_0603");
        assert_eq!(pad_count(&g.package), 2);
        let (x1, y1, dx, dy) = pad_at(&g.package, "1");
        let (x2, y2, dx2, dy2) = pad_at(&g.package, "2");
        assert_eq!((x1, y1), (-x2, y2));
        assert_eq!((dx, dy), (dx2, dy2));
        // about 0.74 by 0.86 mm with the centres about 1.67 mm apart
        assert!((730_000..=750_000).contains(&dx), "{dx}");
        assert!((850_000..=870_000).contains(&dy), "{dy}");
        assert!((820_000..=850_000).contains(&x2), "{x2}");
        assert!(g.notes[0].contains("IPC-7351B") && g.notes[0].contains("prototype"));
    }

    #[test]
    fn larger_chips_get_larger_lands() {
        let small = make("chip", &[("size", "0402")]);
        let large = make("chip", &[("size", "2512")]);
        let (_, _, sdx, sdy) = pad_at(&small.package, "1");
        let (_, _, ldx, ldy) = pad_at(&large.package, "1");
        assert!(ldx > sdx && ldy > sdy);
        let (sx, ..) = pad_at(&small.package, "2");
        let (lx, ..) = pad_at(&large.package, "2");
        assert!(lx > sx);
    }

    #[test]
    fn a_soic_has_its_pads_in_two_columns_numbered_as_the_standard_does() {
        let g = make("soic", &[("pins", "8")]);
        assert_eq!(g.package.name, "SOIC8_N");
        assert_eq!(pad_count(&g.package), 8);
        let (x1, y1, ..) = pad_at(&g.package, "1");
        let (_, y2, ..) = pad_at(&g.package, "2");
        let (x4, y4, ..) = pad_at(&g.package, "4");
        let (x5, y5, ..) = pad_at(&g.package, "5");
        let (x8, y8, ..) = pad_at(&g.package, "8");
        assert!(x1 < 0 && y1 > 0);
        assert_eq!(y1 - y2, 1_270_000);
        assert_eq!((x4, y4), (x1, -y1));
        assert_eq!((x5, y5), (-x1, -y1));
        assert_eq!((x8, y8), (-x1, y1));
        let wide = make("soic", &[("pins", "16"), ("variant", "wide")]);
        assert_eq!(wide.package.name, "SOIC16_W");
        let (wx, ..) = pad_at(&wide.package, "1");
        assert!(wx < x1);
    }

    #[test]
    fn a_tssop_takes_the_variant_and_the_pitch_of_its_family() {
        let g = make("tssop", &[("pins", "16")]);
        assert_eq!(g.package.name, "TSSOP16_4_4");
        let (_, y1, ..) = pad_at(&g.package, "1");
        let (_, y2, ..) = pad_at(&g.package, "2");
        assert_eq!(y1 - y2, 650_000);
        let wide = make("tssop", &[("pins", "28"), ("variant", "6.1")]);
        assert_eq!(wide.package.name, "TSSOP28_6_1");
    }

    #[test]
    fn a_qfn_has_pads_on_four_sides_and_an_optional_exposed_pad() {
        let g = make(
            "qfn",
            &[
                ("pins", "16"),
                ("body", "3"),
                ("pitch", "0.5"),
                ("ep", "1.7"),
            ],
        );
        assert_eq!(g.package.name, "QFN16_3X3_P0_5_EP");
        assert_eq!(pad_count(&g.package), 17);
        let (x1, y1, ..) = pad_at(&g.package, "1");
        let (x5, y5, ..) = pad_at(&g.package, "5");
        let (x9, y9, ..) = pad_at(&g.package, "9");
        let (x13, y13, ..) = pad_at(&g.package, "13");
        assert!(x1 < 0 && y1 > 0);
        assert!(x5 < 0 && y5 < 0);
        assert!(x9 > 0 && y9 < 0);
        assert!(x13 > 0 && y13 > 0);
        let (ex, ey, edx, edy) = pad_at(&g.package, "EP");
        assert_eq!((ex, ey, edx, edy), (0, 0, 1_700_000, 1_700_000));
        // pads on the left and right are wider than they are high, those at the bottom the other way
        let (_, _, ldx, ldy) = pad_at(&g.package, "1");
        let (_, _, bdx, bdy) = pad_at(&g.package, "5");
        assert!(ldx > ldy && bdy > bdx);
    }

    #[test]
    fn a_qfp_names_its_body_and_pitch() {
        let g = make("qfp", &[("pins", "48"), ("body", "7"), ("variant", "p0_5")]);
        assert_eq!(g.package.name, "QFP48_7X7_P0_5");
        assert_eq!(pad_count(&g.package), 48);
    }

    #[test]
    fn a_dip_and_a_header_are_through_hole() {
        let g = make("dip", &[("pins", "8")]);
        assert_eq!(g.package.name, "DIP8_W7_62");
        assert_eq!(pad_count(&g.package), 8);
        let (x1, y1, drill, diameter) = pad_at(&g.package, "1");
        let (x8, y8, ..) = pad_at(&g.package, "8");
        assert_eq!(
            (x1, y1, drill, diameter),
            (-3_810_000, 3_810_000, 800_000, 1_600_000)
        );
        assert_eq!((x8, y8), (3_810_000, 3_810_000));
        match &g.package.items[0] {
            Item::Pad(p) => assert_eq!(p.shape, PadShape::Square),
            other => panic!("{other:?}"),
        }
        let wide = make("dip", &[("pins", "28"), ("row", "wide")]);
        assert_eq!(wide.package.name, "DIP28_W15_24");
        let h = make("header", &[("pins", "3"), ("rows", "2")]);
        assert_eq!(h.package.name, "HDR2X3_P2_54");
        assert_eq!(pad_count(&h.package), 6);
        assert_eq!(pad_at(&h.package, "1").0, 0);
        assert_eq!(
            pad_at(&h.package, "2"),
            (0, -2_540_000, 1_000_000, 1_700_000)
        );
        assert_eq!(pad_at(&h.package, "3").0, 2_540_000);
        let one = make("header", &[("pins", "4")]);
        assert_eq!(one.package.name, "HDR1X4_P2_54");
        assert_eq!(pad_at(&one.package, "4").0, 7_620_000);
    }

    #[test]
    fn every_family_gives_a_footprint_that_passes_the_checks_and_reads_back() {
        let cases: Vec<(&str, Vec<(&str, &str)>)> = vec![
            ("chip", vec![("size", "0201")]),
            ("chip", vec![("size", "0402")]),
            ("chip", vec![("size", "0603")]),
            ("chip", vec![("size", "0805")]),
            ("chip", vec![("size", "1206")]),
            ("chip", vec![("size", "1210")]),
            ("chip", vec![("size", "1812")]),
            ("chip", vec![("size", "2010")]),
            ("chip", vec![("size", "2512")]),
            ("soic", vec![("pins", "8")]),
            ("soic", vec![("pins", "16"), ("variant", "wide")]),
            ("tssop", vec![("pins", "14")]),
            ("tssop", vec![("pins", "28"), ("variant", "6_1")]),
            (
                "qfn",
                vec![
                    ("pins", "32"),
                    ("body", "5"),
                    ("variant", "p0_5"),
                    ("ep", "3.1"),
                ],
            ),
            (
                "qfn",
                vec![("pins", "16"), ("body", "3"), ("variant", "p0_5")],
            ),
            (
                "qfp",
                vec![("pins", "48"), ("body", "7"), ("variant", "p0_5")],
            ),
            (
                "qfp",
                vec![("pins", "100"), ("body", "14"), ("variant", "p0_5")],
            ),
            ("dip", vec![("pins", "8")]),
            ("dip", vec![("pins", "40"), ("row", "wide")]),
            ("header", vec![("pins", "6")]),
            ("header", vec![("pins", "5"), ("rows", "2")]),
        ];
        for (family, list) in cases {
            let g = make(family, &list);
            let mut lib = Library::empty();
            lib.packages.push(Arc::new(g.package));
            let report = check_library(&lib, &LibSettings::default());
            assert!(
                report.findings.is_empty(),
                "{family} {list:?}: {:?}",
                report.findings
            );
            assert!(verify_round_trip(&lib).is_ok(), "{family} {list:?}");
        }
    }

    #[test]
    fn the_same_parameters_always_give_the_same_footprint() {
        let a = make("qfp", &[("pins", "64"), ("body", "10"), ("pitch", "0.5")]);
        let b = make("qfp", &[("body", "10"), ("pitch", "0.50"), ("pins", "64")]);
        assert_eq!(a, b);
    }

    #[test]
    fn what_cannot_be_made_is_refused_with_the_code() {
        let cfg = FootprintConfig::builtin();
        let cases: Vec<(&str, Vec<(&str, &str)>)> = vec![
            ("capacitor", vec![]),
            ("chip", vec![]),
            ("chip", vec![("size", "9999")]),
            ("chip", vec![("size", "06 03")]),
            ("chip", vec![("size", "0603"), ("colour", "red")]),
            ("soic", vec![("pins", "7")]),
            ("soic", vec![("pins", "8"), ("variant", "huge")]),
            ("soic", vec![("pins", "x")]),
            ("tssop", vec![("pins", "200")]),
            ("qfn", vec![("pins", "16")]),
            ("qfn", vec![("pins", "16"), ("body", "3")]),
            ("qfn", vec![("pins", "17"), ("body", "3"), ("pitch", "0.5")]),
            (
                "qfn",
                vec![("pins", "16"), ("body", "3"), ("pitch", "0.5"), ("ep", "9")],
            ),
            (
                "qfp",
                vec![("pins", "48"), ("body", "-7"), ("pitch", "0.5")],
            ),
            ("dip", vec![("pins", "8"), ("row", "0")]),
            ("header", vec![("pins", "3"), ("rows", "3")]),
            ("header", vec![("pins", "3"), ("pitch", "1.0")]),
        ];
        for (family, list) in cases {
            let e = generate(family, &params(&list), &cfg).unwrap_err();
            assert_eq!(e.code, PARAM_INVALID, "{family} {list:?}: {}", e.detail);
        }
    }

    #[test]
    fn a_value_of_the_file_changes_the_footprint() {
        let tighter =
            FootprintConfig::parse(&DEFAULT_CONFIG.replace("chip.toe = 0.35", "chip.toe = 0.15"))
                .unwrap();
        let a = make("chip", &[("size", "0805")]);
        let b = generate("chip", &params(&[("size", "0805")]), &tighter).unwrap();
        let (_, _, adx, _) = pad_at(&a.package, "1");
        let (_, _, bdx, _) = pad_at(&b.package, "1");
        assert!(bdx < adx, "{bdx} {adx}");
        // a file without the common values cannot be used
        let empty = FootprintConfig::parse("chip.0603.length = 1.6\n").unwrap();
        let e = generate("chip", &params(&[("size", "0603")]), &empty).unwrap_err();
        assert_eq!(e.code, SETTINGS_INVALID);
    }

    #[test]
    fn the_silkscreen_stays_away_from_the_pads_by_the_gap_of_the_file() {
        // the lines of a chip are beyond the width of the lands by the gap and half a line
        let g = make("chip", &[("size", "0805")]);
        let (_, _, _, width) = pad_at(&g.package, "1");
        let silk: Vec<&Wire> = g
            .package
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Wire(w) if w.layer == LAYER_T_PLACE => Some(w),
                _ => None,
            })
            .collect();
        assert_eq!(silk.len(), 2);
        assert_eq!(silk[0].y1, width / 2 + 200_000 + 75_000);
        assert_eq!(silk[1].y1, -silk[0].y1);
    }
}
