// SPDX-License-Identifier: GPL-3.0-or-later
//! The built-in functions that the evaluator runs: **pure** functions of their arguments (mathematics,
//! the conversion of units, text, the parts of a file name) and the formatting of `printf` and
//! `sprintf`. None of them reads or writes anything outside the run: no file, no clock, no
//! environment. The functions that need the variables of the run (`sprintf` writes its first
//! argument, `strsplit` fills an array) are in the evaluator.
//!
//! The angles of the trigonometric functions are radians. A length is a nanometre, so `u2mm` divides
//! by a million. The sine, cosine, tangent, exponential and logarithm come from the mathematical
//! library of the machine, so their last digit can differ between machines; the other functions are
//! exact. A result that is not a finite number is an error, never a value.

use crate::codes::{RUN_ARGUMENTS, RUN_FORMAT, RUN_RANGE, RUN_TYPE};
use crate::value::Value;
use crate::world::Fault;

/// The names of the pure functions.
pub const PURE: &[&str] = &[
    "abs",
    "acos",
    "asin",
    "atan",
    "ceil",
    "cos",
    "exp",
    "floor",
    "frac",
    "log",
    "log10",
    "max",
    "min",
    "pow",
    "round",
    "trunc",
    "sin",
    "sqrt",
    "tan",
    "u2inch",
    "u2mic",
    "u2mil",
    "u2mm",
    "inch2u",
    "mic2u",
    "mil2u",
    "mm2u",
    "strchr",
    "strjoin",
    "int",
    "real",
    "char",
    "string",
    "isalnum",
    "isalpha",
    "isdigit",
    "isspace",
    "isupper",
    "islower",
    "ispunct",
    "isxdigit",
    "isgraph",
    "isprint",
    "iscntrl",
    "tolower",
    "toupper",
    "lookup",
    "strlen",
    "strlwr",
    "strrchr",
    "strrstr",
    "strstr",
    "strsub",
    "strtod",
    "strtol",
    "strupr",
    "filedir",
    "fileext",
    "filename",
    "filesetext",
];

/// The names of the functions that the evaluator runs and that need the variables of the run.
pub const WITH_VARIABLES: &[&str] = &["sprintf", "strsplit", "strxstr"];

fn bad(what: impl Into<String>) -> Fault {
    (RUN_ARGUMENTS, what.into())
}

fn arity(name: &str, args: &[Value], min: usize, max: usize) -> Result<(), Fault> {
    if args.len() < min || args.len() > max {
        return Err(bad(format!(
            "{name} takes {}{} arguments, not {}",
            min,
            if max == min {
                String::new()
            } else {
                format!(" to {max}")
            },
            args.len()
        )));
    }
    Ok(())
}

fn num(v: &Value) -> Result<f64, Fault> {
    match v {
        Value::Int(i) => Ok(*i as f64),
        Value::Real(r) => Ok(*r),
        other => Err((
            RUN_TYPE,
            format!("a number is needed, not a {}", other.type_name()),
        )),
    }
}

fn int(v: &Value) -> Result<i64, Fault> {
    match v {
        Value::Int(i) => Ok(*i),
        other => Err((
            RUN_TYPE,
            format!("a whole number is needed, not a {}", other.type_name()),
        )),
    }
}

fn string(v: &Value) -> Result<&str, Fault> {
    match v {
        Value::Str(s) => Ok(s),
        other => Err((
            RUN_TYPE,
            format!("a text is needed, not a {}", other.type_name()),
        )),
    }
}

/// A real result: finite or an error.
pub fn real(x: f64) -> Result<Value, Fault> {
    if x.is_finite() {
        Ok(Value::Real(if x == 0.0 { 0.0 } else { x }))
    } else {
        Err((RUN_RANGE, "the result is not a finite number".to_owned()))
    }
}

/// A whole number from a real one, rounded as the caller chose.
fn to_int(x: f64) -> Result<Value, Fault> {
    if x.is_finite() && x.abs() < 9.2e18 {
        Ok(Value::Int(x as i64))
    } else {
        Err((
            RUN_RANGE,
            "the result is out of the range of int".to_owned(),
        ))
    }
}

/// The position of a byte index as a position in characters.
fn char_index(s: &str, byte: usize) -> i64 {
    s[..byte].chars().count() as i64
}

fn find_char(s: &str, c: &Value, last: bool) -> Result<Value, Fault> {
    let wanted = match c {
        Value::Int(code) => u32::try_from(*code).ok().and_then(char::from_u32),
        Value::Str(t) => t.chars().next(),
        other => {
            return Err((
                RUN_TYPE,
                format!("a character is needed, not a {}", other.type_name()),
            ))
        }
    };
    let found = wanted.and_then(|w| if last { s.rfind(w) } else { s.find(w) });
    Ok(Value::Int(found.map_or(-1, |b| char_index(s, b))))
}

/// The longest start of a text that is a whole number, with its sign.
fn number_prefix(s: &str, fraction: bool) -> &str {
    let t = s.trim_start();
    let b = t.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let d = digits(i);
    i += d;
    let mut any = d > 0;
    if fraction {
        if i < b.len() && b[i] == b'.' {
            let f = digits(i + 1);
            if f > 0 || any {
                i += 1 + f;
                any = any || f > 0;
            }
        }
        if any && i < b.len() && (b[i] == b'e' || b[i] == b'E') {
            let mut j = i + 1;
            if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                j += 1;
            }
            let e = digits(j);
            if e > 0 {
                i = j + e;
            }
        }
    }
    if any {
        &t[..i]
    } else {
        ""
    }
}

fn split_name(path: &str) -> (&str, &str) {
    match path.rfind(['/', '\\']) {
        Some(i) => (&path[..i], &path[i + 1..]),
        None => ("", path),
    }
}

fn extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 => &name[i..],
        _ => "",
    }
}

/// Runs a pure function. `Ok(None)` when the name is not one of them.
pub fn pure(name: &str, args: &[Value]) -> Result<Option<Value>, Fault> {
    let one = |args: &[Value]| -> Result<f64, Fault> {
        arity(name, args, 1, 1)?;
        num(&args[0])
    };
    let value = match name {
        "abs" => {
            arity(name, args, 1, 1)?;
            match &args[0] {
                Value::Int(i) => Value::Int(i.checked_abs().ok_or((
                    RUN_RANGE,
                    "the result is out of the range of int".to_owned(),
                ))?),
                other => real(num(other)?.abs())?,
            }
        }
        "acos" | "asin" => {
            let x = one(args)?;
            if !(-1.0..=1.0).contains(&x) {
                return Err((RUN_RANGE, format!("{name} of a number outside -1 to 1")));
            }
            real(if name == "acos" { x.acos() } else { x.asin() })?
        }
        "atan" => real(one(args)?.atan())?,
        "int" | "char" => {
            arity(name, args, 1, 1)?;
            match &args[0] {
                Value::Int(i) => Value::Int(*i),
                other => to_int(num(other)?.trunc())?,
            }
        }
        "real" => {
            arity(name, args, 1, 1)?;
            real(num(&args[0])?)?
        }
        "string" => {
            arity(name, args, 1, 1)?;
            match &args[0] {
                Value::Str(t) => Value::Str(t.clone()),
                Value::Int(i) => Value::Str(i.to_string()),
                Value::Real(r) => Value::Str(r.to_string()),
                other => {
                    return Err((
                        RUN_TYPE,
                        format!("a text cannot be made from a {}", other.type_name()),
                    ))
                }
            }
        }
        "sin" => real(one(args)?.sin())?,
        "cos" => real(one(args)?.cos())?,
        "tan" => real(one(args)?.tan())?,
        "exp" => real(one(args)?.exp())?,
        "ceil" => to_int(one(args)?.ceil())?,
        "floor" => to_int(one(args)?.floor())?,
        "round" => to_int(one(args)?.round())?,
        "trunc" => to_int(one(args)?.trunc())?,
        "frac" => real(one(args)?.fract())?,
        "log" | "log10" => {
            let x = one(args)?;
            if x <= 0.0 {
                return Err((
                    RUN_RANGE,
                    format!("{name} of a number that is not positive"),
                ));
            }
            real(if name == "log" { x.ln() } else { x.log10() })?
        }
        "sqrt" => {
            let x = one(args)?;
            if x < 0.0 {
                return Err((RUN_RANGE, "sqrt of a negative number".to_owned()));
            }
            real(x.sqrt())?
        }
        "pow" => {
            arity(name, args, 2, 2)?;
            real(num(&args[0])?.powf(num(&args[1])?))?
        }
        "max" | "min" => {
            arity(name, args, 2, 2)?;
            match (&args[0], &args[1]) {
                (Value::Int(a), Value::Int(b)) => {
                    Value::Int(if name == "max" { *a.max(b) } else { *a.min(b) })
                }
                (a, b) => {
                    let (a, b) = (num(a)?, num(b)?);
                    real(if name == "max" { a.max(b) } else { a.min(b) })?
                }
            }
        }
        "u2mm" => real(one(args)? / 1e6)?,
        "u2mic" => real(one(args)? / 1e3)?,
        "u2mil" => real(one(args)? / 25_400.0)?,
        "u2inch" => real(one(args)? / 25_400_000.0)?,
        "mm2u" => to_int((one(args)? * 1e6).round())?,
        "mic2u" => to_int((one(args)? * 1e3).round())?,
        "mil2u" => to_int((one(args)? * 25_400.0).round())?,
        "inch2u" => to_int((one(args)? * 25_400_000.0).round())?,
        "strlen" => {
            arity(name, args, 1, 1)?;
            Value::Int(string(&args[0])?.chars().count() as i64)
        }
        "strlwr" => {
            arity(name, args, 1, 1)?;
            Value::Str(string(&args[0])?.to_lowercase())
        }
        "isalnum" | "isalpha" | "isdigit" | "isspace" | "isupper" | "islower" | "ispunct"
        | "isxdigit" | "isgraph" | "isprint" | "iscntrl" | "tolower" | "toupper" => {
            arity(name, args, 1, 1)?;
            let code = int(&args[0])?;
            let c = u32::try_from(code)
                .ok()
                .and_then(char::from_u32)
                .unwrap_or(char::from(0u8));
            let yes = |b: bool| Value::Int(i64::from(b));
            match name {
                "isalnum" => yes(c.is_ascii_alphanumeric()),
                "isalpha" => yes(c.is_ascii_alphabetic()),
                "isdigit" => yes(c.is_ascii_digit()),
                "isspace" => yes(c.is_ascii_whitespace() || c == char::from(11u8)),
                "isupper" => yes(c.is_ascii_uppercase()),
                "islower" => yes(c.is_ascii_lowercase()),
                "ispunct" => yes(c.is_ascii_punctuation()),
                "isxdigit" => yes(c.is_ascii_hexdigit()),
                "isgraph" => yes(c.is_ascii_graphic()),
                "isprint" => yes(c.is_ascii_graphic() || c == ' '),
                "iscntrl" => yes(c.is_ascii_control()),
                "tolower" => Value::Int(i64::from(c.to_ascii_lowercase() as u32)),
                _ => Value::Int(i64::from(c.to_ascii_uppercase() as u32)),
            }
        }
        "lookup" => {
            arity(name, args, 3, 4)?;
            let Value::Array(lines) = &args[0] else {
                return Err((RUN_TYPE, "lookup searches an array of texts".to_owned()));
            };
            let key = string(&args[1])?;
            let field = usize::try_from(int(&args[2])?)
                .map_err(|_| (RUN_RANGE, "the field is negative".to_owned()))?;
            let sep = if args.len() == 4 {
                string(&args[3])?.to_owned()
            } else {
                "\t".to_owned()
            };
            let mut found = String::new();
            for line in lines {
                let Value::Str(line) = line else { continue };
                let mut parts = line.split(sep.as_str());
                if parts.next() == Some(key) {
                    found = line
                        .split(sep.as_str())
                        .nth(field)
                        .unwrap_or_default()
                        .to_owned();
                    break;
                }
            }
            Value::Str(found)
        }
        "strupr" => {
            arity(name, args, 1, 1)?;
            Value::Str(string(&args[0])?.to_uppercase())
        }
        "strchr" | "strrchr" => {
            arity(name, args, 2, 2)?;
            find_char(string(&args[0])?, &args[1], name == "strrchr")?
        }
        "strstr" | "strrstr" => {
            arity(name, args, 2, 2)?;
            let (s, t) = (string(&args[0])?, string(&args[1])?);
            let found = if name == "strstr" {
                s.find(t)
            } else {
                s.rfind(t)
            };
            Value::Int(found.map_or(-1, |b| char_index(s, b)))
        }
        "strsub" => {
            arity(name, args, 2, 3)?;
            let s = string(&args[0])?;
            let start = int(&args[1])?;
            if start < 0 {
                return Err((RUN_RANGE, "strsub starts before the text".to_owned()));
            }
            let count = match args.get(2) {
                Some(n) => {
                    let n = int(n)?;
                    if n < 0 {
                        return Err((RUN_RANGE, "strsub takes a negative length".to_owned()));
                    }
                    usize::try_from(n).unwrap_or(usize::MAX)
                }
                None => usize::MAX,
            };
            let skip = usize::try_from(start).unwrap_or(usize::MAX);
            Value::Str(s.chars().skip(skip).take(count).collect())
        }
        "strjoin" => {
            arity(name, args, 2, 2)?;
            let Value::Array(items) = &args[0] else {
                return Err((RUN_TYPE, "strjoin needs an array of texts".to_owned()));
            };
            let sep = string(&args[1])?;
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                parts.push(string(item)?);
            }
            Value::Str(parts.join(sep))
        }
        "strtod" => {
            arity(name, args, 1, 1)?;
            let prefix = number_prefix(string(&args[0])?, true);
            real(prefix.parse::<f64>().unwrap_or(0.0))?
        }
        "strtol" => {
            arity(name, args, 1, 1)?;
            let prefix = number_prefix(string(&args[0])?, false);
            if prefix.is_empty() {
                Value::Int(0)
            } else {
                Value::Int(prefix.parse::<i64>().map_err(|_| {
                    (
                        RUN_RANGE,
                        "the number is out of the range of int".to_owned(),
                    )
                })?)
            }
        }
        "filename" => {
            arity(name, args, 1, 1)?;
            Value::Str(split_name(string(&args[0])?).1.to_owned())
        }
        "filedir" => {
            arity(name, args, 1, 1)?;
            Value::Str(split_name(string(&args[0])?).0.to_owned())
        }
        "fileext" => {
            arity(name, args, 1, 1)?;
            Value::Str(extension(split_name(string(&args[0])?).1).to_owned())
        }
        "filesetext" => {
            arity(name, args, 2, 2)?;
            let path = string(&args[0])?;
            let file = split_name(path).1;
            let stem = &file[..file.len() - extension(file).len()];
            let ext = string(&args[1])?;
            let head = &path[..path.len() - file.len()];
            Value::Str(format!("{head}{stem}{ext}"))
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn format_error(what: impl Into<String>) -> Fault {
    (RUN_FORMAT, what.into())
}

struct Spec {
    left: bool,
    plus: bool,
    space: bool,
    zero: bool,
    alt: bool,
    width: usize,
    precision: Option<usize>,
}

fn pad(body: String, spec: &Spec, numeric: bool) -> String {
    let len = body.chars().count();
    if len >= spec.width {
        return body;
    }
    let fill = spec.width - len;
    if spec.left {
        return format!("{body}{}", " ".repeat(fill));
    }
    if spec.zero && numeric {
        let split = body
            .find(|c: char| c.is_ascii_digit())
            .map(|i| {
                if body[i..].starts_with("0x") || body[i..].starts_with("0X") {
                    i + 2
                } else {
                    i
                }
            })
            .unwrap_or(0);
        return format!("{}{}{}", &body[..split], "0".repeat(fill), &body[split..]);
    }
    format!("{}{body}", " ".repeat(fill))
}

fn sign(negative: bool, spec: &Spec) -> &'static str {
    if negative {
        "-"
    } else if spec.plus {
        "+"
    } else if spec.space {
        " "
    } else {
        ""
    }
}

/// `%e` the way the C library writes it: a mantissa and an exponent of at least two digits.
fn exponent_form(v: f64, precision: usize, upper: bool) -> String {
    let raw = format!("{v:.precision$e}");
    let (mantissa, exp) = raw.split_once('e').unwrap_or((&raw, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let text = format!(
        "{mantissa}e{}{:02}",
        if exp < 0 { '-' } else { '+' },
        exp.unsigned_abs()
    );
    if upper {
        text.to_uppercase()
    } else {
        text
    }
}

fn general_form(v: f64, precision: usize, upper: bool, alt: bool) -> String {
    let p = if precision == 0 { 1 } else { precision };
    let exp = if v == 0.0 {
        0
    } else {
        let raw = format!("{v:.prec$e}", prec = p - 1);
        raw.split_once('e')
            .and_then(|(_, e)| e.parse::<i32>().ok())
            .unwrap_or(0)
    };
    let mut text = if (-4..p as i32).contains(&exp) {
        format!("{v:.prec$}", prec = (p as i32 - 1 - exp).max(0) as usize)
    } else {
        exponent_form(v, p - 1, upper)
    };
    if !alt {
        let (head, tail) = match text.find(['e', 'E']) {
            Some(i) => (text[..i].to_owned(), text[i..].to_owned()),
            None => (text.clone(), String::new()),
        };
        let head = if head.contains('.') {
            head.trim_end_matches('0').trim_end_matches('.').to_owned()
        } else {
            head
        };
        text = format!("{head}{tail}");
    }
    text
}

/// Formats like the `printf` of C for the conversions `d i u c s f F e E g G x X o` and `%%`, with
/// the flags `- + space 0 #`, a width and a precision.
pub fn format_c(format: &str, args: &[Value]) -> Result<String, Fault> {
    let mut out = String::new();
    let mut chars = format.chars().peekable();
    let mut next_arg = 0usize;
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut spec = Spec {
            left: false,
            plus: false,
            space: false,
            zero: false,
            alt: false,
            width: 0,
            precision: None,
        };
        while let Some(&f) = chars.peek() {
            match f {
                '-' => spec.left = true,
                '+' => spec.plus = true,
                ' ' => spec.space = true,
                '0' => spec.zero = true,
                '#' => spec.alt = true,
                _ => break,
            }
            chars.next();
        }
        while let Some(d) = chars.peek().and_then(|c| c.to_digit(10)) {
            spec.width = spec
                .width
                .checked_mul(10)
                .and_then(|w| w.checked_add(d as usize))
                .filter(|w| *w <= 4096)
                .ok_or_else(|| format_error("a width larger than 4096"))?;
            chars.next();
        }
        if chars.peek() == Some(&'.') {
            chars.next();
            let mut p = 0usize;
            while let Some(d) = chars.peek().and_then(|c| c.to_digit(10)) {
                p = p
                    .checked_mul(10)
                    .and_then(|w| w.checked_add(d as usize))
                    .filter(|w| *w <= 4096)
                    .ok_or_else(|| format_error("a precision larger than 4096"))?;
                chars.next();
            }
            spec.precision = Some(p);
        }
        while matches!(chars.peek(), Some('l' | 'h' | 'L' | 'z' | 'j' | 't')) {
            chars.next();
        }
        let Some(conv) = chars.next() else {
            return Err(format_error("a % at the end of the format"));
        };
        if conv == '%' {
            out.push('%');
            continue;
        }
        let arg = args
            .get(next_arg)
            .ok_or_else(|| format_error("the format asks for more arguments than were given"))?;
        next_arg += 1;
        match conv {
            'd' | 'i' => {
                let Value::Int(v) = arg else {
                    return Err(format_error(format!("%{conv} needs a whole number")));
                };
                let mut digits = v.unsigned_abs().to_string();
                if let Some(p) = spec.precision {
                    if digits.len() < p {
                        digits = format!("{}{digits}", "0".repeat(p - digits.len()));
                    }
                    spec.zero = false;
                }
                out.push_str(&pad(
                    format!("{}{digits}", sign(*v < 0, &spec)),
                    &spec,
                    true,
                ));
            }
            'u' | 'x' | 'X' | 'o' => {
                let Value::Int(v) = arg else {
                    return Err(format_error(format!("%{conv} needs a whole number")));
                };
                let u = *v as u64;
                let mut digits = match conv {
                    'u' => u.to_string(),
                    'x' => format!("{u:x}"),
                    'X' => format!("{u:X}"),
                    _ => format!("{u:o}"),
                };
                if let Some(p) = spec.precision {
                    if digits.len() < p {
                        digits = format!("{}{digits}", "0".repeat(p - digits.len()));
                    }
                    spec.zero = false;
                }
                if spec.alt && u != 0 {
                    digits = match conv {
                        'x' => format!("0x{digits}"),
                        'X' => format!("0X{digits}"),
                        'o' => format!("0{digits}"),
                        _ => digits,
                    };
                }
                out.push_str(&pad(digits, &spec, true));
            }
            'c' => {
                let ch = match arg {
                    Value::Int(code) => u32::try_from(*code).ok().and_then(char::from_u32),
                    _ => None,
                }
                .ok_or_else(|| format_error("%c needs the code of a character"))?;
                out.push_str(&pad(ch.to_string(), &spec, false));
            }
            's' => {
                let Value::Str(s) = arg else {
                    return Err(format_error("%s needs a text"));
                };
                let shown: String = match spec.precision {
                    Some(p) => s.chars().take(p).collect(),
                    None => s.clone(),
                };
                out.push_str(&pad(shown, &spec, false));
            }
            'f' | 'F' | 'e' | 'E' | 'g' | 'G' => {
                let v = match arg {
                    Value::Int(i) => *i as f64,
                    Value::Real(r) => *r,
                    _ => return Err(format_error(format!("%{conv} needs a number"))),
                };
                let p = spec.precision.unwrap_or(6);
                let body = match conv {
                    'f' | 'F' => format!("{:.p$}", v.abs()),
                    'e' | 'E' => exponent_form(v.abs(), p, conv == 'E'),
                    _ => general_form(v.abs(), p, conv == 'G', spec.alt),
                };
                let negative = v < 0.0;
                out.push_str(&pad(
                    format!("{}{body}", sign(negative, &spec)),
                    &spec,
                    true,
                ));
            }
            other => {
                return Err(format_error(format!(
                    "the conversion %{other} is not known"
                )));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(v: i64) -> Value {
        Value::Int(v)
    }
    fn st(s: &str) -> Value {
        Value::Str(s.to_owned())
    }
    fn run(name: &str, args: &[Value]) -> Value {
        pure(name, args).unwrap().unwrap()
    }

    #[test]
    fn the_mathematics_is_exact_where_it_can_be() {
        assert_eq!(run("abs", &[int(-3)]), int(3));
        assert_eq!(run("max", &[int(2), int(9)]), int(9));
        assert_eq!(run("min", &[Value::Real(2.5), int(9)]), Value::Real(2.5));
        assert_eq!(run("pow", &[int(2), int(10)]), Value::Real(1024.0));
        assert_eq!(run("sqrt", &[int(144)]), Value::Real(12.0));
        assert_eq!(run("round", &[Value::Real(2.5)]), int(3));
        assert_eq!(run("trunc", &[Value::Real(-2.7)]), int(-2));
        assert_eq!(run("trunc", &[Value::Real(2.7)]), int(2));
        assert_eq!(run("isgraph", &[int(65)]), int(1));
        assert_eq!(run("isgraph", &[int(32)]), int(0));
        assert_eq!(run("isprint", &[int(32)]), int(1));
        assert_eq!(run("iscntrl", &[int(10)]), int(1));
        assert_eq!(run("round", &[Value::Real(-2.5)]), int(-3));
        assert_eq!(run("floor", &[Value::Real(-0.5)]), int(-1));
        assert_eq!(run("ceil", &[Value::Real(0.2)]), int(1));
        assert_eq!(run("frac", &[Value::Real(2.75)]), Value::Real(0.75));
        assert_eq!(run("sin", &[int(0)]), Value::Real(0.0));
        assert_eq!(run("cos", &[int(0)]), Value::Real(1.0));
    }

    #[test]
    fn a_result_that_is_not_a_number_is_an_error() {
        for (name, args) in [
            ("sqrt", vec![int(-1)]),
            ("log", vec![int(0)]),
            ("log10", vec![int(-5)]),
            ("acos", vec![int(2)]),
            ("exp", vec![int(10_000)]),
            ("abs", vec![int(i64::MIN)]),
            ("round", vec![Value::Real(1e30)]),
        ] {
            assert_eq!(pure(name, &args).unwrap_err().0, RUN_RANGE, "{name}");
        }
        assert_eq!(pure("sqrt", &[]).unwrap_err().0, RUN_ARGUMENTS);
        assert_eq!(pure("sqrt", &[st("x")]).unwrap_err().0, RUN_TYPE);
        assert_eq!(pure("nothing", &[]).unwrap(), None);
    }

    #[test]
    fn units_convert_both_ways() {
        assert_eq!(run("u2mm", &[int(2_500_000)]), Value::Real(2.5));
        assert_eq!(run("mm2u", &[Value::Real(2.5)]), int(2_500_000));
        assert_eq!(run("inch2u", &[Value::Real(1.0)]), int(25_400_000));
        assert_eq!(run("u2mil", &[int(25_400)]), Value::Real(1.0));
        assert_eq!(run("u2mic", &[int(1500)]), Value::Real(1.5));
        assert_eq!(run("mil2u", &[int(10)]), int(254_000));
        assert_eq!(run("mic2u", &[int(7)]), int(7000));
        assert_eq!(run("u2inch", &[int(50_800_000)]), Value::Real(2.0));
    }

    #[test]
    fn text_functions_count_characters() {
        assert_eq!(run("strlen", &[st("añb")]), int(3));
        assert_eq!(run("strupr", &[st("ab")]), st("AB"));
        assert_eq!(run("strstr", &[st("añbcb"), st("b")]), int(2));
        assert_eq!(run("strrstr", &[st("añbcb"), st("b")]), int(4));
        assert_eq!(run("strstr", &[st("abc"), st("z")]), int(-1));
        assert_eq!(run("strchr", &[st("abca"), int(i64::from(b'c'))]), int(2));
        assert_eq!(run("strrchr", &[st("abca"), st("a")]), int(3));
        assert_eq!(run("strsub", &[st("abcdef"), int(2), int(3)]), st("cde"));
        assert_eq!(run("strsub", &[st("abcdef"), int(4)]), st("ef"));
        assert_eq!(run("strsub", &[st("abc"), int(9)]), st(""));
        assert_eq!(
            run(
                "strjoin",
                &[Value::Array(vec![st("a"), st("b"), st("c")]), st(", ")]
            ),
            st("a, b, c")
        );
        assert_eq!(run("strtol", &[st("  -42abc")]), int(-42));
        assert_eq!(run("strtol", &[st("abc")]), int(0));
        assert_eq!(run("strtod", &[st("3.5e2x")]), Value::Real(350.0));
        assert_eq!(run("strtod", &[st(".5")]), Value::Real(0.5));
        assert_eq!(run("strtod", &[st("-")]), Value::Real(0.0));
    }

    #[test]
    fn the_parts_of_a_file_name_are_text_operations() {
        assert_eq!(run("filename", &[st("a/b\\c.txt")]), st("c.txt"));
        assert_eq!(run("filedir", &[st("a/b/c.txt")]), st("a/b"));
        assert_eq!(run("fileext", &[st("a/b/c.tar.gz")]), st(".gz"));
        assert_eq!(run("fileext", &[st("a.d/c")]), st(""));
        assert_eq!(
            run("filesetext", &[st("a/b/c.txt"), st(".csv")]),
            st("a/b/c.csv")
        );
        assert_eq!(run("filesetext", &[st("c"), st(".csv")]), st("c.csv"));
    }

    fn fmt(f: &str, args: &[Value]) -> String {
        format_c(f, args).unwrap()
    }

    #[test]
    fn printf_formats_like_c() {
        assert_eq!(
            fmt(
                "%d|%5d|%-5d|%05d|%+d",
                &[int(7), int(7), int(7), int(-7), int(7)]
            ),
            "7|    7|7    |-0007|+7"
        );
        assert_eq!(
            fmt("%x %X %o %#x", &[int(255), int(255), int(8), int(255)]),
            "ff FF 10 0xff"
        );
        assert_eq!(
            fmt(
                "%s|%6s|%-6s|%.2s",
                &[st("abc"), st("abc"), st("abc"), st("abc")]
            ),
            "abc|   abc|abc   |ab"
        );
        assert_eq!(fmt("%c%c", &[int(72), int(105)]), "Hi");
        assert_eq!(
            fmt(
                "%f|%.2f|%8.3f|%.0f",
                &[
                    Value::Real(1.5),
                    Value::Real(2.345),
                    Value::Real(-1.23456),
                    Value::Real(2.5)
                ]
            ),
            "1.500000|2.35|  -1.235|2"
        );
        assert_eq!(
            fmt("%e|%.2E", &[Value::Real(12345.678), Value::Real(0.00012)]),
            "1.234568e+04|1.20E-04"
        );
        assert_eq!(
            fmt(
                "%g|%g|%g|%g",
                &[
                    Value::Real(100000.0),
                    Value::Real(1000000.0),
                    Value::Real(0.0001),
                    Value::Real(0.5)
                ]
            ),
            "100000|1e+06|0.0001|0.5"
        );
        assert_eq!(fmt("100%%", &[]), "100%");
        assert_eq!(fmt("%.3d", &[int(5)]), "005");
        assert_eq!(fmt("%f", &[int(2)]), "2.000000");
    }

    #[test]
    fn a_format_that_does_not_match_its_arguments_is_an_error() {
        for (f, args) in [
            ("%d", vec![]),
            ("%d", vec![st("x")]),
            ("%s", vec![int(1)]),
            ("%q", vec![int(1)]),
            ("%", vec![]),
            ("%99999d", vec![int(1)]),
            ("%c", vec![int(-1)]),
        ] {
            assert_eq!(format_c(f, &args).unwrap_err().0, RUN_FORMAT, "{f}");
        }
    }
}
