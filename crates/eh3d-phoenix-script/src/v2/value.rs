// SPDX-License-Identifier: GPL-3.0-or-later
//! The values of a script of the second version, and the small exact helpers that the language
//! uses: wildcards, an integer square root, a sine in integer arithmetic and the text of a length.
//!
//! Every number is a 64-bit integer. A length is written with a unit (`0.2mm`, `8mil`, `1in`,
//! `25um`) and is the integer count of nanometres; an angle is written with `deg` (`22.5deg`) and
//! is the integer count of micro-degrees. A decimal number without a unit does not exist, so the
//! same script gives the same bytes on every computer.

use eh3d_phoenix_board::Id;

/// The kind of an object that a query finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjKind {
    /// A placed component of a board.
    Component,
    /// A track of a board.
    Track,
    /// A via of a board.
    Via,
    /// A copper zone of a board.
    Zone,
    /// A net of a board or of a schematic.
    Net,
    /// A placed part of a schematic.
    Part,
}

impl ObjKind {
    /// The word a script uses for the kind.
    pub fn word(self) -> &'static str {
        match self {
            ObjKind::Component => "component",
            ObjKind::Track => "track",
            ObjKind::Via => "via",
            ObjKind::Zone => "zone",
            ObjKind::Net => "net",
            ObjKind::Part => "part",
        }
    }

    /// The kind a word names.
    pub fn from_word(word: &str) -> Option<ObjKind> {
        [
            ObjKind::Component,
            ObjKind::Track,
            ObjKind::Via,
            ObjKind::Zone,
            ObjKind::Net,
            ObjKind::Part,
        ]
        .into_iter()
        .find(|k| k.word() == word)
    }
}

/// A reference to an object of the document: its kind and its identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Obj {
    /// What it is.
    pub kind: ObjKind,
    /// Its identity in the document.
    pub id: Id,
}

/// A value of the language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Nothing: what an operation returns when it makes nothing.
    Nil,
    /// True or false.
    Bool(bool),
    /// A 64-bit integer (a count, a length in nanometres or an angle in micro-degrees).
    Int(i64),
    /// A text.
    Str(String),
    /// A list.
    List(Vec<Value>),
    /// An object of the document.
    Obj(Obj),
}

/// The deepest nesting of lists inside lists.
pub const MAX_LIST_DEPTH: usize = 8;

impl Value {
    /// The name of the type, as the messages use it.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Str(_) => "text",
            Value::List(_) => "list",
            Value::Obj(_) => "object",
        }
    }

    /// The memory the value takes, in cells: one for a number, a boolean, an object or nil, one and
    /// one more for every 16 characters of a text, one and the cells of its items for a list.
    pub fn cells(&self) -> usize {
        match self {
            Value::Str(s) => 1 + s.len() / 16,
            Value::List(items) => 1 + items.iter().map(Value::cells).sum::<usize>(),
            _ => 1,
        }
    }

    /// How deep lists are nested in the value; 0 for a value that is not a list.
    pub fn depth(&self) -> usize {
        match self {
            Value::List(items) => 1 + items.iter().map(Value::depth).max().unwrap_or(0),
            _ => 0,
        }
    }
}

/// True when `text` matches `pattern`, where `*` stands for any run of characters (also none) and
/// `?` for exactly one; everything else must be equal, with the case.
pub fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while ti < t.len() {
        if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// The integer square root, rounded down; 0 for a number that is not positive.
pub fn isqrt(n: i128) -> i128 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Divides and rounds half away from zero. The divisor must not be zero.
pub fn round_div(n: i128, d: i128) -> i128 {
    let half = d.abs() / 2;
    if (n >= 0) == (d > 0) {
        (n.abs() + half) / d.abs()
    } else {
        -((n.abs() + half) / d.abs())
    }
}

const PI_SCALED: i128 = 3_141_592_653_589_793_238;
const SCALE: i128 = 1_000_000_000_000_000_000;

/// The sine of an angle in micro-degrees, in millionths (so 1 is 1000000), by a series in integer
/// arithmetic: the same on every computer.
pub fn sin_ppm(angle_udeg: i64) -> i64 {
    let mut a = i128::from(angle_udeg).rem_euclid(360_000_000);
    let mut sign = 1i128;
    if a >= 180_000_000 {
        sign = -1;
        a -= 180_000_000;
    }
    if a > 90_000_000 {
        a = 180_000_000 - a;
    }
    let x = a * PI_SCALED / 180_000_000;
    let mut term = x;
    let mut sum = x;
    for k in 1..=10i128 {
        term = -term * x / SCALE * x / SCALE / (2 * k * (2 * k + 1));
        sum += term;
    }
    let ppm = (sum + 500_000_000_000) / 1_000_000_000_000;
    let ppm = ppm.clamp(0, 1_000_000);
    i64::try_from(sign * ppm).unwrap_or(0)
}

/// The cosine of an angle in micro-degrees, in millionths.
pub fn cos_ppm(angle_udeg: i64) -> i64 {
    sin_ppm(angle_udeg.saturating_add(90_000_000))
}

/// The length in nanometres as a text in millimetres with up to six decimals and no trailing zeros.
pub fn mm_text(nm: i64) -> String {
    let abs = nm.unsigned_abs();
    let (whole, frac) = (abs / 1_000_000, abs % 1_000_000);
    let mut text = String::new();
    if nm < 0 {
        text.push('-');
    }
    text.push_str(&whole.to_string());
    if frac != 0 {
        let digits = format!("{frac:06}");
        text.push('.');
        text.push_str(digits.trim_end_matches('0'));
    }
    text
}

/// The key that sorts reference designators the way a person does: the letters first and then the
/// number (`C2` before `C10`).
pub fn natural_key(name: &str) -> (String, u64, String) {
    let split = name
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_ascii_digit())
        .map_or(0, |(i, c)| i + c.len_utf8());
    let (prefix, digits) = name.split_at(split);
    let number = if digits.is_empty() || digits.len() > 18 {
        0
    } else {
        digits.parse::<u64>().unwrap_or(0)
    };
    (prefix.to_owned(), number, name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards_match_runs_and_single_characters() {
        assert!(glob("C*", "C12"));
        assert!(glob("C*", "C"));
        assert!(glob("*", ""));
        assert!(glob("R?", "R1"));
        assert!(!glob("R?", "R12"));
        assert!(glob("*1", "R11"));
        assert!(glob("a*b*c", "aXXbYYc"));
        assert!(!glob("a*b*c", "aXXbYY"));
        assert!(!glob("c*", "C1"));
        assert!(glob("U1", "U1"));
        assert!(!glob("U1", "U10"));
    }

    #[test]
    fn the_square_root_is_exact_for_squares_and_rounds_down() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(-5), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(2), 1);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(1_000_000_000_000), 1_000_000);
        assert_eq!(isqrt(1_000_000_000_001), 1_000_000);
    }

    #[test]
    fn division_rounds_half_away_from_zero() {
        assert_eq!(round_div(7, 2), 4);
        assert_eq!(round_div(-7, 2), -4);
        assert_eq!(round_div(5, 3), 2);
        assert_eq!(round_div(-5, 3), -2);
        assert_eq!(round_div(6, 3), 2);
        assert_eq!(round_div(7, -2), -4);
    }

    #[test]
    fn the_sine_matches_the_known_angles() {
        assert_eq!(sin_ppm(0), 0);
        assert_eq!(sin_ppm(90_000_000), 1_000_000);
        assert_eq!(sin_ppm(180_000_000), 0);
        assert_eq!(sin_ppm(270_000_000), -1_000_000);
        assert_eq!(sin_ppm(30_000_000), 500_000);
        assert_eq!(sin_ppm(-30_000_000), -500_000);
        assert_eq!(sin_ppm(150_000_000), 500_000);
        assert_eq!(cos_ppm(0), 1_000_000);
        assert_eq!(cos_ppm(60_000_000), 500_000);
        assert_eq!(cos_ppm(180_000_000), -1_000_000);
        assert_eq!(sin_ppm(45_000_000), 707_107);
        assert_eq!(sin_ppm(360_000_000), 0);
    }

    #[test]
    fn a_length_is_written_in_millimetres_without_trailing_zeros() {
        assert_eq!(mm_text(0), "0");
        assert_eq!(mm_text(1_000_000), "1");
        assert_eq!(mm_text(1_270_000), "1.27");
        assert_eq!(mm_text(-250_000), "-0.25");
        assert_eq!(mm_text(1), "0.000001");
        assert_eq!(mm_text(-12_000_000), "-12");
    }

    #[test]
    fn designators_sort_by_letters_and_then_by_number() {
        let mut names = vec!["C10", "C2", "R1", "C1", "U1"];
        names.sort_by_key(|n| natural_key(n));
        assert_eq!(names, vec!["C1", "C2", "C10", "R1", "U1"]);
    }

    #[test]
    fn a_value_knows_its_cells_depth_and_type() {
        let v = Value::List(vec![
            Value::Int(1),
            Value::Str("x".repeat(32)),
            Value::List(vec![Value::Nil]),
        ]);
        assert_eq!(v.cells(), 1 + 1 + 3 + 2);
        assert_eq!(v.depth(), 2);
        assert_eq!(v.type_name(), "list");
        assert_eq!(Value::Bool(true).type_name(), "bool");
        assert_eq!(Value::Int(0).depth(), 0);
        assert_eq!(ObjKind::from_word("via"), Some(ObjKind::Via));
        assert_eq!(ObjKind::from_word("pad"), None);
        assert_eq!(ObjKind::Part.word(), "part");
    }
}
