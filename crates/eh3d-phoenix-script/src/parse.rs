// SPDX-License-Identifier: GPL-3.0-or-later
//! The reader of script files.
//!
//! A script is a list of commands. A command is a word followed by arguments and ends at a
//! semicolon or, when the settings say so, at a line break (a backslash at the end of a line joins
//! the next line). A `#` that starts a token begins a comment that runs to the end of the line.
//! An argument is a word (`0.2mm`, `R90`, `GND`), a text in single quotes (`'a b'`; two quotes in a
//! row stand for one) or a coordinate in parentheses (`(10 5.5)`).
//!
//! Nothing here panics: a text that is not closed, a coordinate that is not valid, a missing command
//! name and every limit of the settings become an issue with the line and column, the command that
//! caused it is dropped and the reading goes on (a limit that cannot be passed ends the reading).

use crate::codes::{Diag, LIMIT_EXCEEDED, SYNTAX_COMMAND, SYNTAX_COORD, SYNTAX_STRING};
use crate::settings::ScriptSettings;

/// What an argument is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgKind {
    /// A word, as written.
    Word(String),
    /// A text in quotes, without the quotes.
    Text(String),
    /// A coordinate: the two words between the parentheses.
    Point(String, String),
}

/// One argument of a command and its place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arg {
    /// What it is.
    pub kind: ArgKind,
    /// The line, from 1.
    pub line: usize,
    /// The column, from 1.
    pub col: usize,
}

impl Arg {
    /// The text of a word or of a quoted text.
    pub fn text(&self) -> Option<&str> {
        match &self.kind {
            ArgKind::Word(w) | ArgKind::Text(w) => Some(w),
            ArgKind::Point(_, _) => None,
        }
    }
}

/// A command of a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// The name in capitals.
    pub name: String,
    /// The arguments, in order.
    pub args: Vec<Arg>,
    /// The line of the name, from 1.
    pub line: usize,
    /// The column of the name, from 1.
    pub col: usize,
}

/// A script read: its commands and what could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Parsed {
    /// The commands, in order.
    pub commands: Vec<Command>,
    /// The problems found, in order.
    pub issues: Vec<Diag>,
}

struct Lexer {
    chars: Vec<char>,
    at: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn peek_at(&self, ahead: usize) -> Option<char> {
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

fn is_word_end(c: char) -> bool {
    c.is_whitespace() || matches!(c, ';' | '(' | '\'')
}

/// What the reader is building between two ends of command.
struct Builder<'a> {
    settings: &'a ScriptSettings,
    parsed: Parsed,
    current: Option<Command>,
    dropped: bool,
    stop: bool,
}

impl Builder<'_> {
    fn finish(&mut self) {
        if let Some(command) = self.current.take() {
            self.parsed.commands.push(command);
        }
        self.dropped = false;
    }

    fn issue(&mut self, code: &'static str, line: usize, col: usize, detail: &str) {
        self.parsed.issues.push(Diag::new(code, line, col, detail));
        self.current = None;
        self.dropped = true;
    }

    fn add(&mut self, kind: ArgKind, line: usize, col: usize) {
        if self.dropped || self.stop {
            return;
        }
        if let Some(count) = self.current.as_ref().map(|c| c.args.len()) {
            if count >= self.settings.max_args {
                let detail = format!(
                    "more than {} arguments in one command",
                    self.settings.max_args
                );
                self.issue(LIMIT_EXCEEDED, line, col, &detail);
            } else if let Some(command) = self.current.as_mut() {
                command.args.push(Arg { kind, line, col });
            }
            return;
        }
        match kind {
            ArgKind::Word(word) => {
                if self.parsed.commands.len() >= self.settings.max_commands {
                    let detail = format!("more than {} commands", self.settings.max_commands);
                    self.issue(LIMIT_EXCEEDED, line, col, &detail);
                    self.stop = true;
                    return;
                }
                self.current = Some(Command {
                    name: word.to_ascii_uppercase(),
                    args: Vec::new(),
                    line,
                    col,
                });
            }
            _ => self.issue(
                SYNTAX_COMMAND,
                line,
                col,
                "the line does not start with a word",
            ),
        }
    }
}

/// Reads a script. A script larger than the limit, or with more lines than the limit, is not read
/// at all and gives one issue.
pub fn parse_script(text: &str, settings: &ScriptSettings) -> Parsed {
    if text.len() > settings.max_script_bytes {
        return Parsed {
            commands: Vec::new(),
            issues: vec![Diag::new(
                LIMIT_EXCEEDED,
                1,
                1,
                format!(
                    "the script is larger than {} bytes",
                    settings.max_script_bytes
                ),
            )],
        };
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let lines = text.matches('\n').count() + 1;
    if lines > settings.max_lines {
        return Parsed {
            commands: Vec::new(),
            issues: vec![Diag::new(
                LIMIT_EXCEEDED,
                1,
                1,
                format!("the script has more than {} lines", settings.max_lines),
            )],
        };
    }
    let mut lx = Lexer {
        chars: text.chars().collect(),
        at: 0,
        line: 1,
        col: 1,
    };
    let mut b = Builder {
        settings,
        parsed: Parsed::default(),
        current: None,
        dropped: false,
        stop: false,
    };
    while let Some(c) = lx.peek() {
        if b.stop {
            break;
        }
        match c {
            '\n' => {
                lx.bump();
                if settings.newline_ends_command {
                    b.finish();
                }
            }
            c if c.is_whitespace() => {
                lx.bump();
            }
            ';' => {
                lx.bump();
                b.finish();
            }
            '#' => {
                while let Some(n) = lx.peek() {
                    if n == '\n' {
                        break;
                    }
                    lx.bump();
                }
            }
            '\\' if lx.peek_at(1) == Some('\n')
                || (lx.peek_at(1) == Some('\r') && lx.peek_at(2) == Some('\n')) =>
            {
                while let Some(n) = lx.bump() {
                    if n == '\n' {
                        break;
                    }
                }
            }
            '\'' => read_text(&mut lx, &mut b),
            '(' => read_point(&mut lx, &mut b),
            _ => {
                let (line, col) = (lx.line, lx.col);
                let mut word = String::new();
                while let Some(n) = lx.peek() {
                    if is_word_end(n) {
                        break;
                    }
                    word.push(n);
                    lx.bump();
                }
                if word.chars().count() > settings.max_string_chars {
                    let detail = format!(
                        "a word is longer than {} characters",
                        settings.max_string_chars
                    );
                    b.issue(LIMIT_EXCEEDED, line, col, &detail);
                } else {
                    b.add(ArgKind::Word(word), line, col);
                }
            }
        }
    }
    b.finish();
    b.parsed
}

fn read_text(lx: &mut Lexer, b: &mut Builder<'_>) {
    let (line, col) = (lx.line, lx.col);
    lx.bump();
    let mut text = String::new();
    loop {
        match lx.peek() {
            None | Some('\n') => {
                b.issue(SYNTAX_STRING, line, col, "the text in quotes is not closed");
                return;
            }
            Some('\'') => {
                lx.bump();
                if lx.peek() == Some('\'') {
                    lx.bump();
                    text.push('\'');
                } else {
                    break;
                }
            }
            Some(c) => {
                text.push(c);
                lx.bump();
            }
        }
    }
    if text.chars().count() > b.settings.max_string_chars {
        let detail = format!(
            "a text is longer than {} characters",
            b.settings.max_string_chars
        );
        b.issue(LIMIT_EXCEEDED, line, col, &detail);
    } else {
        b.add(ArgKind::Text(text), line, col);
    }
}

fn read_point(lx: &mut Lexer, b: &mut Builder<'_>) {
    let (line, col) = (lx.line, lx.col);
    lx.bump();
    let mut inside = String::new();
    loop {
        match lx.peek() {
            Some(')') => {
                lx.bump();
                break;
            }
            None | Some('\n') | Some('(') | Some('\'') | Some(';') => {
                b.issue(SYNTAX_COORD, line, col, "the coordinate is not closed");
                return;
            }
            Some(c) => {
                inside.push(c);
                lx.bump();
            }
        }
    }
    let parts: Vec<&str> = inside.split_whitespace().collect();
    match parts.as_slice() {
        [x, y] => b.add(ArgKind::Point((*x).to_owned(), (*y).to_owned()), line, col),
        _ => b.issue(SYNTAX_COORD, line, col, "a coordinate has two numbers"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Parsed {
        parse_script(text, &ScriptSettings::default())
    }

    fn names(p: &Parsed) -> Vec<&str> {
        p.commands.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn commands_end_at_semicolons_and_at_line_breaks() {
        let p = parse("GRID mm 0.5; LAYER Top\nWIRE 'GND' 0.3 (0 0) (10 5.5);\n");
        assert!(p.issues.is_empty());
        assert_eq!(names(&p), vec!["GRID", "LAYER", "WIRE"]);
        assert_eq!(p.commands[0].args.len(), 2);
        let wire = &p.commands[2];
        assert_eq!(wire.args.len(), 4);
        assert_eq!(wire.args[0].kind, ArgKind::Text("GND".to_owned()));
        assert_eq!(wire.args[1].kind, ArgKind::Word("0.3".to_owned()));
        assert_eq!(
            wire.args[3].kind,
            ArgKind::Point("10".to_owned(), "5.5".to_owned())
        );
    }

    #[test]
    fn names_are_in_capitals_and_places_count_from_one() {
        let p = parse("\n  grid mm;\nwire (1 2) (3 4)");
        assert_eq!(names(&p), vec!["GRID", "WIRE"]);
        assert_eq!((p.commands[0].line, p.commands[0].col), (2, 3));
        assert_eq!((p.commands[1].line, p.commands[1].col), (3, 1));
        assert_eq!(
            (p.commands[1].args[0].line, p.commands[1].args[0].col),
            (3, 6)
        );
    }

    #[test]
    fn comments_run_to_the_end_of_the_line() {
        let p = parse("# only a comment\nGRID mm # trailing\nLAYER Top;# more\n");
        assert!(p.issues.is_empty());
        assert_eq!(names(&p), vec!["GRID", "LAYER"]);
        assert_eq!(p.commands[0].args.len(), 1);
    }

    #[test]
    fn a_hash_inside_quotes_or_a_word_is_not_a_comment() {
        let p = parse("TEXT 'a # b' (0 0)\nNAME ab#c d");
        assert!(p.issues.is_empty());
        assert_eq!(
            p.commands[0].args[0].kind,
            ArgKind::Text("a # b".to_owned())
        );
        assert_eq!(p.commands[1].args[0].kind, ArgKind::Word("ab#c".to_owned()));
    }

    #[test]
    fn two_quotes_in_a_row_are_one_quote() {
        let p = parse("VALUE R1 'it''s'");
        assert_eq!(p.commands[0].args[1].kind, ArgKind::Text("it's".to_owned()));
        let empty = parse("VALUE R1 ''");
        assert_eq!(empty.commands[0].args[1].kind, ArgKind::Text(String::new()));
    }

    #[test]
    fn a_backslash_joins_two_lines_into_one_command() {
        let p = parse("WIRE 'A' 0.2 \\\n (0 0) \\\r\n (1 1)\nGRID mm");
        assert!(p.issues.is_empty());
        assert_eq!(names(&p), vec!["WIRE", "GRID"]);
        assert_eq!(p.commands[0].args.len(), 4);
    }

    #[test]
    fn with_the_setting_off_only_the_semicolon_ends_a_command() {
        let settings = ScriptSettings {
            newline_ends_command: false,
            ..ScriptSettings::default()
        };
        let p = parse_script("WIRE 'A'\n (0 0)\n (1 1);\nGRID mm;", &settings);
        assert_eq!(names(&p), vec!["WIRE", "GRID"]);
        assert_eq!(p.commands[0].args.len(), 3);
    }

    #[test]
    fn a_text_that_is_not_closed_drops_its_command_and_the_next_ones_are_read() {
        let p = parse("GRID mm\nTEXT 'oops (0 0)\nLAYER Top");
        assert_eq!(names(&p), vec!["GRID", "LAYER"]);
        assert_eq!(p.issues.len(), 1);
        assert_eq!(p.issues[0].code, SYNTAX_STRING);
        assert_eq!((p.issues[0].line, p.issues[0].col), (2, 6));
    }

    #[test]
    fn a_coordinate_that_is_not_valid_is_an_issue() {
        for (text, col) in [
            ("MOVE R1 (1 2", 9),
            ("MOVE R1 (1)", 9),
            ("MOVE R1 (1 2 3)", 9),
            ("MOVE R1 ((1 2))", 9),
            ("MOVE R1 ()", 9),
        ] {
            let p = parse(text);
            assert_eq!(p.issues.len(), 1, "{text}");
            assert_eq!(p.issues[0].code, SYNTAX_COORD, "{text}");
            assert_eq!(p.issues[0].col, col, "{text}");
            assert!(p.commands.is_empty(), "{text}");
        }
    }

    #[test]
    fn a_line_that_starts_with_text_or_a_coordinate_has_no_command_name() {
        let p = parse("'GND' 1\n(1 2)\nGRID mm");
        assert_eq!(names(&p), vec!["GRID"]);
        assert_eq!(p.issues.len(), 2);
        assert!(p.issues.iter().all(|i| i.code == SYNTAX_COMMAND));
        assert_eq!(p.issues[1].line, 2);
    }

    #[test]
    fn the_limits_of_the_settings_are_issues_and_never_a_panic() {
        let s = ScriptSettings {
            max_script_bytes: 20,
            ..ScriptSettings::default()
        };
        let p = parse_script("GRID mm 0.5; GRID mm 0.5; GRID", &s);
        assert!(p.commands.is_empty());
        assert_eq!(p.issues[0].code, LIMIT_EXCEEDED);

        let s = ScriptSettings {
            max_lines: 2,
            ..ScriptSettings::default()
        };
        assert_eq!(parse_script("a\nb\nc", &s).issues[0].code, LIMIT_EXCEEDED);
        assert!(parse_script("a\nb", &s).issues.is_empty());

        let s = ScriptSettings {
            max_commands: 2,
            ..ScriptSettings::default()
        };
        let p = parse_script("A;B;C;D", &s);
        assert_eq!(names(&p), vec!["A", "B"]);
        assert_eq!(p.issues.len(), 1);
        assert_eq!(p.issues[0].code, LIMIT_EXCEEDED);

        let s = ScriptSettings {
            max_args: 2,
            ..ScriptSettings::default()
        };
        let p = parse_script("A 1 2 3;B 1", &s);
        assert_eq!(names(&p), vec!["B"]);
        assert_eq!(p.issues[0].code, LIMIT_EXCEEDED);

        let s = ScriptSettings {
            max_string_chars: 3,
            ..ScriptSettings::default()
        };
        let p = parse_script("A 'abcd';B 'abc';C abcd", &s);
        assert_eq!(names(&p), vec!["B"]);
        assert_eq!(p.issues.len(), 2);
    }

    #[test]
    fn the_same_text_always_gives_the_same_result() {
        let text = "GRID mm 0.5\nWIRE 'N' (0 0) (1 1)\nBAD 'x\nLAYER Top;";
        assert_eq!(parse(text), parse(text));
    }

    #[test]
    fn text_of_odd_characters_does_not_panic() {
        for text in [
            "",
            ";",
            "\n\n",
            "\\",
            "\\\n",
            "'",
            "(",
            ")",
            "é;ü",
            "\u{0}",
            "a\u{feff}b",
            "A\u{a0}B\u{3000}C",
        ] {
            let _ = parse(text);
        }
        let p = parse("A\u{a0}B\u{3000}C");
        assert_eq!(p.commands[0].args.len(), 2);
    }
}
