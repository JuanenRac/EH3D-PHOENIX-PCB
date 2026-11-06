// SPDX-License-Identifier: GPL-3.0-or-later
//! A small reader that checks the PDF this crate writes, without a PDF library: the header and
//! the end of the file, the table of offsets (every offset must point at its object, and the
//! objects must follow each other without a gap up to the table), the length of every stream, the
//! trailer, the catalog, the tree of pages, and that the content of every page is made of known
//! operators with the right operands, with the saves and restores of the state and the text
//! blocks balanced. It is not a reader of any PDF: it knows only what the writer writes.

/// What the check found in a file that passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfInfo {
    /// The pages.
    pub pages: usize,
    /// The objects, without the free one at the head of the table.
    pub objects: usize,
    /// The operators that paint a path.
    pub painted: usize,
    /// The operators that show a text.
    pub texts: usize,
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from > hay.len() {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    hay.windows(needle.len()).rposition(|w| w == needle)
}

/// The whole number after a key such as `/Length `, skipping spaces.
fn number_after(text: &str, key: &str) -> Option<usize> {
    let start = text.find(key)? + key.len();
    let digits: String = text[start..]
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Num,
    Name,
    Str,
}

fn tokenize(data: &[u8]) -> Result<Vec<(Kind, String)>, String> {
    let mut out: Vec<(Kind, String)> = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let c = data[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'(' {
            let mut depth = 1;
            i += 1;
            while i < data.len() && depth > 0 {
                match data[i] {
                    b'\\' => i += 2,
                    b'(' => {
                        depth += 1;
                        i += 1;
                    }
                    b')' => {
                        depth -= 1;
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            if depth != 0 {
                return Err("a string is not closed".to_owned());
            }
            out.push((Kind::Str, String::new()));
        } else {
            let start = i;
            i += 1;
            while i < data.len()
                && !data[i].is_ascii_whitespace()
                && data[i] != b'('
                && data[i] != b'/'
            {
                i += 1;
            }
            let word = std::str::from_utf8(&data[start..i])
                .map_err(|_| "a token is not text".to_owned())?;
            if word.starts_with('/') {
                out.push((Kind::Name, word.to_owned()));
            } else if let Ok(v) = word.parse::<f64>() {
                if !v.is_finite() {
                    return Err(format!("the number {word} is not finite"));
                }
                out.push((Kind::Num, word.to_owned()));
            } else {
                out.push((Kind::Str, format!("op:{word}")));
            }
        }
    }
    Ok(out)
}

fn operands_of(op: &str) -> Option<&'static [Kind]> {
    use Kind::{Name, Num, Str};
    Some(match op {
        "q" | "Q" | "BT" | "ET" | "h" | "S" | "s" | "f" | "B" => &[],
        "J" | "j" | "w" => &[Num],
        "gs" => &[Name],
        "m" | "l" | "Td" => &[Num, Num],
        "rg" | "RG" => &[Num, Num, Num],
        "re" => &[Num, Num, Num, Num],
        "c" => &[Num, Num, Num, Num, Num, Num],
        "Tf" => &[Name, Num],
        "Tj" => &[Str],
        _ => return None,
    })
}

/// Checks the content of a page and returns how many paths it paints and texts it shows.
fn check_content(data: &[u8]) -> Result<(usize, usize), String> {
    let tokens = tokenize(data)?;
    let mut operands: Vec<Kind> = Vec::new();
    let (mut depth, mut in_text) = (0usize, false);
    let (mut has_point, mut has_path, mut font) = (false, false, false);
    let (mut painted, mut texts) = (0usize, 0usize);
    for (kind, word) in tokens {
        let Some(op) = word.strip_prefix("op:").filter(|_| kind == Kind::Str) else {
            operands.push(kind);
            continue;
        };
        let wanted = operands_of(op)
            .ok_or_else(|| format!("the operator {op} is not one that is written"))?;
        // a string operand is a token without the `op:` mark
        if operands != wanted {
            return Err(format!("the operator {op} has the wrong operands"));
        }
        operands.clear();
        match op {
            "q" => depth += 1,
            "Q" => depth = depth.checked_sub(1).ok_or("Q without q")?,
            "BT" => {
                if in_text {
                    return Err("a text block inside a text block".to_owned());
                }
                in_text = true;
            }
            "ET" => {
                if !in_text {
                    return Err("ET without BT".to_owned());
                }
                in_text = false;
            }
            "Tf" => font = true,
            "Td" if !in_text => return Err("Td outside a text block".to_owned()),
            "Tj" => {
                if !in_text || !font {
                    return Err("Tj outside a text block or without a font".to_owned());
                }
                texts += 1;
            }
            "m" | "re" => {
                has_point = true;
                has_path = true;
            }
            "l" | "c" | "h" if !has_point => {
                return Err(format!("{op} without a current point"));
            }
            "S" | "s" | "f" | "B" => {
                if !has_path || in_text {
                    return Err(format!("{op} without a path"));
                }
                has_path = false;
                has_point = false;
                painted += 1;
            }
            _ => {}
        }
    }
    if depth != 0 || in_text || has_path || !operands.is_empty() {
        return Err(
            "the content ends inside a state, a text block, a path or an operator".to_owned(),
        );
    }
    Ok((painted, texts))
}

/// Checks a PDF written by this crate.
pub fn check_pdf(bytes: &[u8]) -> Result<PdfInfo, String> {
    if !bytes.starts_with(b"%PDF-1.") {
        return Err("the file does not start with the PDF header".to_owned());
    }
    if !bytes.ends_with(b"%%EOF\n") {
        return Err("the file does not end with %%EOF".to_owned());
    }
    let start = rfind(bytes, b"startxref\n").ok_or("there is no startxref")?;
    let after = &bytes[start + 10..];
    let line_end = after
        .iter()
        .position(|b| *b == b'\n')
        .ok_or("startxref has no number")?;
    let xref: usize = std::str::from_utf8(&after[..line_end])
        .ok()
        .and_then(|t| t.parse().ok())
        .ok_or("startxref has no number")?;
    if !bytes.get(xref..).is_some_and(|t| t.starts_with(b"xref\n")) {
        return Err("startxref does not point at the table".to_owned());
    }
    let head = xref + 5;
    let head_end = find(bytes, b"\n", head).ok_or("the table has no size")?;
    let head_text = String::from_utf8_lossy(&bytes[head..head_end]).into_owned();
    let count: usize = head_text
        .strip_prefix("0 ")
        .and_then(|n| n.parse().ok())
        .ok_or("the table does not start at the object 0")?;
    let entries = head_end + 1;
    let mut offsets: Vec<usize> = Vec::new();
    for i in 0..count {
        let at = entries + 20 * i;
        let entry = bytes.get(at..at + 20).ok_or("the table is cut short")?;
        let shaped = entry[10] == b' '
            && entry[16] == b' '
            && entry[18] == b' '
            && entry[19] == b'\n'
            && entry[..10].iter().all(u8::is_ascii_digit)
            && entry[11..16].iter().all(u8::is_ascii_digit);
        if !shaped {
            return Err(format!(
                "the entry {i} of the table is not 20 bytes of the form"
            ));
        }
        let kind = entry[17];
        if (i == 0) != (kind == b'f') || (kind != b'f' && kind != b'n') {
            return Err(format!("the entry {i} of the table has a wrong kind"));
        }
        let offset: usize = std::str::from_utf8(&entry[..10])
            .ok()
            .and_then(|t| t.parse().ok())
            .ok_or("an offset is not a number")?;
        offsets.push(offset);
    }
    let trailer_at = entries + 20 * count;
    if !bytes
        .get(trailer_at..)
        .is_some_and(|t| t.starts_with(b"trailer\n"))
    {
        return Err("the trailer does not follow the table".to_owned());
    }
    let trailer = String::from_utf8_lossy(&bytes[trailer_at..start]).into_owned();
    if number_after(&trailer, "/Size ") != Some(count) || !trailer.contains("/Root 1 0 R") {
        return Err("the trailer does not name the size and the root".to_owned());
    }

    // every object, from its offset to the next one, the last up to the table
    let mut bodies: Vec<(String, Option<Vec<u8>>)> = vec![(String::new(), None)];
    for id in 1..count {
        let offset = offsets[id];
        let end_expected = if id + 1 < count {
            offsets[id + 1]
        } else {
            xref
        };
        let header = format!("{id} 0 obj\n");
        let body_at = offset + header.len();
        if !bytes
            .get(offset..)
            .is_some_and(|t| t.starts_with(header.as_bytes()))
        {
            return Err(format!(
                "the offset of the object {id} does not point at it"
            ));
        }
        let stream_at = find(bytes, b"stream\n", body_at);
        let endobj_at = find(bytes, b"endobj\n", body_at)
            .ok_or_else(|| format!("the object {id} has no endobj"))?;
        match stream_at.filter(|s| *s < endobj_at) {
            Some(s) => {
                let dict = String::from_utf8_lossy(&bytes[body_at..s]).into_owned();
                let length = number_after(&dict, "/Length ")
                    .ok_or_else(|| format!("the stream {id} has no length"))?;
                let data_at = s + 7;
                let data = bytes
                    .get(data_at..data_at + length)
                    .ok_or_else(|| format!("the stream {id} is longer than the file"))?;
                let tail = b"\nendstream\nendobj\n";
                if !bytes
                    .get(data_at + length..)
                    .is_some_and(|t| t.starts_with(tail))
                {
                    return Err(format!("the length of the stream {id} is not the real one"));
                }
                if data_at + length + tail.len() != end_expected {
                    return Err(format!("the object {id} is not followed by the next one"));
                }
                bodies.push((dict, Some(data.to_vec())));
            }
            None => {
                if endobj_at < 1 || bytes[endobj_at - 1] != b'\n' || endobj_at + 7 != end_expected {
                    return Err(format!("the object {id} is not followed by the next one"));
                }
                let dict = String::from_utf8_lossy(&bytes[body_at..endobj_at - 1]).into_owned();
                bodies.push((dict, None));
            }
        }
    }

    if !bodies
        .get(1)
        .is_some_and(|(d, _)| d.contains("/Type /Catalog") && d.contains("/Pages 2 0 R"))
    {
        return Err("the object 1 is not the catalog".to_owned());
    }
    let pages_declared = bodies
        .get(2)
        .and_then(|(d, _)| number_after(d, "/Count "))
        .ok_or("the object 2 has no count of pages")?;
    let (mut pages, mut painted, mut texts) = (0, 0, 0);
    for (id, (dict, _)) in bodies.iter().enumerate().skip(1) {
        if !dict.contains("/Type /Page ") {
            continue;
        }
        pages += 1;
        let contents = number_after(dict, "/Contents ")
            .ok_or_else(|| format!("the page {id} has no content"))?;
        let data = bodies
            .get(contents)
            .and_then(|(_, data)| data.as_ref())
            .ok_or_else(|| format!("the content of the page {id} is not a stream"))?;
        let (p, t) = check_content(data).map_err(|e| format!("the page {id}: {e}"))?;
        painted += p;
        texts += t;
    }
    if pages != pages_declared {
        return Err(format!(
            "the tree declares {pages_declared} pages and the file holds {pages}"
        ));
    }
    Ok(PdfInfo {
        pages,
        objects: count - 1,
        painted,
        texts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{Anchor, Drawing, Item, Layer, LayerKind, Rgba};
    use crate::pdf::to_pdf;

    fn page(text: &str) -> Drawing {
        let black = Rgba::new(0, 0, 0, 255);
        Drawing {
            width: 100.0,
            height: 50.0,
            title: "T".to_owned(),
            scale: 1.0,
            background: Rgba::new(255, 255, 255, 255),
            layers: vec![Layer {
                kind: LayerKind::Outline,
                items: vec![
                    Item::line((0.0, 0.0), (10.0, 5.0), black, 0.25),
                    Item::rect((1.0, 1.0), (4.0, 3.0), Rgba::new(255, 0, 0, 128), 0.0, true),
                    Item::circle((5.0, 5.0), 1.5, black, 0.1, false),
                    Item::polyline(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)], black, 0.1),
                    Item::text((2.0, 2.0), 3.0, Anchor::Start, false, text, black),
                ],
            }],
        }
    }

    #[test]
    fn a_pdf_of_the_writer_passes_with_its_pages_paths_and_texts() {
        let out = to_pdf(&[page("R1 (a)"), page("R2")]);
        let info = check_pdf(&out.bytes).unwrap();
        assert_eq!(info.pages, 2);
        // the paper, a line, a rectangle, a circle and a polyline on each page
        assert_eq!(info.painted, 10);
        assert_eq!(info.texts, 2);
        assert!(info.objects >= 2 + 2 + 2 + 3);
    }

    #[test]
    fn a_text_with_boxes_still_passes() {
        let out = to_pdf(&[page("a縮b")]);
        assert_eq!(out.boxed_chars, 1);
        let info = check_pdf(&out.bytes).unwrap();
        // the box is a path of its own
        assert_eq!(info.painted, 6);
        assert_eq!(info.texts, 2);
    }

    #[test]
    fn a_wrong_offset_is_found() {
        let mut bytes = to_pdf(&[page("R1")]).bytes;
        let p = find(&bytes, b"00000 n \n", 0).unwrap();
        bytes[p - 2] = if bytes[p - 2] == b'9' {
            b'0'
        } else {
            bytes[p - 2] + 1
        };
        let error = check_pdf(&bytes).unwrap_err();
        assert!(error.contains("offset of the object 1"), "{error}");
    }

    #[test]
    fn a_wrong_length_is_found() {
        let mut bytes = to_pdf(&[page("R1")]).bytes;
        let p = find(&bytes, b"/Length ", 0).unwrap() + 8;
        bytes[p] = if bytes[p] == b'9' { b'8' } else { bytes[p] + 1 };
        let error = check_pdf(&bytes).unwrap_err();
        assert!(
            error.contains("length") || error.contains("longer"),
            "{error}"
        );
    }

    #[test]
    fn damage_to_the_end_or_the_content_is_found() {
        let good = to_pdf(&[page("R1")]).bytes;
        assert!(check_pdf(&good[..good.len() - 7]).is_err());
        assert!(check_pdf(b"not a pdf").is_err());
        // a stream whose content is wrong but whose length is right
        let mut bytes = good.clone();
        let p = find(&bytes, b" Tj ET", 0).unwrap();
        bytes[p + 1] = b'X';
        bytes[p + 2] = b'X';
        let error = check_pdf(&bytes).unwrap_err();
        assert!(error.contains("operator"), "{error}");
        // a count of pages that is not the real one
        let mut bytes = good;
        let p = find(&bytes, b"/Count 1", 0).unwrap() + 7;
        bytes[p] = b'3';
        assert!(check_pdf(&bytes).unwrap_err().contains("pages"));
    }

    #[test]
    fn the_content_rules_are_checked_one_by_one() {
        assert_eq!(check_content(b"q 1 0 0 rg 0 0 m 1 1 l S Q"), Ok((1, 0)));
        for bad in [
            "Q",
            "q",
            "0 0 m 1 1 l",
            "1 1 l S",
            "S",
            "BT BT ET ET",
            "ET",
            "/F1 1 Tf (a) Tj",
            "BT (a) Tj ET",
            "1 w 2",
            "1 2 rg",
            "(a",
            "0 0 m foo",
        ] {
            assert!(check_content(bad.as_bytes()).is_err(), "{bad}");
        }
    }
}
