// SPDX-License-Identifier: GPL-3.0-or-later
//! The patterns of `strxstr`: the common part of regular expressions, run without going back.
//!
//! A pattern has characters, `.`, classes `[abc]`, `[a-z]` and `[^...]`, `\d`, `\w`, `\s` (and
//! `\D`, `\W`, `\S`), a backslash before a character that would mean something else, `^` and `$`,
//! groups in parentheses, `|`, and the repetitions `*`, `+` and `?`. Counted repetitions
//! (`{2,5}`), back references and the other constructs are refused. The search keeps every way
//! that is still possible at once, so its cost grows with the product of the lengths of the text
//! and of the pattern and never faster; the work is counted and bounded by the caller. Of the
//! matches that start first, the longest is the answer.

use crate::codes::{RUN_ARGUMENTS, RUN_NOT_WRITTEN, RUN_STEP_LIMIT};
use crate::world::Fault;

/// The longest pattern, in characters.
const MAX_PATTERN_CHARS: usize = 512;
/// The deepest nesting of groups.
const MAX_GROUPS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Class {
    Digit,
    Word,
    Space,
    Range(char, char),
}

impl Class {
    fn holds(&self, c: char) -> bool {
        match self {
            Class::Digit => c.is_ascii_digit(),
            Class::Word => c.is_alphanumeric() || c == '_',
            Class::Space => c.is_whitespace(),
            Class::Range(low, high) => (*low..=*high).contains(&c),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    /// A character of one of the classes (or of none of them when `not`).
    Set { classes: Vec<Class>, not: bool },
    /// Any character.
    Any,
    /// The start of the text.
    Start,
    /// The end of the text.
    End,
    /// Go on at both places, given from this one.
    Split(isize, isize),
    /// Go on at a place given from this one.
    Jump(isize),
}

/// A pattern ready to search.
#[derive(Debug, Clone)]
pub struct Pattern {
    steps: Vec<Step>,
}

fn bad(what: &str) -> Fault {
    (RUN_ARGUMENTS, format!("the pattern is not valid: {what}"))
}

fn one(class: Class) -> Step {
    Step::Set {
        classes: vec![class],
        not: false,
    }
}

fn literal(c: char) -> Step {
    one(Class::Range(c, c))
}

/// What a backslash and a character mean.
fn escape(c: char) -> Result<(Class, bool), Fault> {
    Ok(match c {
        'd' => (Class::Digit, false),
        'D' => (Class::Digit, true),
        'w' => (Class::Word, false),
        'W' => (Class::Word, true),
        's' => (Class::Space, false),
        'S' => (Class::Space, true),
        'n' => (Class::Range('\n', '\n'), false),
        't' => (Class::Range('\t', '\t'), false),
        'r' => (Class::Range('\r', '\r'), false),
        c if c.is_alphanumeric() => return Err((RUN_NOT_WRITTEN, format!("`\\{c}` in a pattern"))),
        c => (Class::Range(c, c), false),
    })
}

struct Reader<'a> {
    chars: &'a [char],
    at: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        self.at += 1;
        c
    }

    fn alternatives(&mut self, depth: usize) -> Result<Vec<Step>, Fault> {
        if depth > MAX_GROUPS {
            return Err(bad("too many groups inside each other"));
        }
        let mut result = self.sequence(depth)?;
        while self.peek() == Some('|') {
            self.at += 1;
            let other = self.sequence(depth)?;
            let (a, b) = (result.len() as isize, other.len() as isize);
            let mut joined = vec![Step::Split(1, a + 2)];
            joined.append(&mut result);
            joined.push(Step::Jump(b + 1));
            joined.extend(other);
            result = joined;
        }
        Ok(result)
    }

    fn sequence(&mut self, depth: usize) -> Result<Vec<Step>, Fault> {
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.atom(depth)?;
            out.extend(self.repeated(atom)?);
        }
        Ok(out)
    }

    fn repeated(&mut self, mut atom: Vec<Step>) -> Result<Vec<Step>, Fault> {
        let Some(mark) = self.peek().filter(|c| matches!(c, '*' | '+' | '?')) else {
            return Ok(atom);
        };
        self.at += 1;
        if matches!(self.peek(), Some('*' | '+' | '?' | '{')) {
            return Err((
                RUN_NOT_WRITTEN,
                "a repetition of a repetition in a pattern".to_owned(),
            ));
        }
        let len = atom.len() as isize;
        Ok(match mark {
            '*' => {
                let mut out = vec![Step::Split(1, len + 2)];
                out.append(&mut atom);
                out.push(Step::Jump(-(len + 1)));
                out
            }
            '+' => {
                atom.push(Step::Split(-len, 1));
                atom
            }
            _ => {
                let mut out = vec![Step::Split(1, len + 1)];
                out.append(&mut atom);
                out
            }
        })
    }

    fn atom(&mut self, depth: usize) -> Result<Vec<Step>, Fault> {
        let Some(c) = self.next() else {
            return Err(bad("it ends too soon"));
        };
        Ok(vec![match c {
            '.' => Step::Any,
            '^' => Step::Start,
            '$' => Step::End,
            '(' => {
                let inner = self.alternatives(depth + 1)?;
                if self.next() != Some(')') {
                    return Err(bad("a group is not closed"));
                }
                return Ok(inner);
            }
            '[' => self.class()?,
            '\\' => {
                let c = self
                    .next()
                    .ok_or_else(|| bad("a backslash ends the pattern"))?;
                let (class, not) = escape(c)?;
                Step::Set {
                    classes: vec![class],
                    not,
                }
            }
            '*' | '+' | '?' => return Err(bad("a repetition of nothing")),
            '{' => return Err((RUN_NOT_WRITTEN, "`{` in a pattern".to_owned())),
            other => literal(other),
        }])
    }

    fn class(&mut self) -> Result<Step, Fault> {
        let not = self.peek() == Some('^');
        if not {
            self.at += 1;
        }
        let mut classes = Vec::new();
        let mut first = true;
        loop {
            let c = self.next().ok_or_else(|| bad("a class is not closed"))?;
            if c == ']' && !first {
                break;
            }
            first = false;
            if c == '\\' {
                let e = self.next().ok_or_else(|| bad("a class is not closed"))?;
                let (class, negated) = escape(e)?;
                if negated {
                    return Err((RUN_NOT_WRITTEN, format!("`\\{e}` inside a class")));
                }
                classes.push(class);
                continue;
            }
            if self.peek() == Some('-') && self.chars.get(self.at + 1).is_some_and(|n| *n != ']') {
                self.at += 1;
                let high = self.next().ok_or_else(|| bad("a class is not closed"))?;
                if high < c {
                    return Err(bad("a range is backwards"));
                }
                classes.push(Class::Range(c, high));
            } else {
                classes.push(Class::Range(c, c));
            }
        }
        Ok(Step::Set { classes, not })
    }
}

impl Pattern {
    /// Reads a pattern.
    pub fn new(text: &str) -> Result<Pattern, Fault> {
        let chars: Vec<char> = text.chars().take(MAX_PATTERN_CHARS + 1).collect();
        if chars.len() > MAX_PATTERN_CHARS {
            return Err(bad("it is too long"));
        }
        let mut reader = Reader {
            chars: &chars,
            at: 0,
        };
        let steps = reader.alternatives(0)?;
        if reader.at < chars.len() {
            return Err(bad("a `)` closes nothing"));
        }
        Ok(Pattern { steps })
    }

    /// Adds a place and the places it leads to without taking a character.
    fn add(&self, list: &mut Vec<usize>, seen: &mut [bool], place: usize, at: usize, end: usize) {
        let mut pending = vec![place];
        while let Some(place) = pending.pop() {
            if seen[place] {
                continue;
            }
            seen[place] = true;
            let to = |offset: isize| place.saturating_add_signed(offset);
            match self.steps.get(place) {
                Some(Step::Split(a, b)) => {
                    pending.push(to(*a));
                    pending.push(to(*b));
                }
                Some(Step::Jump(a)) => pending.push(to(*a)),
                Some(Step::Start) if at == 0 => pending.push(place + 1),
                Some(Step::End) if at == end => pending.push(place + 1),
                Some(Step::Start | Step::End) => {}
                // a step that takes a character, or the end of the pattern
                _ => list.push(place),
            }
        }
    }

    /// The length of the longest match that starts at `start`.
    fn longest_at(
        &self,
        text: &[char],
        start: usize,
        work: &mut u64,
        budget: u64,
    ) -> Result<Option<usize>, Fault> {
        let size = self.steps.len() + 1;
        let mut now = Vec::new();
        let mut seen = vec![false; size];
        self.add(&mut now, &mut seen, 0, start, text.len());
        let mut best = None;
        let mut at = start;
        loop {
            *work += 1 + now.len() as u64;
            if *work > budget {
                return Err((
                    RUN_STEP_LIMIT,
                    "the search of a pattern is too long".to_owned(),
                ));
            }
            if now.contains(&self.steps.len()) {
                best = Some(at - start);
            }
            let Some(&c) = text.get(at) else { break };
            let mut next = Vec::new();
            let mut seen = vec![false; size];
            for &place in &now {
                let takes = match self.steps.get(place) {
                    Some(Step::Any) => true,
                    Some(Step::Set { classes, not }) => classes.iter().any(|k| k.holds(c)) != *not,
                    _ => false,
                };
                if takes {
                    self.add(&mut next, &mut seen, place + 1, at + 1, text.len());
                }
            }
            if next.is_empty() {
                break;
            }
            now = next;
            at += 1;
        }
        Ok(best)
    }

    /// The first match at or after the character `from`: where it starts and how many characters
    /// it has. `work` counts what the search did and `budget` bounds it.
    pub fn find(
        &self,
        text: &[char],
        from: usize,
        work: &mut u64,
        budget: u64,
    ) -> Result<Option<(usize, usize)>, Fault> {
        for start in from..=text.len() {
            if let Some(length) = self.longest_at(text, start, work, budget)? {
                return Ok(Some((start, length)));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(pattern: &str, text: &str, from: usize) -> Option<(usize, usize)> {
        let chars: Vec<char> = text.chars().collect();
        let mut work = 0;
        Pattern::new(pattern)
            .unwrap()
            .find(&chars, from, &mut work, 1_000_000)
            .unwrap()
    }

    #[test]
    fn characters_classes_repetitions_groups_and_anchors_match() {
        assert_eq!(find("b+", "abbbc", 0), Some((1, 3)));
        assert_eq!(find("R[0-9]+", "C1 R12 R3", 0), Some((3, 3)));
        assert_eq!(find("R[0-9]+", "C1 R12 R3", 4), Some((7, 2)));
        assert_eq!(find("^C\\d", "C1 R12", 0), Some((0, 2)));
        assert_eq!(find("^R", "C1 R12", 0), None);
        assert_eq!(find("12$", "C1 R12", 0), Some((4, 2)));
        assert_eq!(find("(ab|c)+d", "xxababcd", 0), Some((2, 6)));
        assert_eq!(find("a.?c", "ac abc", 1), Some((3, 3)));
        assert_eq!(find("[^a-c ]+", "abc def", 0), Some((4, 3)));
        assert_eq!(find("\\.\\w*", "top.brd", 0), Some((3, 4)));
        assert_eq!(find("[]a]+", "x]a]", 0), Some((1, 3)));
        assert_eq!(find("", "abc", 2), Some((2, 0)));
        assert_eq!(find("é+", "café é", 0), Some((3, 1)));
    }

    #[test]
    fn what_is_not_written_or_not_valid_is_refused_with_its_code() {
        for pattern in ["a{2}", "(a)\\1", "a**", "a+?", "\\b"] {
            assert_eq!(
                Pattern::new(pattern).unwrap_err().0,
                RUN_NOT_WRITTEN,
                "{pattern}"
            );
        }
        for pattern in ["(a", "a)", "[a", "*a", "\\", "[z-a]"] {
            assert_eq!(
                Pattern::new(pattern).unwrap_err().0,
                RUN_ARGUMENTS,
                "{pattern}"
            );
        }
        let long = "a".repeat(MAX_PATTERN_CHARS + 1);
        assert!(Pattern::new(&long).is_err());
        let deep = format!("{}a{}", "(".repeat(40), ")".repeat(40));
        assert!(Pattern::new(&deep).is_err());
    }

    #[test]
    fn a_pattern_that_would_go_back_forever_elsewhere_ends_inside_the_budget() {
        let chars = vec!['a'; 2_000];
        let pattern = Pattern::new("(a*)*b").unwrap();
        let mut work = 0;
        let found = pattern.find(&chars, 0, &mut work, 100_000);
        assert_eq!(found.unwrap_err().0, RUN_STEP_LIMIT);
    }

    #[test]
    fn random_patterns_and_texts_never_panic() {
        // a fixed seed: the same strings on every run
        let alphabet: Vec<char> = "ab1 .[]()|*+?^$\\-{d".chars().collect();
        let mut seed = 0x7a31_9c4d_2e5b_8f17u64;
        let mut next = move |span: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % span as u64) as usize
        };
        for _ in 0..5_000 {
            let make = |len: usize, next: &mut dyn FnMut(usize) -> usize| -> String {
                (0..len).map(|_| alphabet[next(alphabet.len())]).collect()
            };
            let pattern = make(next(10), &mut next);
            let text: Vec<char> = make(next(12), &mut next).chars().collect();
            if let Ok(p) = Pattern::new(&pattern) {
                let mut work = 0;
                if let Ok(Some((start, length))) = p.find(&text, 0, &mut work, 100_000) {
                    assert!(start + length <= text.len(), "{pattern:?}");
                }
            }
        }
    }
}
