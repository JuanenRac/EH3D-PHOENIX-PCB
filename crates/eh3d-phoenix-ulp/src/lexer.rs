// SPDX-License-Identifier: GPL-3.0-or-later
//! The lexer: a program as a list of tokens, each with the line and the column where it starts.
//!
//! It is bounded: a program larger than `max_source_bytes` is refused before it is looked at, and
//! the list of tokens stops at `max_tokens`. A problem inside the program (a text that is not
//! closed, a number that is not valid, a character that has no place) is kept with its place and
//! the lexer goes on, so one reading shows several problems; at most `max_problems` are kept and
//! the rest are counted. The lexer never panics and never reads outside the text it was given.

use crate::codes::{
    Diag, CHAR_INVALID, COMMENT_OPEN, DIRECTIVE_INVALID, ESCAPE_INVALID, NUMBER_INVALID,
    SOURCE_TOO_LARGE, STRING_OPEN, TOO_MANY_TOKENS,
};
use crate::settings::UlpSettings;

/// What a token is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A name or a keyword.
    Ident,
    /// A whole number, decimal or hexadecimal.
    Int,
    /// A number with a point or an exponent.
    Real,
    /// A character in single quotes; the text is the character.
    Char,
    /// A text in double quotes (or the name of a file between `<` and `>` after `#include`); the
    /// text is what it says, with the escapes read.
    Str,
    /// An operator or a mark.
    Punct,
    /// `#` and a name; the text is the name.
    Directive,
    /// The end of the program.
    Eof,
}

/// One token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// What it is.
    pub kind: Kind,
    /// Its text (see [`Kind`]).
    pub text: String,
    /// The line where it starts, from one.
    pub line: usize,
    /// The column where it starts, from one, counted in characters.
    pub col: usize,
}

impl Token {
    /// True for the mark `text`.
    pub fn is(&self, text: &str) -> bool {
        self.kind == Kind::Punct && self.text == text
    }

    /// True for the name `text`.
    pub fn is_word(&self, text: &str) -> bool {
        self.kind == Kind::Ident && self.text == text
    }

    /// The token as it is told in a message.
    pub fn describe(&self) -> String {
        match self.kind {
            Kind::Eof => "the end of the program".to_owned(),
            Kind::Str => "a text".to_owned(),
            Kind::Directive => format!("`#{}`", self.text),
            _ => format!("`{}`", self.text),
        }
    }
}

/// The problems found so far: the first ones are kept and the rest are counted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Problems {
    /// The problems kept, in the order they were found.
    pub list: Vec<Diag>,
    /// How many more were found after the list was full.
    pub dropped: usize,
    max: usize,
}

impl Problems {
    /// An empty list that keeps at most `max` problems.
    pub fn new(max: usize) -> Problems {
        Problems {
            list: Vec::new(),
            dropped: 0,
            max,
        }
    }

    /// Adds a problem.
    pub fn push(&mut self, diag: Diag) {
        if self.list.len() < self.max {
            self.list.push(diag);
        } else {
            self.dropped += 1;
        }
    }

    /// How many problems were found, kept or not.
    pub fn total(&self) -> usize {
        self.list.len() + self.dropped
    }
}

/// The result of the lexer: the tokens (the last is always [`Kind::Eof`]) and the problems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexed {
    /// The tokens.
    pub tokens: Vec<Token>,
    /// The problems found while reading the characters.
    pub problems: Problems,
}

const PUNCT3: [&str; 2] = ["<<=", ">>="];
const PUNCT2: [&str; 17] = [
    "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "==", "!=", "<=", ">=", "&&", "||",
    "<<",
];
const PUNCT1: &str = "+-*/%&|^~!<>=?:,;.()[]{}";

struct Cursor {
    chars: Vec<char>,
    at: usize,
    line: usize,
    col: usize,
}

impl Cursor {
    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.at + ahead).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.at).copied()?;
        self.at += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Reads a program into tokens. Only a program that is too large, or has too many tokens, is an
/// error; every other problem is kept in the result.
pub fn lex(source: &str, settings: &UlpSettings) -> Result<Lexed, Diag> {
    if source.len() > settings.max_source_bytes {
        return Err(Diag::at(
            SOURCE_TOO_LARGE,
            1,
            1,
            format!(
                "{} bytes, the limit is {}",
                source.len(),
                settings.max_source_bytes
            ),
        ));
    }
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut cur = Cursor {
        chars: source.chars().collect(),
        at: 0,
        line: 1,
        col: 1,
    };
    let mut tokens: Vec<Token> = Vec::new();
    let mut problems = Problems::new(settings.max_problems);
    loop {
        skip_blank(&mut cur, &mut problems);
        let (line, col) = (cur.line, cur.col);
        let Some(c) = cur.peek(0) else { break };
        if tokens.len() >= settings.max_tokens {
            return Err(Diag::at(
                TOO_MANY_TOKENS,
                line,
                col,
                format!("the limit is {}", settings.max_tokens),
            ));
        }
        let token = if is_name_start(c) {
            let mut name = String::new();
            while let Some(c) = cur.peek(0).filter(|c| is_name_char(*c)) {
                name.push(c);
                cur.bump();
            }
            if name.chars().count() > settings.max_name_chars {
                problems.push(Diag::at(
                    SOURCE_TOO_LARGE,
                    line,
                    col,
                    format!("a name longer than {} characters", settings.max_name_chars),
                ));
            }
            Token {
                kind: Kind::Ident,
                text: name,
                line,
                col,
            }
        } else if c.is_ascii_digit()
            || (c == '.' && cur.peek(1).is_some_and(|d| d.is_ascii_digit()))
        {
            read_number(&mut cur, &mut problems, line, col)
        } else if c == '"' {
            read_string(&mut cur, &mut problems, settings, line, col)
        } else if c == '\'' {
            read_char(&mut cur, &mut problems, line, col)
        } else if c == '#' {
            cur.bump();
            let mut name = String::new();
            while let Some(c) = cur.peek(0).filter(|c| is_name_char(*c)) {
                name.push(c);
                cur.bump();
            }
            if name.is_empty() {
                problems.push(Diag::at(CHAR_INVALID, line, col, "`#` without a name"));
                continue;
            }
            let include = name == "include";
            tokens.push(Token {
                kind: Kind::Directive,
                text: name,
                line,
                col,
            });
            if include {
                if let Some(file) = read_angle_name(&mut cur, &mut problems) {
                    tokens.push(file);
                }
            }
            continue;
        } else if let Some(text) = punct(&cur) {
            for _ in 0..text.len() {
                cur.bump();
            }
            Token {
                kind: Kind::Punct,
                text: text.to_owned(),
                line,
                col,
            }
        } else {
            cur.bump();
            problems.push(Diag::at(
                CHAR_INVALID,
                line,
                col,
                format!("`{}`", c.escape_default()),
            ));
            continue;
        };
        tokens.push(token);
    }
    tokens.push(Token {
        kind: Kind::Eof,
        text: String::new(),
        line: cur.line,
        col: cur.col,
    });
    Ok(Lexed { tokens, problems })
}

fn punct(cur: &Cursor) -> Option<&'static str> {
    let take = |n: usize| -> String { (0..n).filter_map(|i| cur.peek(i)).collect() };
    let three = take(3);
    if let Some(p) = PUNCT3.iter().find(|p| **p == three) {
        return Some(p);
    }
    let two = take(2);
    if let Some(p) = PUNCT2.iter().find(|p| **p == two) {
        return Some(p);
    }
    // `>>` is kept apart from `>=` and `>>=` above only by length, so it is looked for here
    if two == ">>" {
        return Some(">>");
    }
    let one = cur.peek(0)?;
    PUNCT1
        .char_indices()
        .find(|(_, p)| *p == one)
        .map(|(i, _)| &PUNCT1[i..=i])
}

/// Skips blanks, comments and the backslash that joins a line to the next one.
fn skip_blank(cur: &mut Cursor, problems: &mut Problems) {
    loop {
        match cur.peek(0) {
            Some(c) if c.is_whitespace() => {
                cur.bump();
            }
            Some('\\') if matches!(cur.peek(1), Some('\n')) => {
                cur.bump();
            }
            Some('\\') if matches!((cur.peek(1), cur.peek(2)), (Some('\r'), Some('\n'))) => {
                cur.bump();
                cur.bump();
            }
            Some('/') if cur.peek(1) == Some('/') => {
                while cur.peek(0).is_some_and(|c| c != '\n') {
                    cur.bump();
                }
            }
            Some('/') if cur.peek(1) == Some('*') => {
                let (line, col) = (cur.line, cur.col);
                cur.bump();
                cur.bump();
                let mut closed = false;
                while let Some(c) = cur.bump() {
                    if c == '*' && cur.peek(0) == Some('/') {
                        cur.bump();
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    problems.push(Diag::at(COMMENT_OPEN, line, col, "`/*` without `*/`"));
                }
            }
            _ => break,
        }
    }
}

fn read_number(cur: &mut Cursor, problems: &mut Problems, line: usize, col: usize) -> Token {
    let mut text = String::new();
    let mut real = false;
    let mut valid = true;
    let hex = cur.peek(0) == Some('0') && matches!(cur.peek(1), Some('x' | 'X'));
    if hex {
        text.push('0');
        cur.bump();
        text.push('x');
        cur.bump();
        let mut digits = 0;
        while let Some(c) = cur.peek(0).filter(char::is_ascii_hexdigit) {
            text.push(c);
            cur.bump();
            digits += 1;
        }
        if digits == 0 || digits > 16 {
            valid = false;
        }
    } else {
        while let Some(c) = cur.peek(0).filter(char::is_ascii_digit) {
            text.push(c);
            cur.bump();
        }
        if cur.peek(0) == Some('.') {
            real = true;
            text.push('.');
            cur.bump();
            while let Some(c) = cur.peek(0).filter(char::is_ascii_digit) {
                text.push(c);
                cur.bump();
            }
        }
        if matches!(cur.peek(0), Some('e' | 'E')) {
            real = true;
            text.push('e');
            cur.bump();
            if let Some(sign) = cur.peek(0).filter(|c| matches!(c, '+' | '-')) {
                text.push(sign);
                cur.bump();
            }
            let mut digits = 0;
            while let Some(c) = cur.peek(0).filter(char::is_ascii_digit) {
                text.push(c);
                cur.bump();
                digits += 1;
            }
            if digits == 0 {
                valid = false;
            }
        }
        if valid {
            valid = if real {
                text.parse::<f64>().is_ok_and(f64::is_finite)
            } else {
                text.parse::<i64>().is_ok()
            };
        }
    }
    // a name stuck to a number (`12abc`) is one wrong number and not a number and a name
    while let Some(c) = cur.peek(0).filter(|c| is_name_char(*c) || *c == '.') {
        text.push(c);
        cur.bump();
        valid = false;
    }
    if !valid {
        problems.push(Diag::at(NUMBER_INVALID, line, col, format!("`{text}`")));
    }
    Token {
        kind: if real { Kind::Real } else { Kind::Int },
        text,
        line,
        col,
    }
}

/// Reads one escape after a backslash that was taken; the character it stands for.
fn read_escape(cur: &mut Cursor, problems: &mut Problems) -> char {
    let (line, col) = (cur.line, cur.col.saturating_sub(1));
    let Some(c) = cur.peek(0) else {
        return '\\';
    };
    let plain = match c {
        'n' => Some('\n'),
        't' => Some('\t'),
        'r' => Some('\r'),
        'a' => Some('\u{7}'),
        'b' => Some('\u{8}'),
        'f' => Some('\u{c}'),
        'v' => Some('\u{b}'),
        '\\' | '"' | '\'' | '?' => Some(c),
        _ => None,
    };
    if let Some(value) = plain {
        cur.bump();
        return value;
    }
    if c == 'x' || c == 'X' {
        cur.bump();
        let mut value = 0u32;
        let mut digits = 0;
        while let Some(d) = cur
            .peek(0)
            .and_then(|c| c.to_digit(16))
            .filter(|_| digits < 2)
        {
            value = value * 16 + d;
            digits += 1;
            cur.bump();
        }
        if digits == 0 {
            problems.push(Diag::at(ESCAPE_INVALID, line, col, "`\\x` without digits"));
            return 'x';
        }
        return char::from_u32(value).unwrap_or('\u{fffd}');
    }
    if c.is_digit(8) {
        let mut value = 0u32;
        let mut digits = 0;
        while let Some(d) = cur
            .peek(0)
            .and_then(|c| c.to_digit(8))
            .filter(|_| digits < 3)
        {
            value = value * 8 + d;
            digits += 1;
            cur.bump();
        }
        return char::from_u32(value).unwrap_or('\u{fffd}');
    }
    cur.bump();
    problems.push(Diag::at(
        ESCAPE_INVALID,
        line,
        col,
        format!("`\\{}`", c.escape_default()),
    ));
    c
}

fn read_string(
    cur: &mut Cursor,
    problems: &mut Problems,
    settings: &UlpSettings,
    line: usize,
    col: usize,
) -> Token {
    cur.bump();
    let mut text = String::new();
    let mut chars = 0usize;
    let mut too_long = false;
    let mut closed = false;
    while let Some(c) = cur.peek(0) {
        match c {
            '"' => {
                cur.bump();
                closed = true;
                break;
            }
            '\n' => break,
            '\\' => {
                cur.bump();
                let value = read_escape(cur, problems);
                chars += 1;
                if chars <= settings.max_string_chars {
                    text.push(value);
                } else {
                    too_long = true;
                }
            }
            _ => {
                cur.bump();
                chars += 1;
                if chars <= settings.max_string_chars {
                    text.push(c);
                } else {
                    too_long = true;
                }
            }
        }
    }
    if !closed {
        problems.push(Diag::at(
            STRING_OPEN,
            line,
            col,
            "no closing quote on the line",
        ));
    }
    if too_long {
        problems.push(Diag::at(
            SOURCE_TOO_LARGE,
            line,
            col,
            format!(
                "a text longer than {} characters",
                settings.max_string_chars
            ),
        ));
    }
    Token {
        kind: Kind::Str,
        text,
        line,
        col,
    }
}

fn read_char(cur: &mut Cursor, problems: &mut Problems, line: usize, col: usize) -> Token {
    cur.bump();
    let value = match cur.peek(0) {
        Some('\\') => {
            cur.bump();
            Some(read_escape(cur, problems))
        }
        Some(c) if c != '\'' && c != '\n' => {
            cur.bump();
            Some(c)
        }
        _ => None,
    };
    if value.is_some() && cur.peek(0) == Some('\'') {
        cur.bump();
    } else {
        problems.push(Diag::at(
            CHAR_INVALID,
            line,
            col,
            "a character in single quotes is not closed",
        ));
        // leave the rest of the line to the next token
        while cur.peek(0).is_some_and(|c| c != '\'' && c != '\n') {
            cur.bump();
        }
        if cur.peek(0) == Some('\'') {
            cur.bump();
        }
    }
    Token {
        kind: Kind::Char,
        text: value.map(String::from).unwrap_or_default(),
        line,
        col,
    }
}

/// After `#include`, the name of a file between `<` and `>`.
fn read_angle_name(cur: &mut Cursor, problems: &mut Problems) -> Option<Token> {
    while cur.peek(0).is_some_and(|c| c == ' ' || c == '\t') {
        cur.bump();
    }
    if cur.peek(0) != Some('<') {
        return None;
    }
    let (line, col) = (cur.line, cur.col);
    cur.bump();
    let mut text = String::new();
    while let Some(c) = cur.peek(0) {
        if c == '>' || c == '\n' {
            break;
        }
        text.push(c);
        cur.bump();
    }
    if cur.peek(0) == Some('>') {
        cur.bump();
    } else {
        problems.push(Diag::at(DIRECTIVE_INVALID, line, col, "`<` without `>`"));
    }
    Some(Token {
        kind: Kind::Str,
        text,
        line,
        col,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Lexed {
        lex(text, &UlpSettings::default()).unwrap()
    }

    fn texts(l: &Lexed) -> Vec<&str> {
        l.tokens
            .iter()
            .filter(|t| t.kind != Kind::Eof)
            .map(|t| t.text.as_str())
            .collect()
    }

    #[test]
    fn names_numbers_texts_and_marks_are_told_apart() {
        let l = read("int a = 0x1F + 2.5e-3 * 7; string s = \"hi\\n\"; char c = 'x';");
        assert!(l.problems.list.is_empty(), "{:?}", l.problems);
        let kinds: Vec<Kind> = l.tokens.iter().map(|t| t.kind).collect();
        assert_eq!(kinds[0], Kind::Ident);
        assert!(kinds.contains(&Kind::Int) && kinds.contains(&Kind::Real));
        assert!(kinds.contains(&Kind::Str) && kinds.contains(&Kind::Char));
        assert_eq!(kinds.last(), Some(&Kind::Eof));
        assert!(texts(&l).contains(&"hi\n"));
    }

    #[test]
    fn a_token_has_its_line_and_column() {
        let l = read("a\n  bb;\n\tc");
        let at: Vec<(usize, usize)> = l.tokens.iter().map(|t| (t.line, t.col)).collect();
        assert_eq!(at[0], (1, 1));
        assert_eq!(at[1], (2, 3));
        assert_eq!(at[2], (2, 5));
        assert_eq!(at[3], (3, 2));
    }

    #[test]
    fn comments_are_skipped_and_an_open_one_is_a_problem_with_its_place() {
        let l = read("a // x\n/* y\nz */ b");
        assert_eq!(texts(&l), ["a", "b"]);
        let open = read("a\n  /* never closed");
        assert_eq!(open.problems.list.len(), 1);
        let p = &open.problems.list[0];
        assert_eq!((p.code, p.line, p.col), (COMMENT_OPEN, 2, 3));
    }

    #[test]
    fn a_text_that_is_not_closed_stops_at_the_end_of_the_line() {
        let l = read("s = \"abc\nint x;");
        assert_eq!(l.problems.list[0].code, STRING_OPEN);
        assert!(texts(&l).contains(&"int"));
    }

    #[test]
    fn the_escapes_are_read_and_a_wrong_one_is_a_problem() {
        let l = read(r#""a\tb\\c\"d\x41\101""#);
        assert!(l.problems.list.is_empty(), "{:?}", l.problems);
        assert_eq!(l.tokens[0].text, "a\tb\\c\"dAA");
        let bad = read(r#""\q \x""#);
        let codes: Vec<&str> = bad.problems.list.iter().map(|p| p.code).collect();
        assert_eq!(codes, [ESCAPE_INVALID, ESCAPE_INVALID]);
    }

    #[test]
    fn numbers_that_are_not_valid_are_problems() {
        for text in [
            "12abc",
            "0x",
            "1e",
            "1.2.3",
            "99999999999999999999",
            "0x1ffffffffffffffff",
        ] {
            let l = read(text);
            assert_eq!(l.problems.list.len(), 1, "{text}");
            assert_eq!(l.problems.list[0].code, NUMBER_INVALID, "{text}");
        }
        for text in ["0", "007", "1.", ".5", "1e5", "0xff", "2.5E+3"] {
            assert!(read(text).problems.list.is_empty(), "{text}");
        }
    }

    #[test]
    fn the_longest_mark_wins() {
        let l = read("a<<=b>>c<=d++ e&&f");
        assert_eq!(
            texts(&l),
            ["a", "<<=", "b", ">>", "c", "<=", "d", "++", "e", "&&", "f"]
        );
    }

    #[test]
    fn a_directive_is_a_token_and_the_name_of_an_include_in_angles_is_a_text() {
        let l = read("#include <std.ulp>\n#usage \"a\", \"b\"\n#require 6.0");
        assert!(l.problems.list.is_empty(), "{:?}", l.problems);
        let kinds: Vec<Kind> = l.tokens.iter().map(|t| t.kind).collect();
        assert_eq!(kinds[..3], [Kind::Directive, Kind::Str, Kind::Directive]);
        assert_eq!(l.tokens[1].text, "std.ulp");
        let open = read("#include <std.ulp\nint a;");
        assert_eq!(open.problems.list[0].code, DIRECTIVE_INVALID);
    }

    #[test]
    fn a_character_with_no_place_is_a_problem_and_the_reading_goes_on() {
        let l = read("a @ b $ c ` d");
        assert_eq!(texts(&l), ["a", "b", "c", "d"]);
        assert_eq!(l.problems.list.len(), 3);
        assert!(l.problems.list.iter().all(|p| p.code == CHAR_INVALID));
    }

    #[test]
    fn the_limits_stop_the_reading() {
        let small = UlpSettings {
            max_source_bytes: 10,
            ..UlpSettings::default()
        };
        assert_eq!(
            lex("12345678901", &small).unwrap_err().code,
            SOURCE_TOO_LARGE
        );
        let few = UlpSettings {
            max_tokens: 3,
            ..UlpSettings::default()
        };
        let e = lex("a b c d e", &few).unwrap_err();
        assert_eq!((e.code, e.line, e.col), (TOO_MANY_TOKENS, 1, 7));
        assert!(lex("a b c", &few).is_ok());
    }

    #[test]
    fn only_the_first_problems_are_kept_and_the_rest_are_counted() {
        let few = UlpSettings {
            max_problems: 2,
            ..UlpSettings::default()
        };
        let l = lex("@ @ @ @ @", &few).unwrap();
        assert_eq!(
            (
                l.problems.list.len(),
                l.problems.dropped,
                l.problems.total()
            ),
            (2, 3, 5)
        );
    }

    #[test]
    fn a_long_name_or_text_is_a_problem_and_is_cut() {
        let s = UlpSettings {
            max_name_chars: 4,
            max_string_chars: 3,
            ..UlpSettings::default()
        };
        let l = lex("abcdef \"abcdef\"", &s).unwrap();
        assert_eq!(l.problems.list.len(), 2);
        assert_eq!(l.tokens[1].text, "abc");
    }

    #[test]
    fn a_byte_order_mark_and_the_joined_lines_are_not_tokens() {
        let l = read("\u{feff}a \\\n b");
        assert_eq!(texts(&l), ["a", "b"]);
        assert!(l.problems.list.is_empty());
    }

    #[test]
    fn a_character_in_single_quotes_is_read() {
        let l = read("'a' '\\n' 'ab' ''");
        assert_eq!(l.tokens[0].text, "a");
        assert_eq!(l.tokens[1].text, "\n");
        assert!(l.problems.list.len() >= 2);
    }

    #[test]
    fn text_with_letters_from_other_languages_is_kept_inside_texts_and_comments() {
        let l = read("// ñandú 日本\ns = \"año ß 中\";");
        assert!(l.problems.list.is_empty(), "{:?}", l.problems);
        assert!(texts(&l).contains(&"año ß 中"));
    }
}
