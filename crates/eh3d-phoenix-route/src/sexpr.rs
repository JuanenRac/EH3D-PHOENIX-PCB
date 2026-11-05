// SPDX-License-Identifier: GPL-3.0-or-later
//! A reader of the bracketed text of the Specctra files: lists in round brackets, words and
//! words in double quotes. It checks the depth and the size and does nothing else; the meaning of
//! the lists belongs to the code that reads them.

use crate::codes::{RouteError, SES_UNREADABLE};

/// The deepest nesting of brackets that is read.
const MAX_DEPTH: usize = 64;

/// One element of the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Node {
    /// A word or a quoted word, without the quotes.
    Atom(String),
    /// A list.
    List(Vec<Node>),
}

impl Node {
    /// The word, when this is one.
    pub(crate) fn atom(&self) -> Option<&str> {
        match self {
            Node::Atom(text) => Some(text),
            Node::List(_) => None,
        }
    }

    /// The elements, when this is a list.
    pub(crate) fn items(&self) -> &[Node] {
        match self {
            Node::List(items) => items,
            Node::Atom(_) => &[],
        }
    }

    /// The first word of a list (its keyword).
    pub(crate) fn head(&self) -> Option<&str> {
        self.items().first().and_then(Node::atom)
    }

    /// The sub-lists whose keyword is `name`, in order.
    pub(crate) fn lists<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.items()
            .iter()
            .filter(move |n| matches!(n, Node::List(_)) && n.head() == Some(name))
    }

    /// The first sub-list whose keyword is `name`.
    pub(crate) fn list(&self, name: &str) -> Option<&Node> {
        self.lists(name).next()
    }
}

fn line_of(text: &str, at: usize) -> usize {
    1 + text.as_bytes()[..at.min(text.len())]
        .iter()
        .filter(|b| **b == b'\n')
        .count()
}

/// Reads a text that holds exactly one list.
pub(crate) fn parse(text: &str) -> Result<Node, RouteError> {
    let bytes = text.as_bytes();
    let bad = |at: usize, why: &str| {
        RouteError::new(SES_UNREADABLE, format!("line {}: {why}", line_of(text, at)))
    };
    let mut stack: Vec<Vec<Node>> = Vec::new();
    let mut root: Option<Node> = None;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b == b'(' {
            if stack.len() >= MAX_DEPTH {
                return Err(bad(i, "the brackets are nested too deep"));
            }
            if stack.is_empty() && root.is_some() {
                return Err(bad(i, "more than one list at the top"));
            }
            stack.push(Vec::new());
            i += 1;
            continue;
        }
        if b == b')' {
            let Some(done) = stack.pop() else {
                return Err(bad(i, "a closing bracket with no opening one"));
            };
            let node = Node::List(done);
            match stack.last_mut() {
                Some(parent) => parent.push(node),
                None => root = Some(node),
            }
            i += 1;
            continue;
        }
        let Some(parent) = stack.last_mut() else {
            return Err(bad(i, "a word outside the list"));
        };
        if b == b'"' {
            let start = i + 1;
            let Some(len) = text[start..].find('"') else {
                return Err(bad(i, "a quoted word is not closed"));
            };
            parent.push(Node::Atom(text[start..start + len].to_owned()));
            i = start + len + 1;
            continue;
        }
        let start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'('
            && bytes[i] != b')'
        {
            i += 1;
        }
        parent.push(Node::Atom(text[start..i].to_owned()));
    }
    if !stack.is_empty() {
        return Err(bad(bytes.len(), "a list is not closed"));
    }
    root.ok_or_else(|| bad(0, "the text holds no list"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_words_and_quoted_words_are_read() {
        let n = parse("(a (b 1 \"c d\") (b 2))").unwrap();
        assert_eq!(n.head(), Some("a"));
        let bs: Vec<&Node> = n.lists("b").collect();
        assert_eq!(bs.len(), 2);
        assert_eq!(bs[0].items()[2].atom(), Some("c d"));
        assert_eq!(n.list("b").unwrap().items()[1].atom(), Some("1"));
        assert!(n.list("z").is_none());
    }

    #[test]
    fn broken_text_is_refused_with_its_line() {
        for text in ["", "a", "(a", "(a))", "(a) (b)", "(a \"b)"] {
            let e = parse(text).unwrap_err();
            assert_eq!(e.code, SES_UNREADABLE, "{text:?}");
        }
        let e = parse("(a\n\n b))").unwrap_err();
        assert!(e.detail.starts_with("line 3"), "{}", e.detail);
    }

    #[test]
    fn the_nesting_is_limited() {
        let deep = format!("{}{}", "(".repeat(MAX_DEPTH + 1), ")".repeat(MAX_DEPTH + 1));
        assert!(parse(&deep).is_err());
        let ok = format!("{}{}", "(".repeat(MAX_DEPTH), ")".repeat(MAX_DEPTH));
        assert!(parse(&ok).is_ok());
    }
}
