// SPDX-License-Identifier: GPL-3.0-or-later
//! Tells the kind of a design file from its first bytes, without reading the design.
//!
//! The XML form of the reference tool's files has a root element named `eagle` with a `version`
//! attribute (the published `eagle.dtd` declares it). Anything that does not start like XML is
//! reported as not XML: that covers the old binary versions and any other file, and this module
//! does not try to tell them apart.

/// The kind of a file as far as its first bytes show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    /// XML whose root element is `eagle`.
    EagleXml,
    /// XML with another root element, or whose root element was not reached in the bytes looked at.
    OtherXml,
    /// Does not start like XML (binary or unknown).
    NotXml,
}

/// The result of looking at the first bytes of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sniffed {
    /// The kind.
    pub family: Family,
    /// The declared version of an `eagle` root element when it is made only of digits and dots, at most 16 long.
    pub version: Option<String>,
}

const NOT_XML: Sniffed = Sniffed {
    family: Family::NotXml,
    version: None,
};
const OTHER_XML: Sniffed = Sniffed {
    family: Family::OtherXml,
    version: None,
};

fn skip_past(head: &[u8], from: usize, marker: &[u8]) -> Option<usize> {
    head.get(from..)?
        .windows(marker.len())
        .position(|w| w == marker)
        .map(|p| from + p + marker.len())
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

/// Look at the first bytes of a file.
pub fn sniff(head: &[u8]) -> Sniffed {
    let mut pos = if head.starts_with(&[0xEF, 0xBB, 0xBF]) {
        3
    } else {
        0
    };
    while head.get(pos).is_some_and(|b| is_space(*b)) {
        pos += 1;
    }
    if head.get(pos) != Some(&b'<') {
        return NOT_XML;
    }
    // skip the declaration, comments and the document type until the root element
    loop {
        while head.get(pos).is_some_and(|b| is_space(*b)) {
            pos += 1;
        }
        let rest = match head.get(pos..) {
            Some(r) if r.first() == Some(&b'<') => r,
            _ => return OTHER_XML,
        };
        let next = if rest.starts_with(b"<?") {
            skip_past(head, pos, b"?>")
        } else if rest.starts_with(b"<!--") {
            skip_past(head, pos + 4, b"-->")
        } else if rest.starts_with(b"<!") {
            // a document type: it ends at the first `>`, or at `]>` when it has an internal subset
            let end = skip_past(head, pos, b">");
            match (end, rest.iter().position(|b| *b == b'[')) {
                (Some(e), Some(open)) if pos + open < e => skip_past(head, pos + open, b"]>"),
                (e, _) => e,
            }
        } else {
            break;
        };
        match next {
            Some(n) => pos = n,
            None => return OTHER_XML,
        }
    }
    // the root element
    let tag = &head[pos + 1..];
    let name_end = tag
        .iter()
        .position(|b| is_space(*b) || *b == b'>' || *b == b'/')
        .unwrap_or(tag.len());
    if &tag[..name_end] != b"eagle" || name_end == tag.len() {
        return OTHER_XML;
    }
    Sniffed {
        family: Family::EagleXml,
        version: read_version(&tag[name_end..]),
    }
}

/// Read the `version` attribute from the attributes of a start tag (the bytes after the name).
fn read_version(mut attrs: &[u8]) -> Option<String> {
    loop {
        while attrs.first().is_some_and(|b| is_space(*b)) {
            attrs = &attrs[1..];
        }
        if matches!(attrs.first(), None | Some(b'>') | Some(b'/')) {
            return None;
        }
        let name_end = attrs.iter().position(|b| is_space(*b) || *b == b'=')?;
        let name = &attrs[..name_end];
        attrs = &attrs[name_end..];
        while attrs.first().is_some_and(|b| is_space(*b)) {
            attrs = &attrs[1..];
        }
        attrs = attrs.strip_prefix(b"=")?;
        while attrs.first().is_some_and(|b| is_space(*b)) {
            attrs = &attrs[1..];
        }
        let quote = *attrs.first()?;
        if quote != b'"' && quote != b'\'' {
            return None;
        }
        let close = attrs[1..].iter().position(|b| *b == quote)?;
        let value = &attrs[1..1 + close];
        attrs = &attrs[close + 2..];
        if name == b"version" {
            let valid = !value.is_empty()
                && value.len() <= 16
                && value.iter().all(|b| b.is_ascii_digit() || *b == b'.');
            return valid.then(|| String::from_utf8_lossy(value).into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(text: &[u8]) -> (Family, Option<String>) {
        let s = sniff(text);
        (s.family, s.version)
    }

    #[test]
    fn reads_the_version_of_an_eagle_root() {
        let doc = b"<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE eagle SYSTEM \"eagle.dtd\">\n<eagle version=\"9.6.2\">\n<drawing/>";
        assert_eq!(kind(doc), (Family::EagleXml, Some("9.6.2".into())));
    }

    #[test]
    fn copes_with_bom_comments_quotes_and_extra_attributes() {
        let doc = b"\xEF\xBB\xBF  <?xml version='1.0'?><!-- a <eagle version=\"1.0\"> in a comment --><eagle xmlns:x='u' version = '6.5.0' other=\"1\">";
        assert_eq!(kind(doc), (Family::EagleXml, Some("6.5.0".into())));
    }

    #[test]
    fn a_document_type_with_an_internal_subset_is_skipped() {
        let doc = b"<?xml version=\"1.0\"?><!DOCTYPE eagle [ <!ELEMENT eagle ANY> ]><eagle version=\"7.2\">";
        assert_eq!(kind(doc), (Family::EagleXml, Some("7.2".into())));
    }

    #[test]
    fn other_roots_are_other_xml() {
        assert_eq!(
            kind(b"<?xml version=\"1.0\"?><kicad_pcb version=\"20221018\">").0,
            Family::OtherXml
        );
        assert_eq!(kind(b"<eaglet version=\"9.0\">").0, Family::OtherXml);
        assert_eq!(
            kind(b"<eagle").0,
            Family::OtherXml,
            "a name cut off by the end of the bytes is not trusted"
        );
        assert_eq!(
            kind(b"<?xml version=\"1.0\"?><!-- never closed").0,
            Family::OtherXml
        );
    }

    #[test]
    fn anything_else_is_not_xml() {
        assert_eq!(kind(b"").0, Family::NotXml);
        assert_eq!(
            kind(&[0x10, 0x80, 0x00, 0x00, b'<', b'e']).0,
            Family::NotXml
        );
        assert_eq!(kind(b"(kicad_pcb (version 20221018))").0, Family::NotXml);
        assert_eq!(kind(&[0xEF, 0xBB]).0, Family::NotXml);
    }

    #[test]
    fn a_version_that_is_not_digits_and_dots_is_dropped_not_echoed() {
        for doc in [
            &b"<eagle version=\"9.6.2-ClientName\">"[..],
            b"<eagle version=\"\">",
            b"<eagle version=\"1.2.3.4.5.6.7.8.9.0\">",
            b"<eagle version=9.6>",
            b"<eagle other=\"1\">",
            b"<eagle version=\"9.6",
        ] {
            let s = sniff(doc);
            assert_eq!(s.family, Family::EagleXml, "{doc:?}");
            assert_eq!(s.version, None, "{doc:?}");
        }
    }

    #[test]
    fn hostile_bytes_never_panic() {
        // every prefix of a valid document, and a few damaged ones
        let doc = b"<?xml version=\"1.0\"?><!DOCTYPE eagle [ <!ELEMENT a ANY> ]><!-- c --><eagle version=\"9.6.2\" x='y'>";
        for end in 0..=doc.len() {
            let _ = sniff(&doc[..end]);
        }
        for bytes in [
            &b"<"[..],
            b"<!",
            b"<!-",
            b"<?",
            b"<!DOCTYPE [",
            b"<a",
            b"<eagle ",
            b"<eagle =",
            b"<eagle a=",
            b"<eagle a='",
        ] {
            let _ = sniff(bytes);
        }
        let junk: Vec<u8> = (0..4096u32)
            .map(|n| (n.wrapping_mul(2654435761) >> 13) as u8)
            .collect();
        let _ = sniff(&junk);
    }
}
