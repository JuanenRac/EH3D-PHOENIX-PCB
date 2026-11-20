// SPDX-License-Identifier: GPL-3.0-or-later
//! The name of the writer profile that every deterministic output carries: one field with one rule.
//!
//! A writer gives the same bytes for the same input and the same profile, and the profile that decides the
//! form of a file has a name. The name is written into the file as a comment that says which profile it
//! was made with: `EH3D writer profile: phoenix-export/1:generic`, the tag of the format of the profile
//! and the name of the profile. A comment is used because every format that the files have for it
//! (Gerber, Excellon, XML) ignores it, so the file means the same with it and without it. The comment
//! has no date, no name of a person, of a computer or of a file, and no version of the program; the
//! profile is the only thing it says.
//!
//! The rule is here and only here: what an identity may hold, how it is made, how it is written for each
//! format and how it is read back from a file.

/// The words that come before the identity in the comment.
pub const LABEL: &str = "EH3D writer profile: ";

/// The longest name of a profile, in bytes.
pub const NAME_MAX: usize = 64;

/// The longest tag of a format, in bytes.
pub const FORMAT_MAX: usize = 32;

/// Why an identity cannot be made.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WriterProfileError {
    /// The name of the profile is empty, too long or holds something other than letters, digits, `.`, `_` and `-`
    /// (or holds two `-` in a row, which a comment of XML cannot).
    #[error("the name of the writer profile is not usable")]
    Name,
    /// The tag of the format is empty, too long or holds something other than lowercase letters, digits, `.`, `-` and `/`.
    #[error("the tag of the format of the writer profile is not usable")]
    Format,
}

fn name_is_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= NAME_MAX
        && !name.contains("--")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn format_is_valid(format: &str) -> bool {
    !format.is_empty()
        && format.len() <= FORMAT_MAX
        && !format.contains("--")
        && format.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'/')
        })
}

/// The identity of a profile: the tag of its format, a colon and its name (`phoenix-export/1:generic`).
pub fn identity(format: &str, name: &str) -> Result<String, WriterProfileError> {
    if !format_is_valid(format) {
        return Err(WriterProfileError::Format);
    }
    if !name_is_valid(name) {
        return Err(WriterProfileError::Name);
    }
    Ok(format!("{format}:{name}"))
}

/// The name of a profile made fit for an identity: every byte that a name may not hold becomes `_`, and a
/// name that comes out empty is `default`. The same text always gives the same name.
pub fn fit_name(text: &str) -> String {
    let mut out: String = text
        .bytes()
        .take(NAME_MAX)
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-') {
                char::from(b)
            } else {
                '_'
            }
        })
        .collect();
    while out.contains("--") {
        out = out.replace("--", "-_");
    }
    if out.is_empty() {
        "default".to_owned()
    } else {
        out
    }
}

/// The comment of a Gerber file, with no line break: `G04 EH3D writer profile: ID*`.
pub fn gerber_comment(identity: &str) -> String {
    format!("G04 {LABEL}{identity}*")
}

/// The comment of an Excellon file, with no line break: `; EH3D writer profile: ID`.
pub fn excellon_comment(identity: &str) -> String {
    format!("; {LABEL}{identity}")
}

/// The comment of an XML file: `<!-- EH3D writer profile: ID -->`.
pub fn xml_comment(identity: &str) -> String {
    format!("<!-- {LABEL}{identity} -->")
}

/// The line breaks that the text uses: `\r\n` when the first one is, otherwise `\n`.
fn eol_of(text: &str) -> &'static str {
    match text.find('\n') {
        Some(i) if i > 0 && text.as_bytes()[i - 1] == b'\r' => "\r\n",
        _ => "\n",
    }
}

/// Puts a line in a text after its first line, or at its head when the first line starts with none of `keep_first`'s
/// words. The text is returned as it is when it already holds a writer profile (a file has one).
fn put_line(text: &str, line: &str, after_first: impl Fn(&str) -> bool) -> String {
    if find(text).is_some() {
        return text.to_owned();
    }
    let eol = eol_of(text);
    let first_end = text.find('\n').map_or(text.len(), |i| i + 1);
    let first = text[..first_end].trim_end_matches(['\r', '\n']);
    if after_first(first) {
        let mut out = String::with_capacity(text.len() + line.len() + 2);
        out.push_str(&text[..first_end]);
        if !text[..first_end].ends_with('\n') {
            out.push_str(eol);
        }
        out.push_str(line);
        out.push_str(eol);
        out.push_str(&text[first_end..]);
        out
    } else {
        format!("{line}{eol}{text}")
    }
}

/// A Gerber file with the profile written as its first line.
pub fn stamp_gerber(text: &str, identity: &str) -> String {
    put_line(text, &gerber_comment(identity), |_| false)
}

/// An Excellon file with the profile written after its `M48` line (the head of the file), or as the first line
/// when the file does not start with `M48`.
pub fn stamp_excellon(text: &str, identity: &str) -> String {
    put_line(text, &excellon_comment(identity), |first| {
        first.trim() == "M48"
    })
}

/// An XML file with the profile written as a comment after its declaration (`<?xml ...?>`), or as the first line
/// when it has none.
pub fn stamp_xml(text: &str, identity: &str) -> String {
    put_line(text, &xml_comment(identity), |first| {
        first.trim_start().starts_with("<?xml")
    })
}

fn is_identity_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/' | b':')
}

/// The identity that a text carries: the first one that follows the words of the comment, or `None`.
pub fn find(text: &str) -> Option<String> {
    let at = text.find(LABEL)? + LABEL.len();
    let rest = &text.as_bytes()[at..];
    let len = rest.iter().take_while(|b| is_identity_byte(**b)).count();
    (len > 0).then(|| String::from_utf8_lossy(&rest[..len]).into_owned())
}

/// How many times the words of the comment are in a text (a file has exactly one).
pub fn count(text: &str) -> usize {
    text.matches(LABEL).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A generator of numbers with a fixed seed, so the test tries the same cases every time.
    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *seed >> 33
    }

    #[test]
    fn an_identity_is_the_format_a_colon_and_the_name() {
        assert_eq!(
            identity("phoenix-export/1", "generic").unwrap(),
            "phoenix-export/1:generic"
        );
        assert_eq!(
            identity("phoenix-ipc2581/1", "my_maker-2.0").unwrap(),
            "phoenix-ipc2581/1:my_maker-2.0"
        );
    }

    #[test]
    fn a_name_or_a_format_that_the_comment_could_not_hold_is_refused() {
        for name in [
            "", " ", "a b", "a*b", "a%b", "a,b", "a--b", "é", "a\nb", "a:b", "a/b",
        ] {
            assert_eq!(
                identity("phoenix-export/1", name),
                Err(WriterProfileError::Name),
                "{name:?}"
            );
        }
        assert_eq!(
            identity("phoenix-export/1", &"a".repeat(NAME_MAX + 1)),
            Err(WriterProfileError::Name)
        );
        assert!(identity("phoenix-export/1", &"a".repeat(NAME_MAX)).is_ok());
        for format in [
            "",
            "Phoenix",
            "a b",
            "a*b",
            "a--b",
            &"a".repeat(FORMAT_MAX + 1),
        ] {
            assert_eq!(
                identity(format, "generic"),
                Err(WriterProfileError::Format),
                "{format:?}"
            );
        }
    }

    #[test]
    fn a_name_made_fit_is_always_usable_and_the_same_text_gives_the_same_name() {
        for text in [
            "",
            "a b",
            "x--y",
            "é",
            "A*B%C,D",
            "ok-name_1.2",
            &"z".repeat(300),
        ] {
            let name = fit_name(text);
            assert!(
                identity("phoenix-export/1", &name).is_ok(),
                "{text:?} -> {name:?}"
            );
            assert_eq!(name, fit_name(text));
        }
        assert_eq!(fit_name(""), "default");
        assert_eq!(fit_name("ok-name_1.2"), "ok-name_1.2");
        assert_eq!(fit_name("a b"), "a_b");
    }

    #[test]
    fn a_name_made_fit_from_any_bytes_is_usable() {
        // names drawn from a generator with a fixed seed
        let mut seed = 0x0a_f1_7e_u64;
        let alphabet = b"ab-_. *%,\x7f\xc3\xa9\n--";
        for _ in 0..500 {
            let len = (lcg(&mut seed) % 40) as usize;
            let bytes: Vec<u8> = (0..len)
                .map(|_| alphabet[(lcg(&mut seed) as usize) % alphabet.len()])
                .collect();
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let name = fit_name(&text);
            assert!(identity("f/1", &name).is_ok(), "{text:?} -> {name:?}");
        }
    }

    #[test]
    fn the_comment_of_each_format_has_the_words_and_no_line_break() {
        let id = "phoenix-export/1:generic";
        assert_eq!(
            gerber_comment(id),
            "G04 EH3D writer profile: phoenix-export/1:generic*"
        );
        assert_eq!(
            excellon_comment(id),
            "; EH3D writer profile: phoenix-export/1:generic"
        );
        assert_eq!(
            xml_comment(id),
            "<!-- EH3D writer profile: phoenix-export/1:generic -->"
        );
        for c in [gerber_comment(id), excellon_comment(id), xml_comment(id)] {
            assert!(!c.contains('\n') && !c.contains('\r'));
            assert_eq!(find(&c).as_deref(), Some(id));
            assert_eq!(count(&c), 1);
        }
    }

    #[test]
    fn a_gerber_file_gets_the_profile_as_its_first_line_once() {
        let text = "%TF.GenerationSoftware,EH3D,PHOENIX-PCB,0.1.4*%\n%FSLAX46Y46*%\nM02*\n";
        let id = "phoenix-export/1:generic";
        let stamped = stamp_gerber(text, id);
        assert!(stamped.starts_with("G04 EH3D writer profile: phoenix-export/1:generic*\n%TF."));
        assert!(stamped.ends_with(text));
        assert_eq!(count(&stamped), 1);
        // a file that has one keeps it and a second profile does not replace it
        assert_eq!(stamp_gerber(&stamped, "phoenix-export/1:other"), stamped);
        assert_eq!(find(&stamped).as_deref(), Some(id));
    }

    #[test]
    fn an_excellon_file_gets_the_profile_after_its_m48_line() {
        let text = "M48\n; #@! TF.FileFunction,Plated,1,2,PTH\nMETRIC\n%\nM30\n";
        let stamped = stamp_excellon(text, "phoenix-export/1:generic");
        assert!(stamped.starts_with("M48\n; EH3D writer profile: phoenix-export/1:generic\n; #@!"));
        assert_eq!(count(&stamped), 1);
        // a file that does not start with M48 gets it at the head
        let plain = stamp_excellon("METRIC\nM30\n", "phoenix-export/1:generic");
        assert!(plain.starts_with("; EH3D writer profile: phoenix-export/1:generic\nMETRIC\n"));
    }

    #[test]
    fn an_xml_file_gets_the_profile_after_its_declaration() {
        let text = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<IPC-2581 revision=\"C\"/>\n";
        let stamped = stamp_xml(text, "phoenix-ipc2581/1:default");
        assert!(stamped.starts_with(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- EH3D writer profile: phoenix-ipc2581/1:default -->\n<IPC-2581"
        ));
        assert_eq!(count(&stamped), 1);
        let bare = stamp_xml("<a/>\n", "phoenix-ipc2581/1:default");
        assert!(bare.starts_with("<!-- EH3D writer profile:"));
    }

    #[test]
    fn the_line_breaks_of_the_file_are_the_ones_of_the_comment() {
        let text = "M48\r\nMETRIC\r\nM30\r\n";
        let stamped = stamp_excellon(text, "phoenix-export/1:generic");
        assert_eq!(
            stamped,
            "M48\r\n; EH3D writer profile: phoenix-export/1:generic\r\nMETRIC\r\nM30\r\n"
        );
        let no_break = stamp_gerber("M02*", "f/1:x");
        assert_eq!(no_break, "G04 EH3D writer profile: f/1:x*\nM02*");
        let one_line = stamp_excellon("M48", "f/1:x");
        assert_eq!(one_line, "M48\n; EH3D writer profile: f/1:x\n");
    }

    #[test]
    fn stamping_twice_gives_the_bytes_of_stamping_once_and_leaves_the_rest_alone() {
        // texts drawn from a generator with a fixed seed
        let mut seed = 0x0005_7a49_u64;
        let pieces = [
            "M48", "METRIC", "%", "G04 x*", "M30", "", "X1Y2D01*", "; hi",
        ];
        for _ in 0..200 {
            let lines: Vec<&str> = (0..(lcg(&mut seed) % 8))
                .map(|_| pieces[(lcg(&mut seed) as usize) % pieces.len()])
                .collect();
            let eol = if lcg(&mut seed).is_multiple_of(2) {
                "\n"
            } else {
                "\r\n"
            };
            let text = lines.join(eol);
            for stamp in [stamp_gerber, stamp_excellon, stamp_xml] {
                let once = stamp(&text, "f/1:x");
                assert_eq!(stamp(&once, "f/1:x"), once, "{text:?}");
                assert_eq!(count(&once), 1, "{text:?}");
                // what was there is still there, in order: taking the comment out gives the text back
                let without: String = once
                    .split_inclusive('\n')
                    .filter(|l| !l.contains(LABEL))
                    .collect();
                if text.is_empty() {
                    assert!(without.is_empty());
                } else if !text.ends_with('\n') && !text.contains('\n') {
                    // a text of one line gets a line break after it when the comment goes below
                    assert!(
                        without.trim_end() == text.trim_end(),
                        "{text:?} {without:?}"
                    );
                } else {
                    assert_eq!(without.trim_end(), text.trim_end(), "{text:?}");
                }
            }
        }
    }

    #[test]
    fn nothing_but_the_profile_is_in_the_comment() {
        // no date, no user, no host, no version: the comment is the label and the identity
        let c = gerber_comment("phoenix-export/1:generic");
        assert_eq!(
            c.len(),
            "G04 ".len() + LABEL.len() + "phoenix-export/1:generic".len() + "*".len()
        );
        assert!(find("no stamp here").is_none());
        assert!(find("EH3D writer profile: ").is_none());
        assert_eq!(count("no stamp here"), 0);
    }
}
