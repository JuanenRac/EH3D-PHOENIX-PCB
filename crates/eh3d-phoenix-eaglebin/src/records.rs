// SPDX-License-Identifier: GPL-3.0-or-later
//! The container of a binary file: a table of records of 24 bytes and, after it, the long strings.
//!
//! ```text
//! record 0 (header)  byte 0 = 0x10; bytes 4..8 = number of records; bytes 8, 9 = version
//! records 1..N       one object each, the first byte says which one
//! 13 12 99 19        mark of the strings (absent when there are none)
//! u32                number of bytes of the strings
//! strings            each one ends with a zero byte, and one more zero ends the list
//! 12 bytes           not interpreted
//! ```
//!
//! A field of text that is too long for its place in a record starts with the byte `0x7F`; the
//! string is then the next one of the list, in the order of the records and of the fields.
//!
//! All numbers are little-endian.

use crate::codes::{self, BinError};

/// The bytes of one record.
pub const RECORD: usize = 24;
/// The first byte of the header.
pub const HEADER: u8 = 0x10;
/// The mark before the long strings.
pub const STRINGS_MARK: [u8; 4] = [0x13, 0x12, 0x99, 0x19];
/// The byte that says that the text of a field is one of the long strings.
pub const LONG: u8 = 0x7F;
/// The bytes after the strings that the format has and the reader does not interpret.
pub const TAIL: usize = 12;

/// The limits of a read. Everything the file claims is checked against them before memory is
/// asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Largest input in bytes.
    pub max_bytes: usize,
    /// Most records.
    pub max_records: usize,
    /// Most long strings.
    pub max_strings: usize,
    /// Most items (of all kinds) in the converted library.
    pub max_items: usize,
    /// Longest long string, in bytes.
    pub max_string_bytes: usize,
    /// Most work of resolving references: the pins, pads and bytes of names that are copied.
    pub max_work: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_bytes: 64 * 1024 * 1024,
            max_records: 1_500_000,
            max_strings: 300_000,
            max_items: 1_500_000,
            max_string_bytes: 256 * 1024,
            max_work: 64 * 1024 * 1024,
        }
    }
}

/// One record.
pub type Record = [u8; RECORD];

/// An unsigned 16-bit number at `at`; zero when it does not fit in the record.
pub fn u16_at(r: &Record, at: usize) -> u16 {
    match r.get(at..at + 2) {
        Some(b) => u16::from_le_bytes([b[0], b[1]]),
        None => 0,
    }
}

/// An unsigned 32-bit number at `at`; zero when it does not fit in the record.
pub fn u32_at(r: &Record, at: usize) -> u32 {
    match r.get(at..at + 4) {
        Some(b) => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        None => 0,
    }
}

/// A signed 32-bit number at `at`; zero when it does not fit in the record.
pub fn i32_at(r: &Record, at: usize) -> i32 {
    match r.get(at..at + 4) {
        Some(b) => i32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        None => 0,
    }
}

/// The records and the long strings of a file.
#[derive(Debug, Clone)]
pub struct Container {
    /// Every record, the header first.
    pub records: Vec<Record>,
    /// The long strings, in order, without their zero.
    pub strings: Vec<Vec<u8>>,
    /// The version that wrote the file: major and minor.
    pub version: (u8, u8),
    /// Bytes after the strings (or after the records) that are not part of what the format has.
    pub extra_bytes: usize,
    /// True when the strings are announced and are cut short.
    pub strings_cut: bool,
}

/// True when the bytes start like a binary file of the old versions.
pub fn looks_binary(bytes: &[u8]) -> bool {
    bytes.first() == Some(&HEADER)
}

/// Splits the bytes of a file into records and strings.
pub fn split(bytes: &[u8], limits: &Limits) -> Result<Container, BinError> {
    if bytes.len() > limits.max_bytes {
        return Err(BinError::new(codes::LIMIT, "the file is too large"));
    }
    if !looks_binary(bytes) || bytes.len() < RECORD {
        return Err(BinError::new(codes::NOT_BINARY, "no header"));
    }
    let mut header: Record = [0; RECORD];
    header.copy_from_slice(&bytes[..RECORD]);
    let version = (header[8], header[9]);
    if !(3..=5).contains(&version.0) {
        return Err(BinError::new(
            codes::VERSION,
            format!("version {}.{}", version.0, version.1),
        ));
    }
    let count = u32_at(&header, 4) as usize;
    if count > limits.max_records {
        return Err(BinError::new(codes::LIMIT, format!("{count} records")));
    }
    let table = count.checked_mul(RECORD).filter(|&n| n <= bytes.len());
    let Some(table) = table.filter(|_| count > 0) else {
        return Err(BinError::new(
            codes::STRUCTURE,
            format!("the header announces {count} records and the file is shorter"),
        ));
    };
    let records: Vec<Record> = bytes[..table]
        .chunks_exact(RECORD)
        .map(|c| {
            let mut r: Record = [0; RECORD];
            r.copy_from_slice(c);
            r
        })
        .collect();
    let rest = &bytes[table..];
    let mut out = Container {
        records,
        strings: Vec::new(),
        version,
        extra_bytes: 0,
        strings_cut: false,
    };
    if rest.len() < 8 || rest[..4] != STRINGS_MARK {
        out.extra_bytes = rest.len();
        return Ok(out);
    }
    let announced = u32::from_le_bytes([rest[4], rest[5], rest[6], rest[7]]) as usize;
    let available = rest.len() - 8;
    out.strings_cut = announced > available;
    let length = announced.min(available);
    let blob = &rest[8..8 + length];
    // the zero that ends the list
    let blob = blob.strip_suffix(&[0]).unwrap_or(blob);
    let mut parts = blob.split(|&b| b == 0).peekable();
    while let Some(part) = parts.next() {
        // what follows the last zero is not a string
        if parts.peek().is_none() {
            break;
        }
        if out.strings.len() >= limits.max_strings {
            return Err(BinError::new(codes::LIMIT, "too many long strings"));
        }
        if part.len() > limits.max_string_bytes {
            return Err(BinError::new(codes::LIMIT, "a long string is too long"));
        }
        out.strings.push(part.to_vec());
    }
    out.extra_bytes = (available - length).saturating_sub(TAIL);
    Ok(out)
}
