// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading a KiCad netlist file (`.net`, the S-expression that a schematic exports) into the
//! table of the text netlist, read only: nothing is ever written in that format.
//!
//! ```text
//! (export (version "E")
//!   (nets
//!     (net (code "1") (name "GND")
//!       (node (ref "R1") (pin "1"))
//!       (node (ref "C1") (pin "2")))))
//! ```
//!
//! Only the list `nets` is read: for each `net` its `name` and its `node`s, each with a `ref` and
//! a `pin`. The rest of the file (the design, the components, the library parts) is skipped.
//! Names and words may be quoted (with `\"` and `\\`) or bare, as in the older files. The same
//! limits of `config/nets.cfg` as for the text netlist apply, and so does the way a name that the
//! text cannot hold is written (`table_of_pins`), which is told in the notes. The nesting of the
//! lists is limited so that a hostile file cannot overflow the stack.

use std::collections::BTreeMap;

use crate::nets_codes as codes;
use crate::nets_text::{table_of_pins, NetTable, NetsError, NetsSettings, Note};

/// The deepest nesting of lists that is read.
const MAX_DEPTH: usize = 64;

/// What a netlist exported by KiCad starts with after its opening parenthesis.
const ROOT_WORD: &str = "export";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Atom(String),
    List { line: usize, items: Vec<Node> },
}

impl Node {
    fn head(&self) -> Option<&str> {
        match self {
            Node::List { items, .. } => match items.first() {
                Some(Node::Atom(word)) => Some(word.as_str()),
                _ => None,
            },
            Node::Atom(_) => None,
        }
    }

    fn items(&self) -> &[Node] {
        match self {
            Node::List { items, .. } => items,
            Node::Atom(_) => &[],
        }
    }

    fn line(&self) -> usize {
        match self {
            Node::List { line, .. } => *line,
            Node::Atom(_) => 0,
        }
    }

    /// The first child list that starts with `word`.
    fn child(&self, word: &str) -> Option<&Node> {
        self.items()
            .iter()
            .skip(1)
            .find(|n| n.head().is_some_and(|h| h.eq_ignore_ascii_case(word)))
    }

    /// The word that follows the head of the child list `word`: `(name "GND")` gives `GND`.
    fn value_of(&self, word: &str) -> Option<&str> {
        match self.child(word)?.items().get(1) {
            Some(Node::Atom(value)) => Some(value.as_str()),
            _ => None,
        }
    }
}

/// True when the text looks like a KiCad netlist: its first word, inside the first parenthesis,
/// is `export`. A byte order mark and blanks before it are allowed.
pub fn looks_like_kicad_net(text: &str) -> bool {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text).trim_start();
    let Some(rest) = text.strip_prefix('(') else {
        return false;
    };
    let rest = rest.trim_start();
    rest.get(..ROOT_WORD.len())
        .is_some_and(|w| w.eq_ignore_ascii_case(ROOT_WORD))
        && rest[ROOT_WORD.len()..]
            .chars()
            .next()
            .is_none_or(|c| c.is_whitespace() || c == '(' || c == ')')
}

fn invalid(line: usize, detail: impl Into<String>) -> NetsError {
    NetsError::at(codes::LINE_INVALID, line, detail)
}

fn read_tree(text: &str) -> Result<Node, NetsError> {
    let mut stack: Vec<(usize, Vec<Node>)> = Vec::new();
    let mut root: Option<Node> = None;
    let mut line = 1usize;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => line += 1,
            c if c.is_whitespace() => {}
            '(' => {
                if root.is_some() {
                    return Err(invalid(line, "text after the end of the netlist"));
                }
                if stack.len() >= MAX_DEPTH {
                    return Err(NetsError::at(
                        codes::LIMIT_EXCEEDED,
                        line,
                        format!("lists are nested deeper than {MAX_DEPTH}"),
                    ));
                }
                stack.push((line, Vec::new()));
            }
            ')' => {
                let Some((opened, items)) = stack.pop() else {
                    return Err(invalid(line, "a closing parenthesis with none open"));
                };
                let node = Node::List {
                    line: opened,
                    items,
                };
                match stack.last_mut() {
                    Some((_, parent)) => parent.push(node),
                    None => root = Some(node),
                }
            }
            _ => {
                let mut word = String::new();
                if c == '"' {
                    let mut closed = false;
                    while let Some(d) = chars.next() {
                        match d {
                            '\\' => match chars.next() {
                                Some('n') => word.push('\n'),
                                Some(e) => word.push(e),
                                None => break,
                            },
                            '"' => {
                                closed = true;
                                break;
                            }
                            '\n' => {
                                line += 1;
                                word.push('\n');
                            }
                            d => word.push(d),
                        }
                    }
                    if !closed {
                        return Err(invalid(line, "a quoted word is never closed"));
                    }
                } else {
                    word.push(c);
                    while let Some(&d) = chars.peek() {
                        if d.is_whitespace() || d == '(' || d == ')' {
                            break;
                        }
                        word.push(d);
                        chars.next();
                    }
                }
                match stack.last_mut() {
                    Some((_, parent)) => parent.push(Node::Atom(word)),
                    None => return Err(invalid(line, "a word outside the list `export`")),
                }
            }
        }
    }
    if let Some((opened, _)) = stack.last() {
        return Err(invalid(*opened, "a list is never closed"));
    }
    root.ok_or_else(|| invalid(1, "the file holds no list"))
}

/// Reads a KiCad netlist and returns the table of its nets with the notes about the names that the
/// text netlist cannot hold. The first problem stops the reading, with its line: a file that does
/// not start with `(export`, one with no list `nets`, a net with no name, a node with no `ref` or
/// no `pin`, a name longer than `max_name_len`, or more nets, pins on a net or pins than the limits
/// of `settings` allow (`PHOENIX-NETS-002`, `-004` and `-006`).
pub fn read_kicad_net(
    text: &str,
    settings: &NetsSettings,
) -> Result<(NetTable, Vec<Note>), NetsError> {
    if text.len() as u64 > settings.max_input_bytes {
        return Err(NetsError::new(
            codes::LIMIT_EXCEEDED,
            format!(
                "the text is larger than max_input_bytes {}",
                settings.max_input_bytes
            ),
        ));
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let root = read_tree(text)?;
    if !root
        .head()
        .is_some_and(|h| h.eq_ignore_ascii_case(ROOT_WORD))
    {
        return Err(invalid(1, "a KiCad netlist starts with (export"));
    }
    let Some(nets) = root.child("nets") else {
        return Err(invalid(root.line(), "the netlist has no list `nets`"));
    };
    let mut pins: Vec<(String, String, String)> = Vec::new();
    let mut per_net: BTreeMap<String, usize> = BTreeMap::new();
    let mut net_count = 0usize;
    for net in nets.items().iter().skip(1) {
        if !net.head().is_some_and(|h| h.eq_ignore_ascii_case("net")) {
            continue;
        }
        let Some(name) = net.value_of("name") else {
            return Err(invalid(net.line(), "a net has no name"));
        };
        if name.chars().count() > settings.max_name_len {
            return Err(NetsError::at(
                codes::LIMIT_EXCEEDED,
                net.line(),
                "a name is longer than max_name_len",
            ));
        }
        net_count += 1;
        if net_count > settings.max_nets {
            return Err(NetsError::at(
                codes::LIMIT_EXCEEDED,
                net.line(),
                format!("more than max_nets {} nets", settings.max_nets),
            ));
        }
        for node in net.items().iter().skip(1) {
            if !node.head().is_some_and(|h| h.eq_ignore_ascii_case("node")) {
                continue;
            }
            let (Some(part), Some(pad)) = (node.value_of("ref"), node.value_of("pin")) else {
                return Err(NetsError::at(
                    codes::PIN_INVALID,
                    node.line(),
                    "a node needs a ref and a pin",
                ));
            };
            if part.chars().count() > settings.max_name_len
                || pad.chars().count() > settings.max_name_len
            {
                return Err(NetsError::at(
                    codes::LIMIT_EXCEEDED,
                    node.line(),
                    "a pin is longer than max_name_len",
                ));
            }
            let count = per_net.entry(name.to_owned()).or_insert(0);
            *count += 1;
            if *count > settings.max_pins_per_net {
                return Err(NetsError::at(
                    codes::LIMIT_EXCEEDED,
                    node.line(),
                    format!(
                        "more than max_pins_per_net {} pins on a net",
                        settings.max_pins_per_net
                    ),
                ));
            }
            if pins.len() >= settings.max_pins {
                return Err(NetsError::at(
                    codes::LIMIT_EXCEEDED,
                    node.line(),
                    format!("more than max_pins {} pins", settings.max_pins),
                ));
            }
            pins.push((name.to_owned(), part.to_owned(), pad.to_owned()));
        }
    }
    Ok(table_of_pins(pins))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"(export (version "E")
  (design (source "demo.kicad_sch") (tool "Eeschema 8.0"))
  (components
    (comp (ref "R1") (value "10k"))
    (comp (ref "C1") (value "100n")))
  (nets
    (net (code "1") (name "GND") (class "Default")
      (node (ref "R1") (pin "2") (pinfunction "~") (pintype "passive"))
      (node (ref "C1") (pin "2")))
    (net (code "2") (name "/SIG") (node (ref "R1") (pin "1")) (node (ref "C1") (pin "1")))))
"#;

    fn settings() -> NetsSettings {
        NetsSettings::default()
    }

    #[test]
    fn a_kicad_netlist_gives_its_nets_and_pins() {
        let (table, notes) = read_kicad_net(SAMPLE, &settings()).unwrap();
        assert!(notes.is_empty());
        assert_eq!(table.nets.len(), 2);
        assert_eq!(table.nets["GND"].len(), 2);
        assert_eq!(table.nets["/SIG"].len(), 2);
        assert_eq!(table.pin_count(), 4);
    }

    #[test]
    fn bare_words_and_any_case_are_read_as_in_the_older_files() {
        let text = "(EXPORT (version D) (nets (NET (code 1) (name GND) (node (ref U1) (pin 4)))))";
        let (table, _) = read_kicad_net(text, &settings()).unwrap();
        assert_eq!(table.nets["GND"].len(), 1);
    }

    #[test]
    fn the_file_is_recognised_by_its_first_word() {
        assert!(looks_like_kicad_net(SAMPLE));
        assert!(looks_like_kicad_net("\u{feff}  \n(export"));
        assert!(!looks_like_kicad_net("NET GND U1.1\n"));
        assert!(!looks_like_kicad_net("(exports)"));
        assert!(!looks_like_kicad_net(""));
    }

    #[test]
    fn escapes_in_a_quoted_name_are_read() {
        let text = r#"(export (nets (net (name "A\"B") (node (ref "R1") (pin "1")))))"#;
        let (table, notes) = read_kicad_net(text, &settings()).unwrap();
        assert!(table.nets.contains_key("A\"B"));
        assert!(notes.is_empty());
    }

    #[test]
    fn a_name_with_a_blank_is_written_with_underscores_and_noted() {
        let text = r#"(export (nets (net (name "MY NET") (node (ref "R1") (pin "1")))))"#;
        let (table, notes) = read_kicad_net(text, &settings()).unwrap();
        assert!(table.nets.contains_key("MY_NET"));
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn problems_stop_the_reading_with_their_line() {
        let cases = [
            ("(export (design))", codes::LINE_INVALID, 1),
            ("(other (nets))", codes::LINE_INVALID, 1),
            ("(export (nets\n (net (code 1))))", codes::LINE_INVALID, 2),
            (
                "(export (nets\n (net (name A)\n  (node (ref R1)))))",
                codes::PIN_INVALID,
                3,
            ),
            ("(export (nets", codes::LINE_INVALID, 1),
            ("(export) )", codes::LINE_INVALID, 1),
            ("(export (nets (net (name \"A", codes::LINE_INVALID, 1),
            ("hello", codes::LINE_INVALID, 1),
            ("(export) (export)", codes::LINE_INVALID, 1),
        ];
        for (text, code, line) in cases {
            let error = read_kicad_net(text, &settings()).unwrap_err();
            assert_eq!(error.code, code, "{text}");
            assert_eq!(error.line, line, "{text}");
        }
    }

    #[test]
    fn deep_nesting_is_refused_without_overflowing_the_stack() {
        let text = format!("(export {}{})", "(".repeat(5000), ")".repeat(5000));
        let error = read_kicad_net(&text, &settings()).unwrap_err();
        assert_eq!(error.code, codes::LIMIT_EXCEEDED);
    }

    #[test]
    fn the_limits_of_the_settings_apply() {
        let mut s = settings();
        s.max_nets = 1;
        assert_eq!(
            read_kicad_net(SAMPLE, &s).unwrap_err().code,
            codes::LIMIT_EXCEEDED
        );
        let mut s = settings();
        s.max_pins_per_net = 1;
        assert_eq!(
            read_kicad_net(SAMPLE, &s).unwrap_err().code,
            codes::LIMIT_EXCEEDED
        );
        let mut s = settings();
        s.max_pins = 3;
        assert_eq!(
            read_kicad_net(SAMPLE, &s).unwrap_err().code,
            codes::LIMIT_EXCEEDED
        );
        let mut s = settings();
        s.max_name_len = 2;
        assert_eq!(
            read_kicad_net(SAMPLE, &s).unwrap_err().code,
            codes::LIMIT_EXCEEDED
        );
        let mut s = settings();
        s.max_input_bytes = 10;
        assert_eq!(
            read_kicad_net(SAMPLE, &s).unwrap_err().code,
            codes::LIMIT_EXCEEDED
        );
    }

    #[test]
    fn a_pin_on_two_nets_stays_on_the_first_and_is_noted() {
        let text = "(export (nets (net (name A) (node (ref R1) (pin 1))) (net (name B) (node (ref R1) (pin 1)))))";
        let (table, notes) = read_kicad_net(text, &settings()).unwrap();
        assert_eq!(table.nets.len(), 1);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].code, codes::PIN_ON_TWO_NETS);
    }

    /// A small linear congruential generator, so the cases are the same on every run.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
    }

    #[test]
    fn a_table_written_as_a_kicad_netlist_is_read_back_the_same() {
        let mut rng = Lcg(0x5eed_1234);
        for _round in 0..40 {
            let nets = 1 + (rng.next() % 6) as usize;
            let mut pins: Vec<(String, String, String)> = Vec::new();
            let mut used = std::collections::BTreeSet::new();
            for n in 0..nets {
                let count = 1 + (rng.next() % 5) as usize;
                for _ in 0..count {
                    let part = format!("U{}", rng.next() % 9 + 1);
                    let pad = format!("{}", rng.next() % 12 + 1);
                    if used.insert((part.clone(), pad.clone())) {
                        pins.push((format!("N{n}"), part, pad));
                    }
                }
            }
            let (table, _) = table_of_pins(pins);
            let mut text = String::from("(export (version \"E\") (nets\n");
            for (index, (name, set)) in table.ordered().into_iter().enumerate() {
                text.push_str(&format!("(net (code \"{index}\") (name \"{name}\")\n"));
                for pin in set {
                    text.push_str(&format!(
                        "  (node (ref \"{}\") (pin \"{}\"))\n",
                        pin.part, pin.pad
                    ));
                }
                text.push_str(")\n");
            }
            text.push_str("))\n");
            let (back, notes) = read_kicad_net(&text, &settings()).unwrap();
            assert!(notes.is_empty());
            assert_eq!(back, table);
        }
    }
}
