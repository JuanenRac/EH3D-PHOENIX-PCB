// SPDX-License-Identifier: GPL-3.0-or-later
//! The log of the console panel, without a screen: the notes and the errors that the window had to say,
//! in the order they came, as many as the setting `console_max_lines` allows (the oldest go first).
//! A line that repeats the one before it is not written twice, so a note that the window shows at every
//! frame fills nothing. The log holds the text already in the language of the window, and nothing in it
//! reaches a file or the network.

use std::collections::VecDeque;

/// How serious a line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Something that was done or that is so.
    Info,
    /// A problem the person should know about.
    Error,
}

/// A line of the console.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The number of the line since the window started; it grows by one for each line that is written.
    pub number: u64,
    /// How serious it is.
    pub level: Level,
    /// The text.
    pub text: String,
}

/// The lines of the console.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleLog {
    lines: VecDeque<Line>,
    max: usize,
    next: u64,
    fed: Vec<String>,
}

/// How many sources of text a log follows at once (see [`ConsoleLog::feed`]).
pub const SOURCES: usize = 4;

impl ConsoleLog {
    /// An empty log that keeps at most `max_lines` lines (at least one).
    pub fn new(max_lines: usize) -> ConsoleLog {
        ConsoleLog {
            lines: VecDeque::new(),
            max: max_lines.max(1),
            next: 1,
            fed: vec![String::new(); SOURCES],
        }
    }

    /// Follows a text that the window shows at every frame (a note in the status bar, an error): the line
    /// is written when the text of that source changed since the last time, and never again while it
    /// stays the same. An empty text says that the source has nothing to say now, so the same text
    /// is written again when it comes back. `source` is a number below [`SOURCES`]; another is ignored.
    /// The answer says whether a line was written.
    pub fn feed(&mut self, source: usize, level: Level, text: &str) -> bool {
        let text = text.trim();
        let Some(last) = self.fed.get_mut(source) else {
            return false;
        };
        if last == text {
            return false;
        }
        *last = text.to_string();
        self.push(level, text)
    }

    /// Writes a line. A line equal to the last one (same level and same text) and a line with no text are
    /// not written; the answer says whether the line was written.
    pub fn push(&mut self, level: Level, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() {
            return false;
        }
        if self
            .lines
            .back()
            .is_some_and(|l| l.level == level && l.text == text)
        {
            return false;
        }
        while self.lines.len() >= self.max {
            self.lines.pop_front();
        }
        self.lines.push_back(Line {
            number: self.next,
            level,
            text: text.to_string(),
        });
        self.next += 1;
        true
    }

    /// The lines, the oldest first.
    pub fn lines(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter()
    }

    /// How many lines are kept.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether there is no line.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// How many of the lines kept are errors.
    pub fn errors(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| l.level == Level::Error)
            .count()
    }

    /// Empties the log; the numbers of the lines that come after go on from where they were.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// The lines as text, one for each line, the oldest first.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for l in &self.lines {
            out.push_str(&l.text);
            out.push('\n');
        }
        out
    }
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
    fn lines_come_in_order_and_are_numbered() {
        let mut log = ConsoleLog::new(10);
        assert!(log.is_empty());
        assert!(log.push(Level::Info, "opened"));
        assert!(log.push(Level::Error, "not saved"));
        assert!(log.push(Level::Info, "saved"));
        let numbers: Vec<u64> = log.lines().map(|l| l.number).collect();
        assert_eq!(numbers, [1, 2, 3]);
        assert_eq!(log.text(), "opened\nnot saved\nsaved\n");
        assert_eq!(log.errors(), 1);
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn a_line_that_repeats_the_one_before_is_not_written_twice() {
        let mut log = ConsoleLog::new(10);
        assert!(log.push(Level::Info, "saved"));
        assert!(!log.push(Level::Info, "saved"));
        assert!(!log.push(Level::Info, "  saved  "));
        // the same words as an error are another line
        assert!(log.push(Level::Error, "saved"));
        // the same words after another line are written again
        assert!(log.push(Level::Info, "saved"));
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn an_empty_line_is_not_written() {
        let mut log = ConsoleLog::new(3);
        assert!(!log.push(Level::Info, ""));
        assert!(!log.push(Level::Error, " \t\n"));
        assert!(log.is_empty());
    }

    #[test]
    fn the_oldest_lines_go_first_when_the_log_is_full() {
        let mut log = ConsoleLog::new(3);
        for i in 0..5 {
            log.push(Level::Info, &format!("line {i}"));
        }
        let text: Vec<&str> = log.lines().map(|l| l.text.as_str()).collect();
        assert_eq!(text, ["line 2", "line 3", "line 4"]);
        assert_eq!(log.lines().next().map(|l| l.number), Some(3));
    }

    #[test]
    fn a_log_that_may_hold_none_holds_one() {
        let mut log = ConsoleLog::new(0);
        log.push(Level::Info, "a");
        log.push(Level::Info, "b");
        assert_eq!(log.len(), 1);
        assert_eq!(log.text(), "b\n");
    }

    #[test]
    fn clearing_empties_the_log_and_the_numbers_go_on() {
        let mut log = ConsoleLog::new(5);
        log.push(Level::Info, "a");
        log.push(Level::Info, "b");
        log.clear();
        assert!(log.is_empty());
        assert!(log.push(Level::Info, "a"));
        assert_eq!(log.lines().next().map(|l| l.number), Some(3));
    }

    #[test]
    fn whatever_is_written_the_log_keeps_its_rules() {
        // many writes drawn from a generator with a fixed seed
        let mut seed = 0xc0_50_1e_u64;
        for max in [1_usize, 2, 7, 50] {
            let mut log = ConsoleLog::new(max);
            let mut last_written: Option<(Level, String)> = None;
            let mut written = 0u64;
            for _ in 0..500 {
                let n = lcg(&mut seed);
                let level = if n.is_multiple_of(3) {
                    Level::Error
                } else {
                    Level::Info
                };
                let text = format!("note {}", n % 5);
                let before = log.len();
                let did = log.push(level, &text);
                let repeat = last_written.as_ref() == Some(&(level, text.clone()));
                assert_eq!(did, !repeat, "{level:?} {text}");
                if did {
                    written += 1;
                    last_written = Some((level, text));
                } else {
                    assert_eq!(log.len(), before);
                }
                assert!(log.len() <= max);
                // the numbers rise by one from line to line, and the last one is the count written
                let numbers: Vec<u64> = log.lines().map(|l| l.number).collect();
                assert!(numbers.windows(2).all(|w| w[1] == w[0] + 1), "{numbers:?}");
                assert_eq!(numbers.last().copied(), Some(written));
                // no two neighbours are the same line
                let pairs: Vec<(Level, &str)> =
                    log.lines().map(|l| (l.level, l.text.as_str())).collect();
                assert!(pairs.windows(2).all(|w| w[0] != w[1]));
            }
        }
    }

    #[test]
    fn a_text_shown_at_every_frame_is_written_once_and_again_when_it_comes_back() {
        let mut log = ConsoleLog::new(10);
        assert!(log.feed(0, Level::Info, "saved"));
        for _ in 0..100 {
            assert!(!log.feed(0, Level::Info, "saved"));
        }
        assert_eq!(log.len(), 1);
        // two sources that alternate every frame do not fill the log
        for _ in 0..50 {
            log.feed(1, Level::Error, "file missing");
            log.feed(0, Level::Info, "saved");
        }
        assert_eq!(log.len(), 2);
        // the source that has nothing to say now, and then says the same again
        assert!(!log.feed(0, Level::Info, ""));
        assert!(log.feed(0, Level::Info, "saved"));
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn a_source_that_does_not_exist_writes_nothing() {
        let mut log = ConsoleLog::new(10);
        assert!(!log.feed(SOURCES, Level::Info, "x"));
        assert!(!log.feed(usize::MAX, Level::Error, "x"));
        assert!(log.is_empty());
    }

    #[test]
    fn what_is_fed_from_several_sources_keeps_the_rules_of_the_log() {
        // texts drawn from a generator with a fixed seed
        let mut seed = 0xfeed_u64;
        let mut log = ConsoleLog::new(20);
        for _ in 0..2000 {
            let n = lcg(&mut seed);
            let source = (n % 5) as usize;
            let text = if n.is_multiple_of(7) {
                String::new()
            } else {
                format!("t{}", (n >> 4) % 3)
            };
            let level = if source.is_multiple_of(2) {
                Level::Info
            } else {
                Level::Error
            };
            log.feed(source, level, &text);
            assert!(log.len() <= 20);
            let pairs: Vec<(Level, &str)> =
                log.lines().map(|l| (l.level, l.text.as_str())).collect();
            assert!(pairs.windows(2).all(|w| w[0] != w[1]));
            assert!(pairs.iter().all(|(_, t)| !t.is_empty()));
        }
    }
}
