// SPDX-License-Identifier: GPL-3.0-or-later
//! A bounded reader of the archive of an ODB++ job (`.tgz`, `.tar.gz` or a plain `.tar`), for the
//! basic subset only: it lists the entries, finds the steps and reads the matrix of the job with
//! [`crate::odb::parse_matrix`]. Nothing is written to disk: the archive is read in memory.
//!
//! An archive from outside is hostile until shown otherwise, so the reader is bounded and strict:
//! the size of the archive, the entries, the size of one entry, the size of all of them once
//! unpacked, how much the unpacking may grow compared with the packed size, how deep a path goes
//! and how long a name is are all limits; the CRC-32 and the size that the gzip trailer declares
//! must be right; and these are refused by name, none of them skipped: a path that is empty,
//! absolute, with a drive letter, with a backslash, with `..` or `.` parts, with control
//! characters or a NUL; the same path twice (in any case); a link of any kind (symbolic or hard),
//! a device, a pipe and every entry type but files and folders; the extension headers of PAX and
//! GNU (long names), which the basic subset does not read; a header whose checksum is wrong; a size
//! that is not plain octal; and an archive that is cut. The refusals use the code `PHOENIX-GBR-020`
//! (the format is not valid or not supported); a limit that is reached uses the codes of the limits
//! and of the size of the input that the other readers of this crate use.
//!
//! Not read: the features of the layers, the components, the nets and every other file of the job.

use eh3d_phoenix_ipc2581::zipread::{crc32, inflate};

use crate::codes::{GbrError, INPUT_TOO_LARGE, LIMIT_EXCEEDED, ODB_UNSUPPORTED};
use crate::odb::{parse_matrix, OdbLimits, OdbMatrix};

/// The limits of the reader of the archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    /// The most bytes of the archive as it is given.
    pub max_input_bytes: usize,
    /// The most entries.
    pub max_entries: usize,
    /// The most bytes of one entry.
    pub max_entry_bytes: usize,
    /// The most bytes of all the entries (and of the unpacked archive).
    pub max_total_bytes: usize,
    /// The most the unpacked archive may be, as a multiple of the packed one.
    pub max_ratio: usize,
    /// The most parts of a path (`job/steps/pcb` has three).
    pub max_depth: usize,
    /// The most bytes of a path.
    pub max_name_bytes: usize,
}

impl Default for ArchiveLimits {
    fn default() -> ArchiveLimits {
        ArchiveLimits {
            max_input_bytes: 256 << 20,
            max_entries: 100_000,
            max_entry_bytes: 256 << 20,
            max_total_bytes: 1 << 30,
            max_ratio: 200,
            max_depth: 12,
            max_name_bytes: 255,
        }
    }
}

/// An entry of the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    /// The path, with `/`.
    pub path: String,
    /// The size in bytes (0 for a folder).
    pub size: u64,
    /// True for a folder.
    pub folder: bool,
}

/// What is read of the archive of a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OdbJob {
    /// The folder the matrix is in (the first part of its path), empty when it is at the top.
    pub root: String,
    /// Every entry, in the order of the archive.
    pub entries: Vec<ArchiveEntry>,
    /// The steps found under `steps/`, sorted.
    pub steps: Vec<String>,
    /// The matrix of the job.
    pub matrix: OdbMatrix,
}

fn refuse(detail: impl Into<String>) -> GbrError {
    GbrError::new(ODB_UNSUPPORTED, detail)
}

fn limit(detail: impl Into<String>) -> GbrError {
    GbrError::new(LIMIT_EXCEEDED, detail)
}

/// The data of a gzip file, unpacked in memory within the limits.
fn gunzip(bytes: &[u8], limits: &ArchiveLimits) -> Result<Vec<u8>, GbrError> {
    if bytes.len() < 18 || bytes[2] != 8 {
        return Err(refuse(
            "the gzip header is not valid or the method is not deflate",
        ));
    }
    let flags = bytes[3];
    if flags & 0xe0 != 0 {
        return Err(refuse("the gzip header has reserved flags"));
    }
    let mut at = 10;
    if flags & 0x04 != 0 {
        let n = bytes
            .get(at..at + 2)
            .ok_or_else(|| refuse("the gzip header is cut"))?;
        at += 2 + u16::from_le_bytes([n[0], n[1]]) as usize;
    }
    for flag in [0x08, 0x10] {
        if flags & flag != 0 {
            let end = bytes
                .get(at..)
                .and_then(|rest| rest.iter().position(|b| *b == 0))
                .ok_or_else(|| refuse("the gzip header is cut"))?;
            at += end + 1;
        }
    }
    if flags & 0x02 != 0 {
        at += 2;
    }
    if at + 8 > bytes.len() {
        return Err(refuse("the gzip file is cut"));
    }
    let (data, trailer) = bytes[at..].split_at(bytes.len() - at - 8);
    let crc = u32::from_le_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
    let size = u32::from_le_bytes([trailer[4], trailer[5], trailer[6], trailer[7]]);
    let cap = limits
        .max_total_bytes
        .min(bytes.len().saturating_mul(limits.max_ratio));
    let out = inflate(data, cap).map_err(|e| limit(format!("the gzip data: {e}")))?;
    if out.len() as u32 != size || crc32(&out) != crc {
        return Err(refuse(
            "the size or the CRC-32 of the gzip trailer is wrong",
        ));
    }
    Ok(out)
}

fn octal(field: &[u8]) -> Option<u64> {
    let text: Vec<u8> = field
        .iter()
        .copied()
        .take_while(|b| *b != 0)
        .collect::<Vec<u8>>();
    let text = std::str::from_utf8(&text).ok()?.trim_matches(' ');
    if text.is_empty() {
        return Some(0);
    }
    if !text.bytes().all(|b| (b'0'..=b'7').contains(&b)) || text.len() > 11 {
        return None;
    }
    u64::from_str_radix(text, 8).ok()
}

fn text_field(field: &[u8]) -> Result<String, GbrError> {
    let end = field.iter().position(|b| *b == 0).unwrap_or(field.len());
    String::from_utf8(field[..end].to_vec()).map_err(|_| refuse("a path is not UTF-8"))
}

/// True when the path is one the reader accepts; says why not otherwise.
fn check_path(path: &str, limits: &ArchiveLimits) -> Result<(), String> {
    if path.is_empty() {
        return Err("an empty path".to_owned());
    }
    if path.len() > limits.max_name_bytes {
        return Err(format!("a path of {} bytes", path.len()));
    }
    if path.starts_with('/') {
        return Err(format!("the absolute path {path}"));
    }
    if path.contains('\\') {
        return Err(format!("the path {path} has a backslash"));
    }
    if path.chars().any(char::is_control) {
        return Err("a path with control characters".to_owned());
    }
    let trimmed = path.trim_end_matches('/');
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts
        .iter()
        .any(|p| p.is_empty() || *p == "." || *p == "..")
    {
        return Err(format!("the path {path} has an empty, . or .. part"));
    }
    if parts[0].len() == 2 && parts[0].ends_with(':') {
        return Err(format!("the path {path} starts with a drive letter"));
    }
    if parts.len() > limits.max_depth {
        return Err(format!("the path {path} is {} parts deep", parts.len()));
    }
    Ok(())
}

/// Read the tar entries of `data`: the list and the byte range of each file.
fn read_tar(
    data: &[u8],
    limits: &ArchiveLimits,
) -> Result<Vec<(ArchiveEntry, std::ops::Range<usize>)>, GbrError> {
    let mut out: Vec<(ArchiveEntry, std::ops::Range<usize>)> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut total: u64 = 0;
    let mut at = 0usize;
    while at + 512 <= data.len() {
        let header = &data[at..at + 512];
        if header.iter().all(|b| *b == 0) {
            return Ok(out);
        }
        // the checksum counts the field of the checksum as spaces
        let sum: u64 = header
            .iter()
            .enumerate()
            .map(|(i, b)| {
                if (148..156).contains(&i) {
                    32
                } else {
                    u64::from(*b)
                }
            })
            .sum();
        if octal(&header[148..156]) != Some(sum) {
            return Err(refuse(format!(
                "the checksum of the header at byte {at} is wrong"
            )));
        }
        let mut path = text_field(&header[0..100])?;
        if &header[257..262] == b"ustar" {
            let prefix = text_field(&header[345..500])?;
            if !prefix.is_empty() {
                path = format!("{prefix}/{path}");
            }
        }
        let size = octal(&header[124..136])
            .ok_or_else(|| refuse(format!("the size of {path} is not plain octal")))?;
        let folder = match header[156] {
            b'0' | 0 => false,
            b'5' => true,
            b'1' | b'2' => return Err(refuse(format!("{path} is a link"))),
            b'x' | b'g' | b'L' | b'K' => {
                return Err(refuse(format!(
                    "{path}: the extension headers are not read"
                )))
            }
            other => {
                return Err(refuse(format!(
                    "{path} is of the type {:?}, which is not a file or a folder",
                    other as char
                )))
            }
        };
        check_path(&path, limits).map_err(refuse)?;
        let key = path.trim_end_matches('/').to_lowercase();
        if !seen.insert(key) {
            return Err(refuse(format!("the path {path} is in the archive twice")));
        }
        if out.len() >= limits.max_entries {
            return Err(limit(format!("more than {} entries", limits.max_entries)));
        }
        if folder && size != 0 {
            return Err(refuse(format!("the folder {path} has data")));
        }
        if size > limits.max_entry_bytes as u64 {
            return Err(GbrError::new(
                INPUT_TOO_LARGE,
                format!(
                    "{path} has {size} bytes, the limit is {}",
                    limits.max_entry_bytes
                ),
            ));
        }
        total += size;
        if total > limits.max_total_bytes as u64 {
            return Err(limit(format!(
                "the entries hold more than {} bytes",
                limits.max_total_bytes
            )));
        }
        let start = at + 512;
        let end = start
            .checked_add(size as usize)
            .filter(|e| *e <= data.len())
            .ok_or_else(|| refuse(format!("the archive is cut inside {path}")))?;
        out.push((
            ArchiveEntry {
                path: path.trim_end_matches('/').to_owned(),
                size,
                folder,
            },
            start..end,
        ));
        at = start + (size as usize).div_ceil(512) * 512;
    }
    // no end block: accepted only when the data ended exactly after an entry
    if at == data.len() {
        Ok(out)
    } else {
        Err(refuse("the archive is cut"))
    }
}

/// Read the archive of an ODB++ job. `bytes` is a `.tgz`/`.tar.gz` (told by its first two bytes) or a
/// plain tar; a ZIP archive and anything else is refused.
pub fn read_odb_archive(bytes: &[u8], limits: &ArchiveLimits) -> Result<OdbJob, GbrError> {
    if bytes.len() > limits.max_input_bytes {
        return Err(GbrError::new(
            INPUT_TOO_LARGE,
            format!(
                "{} bytes, the limit is {}",
                bytes.len(),
                limits.max_input_bytes
            ),
        ));
    }
    let unpacked;
    let tar: &[u8] = if bytes.starts_with(&[0x1f, 0x8b]) {
        unpacked = gunzip(bytes, limits)?;
        &unpacked
    } else if bytes.starts_with(b"PK") {
        return Err(refuse("a ZIP archive is not an ODB++ archive"));
    } else {
        bytes
    };
    let entries = read_tar(tar, limits)?;
    let matrix_at = entries
        .iter()
        .position(|(e, _)| {
            !e.folder && (e.path == "matrix/matrix" || e.path.ends_with("/matrix/matrix"))
        })
        .ok_or_else(|| refuse("there is no matrix/matrix in the archive"))?;
    let (matrix_entry, range) = &entries[matrix_at];
    let root = matrix_entry
        .path
        .strip_suffix("matrix/matrix")
        .unwrap_or("")
        .trim_end_matches('/')
        .to_owned();
    let matrix = parse_matrix(&tar[range.clone()], &OdbLimits::default())?;
    let prefix = if root.is_empty() {
        "steps/".to_owned()
    } else {
        format!("{root}/steps/")
    };
    let mut steps: Vec<String> = entries
        .iter()
        .filter_map(|(e, _)| e.path.strip_prefix(prefix.as_str()))
        .filter_map(|rest| rest.split('/').next())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    steps.sort();
    steps.dedup();
    Ok(OdbJob {
        root,
        entries: entries.into_iter().map(|(e, _)| e).collect(),
        steps,
        matrix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATRIX: &str = "STEP {\n   COL=1\n   NAME=pcb\n}\nLAYER {\n   ROW=1\n   CONTEXT=BOARD\n   TYPE=SIGNAL\n   NAME=top\n   POLARITY=POSITIVE\n}\nLAYER {\n   ROW=2\n   CONTEXT=BOARD\n   TYPE=DRILL\n   NAME=drill\n   POLARITY=POSITIVE\n}\n";

    fn header(path: &str, size: usize, kind: u8) -> [u8; 512] {
        let mut h = [0u8; 512];
        h[..path.len()].copy_from_slice(path.as_bytes());
        h[100..107].copy_from_slice(b"0000644");
        h[124..135].copy_from_slice(format!("{size:011o}").as_bytes());
        h[156] = kind;
        h[257..262].copy_from_slice(b"ustar");
        h[148..156].copy_from_slice(b"        ");
        let sum: u64 = h.iter().map(|b| u64::from(*b)).sum();
        h[148..155].copy_from_slice(format!("{sum:07o}").as_bytes());
        h[155] = 0;
        h
    }

    fn tar(entries: &[(&str, &[u8], u8)]) -> Vec<u8> {
        let mut out = Vec::new();
        for (path, data, kind) in entries {
            out.extend(header(path, data.len(), *kind));
            out.extend(*data);
            out.resize(out.len().div_ceil(512) * 512, 0);
        }
        out.extend([0u8; 1024]);
        out
    }

    fn gzip_stored(data: &[u8]) -> Vec<u8> {
        let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
        let mut chunks: Vec<&[u8]> = data.chunks(65_535).collect();
        if chunks.is_empty() {
            chunks.push(&[]);
        }
        let last = chunks.len() - 1;
        for (i, chunk) in chunks.iter().enumerate() {
            out.push(u8::from(i == last));
            out.extend((chunk.len() as u16).to_le_bytes());
            out.extend((!(chunk.len() as u16)).to_le_bytes());
            out.extend(*chunk);
        }
        out.extend(crc32(data).to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out
    }

    fn job() -> Vec<u8> {
        tar(&[
            ("job/", b"", b'5'),
            ("job/matrix/", b"", b'5'),
            ("job/matrix/matrix", MATRIX.as_bytes(), b'0'),
            ("job/steps/pcb/layers/top/features", b"#\n", b'0'),
            ("job/steps/panel/stephdr", b"x", b'0'),
        ])
    }

    #[test]
    fn a_tgz_and_a_plain_tar_of_a_job_give_the_steps_the_layers_and_the_entries() {
        for bytes in [gzip_stored(&job()), job()] {
            let job = read_odb_archive(&bytes, &ArchiveLimits::default()).unwrap();
            assert_eq!(job.root, "job");
            assert_eq!(job.steps, ["panel", "pcb"]);
            assert_eq!(job.matrix.layers.len(), 2);
            assert_eq!(job.entries.len(), 5);
            assert!(job.entries[0].folder);
        }
    }

    #[test]
    fn a_job_with_the_matrix_at_the_top_has_no_root() {
        let bytes = tar(&[
            ("matrix/matrix", MATRIX.as_bytes(), b'0'),
            ("steps/pcb/x", b"1", b'0'),
        ]);
        let job = read_odb_archive(&bytes, &ArchiveLimits::default()).unwrap();
        assert_eq!(
            (job.root.as_str(), job.steps.as_slice()),
            ("", ["pcb".to_owned()].as_slice())
        );
    }

    fn refused(entries: &[(&str, &[u8], u8)]) -> String {
        let mut all = vec![("job/matrix/matrix", MATRIX.as_bytes(), b'0')];
        all.extend_from_slice(entries);
        let err = read_odb_archive(&tar(&all), &ArchiveLimits::default()).unwrap_err();
        assert_eq!(err.code, ODB_UNSUPPORTED);
        err.detail
    }

    #[test]
    fn dangerous_paths_and_entry_types_are_refused_by_name() {
        assert!(refused(&[("../evil", b"x", b'0')]).contains(".."));
        assert!(refused(&[("job/../evil", b"x", b'0')]).contains(".."));
        assert!(refused(&[("/etc/passwd", b"x", b'0')]).contains("absolute"));
        assert!(refused(&[("C:/x", b"x", b'0')]).contains("drive"));
        assert!(refused(&[("job\\x", b"x", b'0')]).contains("backslash"));
        assert!(refused(&[("job/./x", b"x", b'0')]).contains(". or .."));
        assert!(refused(&[("job/link", b"", b'2')]).contains("link"));
        assert!(refused(&[("job/hard", b"", b'1')]).contains("link"));
        assert!(refused(&[("job/dev", b"", b'3')]).contains("not a file or a folder"));
        assert!(refused(&[("job/pax", b"x", b'x')]).contains("extension headers"));
        assert!(refused(&[("JOB/MATRIX/MATRIX", b"x", b'0')]).contains("twice"));
        let deep = format!("job/{}", vec!["a"; 20].join("/"));
        assert!(refused(&[(deep.as_str(), b"x", b'0')]).contains("deep"));
    }

    #[test]
    fn the_limits_stop_too_many_entries_a_big_entry_and_a_big_archive() {
        let bytes = job();
        let few = ArchiveLimits {
            max_entries: 2,
            ..ArchiveLimits::default()
        };
        assert_eq!(
            read_odb_archive(&bytes, &few).unwrap_err().code,
            LIMIT_EXCEEDED
        );
        let small = ArchiveLimits {
            max_entry_bytes: 10,
            ..ArchiveLimits::default()
        };
        assert_eq!(
            read_odb_archive(&bytes, &small).unwrap_err().code,
            INPUT_TOO_LARGE
        );
        let tiny = ArchiveLimits {
            max_input_bytes: 100,
            ..ArchiveLimits::default()
        };
        assert_eq!(
            read_odb_archive(&bytes, &tiny).unwrap_err().code,
            INPUT_TOO_LARGE
        );
        let total = ArchiveLimits {
            max_total_bytes: MATRIX.len(),
            ..ArchiveLimits::default()
        };
        assert_eq!(
            read_odb_archive(&bytes, &total).unwrap_err().code,
            LIMIT_EXCEEDED
        );
    }

    /// A deflate stream (fixed codes) of a zero byte followed by `matches` copies of 258 bytes.
    fn zeros_deflate(matches: usize) -> Vec<u8> {
        struct Bits(Vec<u8>, u32);
        impl Bits {
            fn put(&mut self, value: u32, count: u32, msb_first: bool) {
                for i in 0..count {
                    let bit = if msb_first {
                        (value >> (count - 1 - i)) & 1
                    } else {
                        (value >> i) & 1
                    };
                    if self.1.is_multiple_of(8) {
                        self.0.push(0);
                    }
                    let last = self.0.len() - 1;
                    self.0[last] |= (bit as u8) << (self.1 % 8);
                    self.1 += 1;
                }
            }
        }
        let mut b = Bits(Vec::new(), 0);
        b.put(1, 1, false);
        b.put(1, 2, false);
        b.put(0b0011_0000, 8, true);
        for _ in 0..matches {
            b.put(0b1100_0101, 8, true);
            b.put(0, 5, true);
        }
        b.put(0, 7, true);
        b.0
    }

    #[test]
    fn a_gzip_bomb_is_stopped_by_the_ratio_before_it_is_unpacked() {
        let mut gz = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
        gz.extend(zeros_deflate(40_000));
        // the trailer is wrong on purpose: the ratio stops the work before it is looked at
        gz.extend([0u8; 8]);
        // the stream grows 160 times, so the default ratio of 200 would let it through to the trailer
        let strict = ArchiveLimits {
            max_ratio: 20,
            ..ArchiveLimits::default()
        };
        let err = read_odb_archive(&gz, &strict).unwrap_err();
        assert_eq!(err.code, LIMIT_EXCEEDED, "{}", err.detail);
        // a small stream within the ratio unpacks, but its trailer is checked
        let mut small = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
        small.extend(zeros_deflate(2));
        small.extend([0u8; 8]);
        let err = read_odb_archive(&small, &ArchiveLimits::default()).unwrap_err();
        assert_eq!(err.code, ODB_UNSUPPORTED);
    }

    #[test]
    fn damaged_archives_are_refused_never_read_on() {
        let good = job();
        let mut bad_sum = good.clone();
        bad_sum[0] ^= 1;
        assert!(read_odb_archive(&bad_sum, &ArchiveLimits::default())
            .unwrap_err()
            .detail
            .contains("checksum"));
        let cut = &good[..700];
        assert!(read_odb_archive(cut, &ArchiveLimits::default()).is_err());
        let mut gz = gzip_stored(&good);
        let at = gz.len() - 6;
        gz[at] ^= 0xff;
        assert!(read_odb_archive(&gz, &ArchiveLimits::default())
            .unwrap_err()
            .detail
            .contains("CRC"));
        assert!(read_odb_archive(b"PK\x03\x04", &ArchiveLimits::default())
            .unwrap_err()
            .detail
            .contains("ZIP"));
        assert!(read_odb_archive(b"", &ArchiveLimits::default()).is_err());
        let no_matrix = tar(&[("job/steps/pcb/x", b"1", b'0')]);
        assert!(read_odb_archive(&no_matrix, &ArchiveLimits::default())
            .unwrap_err()
            .detail
            .contains("no matrix"));
        // a matrix that the reader of the matrix refuses is refused with its code
        let bad = tar(&[("job/matrix/matrix", b"LAYER {\n", b'0')]);
        assert_eq!(
            read_odb_archive(&bad, &ArchiveLimits::default())
                .unwrap_err()
                .code,
            ODB_UNSUPPORTED
        );
    }

    #[test]
    fn random_damage_never_panics() {
        let base = gzip_stored(&job());
        let mut seed: u64 = 7;
        for _ in 0..400 {
            let mut copy = base.clone();
            for _ in 0..3 {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let at = (seed >> 33) as usize % copy.len();
                copy[at] ^= (seed >> 20) as u8;
            }
            let _ = read_odb_archive(&copy, &ArchiveLimits::default());
        }
    }
}
