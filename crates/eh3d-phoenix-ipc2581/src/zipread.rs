// SPDX-License-Identifier: GPL-3.0-or-later
//! A reader of ZIP archives that is safe for files from other people.
//!
//! Sample files of IPC-2581 usually come in a ZIP archive. This reader lists the entries from the
//! central directory and unpacks one entry at a time **in memory**: nothing is ever written to a
//! folder, so a name such as `../../x` cannot leave one; such a name is refused all the same, as
//! are absolute names, drive letters, backslashes, empty names and the same name twice (the two
//! readers of an archive that disagree about a repeated name are a known trick). It reads the
//! methods *stored* and *deflate*; encryption, ZIP64, several disks and every other method are
//! refused by name. The limits come from the settings: the entries, the bytes of all of them once
//! unpacked, the bytes of one and how much an entry may grow compared with its packed size. The
//! size and the CRC-32 that the archive declares are checked on every entry that is unpacked.

use crate::codes::{
    Ipc2581Error, READ_NOT_IPC2581, READ_NOT_XML, READ_ZIP_INVALID, READ_ZIP_NO_IPC2581,
};
use crate::read::{read_bytes, ReadOptions};
use crate::read_model::IpcDoc;
use crate::settings::Ipc2581Settings;

/// What the reader accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZipLimits {
    /// The most entries the archive may list.
    pub max_entries: usize,
    /// The most bytes all the entries may hold once unpacked.
    pub max_total_bytes: u64,
    /// The most bytes one entry may hold once unpacked.
    pub max_entry_bytes: u64,
    /// The most an entry may grow, as a multiple of its packed size.
    pub max_ratio: u64,
}

impl From<&Ipc2581Settings> for ZipLimits {
    fn from(s: &Ipc2581Settings) -> ZipLimits {
        ZipLimits {
            max_entries: s.zip_max_entries,
            max_total_bytes: s.zip_max_total_bytes as u64,
            max_entry_bytes: s.read_max_bytes as u64,
            max_ratio: s.zip_max_ratio as u64,
        }
    }
}

impl Default for ZipLimits {
    fn default() -> Self {
        ZipLimits {
            max_entries: 10_000,
            max_total_bytes: 1 << 30,
            max_entry_bytes: 128 << 20,
            max_ratio: 1100,
        }
    }
}

/// One entry of the central directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntry {
    /// The name as written in the archive.
    pub name: String,
    /// The bytes once unpacked, as the archive declares.
    pub size: u64,
    /// The bytes as stored in the archive.
    pub packed: u64,
    method: u16,
    crc32: u32,
    data_at: usize,
}

impl ZipEntry {
    /// True for a folder (a name that ends with a slash).
    pub fn is_folder(&self) -> bool {
        self.name.ends_with('/')
    }
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// True when the name of an entry stays inside a folder it is unpacked to: it is not empty, not
/// absolute, has no drive letter, no backslash, no control character, no `..` part and is not
/// absurdly long.
pub fn safe_entry_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 1024 {
        return false;
    }
    if name.starts_with('/') || name.contains('\\') || name.contains(':') {
        return false;
    }
    if name.chars().any(|c| c.is_control()) {
        return false;
    }
    !name.split('/').any(|part| part == "..")
}

/// Lists the entries of an archive. Every refusal says what it is about.
pub fn list(bytes: &[u8], limits: &ZipLimits) -> Result<Vec<ZipEntry>, String> {
    if bytes.len() < 22 {
        return Err("the archive is too short to be a ZIP archive".to_owned());
    }
    let first = bytes.len().saturating_sub(22 + 65_535);
    let mut eocd = None;
    let mut at = bytes.len() - 22;
    loop {
        if u32_at(bytes, at) == Some(0x0605_4b50) {
            eocd = Some(at);
            break;
        }
        if at == first {
            break;
        }
        at -= 1;
    }
    let eocd = eocd.ok_or("no end of central directory: not a ZIP archive")?;
    let disk = u16_at(bytes, eocd + 4).unwrap_or(1);
    let cd_disk = u16_at(bytes, eocd + 6).unwrap_or(1);
    let count = u16_at(bytes, eocd + 10).ok_or("the end record is cut")?;
    let cd_size = u32_at(bytes, eocd + 12).ok_or("the end record is cut")?;
    let cd_offset = u32_at(bytes, eocd + 16).ok_or("the end record is cut")?;
    if disk != 0 || cd_disk != 0 {
        return Err("an archive over several disks is not read".to_owned());
    }
    if count == 0xFFFF || cd_size == 0xFFFF_FFFF || cd_offset == 0xFFFF_FFFF {
        return Err("ZIP64 is not read".to_owned());
    }
    if usize::from(count) > limits.max_entries {
        return Err(format!(
            "the archive lists {count} entries and the limit is {}",
            limits.max_entries
        ));
    }
    let cd_offset = cd_offset as usize;
    let cd_end = cd_offset
        .checked_add(cd_size as usize)
        .filter(|e| *e <= eocd)
        .ok_or("the central directory is not inside the archive")?;
    let mut entries: Vec<ZipEntry> = Vec::new();
    let mut at = cd_offset;
    let mut total: u64 = 0;
    for _ in 0..count {
        if at.checked_add(46).is_none_or(|e| e > cd_end) || u32_at(bytes, at) != Some(0x0201_4b50) {
            return Err("the central directory is damaged".to_owned());
        }
        let flags = u16_at(bytes, at + 8).unwrap_or(0);
        let method = u16_at(bytes, at + 10).unwrap_or(0xFFFF);
        let crc32 = u32_at(bytes, at + 16).unwrap_or(0);
        let packed = u32_at(bytes, at + 20).unwrap_or(0);
        let size = u32_at(bytes, at + 24).unwrap_or(0);
        let name_len = usize::from(u16_at(bytes, at + 28).unwrap_or(0));
        let extra_len = usize::from(u16_at(bytes, at + 30).unwrap_or(0));
        let comment_len = usize::from(u16_at(bytes, at + 32).unwrap_or(0));
        let local = u32_at(bytes, at + 42).unwrap_or(0) as usize;
        let name_bytes = at
            .checked_add(46)
            .and_then(|s| bytes.get(s..s.checked_add(name_len)?))
            .ok_or("an entry name is cut")?;
        let name = String::from_utf8_lossy(name_bytes).into_owned();
        at = at
            .checked_add(46 + name_len + extra_len + comment_len)
            .filter(|e| *e <= cd_end)
            .ok_or("the central directory is damaged")?;
        if !safe_entry_name(&name) {
            return Err(format!(
                "the entry name '{}' would leave the folder it is unpacked to",
                name.escape_debug()
            ));
        }
        if entries.iter().any(|e| e.name == name) {
            return Err(format!("the name '{name}' appears twice"));
        }
        if flags & 1 != 0 {
            return Err(format!("the entry '{name}' is encrypted"));
        }
        if size == 0xFFFF_FFFF || packed == 0xFFFF_FFFF {
            return Err("ZIP64 is not read".to_owned());
        }
        if method != 0 && method != 8 {
            return Err(format!(
                "the entry '{name}' uses the method {method}; only stored and deflate are read"
            ));
        }
        if method == 0 && size != packed {
            return Err(format!("the sizes of the stored entry '{name}' differ"));
        }
        let size = u64::from(size);
        let packed = u64::from(packed);
        if size > limits.max_entry_bytes {
            return Err(format!(
                "the entry '{name}' holds {size} bytes and the limit of one entry is {}",
                limits.max_entry_bytes
            ));
        }
        if size > packed.max(1).saturating_mul(limits.max_ratio) {
            return Err(format!(
                "the entry '{name}' grows more than {} times when unpacked",
                limits.max_ratio
            ));
        }
        total = total.saturating_add(size);
        if total > limits.max_total_bytes {
            return Err(format!(
                "the entries hold more than {} bytes once unpacked",
                limits.max_total_bytes
            ));
        }
        if u32_at(bytes, local) != Some(0x0403_4b50) {
            return Err(format!("the local header of '{name}' is damaged"));
        }
        let local_name = usize::from(u16_at(bytes, local + 26).unwrap_or(0));
        let local_extra = usize::from(u16_at(bytes, local + 28).unwrap_or(0));
        let data_at = local
            .checked_add(30 + local_name + local_extra)
            .ok_or("an offset is out of range")?;
        let data_end = data_at
            .checked_add(packed as usize)
            .filter(|e| *e <= cd_offset)
            .ok_or_else(|| format!("the data of '{name}' is not inside the archive"))?;
        let _ = data_end;
        entries.push(ZipEntry {
            name,
            size,
            packed,
            method,
            crc32,
            data_at,
        });
    }
    Ok(entries)
}

/// Unpacks one entry in memory. The size and the CRC-32 must be the ones the archive declares.
pub fn unpack(bytes: &[u8], entry: &ZipEntry, limits: &ZipLimits) -> Result<Vec<u8>, String> {
    let packed = bytes
        .get(entry.data_at..entry.data_at.saturating_add(entry.packed as usize))
        .ok_or_else(|| format!("the data of '{}' is cut", entry.name))?;
    let out = if entry.method == 0 {
        packed.to_vec()
    } else {
        let cap = entry.size.min(limits.max_entry_bytes) as usize;
        inflate(packed, cap).map_err(|e| format!("'{}': {e}", entry.name))?
    };
    if out.len() as u64 != entry.size {
        return Err(format!(
            "'{}' unpacks to {} bytes and the archive says {}",
            entry.name,
            out.len(),
            entry.size
        ));
    }
    if crc32(&out) != entry.crc32 {
        return Err(format!("the CRC-32 of '{}' does not match", entry.name));
    }
    Ok(out)
}

/// True for the bytes that start a ZIP archive (the signature of a local header, or of an empty
/// archive).
pub fn looks_like_zip(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x50, 0x4b, 0x03, 0x04]) || bytes.starts_with(&[0x50, 0x4b, 0x05, 0x06])
}

fn is_candidate(entry: &ZipEntry) -> bool {
    if entry.is_folder() || entry.name.starts_with("__MACOSX/") || entry.name.contains("/._") {
        return false;
    }
    let lower = entry.name.to_ascii_lowercase();
    ["xml", "cvg", "ipc2581"]
        .iter()
        .any(|ext| lower.rsplit('.').next() == Some(ext) && lower.contains('.'))
}

/// How many archives may be inside archives (the sample sets of some tools hold the file in a
/// ZIP archive inside a ZIP archive).
const MAX_NESTING: usize = 2;

/// Reads the IPC-2581 file inside an archive: the candidates (names ending `.xml`, `.cvg` or
/// `.ipc2581`) from the largest to the smallest are unpacked one at a time in memory, and the first
/// that reads as an IPC-2581 file is returned with its name. An entry that is XML of another kind
/// is passed over; any other failure of the reader is the answer. When no entry reads, the
/// archives inside the archive are tried the same way (up to two levels; the name then has the
/// form `outer.zip!/inner/name.xml`).
pub fn read_archive(
    bytes: &[u8],
    options: &ReadOptions,
    limits: &ZipLimits,
) -> Result<(String, IpcDoc), Ipc2581Error> {
    read_nested(bytes, options, limits, 0)
}

fn read_nested(
    bytes: &[u8],
    options: &ReadOptions,
    limits: &ZipLimits,
    depth: usize,
) -> Result<(String, IpcDoc), Ipc2581Error> {
    let entries = list(bytes, limits).map_err(|e| Ipc2581Error::new(READ_ZIP_INVALID, e))?;
    let mut candidates: Vec<&ZipEntry> = entries.iter().filter(|e| is_candidate(e)).collect();
    candidates.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    for entry in candidates {
        let data =
            unpack(bytes, entry, limits).map_err(|e| Ipc2581Error::new(READ_ZIP_INVALID, e))?;
        match read_bytes(&data, options) {
            Ok(doc) => return Ok((entry.name.clone(), doc)),
            Err(e) if e.code == READ_NOT_IPC2581 || e.code == READ_NOT_XML => continue,
            Err(e) => return Err(e),
        }
    }
    if depth + 1 < MAX_NESTING {
        let mut inner: Vec<&ZipEntry> = entries
            .iter()
            .filter(|e| !e.is_folder() && e.name.to_ascii_lowercase().ends_with(".zip"))
            .collect();
        inner.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
        for entry in inner {
            let data =
                unpack(bytes, entry, limits).map_err(|e| Ipc2581Error::new(READ_ZIP_INVALID, e))?;
            match read_nested(&data, options, limits, depth + 1) {
                Ok((name, doc)) => return Ok((format!("{}!/{name}", entry.name), doc)),
                Err(e) if e.code == READ_ZIP_NO_IPC2581 => continue,
                Err(e) => return Err(e),
            }
        }
    }
    Err(Ipc2581Error::new(
        READ_ZIP_NO_IPC2581,
        format!("{} entries, none of them an IPC-2581 file", entries.len()),
    ))
}

/// The CRC-32 of the ZIP format (the polynomial 0xEDB88320).
pub fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for b in data {
        crc = table[((crc ^ u32::from(*b)) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    buf: u32,
    count: u32,
}

impl Bits<'_> {
    fn need(&mut self, n: u32) -> Result<u32, String> {
        while self.count < n {
            let byte = *self
                .data
                .get(self.pos)
                .ok_or("the deflate stream ends too soon")?;
            self.pos += 1;
            self.buf |= u32::from(byte) << self.count;
            self.count += 8;
        }
        let v = self.buf & ((1u32 << n) - 1);
        self.buf = if n >= 32 { 0 } else { self.buf >> n };
        self.count -= n;
        Ok(v)
    }
}

struct Huffman {
    count: [u16; 16],
    symbol: Vec<u16>,
}

fn huffman(lengths: &[u8]) -> Result<Huffman, String> {
    let mut count = [0u16; 16];
    for l in lengths {
        count[usize::from(*l)] += 1;
    }
    let mut left: i32 = 1;
    for c in &count[1..] {
        left = (left << 1) - i32::from(*c);
        if left < 0 {
            return Err("a Huffman table of the deflate stream is over-subscribed".to_owned());
        }
    }
    let mut offsets = [0u16; 16];
    for len in 1..15 {
        offsets[len + 1] = offsets[len] + count[len];
    }
    let mut symbol = vec![0u16; lengths.len()];
    for (sym, l) in lengths.iter().enumerate() {
        if *l != 0 {
            symbol[usize::from(offsets[usize::from(*l)])] = sym as u16;
            offsets[usize::from(*l)] += 1;
        }
    }
    Ok(Huffman { count, symbol })
}

fn decode(bits: &mut Bits<'_>, h: &Huffman) -> Result<u16, String> {
    let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
    for len in 1..16 {
        code |= bits.need(1)? as i32;
        let count = i32::from(h.count[len]);
        if code - count < first {
            return h
                .symbol
                .get((index + (code - first)) as usize)
                .copied()
                .ok_or_else(|| "a Huffman code is out of range".to_owned());
        }
        index += count;
        first += count;
        first <<= 1;
        code <<= 1;
    }
    Err("a Huffman code is not valid".to_owned())
}

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

fn codes(
    bits: &mut Bits<'_>,
    out: &mut Vec<u8>,
    cap: usize,
    lit: &Huffman,
    dist: &Huffman,
) -> Result<(), String> {
    loop {
        let sym = usize::from(decode(bits, lit)?);
        if sym < 256 {
            if out.len() >= cap {
                return Err("the stream unpacks to more than it declares".to_owned());
            }
            out.push(sym as u8);
        } else if sym == 256 {
            return Ok(());
        } else {
            let s = sym - 257;
            if s >= 29 {
                return Err("a length code is not valid".to_owned());
            }
            let len = usize::from(LEN_BASE[s]) + bits.need(u32::from(LEN_EXTRA[s]))? as usize;
            let d = usize::from(decode(bits, dist)?);
            if d >= 30 {
                return Err("a distance code is not valid".to_owned());
            }
            let distance =
                usize::from(DIST_BASE[d]) + bits.need(u32::from(DIST_EXTRA[d]))? as usize;
            if distance > out.len() {
                return Err("a distance reaches before the start of the data".to_owned());
            }
            if out.len().saturating_add(len) > cap {
                return Err("the stream unpacks to more than it declares".to_owned());
            }
            let from = out.len() - distance;
            for i in 0..len {
                let b = out[from + i];
                out.push(b);
            }
        }
    }
}

/// Unpacks a raw deflate stream; the result may not be longer than `cap`.
pub fn inflate(data: &[u8], cap: usize) -> Result<Vec<u8>, String> {
    let mut bits = Bits {
        data,
        pos: 0,
        buf: 0,
        count: 0,
    };
    let mut out: Vec<u8> = Vec::new();
    loop {
        let last = bits.need(1)?;
        match bits.need(2)? {
            0 => {
                bits.buf = 0;
                bits.count = 0;
                let head = data
                    .get(bits.pos..bits.pos + 4)
                    .ok_or("a stored block is cut")?;
                let len = usize::from(u16::from_le_bytes([head[0], head[1]]));
                let inv = u16::from_le_bytes([head[2], head[3]]);
                if inv != !(len as u16) {
                    return Err("a stored block has a wrong length check".to_owned());
                }
                bits.pos += 4;
                let body = data
                    .get(bits.pos..bits.pos + len)
                    .ok_or("a stored block is cut")?;
                if out.len() + len > cap {
                    return Err("the stream unpacks to more than it declares".to_owned());
                }
                out.extend_from_slice(body);
                bits.pos += len;
            }
            1 => {
                let mut lengths = [8u8; 288];
                lengths[144..256].fill(9);
                lengths[256..280].fill(7);
                let lit = huffman(&lengths)?;
                let dist = huffman(&[5u8; 30])?;
                codes(&mut bits, &mut out, cap, &lit, &dist)?;
            }
            2 => {
                let nlen = bits.need(5)? as usize + 257;
                let ndist = bits.need(5)? as usize + 1;
                let ncode = bits.need(4)? as usize + 4;
                if nlen > 286 || ndist > 30 {
                    return Err("a dynamic block counts too many codes".to_owned());
                }
                const ORDER: [usize; 19] = [
                    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
                ];
                let mut cl = [0u8; 19];
                for slot in ORDER.iter().take(ncode) {
                    cl[*slot] = bits.need(3)? as u8;
                }
                let clh = huffman(&cl)?;
                let mut lengths = vec![0u8; nlen + ndist];
                let mut i = 0;
                while i < nlen + ndist {
                    let sym = decode(&mut bits, &clh)?;
                    if sym < 16 {
                        lengths[i] = sym as u8;
                        i += 1;
                    } else {
                        let (value, repeat) = match sym {
                            16 => {
                                if i == 0 {
                                    return Err("a length repeats before any exists".to_owned());
                                }
                                (lengths[i - 1], 3 + bits.need(2)? as usize)
                            }
                            17 => (0, 3 + bits.need(3)? as usize),
                            _ => (0, 11 + bits.need(7)? as usize),
                        };
                        if i + repeat > nlen + ndist {
                            return Err("the code lengths run past their count".to_owned());
                        }
                        lengths[i..i + repeat].fill(value);
                        i += repeat;
                    }
                }
                if lengths[256] == 0 {
                    return Err("a dynamic block has no end code".to_owned());
                }
                let lit = huffman(&lengths[..nlen])?;
                let dist = huffman(&lengths[nlen..])?;
                codes(&mut bits, &mut out, cap, &lit, &dist)?;
            }
            _ => return Err("a block of the deflate stream has an unknown type".to_owned()),
        }
        if last == 1 {
            return Ok(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes an archive of stored entries; `local_name` lets a test lie about a name.
    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        archive_with(
            entries
                .iter()
                .map(|(n, d)| (*n, 0u16, d.to_vec(), d.len() as u32, crc32(d))),
        )
    }

    fn archive_with<'a>(
        entries: impl Iterator<Item = (&'a str, u16, Vec<u8>, u32, u32)>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        let mut count = 0u16;
        for (name, method, packed, size, crc) in entries {
            let local = out.len() as u32;
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&[20, 0, 0, 0]);
            out.extend_from_slice(&method.to_le_bytes());
            out.extend_from_slice(&[0, 0, 0, 0]);
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&packed);
            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
            central.extend_from_slice(&method.to_le_bytes());
            central.extend_from_slice(&[0, 0, 0, 0]);
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            central.extend_from_slice(&size.to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&[0; 12]);
            central.extend_from_slice(&local.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
            count += 1;
        }
        let cd_at = out.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&(central.len() as u32).to_le_bytes());
        out.extend_from_slice(&cd_at.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out
    }

    /// `"hello hello hello hello\n"` packed by a deflate encoder with the fixed table.
    const FIXED_HELLO: [u8; 11] = [
        0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xc8, 0x40, 0x27, 0xb9, 0x00,
    ];

    // The text below packed by a deflate encoder with a dynamic table.
    const DYN_TEXT_LEN: usize = 2039;
    const DYN_CRC: u32 = 0x650edf42;
    const DYNAMIC: [u8; 344] = [
        141, 149, 75, 110, 132, 48, 24, 131, 247, 61, 69, 142, 192, 255, 8, 129, 246, 14, 189, 3,
        66, 236, 152, 233, 104, 212, 46, 122, 251, 62, 52, 18, 249, 144, 43, 149, 21, 27, 11, 199,
        159, 113, 246, 229, 115, 187, 151, 225, 185, 172, 111, 183, 219, 247, 91, 212, 242, 113,
        121, 41, 215, 237, 189, 188, 14, 101, 221, 183, 229, 190, 92, 215, 173, 216, 240, 243, 148,
        235, 229, 105, 255, 85, 216, 161, 24, 15, 69, 163, 194, 188, 30, 10, 63, 20, 237, 80, 24,
        21, 94, 187, 111, 132, 116, 53, 81, 17, 173, 251, 70, 74, 87, 78, 69, 237, 207, 81, 165,
        171, 153, 138, 177, 63, 199, 40, 93, 5, 21, 173, 63, 71, 147, 174, 236, 20, 239, 212, 31,
        100, 146, 182, 18, 10, 3, 144, 89, 218, 50, 230, 107, 32, 98, 131, 52, 86, 41, 1, 18, 51,
        77, 145, 17, 27, 160, 152, 75, 107, 35, 37, 160, 98, 33, 157, 49, 49, 3, 22, 75, 233, 140,
        141, 52, 112, 177, 170, 51, 163, 4, 92, 108, 148, 198, 216, 73, 231, 159, 210, 164, 49, 38,
        230, 4, 51, 73, 99, 108, 165, 19, 204, 44, 141, 177, 150, 14, 46, 62, 104, 150, 76, 217, 1,
        198, 77, 58, 99, 49, 29, 96, 220, 117, 253, 25, 179, 131, 140, 135, 180, 198, 102, 58, 200,
        120, 106, 152, 204, 57, 128, 198, 171, 180, 198, 102, 6, 87, 108, 148, 206, 152, 89, 0,
        141, 55, 233, 140, 205, 12, 162, 153, 116, 102, 148, 144, 204, 44, 141, 177, 153, 1, 50,
        161, 103, 255, 148, 24, 192, 132, 222, 125, 54, 51, 0, 38, 244, 240, 179, 153, 9, 46, 161,
        151, 255, 212, 204, 4, 152, 208, 219, 207, 102, 38, 239, 23, 61, 254, 167, 102, 38, 200,
        132, 158, 127, 54, 51, 65, 38, 254, 216, 127, 230, 156, 68, 163, 47, 0, 54, 51, 137, 102,
        254, 199, 37, 158, 15, 52, 95,
    ];

    fn dynamic_text() -> Vec<u8> {
        (0..40)
            .flat_map(|i: u32| {
                format!(
                    "layer {i}: copper {} um; net N{} clearance {} nm\n",
                    35 + i % 3,
                    i * 7 % 13,
                    100_000 + i * 125
                )
                .into_bytes()
            })
            .collect()
    }

    #[test]
    fn the_crc_is_the_one_of_the_zip_format() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn stored_entries_are_listed_and_unpacked_in_memory() {
        let zip = archive(&[("a/b.xml", b"<x/>"), ("c.txt", b"text")]);
        let limits = ZipLimits::default();
        let entries = list(&zip, &limits).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "a/b.xml");
        assert_eq!(unpack(&zip, &entries[0], &limits).unwrap(), b"<x/>");
        assert_eq!(unpack(&zip, &entries[1], &limits).unwrap(), b"text");
    }

    #[test]
    fn a_name_that_leaves_the_folder_is_refused() {
        let limits = ZipLimits::default();
        for name in [
            "../evil.xml",
            "a/../../evil.xml",
            "/etc/passwd",
            "C:/x.xml",
            "..\\x.xml",
            "a\\b.xml",
            "",
        ] {
            let zip = archive(&[(name, b"x")]);
            let e = list(&zip, &limits).unwrap_err();
            assert!(e.contains("leave the folder"), "{name:?}: {e}");
        }
        assert!(safe_entry_name("dir/file.xml"));
        assert!(safe_entry_name("a..b/c"));
        assert!(!safe_entry_name("a/\u{1}b"));
    }

    const MINI: &str = r#"<?xml version="1.0"?><IPC-2581 revision="C"><Content roleRef="a"/><LogisticHeader/><HistoryRecord number="1"/><Ecad name="e"><CadHeader units="MILLIMETER"/><CadData><Layer name="TOP" layerFunction="SIGNAL" side="TOP" polarity="POSITIVE"/></CadData></Ecad></IPC-2581>"#;

    #[test]
    fn the_ipc2581_file_is_found_inside_an_archive_and_read() {
        let other = b"<?xml version=\"1.0\"?><notipc>padding padding padding padding padding padding padding</notipc>";
        let zip = archive(&[
            ("docs/readme.txt", b"hello"),
            ("__MACOSX/._board.xml", b"junk"),
            ("big.xml", other),
            ("sub/board.xml", MINI.as_bytes()),
        ]);
        let (name, doc) =
            read_archive(&zip, &ReadOptions::default(), &ZipLimits::default()).unwrap();
        assert_eq!(name, "sub/board.xml");
        assert_eq!(doc.revision, "C");
        assert!(looks_like_zip(&zip));
        assert!(!looks_like_zip(MINI.as_bytes()));
    }

    #[test]
    fn an_archive_without_an_ipc2581_file_or_with_a_bad_one_gives_a_registered_code() {
        let options = ReadOptions::default();
        let limits = ZipLimits::default();
        let none = archive(&[("a.txt", b"hello"), ("b.xml", b"<other/>")]);
        let e = read_archive(&none, &options, &limits).unwrap_err();
        assert_eq!(e.code, READ_ZIP_NO_IPC2581);
        let slip = archive(&[("../x.xml", MINI.as_bytes())]);
        let e = read_archive(&slip, &options, &limits).unwrap_err();
        assert_eq!(e.code, READ_ZIP_INVALID);
        assert!(e.detail.contains("leave the folder"));
        let tiny = ZipLimits {
            max_entry_bytes: 10,
            ..limits
        };
        let big = archive(&[("board.xml", MINI.as_bytes())]);
        assert_eq!(
            read_archive(&big, &options, &tiny).unwrap_err().code,
            READ_ZIP_INVALID
        );
    }

    #[test]
    fn the_limits_come_from_the_settings() {
        let settings = Ipc2581Settings {
            zip_max_entries: 7,
            zip_max_total_bytes: 99,
            zip_max_ratio: 5,
            read_max_bytes: 55,
            ..Ipc2581Settings::default()
        };
        assert_eq!(
            ZipLimits::from(&settings),
            ZipLimits {
                max_entries: 7,
                max_total_bytes: 99,
                max_entry_bytes: 55,
                max_ratio: 5
            }
        );
    }

    #[test]
    fn an_archive_inside_an_archive_is_read_and_a_third_level_is_not() {
        let options = ReadOptions::default();
        let limits = ZipLimits::default();
        let inner = archive(&[("ipc-2581/board.xml", MINI.as_bytes())]);
        let outer = archive(&[("notes.txt", b"x"), ("data/pack.zip", &inner)]);
        let (name, doc) = read_archive(&outer, &options, &limits).unwrap();
        assert_eq!(name, "data/pack.zip!/ipc-2581/board.xml");
        assert_eq!(doc.revision, "C");
        let third = archive(&[("deep.zip", &outer)]);
        assert_eq!(
            read_archive(&third, &options, &limits).unwrap_err().code,
            READ_ZIP_NO_IPC2581
        );
    }

    #[test]
    fn the_same_name_twice_is_refused() {
        let zip = archive(&[("a.xml", b"1"), ("a.xml", b"2")]);
        assert!(list(&zip, &ZipLimits::default())
            .unwrap_err()
            .contains("twice"));
    }

    #[test]
    fn the_limits_are_enforced_and_named() {
        let zip = archive(&[("a", b"12345"), ("b", b"12345"), ("c", b"12345")]);
        let few = ZipLimits {
            max_entries: 2,
            ..ZipLimits::default()
        };
        assert!(list(&zip, &few).unwrap_err().contains("3 entries"));
        let small = ZipLimits {
            max_total_bytes: 12,
            ..ZipLimits::default()
        };
        assert!(list(&zip, &small).unwrap_err().contains("12 bytes"));
        let one = ZipLimits {
            max_entry_bytes: 4,
            ..ZipLimits::default()
        };
        assert!(list(&zip, &one).unwrap_err().contains("limit of one entry"));
    }

    #[test]
    fn a_deflate_entry_is_unpacked_and_its_size_and_crc_are_checked() {
        let text = b"hello hello hello hello\n";
        let ok = archive_with(std::iter::once((
            "h.txt",
            8u16,
            FIXED_HELLO.to_vec(),
            text.len() as u32,
            crc32(text),
        )));
        let limits = ZipLimits::default();
        let entries = list(&ok, &limits).unwrap();
        assert_eq!(unpack(&ok, &entries[0], &limits).unwrap(), text);
        let wrong_crc = archive_with(std::iter::once((
            "h.txt",
            8u16,
            FIXED_HELLO.to_vec(),
            text.len() as u32,
            1,
        )));
        let e = list(&wrong_crc, &limits).unwrap();
        assert!(unpack(&wrong_crc, &e[0], &limits)
            .unwrap_err()
            .contains("CRC-32"));
        let wrong_size = archive_with(std::iter::once((
            "h.txt",
            8u16,
            FIXED_HELLO.to_vec(),
            10,
            crc32(text),
        )));
        let e = list(&wrong_size, &limits).unwrap();
        assert!(unpack(&wrong_size, &e[0], &limits).is_err());
    }

    #[test]
    fn a_dynamic_table_is_read_and_agrees_with_the_text() {
        let text = dynamic_text();
        assert_eq!(text.len(), DYN_TEXT_LEN);
        assert_eq!(crc32(&text), DYN_CRC);
        assert_eq!(inflate(&DYNAMIC, 1 << 20).unwrap(), text);
        assert!(inflate(&DYNAMIC, text.len() - 1).is_err());
        let zip = archive_with(std::iter::once((
            "d.txt",
            8u16,
            DYNAMIC.to_vec(),
            DYN_TEXT_LEN as u32,
            DYN_CRC,
        )));
        let limits = ZipLimits::default();
        let entries = list(&zip, &limits).unwrap();
        assert_eq!(unpack(&zip, &entries[0], &limits).unwrap(), text);
    }

    #[test]
    fn a_bomb_is_stopped_by_the_ratio_before_anything_is_unpacked() {
        // a stream of 3 bytes that says it holds a million
        let zip = archive_with(std::iter::once(("b", 8u16, vec![3, 0, 0], 1_000_000, 0)));
        let e = list(&zip, &ZipLimits::default()).unwrap_err();
        assert!(e.contains("grows more than"), "{e}");
    }

    #[test]
    fn a_stream_longer_than_it_declares_is_cut_at_the_declared_size() {
        assert!(inflate(&FIXED_HELLO, 4).is_err());
        assert_eq!(inflate(&FIXED_HELLO, 24).unwrap().len(), 24);
    }

    #[test]
    fn stored_and_fixed_blocks_are_read() {
        // a stored block "abc", final
        let stored = [1u8, 3, 0, 0xfc, 0xff, b'a', b'b', b'c'];
        assert_eq!(inflate(&stored, 10).unwrap(), b"abc");
        // fixed Huffman block holding "a" and the end code
        let fixed = [0x4b, 0x04, 0x00];
        assert_eq!(inflate(&fixed, 10).unwrap(), b"a");
        // a stored block with a wrong complement
        let bad = [1u8, 3, 0, 0, 0, b'a', b'b', b'c'];
        assert!(inflate(&bad, 10).is_err());
    }

    #[test]
    fn what_this_reader_does_not_read_is_named() {
        let limits = ZipLimits::default();
        let mut zip = archive(&[("a", b"x")]);
        // the method in the central directory becomes 12 (bzip2)
        let cd = zip
            .windows(4)
            .position(|w| w == [0x50, 0x4b, 0x01, 0x02])
            .unwrap();
        zip[cd + 10] = 12;
        assert!(list(&zip, &limits).unwrap_err().contains("method 12"));
        let mut zip = archive(&[("a", b"x")]);
        zip[cd + 8] = 1;
        assert!(list(&zip, &limits).unwrap_err().contains("encrypted"));
        assert!(list(b"not a zip archive at all, no", &limits).is_err());
        assert!(list(&[], &limits).is_err());
    }

    #[test]
    fn damaged_archives_never_panic_with_a_fixed_seed() {
        let base = archive(&[("a/b.xml", b"<x/>"), ("c.txt", b"text text text")]);
        let mut state: u64 = 0x21b_c0de;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as usize
        };
        let limits = ZipLimits::default();
        for _ in 0..3000 {
            let mut bytes = base.clone();
            for _ in 0..(1 + next() % 5) {
                let at = next() % bytes.len();
                match next() % 3 {
                    0 => bytes[at] = (next() % 256) as u8,
                    1 => {
                        bytes.remove(at);
                    }
                    _ => bytes.insert(at, (next() % 256) as u8),
                }
            }
            if let Ok(entries) = list(&bytes, &limits) {
                for e in &entries {
                    let _ = unpack(&bytes, e, &limits);
                }
            }
        }
        // random garbage as a deflate stream
        for _ in 0..3000 {
            let junk: Vec<u8> = (0..(1 + next() % 40))
                .map(|_| (next() % 256) as u8)
                .collect();
            let _ = inflate(&junk, 1 << 16);
        }
    }
}
