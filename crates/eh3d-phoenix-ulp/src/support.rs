// SPDX-License-Identifier: GPL-3.0-or-later
//! The list of names that the compatibility report knows, and what is decided for each one: a data
//! file (`config/ulp_support.cfg`) and not code, so the scope follows the programs that are really
//! used. One name for each line: `kind name status`; a name that ends in `*` stands for every name
//! that starts with the rest, and an exact name wins over one with a star.

use crate::codes::{Diag, SUPPORT_INVALID};

/// The list that ships with the program.
pub const SHIPPED: &str = include_str!("../../../config/ulp_support.cfg");

/// The largest list that is read, in bytes.
pub const MAX_BYTES: usize = 1 << 20;

/// What kind of name an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UseKind {
    /// `include`, `usage`, `require`.
    Directive,
    /// `if`, `for`, a block, a call followed by a block...
    Statement,
    /// `int`, `UL_WIRE`...
    Type,
    /// A function that is called.
    Function,
    /// A member of an object (`E.name`, `B.elements(E)`).
    Member,
    /// A named value (`UL_WIRE`, `PI`, `argv`).
    Constant,
}

impl UseKind {
    /// Every kind, in the order of a report.
    pub const ALL: [UseKind; 6] = [
        UseKind::Directive,
        UseKind::Statement,
        UseKind::Type,
        UseKind::Function,
        UseKind::Member,
        UseKind::Constant,
    ];

    /// The word of the data file and of the JSON.
    pub fn word(self) -> &'static str {
        match self {
            UseKind::Directive => "directive",
            UseKind::Statement => "statement",
            UseKind::Type => "type",
            UseKind::Function => "function",
            UseKind::Member => "member",
            UseKind::Constant => "constant",
        }
    }

    fn from_word(word: &str) -> Option<UseKind> {
        UseKind::ALL.into_iter().find(|k| k.word() == word)
    }
}

/// What is decided for a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Read access to the model and pure computation.
    Allowed,
    /// Allowed inside the limits of the sandbox.
    Limited,
    /// Not decided or not written yet.
    Pending,
    /// The sandbox never lets it through.
    Denied,
}

impl Status {
    /// The word of the data file and of the JSON.
    pub fn word(self) -> &'static str {
        match self {
            Status::Allowed => "allowed",
            Status::Limited => "limited",
            Status::Pending => "pending",
            Status::Denied => "denied",
        }
    }

    fn from_word(word: &str) -> Option<Status> {
        [
            Status::Allowed,
            Status::Limited,
            Status::Pending,
            Status::Denied,
        ]
        .into_iter()
        .find(|s| s.word() == word)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    kind: UseKind,
    name: String,
    status: Status,
}

/// The list of names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Support {
    entries: Vec<Entry>,
}

fn name_is_valid(name: &str) -> bool {
    let stem = name.strip_suffix('*').unwrap_or(name);
    !stem.is_empty()
        && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !stem.starts_with(|c: char| c.is_ascii_digit())
}

impl Support {
    /// Reads a list. Blank lines and lines that start with `#` are ignored; a line that is not
    /// `kind name status`, a word that is not known or a name given twice is refused with its line.
    pub fn parse(text: &str) -> Result<Support, Diag> {
        if text.len() > MAX_BYTES {
            return Err(Diag::new(
                SUPPORT_INVALID,
                format!("the list is larger than {MAX_BYTES} bytes"),
            ));
        }
        let mut entries: Vec<Entry> = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            let [kind, name, status] = words.as_slice() else {
                return Err(Diag::at(
                    SUPPORT_INVALID,
                    line,
                    1,
                    "a line is `kind name status`",
                ));
            };
            let kind = UseKind::from_word(kind).ok_or_else(|| {
                Diag::at(SUPPORT_INVALID, line, 1, format!("unknown kind `{kind}`"))
            })?;
            let status = Status::from_word(status).ok_or_else(|| {
                Diag::at(
                    SUPPORT_INVALID,
                    line,
                    1,
                    format!("unknown status `{status}`"),
                )
            })?;
            if !name_is_valid(name) {
                return Err(Diag::at(
                    SUPPORT_INVALID,
                    line,
                    1,
                    format!("`{name}` is not a name"),
                ));
            }
            if entries.iter().any(|e| e.kind == kind && e.name == *name) {
                return Err(Diag::at(
                    SUPPORT_INVALID,
                    line,
                    1,
                    format!("`{name}` is listed twice"),
                ));
            }
            entries.push(Entry {
                kind,
                name: (*name).to_owned(),
                status,
            });
        }
        Ok(Support { entries })
    }

    /// The list that ships with the program.
    pub fn shipped() -> Support {
        // the shipped list is checked by a test; an empty list would only make every name unknown
        Support::parse(SHIPPED).unwrap_or(Support {
            entries: Vec::new(),
        })
    }

    /// How many names the list holds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True for a list with no names.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// What is decided for a name: the exact entry, or else the longest entry with a star that
    /// the name starts with. `None` when the list does not know the name.
    pub fn lookup(&self, kind: UseKind, name: &str) -> Option<Status> {
        if let Some(e) = self
            .entries
            .iter()
            .find(|e| e.kind == kind && e.name == name)
        {
            return Some(e.status);
        }
        self.entries
            .iter()
            .filter(|e| e.kind == kind)
            .filter_map(|e| {
                let stem = e.name.strip_suffix('*')?;
                name.starts_with(stem).then_some((stem.len(), e.status))
            })
            .max_by_key(|(len, _)| *len)
            .map(|(_, status)| status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_that_ships_is_valid_and_has_every_kind() {
        let s = Support::parse(SHIPPED).unwrap();
        assert!(s.len() > 150, "{}", s.len());
        for kind in UseKind::ALL {
            assert!(
                s.entries.iter().any(|e| e.kind == kind),
                "no {}",
                kind.word()
            );
        }
        assert_eq!(Support::shipped(), s);
    }

    #[test]
    fn the_names_that_matter_are_decided_as_the_design_says() {
        let s = Support::shipped();
        assert_eq!(
            s.lookup(UseKind::Function, "sprintf"),
            Some(Status::Allowed)
        );
        assert_eq!(s.lookup(UseKind::Function, "output"), Some(Status::Limited));
        assert_eq!(s.lookup(UseKind::Function, "system"), Some(Status::Denied));
        assert_eq!(
            s.lookup(UseKind::Function, "dlgDialog"),
            Some(Status::Allowed)
        );
        assert_eq!(s.lookup(UseKind::Type, "UL_ELEMENT"), Some(Status::Allowed));
        assert_eq!(
            s.lookup(UseKind::Directive, "include"),
            Some(Status::Limited)
        );
        assert_eq!(s.lookup(UseKind::Function, "nobody"), None);
        // the same word of another kind is another name
        assert_eq!(s.lookup(UseKind::Member, "system"), None);
    }

    #[test]
    fn an_exact_name_wins_over_a_name_with_a_star_and_the_longest_star_wins() {
        let s = Support::parse(
            "function dlg* pending\nfunction dlgLabel allowed\nfunction dlgList* denied\n",
        )
        .unwrap();
        assert_eq!(
            s.lookup(UseKind::Function, "dlgLabel"),
            Some(Status::Allowed)
        );
        assert_eq!(
            s.lookup(UseKind::Function, "dlgOther"),
            Some(Status::Pending)
        );
        assert_eq!(
            s.lookup(UseKind::Function, "dlgListView"),
            Some(Status::Denied)
        );
        assert_eq!(s.lookup(UseKind::Function, "dl"), None);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        for (text, line) in [
            ("function sprintf\n", 1),
            ("\nfunction sprintf allowed extra\n", 2),
            ("# a\nverb sprintf allowed\n", 2),
            ("function sprintf maybe\n", 1),
            ("function 1abc allowed\n", 1),
            ("function a-b allowed\n", 1),
            ("function * allowed\n", 1),
            ("function a allowed\nfunction a denied\n", 2),
        ] {
            let e = Support::parse(text).unwrap_err();
            assert_eq!(e.code, SUPPORT_INVALID, "{text:?}");
            assert_eq!(e.line, line, "{text:?}");
        }
    }

    #[test]
    fn a_list_that_is_too_large_is_refused() {
        let big = "#".repeat(MAX_BYTES + 1);
        assert_eq!(Support::parse(&big).unwrap_err().code, SUPPORT_INVALID);
        assert!(Support::parse("").unwrap().is_empty());
    }
}
