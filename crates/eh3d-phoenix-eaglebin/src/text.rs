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

/// The names of the technologies of a device: the last character of each name has its high bit
/// set, and a byte with only that bit is the empty name. A device with none has the empty one.
pub fn technologies(bytes: &[u8], place: &str, report: &mut Report) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = Vec::new();
    for &b in bytes {
        if b == 0 {
            break;
        }
        if b & 0x80 == 0 {
            current.push(b);
            continue;
        }
        if b & 0x7F != 0 {
            current.push(b & 0x7F);
        }
        names.push(decode(&current, place, report));
        current.clear();
    }
    if !current.is_empty() {
        report.note(codes::VALUE, "technology name without its end", place);
        names.push(decode(&current, place, report));
    }
    if names.is_empty() {
        names.push(String::new());
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
        let list = technologies(b"\x80AL\xd3HC\xd4\xd3\0zz\xd3", "x", &mut report);
        assert_eq!(list, ["", "ALS", "HCT", "S"]);
        assert_eq!(technologies(b"", "x", &mut report), [""]);
        assert!(report.notes.is_empty());
        assert_eq!(technologies(b"AB", "x", &mut report), ["AB"]);
        assert_eq!(report.noted(codes::VALUE), 1);
    }
}
