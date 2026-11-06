//! A minimal `.zip` writer and reader with no dependency: every entry is stored (method 0, no
//! compression), the CRC-32 is worked out here and the fields that vary between runs are fixed, so
//! the same files always give the same bytes.
//!
//! * the date of every entry is 1980-01-01 00:00:00, the first date a zip file can hold;
//! * the names are UTF-8 (flag bit 11), the entries have no extra field, no comment and no
//!   attributes, so no permission or owner is written;
//! * the entries are written in the order given, the central directory in the same order.
//!
//! The reader accepts only what this writer writes (stored entries, no zip64, no comment at the end)
//! and checks the CRC-32 and the sizes of every entry. The format limits are those of a plain zip
//! file: 65 535 entries, and 4 GiB less one byte for a file or for the whole.

use crate::fabcodes::{FabError, FABPACK_FAILED, FABPACK_MISMATCH, FABPACK_TOO_LARGE};

const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const END_SIGNATURE: u32 = 0x0605_4b50;
/// Version 2.0: stored entries, directories not used.
const VERSION: u16 = 20;
/// Bit 11: the names are UTF-8.
const FLAGS: u16 = 0x0800;
/// 1980-01-01: year 0 since 1980, month 1, day 1.
const DOS_DATE: u16 = (1 << 5) | 1;

const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = crc_table();

/// The CRC-32 (IEEE 802.3, the one of zip) of `bytes`.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for b in bytes {
        c = CRC_TABLE[((c ^ u32::from(*b)) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

/// Checks that a name can be the path of an entry: relative, with `/` as the separator, no empty,
/// `.` or `..` part, no control character and no backslash.
pub fn check_name(name: &str) -> Result<(), FabError> {
    let bad = |why: &str| FabError::new(FABPACK_FAILED, format!("the file name '{name}' {why}"));
    if name.is_empty() || name.len() > 255 {
        return Err(bad("is empty or longer than 255 bytes"));
    }
    if name
        .chars()
        .any(|c| c.is_control() || c == '\\' || c == ':')
    {
        return Err(bad("holds a control character, a backslash or a colon"));
    }
    if name
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(bad("is not a plain relative path"));
    }
    Ok(())
}

fn put16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn too_large(what: &str) -> FabError {
    FabError::new(FABPACK_TOO_LARGE, what.to_owned())
}

/// Writes a zip file with the entries in the order given.
pub fn write_zip(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, FabError> {
    if entries.len() > usize::from(u16::MAX) {
        return Err(too_large("more than 65535 files"));
    }
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    for (name, data) in entries {
        check_name(name)?;
        let size = u32::try_from(data.len())
            .map_err(|_| too_large(&format!("{name} is 4 GiB or more")))?;
        let offset =
            u32::try_from(out.len()).map_err(|_| too_large("the package is 4 GiB or more"))?;
        let crc = crc32(data);
        // the length of a name is at most 255 bytes (checked above)
        let name_len = name.len() as u16;
        put32(&mut out, LOCAL_SIGNATURE);
        put16(&mut out, VERSION);
        put16(&mut out, FLAGS);
        put16(&mut out, 0);
        put16(&mut out, 0);
        put16(&mut out, DOS_DATE);
        put32(&mut out, crc);
        put32(&mut out, size);
        put32(&mut out, size);
        put16(&mut out, name_len);
        put16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);

        put32(&mut central, CENTRAL_SIGNATURE);
        put16(&mut central, VERSION);
        put16(&mut central, VERSION);
        put16(&mut central, FLAGS);
        put16(&mut central, 0);
        put16(&mut central, 0);
        put16(&mut central, DOS_DATE);
        put32(&mut central, crc);
        put32(&mut central, size);
        put32(&mut central, size);
        put16(&mut central, name_len);
        put16(&mut central, 0);
        put16(&mut central, 0);
        put16(&mut central, 0);
        put16(&mut central, 0);
        put32(&mut central, 0);
        put32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset =
        u32::try_from(out.len()).map_err(|_| too_large("the package is 4 GiB or more"))?;
    let central_size =
        u32::try_from(central.len()).map_err(|_| too_large("the package is 4 GiB or more"))?;
    out.extend_from_slice(&central);
    let count = entries.len() as u16;
    put32(&mut out, END_SIGNATURE);
    put16(&mut out, 0);
    put16(&mut out, 0);
    put16(&mut out, count);
    put16(&mut out, count);
    put32(&mut out, central_size);
    put32(&mut out, central_offset);
    put16(&mut out, 0);
    if u32::try_from(out.len()).is_err() {
        return Err(too_large("the package is 4 GiB or more"));
    }
    Ok(out)
}

fn unreadable(what: &str) -> FabError {
    FabError::new(FABPACK_MISMATCH, format!("the zip file {what}"))
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, FabError> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| unreadable("ends too soon"))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, FabError> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| unreadable("ends too soon"))
}

/// Reads a zip file written by [`write_zip`]: the entries in order, each with its name and bytes.
/// Every CRC-32 and size is checked.
pub fn read_zip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, FabError> {
    if bytes.len() < 22 {
        return Err(unreadable("is too short"));
    }
    let end = bytes.len() - 22;
    if u32_at(bytes, end)? != END_SIGNATURE || u16_at(bytes, end + 20)? != 0 {
        return Err(unreadable("has no end record where it should be"));
    }
    let count = usize::from(u16_at(bytes, end + 10)?);
    let central_size = u32_at(bytes, end + 12)? as usize;
    let central_offset = u32_at(bytes, end + 16)? as usize;
    if central_offset.checked_add(central_size) != Some(end) {
        return Err(unreadable("has a central directory out of place"));
    }
    let mut entries = Vec::with_capacity(count);
    let mut at = central_offset;
    for _ in 0..count {
        if u32_at(bytes, at)? != CENTRAL_SIGNATURE {
            return Err(unreadable("has a bad central directory entry"));
        }
        if u16_at(bytes, at + 10)? != 0 {
            return Err(unreadable("holds an entry that is not stored"));
        }
        let crc = u32_at(bytes, at + 16)?;
        let size = u32_at(bytes, at + 24)? as usize;
        if u32_at(bytes, at + 20)? as usize != size {
            return Err(unreadable("holds an entry whose sizes differ"));
        }
        let name_len = usize::from(u16_at(bytes, at + 28)?);
        let skip = usize::from(u16_at(bytes, at + 30)?) + usize::from(u16_at(bytes, at + 32)?);
        let local = u32_at(bytes, at + 42)? as usize;
        let name_bytes = bytes
            .get(at + 46..at + 46 + name_len)
            .ok_or_else(|| unreadable("ends too soon"))?;
        let name = String::from_utf8(name_bytes.to_vec())
            .map_err(|_| unreadable("holds a name that is not UTF-8"))?;
        at += 46 + name_len + skip;
        if u32_at(bytes, local)? != LOCAL_SIGNATURE {
            return Err(unreadable("has a bad local header"));
        }
        let local_name = usize::from(u16_at(bytes, local + 26)?);
        let local_extra = usize::from(u16_at(bytes, local + 28)?);
        let start = local + 30 + local_name + local_extra;
        let data = bytes
            .get(start..start + size)
            .ok_or_else(|| unreadable("ends too soon"))?;
        if crc32(data) != crc {
            return Err(FabError::new(
                FABPACK_MISMATCH,
                format!("the CRC-32 of {name} is wrong"),
            ));
        }
        entries.push((name, data.to_vec()));
    }
    if at != end {
        return Err(unreadable("has bytes after its central directory"));
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, data: &[u8]) -> (String, Vec<u8>) {
        (name.to_owned(), data.to_vec())
    }

    #[test]
    fn the_crc_is_the_one_of_zip() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"a"), 0xE8B7_BE43);
    }

    #[test]
    fn the_entries_are_written_stored_with_the_first_date_of_zip() {
        let zip = write_zip(&[entry("a.txt", b"hello")]).unwrap();
        assert_eq!(&zip[0..4], &[0x50, 0x4B, 0x03, 0x04]);
        // flags: UTF-8 names
        assert_eq!(&zip[6..8], &[0x00, 0x08]);
        // method stored, time 0, date 1980-01-01
        assert_eq!(&zip[8..10], &[0, 0]);
        assert_eq!(&zip[10..12], &[0, 0]);
        assert_eq!(&zip[12..14], &[0x21, 0x00]);
        assert_eq!(&zip[14..18], &crc32(b"hello").to_le_bytes());
        assert_eq!(&zip[18..22], &5u32.to_le_bytes());
        assert_eq!(&zip[30..35], b"a.txt");
        assert_eq!(&zip[35..40], b"hello");
        // the end record is the last 22 bytes and has no comment
        let end = zip.len() - 22;
        assert_eq!(&zip[end..end + 4], &[0x50, 0x4B, 0x05, 0x06]);
        assert_eq!(&zip[end + 20..], &[0, 0]);
        // 30 + 5 + 5 for the entry, 46 + 5 for its central record, 22 for the end
        assert_eq!(zip.len(), 40 + 51 + 22);
    }

    #[test]
    fn a_zip_is_read_back_and_is_the_same_every_time() {
        let files = vec![
            entry("gerber/top.gbr", b"G04*\nM02*\n"),
            entry("empty.txt", b""),
            entry("manifest.json", b"{}\n"),
        ];
        let one = write_zip(&files).unwrap();
        assert_eq!(one, write_zip(&files).unwrap());
        assert_eq!(read_zip(&one).unwrap(), files);
        assert_eq!(read_zip(&write_zip(&[]).unwrap()).unwrap(), Vec::new());
    }

    #[test]
    fn a_damaged_zip_is_refused() {
        let zip = write_zip(&[entry("a.txt", b"hello")]).unwrap();
        let mut bad_data = zip.clone();
        bad_data[35] ^= 1;
        assert_eq!(read_zip(&bad_data).unwrap_err().code, FABPACK_MISMATCH);
        assert!(read_zip(&zip[..zip.len() - 1]).is_err());
        assert!(read_zip(&zip[..10]).is_err());
        let mut bad_end = zip.clone();
        let n = bad_end.len();
        bad_end[n - 22] = 0;
        assert!(read_zip(&bad_end).is_err());
        let mut compressed = zip.clone();
        // the method in the central directory
        let central = 40;
        compressed[central + 10] = 8;
        assert!(read_zip(&compressed).is_err());
    }

    #[test]
    fn only_plain_relative_names_are_written() {
        for bad in [
            "", "/a", "a/", "a//b", "../a", "a/./b", "a\\b", "c:x", "a\nb", "a/../b",
        ] {
            assert!(write_zip(&[entry(bad, b"x")]).is_err(), "{bad:?}");
        }
        assert!(write_zip(&[entry(&"n".repeat(256), b"x")]).is_err());
        assert!(write_zip(&[entry("dir/file name.txt", b"x")]).is_ok());
        assert!(write_zip(&[entry("ñandú.txt", b"x")]).is_ok());
    }
}
