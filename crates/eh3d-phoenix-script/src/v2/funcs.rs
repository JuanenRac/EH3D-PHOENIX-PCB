// SPDX-License-Identifier: GPL-3.0-or-later
//! The table of the functions of the language. It is the only list of them: the interpreter looks
//! a call up here, the static check reads the number of arguments and the documents a function
//! applies to from here, and `phoenix script help` prints its reference from here, so the three
//! always agree.

use crate::codes::Diag;
use crate::v2::codes::TYPE_MISMATCH;
use crate::v2::interp::Interp;
use crate::v2::value::Value;
use crate::v2::world::{BOARD, LIBRARY, SCHEMATIC};

/// The body of a function: the interpreter, the evaluated arguments and the value it returns.
pub(crate) type RunFn = fn(&mut Interp<'_>, &[Value]) -> Result<Value, Diag>;

/// One function of the language.
pub(crate) struct FuncDef {
    /// The name a script calls it by.
    pub name: &'static str,
    /// The fewest arguments.
    pub min: usize,
    /// The most arguments.
    pub max: usize,
    /// The documents it applies to, as bits (`BOARD`, `SCHEMATIC`, `LIBRARY`); 0: any, even none.
    pub targets: u8,
    /// True when it changes the document.
    pub op: bool,
    /// How it is written, for the reference.
    pub sig: &'static str,
    /// What it does, for the reference.
    pub doc: &'static str,
    /// The body.
    pub run: RunFn,
}

/// Builds an entry of a table.
#[allow(clippy::too_many_arguments)]
pub(crate) const fn def(
    name: &'static str,
    min: usize,
    max: usize,
    targets: u8,
    op: bool,
    sig: &'static str,
    doc: &'static str,
    run: RunFn,
) -> FuncDef {
    FuncDef {
        name,
        min,
        max,
        targets,
        op,
        sig,
        doc,
        run,
    }
}

/// The groups of the table, in the order of the reference, with the heading of each.
pub(crate) fn groups() -> [(&'static str, &'static [FuncDef]); 6] {
    [
        ("Values, text and lists", crate::v2::funcs_core::TABLE),
        ("Output and checks", crate::v2::funcs_core::OUTPUT),
        (
            "Queries and measures (board and schematic)",
            crate::v2::funcs_doc::TABLE,
        ),
        ("Board operations", crate::v2::funcs_board::TABLE),
        (
            "Schematic and library operations",
            crate::v2::funcs_sch::TABLE,
        ),
        ("Data files", crate::v2::funcs_io::TABLE),
    ]
}

/// The function with a name.
pub(crate) fn find(name: &str) -> Option<&'static FuncDef> {
    groups()
        .into_iter()
        .flat_map(|(_, table)| table.iter())
        .find(|f| f.name == name)
}

/// The words for the documents a function applies to.
pub(crate) fn targets_text(mask: u8) -> String {
    if mask == 0 {
        return "any".to_owned();
    }
    let mut words = Vec::new();
    if mask & BOARD != 0 {
        words.push("board");
    }
    if mask & SCHEMATIC != 0 {
        words.push("schematic");
    }
    if mask & LIBRARY != 0 {
        words.push("library");
    }
    words.join(", ")
}

fn bad(index: usize, want: &str, got: &Value) -> Diag {
    Diag::plain(
        TYPE_MISMATCH,
        format!(
            "argument {} must be {want}, not {}",
            index + 1,
            got.type_name()
        ),
    )
}

/// Argument `i` as an integer.
pub(crate) fn int(args: &[Value], i: usize) -> Result<i64, Diag> {
    match args.get(i) {
        Some(Value::Int(n)) => Ok(*n),
        Some(other) => Err(bad(i, "an int", other)),
        None => Err(Diag::plain(
            TYPE_MISMATCH,
            format!("argument {} is missing", i + 1),
        )),
    }
}

/// Argument `i` as a text.
pub(crate) fn text(args: &[Value], i: usize) -> Result<&str, Diag> {
    match args.get(i) {
        Some(Value::Str(s)) => Ok(s.as_str()),
        Some(other) => Err(bad(i, "a text", other)),
        None => Err(Diag::plain(
            TYPE_MISMATCH,
            format!("argument {} is missing", i + 1),
        )),
    }
}

/// Argument `i` as a list.
pub(crate) fn list(args: &[Value], i: usize) -> Result<&[Value], Diag> {
    match args.get(i) {
        Some(Value::List(items)) => Ok(items.as_slice()),
        Some(other) => Err(bad(i, "a list", other)),
        None => Err(Diag::plain(
            TYPE_MISMATCH,
            format!("argument {} is missing", i + 1),
        )),
    }
}

/// Argument `i` as a boolean.
pub(crate) fn boolean(args: &[Value], i: usize) -> Result<bool, Diag> {
    match args.get(i) {
        Some(Value::Bool(b)) => Ok(*b),
        Some(other) => Err(bad(i, "a bool", other)),
        None => Err(Diag::plain(
            TYPE_MISMATCH,
            format!("argument {} is missing", i + 1),
        )),
    }
}

/// Argument `i` as an angle that is a whole number of quarter turns.
pub(crate) fn quarter_turns(args: &[Value], i: usize) -> Result<i32, Diag> {
    let udeg = int(args, i)?;
    if udeg % 90_000_000 != 0 {
        return Err(Diag::plain(
            TYPE_MISMATCH,
            format!("argument {} must be a multiple of 90deg", i + 1),
        ));
    }
    i32::try_from((udeg / 90_000_000).rem_euclid(4))
        .map_err(|_| Diag::plain(TYPE_MISMATCH, "the angle is out of range"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_are_unique_and_the_counts_are_consistent() {
        let mut seen = std::collections::BTreeSet::new();
        for (_, table) in groups() {
            for f in table {
                assert!(seen.insert(f.name), "{} is listed twice", f.name);
                assert!(f.min <= f.max, "{}", f.name);
                assert!(!f.doc.is_empty() && f.sig.starts_with(f.name), "{}", f.name);
                assert!(
                    !crate::v2::syntax::is_keyword(f.name),
                    "{} is a keyword",
                    f.name
                );
                assert!(
                    f.targets & !(BOARD | SCHEMATIC | LIBRARY) == 0,
                    "{}",
                    f.name
                );
            }
        }
        assert!(seen.len() >= 60);
        assert!(find("move_to").is_some());
        assert!(find("nothing_here").is_none());
    }

    #[test]
    fn the_helpers_name_the_argument_and_the_types() {
        let args = [Value::Int(3), Value::Str("a".to_owned())];
        assert_eq!(int(&args, 0).unwrap(), 3);
        assert_eq!(text(&args, 1).unwrap(), "a");
        let e = int(&args, 1).unwrap_err();
        assert!(e.detail.contains("argument 2") && e.detail.contains("text"));
        assert!(list(&args, 0).is_err());
        assert!(boolean(&args, 5).is_err());
        assert_eq!(quarter_turns(&[Value::Int(-90_000_000)], 0).unwrap(), 3);
        assert!(quarter_turns(&[Value::Int(45_000_000)], 0).is_err());
        assert_eq!(targets_text(0), "any");
        assert_eq!(targets_text(BOARD | LIBRARY), "board, library");
    }
}
