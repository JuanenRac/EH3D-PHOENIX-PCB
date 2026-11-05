// SPDX-License-Identifier: GPL-3.0-or-later
//! A bounded reader of the s-expressions that KiCad 6 to 9 write.
//!
//! The text is a tree of lists whose items are atoms (`yes`, `1.27`), quoted strings (`"R1"`) or
//! lists. The reader keeps the line and the column of every item, uses no recursion (the nesting
//! is a counter, so the depth cannot overflow the stack), refuses what the settings limit (size,
//! depth, nodes, length of a text) with a typed error and never panics: a text that ends early, a
//! parenthesis that does not match or a string that is never closed gives an error that says
//! where.

use crate::error::KicadError;
use crate::settings::KicadSettings;

/// What a node holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// A word without quotes.
    Atom(String),
    /// A text between double quotes, with its escapes resolved.
    Str(String),
    /// A list.
    List(Vec<Node>),
}

/// One item of the tree with the place it starts at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// The line, counted from 1.
    pub line: u32,
    /// The column, counted from 1 in characters.
    pub column: u32,
    /// The content.
    pub value: Value,
}

impl Node {
    /// The items of a list, `None` for an atom or a string.
    pub fn list(&self) -> Option<&[Node]> {
        match &self.value {
            Value::List(items) => Some(items),
            _ => None,
        }
    }

    /// The text of an atom or a string, `None` for a list.
    pub fn text(&self) -> Option<&str> {
        match &self.value {
            Value::Atom(t) | Value::Str(t) => Some(t),
            Value::List(_) => None,
        }
    }

    /// The name of a list: its first item when that is an atom or a string.
    pub fn head(&self) -> Option<&str> {
        self.list()?.first()?.text()
    }

    /// The items of a list after its name (empty for anything else).
    pub fn args(&self) -> &[Node] {
        match self.list() {
            Some(items) if !items.is_empty() => &items[1..],
            _ => &[],
        }
    }

    /// The text of the argument number `index`.
    pub fn arg_text(&self, index: usize) -> Option<&str> {
        self.args().get(index)?.text()
    }

    /// The first sub-list called `name`.
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.args().iter().find(|n| n.head() == Some(name))
    }

    /// Every sub-list called `name`, in order.
    pub fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.args().iter().filter(move |n| n.head() == Some(name))
    }

    /// True when the arguments hold the bare word `word` (as in `(pin passive line hide)`).
    pub fn has_word(&self, word: &str) -> bool {
        self.args()
            .iter()
            .any(|n| matches!(&n.value, Value::Atom(t) if t == word))
    }
}

fn clamp32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn is_delimiter(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'(' | b')' | b'"')
}

/// Reads a text into its root list. The text must hold exactly one list.
pub fn parse(text: &str, settings: &KicadSettings) -> Result<Node, KicadError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let bytes = text.as_bytes();
    if bytes.len() as u64 > settings.max_file_bytes {
        return Err(KicadError::Limit {
            what: "the size of the text in bytes".to_owned(),
            value: bytes.len() as u64,
            limit: settings.max_file_bytes,
            line: 0,
            column: 0,
        });
    }
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut stack: Vec<(Vec<Node>, usize, usize)> = Vec::new();
    let mut root: Option<Node> = None;
    let mut nodes = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'\n' => {
                line += 1;
                col = 1;
                i += 1;
            }
            b' ' | b'\t' | b'\r' => {
                col += 1;
                i += 1;
            }
            b'(' => {
                if root.is_some() {
                    return Err(KicadError::Syntax {
                        line,
                        column: col,
                        reason: "a second list after the end of the first".to_owned(),
                    });
                }
                if stack.len() >= settings.max_depth {
                    return Err(KicadError::Limit {
                        what: "the nesting of lists".to_owned(),
                        value: stack.len() as u64 + 1,
                        limit: settings.max_depth as u64,
                        line,
                        column: col,
                    });
                }
                nodes += 1;
                if nodes > settings.max_nodes {
                    return Err(KicadError::Limit {
                        what: "the number of nodes".to_owned(),
                        value: nodes as u64,
                        limit: settings.max_nodes as u64,
                        line,
                        column: col,
                    });
                }
                stack.push((Vec::new(), line, col));
                i += 1;
                col += 1;
            }
            b')' => {
                let Some((items, open_line, open_col)) = stack.pop() else {
                    return Err(KicadError::Syntax {
                        line,
                        column: col,
                        reason: "a `)` that closes nothing".to_owned(),
                    });
                };
                let node = Node {
                    line: clamp32(open_line),
                    column: clamp32(open_col),
                    value: Value::List(items),
                };
                match stack.last_mut() {
                    Some(parent) => parent.0.push(node),
                    None => root = Some(node),
                }
                i += 1;
                col += 1;
            }
            b'"' => {
                let (start_line, start_col) = (line, col);
                i += 1;
                col += 1;
                let mut buffer: Vec<u8> = Vec::new();
                let mut closed = false;
                while i < bytes.len() {
                    let c = bytes[i];
                    i += 1;
                    match c {
                        b'"' => {
                            col += 1;
                            closed = true;
                            break;
                        }
                        b'\\' => {
                            col += 1;
                            let Some(&next) = bytes.get(i) else {
                                break;
                            };
                            i += 1;
                            if next & 0xC0 != 0x80 {
                                col += 1;
                            }
                            if next == b'\n' {
                                line += 1;
                                col = 1;
                            }
                            buffer.push(match next {
                                b'n' => b'\n',
                                b't' => b'\t',
                                b'r' => b'\r',
                                other => other,
                            });
                        }
                        b'\n' => {
                            line += 1;
                            col = 1;
                            buffer.push(c);
                        }
                        other => {
                            if other & 0xC0 != 0x80 {
                                col += 1;
                            }
                            buffer.push(other);
                        }
                    }
                    if buffer.len() > settings.max_string_bytes {
                        return Err(KicadError::Limit {
                            what: "the length of a quoted text in bytes".to_owned(),
                            value: buffer.len() as u64,
                            limit: settings.max_string_bytes as u64,
                            line: start_line,
                            column: start_col,
                        });
                    }
                }
                if !closed {
                    return Err(KicadError::Truncated {
                        line,
                        column: col,
                        open: stack.len(),
                    });
                }
                let content = match String::from_utf8(buffer) {
                    Ok(s) => s,
                    Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
                };
                push_leaf(
                    &mut stack,
                    root.is_some(),
                    &mut nodes,
                    settings,
                    Node {
                        line: clamp32(start_line),
                        column: clamp32(start_col),
                        value: Value::Str(content),
                    },
                )?;
            }
            _ => {
                let start = i;
                let start_col = col;
                while i < bytes.len() && !is_delimiter(bytes[i]) {
                    if bytes[i] & 0xC0 != 0x80 {
                        col += 1;
                    }
                    i += 1;
                }
                if i - start > settings.max_string_bytes {
                    return Err(KicadError::Limit {
                        what: "the length of an atom in bytes".to_owned(),
                        value: (i - start) as u64,
                        limit: settings.max_string_bytes as u64,
                        line,
                        column: start_col,
                    });
                }
                let word = text.get(start..i).unwrap_or("").to_owned();
                push_leaf(
                    &mut stack,
                    root.is_some(),
                    &mut nodes,
                    settings,
                    Node {
                        line: clamp32(line),
                        column: clamp32(start_col),
                        value: Value::Atom(word),
                    },
                )?;
            }
        }
    }
    if !stack.is_empty() {
        return Err(KicadError::Truncated {
            line,
            column: col,
            open: stack.len(),
        });
    }
    root.ok_or_else(|| KicadError::Syntax {
        line,
        column: col,
        reason: "the text holds no list".to_owned(),
    })
}

/// Adds an atom or a string to the innermost open list. Outside a list it is an error.
fn push_leaf(
    stack: &mut [(Vec<Node>, usize, usize)],
    finished: bool,
    nodes: &mut usize,
    settings: &KicadSettings,
    node: Node,
) -> Result<(), KicadError> {
    let (line, column) = (node.line as usize, node.column as usize);
    *nodes += 1;
    if *nodes > settings.max_nodes {
        return Err(KicadError::Limit {
            what: "the number of nodes".to_owned(),
            value: *nodes as u64,
            limit: settings.max_nodes as u64,
            line,
            column,
        });
    }
    match stack.last_mut() {
        Some(parent) => {
            parent.0.push(node);
            Ok(())
        }
        None => Err(KicadError::Syntax {
            line,
            column,
            reason: if finished {
                "text after the end of the list".to_owned()
            } else {
                "text before the first `(`".to_owned()
            },
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s() -> KicadSettings {
        KicadSettings::default()
    }

    #[test]
    fn a_small_tree_keeps_its_shape_and_its_places() {
        let root = parse("(kicad_pcb (version 20211014)\n  (name \"a b\"))", &s()).unwrap();
        assert_eq!(root.head(), Some("kicad_pcb"));
        let version = root.child("version").unwrap();
        assert_eq!(version.arg_text(0), Some("20211014"));
        assert_eq!((version.line, version.column), (1, 12));
        let name = root.child("name").unwrap();
        assert_eq!(name.arg_text(0), Some("a b"));
        assert_eq!((name.line, name.column), (2, 3));
        assert!(root.child("missing").is_none());
    }

    #[test]
    fn escapes_and_multi_line_strings_are_resolved() {
        let root = parse("(t \"a\\\"b\\\\c\\nd\" \"x\ny\")", &s()).unwrap();
        assert_eq!(root.arg_text(0), Some("a\"b\\c\nd"));
        assert_eq!(root.arg_text(1), Some("x\ny"));
        let root = parse("(t \"é字\" é)", &s()).unwrap();
        assert_eq!(root.arg_text(0), Some("é字"));
        assert_eq!(root.arg_text(1), Some("é"));
    }

    #[test]
    fn a_byte_order_mark_and_trailing_space_are_accepted() {
        let root = parse("\u{feff}(a (b))\n\n  \t", &s()).unwrap();
        assert_eq!(root.head(), Some("a"));
        assert!(!root.has_word("b"));
        let root = parse("(a hide (b))", &s()).unwrap();
        assert!(root.has_word("hide"));
    }

    #[test]
    fn children_are_found_in_order() {
        let root = parse("(a (x 1) (y 2) (x 3))", &s()).unwrap();
        let values: Vec<&str> = root.children("x").filter_map(|n| n.arg_text(0)).collect();
        assert_eq!(values, vec!["1", "3"]);
    }

    #[test]
    fn a_text_that_ends_early_says_where_and_how_many_lists_are_open() {
        match parse("(a\n (b (c 1)\n", &s()) {
            Err(KicadError::Truncated { line, open, .. }) => {
                assert_eq!(line, 3);
                assert_eq!(open, 2);
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            parse("(a \"never closed", &s()),
            Err(KicadError::Truncated { open: 1, .. })
        ));
        assert!(matches!(
            parse("(a \"cut by a backslash\\", &s()),
            Err(KicadError::Truncated { .. })
        ));
    }

    #[test]
    fn damaged_text_gives_a_syntax_error_with_a_position() {
        match parse("(a))", &s()) {
            Err(KicadError::Syntax { line, column, .. }) => assert_eq!((line, column), (1, 4)),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            parse("(a) (b)", &s()),
            Err(KicadError::Syntax { column: 5, .. })
        ));
        assert!(matches!(
            parse("word (a)", &s()),
            Err(KicadError::Syntax { column: 1, .. })
        ));
        assert!(matches!(
            parse("(a) word", &s()),
            Err(KicadError::Syntax { column: 5, .. })
        ));
        assert!(matches!(parse("", &s()), Err(KicadError::Syntax { .. })));
        assert!(matches!(
            parse("   \n", &s()),
            Err(KicadError::Syntax { .. })
        ));
    }

    #[test]
    fn the_limits_of_the_settings_are_enforced() {
        let mut small = s();
        small.max_depth = 3;
        assert!(parse("(a (b (c)))", &small).is_ok());
        assert!(matches!(
            parse("(a (b (c (d))))", &small),
            Err(KicadError::Limit { limit: 3, .. })
        ));
        small = s();
        small.max_nodes = 4;
        assert!(parse("(a b c d)", &small).is_err());
        assert!(parse("(a b c)", &small).is_ok());
        small = s();
        small.max_string_bytes = 5;
        assert!(parse("(a \"123456\")", &small).is_err());
        assert!(parse("(a 123456789)", &small).is_err());
        assert!(parse("(a 12345)", &small).is_ok());
        small = s();
        small.max_file_bytes = 6;
        assert!(matches!(
            parse("(a b c)", &small),
            Err(KicadError::Limit { .. })
        ));
    }

    #[test]
    fn a_very_deep_text_is_refused_without_overflowing_the_stack() {
        let deep = "(".repeat(200_000);
        assert!(matches!(parse(&deep, &s()), Err(KicadError::Limit { .. })));
        let mut loose = s();
        loose.max_depth = 1000;
        let closed = format!("{}{}", "(".repeat(1000), ")".repeat(1000));
        assert!(parse(&closed, &loose).is_ok());
        let one_more = format!("{}{}", "(".repeat(1001), ")".repeat(1001));
        assert!(parse(&one_more, &loose).is_err());
    }

    #[test]
    fn no_cut_of_a_valid_text_panics() {
        let text = "(kicad_sch (version 20230121) (generator eeschema)\n (wire (pts (xy 1.5 2) (xy 3 4)) (uuid \"é-1\")))\n";
        for cut in 0..text.len() {
            if text.is_char_boundary(cut) {
                let _ = parse(&text[..cut], &s());
            }
        }
        assert!(parse(text, &s()).is_ok());
    }
}
