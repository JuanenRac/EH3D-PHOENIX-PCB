// SPDX-License-Identifier: GPL-3.0-or-later
//! The generator of footprints. A footprint is made from a family, a variant, a density level and
//! a few parameters (the number of pins, the prefix of the name), with the dimensions of the
//! family file.
//!
//! The land of a lead follows the method of IPC-7351B: the dimensions of the lead plus a fillet
//! goal at the toe, the heel and the side, with the tolerances of the component, of the
//! fabrication and of the placement added in quadrature. The goals depend on the density level.
//! The silkscreen is always drawn outside the box of the pads and at the distance of the family,
//! the courtyard is the box of the pads and the body grown by the excess of the density level.

use std::collections::BTreeMap;

use crate::codes::{IpcError, LAND_IMPOSSIBLE, PARAM_INVALID};
use crate::family::{Density, Family, FamilySet, Goals, Variant};
use crate::fp::{grow, pads_bounds, union, IpcDot, IpcFootprint, IpcLine, IpcPad, PadKind, Rect};

/// The parameters of a generator, by name.
pub type Params = BTreeMap<String, String>;

/// The letters of the rows of a ball grid: the alphabet without I, O, Q, S, X and Z.
const GRID_ROWS: &str = "ABCDEFGHJKLMNPRTUVWY";

fn bad(what: impl Into<String>) -> IpcError {
    IpcError::new(PARAM_INVALID, what)
}

fn impossible(what: impl Into<String>) -> IpcError {
    IpcError::new(LAND_IMPOSSIBLE, what)
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

/// Millimetres in hundredths, as the names of IPC-7351B write them.
fn h100(mm: f64) -> i64 {
    (mm * 100.0).round() as i64
}

/// A land: its length, its width and the distance of its centre from the axis, in nanometres.
struct Land {
    len: i64,
    width: i64,
    centre: i64,
}

/// What a layout makes before the courtyard is added.
struct Built {
    name: String,
    pads: Vec<IpcPad>,
    silk: Vec<IpcLine>,
    dots: Vec<IpcDot>,
    body: Rect,
    pitch: i64,
    pitch_exact: bool,
    notes: Vec<String>,
}

/// Everything a layout needs to know.
struct Ctx<'a> {
    family: &'a Family,
    variant: &'a Variant,
    goals: Goals,
    level: Density,
    prefix: String,
    grid: i64,
    silk_w: i64,
    gap: i64,
    bracket: i64,
    dot_r: i64,
    ep_paste: u32,
    drill_step: i64,
    pad_gap: i64,
    min_ring: i64,
}

fn smd(name: &str, x: i64, y: i64, w: i64, h: i64) -> IpcPad {
    IpcPad {
        name: name.to_owned(),
        x,
        y,
        w,
        h,
        shape: PadKind::Rect,
        drill: 0,
        mask: true,
        paste: 100,
    }
}

fn line(x1: i64, y1: i64, x2: i64, y2: i64, width: i64) -> IpcLine {
    IpcLine {
        x1,
        y1,
        x2,
        y2,
        width,
    }
}

/// The four sides of a box as lines.
fn box_lines(r: Rect, width: i64) -> Vec<IpcLine> {
    vec![
        line(r.0, r.1, r.2, r.1, width),
        line(r.2, r.1, r.2, r.3, width),
        line(r.2, r.3, r.0, r.3, width),
        line(r.0, r.3, r.0, r.1, width),
    ]
}

fn allowed(params: &Params, names: &[&str]) -> Result<(), IpcError> {
    for key in params.keys() {
        if !names.contains(&key.as_str()) {
            let list = if names.is_empty() {
                "none".to_owned()
            } else {
                names.join(", ")
            };
            return Err(bad(format!(
                "{key} is not a parameter of the family (it has {list})"
            )));
        }
    }
    Ok(())
}

fn count_param(params: &Params, key: &str) -> Result<Option<u32>, IpcError> {
    match params.get(key) {
        None => Ok(None),
        Some(text) => text
            .trim()
            .parse::<u32>()
            .map(Some)
            .map_err(|_| bad(format!("{key} is not a whole number"))),
    }
}

fn pins_param(params: &Params, min: u32, max: u32) -> Result<u32, IpcError> {
    let pins = match count_param(params, "pins")? {
        Some(n) => n,
        None if min == max => min,
        None => return Err(bad("the parameter pins is needed")),
    };
    if !(min..=max).contains(&pins) {
        return Err(bad(format!("pins must be from {min} to {max}")));
    }
    Ok(pins)
}

impl Ctx<'_> {
    fn need(&self, key: &str) -> Result<f64, IpcError> {
        self.variant.need(key)
    }

    /// The land of a lead by the method of IPC-7351B, from the overall length across the
    /// leads, the length of a lead and its width, each with its tolerance.
    fn land(&self, l: f64, lt: f64, t: f64, tt: f64, w: f64, wt: f64) -> Result<Land, IpcError> {
        let tol = self.family.tolerances;
        let g = self.goals;
        let lmin = l - lt;
        let lmax = l + lt;
        let cl = lmax - lmin;
        let tmin = t - tt;
        let ct = 2.0 * tt;
        let smax = lmax - 2.0 * tmin;
        let cs = (cl * cl + 2.0 * ct * ct).sqrt();
        let wmin = w - wt;
        let cw = 2.0 * wt;
        let fp = tol.fab * tol.fab + tol.place * tol.place;
        let zmax = lmin + 2.0 * g.toe + (cl * cl + fp).sqrt();
        let gmin = smax - 2.0 * g.heel - (cs * cs + fp).sqrt();
        let xmax = wmin + 2.0 * g.side + (cw * cw + fp).sqrt();
        if gmin <= 0.0 || zmax <= gmin || xmax <= 0.0 {
            return Err(impossible(
                "the dimensions give lands that touch or have no size; check the values of the variant",
            ));
        }
        let len = snap(nm((zmax - gmin) / 2.0), self.grid).max(self.grid);
        let width = snap(nm(xmax), self.grid).max(self.grid);
        let centre = snap(nm((zmax + gmin) / 4.0), self.grid);
        if centre <= len / 2 {
            return Err(impossible(
                "the lands of the two sides would cross the axis",
            ));
        }
        Ok(Land { len, width, centre })
    }

    fn lead_land(&self) -> Result<Land, IpcError> {
        self.land(
            self.need("span")?,
            self.need("span_tol")?,
            self.need("lead")?,
            self.need("lead_tol")?,
            self.need("lead_w")?,
            self.need("lead_w_tol")?,
        )
    }

    fn two_terminal(&self, params: &Params) -> Result<Built, IpcError> {
        allowed(params, &["prefix"])?;
        let length = self.need("length")?;
        let width = self.need("width")?;
        let land = self.land(
            length,
            self.need("length_tol")?,
            self.need("term")?,
            self.need("term_tol")?,
            width,
            self.need("width_tol")?,
        )?;
        let c = land.centre;
        let mut silk = Vec::new();
        if self.variant.get_or("silk", 1.0) > 0.5 {
            let y = land.width / 2 + self.gap + self.silk_w / 2;
            let reach = c + land.len / 2;
            silk.push(line(-reach, y, reach, y, self.silk_w));
            silk.push(line(-reach, -y, reach, -y, self.silk_w));
        }
        let (bx, by) = (nm(length) / 2, nm(width) / 2);
        Ok(Built {
            name: format!(
                "{}{}X{}{}",
                self.prefix,
                self.variant.code,
                h100(self.need("height")?),
                self.level.letter()
            ),
            pads: vec![
                smd("1", -c, 0, land.len, land.width),
                smd("2", c, 0, land.len, land.width),
            ],
            silk,
            dots: Vec::new(),
            body: (-bx, -by, bx, by),
            pitch: 0,
            pitch_exact: false,
            notes: vec![format!(
                "pads {} x {} mm, centres {} mm apart",
                mm_text(land.len),
                mm_text(land.width),
                mm_text(2 * c)
            )],
        })
    }

    fn dual(&self, params: &Params) -> Result<Built, IpcError> {
        allowed(params, &["pins", "prefix"])?;
        let pins = pins_param(
            params,
            self.need("pins_min")? as u32,
            self.need("pins_max")? as u32,
        )?;
        if !pins.is_multiple_of(2) {
            return Err(bad("pins must be even"));
        }
        let land = self.lead_land()?;
        let pitch_mm = self.need("pitch")?;
        let pitch = nm(pitch_mm);
        if pitch <= land.width {
            return Err(impossible(
                "the pitch is not larger than the width of the lands",
            ));
        }
        let m = (pins / 2) as i64;
        let ytop = (m - 1) * pitch / 2;
        let c = land.centre;
        let mut pads = Vec::new();
        for i in 0..m {
            pads.push(smd(
                &(i + 1).to_string(),
                -c,
                ytop - i * pitch,
                land.len,
                land.width,
            ));
        }
        for i in 0..m {
            pads.push(smd(
                &(m + 1 + i).to_string(),
                c,
                -ytop + i * pitch,
                land.len,
                land.width,
            ));
        }
        let mut ep_text = String::new();
        let mut ep_pad = 0;
        if let Some(ep_w) = self.variant.get("ep_w") {
            let ep_h = self.need("ep_h")?;
            let (ew, eh) = (nm(ep_w), nm(ep_h));
            if ew / 2 >= c - land.len / 2 {
                return Err(impossible("the exposed pad touches the lands"));
            }
            let mut pad = smd(&(pins + 1).to_string(), 0, 0, ew, eh);
            pad.paste = self.ep_paste;
            pads.push(pad);
            ep_pad = 1;
            ep_text = format!("T{}X{}", h100(ep_w), h100(ep_h));
        }
        let body_w = nm(self.need("body_w")?);
        let body_len = match self.variant.get("body_l") {
            Some(l) => nm(l),
            None => m * pitch - nm(self.variant.get_or("body_trim", 0.0)),
        };
        let y = ytop + land.width / 2 + self.gap + self.silk_w / 2;
        let silk = vec![
            line(-body_w / 2, y, body_w / 2, y, self.silk_w),
            line(-body_w / 2, -y, body_w / 2, -y, self.silk_w),
        ];
        let dots = vec![IpcDot {
            x: -(c + land.len / 2 + self.gap + self.dot_r),
            y: ytop,
            r: self.dot_r,
        }];
        let height = h100(self.need("height")?);
        let name = if self.family.naming == "dual_body" {
            format!(
                "{}{}P{}X{}X{}-{}{}{}",
                self.prefix,
                h100(pitch_mm),
                h100(self.need("body_w")?),
                body_len / 10_000,
                height,
                pins + ep_pad,
                ep_text,
                self.level.letter()
            )
        } else {
            format!(
                "{}{}P{}X{}-{}{}",
                self.prefix,
                h100(pitch_mm),
                h100(self.need("span")?),
                height,
                pins,
                self.level.letter()
            )
        };
        Ok(Built {
            name,
            pads,
            silk,
            dots,
            body: (-body_w / 2, -body_len / 2, body_w / 2, body_len / 2),
            pitch,
            pitch_exact: true,
            notes: vec![format!(
                "pads {} x {} mm, columns {} mm apart, pitch {} mm",
                mm_text(land.len),
                mm_text(land.width),
                mm_text(2 * c),
                mm_text(pitch)
            )],
        })
    }

    fn sot(&self, params: &Params) -> Result<Built, IpcError> {
        allowed(params, &["prefix"])?;
        let land = self.lead_land()?;
        let pitch_mm = self.need("pitch")?;
        let p = nm(pitch_mm);
        let c = land.centre;
        let mut pads = Vec::new();
        let first = self.variant.slots_a.len();
        for (i, h) in self.variant.slots_a.iter().enumerate() {
            let x = i64::from(*h) * p / 2;
            pads.push(smd(&(i + 1).to_string(), x, -c, land.width, land.len));
        }
        let tab = match (self.variant.get("tab_w"), self.variant.get("tab_h")) {
            (Some(w), Some(h)) => Some((nm(w), nm(h))),
            _ => None,
        };
        for (i, h) in self.variant.slots_b.iter().enumerate() {
            let x = i64::from(*h) * p / 2;
            let (w, hh) = tab.unwrap_or((land.width, land.len));
            // a tab is as long as it is and ends where the other lands end, so that the box of
            // the pads stays centred on the origin
            let y = if tab.is_some() {
                c + land.len / 2 - hh / 2
            } else {
                c
            };
            pads.push(smd(&(first + i + 1).to_string(), x, y, w, hh));
        }
        let pins = pads.len();
        let reach = pads.iter().map(|q| q.x.abs() + q.w / 2).max().unwrap_or(0);
        let xs = reach + self.gap + self.silk_w / 2;
        let half = c - land.len / 2 - self.gap - self.silk_w / 2;
        let mut silk = Vec::new();
        if half > 0 {
            silk.push(line(xs, -half, xs, half, self.silk_w));
            silk.push(line(-xs, -half, -xs, half, self.silk_w));
        }
        let (bl, bw) = (nm(self.need("body_l")?), nm(self.need("body_w")?));
        Ok(Built {
            name: format!(
                "{}{}P{}X{}-{}{}",
                self.prefix,
                h100(pitch_mm),
                h100(self.need("span")?),
                h100(self.need("height")?),
                pins,
                self.level.letter()
            ),
            pads,
            silk,
            dots: Vec::new(),
            body: (-bl / 2, -bw / 2, bl / 2, bw / 2),
            pitch: p,
            pitch_exact: false,
            notes: vec![format!(
                "pads {} x {} mm, rows {} mm apart, pitch {} mm",
                mm_text(land.width),
                mm_text(land.len),
                mm_text(2 * c),
                mm_text(p)
            )],
        })
    }

    fn quad(&self, params: &Params) -> Result<Built, IpcError> {
        allowed(params, &["prefix"])?;
        let pins = self.need("pins")? as i64;
        if pins < 4 || pins % 4 != 0 {
            return Err(bad("pins must be a multiple of four"));
        }
        let land = self.lead_land()?;
        let pitch_mm = self.need("pitch")?;
        let p = nm(pitch_mm);
        if p <= land.width {
            return Err(impossible(
                "the pitch is not larger than the width of the lands",
            ));
        }
        let m = pins / 4;
        let ytop = (m - 1) * p / 2;
        let c = land.centre;
        let inner = c - land.len / 2;
        if ytop + land.width / 2 >= inner {
            return Err(impossible(
                "the lands of two sides would touch in the corner",
            ));
        }
        let mut pads = Vec::new();
        for i in 0..m {
            pads.push(smd(
                &(i + 1).to_string(),
                -c,
                ytop - i * p,
                land.len,
                land.width,
            ));
        }
        for i in 0..m {
            pads.push(smd(
                &(m + i + 1).to_string(),
                -ytop + i * p,
                -c,
                land.width,
                land.len,
            ));
        }
        for i in 0..m {
            pads.push(smd(
                &(2 * m + i + 1).to_string(),
                c,
                -ytop + i * p,
                land.len,
                land.width,
            ));
        }
        for i in 0..m {
            pads.push(smd(
                &(3 * m + i + 1).to_string(),
                ytop - i * p,
                c,
                land.width,
                land.len,
            ));
        }
        let mut ep_text = String::new();
        let mut ep_pad = 0;
        if let Some(ep) = self.variant.get("ep") {
            let e = nm(ep);
            if e / 2 >= inner {
                return Err(impossible("the exposed pad touches the lands"));
            }
            let mut pad = smd(&(pins + 1).to_string(), 0, 0, e, e);
            pad.paste = self.ep_paste;
            pads.push(pad);
            ep_pad = 1;
            ep_text = format!("T{}X{}", h100(ep), h100(ep));
        }
        let outer = c + land.len / 2;
        let clear = ytop + land.width / 2 + self.gap + self.silk_w / 2;
        let length = self.bracket.min(outer - clear);
        let mut silk = Vec::new();
        if length > 0 {
            for sx in [-1_i64, 1] {
                for sy in [-1_i64, 1] {
                    silk.push(line(
                        sx * outer,
                        sy * outer,
                        sx * (outer - length),
                        sy * outer,
                        self.silk_w,
                    ));
                    silk.push(line(
                        sx * outer,
                        sy * outer,
                        sx * outer,
                        sy * (outer - length),
                        self.silk_w,
                    ));
                }
            }
        }
        let dots = vec![IpcDot {
            x: -(c + land.len / 2 + self.gap + self.dot_r),
            y: ytop,
            r: self.dot_r,
        }];
        let bs = nm(self.need("body")?);
        let height = h100(self.need("height")?);
        let name = if self.family.naming == "quad_body" {
            let side = h100(self.need("body")?);
            format!(
                "{}{}P{}X{}X{}-{}{}{}",
                self.prefix,
                h100(pitch_mm),
                side,
                side,
                height,
                pins + ep_pad,
                ep_text,
                self.level.letter()
            )
        } else {
            let side = h100(self.need("span")?);
            format!(
                "{}{}P{}X{}X{}-{}{}",
                self.prefix,
                h100(pitch_mm),
                side,
                side,
                height,
                pins,
                self.level.letter()
            )
        };
        Ok(Built {
            name,
            pads,
            silk,
            dots,
            body: (-bs / 2, -bs / 2, bs / 2, bs / 2),
            pitch: p,
            pitch_exact: true,
            notes: vec![format!(
                "pads {} x {} mm, sides {} mm apart, pitch {} mm",
                mm_text(land.len),
                mm_text(land.width),
                mm_text(2 * c),
                mm_text(p)
            )],
        })
    }

    fn grid(&self, params: &Params) -> Result<Built, IpcError> {
        allowed(params, &["prefix"])?;
        let cols = self.need("cols")? as i64;
        let rows = self.need("rows")? as i64;
        if cols < 1 || rows < 1 || rows as usize > GRID_ROWS.len() {
            return Err(bad(format!(
                "the grid needs from 1 to {} rows",
                GRID_ROWS.len()
            )));
        }
        let pitch_mm = self.need("pitch")?;
        let p = nm(pitch_mm);
        let ball = self.need("ball")?;
        if self.goals.pad_ratio <= 0.0 {
            return Err(impossible("the density level has no ratio for the pads"));
        }
        let dia = snap(nm(ball * self.goals.pad_ratio), self.grid);
        if dia <= 0 || dia + self.pad_gap / 2 > p {
            return Err(impossible("the pads of the balls would touch"));
        }
        let letters: Vec<char> = GRID_ROWS.chars().collect();
        let mut pads = Vec::new();
        for r in 0..rows {
            for col in 0..cols {
                let name = format!("{}{}", letters[r as usize], col + 1);
                let x = (2 * col - (cols - 1)) * p / 2;
                let y = ((rows - 1) - 2 * r) * p / 2;
                pads.push(IpcPad {
                    name,
                    x,
                    y,
                    w: dia,
                    h: dia,
                    shape: PadKind::Round,
                    drill: 0,
                    mask: true,
                    paste: 100,
                });
            }
        }
        let (bl, bw) = (nm(self.need("body_l")?), nm(self.need("body_w")?));
        let reach_x = (cols - 1) * p / 2 + dia / 2;
        let reach_y = (rows - 1) * p / 2 + dia / 2;
        let bx = (bl / 2 + self.silk_w / 2).max(reach_x + self.gap + self.silk_w / 2);
        let by = (bw / 2 + self.silk_w / 2).max(reach_y + self.gap + self.silk_w / 2);
        let length = self.bracket.min(bx).min(by);
        let mut silk = Vec::new();
        for sx in [-1_i64, 1] {
            for sy in [-1_i64, 1] {
                silk.push(line(
                    sx * bx,
                    sy * by,
                    sx * (bx - length),
                    sy * by,
                    self.silk_w,
                ));
                silk.push(line(
                    sx * bx,
                    sy * by,
                    sx * bx,
                    sy * (by - length),
                    self.silk_w,
                ));
            }
        }
        Ok(Built {
            name: format!(
                "{}{}C{}P{}X{}_{}X{}X{}{}",
                self.prefix,
                cols * rows,
                h100(pitch_mm),
                cols,
                rows,
                h100(self.need("body_l")?),
                h100(self.need("body_w")?),
                h100(self.need("height")?),
                self.level.letter()
            ),
            pads,
            silk,
            dots: Vec::new(),
            body: (-bl / 2, -bw / 2, bl / 2, bw / 2),
            pitch: p,
            pitch_exact: true,
            notes: vec![format!(
                "{cols} x {rows} balls, pads {} mm for balls of {ball} mm, pitch {} mm",
                mm_text(dia),
                mm_text(p)
            )],
        })
    }

    /// The hole and the pad of a through-hole lead, in nanometres.
    fn hole_and_pad(&self, drill: i64, wanted: i64, limit: i64) -> Result<i64, IpcError> {
        let dia = wanted.min(limit);
        if dia - drill < 2 * self.min_ring {
            return Err(impossible(
                "the ring of copper around the hole would be too thin",
            ));
        }
        Ok(dia)
    }

    fn tht_dual(&self, params: &Params) -> Result<Built, IpcError> {
        allowed(params, &["pins", "prefix"])?;
        let pins = pins_param(
            params,
            self.need("pins_min")? as u32,
            self.need("pins_max")? as u32,
        )?;
        if !pins.is_multiple_of(2) {
            return Err(bad("pins must be even"));
        }
        let pitch_mm = self.need("pitch")?;
        let p = nm(pitch_mm);
        let row = nm(self.need("row")?);
        let (lead_w, lead_t) = (self.need("lead_w")?, self.need("lead_t")?);
        let diagonal = (lead_w * lead_w + lead_t * lead_t).sqrt() + self.goals.drill_extra;
        let step = self.drill_step.max(1);
        let drill = ((nm(diagonal) as f64 / step as f64).ceil() as i64) * step;
        let dia = self.hole_and_pad(drill, drill + 2 * nm(self.goals.annular), p - self.pad_gap)?;
        let m = (pins / 2) as i64;
        let ytop = (m - 1) * p / 2;
        let mut pads = Vec::new();
        for i in 0..m {
            pads.push(tht_pad(
                &(i + 1).to_string(),
                -row / 2,
                ytop - i * p,
                dia,
                dia,
                drill,
                i == 0,
            ));
        }
        for i in 0..m {
            pads.push(tht_pad(
                &(m + 1 + i).to_string(),
                row / 2,
                -ytop + i * p,
                dia,
                dia,
                drill,
                false,
            ));
        }
        let xs = row / 2 + dia / 2 + self.gap + self.silk_w / 2;
        let ys = ytop + dia / 2 + self.gap + self.silk_w / 2;
        let silk = box_lines((-xs, -ys, xs, ys), self.silk_w);
        let dots = vec![IpcDot {
            x: -(xs + self.silk_w / 2 + self.gap + self.dot_r),
            y: ytop,
            r: self.dot_r,
        }];
        let body_w = nm(self.need("body_w")?);
        let body_len = m * p - nm(self.need("body_trim")?);
        Ok(Built {
            name: format!(
                "{}{}W{}P{}L{}H{}Q{}{}",
                self.prefix,
                h100(self.need("row")?),
                h100(lead_w),
                h100(pitch_mm),
                body_len / 10_000,
                h100(self.need("height")?),
                pins,
                self.level.letter()
            ),
            pads,
            silk,
            dots,
            body: (-body_w / 2, -body_len / 2, body_w / 2, body_len / 2),
            pitch: p,
            pitch_exact: true,
            notes: vec![format!(
                "holes {} mm, pads {} mm, rows {} mm apart, pitch {} mm",
                mm_text(drill),
                mm_text(dia),
                mm_text(row),
                mm_text(p)
            )],
        })
    }

    fn tht_row(&self, params: &Params) -> Result<Built, IpcError> {
        allowed(params, &["pins", "rows", "prefix"])?;
        let pins = pins_count(
            params,
            self.need("pins_min")? as u32,
            self.need("pins_max")? as u32,
        )?;
        let rows = count_param(params, "rows")?.unwrap_or(1);
        if rows < 1 || f64::from(rows) > self.need("rows_max")? {
            return Err(bad(format!(
                "rows must be from 1 to {}",
                self.need("rows_max")?
            )));
        }
        if !pins.is_multiple_of(rows) {
            return Err(bad("pins must be a multiple of rows"));
        }
        let (pins, rows) = (i64::from(pins), i64::from(rows));
        let cols = pins / rows;
        let pitch_mm = self.need("pitch")?;
        let p = nm(pitch_mm);
        let gap_rows = nm(self.variant.get_or("row_gap", pitch_mm));
        let drill = nm(self.need("drill")?);
        let (pw, ph) = (nm(self.need("pad_w")?), nm(self.need("pad_h")?));
        self.hole_and_pad(drill, pw.min(ph), pw.min(ph))?;
        let kind = if pw == ph {
            PadKind::Round
        } else {
            PadKind::Oval
        };
        let mut pads = Vec::new();
        for k in 0..pins {
            let (col, r) = if rows == 1 {
                (k, 0)
            } else {
                (k / rows, k % rows)
            };
            let x = (2 * col - (cols - 1)) * p / 2;
            let y = if rows == 1 {
                0
            } else {
                ((rows - 1) - 2 * r) * gap_rows / 2
            };
            let mut pad = tht_pad(&(k + 1).to_string(), x, y, pw, ph, drill, k == 0);
            if k != 0 {
                pad.shape = kind;
            }
            pads.push(pad);
        }
        let reach_x = (cols - 1) * p / 2 + pw / 2;
        let reach_y = (rows - 1) * gap_rows / 2 + ph / 2;
        let xs = reach_x + self.gap + self.silk_w / 2;
        let ys = reach_y + self.gap + self.silk_w / 2;
        let silk = box_lines((-xs, -ys, xs, ys), self.silk_w);
        let top = (rows - 1) * gap_rows / 2;
        let dots = vec![IpcDot {
            x: -(xs + self.silk_w / 2 + self.gap + self.dot_r),
            y: top,
            r: self.dot_r,
        }];
        let (bx, by) = (
            cols * p / 2,
            (nm(self.need("body_w")?) + (rows - 1) * gap_rows) / 2,
        );
        let base = if self.variant.code.is_empty() {
            self.prefix.clone()
        } else {
            format!("{}_{}", self.prefix, self.variant.code)
        };
        Ok(Built {
            name: format!(
                "{base}{rows}X{cols:02}P{}{}",
                h100(pitch_mm),
                self.level.letter()
            ),
            pads,
            silk,
            dots,
            body: (-bx, -by, bx, by),
            pitch: p,
            pitch_exact: true,
            notes: vec![format!(
                "{pins} pins in {rows} row(s), holes {} mm, pads {} x {} mm, pitch {} mm",
                mm_text(drill),
                mm_text(pw),
                mm_text(ph),
                mm_text(p)
            )],
        })
    }

    fn explicit(&self, params: &Params) -> Result<Built, IpcError> {
        allowed(params, &[])?;
        let pads: Vec<IpcPad> = self
            .variant
            .pads
            .iter()
            .map(|q| smd(&q.name, nm(q.x), nm(q.y), nm(q.w), nm(q.h)))
            .collect();
        let (bl, bw) = (nm(self.need("body_l")?), nm(self.need("body_w")?));
        Ok(Built {
            name: format!("{}{}", self.variant.code, self.level.letter()),
            pads,
            silk: Vec::new(),
            dots: Vec::new(),
            body: (-bl / 2, -bw / 2, bl / 2, bw / 2),
            pitch: nm(self.need("pitch")?),
            pitch_exact: false,
            notes: vec!["pads written out in the family file".to_owned()],
        })
    }
}

/// The number of pins of a row family: the parameter, which is needed.
fn pins_count(params: &Params, min: u32, max: u32) -> Result<u32, IpcError> {
    let Some(pins) = count_param(params, "pins")? else {
        return Err(bad("the parameter pins is needed"));
    };
    if !(min..=max).contains(&pins) {
        return Err(bad(format!("pins must be from {min} to {max}")));
    }
    Ok(pins)
}

fn tht_pad(name: &str, x: i64, y: i64, w: i64, h: i64, drill: i64, first: bool) -> IpcPad {
    IpcPad {
        name: name.to_owned(),
        x,
        y,
        w,
        h,
        shape: if first { PadKind::Rect } else { PadKind::Round },
        drill,
        mask: true,
        paste: 0,
    }
}

/// A length in nanometres as millimetres, for a note.
fn mm_text(value: i64) -> String {
    eh3d_phoenix_libedit::write::fmt_mm(value)
}

/// Makes a footprint from a family, a variant, a density level and parameters.
pub fn generate(
    set: &FamilySet,
    family: &str,
    variant: &str,
    level: Density,
    params: &Params,
) -> Result<IpcFootprint, IpcError> {
    let fam = set.get(family)?;
    let var = fam.variant(variant)?;
    let prefix = match params.get("prefix") {
        None => fam.prefixes[0].clone(),
        Some(p) => {
            let upper = p.trim().to_ascii_uppercase();
            if !fam.prefixes.contains(&upper) {
                return Err(bad(format!(
                    "the prefix {p} is not one of {}",
                    fam.prefixes.join(", ")
                )));
            }
            upper
        }
    };
    let d = fam.drawing;
    let ctx = Ctx {
        family: fam,
        variant: var,
        goals: fam.goals(var, level),
        level,
        prefix: prefix.clone(),
        grid: nm(d.grid),
        silk_w: nm(d.silk_width),
        gap: nm(d.silk_gap),
        bracket: nm(d.bracket),
        dot_r: nm(d.dot_r),
        ep_paste: d.ep_paste,
        drill_step: nm(d.drill_step),
        pad_gap: nm(d.pad_gap),
        min_ring: nm(d.min_ring),
    };
    let built = match fam.layout.as_str() {
        "two_terminal" => ctx.two_terminal(params)?,
        "dual" => ctx.dual(params)?,
        "sot" => ctx.sot(params)?,
        "quad" => ctx.quad(params)?,
        "grid" => ctx.grid(params)?,
        "tht_dual" => ctx.tht_dual(params)?,
        "tht_row" => ctx.tht_row(params)?,
        _ => ctx.explicit(params)?,
    };
    let excess = nm(ctx.goals.courtyard);
    let Some(pad_box) = pads_bounds(&built.pads) else {
        return Err(impossible("the footprint has no pads"));
    };
    let court = grow(union(pad_box, built.body), excess);
    let mut notes = vec![format!(
        "{}: dimensions of the family {}",
        fam.status, fam.family
    )];
    notes.extend(built.notes);
    Ok(IpcFootprint {
        name: built.name,
        family: fam.family.clone(),
        variant: variant.to_owned(),
        prefix,
        density: level,
        pitch: built.pitch,
        pitch_exact: built.pitch_exact,
        pads: built.pads,
        silk: built.silk,
        dots: built.dots,
        body: built.body,
        court,
        court_excess: excess,
        silk_gap: ctx.gap,
        notes,
        verified: fam.verified,
    })
}

/// The parameter sets with which every variant of a family is generated to check the library:
/// the least and the most pins, and one row and two where the family has both.
pub fn sample_params(family: &Family, variant: &Variant) -> Vec<Params> {
    let pins = |n: f64| -> Params {
        let mut p = Params::new();
        p.insert("pins".to_owned(), (n as u32).to_string());
        p
    };
    match family.layout.as_str() {
        "dual" | "tht_dual" => {
            let (lo, hi) = (
                variant.get_or("pins_min", 0.0),
                variant.get_or("pins_max", 0.0),
            );
            let mut out = vec![pins(lo)];
            if hi > lo {
                out.push(pins(hi));
            }
            out
        }
        "tht_row" => {
            let (lo, hi) = (
                variant.get_or("pins_min", 1.0),
                variant.get_or("pins_max", 1.0),
            );
            let mut out = vec![pins(lo), pins(hi)];
            if variant.get_or("rows_max", 1.0) >= 2.0 {
                for n in [lo * 2.0, hi] {
                    let mut p = pins(n);
                    p.insert("rows".to_owned(), "2".to_owned());
                    out.push(p);
                }
            }
            out
        }
        _ => vec![Params::new()],
    }
}

/// Every footprint of the library: each variant of each family at each density level, with the
/// parameter sets of [`sample_params`].
pub fn library(set: &FamilySet) -> Result<Vec<IpcFootprint>, IpcError> {
    let mut out = Vec::new();
    for family in &set.families {
        for (name, variant) in &family.variants {
            for params in sample_params(family, variant) {
                for level in Density::ALL {
                    out.push(generate(set, &family.family, name, level, &params)?);
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{FAMILY_UNKNOWN, VARIANT_UNKNOWN};

    fn set() -> FamilySet {
        FamilySet::builtin().unwrap()
    }

    fn params(extra: &[(&str, &str)]) -> Params {
        extra
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn make(family: &str, variant: &str, level: Density, extra: &[(&str, &str)]) -> IpcFootprint {
        generate(&set(), family, variant, level, &params(extra)).unwrap()
    }

    fn pad<'a>(fp: &'a IpcFootprint, name: &str) -> &'a IpcPad {
        fp.pads.iter().find(|p| p.name == name).unwrap()
    }

    #[test]
    fn the_names_follow_the_naming_of_ipc_7351b() {
        let b = Density::B;
        assert_eq!(make("chip", "0603", b, &[]).name, "RESC1608X55N");
        assert_eq!(
            make("chip", "0805", Density::A, &[("prefix", "capc")]).name,
            "CAPC2012X60M"
        );
        assert_eq!(
            make("soic", "narrow", b, &[("pins", "8")]).name,
            "SOIC127P600X175-8N"
        );
        assert_eq!(make("sot", "sot23_3", b, &[]).name, "SOT95P240X110-3N");
        assert_eq!(make("sot", "sot23_5", b, &[]).name, "SOT95P280X145-5N");
        assert_eq!(make("sot89", "sot89_3", b, &[]).name, "SOT150P450X150-4N");
        assert_eq!(
            make("qfp", "lqfp48_7x7", b, &[]).name,
            "QFP50P900X900X160-48N"
        );
        assert_eq!(
            make("qfn", "qfn16_3x3_p0_5", Density::C, &[]).name,
            "QFN50P300X300X80-17T170X170L"
        );
        assert_eq!(
            make("bga", "p0_8_10x10", b, &[]).name,
            "BGA100C80P10X10_900X900X120N"
        );
        assert_eq!(
            make("dip", "narrow", b, &[("pins", "8")]).name,
            "DIP762W50P254L927H508Q8N"
        );
        assert_eq!(
            make("header", "p2_54", b, &[("pins", "5")]).name,
            "HDR1X05P254N"
        );
        assert_eq!(
            make("connector", "screw_5p08", b, &[("pins", "3")]).name,
            "CONN_SCREW1X03P508N"
        );
    }

    #[test]
    fn a_denser_level_gives_less_copper_and_a_smaller_courtyard() {
        let a = make("chip", "0603", Density::A, &[]);
        let b = make("chip", "0603", Density::B, &[]);
        let c = make("chip", "0603", Density::C, &[]);
        assert_eq!(
            (
                a.name.ends_with('M'),
                b.name.ends_with('N'),
                c.name.ends_with('L')
            ),
            (true, true, true)
        );
        assert!(a.pads[0].w > b.pads[0].w && b.pads[0].w > c.pads[0].w);
        assert!(a.court_excess > b.court_excess && b.court_excess > c.court_excess);
        assert_eq!(b.court_excess, 250_000);
    }

    #[test]
    fn the_soic_has_two_columns_numbered_from_the_top_left() {
        let fp = make("soic", "narrow", Density::B, &[("pins", "8")]);
        assert_eq!(fp.pads.len(), 8);
        let (p1, p4, p5, p8) = (pad(&fp, "1"), pad(&fp, "4"), pad(&fp, "5"), pad(&fp, "8"));
        assert_eq!((p1.x < 0, p1.y), (true, 1_905_000));
        assert_eq!((p4.x < 0, p4.y), (true, -1_905_000));
        assert_eq!((p5.x > 0, p5.y), (true, -1_905_000));
        assert_eq!((p8.x > 0, p8.y), (true, 1_905_000));
        assert_eq!(p1.y - pad(&fp, "2").y, 1_270_000);
        assert_eq!(fp.pitch, 1_270_000);
        assert_eq!(fp.dots.len(), 1);
    }

    #[test]
    fn the_sot_23_has_two_pads_on_one_side_and_one_on_the_other() {
        let fp = make("sot", "sot23_3", Density::B, &[]);
        assert_eq!(fp.pads.len(), 3);
        let (p1, p2, p3) = (pad(&fp, "1"), pad(&fp, "2"), pad(&fp, "3"));
        assert_eq!((p1.x, p2.x, p3.x), (-950_000, 950_000, 0));
        assert!(p1.y < 0 && p2.y < 0 && p3.y > 0);
        assert_eq!(p1.y, -p3.y);
        let sot223 = make("sot", "sot223_4", Density::B, &[]);
        let tab = pad(&sot223, "4");
        assert_eq!((tab.w, tab.h), (3_400_000, 2_100_000));
    }

    #[test]
    fn the_ball_grid_names_its_balls_by_row_letter_and_column() {
        let fp = make("bga", "p0_8_10x10", Density::B, &[]);
        assert_eq!(fp.pads.len(), 100);
        let (first, last) = (pad(&fp, "A1"), pad(&fp, "K10"));
        assert_eq!((first.x, first.y), (-3_600_000, 3_600_000));
        assert_eq!((last.x, last.y), (3_600_000, -3_600_000));
        assert!(fp
            .pads
            .iter()
            .all(|p| p.shape == PadKind::Round && p.w == p.h));
        assert!(fp.pads.iter().all(|p| !p.name.starts_with('I')));
    }

    #[test]
    fn the_exposed_pad_is_numbered_after_the_pins_and_has_less_paste() {
        let fp = make("qfn", "qfn16_3x3_p0_5", Density::B, &[]);
        assert_eq!(fp.pads.len(), 17);
        let ep = pad(&fp, "17");
        assert_eq!(
            (ep.x, ep.y, ep.w, ep.h, ep.paste),
            (0, 0, 1_700_000, 1_700_000, 60)
        );
        assert_eq!(pad(&fp, "1").paste, 100);
        let dfn = make("dfn", "dfn8_3x3_p0_65", Density::B, &[]);
        assert_eq!(dfn.pads.len(), 9);
        assert!(dfn.name.contains("-9T190X150"), "{}", dfn.name);
    }

    #[test]
    fn a_header_is_centred_and_its_first_pad_is_square() {
        let fp = make("header", "p2_54", Density::B, &[("pins", "5")]);
        assert_eq!(fp.pads.len(), 5);
        assert_eq!(pad(&fp, "1").shape, PadKind::Rect);
        assert_eq!(pad(&fp, "2").shape, PadKind::Round);
        assert_eq!((pad(&fp, "1").x, pad(&fp, "5").x), (-5_080_000, 5_080_000));
        assert!(fp.pads.iter().all(|p| p.drill == 1_000_000 && p.paste == 0));
        let two = make(
            "header",
            "p2_54",
            Density::B,
            &[("pins", "6"), ("rows", "2")],
        );
        assert_eq!(two.name, "HDR2X03P254N");
        assert!(pad(&two, "1").y > 0 && pad(&two, "2").y < 0);
        assert_eq!(pad(&two, "1").x, pad(&two, "2").x);
    }

    #[test]
    fn bad_requests_are_refused_with_their_codes() {
        let s = set();
        let b = Density::B;
        let none = Params::new();
        let e = generate(&s, "chop", "x", b, &none).unwrap_err();
        assert_eq!(e.code, FAMILY_UNKNOWN);
        let e = generate(&s, "chip", "9999", b, &none).unwrap_err();
        assert_eq!(e.code, VARIANT_UNKNOWN);
        let wrong: [&[(&str, &str)]; 5] = [
            &[("pins", "2")],
            &[("pins", "9")],
            &[("pins", "many")],
            &[("pins", "8"), ("colour", "red")],
            &[],
        ];
        for extra in wrong {
            let e = generate(&s, "soic", "narrow", b, &params(extra)).unwrap_err();
            assert_eq!(e.code, PARAM_INVALID, "{extra:?}");
        }
        let e = generate(&s, "chip", "0603", b, &params(&[("prefix", "BEAD")])).unwrap_err();
        assert_eq!(e.code, PARAM_INVALID);
        let two_rows = params(&[("pins", "5"), ("rows", "2")]);
        let e = generate(&s, "header", "p2_54", b, &two_rows).unwrap_err();
        assert_eq!(e.code, PARAM_INVALID);
        let four_rows = params(&[("pins", "4"), ("rows", "2")]);
        let e = generate(&s, "connector", "xh_2p5", b, &four_rows).unwrap_err();
        assert_eq!(e.code, PARAM_INVALID);
    }

    #[test]
    fn the_whole_library_is_made_with_unique_names() {
        let fps = library(&set()).unwrap();
        assert_eq!(fps.len(), 74 * 3);
        let mut names: Vec<&str> = fps.iter().map(|f| f.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), fps.len());
        assert!(fps.iter().all(|f| !f.verified));
        assert!(fps.iter().all(|f| f.notes[0].contains("not verified")));
    }

    #[test]
    fn a_family_with_impossible_dimensions_is_refused() {
        let mut s = set();
        let chip = s.families.iter_mut().find(|f| f.family == "chip").unwrap();
        let v = chip.variants.get_mut("0603").unwrap();
        v.values.insert("term".to_owned(), 5.0);
        let e = generate(&s, "chip", "0603", Density::B, &Params::new()).unwrap_err();
        assert_eq!(e.code, LAND_IMPOSSIBLE);
    }
}
