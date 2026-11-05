// SPDX-License-Identifier: GPL-3.0-or-later
//! The reader of Gerber RS-274X files.
//!
//! Read: the format (`FS`) and the units (`MO`), standard apertures (`C`, `R`, `O`, `P`) and
//! macro apertures (`AM`), the polarity (`LPD`, `LPC`), lines, flashes and arcs (`G01`, `G02`,
//! `G03`, `G74`, `G75`), regions (`G36`, `G37`), the file attributes (`TF`) and the old unit
//! codes (`G70`, `G71`). Skipped with a finding: step and repeat, aperture blocks and transforms
//! (the file is marked incomplete). Everything is bounded by the settings.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{FRAC_PI_2, PI};

use crate::codes::{
    Finding, GbrError, Severity, APERTURE_INVALID, ARC_INVALID, COORDINATE_RANGE, FEATURE_SKIPPED,
    FORMAT_INVALID, GERBER_SYNTAX, INPUT_TOO_LARGE, LIMIT_EXCEEDED, MACRO_INVALID, NO_END,
    REGION_INVALID,
};
use crate::macros::instantiate;
use crate::model::{Aperture, ApertureShape, GerberLayer, Item, Units, P};
use crate::settings::{ImportSettings, ZeroOmission};
use crate::units::{decimal_to_nm, integer_to_nm, split_sign};

/// The most findings that one file keeps; the rest are counted out.
const MAX_FINDINGS: usize = 1000;

/// The longest block (a command up to its `*`) that is read, in characters.
const MAX_BLOCK_CHARS: usize = 1 << 20;

struct Block {
    text: String,
    line: usize,
    extended: bool,
}

fn syntax(line: usize, detail: impl Into<String>) -> GbrError {
    GbrError::at(GERBER_SYNTAX, line, detail)
}

/// Cuts the text into blocks: the commands between `%` and `%`, and the data blocks up to `*`.
fn split_blocks(text: &str, settings: &ImportSettings) -> Result<Vec<Block>, GbrError> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut current = String::new();
    let mut in_extended = false;
    let mut start_line = 1usize;
    let mut line = 1usize;
    for c in text.chars() {
        if c == '\n' {
            line += 1;
        }
        if c == '\n' || c == '\r' {
            continue;
        }
        if c == '%' {
            if in_extended {
                blocks.push(Block {
                    text: std::mem::take(&mut current),
                    line: start_line,
                    extended: true,
                });
                in_extended = false;
            } else {
                if !current.trim().is_empty() {
                    return Err(syntax(line, "a data block does not end with '*'"));
                }
                current.clear();
                in_extended = true;
                start_line = line;
            }
        } else if c == '*' && !in_extended {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                blocks.push(Block {
                    text: trimmed.to_owned(),
                    line: start_line,
                    extended: false,
                });
            }
            current.clear();
        } else {
            if current.is_empty() && !in_extended {
                start_line = line;
            }
            current.push(c);
            if current.len() > MAX_BLOCK_CHARS {
                return Err(GbrError::at(
                    LIMIT_EXCEEDED,
                    line,
                    "a command is longer than the limit",
                ));
            }
        }
        if blocks.len() > settings.max_commands {
            return Err(GbrError::at(
                LIMIT_EXCEEDED,
                line,
                format!("more than {} commands", settings.max_commands),
            ));
        }
    }
    if in_extended {
        return Err(syntax(
            start_line,
            "an extended command is not closed by '%'",
        ));
    }
    if !current.trim().is_empty() {
        return Err(syntax(line, "the file ends inside a data block"));
    }
    Ok(blocks)
}

/// The letter-and-number words of a data block (`X100Y-5D01` gives X, Y and D).
pub(crate) fn words(block: &str) -> Option<Vec<(char, &str)>> {
    let bytes = block.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' {
            i += 1;
            continue;
        }
        if !c.is_ascii_uppercase() {
            return None;
        }
        let start = i + 1;
        let mut j = start;
        while j < bytes.len()
            && (bytes[j].is_ascii_digit() || matches!(bytes[j], b'+' | b'-' | b'.'))
        {
            j += 1;
        }
        out.push((c as char, &block[start..j]));
        i = j;
    }
    Some(out)
}

#[derive(Clone, Copy)]
struct Format {
    omission: ZeroOmission,
    x_integer: u32,
    x_decimal: u32,
    y_integer: u32,
    y_decimal: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Interpolation {
    Linear,
    Clockwise,
    CounterClockwise,
}

struct Reader<'a> {
    settings: &'a ImportSettings,
    units: Option<Units>,
    format: Option<Format>,
    dark: bool,
    interpolation: Interpolation,
    multi_quadrant: bool,
    region: bool,
    contour: Vec<P>,
    apertures: BTreeMap<u32, Aperture>,
    macros: BTreeMap<String, Vec<String>>,
    selected: Option<u32>,
    at: P,
    last_op: Option<u32>,
    items: Vec<Item>,
    region_points: usize,
    findings: Vec<Finding>,
    noted: BTreeSet<&'static str>,
    attributes: BTreeMap<String, Vec<String>>,
    block_depth: u32,
    ended: bool,
    complete: bool,
}

fn angle_of(from: P, centre: P) -> f64 {
    ((from.y - centre.y) as f64).atan2((from.x - centre.x) as f64)
}

fn distance(a: P, b: P) -> f64 {
    ((a.x - b.x) as f64).hypot((a.y - b.y) as f64)
}

/// The angle that an arc covers, from 0 to a full turn. Equal ends are a full circle.
fn sweep_of(from: P, to: P, centre: P, clockwise: bool) -> f64 {
    if from == to {
        return 2.0 * PI;
    }
    let (a0, a1) = (angle_of(from, centre), angle_of(to, centre));
    let raw = if clockwise { a0 - a1 } else { a1 - a0 };
    raw.rem_euclid(2.0 * PI)
}

impl<'a> Reader<'a> {
    fn new(settings: &'a ImportSettings) -> Reader<'a> {
        Reader {
            settings,
            units: None,
            format: None,
            dark: true,
            interpolation: Interpolation::Linear,
            multi_quadrant: true,
            region: false,
            contour: Vec::new(),
            apertures: BTreeMap::new(),
            macros: BTreeMap::new(),
            selected: None,
            at: P { x: 0, y: 0 },
            last_op: None,
            items: Vec::new(),
            region_points: 0,
            findings: Vec::new(),
            noted: BTreeSet::new(),
            attributes: BTreeMap::new(),
            block_depth: 0,
            ended: false,
            complete: true,
        }
    }

    fn note(&mut self, code: &'static str, severity: Severity, line: usize, detail: &str) {
        if self.findings.len() < MAX_FINDINGS {
            self.findings
                .push(Finding::new(code, severity, line, detail));
        }
    }

    fn note_once(
        &mut self,
        key: &'static str,
        code: &'static str,
        severity: Severity,
        line: usize,
        detail: &str,
    ) {
        if self.noted.insert(key) {
            self.note(code, severity, line, detail);
        }
    }

    fn skipped(&mut self, key: &'static str, line: usize, detail: &str) {
        self.complete = false;
        self.note_once(key, FEATURE_SKIPPED, Severity::Warning, line, detail);
    }

    fn unit_nm(&self, line: usize) -> Result<i64, GbrError> {
        self.units.map(Units::nm).ok_or_else(|| {
            GbrError::at(
                FORMAT_INVALID,
                line,
                "a size or a coordinate comes before the units (MO)",
            )
        })
    }

    fn check_range(&self, value: i64, line: usize) -> Result<i64, GbrError> {
        if value.abs() > self.settings.coordinate_limit_nm {
            Err(GbrError::at(
                COORDINATE_RANGE,
                line,
                format!("{value} nm is beyond the limit"),
            ))
        } else {
            Ok(value)
        }
    }

    fn size(&self, text: &str, line: usize) -> Result<i64, GbrError> {
        let value = decimal_to_nm(text, self.unit_nm(line)?).ok_or_else(|| {
            GbrError::at(APERTURE_INVALID, line, format!("'{text}' is not a size"))
        })?;
        if value < 0 {
            return Err(GbrError::at(
                APERTURE_INVALID,
                line,
                format!("'{text}' is negative"),
            ));
        }
        self.check_range(value, line)
    }

    fn coordinate(&self, raw: &str, is_x: bool, line: usize) -> Result<i64, GbrError> {
        let unit_nm = self.unit_nm(line)?;
        let value = if raw.contains('.') {
            decimal_to_nm(raw, unit_nm)
        } else {
            let format = self.format.ok_or_else(|| {
                GbrError::at(
                    FORMAT_INVALID,
                    line,
                    "a coordinate comes before the format (FS)",
                )
            })?;
            let (negative, digits) = split_sign(raw);
            let (integer, decimal) = if is_x {
                (format.x_integer, format.x_decimal)
            } else {
                (format.y_integer, format.y_decimal)
            };
            integer_to_nm(digits, negative, integer, decimal, format.omission, unit_nm)
        };
        let value = value.ok_or_else(|| {
            GbrError::at(
                GERBER_SYNTAX,
                line,
                format!("'{raw}' is not a coordinate of the format"),
            )
        })?;
        self.check_range(value, line)
    }

    fn push(&mut self, item: Item, line: usize) -> Result<(), GbrError> {
        if self.items.len() >= self.settings.max_objects {
            return Err(GbrError::at(
                LIMIT_EXCEEDED,
                line,
                format!("more than {} objects", self.settings.max_objects),
            ));
        }
        self.items.push(item);
        Ok(())
    }

    fn extended(&mut self, text: &str, line: usize) -> Result<(), GbrError> {
        let parts: Vec<&str> = text.split('*').collect();
        let command = parts[0];
        if !command.is_ascii() {
            return Err(syntax(
                line,
                "a command holds characters that are not ASCII",
            ));
        }
        if command.starts_with("AM") {
            return self.define_macro(&parts, line);
        }
        if command.len() < 2 {
            return Err(syntax(line, "an empty extended command"));
        }
        match &command[..2] {
            "FS" => self.format_command(command, line),
            "MO" => match command {
                "MOMM" => {
                    self.units = Some(Units::Millimetres);
                    Ok(())
                }
                "MOIN" => {
                    self.units = Some(Units::Inches);
                    Ok(())
                }
                _ => Err(GbrError::at(
                    FORMAT_INVALID,
                    line,
                    format!("'{command}' is not a unit"),
                )),
            },
            "AD" => self.define_aperture(command, line),
            "LP" => match command {
                "LPD" => {
                    self.dark = true;
                    Ok(())
                }
                "LPC" => {
                    self.dark = false;
                    Ok(())
                }
                _ => Err(syntax(line, format!("'{command}' is not a polarity"))),
            },
            "TF" => {
                let mut values = command[2..].split(',');
                let name = values.next().unwrap_or("").to_owned();
                if name.is_empty() {
                    return Err(syntax(line, "an attribute without a name"));
                }
                let rest: Vec<String> = values.map(str::to_owned).collect();
                self.attributes.insert(name, rest);
                Ok(())
            }
            "TA" | "TO" | "TD" | "IN" | "LN" => Ok(()),
            "SR" => {
                if command.len() > 2 {
                    self.skipped("SR", line, "step and repeat (SR)");
                }
                Ok(())
            }
            "AB" => {
                if command.len() > 2 {
                    self.block_depth += 1;
                    self.skipped("AB", line, "aperture blocks (AB)");
                    if let Some(number) = command[2..]
                        .strip_prefix('D')
                        .and_then(|n| n.parse::<u32>().ok())
                    {
                        self.apertures.insert(
                            number,
                            Aperture {
                                shape: ApertureShape::Macro {
                                    name: "AB".to_owned(),
                                    shapes: Vec::new(),
                                },
                                hole: None,
                            },
                        );
                    }
                } else {
                    self.block_depth = self.block_depth.saturating_sub(1);
                }
                Ok(())
            }
            "LM" | "LR" | "LS" => {
                self.skipped("LT", line, "aperture transforms (LM, LR, LS)");
                Ok(())
            }
            "IP" => {
                if command == "IPNEG" {
                    self.skipped("IPNEG", line, "negative image polarity (IPNEG)");
                }
                Ok(())
            }
            "OF" | "SF" | "MI" | "AS" | "IR" => {
                self.skipped(
                    "OLD",
                    line,
                    "an obsolete image command (OF, SF, MI, AS, IR)",
                );
                Ok(())
            }
            _ => Err(syntax(line, format!("unknown command '{command}'"))),
        }
    }

    fn format_command(&mut self, command: &str, line: usize) -> Result<(), GbrError> {
        let bad = |what: &str| GbrError::at(FORMAT_INVALID, line, what.to_owned());
        let rest = command[2..].as_bytes();
        let omission = match rest.first() {
            Some(b'L' | b'D') => ZeroOmission::Leading,
            Some(b'T') => ZeroOmission::Trailing,
            _ => return Err(bad("the zero omission is not L, T or D")),
        };
        match rest.get(1) {
            Some(b'A') => {}
            Some(b'I') => return Err(bad("incremental coordinates are not read")),
            _ => return Err(bad("the coordinate mode is not A")),
        }
        let tail = &rest[2..];
        if tail.len() != 6
            || tail[0] != b'X'
            || tail[3] != b'Y'
            || ![tail[1], tail[2], tail[4], tail[5]]
                .iter()
                .all(u8::is_ascii_digit)
        {
            return Err(bad("the format is not XnnYnn"));
        }
        let digit = |b: u8| u32::from(b - b'0');
        let format = Format {
            omission,
            x_integer: digit(tail[1]),
            x_decimal: digit(tail[2]),
            y_integer: digit(tail[4]),
            y_decimal: digit(tail[5]),
        };
        for (integer, decimal) in [
            (format.x_integer, format.x_decimal),
            (format.y_integer, format.y_decimal),
        ] {
            if integer == 0 || decimal == 0 || integer + decimal > 15 {
                return Err(bad("the digits of the format are not 1 to 8"));
            }
        }
        self.format = Some(format);
        Ok(())
    }

    fn define_macro(&mut self, parts: &[&str], line: usize) -> Result<(), GbrError> {
        let name = parts[0][2..].trim();
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'$'))
        {
            return Err(GbrError::at(
                MACRO_INVALID,
                line,
                "a macro without a usable name",
            ));
        }
        let statements: Vec<String> = parts[1..]
            .iter()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        if statements.len() > 4 * self.settings.max_macro_primitives {
            return Err(GbrError::at(
                LIMIT_EXCEEDED,
                line,
                "a macro has more statements than the limit",
            ));
        }
        self.macros.insert(name.to_owned(), statements);
        Ok(())
    }

    fn define_aperture(&mut self, command: &str, line: usize) -> Result<(), GbrError> {
        let bad = |what: String| GbrError::at(APERTURE_INVALID, line, what);
        let body = command[2..]
            .strip_prefix('D')
            .ok_or_else(|| bad("an aperture without D".to_owned()))?;
        let digits = body.bytes().take_while(u8::is_ascii_digit).count();
        let number: u32 = body[..digits]
            .parse()
            .map_err(|_| bad("an aperture without a number".to_owned()))?;
        if number < 10 {
            return Err(bad(format!("D{number} is reserved")));
        }
        if self.apertures.len() >= self.settings.max_apertures {
            return Err(GbrError::at(
                LIMIT_EXCEEDED,
                line,
                format!("more than {} apertures", self.settings.max_apertures),
            ));
        }
        let rest = &body[digits..];
        let (name, list) = rest.split_once(',').unwrap_or((rest, ""));
        let params: Vec<&str> = if list.is_empty() {
            Vec::new()
        } else {
            list.split('X').collect()
        };
        let hole_at = |index: usize| -> Result<Option<i64>, GbrError> {
            params.get(index).map(|p| self.size(p, line)).transpose()
        };
        let need = |low: usize, high: usize| -> Result<(), GbrError> {
            if (low..=high).contains(&params.len()) {
                Ok(())
            } else {
                Err(bad(format!(
                    "aperture {name} takes {low} to {high} parameters, not {}",
                    params.len()
                )))
            }
        };
        let aperture = match name {
            "C" => {
                need(1, 2)?;
                Aperture {
                    shape: ApertureShape::Circle {
                        diameter: self.size(params[0], line)?,
                    },
                    hole: hole_at(1)?,
                }
            }
            "R" | "O" => {
                need(2, 3)?;
                let (width, height) = (self.size(params[0], line)?, self.size(params[1], line)?);
                Aperture {
                    shape: if name == "R" {
                        ApertureShape::Rect { width, height }
                    } else {
                        ApertureShape::Obround { width, height }
                    },
                    hole: hole_at(2)?,
                }
            }
            "P" => {
                need(2, 4)?;
                let vertices: u32 = params[1]
                    .parse()
                    .ok()
                    .filter(|v| (3..=12).contains(v))
                    .ok_or_else(|| bad("a polygon has 3 to 12 corners".to_owned()))?;
                let rotation = match params.get(2) {
                    Some(text) => {
                        let degrees = parse_number(text)
                            .ok_or_else(|| bad(format!("'{text}' is not an angle")))?;
                        if degrees.abs() > 1.0e6 {
                            return Err(bad("an angle is out of range".to_owned()));
                        }
                        (degrees * 1.0e6).round() as i64
                    }
                    None => 0,
                };
                Aperture {
                    shape: ApertureShape::Polygon {
                        diameter: self.size(params[0], line)?,
                        vertices,
                        rotation,
                    },
                    hole: hole_at(3)?,
                }
            }
            other => {
                let statements = self.macros.get(other).ok_or_else(|| {
                    GbrError::at(
                        MACRO_INVALID,
                        line,
                        format!("aperture {other} is neither C, R, O, P nor a macro"),
                    )
                })?;
                let mut values = Vec::with_capacity(params.len());
                for text in &params {
                    values.push(parse_number(text).ok_or_else(|| {
                        GbrError::at(MACRO_INVALID, line, format!("'{text}' is not a number"))
                    })?);
                }
                let unit_nm = self.unit_nm(line)?;
                let (shapes, skipped) = instantiate(statements, &values, unit_nm, self.settings)
                    .map_err(|e| GbrError::at(MACRO_INVALID, line, format!("{other}: {e}")))?;
                if skipped > 0 {
                    self.skipped(
                        "MACRO",
                        line,
                        "a thermal or moire primitive of a macro was left out",
                    );
                }
                Aperture {
                    shape: ApertureShape::Macro {
                        name: other.to_owned(),
                        shapes,
                    },
                    hole: None,
                }
            }
        };
        self.apertures.insert(number, aperture);
        Ok(())
    }

    fn arc_centre(
        &mut self,
        from: P,
        to: P,
        i: i64,
        j: i64,
        clockwise: bool,
        line: usize,
    ) -> Result<P, GbrError> {
        if self.multi_quadrant {
            let centre = P {
                x: from.x + i,
                y: from.y + j,
            };
            let difference = (distance(from, centre) - distance(to, centre)).abs();
            if difference > self.settings.arc_radius_tolerance_nm as f64 {
                self.note(
                    ARC_INVALID,
                    Severity::Warning,
                    line,
                    "the two radii of an arc differ more than the tolerance",
                );
            }
            return Ok(centre);
        }
        if from == to {
            return Err(GbrError::at(
                ARC_INVALID,
                line,
                "a full circle needs the multi quadrant mode (G75)",
            ));
        }
        let (i, j) = (i.abs(), j.abs());
        let mut best: Option<(f64, P)> = None;
        for (sx, sy) in [(1i64, 1i64), (1, -1), (-1, 1), (-1, -1)] {
            let centre = P {
                x: from.x + sx * i,
                y: from.y + sy * j,
            };
            if sweep_of(from, to, centre, clockwise) > FRAC_PI_2 + 1e-6 {
                continue;
            }
            let difference = (distance(from, centre) - distance(to, centre)).abs();
            if best.is_none_or(|(d, _)| difference < d) {
                best = Some((difference, centre));
            }
        }
        match best {
            Some((difference, centre))
                if difference <= self.settings.arc_radius_tolerance_nm as f64 =>
            {
                Ok(centre)
            }
            _ => Err(GbrError::at(
                ARC_INVALID,
                line,
                "no centre of the single quadrant mode fits the arc",
            )),
        }
    }

    /// The points of an arc after its start, the last one being the end.
    fn arc_points(&self, from: P, to: P, centre: P, clockwise: bool) -> Vec<P> {
        let radius = distance(from, centre);
        let sweep = sweep_of(from, to, centre, clockwise);
        let tolerance = self.settings.arc_chord_nm as f64;
        let step = if radius > tolerance {
            (2.0 * (1.0 - tolerance / radius).acos()).max(1e-4)
        } else {
            FRAC_PI_2
        };
        let pieces = ((sweep / step).ceil() as usize).clamp(1, self.settings.arc_max_segments);
        let start = angle_of(from, centre);
        let direction = if clockwise { -1.0 } else { 1.0 };
        let mut points = Vec::with_capacity(pieces);
        for k in 1..pieces {
            let angle = start + direction * sweep * (k as f64) / (pieces as f64);
            points.push(P {
                x: (centre.x as f64 + radius * angle.cos()).round() as i64,
                y: (centre.y as f64 + radius * angle.sin()).round() as i64,
            });
        }
        points.push(to);
        points
    }

    fn finish_contour(&mut self, line: usize) -> Result<(), GbrError> {
        let mut points = std::mem::take(&mut self.contour);
        let closed = points.len() > 1 && points.first() == points.last();
        if closed {
            points.pop();
        } else if points.len() >= 3 {
            self.note(
                REGION_INVALID,
                Severity::Warning,
                line,
                "a contour is not closed; it is closed by a straight line",
            );
        }
        if points.len() < 3 {
            if !points.is_empty() && (closed || points.len() > 1) {
                self.note(
                    REGION_INVALID,
                    Severity::Warning,
                    line,
                    "a contour has fewer than three corners and was left out",
                );
            }
            return Ok(());
        }
        let dark = self.dark;
        self.push(Item::Region { points, dark }, line)
    }

    fn graphic(&mut self, block: &str, line: usize) -> Result<(), GbrError> {
        let list = words(block).ok_or_else(|| syntax(line, format!("cannot read '{block}'")))?;
        let (mut x, mut y, mut i, mut j): (Option<&str>, Option<&str>, Option<&str>, Option<&str>) =
            (None, None, None, None);
        let (mut op, mut select): (Option<u32>, Option<u32>) = (None, None);
        for (letter, value) in list {
            let number = || -> Result<u32, GbrError> {
                value
                    .parse()
                    .map_err(|_| syntax(line, format!("'{letter}{value}' is not a code")))
            };
            match letter {
                'G' => match number()? {
                    0 | 54 | 55 | 90 => {}
                    1 => self.interpolation = Interpolation::Linear,
                    2 => self.interpolation = Interpolation::Clockwise,
                    3 => self.interpolation = Interpolation::CounterClockwise,
                    36 => {
                        if self.region {
                            return Err(GbrError::at(REGION_INVALID, line, "regions are nested"));
                        }
                        self.region = true;
                        self.contour.clear();
                    }
                    37 => {
                        if !self.region {
                            return Err(GbrError::at(REGION_INVALID, line, "G37 without G36"));
                        }
                        self.finish_contour(line)?;
                        self.region = false;
                    }
                    70 => self.units = Some(Units::Inches),
                    71 => self.units = Some(Units::Millimetres),
                    74 => self.multi_quadrant = false,
                    75 => self.multi_quadrant = true,
                    91 => {
                        return Err(GbrError::at(
                            FORMAT_INVALID,
                            line,
                            "incremental coordinates (G91) are not read",
                        ))
                    }
                    other => return Err(syntax(line, format!("G{other} is not known"))),
                },
                'D' => match number()? {
                    0 => {}
                    n @ 1..=3 => op = Some(n),
                    n if n >= 10 => select = Some(n),
                    n => return Err(syntax(line, format!("D{n} is not known"))),
                },
                'M' => match number()? {
                    0..=2 => self.ended = true,
                    n => return Err(syntax(line, format!("M{n} is not known"))),
                },
                'X' => x = Some(value),
                'Y' => y = Some(value),
                'I' => i = Some(value),
                'J' => j = Some(value),
                'N' => {}
                other => return Err(syntax(line, format!("the word '{other}' is not known"))),
            }
        }
        if let Some(n) = select {
            if !self.apertures.contains_key(&n) {
                return Err(GbrError::at(
                    APERTURE_INVALID,
                    line,
                    format!("D{n} is selected before it is defined"),
                ));
            }
            self.selected = Some(n);
        }
        let has_coordinates = x.is_some() || y.is_some() || i.is_some() || j.is_some();
        let operation = match (op, has_coordinates) {
            (Some(n), _) => {
                self.last_op = Some(n);
                Some(n)
            }
            (None, true) => Some(self.last_op.ok_or_else(|| {
                syntax(line, "coordinates without an operation (D01, D02 or D03)")
            })?),
            (None, false) => None,
        };
        let Some(operation) = operation else {
            return Ok(());
        };
        let to = P {
            x: match x {
                Some(text) => self.coordinate(text, true, line)?,
                None => self.at.x,
            },
            y: match y {
                Some(text) => self.coordinate(text, false, line)?,
                None => self.at.y,
            },
        };
        let offset_i = match i {
            Some(text) => self.coordinate(text, true, line)?,
            None => 0,
        };
        let offset_j = match j {
            Some(text) => self.coordinate(text, false, line)?,
            None => 0,
        };
        self.operate(operation, to, offset_i, offset_j, line)?;
        self.at = to;
        Ok(())
    }

    fn operate(
        &mut self,
        operation: u32,
        to: P,
        offset_i: i64,
        offset_j: i64,
        line: usize,
    ) -> Result<(), GbrError> {
        let from = self.at;
        match operation {
            2 => {
                if self.region {
                    self.finish_contour(line)?;
                    self.contour.push(to);
                    self.count_region_points(1, line)?;
                }
                Ok(())
            }
            1 => {
                let curved = self.interpolation != Interpolation::Linear;
                let clockwise = self.interpolation == Interpolation::Clockwise;
                if self.region {
                    if self.contour.is_empty() {
                        self.contour.push(from);
                        self.count_region_points(1, line)?;
                    }
                    if curved {
                        let centre =
                            self.arc_centre(from, to, offset_i, offset_j, clockwise, line)?;
                        let points = self.arc_points(from, to, centre, clockwise);
                        self.count_region_points(points.len(), line)?;
                        self.contour.extend(points);
                    } else {
                        self.contour.push(to);
                        self.count_region_points(1, line)?;
                    }
                    return Ok(());
                }
                let number = self.selected.ok_or_else(|| {
                    GbrError::at(APERTURE_INVALID, line, "a line is drawn with no aperture")
                })?;
                let (width, round) = match self.apertures.get(&number) {
                    Some(a) => (a.stroke_width(), a.is_circle()),
                    None => (0, true),
                };
                if !round {
                    self.note_once(
                        "STROKE",
                        APERTURE_INVALID,
                        Severity::Warning,
                        line,
                        "a line is drawn with an aperture that is not round; the smaller side is its width",
                    );
                }
                let dark = self.dark;
                if curved {
                    let centre = self.arc_centre(from, to, offset_i, offset_j, clockwise, line)?;
                    self.push(
                        Item::Arc {
                            from,
                            to,
                            centre,
                            clockwise,
                            width,
                            dark,
                        },
                        line,
                    )
                } else {
                    self.push(
                        Item::Line {
                            from,
                            to,
                            width,
                            dark,
                        },
                        line,
                    )
                }
            }
            _ => {
                if self.region {
                    self.note(
                        REGION_INVALID,
                        Severity::Warning,
                        line,
                        "a flash inside a region was left out",
                    );
                    return Ok(());
                }
                let number = self.selected.ok_or_else(|| {
                    GbrError::at(APERTURE_INVALID, line, "a flash is made with no aperture")
                })?;
                let dark = self.dark;
                self.push(
                    Item::Flash {
                        at: to,
                        aperture: number,
                        dark,
                    },
                    line,
                )
            }
        }
    }

    fn count_region_points(&mut self, more: usize, line: usize) -> Result<(), GbrError> {
        self.region_points += more;
        if self.region_points > self.settings.max_region_points {
            Err(GbrError::at(
                LIMIT_EXCEEDED,
                line,
                format!(
                    "more than {} region points",
                    self.settings.max_region_points
                ),
            ))
        } else {
            Ok(())
        }
    }
}

fn parse_number(text: &str) -> Option<f64> {
    let value: f64 = text.trim().parse().ok()?;
    value.is_finite().then_some(value)
}

/// Reads a Gerber file. An error is a file that cannot be read at all; what is read but doubtful
/// is a finding of the result.
pub fn parse_gerber(text: &str, settings: &ImportSettings) -> Result<GerberLayer, GbrError> {
    if text.len() > settings.max_file_bytes {
        return Err(GbrError::new(
            INPUT_TOO_LARGE,
            format!(
                "{} bytes, the limit is {}",
                text.len(),
                settings.max_file_bytes
            ),
        ));
    }
    let blocks = split_blocks(text, settings)?;
    if blocks.is_empty() {
        return Err(syntax(0, "the file has no command"));
    }
    let mut reader = Reader::new(settings);
    for block in &blocks {
        if reader.ended {
            break;
        }
        if block.extended {
            reader.extended(&block.text, block.line)?;
        } else if block.text.starts_with("G04") || reader.block_depth > 0 {
            continue;
        } else {
            reader.graphic(&block.text, block.line)?;
        }
    }
    let last_line = blocks.last().map_or(0, |b| b.line);
    if reader.region {
        reader.note(
            REGION_INVALID,
            Severity::Warning,
            last_line,
            "the file ends inside a region",
        );
        reader.finish_contour(last_line)?;
    }
    if !reader.ended {
        reader.note(NO_END, Severity::Warning, last_line, "no M02");
    }
    Ok(GerberLayer {
        units: reader.units.unwrap_or(Units::Millimetres),
        attributes: reader.attributes,
        apertures: reader.apertures,
        items: reader.items,
        findings: reader.findings,
        ended: reader.ended,
        complete: reader.complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MacroShape;

    fn read(text: &str) -> GerberLayer {
        parse_gerber(text, &ImportSettings::default()).unwrap()
    }

    fn fail(text: &str) -> GbrError {
        parse_gerber(text, &ImportSettings::default()).unwrap_err()
    }

    const HEAD: &str = "%FSLAX46Y46*%\n%MOMM*%\n";

    #[test]
    fn lines_flashes_and_attributes() {
        let layer = read(&format!(
            "%TF.FileFunction,Copper,L1,Top*%\n{HEAD}%ADD10C,0.25*%\n%ADD11R,1.5X0.8*%\n%ADD12O,2X1X0.3*%\n%ADD13P,1.2X6X15*%\nD10*\nX1000000Y2000000D02*\nG01*\nX5500000Y2000000D01*\nD11*\nX3000000Y3000000D03*\nD12*\nX-1500000Y-2500000D03*\nM02*\n"
        ));
        assert!(layer.ended && layer.complete);
        assert_eq!(layer.findings, Vec::new());
        assert_eq!(layer.units, Units::Millimetres);
        assert_eq!(
            layer.attributes[".FileFunction"],
            vec!["Copper", "L1", "Top"]
        );
        assert_eq!(layer.apertures.len(), 4);
        assert_eq!(
            layer.apertures[&12],
            Aperture {
                shape: ApertureShape::Obround {
                    width: 2_000_000,
                    height: 1_000_000
                },
                hole: Some(300_000)
            }
        );
        assert_eq!(
            layer.apertures[&13].shape,
            ApertureShape::Polygon {
                diameter: 1_200_000,
                vertices: 6,
                rotation: 15_000_000
            }
        );
        assert_eq!(layer.items.len(), 3);
        assert_eq!(
            layer.items[0],
            Item::Line {
                from: P {
                    x: 1_000_000,
                    y: 2_000_000
                },
                to: P {
                    x: 5_500_000,
                    y: 2_000_000
                },
                width: 250_000,
                dark: true
            }
        );
        assert_eq!(
            layer.items[2],
            Item::Flash {
                at: P {
                    x: -1_500_000,
                    y: -2_500_000
                },
                aperture: 12,
                dark: true
            }
        );
        assert_eq!((layer.flashes(), layer.lines(), layer.arcs()), (2, 1, 0));
    }

    #[test]
    fn omitted_coordinates_keep_their_value_and_the_operation_is_modal() {
        let layer = read(&format!(
            "{HEAD}%ADD10C,0.1*%\nD10*\nX1000000Y1000000D02*\nX2000000D01*\nY3000000*\nM02*\n"
        ));
        let ends: Vec<P> = layer
            .items
            .iter()
            .map(|i| match i {
                Item::Line { to, .. } => *to,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            ends,
            vec![
                P {
                    x: 2_000_000,
                    y: 1_000_000
                },
                P {
                    x: 2_000_000,
                    y: 3_000_000
                }
            ]
        );
    }

    #[test]
    fn zero_omission_units_and_the_old_unit_codes() {
        // inches, 2.4, trailing zeros left out: 15 is 15.0000 in and 2 is 20.0000 in
        let layer = read("%FSTAX24Y24*%\n%MOIN*%\n%ADD10C,0.01*%\nD10*\nX15Y2D03*\nM02*\n");
        assert_eq!(
            layer.items[0],
            Item::Flash {
                at: P {
                    x: 381_000_000,
                    y: 508_000_000
                },
                aperture: 10,
                dark: true
            }
        );
        assert_eq!(
            layer.apertures[&10].shape,
            ApertureShape::Circle { diameter: 254_000 }
        );
        let old = read("%FSLAX24Y24*%\nG70*\n%ADD10C,0.01*%\nD10*\nX10000Y0D03*\nG71*\nM02*\n");
        assert_eq!(old.units, Units::Millimetres);
        // a decimal point in a coordinate is allowed
        let dotted = read(&format!(
            "{HEAD}%ADD10C,0.1*%\nD10*\nX1.5Y-2.25D03*\nM02*\n"
        ));
        assert_eq!(
            dotted.items[0],
            Item::Flash {
                at: P {
                    x: 1_500_000,
                    y: -2_250_000
                },
                aperture: 10,
                dark: true
            }
        );
    }

    #[test]
    fn polarity_is_kept_per_object() {
        let layer = read(&format!(
            "{HEAD}%ADD10C,1*%\nD10*\n%LPD*%\nX0Y0D03*\n%LPC*%\nX1000000Y0D03*\n%LPD*%\nX2000000Y0D03*\nM02*\n"
        ));
        let polarity: Vec<bool> = layer
            .items
            .iter()
            .map(|i| match i {
                Item::Flash { dark, .. } => *dark,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(polarity, vec![true, false, true]);
    }

    #[test]
    fn regions_close_and_split_at_each_move() {
        let layer = read(&format!(
            "{HEAD}G36*\nX0Y0D02*\nG01*\nX1000000Y0D01*\nX1000000Y1000000D01*\nX0Y0D01*\nX5000000Y5000000D02*\nX6000000Y5000000D01*\nX6000000Y6000000D01*\nG37*\nM02*\n"
        ));
        assert_eq!(layer.regions(), 2);
        match &layer.items[0] {
            Item::Region { points, dark } => {
                assert_eq!(points.len(), 3);
                assert!(*dark);
            }
            other => panic!("{other:?}"),
        }
        // the second contour was open: a finding, and it is closed
        assert!(layer
            .findings
            .iter()
            .any(|f| f.code == REGION_INVALID && f.detail.contains("not closed")));
    }

    #[test]
    fn multi_quadrant_arcs_in_a_region_become_straight_pieces() {
        let layer = read(&format!(
            "{HEAD}G36*\nX0Y0D02*\nG01*\nX2000000Y0D01*\nG75*\nG03*\nX0Y2000000I-2000000J0D01*\nG01*\nX0Y0D01*\nG37*\nM02*\n"
        ));
        match &layer.items[0] {
            Item::Region { points, .. } => {
                // a quarter circle of 2 mm radius within 5 um needs a dozen pieces
                assert!(points.len() > 10, "{}", points.len());
                let end = points[points.len() - 1];
                assert_eq!(end, P { x: 0, y: 2_000_000 });
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn drawn_arcs_keep_their_centre_and_direction() {
        let layer = read(&format!(
            "{HEAD}%ADD10C,0.1*%\nD10*\nX2000000Y1000000D02*\nG75*\nG03*\nX1000000Y2000000I-1000000J0D01*\nG02*\nX2000000Y1000000I0J-1000000D01*\nM02*\n"
        ));
        assert_eq!(layer.arcs(), 2);
        match layer.items[0] {
            Item::Arc {
                centre, clockwise, ..
            } => {
                assert_eq!(
                    centre,
                    P {
                        x: 1_000_000,
                        y: 1_000_000
                    }
                );
                assert!(!clockwise);
            }
            ref other => panic!("{other:?}"),
        }
        assert_eq!(layer.findings, Vec::new());
    }

    #[test]
    fn a_single_quadrant_arc_takes_the_centre_that_fits() {
        let layer = read(&format!(
            "{HEAD}%ADD10C,0.1*%\nD10*\nX2000000Y1000000D02*\nG74*\nG03*\nX1000000Y2000000I1000000J0D01*\nM02*\n"
        ));
        match layer.items[0] {
            Item::Arc { centre, .. } => assert_eq!(
                centre,
                P {
                    x: 1_000_000,
                    y: 1_000_000
                }
            ),
            ref other => panic!("{other:?}"),
        }
        // a full circle needs the multi quadrant mode
        let e = fail(&format!(
            "{HEAD}%ADD10C,0.1*%\nD10*\nX2000000Y1000000D02*\nG74*\nG03*\nX2000000Y1000000I1000000J0D01*\nM02*\n"
        ));
        assert_eq!(e.code, ARC_INVALID);
        let full = read(&format!(
            "{HEAD}%ADD10C,0.1*%\nD10*\nX2000000Y1000000D02*\nG75*\nG03*\nX2000000Y1000000I-1000000J0D01*\nM02*\n"
        ));
        match full.items[0] {
            Item::Arc { centre, .. } => assert_eq!(
                centre,
                P {
                    x: 1_000_000,
                    y: 1_000_000
                }
            ),
            ref other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_macro_aperture_is_evaluated_with_its_parameters() {
        let layer = read(&format!(
            "{HEAD}%AMBOX*\n0 a box*\n21,1,$1,$2,0,0,0*\n1,0,$3,0,0*%\n%ADD10BOX,2X1X0.4*%\nD10*\nX0Y0D03*\nM02*\n"
        ));
        match &layer.apertures[&10].shape {
            ApertureShape::Macro { name, shapes } => {
                assert_eq!(name, "BOX");
                assert_eq!(shapes.len(), 2);
                assert_eq!(
                    shapes[0],
                    MacroShape::CenterLine {
                        exposure: true,
                        width: 2_000_000,
                        height: 1_000_000,
                        centre: P { x: 0, y: 0 },
                        rotation: 0
                    }
                );
            }
            other => panic!("{other:?}"),
        }
        let e = fail(&format!("{HEAD}%ADD10NOPE,1*%\nM02*\n"));
        assert_eq!(e.code, MACRO_INVALID);
    }

    #[test]
    fn features_that_are_not_read_are_reported_and_the_file_is_incomplete() {
        let layer = read(&format!(
            "{HEAD}%ADD10C,0.1*%\n%SRX2Y2I1J1*%\nD10*\nX0Y0D03*\n%SR*%\n%LMX*%\n%ABD11*%\nX5Y5D03*\n%AB*%\nM02*\n"
        ));
        assert!(!layer.complete);
        assert_eq!(layer.flashes(), 1);
        let skipped = layer
            .findings
            .iter()
            .filter(|f| f.code == FEATURE_SKIPPED)
            .count();
        assert_eq!(skipped, 3);
    }

    #[test]
    fn errors_have_stable_codes() {
        for (text, code) in [
            (String::new(), GERBER_SYNTAX),
            ("X0Y0D03*\nM02*\n".to_owned(), FORMAT_INVALID),
            (format!("{HEAD}D10*\nM02*\n"), APERTURE_INVALID),
            (
                format!("{HEAD}%ADD10C,1*%\nX0Y0D03*\nM02*\n"),
                APERTURE_INVALID,
            ),
            (
                format!("{HEAD}%ADD10C,1*%\nD10*\nX0Y0\nM02*\n"),
                GERBER_SYNTAX,
            ),
            (
                format!("{HEAD}%ADD10C,1*%\nD10*\nX0Y0D07*\nM02*\n"),
                GERBER_SYNTAX,
            ),
            (format!("{HEAD}%ADD9C,1*%\nM02*\n"), APERTURE_INVALID),
            (format!("{HEAD}%ADD10C*%\nM02*\n"), APERTURE_INVALID),
            (format!("{HEAD}%ADD10C,x*%\nM02*\n"), APERTURE_INVALID),
            (format!("{HEAD}%ADD10P,1X2*%\nM02*\n"), APERTURE_INVALID),
            (format!("{HEAD}%QQ*%\nM02*\n"), GERBER_SYNTAX),
            (format!("{HEAD}%LPX*%\nM02*\n"), GERBER_SYNTAX),
            ("%FSLIX46Y46*%\n".to_owned(), FORMAT_INVALID),
            ("%FSLAX46Y46*%\n%MOXX*%\n".to_owned(), FORMAT_INVALID),
            ("%FSLAX46Y4*%\n".to_owned(), FORMAT_INVALID),
            ("%FSLAX06Y46*%\n".to_owned(), FORMAT_INVALID),
            (format!("{HEAD}G91*\nM02*\n"), FORMAT_INVALID),
            (format!("{HEAD}G37*\nM02*\n"), REGION_INVALID),
            (format!("{HEAD}G36*\nG36*\nM02*\n"), REGION_INVALID),
            (
                format!("{HEAD}%ADD10C,1*%\nD10*\nX99999999999999D03*\nM02*\n"),
                COORDINATE_RANGE,
            ),
            (format!("{HEAD}%ADD10C,1*%\nD10*\nX1Y1D03\n"), GERBER_SYNTAX),
            (format!("{HEAD}%ADD10C,1*\nM02*\n"), GERBER_SYNTAX),
            (format!("{HEAD}Q1*\nM02*\n"), GERBER_SYNTAX),
        ] {
            let e = parse_gerber(&text, &ImportSettings::default()).unwrap_err();
            assert_eq!(e.code, code, "{text:?}: {e}");
        }
    }

    #[test]
    fn a_file_without_its_end_command_is_read_with_a_finding() {
        let layer = read(&format!("{HEAD}%ADD10C,1*%\nD10*\nX0Y0D03*\n"));
        assert!(!layer.ended);
        assert_eq!(layer.flashes(), 1);
        assert_eq!(layer.findings.len(), 1);
        assert_eq!(layer.findings[0].code, NO_END);
        // what follows the end command is not read
        let after = read(&format!(
            "{HEAD}%ADD10C,1*%\nD10*\nX0Y0D03*\nM02*\nX1Y1D03*\n"
        ));
        assert_eq!(after.flashes(), 1);
    }

    #[test]
    fn the_limits_of_the_settings_are_enforced() {
        let text = format!("{HEAD}%ADD10C,1*%\nD10*\nX0Y0D03*\nX1000000Y0D03*\nM02*\n");
        let objects = ImportSettings {
            max_objects: 1,
            ..ImportSettings::default()
        };
        assert_eq!(
            parse_gerber(&text, &objects).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        let commands = ImportSettings {
            max_commands: 3,
            ..ImportSettings::default()
        };
        assert_eq!(
            parse_gerber(&text, &commands).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        let apertures = ImportSettings {
            max_apertures: 1,
            ..ImportSettings::default()
        };
        let two = format!("{HEAD}%ADD10C,1*%\n%ADD11C,2*%\nM02*\n");
        assert_eq!(
            parse_gerber(&two, &apertures).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        let bytes = ImportSettings {
            max_file_bytes: 10,
            ..ImportSettings::default()
        };
        assert_eq!(
            parse_gerber(&text, &bytes).unwrap_err().code,
            INPUT_TOO_LARGE
        );
        let points = ImportSettings {
            max_region_points: 2,
            ..ImportSettings::default()
        };
        let region =
            format!("{HEAD}G36*\nX0Y0D02*\nX1000000Y0D01*\nX1000000Y1000000D01*\nG37*\nM02*\n");
        assert_eq!(
            parse_gerber(&region, &points).unwrap_err().code,
            LIMIT_EXCEEDED
        );
    }

    #[test]
    fn the_line_of_an_error_is_given() {
        let e = fail(&format!("{HEAD}%ADD10C,1*%\nD10*\n\nX0Y0D07*\nM02*\n"));
        assert_eq!(e.line, 6);
    }
}
