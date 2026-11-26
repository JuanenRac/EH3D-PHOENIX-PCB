// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the records: short ones sit in their field, long ones follow the records. The
//! long ones are taken in the order of the records and, inside a record, from the last field to
//! the first (the name before the description).

use crate::codes;
use crate::records::{Record, LONG};
use crate::report::Report;

/// The long strings of a file, taken one by one.
#[derive(Debug)]
pub struct Strings<'a> {
    list: &'a [Vec<u8>],
    next: usize,
    /// How many fields asked for a string when none was left.
    pub missing: usize,
}

impl<'a> Strings<'a> {
    /// The strings of a file.
    pub fn new(list: &'a [Vec<u8>]) -> Strings<'a> {
        Strings {
            list,
            next: 0,
            missing: 0,
        }
    }

    /// How many strings no field asked for.
    pub fn left(&self) -> usize {
        self.list.len().saturating_sub(self.next)
    }

    /// The bytes of the field `start..end` of a record.
    pub fn field(&mut self, r: &Record, start: usize, end: usize) -> Vec<u8> {
        let place = r.get(start..end).unwrap_or(&[]);
        if place.first() == Some(&LONG) {
            return match self.list.get(self.next) {
                Some(s) => {
                    self.next += 1;
                    s.clone()
                }
                None => {
                    self.missing += 1;
                    Vec::new()
                }
            };
        }
        place.iter().copied().take_while(|&b| b != 0).collect()
    }
}

/// True for a character that an XML file can hold.
fn allowed(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && c != '\u{FFFE}' && c != '\u{FFFF}')
}

/// The text of the bytes of a field: UTF-8 when it is, else Latin-1. Characters that XML cannot
/// hold are left out and noted.
pub fn decode(bytes: &[u8], place: &str, report: &mut Report) -> String {
    let text: String = match std::str::from_utf8(bytes) {
        Ok(t) => t.to_owned(),
        Err(_) => {
            report.note(codes::ENCODING, "text that is not UTF-8", place);
            bytes.iter().map(|&b| char::from(b)).collect()
        }
    };
    if text.chars().all(allowed) {
        return text;
    }
    report.note(codes::VALUE, "control characters left out of a text", place);
    text.chars().filter(|&c| allowed(c)).collect()
}

/// The byte that goes before each name in the second form of a list of technologies.
const SEPARATOR: u8 = 0x04;
/// The byte that goes before each name of an attribute, ahead of the first technology.
const ATTRIBUTE: u8 = 0x01;
/// The byte that goes before the value of each attribute, after the name of a technology.
const VALUE: u8 = 0x02;
/// The same for a value that a part cannot change (inferred).
const CONSTANT: u8 = 0x03;

/// An attribute of a technology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The name.
    pub name: String,
    /// The value.
    pub value: String,
    /// True when a part cannot change the value.
    pub constant: bool,
}

/// A technology of a device with its attributes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Technology {
    /// The name; empty for the technology without a name.
    pub name: String,
    /// The attributes.
    pub attributes: Vec<Attribute>,
}

impl Technology {
    fn named(name: String) -> Technology {
        Technology {
            name,
            attributes: Vec::new(),
        }
    }
}

/// The list of the second form: the names of the attributes (each after the byte `0x01`), then for
/// each technology its name after `0x04` and the values of the attributes, each after `0x02`, or
/// after `0x03` when it is constant.
fn with_separators(bytes: &[u8], place: &str, report: &mut Report) -> Vec<Technology> {
    let mut blocks = bytes.split(|&b| b == SEPARATOR);
    let head = blocks.next().unwrap_or_default();
    let names: Vec<String> = head
        .split(|&b| b == ATTRIBUTE)
        .skip(1)
        .map(|name| decode(name, place, report))
        .collect();
    let mut out = Vec::new();
    for block in blocks {
        let is_mark = |b: &u8| *b == VALUE || *b == CONSTANT;
        let first = block.iter().position(is_mark).unwrap_or(block.len());
        let name = decode(&block[..first], place, report);
        let mut rest = &block[first..];
        let mut attributes = Vec::new();
        while let Some((&mark, after)) = rest.split_first() {
            let end = after.iter().position(is_mark).unwrap_or(after.len());
            let Some(attribute) = names.get(attributes.len()) else {
                report.note(codes::VALUE, "value of an attribute without a name", place);
                break;
            };
            if mark == CONSTANT {
                report.note(codes::VALUE, "constant attribute (inferred)", place);
            }
            attributes.push(Attribute {
                name: attribute.clone(),
                value: decode(&after[..end], place, report),
                constant: mark == CONSTANT,
            });
            rest = &after[end..];
        }
        out.push(Technology { name, attributes });
    }
    if out.is_empty() {
        report.note(codes::VALUE, "attributes without a technology", place);
        out.push(Technology::default());
    }
    out
}

/// The technologies of a device. In one form the last character of each name has its high bit
/// set, and a byte with only that bit is the empty name; in the other each name follows the byte
/// `0x04`, with the attributes of the device when it has them. A device with none has the empty
/// one.
pub fn technologies(bytes: &[u8], place: &str, report: &mut Report) -> Vec<Technology> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let bytes = &bytes[..end];
    if matches!(bytes.first(), Some(&SEPARATOR) | Some(&ATTRIBUTE)) {
        return with_separators(bytes, place, report);
    }
    let mut names = Vec::new();
    let mut current = Vec::new();
    for &b in bytes {
        if b & 0x80 == 0 {
            current.push(b);
            continue;
        }
        if b & 0x7F != 0 {
            current.push(b & 0x7F);
        }
        names.push(Technology::named(decode(&current, place, report)));
        current.clear();
    }
    if !current.is_empty() {
        report.note(codes::VALUE, "technology name without its end", place);
        names.push(Technology::named(decode(&current, place, report)));
    }
    if names.is_empty() {
        names.push(Technology::default());
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_is_short_up_to_its_zero_or_long_in_order() {
        let list = vec![b"first long".to_vec(), b"second long".to_vec()];
        let mut s = Strings::new(&list);
        let mut r: Record = [0; 24];
        r[16..20].copy_from_slice(b"abcd");
        assert_eq!(s.field(&r, 16, 24), b"abcd");
        assert_eq!(s.field(&r, 16, 18), b"ab");
        r[16] = LONG;
        assert_eq!(s.field(&r, 16, 24), b"first long");
        assert_eq!(s.left(), 1);
        assert_eq!(s.field(&r, 16, 24), b"second long");
        assert_eq!(s.field(&r, 16, 24), b"");
        assert_eq!((s.left(), s.missing), (0, 1));
        assert_eq!(s.field(&r, 30, 40), b"");
    }

    #[test]
    fn texts_that_are_not_utf8_or_have_control_characters_are_noted() {
        let mut report = Report::default();
        assert_eq!(decode(b"1k\xB5", "x", &mut report), "1k\u{B5}");
        assert_eq!(decode(b"A\x1bB\n", "x", &mut report), "AB\n");
        assert_eq!(report.noted(codes::ENCODING), 1);
        assert_eq!(report.noted(codes::VALUE), 1);
    }

    #[test]
    fn technologies_end_with_the_high_bit() {
        let mut report = Report::default();
        let names = |bytes: &[u8], report: &mut Report| -> Vec<String> {
            technologies(bytes, "x", report)
                .into_iter()
                .map(|t| t.name)
                .collect()
        };
        let list = names(b"\x80AL\xd3HC\xd4\xd3\0zz\xd3", &mut report);
        assert_eq!(list, ["", "ALS", "HCT", "S"]);
        assert_eq!(names(b"", &mut report), [""]);
        let list = names(b"\x04344\x04345\0zz", &mut report);
        assert_eq!(list, ["344", "345"]);
        assert!(report.notes.is_empty());
        assert_eq!(names(b"AB", &mut report), ["AB"]);
        assert_eq!(report.noted(codes::VALUE), 1);
    }

    #[test]
    fn the_attributes_of_the_technologies_come_with_their_values() {
        let mut report = Report::default();
        let list = technologies(
            b"\x01MPN\x01MAKER\x04\x02a1\x02\x04SMD\x02b2\x02ACME\0zz",
            "x",
            &mut report,
        );
        let pairs = |t: &Technology| -> Vec<(String, String, bool)> {
            t.attributes
                .iter()
                .map(|a| (a.name.clone(), a.value.clone(), a.constant))
                .collect()
        };
        let row = |a: &str, b: &str, c: bool| (a.to_owned(), b.to_owned(), c);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "");
        assert_eq!(
            pairs(&list[0]),
            [row("MPN", "a1", false), row("MAKER", "", false)]
        );
        assert_eq!(list[1].name, "SMD");
        assert_eq!(
            pairs(&list[1]),
            [row("MPN", "b2", false), row("MAKER", "ACME", false)]
        );
        assert!(report.notes.is_empty(), "{:?}", report.notes);
        let mut inferred = Report::default();
        let list = technologies(b"\x01A\x01B\x04T\x03k\x02v", "x", &mut inferred);
        assert_eq!(pairs(&list[0]), [row("A", "k", true), row("B", "v", false)]);
        assert_eq!(inferred.noted(codes::VALUE), 1);
        // damaged lists are noted and never panic
        assert_eq!(technologies(b"\x01A", "x", &mut report).len(), 1);
        let extra = technologies(b"\x04T\x02v", "x", &mut report);
        assert!(extra[0].attributes.is_empty());
        assert_eq!(report.noted(codes::VALUE), 2);
    }
}
