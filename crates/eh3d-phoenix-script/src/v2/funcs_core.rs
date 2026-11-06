// SPDX-License-Identifier: GPL-3.0-or-later
//! The functions that need no document: arithmetic in integers, text, lists, output and checks.

use crate::codes::Diag;
use crate::v2::codes::{ARITHMETIC, ASSERTION, MEMORY_LIMIT, TYPE_MISMATCH};
use crate::v2::funcs::{boolean, def, int, list, text, FuncDef};
use crate::v2::interp::Interp;
use crate::v2::value::{cos_ppm, glob, isqrt, mm_text, round_div, sin_ppm, Value};

type R = Result<Value, Diag>;

fn big(n: i128) -> R {
    i64::try_from(n)
        .map(Value::Int)
        .map_err(|_| Diag::plain(ARITHMETIC, "the result does not fit 64 bits"))
}

fn too_long(what: &str, limit: usize) -> Diag {
    Diag::plain(MEMORY_LIMIT, format!("{what} of more than {limit}"))
}

fn f_len(_i: &mut Interp<'_>, a: &[Value]) -> R {
    match &a[0] {
        Value::List(items) => big(items.len() as i128),
        Value::Str(s) => big(s.chars().count() as i128),
        other => Err(Diag::plain(
            TYPE_MISMATCH,
            format!("len takes a list or a text, not {}", other.type_name()),
        )),
    }
}

fn f_str(i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Str(i.render(&a[0])))
}

fn f_int(_i: &mut Interp<'_>, a: &[Value]) -> R {
    match &a[0] {
        Value::Int(n) => Ok(Value::Int(*n)),
        Value::Str(s) => s
            .trim()
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| Diag::plain(TYPE_MISMATCH, format!("{s:?} is not an integer"))),
        other => Err(Diag::plain(
            TYPE_MISMATCH,
            format!("int takes a text or a number, not {}", other.type_name()),
        )),
    }
}

fn f_abs(_i: &mut Interp<'_>, a: &[Value]) -> R {
    int(a, 0)?
        .checked_abs()
        .map(Value::Int)
        .ok_or_else(|| Diag::plain(ARITHMETIC, "the result does not fit 64 bits"))
}

fn f_min(_i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Int(int(a, 0)?.min(int(a, 1)?)))
}

fn f_max(_i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Int(int(a, 0)?.max(int(a, 1)?)))
}

fn f_isqrt(_i: &mut Interp<'_>, a: &[Value]) -> R {
    big(isqrt(i128::from(int(a, 0)?)))
}

fn f_muldiv(_i: &mut Interp<'_>, a: &[Value]) -> R {
    let (x, y, z) = (int(a, 0)?, int(a, 1)?, int(a, 2)?);
    if z == 0 {
        return Err(Diag::plain(ARITHMETIC, "division by zero"));
    }
    big(round_div(i128::from(x) * i128::from(y), i128::from(z)))
}

fn f_sin_ppm(_i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Int(sin_ppm(int(a, 0)?)))
}

fn f_cos_ppm(_i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Int(cos_ppm(int(a, 0)?)))
}

fn f_polar_x(_i: &mut Interp<'_>, a: &[Value]) -> R {
    let (r, angle) = (int(a, 0)?, int(a, 1)?);
    big(round_div(
        i128::from(r) * i128::from(cos_ppm(angle)),
        1_000_000,
    ))
}

fn f_polar_y(_i: &mut Interp<'_>, a: &[Value]) -> R {
    let (r, angle) = (int(a, 0)?, int(a, 1)?);
    big(round_div(
        i128::from(r) * i128::from(sin_ppm(angle)),
        1_000_000,
    ))
}

fn f_deg(_i: &mut Interp<'_>, a: &[Value]) -> R {
    big(i128::from(int(a, 0)?) * 1_000_000)
}

fn f_mm(_i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Str(mm_text(int(a, 0)?)))
}

fn f_push(i: &mut Interp<'_>, a: &[Value]) -> R {
    let items = list(a, 0)?;
    let limit = i.ctx.settings.max_list_len;
    if items.len() >= limit {
        return Err(too_long("a list", limit));
    }
    let mut out = items.to_vec();
    out.push(a[1].clone());
    Ok(Value::List(out))
}

fn f_concat(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (x, y) = (list(a, 0)?, list(a, 1)?);
    let limit = i.ctx.settings.max_list_len;
    if x.len() + y.len() > limit {
        return Err(too_long("a list", limit));
    }
    let mut out = x.to_vec();
    out.extend_from_slice(y);
    Ok(Value::List(out))
}

fn f_reverse(_i: &mut Interp<'_>, a: &[Value]) -> R {
    let mut out = list(a, 0)?.to_vec();
    out.reverse();
    Ok(Value::List(out))
}

fn f_sum(_i: &mut Interp<'_>, a: &[Value]) -> R {
    let mut total: i64 = 0;
    for item in list(a, 0)? {
        let Value::Int(n) = item else {
            return Err(Diag::plain(
                TYPE_MISMATCH,
                format!("sum takes a list of ints, not of {}", item.type_name()),
            ));
        };
        total = total
            .checked_add(*n)
            .ok_or_else(|| Diag::plain(ARITHMETIC, "the result does not fit 64 bits"))?;
    }
    Ok(Value::Int(total))
}

fn f_join(i: &mut Interp<'_>, a: &[Value]) -> R {
    let sep = text(a, 1)?;
    let limit = i.ctx.settings.max_text_bytes;
    let mut out = String::new();
    for (k, item) in list(a, 0)?.iter().enumerate() {
        if k > 0 {
            out.push_str(sep);
        }
        out.push_str(&i.render(item));
        if out.len() > limit {
            return Err(too_long("a text", limit));
        }
    }
    Ok(Value::Str(out))
}

fn f_contains(_i: &mut Interp<'_>, a: &[Value]) -> R {
    match &a[0] {
        Value::List(items) => Ok(Value::Bool(items.contains(&a[1]))),
        Value::Str(s) => Ok(Value::Bool(s.contains(text(a, 1)?))),
        other => Err(Diag::plain(
            TYPE_MISMATCH,
            format!("contains takes a list or a text, not {}", other.type_name()),
        )),
    }
}

fn f_sort(_i: &mut Interp<'_>, a: &[Value]) -> R {
    let items = list(a, 0)?;
    if items.iter().all(|v| matches!(v, Value::Int(_))) {
        let mut out: Vec<i64> = items
            .iter()
            .filter_map(|v| {
                if let Value::Int(n) = v {
                    Some(*n)
                } else {
                    None
                }
            })
            .collect();
        out.sort_unstable();
        return Ok(Value::List(out.into_iter().map(Value::Int).collect()));
    }
    if items.iter().all(|v| matches!(v, Value::Str(_))) {
        let mut out: Vec<String> = items
            .iter()
            .filter_map(|v| {
                if let Value::Str(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect();
        out.sort();
        return Ok(Value::List(out.into_iter().map(Value::Str).collect()));
    }
    Err(Diag::plain(
        TYPE_MISMATCH,
        "sort takes a list of ints or a list of texts",
    ))
}

fn f_range(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (first, last) = (int(a, 0)?, int(a, 1)?);
    let limit = i.ctx.settings.max_list_len;
    let count = i128::from(last) - i128::from(first);
    if count > limit as i128 {
        return Err(too_long("a list", limit));
    }
    Ok(Value::List((first..last).map(Value::Int).collect()))
}

fn f_upper(_i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Str(text(a, 0)?.to_uppercase()))
}

fn f_lower(_i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Str(text(a, 0)?.to_lowercase()))
}

fn f_starts_with(_i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Bool(text(a, 0)?.starts_with(text(a, 1)?)))
}

fn f_ends_with(_i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Bool(text(a, 0)?.ends_with(text(a, 1)?)))
}

fn f_zpad(_i: &mut Interp<'_>, a: &[Value]) -> R {
    let (n, width) = (int(a, 0)?, int(a, 1)?);
    if !(0..=64).contains(&width) {
        return Err(Diag::plain(ARITHMETIC, "the width is not 0 to 64"));
    }
    let width = usize::try_from(width).unwrap_or(0);
    Ok(Value::Str(format!("{n:0width$}")))
}

fn f_repeat(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (s, n) = (text(a, 0)?, int(a, 1)?);
    let limit = i.ctx.settings.max_text_bytes;
    let n = usize::try_from(n).map_err(|_| Diag::plain(ARITHMETIC, "the count is negative"))?;
    if s.len().saturating_mul(n) > limit {
        return Err(too_long("a text", limit));
    }
    Ok(Value::Str(s.repeat(n)))
}

fn f_glob(_i: &mut Interp<'_>, a: &[Value]) -> R {
    Ok(Value::Bool(glob(text(a, 0)?, text(a, 1)?)))
}

fn f_split(i: &mut Interp<'_>, a: &[Value]) -> R {
    let (s, sep) = (text(a, 0)?, text(a, 1)?);
    if sep.is_empty() {
        return Err(Diag::plain(TYPE_MISMATCH, "the separator is empty"));
    }
    let limit = i.ctx.settings.max_list_len;
    let parts: Vec<Value> = s
        .split(sep)
        .take(limit + 1)
        .map(|p| Value::Str(p.to_owned()))
        .collect();
    if parts.len() > limit {
        return Err(too_long("a list", limit));
    }
    Ok(Value::List(parts))
}

fn f_lines(i: &mut Interp<'_>, a: &[Value]) -> R {
    let limit = i.ctx.settings.max_list_len;
    let parts: Vec<Value> = text(a, 0)?
        .lines()
        .take(limit + 1)
        .map(|p| Value::Str(p.to_owned()))
        .collect();
    if parts.len() > limit {
        return Err(too_long("a list", limit));
    }
    Ok(Value::List(parts))
}

fn f_print(i: &mut Interp<'_>, a: &[Value]) -> R {
    let parts: Vec<String> = a.iter().map(|v| i.render(v)).collect();
    i.emit(parts.join(" "))?;
    Ok(Value::Nil)
}

fn f_assert(i: &mut Interp<'_>, a: &[Value]) -> R {
    if boolean(a, 0)? {
        return Ok(Value::Nil);
    }
    let why = match a.get(1) {
        Some(v) => i.render(v),
        None => "the condition is false".to_owned(),
    };
    Err(Diag::plain(ASSERTION, why))
}

fn f_fail(i: &mut Interp<'_>, a: &[Value]) -> R {
    Err(Diag::plain(ASSERTION, i.render(&a[0])))
}

/// The functions on numbers, text and lists.
pub(crate) static TABLE: &[FuncDef] = &[
    def(
        "len",
        1,
        1,
        0,
        false,
        "len(x)",
        "The number of items of a list or of characters of a text.",
        f_len,
    ),
    def(
        "str",
        1,
        1,
        0,
        false,
        "str(x)",
        "The text of any value; an object is written by its designator or its net.",
        f_str,
    ),
    def(
        "int",
        1,
        1,
        0,
        false,
        "int(text)",
        "The integer a text holds; an error when it holds anything else.",
        f_int,
    ),
    def(
        "abs",
        1,
        1,
        0,
        false,
        "abs(n)",
        "The absolute value.",
        f_abs,
    ),
    def(
        "min",
        2,
        2,
        0,
        false,
        "min(a, b)",
        "The smaller of two integers.",
        f_min,
    ),
    def(
        "max",
        2,
        2,
        0,
        false,
        "max(a, b)",
        "The larger of two integers.",
        f_max,
    ),
    def(
        "isqrt",
        1,
        1,
        0,
        false,
        "isqrt(n)",
        "The integer square root, rounded down.",
        f_isqrt,
    ),
    def(
        "muldiv",
        3,
        3,
        0,
        false,
        "muldiv(a, b, c)",
        "a * b / c computed in 128 bits and rounded half away from zero.",
        f_muldiv,
    ),
    def(
        "sin_ppm",
        1,
        1,
        0,
        false,
        "sin_ppm(angle)",
        "The sine of an angle in micro-degrees, in millionths (1000000 is 1).",
        f_sin_ppm,
    ),
    def(
        "cos_ppm",
        1,
        1,
        0,
        false,
        "cos_ppm(angle)",
        "The cosine of an angle in micro-degrees, in millionths.",
        f_cos_ppm,
    ),
    def(
        "polar_x",
        2,
        2,
        0,
        false,
        "polar_x(radius, angle)",
        "radius * cos(angle), rounded to the nanometre.",
        f_polar_x,
    ),
    def(
        "polar_y",
        2,
        2,
        0,
        false,
        "polar_y(radius, angle)",
        "radius * sin(angle), rounded to the nanometre.",
        f_polar_y,
    ),
    def(
        "deg",
        1,
        1,
        0,
        false,
        "deg(n)",
        "A whole number of degrees as micro-degrees (the same as writing 90deg).",
        f_deg,
    ),
    def(
        "mm",
        1,
        1,
        0,
        false,
        "mm(length)",
        "A length in nanometres as a text in millimetres, without trailing zeros.",
        f_mm,
    ),
    def(
        "push",
        2,
        2,
        0,
        false,
        "push(list, item)",
        "A new list with the item added at the end.",
        f_push,
    ),
    def(
        "concat",
        2,
        2,
        0,
        false,
        "concat(a, b)",
        "A new list with the items of both lists.",
        f_concat,
    ),
    def(
        "reverse",
        1,
        1,
        0,
        false,
        "reverse(list)",
        "A new list in the opposite order.",
        f_reverse,
    ),
    def(
        "sum",
        1,
        1,
        0,
        false,
        "sum(list)",
        "The sum of a list of integers.",
        f_sum,
    ),
    def(
        "join",
        2,
        2,
        0,
        false,
        "join(list, separator)",
        "The items of a list written as one text.",
        f_join,
    ),
    def(
        "contains",
        2,
        2,
        0,
        false,
        "contains(list_or_text, x)",
        "Whether a list has the item, or a text has the piece.",
        f_contains,
    ),
    def(
        "sort",
        1,
        1,
        0,
        false,
        "sort(list)",
        "A new list of integers or of texts in ascending order.",
        f_sort,
    ),
    def(
        "range",
        2,
        2,
        0,
        false,
        "range(first, end)",
        "The list first, first+1, ... end-1.",
        f_range,
    ),
    def(
        "upper",
        1,
        1,
        0,
        false,
        "upper(text)",
        "The text in capitals.",
        f_upper,
    ),
    def(
        "lower",
        1,
        1,
        0,
        false,
        "lower(text)",
        "The text in small letters.",
        f_lower,
    ),
    def(
        "starts_with",
        2,
        2,
        0,
        false,
        "starts_with(text, prefix)",
        "Whether the text starts with the prefix.",
        f_starts_with,
    ),
    def(
        "ends_with",
        2,
        2,
        0,
        false,
        "ends_with(text, suffix)",
        "Whether the text ends with the suffix.",
        f_ends_with,
    ),
    def(
        "zpad",
        2,
        2,
        0,
        false,
        "zpad(n, width)",
        "The number written with zeros in front up to the width.",
        f_zpad,
    ),
    def(
        "repeat",
        2,
        2,
        0,
        false,
        "repeat(text, n)",
        "The text written n times.",
        f_repeat,
    ),
    def(
        "glob",
        2,
        2,
        0,
        false,
        "glob(pattern, text)",
        "Whether the text matches the pattern (* any run, ? one character).",
        f_glob,
    ),
    def(
        "split",
        2,
        2,
        0,
        false,
        "split(text, separator)",
        "The pieces of a text between the separators, as a list.",
        f_split,
    ),
    def(
        "lines",
        1,
        1,
        0,
        false,
        "lines(text)",
        "The lines of a text, as a list.",
        f_lines,
    ),
];

/// The functions that write output and stop a script that is wrong.
pub(crate) static OUTPUT: &[FuncDef] = &[
    def(
        "print",
        0,
        64,
        0,
        false,
        "print(x, ...)",
        "Writes a line with the values separated by a blank.",
        f_print,
    ),
    def(
        "assert",
        1,
        2,
        0,
        false,
        "assert(condition, message)",
        "Stops the script with an error when the condition is false.",
        f_assert,
    ),
    def(
        "fail",
        1,
        1,
        0,
        false,
        "fail(message)",
        "Stops the script with an error.",
        f_fail,
    ),
];
