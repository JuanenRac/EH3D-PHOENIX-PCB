// SPDX-License-Identifier: GPL-3.0-or-later
//! Rules for the names and the paths that a program writes: the names that Windows keeps for its
//! devices, the paths that reach a device or another computer, and the alternate data streams of
//! NTFS. The rules look at the text only; nothing is read from the disk here.

/// True when `name` is, without regard to case and to any extension after the first dot, one of
/// the names that Windows keeps for its devices (`CON`, `PRN`, `AUX`, `NUL`, `COM1` to `COM9`,
/// `LPT1` to `LPT9`, `CONIN$`, `CONOUT$`). A file of such a name is not a file on that system, and
/// trailing spaces do not change that.
pub fn is_reserved_device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").trim_end_matches(' ');
    let upper = stem.to_uppercase();
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    for prefix in ["COM", "LPT"] {
        if let Some(rest) = upper.strip_prefix(prefix) {
            let mut chars = rest.chars();
            if let (Some(c), None) = (chars.next(), chars.next()) {
                if matches!(c, '1'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}') {
                    return true;
                }
            }
        }
    }
    false
}

/// Why `path` must not be written, or `None` when its text is acceptable. The reasons are fixed
/// words: an empty path, a character that ends a name (NUL), a device or verbatim path
/// (`\\.\`, `\\?\`), a path of another computer (`\\server\share`), an alternate data stream
/// (`name:stream`) and a part named like a Windows device. A drive letter (`C:\`) is accepted and
/// so are `.` and `..`: the person who types a path chooses where it goes.
pub fn output_path_problem(path: &str) -> Option<&'static str> {
    if path.is_empty() {
        return Some("empty path");
    }
    if path.contains('\0') {
        return Some("the path holds a NUL character");
    }
    let slashes = path.replace('\\', "/");
    if slashes.starts_with("//./") || slashes.starts_with("//?/") {
        return Some("device and verbatim paths are not written");
    }
    if slashes.starts_with("//") {
        return Some("paths of other computers are not written");
    }
    let bytes = slashes.as_bytes();
    let after_drive = if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        &slashes[2..]
    } else {
        slashes.as_str()
    };
    if after_drive.contains(':') {
        return Some("alternate data streams are not written");
    }
    if after_drive
        .split('/')
        .any(|part| !part.is_empty() && is_reserved_device_name(part))
    {
        return Some("a part of the path is named like a Windows device");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_of_the_devices_are_found_with_any_case_and_extension() {
        for name in [
            "CON",
            "con",
            "Nul.txt",
            "aux.kicad_pcb",
            "COM1",
            "com9.log",
            "LPT3",
            "CONIN$",
            "prn ",
            "COM\u{b9}",
        ] {
            assert!(is_reserved_device_name(name), "{name}");
        }
        for name in [
            "CONSOLE",
            "COM0",
            "COM10",
            "LPT",
            "x.con",
            "console.txt",
            "",
        ] {
            assert!(!is_reserved_device_name(name), "{name}");
        }
    }

    #[test]
    fn a_path_is_refused_for_each_of_its_dangers_and_accepted_otherwise() {
        for (path, ok) in [
            ("out.json", true),
            ("C:\\work\\out.json", true),
            ("../up/out.json", true),
            ("folder/sub/out.json", true),
            ("", false),
            ("a\0b", false),
            ("\\\\.\\COM1", false),
            ("//?/C:/x", false),
            ("\\\\server\\share\\x", false),
            ("out.json:stream", false),
            ("C:\\work\\out.json:stream", false),
            ("work/NUL", false),
            ("work\\aux.txt", false),
        ] {
            assert_eq!(output_path_problem(path).is_none(), ok, "{path:?}");
        }
    }
}
