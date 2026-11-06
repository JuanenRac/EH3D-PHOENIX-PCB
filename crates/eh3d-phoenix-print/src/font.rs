// SPDX-License-Identifier: GPL-3.0-or-later
//! The standard fonts of a PDF reader, Helvetica and Courier, with the encoding WinAnsi: which
//! characters they can show and how wide they are, so that a text can be centred or cut to a
//! width without a font file.

/// The width of the characters 32 to 126 of Helvetica, in thousandths of the size of the font.
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

/// The width of every character of Courier, in thousandths of the size of the font.
const COURIER: u16 = 600;

/// The width of the characters from 160 to 255 of Helvetica, near the mean.
const LATIN_WIDE: u16 = 556;

/// The byte of a character in WinAnsi, or `None` when the fonts cannot show it: the printable
/// ASCII and the characters of Latin-1.
pub fn winansi(c: char) -> Option<u8> {
    let code = u32::from(c);
    match code {
        0x20..=0x7e | 0xa0..=0xff => u8::try_from(code).ok(),
        _ => None,
    }
}

/// The width of a character in thousandths of the size of the font.
pub fn advance(c: char, mono: bool) -> u16 {
    if mono {
        return COURIER;
    }
    let code = u32::from(c);
    match code {
        0x20..=0x7e => HELVETICA[(code - 0x20) as usize],
        0xa0..=0xff => LATIN_WIDE,
        // a character drawn as a box
        _ => 800,
    }
}

/// The width of a text in millimetres for a size of font in millimetres.
pub fn text_width(text: &str, size: f64, mono: bool) -> f64 {
    let thousandths: u32 = text.chars().map(|c| u32::from(advance(c, mono))).sum();
    f64::from(thousandths) * size / 1000.0
}

/// The text cut to a width, with two points at the end when something was cut.
pub fn fit_text(text: &str, size: f64, mono: bool, width: f64) -> String {
    if text_width(text, size, mono) <= width {
        return text.to_owned();
    }
    let mut kept = String::new();
    for c in text.chars() {
        let mut next = kept.clone();
        next.push(c);
        next.push_str("..");
        if text_width(&next, size, mono) > width {
            break;
        }
        kept.push(c);
    }
    kept.push_str("..");
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_latin_one_are_shown_and_the_rest_is_not() {
        assert_eq!(winansi('A'), Some(b'A'));
        assert_eq!(winansi(' '), Some(0x20));
        assert_eq!(winansi('é'), Some(0xe9));
        assert_eq!(winansi('ß'), Some(0xdf));
        assert_eq!(winansi('\u{7f}'), None);
        assert_eq!(winansi('\n'), None);
        assert_eq!(winansi('縮'), None);
        assert_eq!(winansi('€'), None);
    }

    #[test]
    fn widths_follow_the_font_tables() {
        assert_eq!(advance('i', false), 222);
        assert_eq!(advance('W', false), 944);
        assert_eq!(advance('W', true), 600);
        assert_eq!(advance('縮', false), 800);
        assert!((text_width("iW", 10.0, false) - 11.66).abs() < 1e-9);
        assert!((text_width("abc", 2.0, true) - 3.6).abs() < 1e-9);
        assert_eq!(text_width("", 10.0, false), 0.0);
    }

    #[test]
    fn a_long_text_is_cut_to_the_width() {
        assert_eq!(fit_text("short", 2.0, true, 100.0), "short");
        let cut = fit_text("a long text indeed", 2.0, true, 10.0);
        assert!(cut.ends_with(".."));
        assert!(text_width(&cut, 2.0, true) <= 10.0);
        assert!(cut.len() < "a long text indeed".len());
    }
}
