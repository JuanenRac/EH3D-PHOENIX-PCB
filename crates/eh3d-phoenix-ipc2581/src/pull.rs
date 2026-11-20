// SPDX-License-Identifier: GPL-3.0-or-later
//! A pull parser for the XML of an IPC-2581 file.
//!
//! The files of a board can be tens of megabytes, so the reader does not build the tree of the
//! whole document. The parser hands out one event at a time (an element opens, an element
//! closes) and can read one subtree into a small tree ([`Node`]) when the reader wants all of an
//! element at once. It never recurses, so the depth of the input cannot overflow the stack, and
//! it refuses input over its [`PullLimits`]. Text between elements is not kept: the format of
//! IPC-2581 carries everything in attributes. Comments, processing instructions, a document type
//! and CDATA sections are skipped; names are read without their namespace prefix.

use std::borrow::Cow;
use std::fmt;

/// The limits of one parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PullLimits {
    /// Largest input in bytes.
    pub max_bytes: usize,
    /// Deepest nesting of elements.
    pub max_depth: usize,
    /// Most elements in the document.
    pub max_elements: usize,
    /// Most attributes on one element.
    pub max_attrs: usize,
}

impl Default for PullLimits {
    fn default() -> Self {
        PullLimits {
            max_bytes: 128 * 1024 * 1024,
            max_depth: 64,
            max_elements: 50_000_000,
            max_attrs: 256,
        }
    }
}

/// Why a parse stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullError {
    /// The text is not well-formed XML; the byte offset says where.
    Malformed {
        /// Byte offset of the problem.
        at: usize,
        /// What was expected.
        what: &'static str,
    },
    /// A limit was exceeded.
    Limit(&'static str),
}

impl fmt::Display for PullError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PullError::Malformed { at, what } => write!(f, "malformed XML at byte {at}: {what}"),
            PullError::Limit(what) => write!(f, "limit exceeded: {what}"),
        }
    }
}

impl std::error::Error for PullError {}

/// The opening of an element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag<'a> {
    /// The name without a namespace prefix.
    pub name: &'a str,
    /// The attributes in file order, names without a prefix, values with entities resolved.
    pub attrs: Vec<(&'a str, Cow<'a, str>)>,
}

impl<'a> Tag<'a> {
    /// The value of an attribute.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.as_ref())
    }
}

/// One step of the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<'a> {
    /// An element opens (an empty element gives this and then [`Event::End`]).
    Start(Tag<'a>),
    /// An element closes; the name is the one that opened.
    End(&'a str),
}

/// A subtree read whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<'a> {
    /// The opening tag.
    pub tag: Tag<'a>,
    /// The child elements in order.
    pub children: Vec<Node<'a>>,
}

impl<'a> Node<'a> {
    /// The name of the element.
    pub fn name(&self) -> &'a str {
        self.tag.name
    }

    /// The value of an attribute.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.tag.attr(name)
    }

    /// The first child with a name.
    pub fn child(&self, name: &str) -> Option<&Node<'a>> {
        self.children.iter().find(|c| c.tag.name == name)
    }

    /// The children with a name.
    pub fn children_named<'s>(&'s self, name: &'s str) -> impl Iterator<Item = &'s Node<'a>> + 's {
        self.children.iter().filter(move |c| c.tag.name == name)
    }

    /// The number of elements of the subtree, itself included.
    pub fn count(&self) -> usize {
        let mut total = 0;
        let mut stack = vec![self];
        while let Some(n) = stack.pop() {
            total += 1;
            stack.extend(n.children.iter());
        }
        total
    }
}

/// The parser.
pub struct Pull<'a> {
    text: &'a str,
    s: &'a [u8],
    i: usize,
    limits: PullLimits,
    open: Vec<&'a str>,
    elements: usize,
    pending_end: Option<&'a str>,
    root_done: bool,
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':') || b >= 0x80
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Resolves the five named entities and the numeric references; anything else is kept as written.
fn unescape(raw: &str) -> Cow<'_, str> {
    if !raw.contains('&') {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        let resolved = match rest.find(';') {
            Some(e) if e <= 12 => {
                let ent = &rest[1..e];
                let ch = match ent {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    _ => ent
                        .strip_prefix("#x")
                        .or_else(|| ent.strip_prefix("#X"))
                        .and_then(|h| u32::from_str_radix(h, 16).ok())
                        .or_else(|| ent.strip_prefix('#').and_then(|d| d.parse::<u32>().ok()))
                        .and_then(char::from_u32),
                };
                ch.map(|c| (c, e + 1))
            }
            _ => None,
        };
        match resolved {
            Some((c, used)) => {
                out.push(c);
                rest = &rest[used..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

impl<'a> Pull<'a> {
    /// A parser over `text`; a leading byte order mark is skipped.
    pub fn new(text: &'a str, limits: PullLimits) -> Result<Pull<'a>, PullError> {
        if text.len() > limits.max_bytes {
            return Err(PullError::Limit("the input is larger than the byte limit"));
        }
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        Ok(Pull {
            text,
            s: text.as_bytes(),
            i: 0,
            limits,
            open: Vec::new(),
            elements: 0,
            pending_end: None,
            root_done: false,
        })
    }

    /// The byte offset reached.
    pub fn position(&self) -> usize {
        self.i
    }

    /// How many elements are open now.
    pub fn depth(&self) -> usize {
        self.open.len()
    }

    /// How many elements have opened so far.
    pub fn elements(&self) -> usize {
        self.elements
    }

    fn bad<T>(&self, what: &'static str) -> Result<T, PullError> {
        Err(PullError::Malformed { at: self.i, what })
    }

    fn skip_space(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn skip_until(&mut self, end: &[u8], what: &'static str) -> Result<(), PullError> {
        match find(&self.s[self.i..], end) {
            Some(p) => {
                self.i += p + end.len();
                Ok(())
            }
            None => self.bad(what),
        }
    }

    fn name(&mut self) -> Result<&'a str, PullError> {
        let start = self.i;
        while self.i < self.s.len() && is_name_byte(self.s[self.i]) {
            self.i += 1;
        }
        if start == self.i {
            return self.bad("a name");
        }
        Ok(&self.text[start..self.i])
    }

    fn skip_doctype(&mut self) -> Result<(), PullError> {
        let mut depth = 0usize;
        while self.i < self.s.len() {
            match self.s[self.i] {
                b'[' => depth += 1,
                b']' => depth = depth.saturating_sub(1),
                b'"' | b'\'' => {
                    let q = self.s[self.i];
                    self.i += 1;
                    while self.i < self.s.len() && self.s[self.i] != q {
                        self.i += 1;
                    }
                }
                b'>' if depth == 0 => {
                    self.i += 1;
                    return Ok(());
                }
                _ => {}
            }
            self.i += 1;
        }
        self.bad("the end of the document type")
    }

    fn attr_value(&mut self) -> Result<Cow<'a, str>, PullError> {
        let q = match self.s.get(self.i) {
            Some(&q) if q == b'"' || q == b'\'' => q,
            _ => return self.bad("a quoted attribute value"),
        };
        self.i += 1;
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i] != q {
            if self.s[self.i] == b'<' {
                return self.bad("no < inside an attribute value");
            }
            self.i += 1;
        }
        if self.i >= self.s.len() {
            return self.bad("the end of an attribute value");
        }
        let raw = &self.text[start..self.i];
        self.i += 1;
        Ok(unescape(raw))
    }

    /// The next event; `None` at the end of the document.
    pub fn next_event(&mut self) -> Result<Option<Event<'a>>, PullError> {
        if let Some(name) = self.pending_end.take() {
            return Ok(Some(Event::End(name)));
        }
        loop {
            // character data between elements is not kept
            while self.i < self.s.len() && self.s[self.i] != b'<' {
                self.i += 1;
            }
            if self.i >= self.s.len() {
                if self.open.is_empty() && self.root_done {
                    return Ok(None);
                }
                return self.bad("the end of the document: an element is not closed");
            }
            if self.starts(b"<!--") {
                self.i += 4;
                self.skip_until(b"-->", "the end of a comment")?;
            } else if self.starts(b"<![CDATA[") {
                self.i += 9;
                self.skip_until(b"]]>", "the end of a CDATA section")?;
            } else if self.starts(b"<!") {
                self.i += 2;
                self.skip_doctype()?;
            } else if self.starts(b"<?") {
                self.i += 2;
                self.skip_until(b"?>", "the end of a processing instruction")?;
            } else if self.starts(b"</") {
                self.i += 2;
                let name = local(self.name()?);
                self.skip_space();
                if self.s.get(self.i) != Some(&b'>') {
                    return self.bad("> to end a closing tag");
                }
                self.i += 1;
                match self.open.pop() {
                    Some(open) if open == name => {
                        if self.open.is_empty() {
                            self.root_done = true;
                        }
                        return Ok(Some(Event::End(open)));
                    }
                    _ => return self.bad("a closing tag that matches the open element"),
                }
            } else {
                return self.start_tag().map(Some);
            }
        }
    }

    fn starts(&self, p: &[u8]) -> bool {
        self.s[self.i..].starts_with(p)
    }

    fn start_tag(&mut self) -> Result<Event<'a>, PullError> {
        if self.root_done {
            return self.bad("only one root element");
        }
        self.i += 1;
        let name = local(self.name()?);
        let mut attrs: Vec<(&'a str, Cow<'a, str>)> = Vec::new();
        let empty;
        loop {
            self.skip_space();
            match self.s.get(self.i) {
                Some(b'>') => {
                    self.i += 1;
                    empty = false;
                    break;
                }
                Some(b'/') => {
                    if self.s.get(self.i + 1) != Some(&b'>') {
                        return self.bad("> after / in a tag");
                    }
                    self.i += 2;
                    empty = true;
                    break;
                }
                Some(_) => {
                    if attrs.len() >= self.limits.max_attrs {
                        return Err(PullError::Limit("an element has too many attributes"));
                    }
                    let raw_key = self.name()?;
                    let key = local(raw_key);
                    self.skip_space();
                    if self.s.get(self.i) != Some(&b'=') {
                        return self.bad("= after an attribute name");
                    }
                    self.i += 1;
                    self.skip_space();
                    let value = self.attr_value()?;
                    if raw_key == "xmlns" || raw_key.starts_with("xmlns:") {
                        // a namespace declaration is not data
                        continue;
                    }
                    if attrs.iter().any(|(k, _)| *k == key) {
                        return self.bad("an attribute that is not repeated");
                    }
                    attrs.push((key, value));
                }
                None => return self.bad("the end of a tag"),
            }
        }
        self.elements += 1;
        if self.elements > self.limits.max_elements {
            return Err(PullError::Limit("the document has too many elements"));
        }
        if self.open.len() >= self.limits.max_depth {
            return Err(PullError::Limit("the elements are nested too deep"));
        }
        if empty {
            self.pending_end = Some(name);
            if self.open.is_empty() {
                self.root_done = true;
            }
        } else {
            self.open.push(name);
        }
        Ok(Event::Start(Tag { name, attrs }))
    }

    /// Reads the rest of the element that `tag` opened, as a tree. The element must be the last
    /// one that [`Pull::next_event`] returned.
    pub fn subtree(&mut self, tag: Tag<'a>) -> Result<Node<'a>, PullError> {
        let mut stack: Vec<Node<'a>> = vec![Node {
            tag,
            children: Vec::new(),
        }];
        loop {
            match self.next_event()? {
                None => return self.bad("the end of the document inside an element"),
                Some(Event::Start(t)) => stack.push(Node {
                    tag: t,
                    children: Vec::new(),
                }),
                Some(Event::End(_)) => {
                    let done = stack.pop();
                    match (done, stack.last_mut()) {
                        (Some(node), Some(parent)) => parent.children.push(node),
                        (Some(node), None) => return Ok(node),
                        (None, _) => return self.bad("an element to close"),
                    }
                }
            }
        }
    }

    /// Skips the rest of the element that was just opened; returns how many elements were in it,
    /// the element itself included.
    pub fn skip_subtree(&mut self) -> Result<usize, PullError> {
        let mut depth = 1usize;
        let mut count = 1usize;
        while depth > 0 {
            match self.next_event()? {
                None => return self.bad("the end of the document inside an element"),
                Some(Event::Start(_)) => {
                    depth += 1;
                    count += 1;
                }
                Some(Event::End(_)) => depth -= 1,
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(text: &str) -> Result<Vec<String>, PullError> {
        let mut p = Pull::new(text, PullLimits::default())?;
        let mut out = Vec::new();
        while let Some(e) = p.next_event()? {
            out.push(match e {
                Event::Start(t) => {
                    let attrs: Vec<String> =
                        t.attrs.iter().map(|(k, v)| format!("{k}={v}")).collect();
                    format!("<{} {}>", t.name, attrs.join(","))
                }
                Event::End(n) => format!("</{n}>"),
            });
        }
        Ok(out)
    }

    #[test]
    fn elements_attributes_and_empty_elements_come_in_order() {
        let got = events("<?xml version=\"1.0\"?><!-- c --><a x=\"1\" y='2'><b/><c z=\"&lt;&amp;&#65;&#x42;\"></c></a>").unwrap();
        assert_eq!(
            got,
            ["<a x=1,y=2>", "<b >", "</b>", "<c z=<&AB>", "</c>", "</a>"]
        );
    }

    #[test]
    fn prefixes_the_byte_order_mark_doctype_and_cdata_are_handled() {
        let got = events("\u{feff}<!DOCTYPE a [<!ENTITY e \"x\">]><n:a xmlns:n=\"u\" n:k=\"v\"><![CDATA[<b>]]>text</n:a>").unwrap();
        assert_eq!(got, ["<a k=v>", "</a>"]);
    }

    #[test]
    fn a_subtree_is_read_whole_and_counted() {
        let text = "<r><a k=\"1\"><b/><b/><c><d/></c></a><e/></r>";
        let mut p = Pull::new(text, PullLimits::default()).unwrap();
        assert!(matches!(p.next_event().unwrap(), Some(Event::Start(_))));
        let a = match p.next_event().unwrap() {
            Some(Event::Start(t)) => p.subtree(t).unwrap(),
            other => panic!("{other:?}"),
        };
        assert_eq!(a.name(), "a");
        assert_eq!(a.attr("k"), Some("1"));
        assert_eq!(a.children_named("b").count(), 2);
        assert_eq!(a.child("c").unwrap().child("d").unwrap().name(), "d");
        assert_eq!(a.count(), 5);
        match p.next_event().unwrap() {
            Some(Event::Start(t)) => assert_eq!(t.name, "e"),
            other => panic!("{other:?}"),
        }
        assert_eq!(p.skip_subtree().unwrap(), 1);
        assert!(matches!(p.next_event().unwrap(), Some(Event::End("r"))));
        assert_eq!(p.next_event().unwrap(), None);
    }

    #[test]
    fn skipping_counts_the_elements() {
        let mut p = Pull::new("<r><a><b/><b/></a></r>", PullLimits::default()).unwrap();
        p.next_event().unwrap();
        p.next_event().unwrap();
        assert_eq!(p.skip_subtree().unwrap(), 3);
        assert_eq!(p.elements(), 4);
    }

    #[test]
    fn malformed_input_is_an_error_and_never_a_panic() {
        for text in [
            "",
            "text only",
            "<a>",
            "<a></b>",
            "<a><b></a></b>",
            "<a x=1></a>",
            "<a x=\"1></a>",
            "<a x=\"1\" x=\"2\"/>",
            "<a/><b/>",
            "<a",
            "<a x",
            "<a x=",
            "<a / >",
            "<!-- open",
            "<a><![CDATA[ open",
            "<?pi",
            "<a x=\"<\"/>",
            "</a>",
            "<>",
            "<a></a",
        ] {
            assert!(events(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn the_limits_are_enforced() {
        let small = PullLimits {
            max_bytes: 10,
            ..PullLimits::default()
        };
        assert!(matches!(
            Pull::new("<a>0123456789</a>", small),
            Err(PullError::Limit(_))
        ));
        let deep = PullLimits {
            max_depth: 3,
            ..PullLimits::default()
        };
        let mut p = Pull::new("<a><b><c><d/></c></b></a>", deep).unwrap();
        let mut err = None;
        loop {
            match p.next_event() {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(e) => {
                    err = Some(e);
                    break;
                }
            }
        }
        assert!(matches!(err, Some(PullError::Limit(_))));
        let few = PullLimits {
            max_elements: 2,
            ..PullLimits::default()
        };
        let mut p = Pull::new("<a><b/><b/></a>", few).unwrap();
        assert!(p.next_event().is_ok());
        assert!(p.next_event().is_ok());
        assert!(p.next_event().is_ok());
        assert!(matches!(p.next_event(), Err(PullError::Limit(_))));
        let attrs = PullLimits {
            max_attrs: 2,
            ..PullLimits::default()
        };
        let mut p = Pull::new("<a x=\"1\" y=\"2\" z=\"3\"/>", attrs).unwrap();
        assert!(matches!(p.next_event(), Err(PullError::Limit(_))));
    }

    #[test]
    fn a_very_deep_document_does_not_overflow_the_stack() {
        let limits = PullLimits {
            max_depth: 100_000,
            ..PullLimits::default()
        };
        let mut text = String::new();
        for _ in 0..50_000 {
            text.push_str("<a>");
        }
        for _ in 0..50_000 {
            text.push_str("</a>");
        }
        // skipping and counting is iterative, however deep the input is
        let mut p = Pull::new(&text, limits).unwrap();
        assert!(matches!(p.next_event().unwrap(), Some(Event::Start(_))));
        assert_eq!(p.skip_subtree().unwrap(), 50_000);
        assert_eq!(p.next_event().unwrap(), None);
        // a tree of the deepest nesting the settings allow is built and counted without recursion
        let limits = PullLimits {
            max_depth: 1000,
            ..PullLimits::default()
        };
        let text = format!("{}{}", "<a>".repeat(1000), "</a>".repeat(1000));
        let mut p = Pull::new(&text, limits).unwrap();
        let first = match p.next_event().unwrap() {
            Some(Event::Start(t)) => t,
            other => panic!("{other:?}"),
        };
        assert_eq!(p.subtree(first).unwrap().count(), 1000);
    }

    #[test]
    fn mutated_inputs_never_panic_with_a_fixed_seed() {
        let base = "<IPC-2581 revision=\"C\"><Content a=\"1\"><L n=\"x\"/><L n=\"y\"/></Content><!-- c --><E k=\"&amp;\"><F/></E></IPC-2581>";
        let mut state: u64 = 0x5eed_2581;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as usize
        };
        for _ in 0..3000 {
            let mut bytes = base.as_bytes().to_vec();
            for _ in 0..(1 + next() % 4) {
                let at = next() % bytes.len();
                match next() % 3 {
                    0 => bytes[at] = b"<>/=\"'&! x"[next() % 10],
                    1 => {
                        bytes.remove(at);
                    }
                    _ => bytes.insert(at, b"<>/=\"'&!"[next() % 8]),
                }
            }
            let text = String::from_utf8_lossy(&bytes).into_owned();
            if let Ok(mut p) = Pull::new(&text, PullLimits::default()) {
                while let Ok(Some(_)) = p.next_event() {}
            }
        }
    }
}
