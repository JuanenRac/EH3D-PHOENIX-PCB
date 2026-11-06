// SPDX-License-Identifier: GPL-3.0-or-later
//! The reading of settings files with a size limit, so that a settings file that is a huge file
//! (or a device that never ends) cannot fill the memory.

use std::io::{self, Read};
use std::path::Path;

/// The largest settings file that any command reads, in bytes. The settings files of the
/// repository are a few kilobytes.
pub const MAX_CONFIG_BYTES: u64 = 1024 * 1024;

/// Reads a settings file as text; a file larger than [`MAX_CONFIG_BYTES`] is an error of kind
/// `InvalidData`, and so is one that is not UTF-8. At most one byte more than the limit is read.
pub fn read_config_text(path: impl AsRef<Path>) -> io::Result<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_CONFIG_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}: the file is larger than {MAX_CONFIG_BYTES} bytes",
                crate::sec_codes::SETTINGS_TOO_LARGE
            ),
        ));
    }
    String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_file_is_read_and_a_large_one_or_a_missing_one_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let small = dir.path().join("a.cfg");
        std::fs::write(&small, "key = 1\n").unwrap();
        assert_eq!(read_config_text(&small).unwrap(), "key = 1\n");
        let at_limit = dir.path().join("limit.cfg");
        std::fs::write(&at_limit, vec![b'#'; MAX_CONFIG_BYTES as usize]).unwrap();
        assert_eq!(
            read_config_text(&at_limit).unwrap().len() as u64,
            MAX_CONFIG_BYTES
        );
        let large = dir.path().join("large.cfg");
        std::fs::write(&large, vec![b'#'; MAX_CONFIG_BYTES as usize + 1]).unwrap();
        let e = read_config_text(&large).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
        assert!(e.to_string().contains("larger than"));
        assert!(read_config_text(dir.path().join("missing.cfg")).is_err());
    }

    #[test]
    fn a_file_that_is_not_utf8_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("b.cfg");
        std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(
            read_config_text(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
