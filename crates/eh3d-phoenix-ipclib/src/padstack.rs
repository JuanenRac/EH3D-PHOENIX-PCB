// SPDX-License-Identifier: GPL-3.0-or-later
//! The names of padstacks, made and read by rule.
//!
//! A padstack name says what the padstack is: a letter for the shape (`c` circle, `s` square,
//! `r` rectangle, `b` oblong, `d` D-shaped, `u` outline of the user, `v` via, `vs` square via, `w`
//! mounting hole), its sizes, the hole (`h` plated, `hn` not plated, a slot as length and width),
//! and the modifiers: `z` the land of the inner layers, `x` the land of the opposite side, `m` the
//! solder mask opening, `mx` the opening of the opposite side, `p` the paste, `a` the assembly
//! land, `y` the antipad of the planes, `t` the thermal relief (inner diameter, outer diameter
//! and spoke width, and `_2` for two spokes instead of four) and `l` the layers a via joins.
//! Sizes are in millimetres with two decimals, without the point and without leading zeros: 1.50
//! is `150` and 0.15 is `15`. Two sizes are joined by `_` (`r200_100`, `c130_95` for a donut).
//!
//! [`Padstack::name`] writes the one canonical name of a padstack, [`Padstack::parse`] reads a
//! name (the modifiers may come in any order), [`Padstack::validate`] finds what cannot be (a hole
//! as large as its pad) and [`check_name`] gives all of it as a list of findings. The conversion
//! of nanometres to hundredths of a millimetre rounds half away from zero, in integers. The
//! characters that Windows does not allow in a file name are refused.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// One hundredth of a millimetre, in nanometres.
pub const UNIT_NM: i64 = 10_000;

/// The characters that no name may hold (they cannot be in a file name on Windows, or they are
/// used by the formats that carry the name).
pub const FORBIDDEN: &str = "\",;:/\\[](){}.*&%#$!@^= ";

/// The shape of the land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `c`: a circle, or a donut with a second size.
    Circle,
    /// `s`: a square.
    Square,
    /// `r`: a rectangle, length then width.
    Rect,
    /// `b`: an oblong, length then width.
    Oblong,
    /// `d`: a D shape, length then width.
    DShape,
    /// `u`: the outline of the user, numbered.
    User,
    /// `v`: a via with a circular land.
    Via,
    /// `vs`: a via with a square land.
    ViaSquare,
    /// `w`: a mounting hole with a land.
    Mount,
}

impl Shape {
    /// The letters of the shape.
    pub fn letters(self) -> &'static str {
        match self {
            Shape::Circle => "c",
            Shape::Square => "s",
            Shape::Rect => "r",
            Shape::Oblong => "b",
            Shape::DShape => "d",
            Shape::User => "u",
            Shape::Via => "v",
            Shape::ViaSquare => "vs",
            Shape::Mount => "w",
        }
    }

    fn two_sizes(self) -> bool {
        matches!(self, Shape::Rect | Shape::Oblong | Shape::DShape)
    }
}

/// The hole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hole {
    /// The diameter, or the length of a slot.
    pub size: i64,
    /// The width of a slot; `None` for a round hole.
    pub slot_width: Option<i64>,
    /// True when the hole is plated.
    pub plated: bool,
}

/// The thermal relief of the land in the planes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thermal {
    /// The inner diameter.
    pub inner: i64,
    /// The outer diameter.
    pub outer: i64,
    /// The width of a spoke.
    pub spoke: i64,
    /// Two spokes instead of four.
    pub two_spokes: bool,
}

/// A padstack as the name says it. Every size is in hundredths of a millimetre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Padstack {
    /// The shape.
    pub shape: Shape,
    /// The first size: the diameter, the side, the length, or the number of an outline.
    pub a: i64,
    /// The second size: the width of a rectangle, an oblong or a D shape, or the inner diameter
    /// of a donut.
    pub b: Option<i64>,
    /// The hole.
    pub hole: Option<Hole>,
    /// The land of the inner layers (`z`).
    pub inner: Option<i64>,
    /// The land of the opposite side (`x`).
    pub opposite: Option<i64>,
    /// The solder mask opening (`m`); 0 for none.
    pub mask: Option<i64>,
    /// The solder mask opening of the opposite side (`mx`); 0 for none.
    pub mask_opposite: Option<i64>,
    /// The paste (`p`).
    pub paste: Option<i64>,
    /// The assembly land (`a`).
    pub assembly: Option<i64>,
    /// The antipad of the planes (`y`).
    pub antipad: Option<i64>,
    /// The thermal relief (`t`).
    pub thermal: Option<Thermal>,
    /// The layers a via joins (`l`), from and to, counted from 1.
    pub span: Option<(u8, u8)>,
}

impl Padstack {
    /// A padstack of a shape and a first size, with nothing else.
    pub fn new(shape: Shape, a: i64) -> Padstack {
        Padstack {
            shape,
            a,
            b: None,
            hole: None,
            inner: None,
            opposite: None,
            mask: None,
            mask_opposite: None,
            paste: None,
            assembly: None,
            antipad: None,
            thermal: None,
            span: None,
        }
    }
}

/// Hundredths of a millimetre from nanometres, rounded half away from zero; `None` for a negative
/// length or one that does not fit.
pub fn units_from_nm(nm: i64) -> Option<i64> {
    if nm < 0 {
        return None;
    }
    let half = UNIT_NM / 2;
    nm.checked_add(half).map(|v| v / UNIT_NM)
}

/// Nanometres from hundredths of a millimetre.
pub fn nm_from_units(units: i64) -> i64 {
    units.saturating_mul(UNIT_NM)
}

/// A size as millimetres with two decimals (`150` is `1.50`).
pub fn mm_text(units: i64) -> String {
    format!("{}.{:02}", units / 100, units % 100)
}

fn push_size(out: &mut String, size: i64) {
    let _ = write!(out, "{size}");
}

impl Padstack {
    /// The canonical name: the shape and its sizes, the hole, then the modifiers in the order
    /// `z`, `x`, `m`, `mx`, `p`, `a`, `y`, `t`, `l`.
    pub fn name(&self) -> String {
        let mut out = String::from(self.shape.letters());
        push_size(&mut out, self.a);
        if let Some(b) = self.b {
            out.push('_');
            push_size(&mut out, b);
        }
        if let Some(hole) = &self.hole {
            out.push_str(if hole.plated { "h" } else { "hn" });
            push_size(&mut out, hole.size);
            if let Some(w) = hole.slot_width {
                out.push('_');
                push_size(&mut out, w);
            }
        }
        for (letters, value) in [
            ("z", self.inner),
            ("x", self.opposite),
            ("m", self.mask),
            ("mx", self.mask_opposite),
            ("p", self.paste),
            ("a", self.assembly),
            ("y", self.antipad),
        ] {
            if let Some(v) = value {
                out.push_str(letters);
                push_size(&mut out, v);
            }
        }
        if let Some(t) = &self.thermal {
            out.push('t');
            for (i, v) in [t.inner, t.outer, t.spoke].into_iter().enumerate() {
                if i > 0 {
                    out.push('_');
                }
                push_size(&mut out, v);
            }
            if t.two_spokes {
                out.push_str("_2");
            }
        }
        if let Some((from, to)) = self.span {
            let _ = write!(out, "l{from}-{to}");
        }
        out
    }
}

/// Why a name could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameError {
    /// The place in the name, counted in bytes from 0.
    pub at: usize,
    /// What was wrong, in English.
    pub what: String,
}

struct Cursor<'a> {
    text: &'a str,
    at: usize,
}

impl Cursor<'_> {
    fn err<T>(&self, what: &str) -> Result<T, NameError> {
        Err(NameError {
            at: self.at,
            what: what.to_owned(),
        })
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn number(&mut self) -> Result<i64, NameError> {
        let start = self.at;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.at += 1;
        }
        let digits = &self.text[start..self.at];
        if digits.is_empty() {
            return self.err("a number is missing");
        }
        if digits.len() > 7 {
            return self.err("a number has more than 7 digits");
        }
        if digits.len() > 1 && digits.starts_with('0') {
            return self.err("a number has a leading zero");
        }
        // at most 7 digits: always fits
        Ok(digits.parse().unwrap_or(0))
    }

    fn pair(&mut self) -> Result<(i64, Option<i64>), NameError> {
        let a = self.number()?;
        if self.eat(b'_') {
            Ok((a, Some(self.number()?)))
        } else {
            Ok((a, None))
        }
    }

    fn letters(&mut self) -> Option<&'static str> {
        const TOKENS: [&str; 11] = ["hn", "mx", "h", "z", "x", "m", "p", "a", "y", "t", "l"];
        let rest = &self.text[self.at..];
        let found = TOKENS.iter().find(|t| rest.starts_with(**t))?;
        self.at += found.len();
        Some(found)
    }
}

impl Padstack {
    /// Reads a name. The modifiers may come in any order; a modifier twice, a number with a
    /// leading zero, a missing size or anything after the last modifier is refused with its place.
    pub fn parse(name: &str) -> Result<Padstack, NameError> {
        let mut c = Cursor { text: name, at: 0 };
        let shape = match c.peek() {
            Some(b'c') => Shape::Circle,
            Some(b's') => Shape::Square,
            Some(b'r') => Shape::Rect,
            Some(b'b') => Shape::Oblong,
            Some(b'd') => Shape::DShape,
            Some(b'u') => Shape::User,
            Some(b'v') => {
                if name.as_bytes().get(1) == Some(&b's') {
                    c.at += 1;
                    Shape::ViaSquare
                } else {
                    Shape::Via
                }
            }
            Some(b'w') => Shape::Mount,
            _ => return c.err("a name starts with the letter of a shape: c s r b d u v vs w"),
        };
        c.at += 1;
        let (a, b) = c.pair()?;
        if shape.two_sizes() && b.is_none() {
            return c.err("this shape needs two sizes joined by _");
        }
        if b.is_some() && !shape.two_sizes() && shape != Shape::Circle {
            return c.err("only a circle (a donut) and the shapes of two sizes take a second size");
        }
        let mut stack = Padstack::new(shape, a);
        stack.b = b;
        let mut seen: Vec<&str> = Vec::new();
        while c.peek().is_some() {
            let before = c.at;
            let Some(token) = c.letters() else {
                return c.err("a modifier is expected: h hn z x m mx p a y t l");
            };
            let plated = token != "hn";
            let key = if plated { token } else { "h" };
            if seen.contains(&key) {
                c.at = before;
                return c.err("a modifier appears twice");
            }
            seen.push(key);
            match key {
                "h" => {
                    let (size, slot) = c.pair()?;
                    stack.hole = Some(Hole {
                        size,
                        slot_width: slot,
                        plated,
                    });
                }
                "z" => stack.inner = Some(c.number()?),
                "x" => stack.opposite = Some(c.number()?),
                "m" => stack.mask = Some(c.number()?),
                "mx" => stack.mask_opposite = Some(c.number()?),
                "p" => stack.paste = Some(c.number()?),
                "a" => stack.assembly = Some(c.number()?),
                "y" => stack.antipad = Some(c.number()?),
                "t" => {
                    let inner = c.number()?;
                    if !c.eat(b'_') {
                        return c.err("a thermal relief is t inner _ outer _ spoke");
                    }
                    let outer = c.number()?;
                    if !c.eat(b'_') {
                        return c.err("a thermal relief is t inner _ outer _ spoke");
                    }
                    let spoke = c.number()?;
                    let mut two_spokes = false;
                    if c.peek() == Some(b'_') {
                        c.at += 1;
                        if c.number()? != 2 {
                            return c.err("after the spoke only _2 (two spokes) may follow");
                        }
                        two_spokes = true;
                    }
                    stack.thermal = Some(Thermal {
                        inner,
                        outer,
                        spoke,
                        two_spokes,
                    });
                }
                _ => {
                    let from = c.number()?;
                    if !c.eat(b'-') {
                        return c.err("the layers of a via are l from - to");
                    }
                    let to = c.number()?;
                    match (u8::try_from(from), u8::try_from(to)) {
                        (Ok(f), Ok(t)) => stack.span = Some((f, t)),
                        _ => return c.err("a layer number is above 255"),
                    }
                }
            }
        }
        Ok(stack)
    }

    /// What cannot be true of the padstack, in English; empty when it can be.
    pub fn validate(&self) -> Vec<String> {
        let mut out = Vec::new();
        let smallest = match (self.shape, self.b) {
            (Shape::Circle, Some(_)) => 0,
            (s, Some(b)) if s.two_sizes() => self.a.min(b),
            _ => self.a,
        };
        if self.shape != Shape::User && self.a == 0 {
            out.push("the size of the land is 0".to_owned());
        }
        if let (Shape::Circle, Some(inner)) = (self.shape, self.b) {
            if inner >= self.a {
                out.push(
                    "the inner diameter of a donut is not below its outer diameter".to_owned(),
                );
            }
            if self.hole.is_some() {
                out.push("a donut has no hole of its own".to_owned());
            }
        }
        if self.shape.two_sizes() && self.b == Some(0) {
            out.push("the width of the land is 0".to_owned());
        }
        if let Some(h) = &self.hole {
            if h.size == 0 {
                out.push("the hole is 0".to_owned());
            }
            if let Some(w) = h.slot_width {
                if w == 0 || w >= h.size {
                    out.push("a slot is longer than it is wide".to_owned());
                }
            }
            let width = h.slot_width.unwrap_or(h.size);
            if smallest > 0 && width >= smallest && self.shape != Shape::User {
                out.push("the hole is not smaller than the land".to_owned());
            }
            if !h.plated && (self.inner.is_some() || self.thermal.is_some()) {
                out.push(
                    "a hole that is not plated has no inner land or thermal relief".to_owned(),
                );
            }
        } else if self.shape == Shape::Mount {
            out.push("a mounting hole needs a hole".to_owned());
        }
        if let (Some(z), Some(h)) = (self.inner, &self.hole) {
            if z <= h.slot_width.unwrap_or(h.size) {
                out.push("the inner land is not larger than the hole".to_owned());
            }
        }
        if let Some(t) = &self.thermal {
            if t.inner >= t.outer {
                out.push(
                    "the inner diameter of the thermal relief is not below the outer".to_owned(),
                );
            }
            if t.spoke == 0 {
                out.push("the spoke of the thermal relief is 0".to_owned());
            }
        }
        if let Some((from, to)) = self.span {
            if !matches!(self.shape, Shape::Via | Shape::ViaSquare) {
                out.push("the layers of a via are only for a via".to_owned());
            }
            if from == 0 || to == 0 || from >= to {
                out.push(
                    "the layers of a via run from a smaller to a larger number from 1".to_owned(),
                );
            }
        }
        out
    }
}

/// A length written in millimetres (`1.6`, `0.15`, at most 6 decimals) as nanometres.
fn mm_to_nm(text: &str) -> Result<i64, String> {
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    let digits = |t: &str| t.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || !digits(whole) || !digits(frac) || frac.len() > 6 || whole.len() > 4 {
        return Err(format!("'{text}' is not a length in millimetres"));
    }
    let whole: i64 = whole
        .parse()
        .map_err(|_| format!("'{text}' is not a length"))?;
    let mut nm = whole * 1_000_000;
    for (i, b) in frac.bytes().enumerate() {
        nm += i64::from(b - b'0') * 10_i64.pow(5 - i as u32);
    }
    Ok(nm)
}

fn units_of(key: &str, text: &str) -> Result<i64, String> {
    let nm = mm_to_nm(text).map_err(|e| format!("{key}: {e}"))?;
    units_from_nm(nm).ok_or_else(|| format!("{key}: the length does not fit"))
}

impl Padstack {
    /// Makes a padstack from `key = value` pairs with the lengths in millimetres: `shape` (the
    /// letters `c s r b d u v vs w`), `size`, `size2`, `hole`, `slot` (the width of a slot, `hole`
    /// being its length), `plated` (`true` or `false`), `inner`, `opposite`, `mask`,
    /// `mask_opposite`, `paste`, `assembly`, `antipad`, `thermal` (`inner/outer/spoke`, and
    /// `/2` for two spokes) and `span` (`1-3`). A key that is not here or a length that is not a
    /// number is refused with the key.
    pub fn from_pairs(pairs: &BTreeMap<String, String>) -> Result<Padstack, String> {
        for key in pairs.keys() {
            const KEYS: [&str; 15] = [
                "shape",
                "size",
                "size2",
                "hole",
                "slot",
                "plated",
                "inner",
                "opposite",
                "mask",
                "mask_opposite",
                "paste",
                "assembly",
                "antipad",
                "thermal",
                "span",
            ];
            if !KEYS.contains(&key.as_str()) {
                return Err(format!("unknown key {key}"));
            }
        }
        let shape = match pairs.get("shape").map(String::as_str) {
            Some("c") => Shape::Circle,
            Some("s") => Shape::Square,
            Some("r") => Shape::Rect,
            Some("b") => Shape::Oblong,
            Some("d") => Shape::DShape,
            Some("u") => Shape::User,
            Some("v") => Shape::Via,
            Some("vs") => Shape::ViaSquare,
            Some("w") => Shape::Mount,
            Some(other) => return Err(format!("shape: '{other}' is not c s r b d u v vs w")),
            None => return Err("shape is missing".to_owned()),
        };
        let size = pairs.get("size").ok_or("size is missing")?;
        let a = if shape == Shape::User {
            size.parse::<i64>()
                .map_err(|_| "size: the number of an outline is a whole number".to_owned())?
        } else {
            units_of("size", size)?
        };
        let mut stack = Padstack::new(shape, a);
        if let Some(v) = pairs.get("size2") {
            stack.b = Some(units_of("size2", v)?);
        }
        if let Some(v) = pairs.get("hole") {
            let plated = match pairs.get("plated").map(String::as_str) {
                None | Some("true") => true,
                Some("false") => false,
                Some(_) => return Err("plated: true or false".to_owned()),
            };
            stack.hole = Some(Hole {
                size: units_of("hole", v)?,
                slot_width: pairs.get("slot").map(|w| units_of("slot", w)).transpose()?,
                plated,
            });
        } else if pairs.contains_key("slot") || pairs.contains_key("plated") {
            return Err("slot and plated need a hole".to_owned());
        }
        for (key, field) in [
            ("inner", &mut stack.inner),
            ("opposite", &mut stack.opposite),
            ("mask", &mut stack.mask),
            ("mask_opposite", &mut stack.mask_opposite),
            ("paste", &mut stack.paste),
            ("assembly", &mut stack.assembly),
            ("antipad", &mut stack.antipad),
        ] {
            if let Some(v) = pairs.get(key) {
                *field = Some(units_of(key, v)?);
            }
        }
        if let Some(v) = pairs.get("thermal") {
            let parts: Vec<&str> = v.split('/').collect();
            let two_spokes = match parts.len() {
                3 => false,
                4 if parts[3] == "2" => true,
                _ => return Err("thermal: inner/outer/spoke, and /2 for two spokes".to_owned()),
            };
            stack.thermal = Some(Thermal {
                inner: units_of("thermal", parts[0])?,
                outer: units_of("thermal", parts[1])?,
                spoke: units_of("thermal", parts[2])?,
                two_spokes,
            });
        }
        if let Some(v) = pairs.get("span") {
            let (from, to) = v.split_once('-').ok_or("span: from-to, like 1-3")?;
            let layer = |t: &str| {
                t.parse::<u8>()
                    .map_err(|_| "span: layer numbers from 1 to 255")
            };
            stack.span = Some((layer(from)?, layer(to)?));
        }
        Ok(stack)
    }
}

/// How serious a finding about a name is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gravity {
    /// The name cannot be.
    Error,
    /// The name is readable but is not the canonical one.
    Warning,
}

/// One finding about a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameIssue {
    /// How serious it is.
    pub gravity: Gravity,
    /// What it is, in English.
    pub detail: String,
}

/// Judges a name: forbidden characters, whether it reads, what the padstack cannot be, and whether
/// the name is the canonical one of the padstack it says.
pub fn check_name(name: &str) -> Vec<NameIssue> {
    let mut out = Vec::new();
    if name.is_empty() || name.len() > 64 {
        out.push(NameIssue {
            gravity: Gravity::Error,
            detail: "a name is 1 to 64 characters".to_owned(),
        });
        return out;
    }
    if let Some(bad) = name
        .chars()
        .find(|c| FORBIDDEN.contains(*c) || !c.is_ascii())
    {
        out.push(NameIssue {
            gravity: Gravity::Error,
            detail: format!(
                "the character '{}' cannot be in a name",
                bad.escape_default()
            ),
        });
        return out;
    }
    match Padstack::parse(name) {
        Err(e) => out.push(NameIssue {
            gravity: Gravity::Error,
            detail: format!("at {}: {}", e.at, e.what),
        }),
        Ok(stack) => {
            for detail in stack.validate() {
                out.push(NameIssue {
                    gravity: Gravity::Error,
                    detail,
                });
            }
            let canonical = stack.name();
            if canonical != name {
                out.push(NameIssue {
                    gravity: Gravity::Warning,
                    detail: format!("the canonical name is {canonical}"),
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
    fn the_examples_of_the_naming_rule_read_and_write_back_the_same() {
        for name in [
            "c150h90z140x170m165mx185a200y300",
            "w700h400z520m720",
            "v30h15l1-3",
            "c130_95",
            "r200_100",
            "s480p4",
            "c150",
            "b300_150h100",
            "d200_100",
            "c160hn80",
            "vs40h20",
            "u12",
            "c200h100t160_240_40",
            "c200h100t160_240_40_2",
            "c150h90m0",
        ] {
            let stack = Padstack::parse(name).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert_eq!(stack.name(), name, "{name}");
        }
    }

    #[test]
    fn the_parts_of_a_long_name_are_what_the_rule_says() {
        let p = Padstack::parse("c150h90z140x170m165mx185a200y300").unwrap();
        assert_eq!((p.shape, p.a), (Shape::Circle, 150));
        assert_eq!(p.hole.map(|h| (h.size, h.plated)), Some((90, true)));
        assert_eq!(
            (
                p.inner,
                p.opposite,
                p.mask,
                p.mask_opposite,
                p.assembly,
                p.antipad
            ),
            (
                Some(140),
                Some(170),
                Some(165),
                Some(185),
                Some(200),
                Some(300)
            )
        );
        let via = Padstack::parse("v30h15l1-3").unwrap();
        assert_eq!((via.shape, via.span), (Shape::Via, Some((1, 3))));
        let donut = Padstack::parse("c130_95").unwrap();
        assert_eq!(donut.b, Some(95));
        let np = Padstack::parse("c160hn80").unwrap();
        assert_eq!(np.hole.map(|h| h.plated), Some(false));
        let slot = Padstack::parse("b300_150h250_100").unwrap();
        assert_eq!(
            slot.hole.map(|h| (h.size, h.slot_width)),
            Some((250, Some(100)))
        );
        let t = Padstack::parse("c200h100t160_240_40_2").unwrap();
        assert_eq!(
            t.thermal.map(|t| (t.inner, t.outer, t.spoke, t.two_spokes)),
            Some((160, 240, 40, true))
        );
    }

    #[test]
    fn the_modifiers_may_come_in_any_order_but_the_canonical_name_is_one() {
        let p = Padstack::parse("c150m165h90z140").unwrap();
        assert_eq!(p.name(), "c150h90z140m165");
        let issues = check_name("c150m165h90z140");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].gravity, Gravity::Warning);
        assert!(issues[0].detail.contains("c150h90z140m165"));
        assert!(check_name("c150h90z140m165").is_empty());
    }

    #[test]
    fn numbers_come_from_nanometres_in_integers_rounded_half_away_from_zero() {
        assert_eq!(units_from_nm(1_500_000), Some(150));
        assert_eq!(units_from_nm(150_000), Some(15));
        assert_eq!(units_from_nm(904_999), Some(90));
        assert_eq!(units_from_nm(905_000), Some(91));
        assert_eq!(units_from_nm(0), Some(0));
        assert_eq!(units_from_nm(-1), None);
        assert_eq!(units_from_nm(i64::MAX), None);
        assert_eq!(nm_from_units(15), 150_000);
        assert_eq!(mm_text(150), "1.50");
        assert_eq!(mm_text(5), "0.05");
        let mut p = Padstack::new(Shape::Circle, units_from_nm(1_600_000).unwrap());
        p.hole = Some(Hole {
            size: units_from_nm(800_000).unwrap(),
            slot_width: None,
            plated: true,
        });
        assert_eq!(p.name(), "c160h80");
    }

    #[test]
    fn a_name_that_cannot_be_read_is_refused_with_its_place() {
        for (name, at, needle) in [
            ("", 0, "1 to 64"),
            ("x150", 0, "starts with the letter"),
            ("c", 1, "a number is missing"),
            ("c015", 4, "leading zero"),
            ("c12345678", 9, "more than 7 digits"),
            ("r200", 4, "two sizes"),
            ("s100_50", 7, "only a circle"),
            ("c150h", 5, "a number is missing"),
            ("c150h90h80", 7, "twice"),
            ("c150q9", 4, "a modifier is expected"),
            ("c150z", 5, "a number is missing"),
            ("c150h90t160_240", 15, "t inner"),
            ("c150h90t160_240_40_3", 20, "only _2"),
            ("v30l1", 5, "from - to"),
            ("v30l1-999", 9, "above 255"),
            ("c150h90 ", 0, "cannot be in a name"),
            ("c1.5", 0, "cannot be in a name"),
            ("c150/h90", 0, "cannot be in a name"),
        ] {
            let issues = check_name(name);
            assert_eq!(issues[0].gravity, Gravity::Error, "{name}");
            assert!(
                issues[0].detail.contains(needle),
                "{name}: {}",
                issues[0].detail
            );
            if !issues[0].detail.contains("cannot be in a name") && !name.is_empty() {
                assert_eq!(Padstack::parse(name).unwrap_err().at, at, "{name}");
            }
        }
    }

    #[test]
    fn what_cannot_be_true_of_a_padstack_is_found() {
        let issues = |name: &str| -> Vec<String> {
            check_name(name).into_iter().map(|i| i.detail).collect()
        };
        assert!(issues("c150h150")[0].contains("not smaller than the land"));
        assert!(issues("r200_100h100")[0].contains("not smaller than the land"));
        assert!(issues("c95_130")[0].contains("donut"));
        assert!(issues("c130_95h50")[0].contains("no hole of its own"));
        assert!(issues("c150hn80z140")[0].contains("not plated"));
        assert!(issues("c150h90z80")[0].contains("not larger than the hole"));
        assert!(issues("w700")[0].contains("needs a hole"));
        assert!(issues("c200h100t240_160_40")[0].contains("thermal"));
        assert!(issues("c200h100t160_240_0")[0].contains("spoke"));
        assert!(issues("c150h90l1-3")[0].contains("only for a via"));
        assert!(issues("v30h15l3-1")[0].contains("run from a smaller"));
        assert!(issues("c0")[0].contains("size of the land is 0"));
        assert!(issues("b300_150h100_150")[0].contains("longer than it is wide"));
        assert!(issues("c150h90").is_empty());
    }

    #[test]
    fn names_with_every_forbidden_character_are_refused() {
        for c in FORBIDDEN.chars() {
            let name = format!("c150{c}");
            let issues = check_name(&name);
            assert!(!issues.is_empty(), "{c:?}");
            assert!(issues[0].detail.contains("cannot be in a name"), "{c:?}");
        }
        assert!(check_name("c150é")[0]
            .detail
            .contains("cannot be in a name"));
        assert!(check_name(&"c".repeat(65))[0].detail.contains("1 to 64"));
    }

    #[test]
    fn names_made_from_random_padstacks_read_back_to_the_same_padstack() {
        let mut state = 0x1234_5678_9abc_def1u64;
        let mut next = move |n: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % n) as i64
        };
        for _ in 0..500 {
            let shape = [
                Shape::Circle,
                Shape::Square,
                Shape::Rect,
                Shape::Oblong,
                Shape::DShape,
                Shape::User,
                Shape::Via,
                Shape::ViaSquare,
                Shape::Mount,
            ][next(9) as usize];
            let mut p = Padstack::new(shape, next(9000));
            if shape.two_sizes() || (shape == Shape::Circle && next(4) == 0) {
                p.b = Some(next(9000));
            }
            if next(2) == 0 {
                p.hole = Some(Hole {
                    size: next(500),
                    slot_width: (next(3) == 0).then(|| next(300)),
                    plated: next(2) == 0,
                });
            }
            p.inner = (next(2) == 0).then(|| next(9000));
            p.opposite = (next(2) == 0).then(|| next(9000));
            p.mask = (next(2) == 0).then(|| next(9000));
            p.mask_opposite = (next(3) == 0).then(|| next(9000));
            p.paste = (next(3) == 0).then(|| next(9000));
            p.assembly = (next(3) == 0).then(|| next(9000));
            p.antipad = (next(3) == 0).then(|| next(9000));
            if next(3) == 0 {
                p.thermal = Some(Thermal {
                    inner: next(500),
                    outer: next(900),
                    spoke: next(100),
                    two_spokes: next(2) == 0,
                });
            }
            if next(3) == 0 {
                p.span = Some((next(250) as u8, next(250) as u8));
            }
            let name = p.name();
            assert_eq!(Padstack::parse(&name).unwrap(), p, "{name}");
            // the canonical name never gets the warning of a different canonical name
            assert!(
                check_name(&name)
                    .iter()
                    .all(|i| i.gravity == Gravity::Error || !i.detail.contains("canonical")),
                "{name}"
            );
        }
    }

    fn pairs(list: &[(&str, &str)]) -> BTreeMap<String, String> {
        list.iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn a_padstack_is_made_from_lengths_in_millimetres() {
        let p = Padstack::from_pairs(&pairs(&[
            ("shape", "c"),
            ("size", "1.5"),
            ("hole", "0.9"),
            ("inner", "1.4"),
            ("opposite", "1.7"),
            ("mask", "1.65"),
            ("mask_opposite", "1.85"),
            ("assembly", "2"),
            ("antipad", "3.0"),
        ]))
        .unwrap();
        assert_eq!(p.name(), "c150h90z140x170m165mx185a200y300");
        let v = Padstack::from_pairs(&pairs(&[
            ("shape", "v"),
            ("size", "0.3"),
            ("hole", "0.15"),
            ("span", "1-3"),
        ]))
        .unwrap();
        assert_eq!(v.name(), "v30h15l1-3");
        let t = Padstack::from_pairs(&pairs(&[
            ("shape", "c"),
            ("size", "2"),
            ("hole", "1"),
            ("plated", "true"),
            ("thermal", "1.6/2.4/0.4/2"),
        ]))
        .unwrap();
        assert_eq!(t.name(), "c200h100t160_240_40_2");
        let r = Padstack::from_pairs(&pairs(&[
            ("shape", "b"),
            ("size", "3"),
            ("size2", "1.5"),
            ("hole", "2.5"),
            ("slot", "1"),
            ("plated", "false"),
        ]))
        .unwrap();
        assert_eq!(r.name(), "b300_150hn250_100");
        // a length with more than two decimals is rounded half away from zero
        let rounded = Padstack::from_pairs(&pairs(&[("shape", "c"), ("size", "1.505")])).unwrap();
        assert_eq!(rounded.name(), "c151");
    }

    #[test]
    fn a_wrong_pair_is_refused_with_its_key() {
        for (list, needle) in [
            (vec![("size", "1")], "shape is missing"),
            (vec![("shape", "c")], "size is missing"),
            (vec![("shape", "z"), ("size", "1")], "shape: 'z'"),
            (vec![("shape", "c"), ("size", "abc")], "size: 'abc'"),
            (vec![("shape", "c"), ("size", "-1")], "size: '-1'"),
            (
                vec![("shape", "c"), ("size", "1.1234567")],
                "size: '1.1234567'",
            ),
            (
                vec![("shape", "c"), ("size", "1"), ("colour", "red")],
                "unknown key colour",
            ),
            (
                vec![("shape", "c"), ("size", "1"), ("slot", "1")],
                "need a hole",
            ),
            (
                vec![
                    ("shape", "c"),
                    ("size", "1"),
                    ("hole", "0.5"),
                    ("plated", "x"),
                ],
                "plated",
            ),
            (
                vec![("shape", "c"), ("size", "2"), ("thermal", "1/2")],
                "thermal:",
            ),
            (vec![("shape", "v"), ("size", "1"), ("span", "13")], "span:"),
            (
                vec![("shape", "v"), ("size", "1"), ("span", "1-300")],
                "span:",
            ),
            (vec![("shape", "u"), ("size", "1.5")], "whole number"),
        ] {
            let e = Padstack::from_pairs(&pairs(&list)).unwrap_err();
            assert!(e.contains(needle), "{list:?}: {e}");
        }
    }

    #[test]
    fn damaged_names_never_panic_with_a_fixed_seed() {
        let alphabet = b"csrbdu vwhnzxmpayt l_-0123456789.";
        let mut state = 0x5eed_5eedu64;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as usize
        };
        for _ in 0..5000 {
            let len = 1 + next() % 24;
            let name: String = (0..len)
                .map(|_| char::from(alphabet[next() % alphabet.len()]))
                .collect();
            let _ = check_name(&name);
        }
    }
}
