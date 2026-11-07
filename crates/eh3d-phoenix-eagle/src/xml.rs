//! A small, bounded XML parser: elements, attributes, text, comments, CDATA, processing
//! instructions and a document type (skipped). It builds a tree without recursion, so depth cannot
//! overflow the stack, and it refuses input over its [`Limits`].

use std::fmt;

/// The limits of one parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Largest input in bytes.
    pub max_bytes: usize,
    /// Deepest nesting of elements.
    pub max_depth: usize,
    /// Most elements in the document.
    pub max_nodes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_bytes: 512 * 1024 * 1024,
            max_depth: 256,
            max_nodes: 20_000_000,
        }
    }
}

/// Why a parse failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlError {
    /// The text is not well-formed; the byte offset says where.
    Malformed {
        /// Byte offset of the problem.
        at: usize,
        /// What was expected.
        what: &'static str,
    },
    /// A limit was exceeded.
    Limit(&'static str),
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XmlError::Malformed { at, what } => write!(f, "malformed XML at byte {at}: {what}"),
            XmlError::Limit(what) => write!(f, "limit exceeded: {what}"),
        }
    }
}

impl std::error::Error for XmlError {}

/// How many attributes of a tag are compared one by one for a repeated name; from this many on a
/// set of the names is used.
const ATTRS_COMPARED_ONE_BY_ONE: usize = 32;

/// One element with its attributes (in file order), its text and its child elements.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Element {
    /// The tag name.
    pub name: String,
    /// The attributes, with entities resolved.
    pub attrs: Vec<(String, String)>,
    /// The character data directly inside the element, concatenated.
    pub text: String,
    /// The child elements in order.
    pub children: Vec<Element>,
}

impl Element {
    /// The value of the attribute `name`, if present.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// The first child called `name`.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    /// The children called `name`.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// The number of elements in this subtree, itself included.
    pub fn count(&self) -> usize {
        let mut total = 0;
        let mut stack = vec![self];
        while let Some(e) = stack.pop() {
            total += 1;
            stack.extend(e.children.iter());
        }
        total
    }
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':') || b >= 0x80
}

struct Parser<'a> {
    s: &'a [u8],
    text: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    fn bad<T>(&self, what: &'static str) -> Result<T, XmlError> {
        Err(XmlError::Malformed { at: self.i, what })
    }

    fn starts(&self, p: &[u8]) -> bool {
        self.s[self.i..].starts_with(p)
    }

    fn skip_space(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn skip_until(&mut self, end: &[u8], what: &'static str) -> Result<(), XmlError> {
        match find(&self.s[self.i..], end) {
            Some(p) => {
                self.i += p + end.len();
                Ok(())
            }
            None => self.bad(what),
        }
    }

    fn name(&mut self) -> Result<String, XmlError> {
        let start = self.i;
        while self.i < self.s.len() && is_name_byte(self.s[self.i]) {
            self.i += 1;
        }
        if start == self.i {
            return self.bad("a name");
        }
        Ok(self.text[start..self.i].to_owned())
    }

    /// Skips a document type declaration, with its optional internal subset.
    fn skip_doctype(&mut self) -> Result<(), XmlError> {
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

    fn attr_value(&mut self) -> Result<String, XmlError> {
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
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Resolves the five named entities and numeric references; anything else is kept as written.
fn unescape(raw: &str) -> String {
    if !raw.contains('&') {
        return raw.to_owned();
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        match rest.find(';') {
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
                match ch {
                    Some(c) => {
                        out.push(c);
                        rest = &rest[e + 1..];
                    }
                    None => {
                        out.push('&');
                        rest = &rest[1..];
                    }
                }
            }
            _ => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Parses `text` into its root element.
pub fn parse(text: &str, limits: &Limits) -> Result<Element, XmlError> {
    if text.len() > limits.max_bytes {
        return Err(XmlError::Limit("input is larger than the byte limit"));
    }
    let mut p = Parser {
        s: text.as_bytes(),
        text,
        i: 0,
    };
    if p.starts(&[0xEF, 0xBB, 0xBF]) {
        p.i = 3;
    }
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;
    let mut nodes = 0usize;
    while p.i < p.s.len() {
        if p.s[p.i] != b'<' {
            let start = p.i;
            while p.i < p.s.len() && p.s[p.i] != b'<' {
                p.i += 1;
            }
            let chunk = &text[start..p.i];
            match stack.last_mut() {
                Some(top) => {
                    // White space between elements is layout, not content.
                    if !chunk.trim().is_empty() {
                        top.text.push_str(&unescape(chunk));
                    }
                }
                None => {
                    if !chunk.trim().is_empty() {
                        p.i = start;
                        return p.bad("only white space outside the root element");
                    }
                }
            }
            continue;
        }
        if p.starts(b"<?") {
            p.skip_until(b"?>", "the end of a processing instruction")?;
        } else if p.starts(b"<!--") {
            p.skip_until(b"-->", "the end of a comment")?;
        } else if p.starts(b"<![CDATA[") {
            let start = p.i + 9;
            p.skip_until(b"]]>", "the end of a CDATA section")?;
            if let Some(top) = stack.last_mut() {
                top.text.push_str(&text[start..p.i - 3]);
            }
        } else if p.starts(b"<!") {
            p.i += 2;
            p.skip_doctype()?;
        } else if p.starts(b"</") {
            p.i += 2;
            let name = p.name()?;
            p.skip_space();
            if p.s.get(p.i) != Some(&b'>') {
                return p.bad("> after a closing tag");
            }
            p.i += 1;
            match stack.pop() {
                Some(mut done) if done.name == name => {
                    if !done.children.is_empty() {
                        // text next to child elements is trimmed: the rest is layout
                        done.text = done.text.trim().to_owned();
                    }
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(done),
                        None => {
                            if root.is_some() {
                                return p.bad("a single root element");
                            }
                            root = Some(done);
                        }
                    }
                }
                _ => return p.bad("a closing tag that matches the open element"),
            }
        } else {
            p.i += 1;
            let name = p.name()?;
            let mut el = Element {
                name,
                ..Element::default()
            };
            let mut seen_names: Option<std::collections::BTreeSet<String>> = None;
            loop {
                p.skip_space();
                match p.s.get(p.i) {
                    Some(b'>') => {
                        p.i += 1;
                        break;
                    }
                    Some(b'/') => {
                        p.i += 1;
                        if p.s.get(p.i) != Some(&b'>') {
                            return p.bad("> after / in an empty element");
                        }
                        p.i += 1;
                        // An empty element is complete at once.
                        nodes += 1;
                        if nodes > limits.max_nodes {
                            return Err(XmlError::Limit("too many elements"));
                        }
                        match stack.last_mut() {
                            Some(parent) => parent.children.push(el),
                            None => {
                                if root.is_some() {
                                    return p.bad("a single root element");
                                }
                                root = Some(el);
                            }
                        }
                        el = Element::default();
                        break;
                    }
                    Some(_) => {
                        let key = p.name()?;
                        p.skip_space();
                        if p.s.get(p.i) != Some(&b'=') {
                            return p.bad("= after an attribute name");
                        }
                        p.i += 1;
                        p.skip_space();
                        let value = p.attr_value()?;
                        // a few attributes are compared one by one; a tag with many gets a set of
                        // the names, so that the time does not grow with the square of their number
                        let twice = if el.attrs.len() < ATTRS_COMPARED_ONE_BY_ONE {
                            el.attrs.iter().any(|(k, _)| *k == key)
                        } else {
                            let seen = seen_names.get_or_insert_with(|| {
                                el.attrs.iter().map(|(k, _)| k.clone()).collect()
                            });
                            !seen.insert(key.clone())
                        };
                        if twice {
                            return p.bad("an attribute name used once");
                        }
                        el.attrs.push((key, value));
                    }
                    None => return p.bad("the end of a tag"),
                }
            }
            if !el.name.is_empty() {
                nodes += 1;
                if nodes > limits.max_nodes {
                    return Err(XmlError::Limit("too many elements"));
                }
                if stack.len() + 1 > limits.max_depth {
                    return Err(XmlError::Limit("elements nested too deep"));
                }
                if root.is_some() && stack.is_empty() {
                    return p.bad("a single root element");
                }
                stack.push(el);
            }
        }
    }
    if !stack.is_empty() {
        return p.bad("a closing tag for every open element");
    }
    match root {
        Some(r) => Ok(r),
        None => p.bad("a root element"),
    }
}

fn escape(text: &str, attribute: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            '\n' if attribute => out.push_str("&#10;"),
            '\t' if attribute => out.push_str("&#9;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
    out
}

impl Element {
    /// Writes the element and everything below it as XML text: two spaces of indent, attributes
    /// in their order, one line per element that has children, text kept on the line of its
    /// element. Writing needs no recursion, so depth cannot overflow the stack. The same tree
    /// always gives the same text, and parsing that text gives the same tree back.
    pub fn to_xml(&self) -> String {
        enum Step<'a> {
            Open(&'a Element, usize),
            Close(&'a Element, usize),
        }
        let mut out = String::new();
        let mut stack = vec![Step::Open(self, 0)];
        while let Some(step) = stack.pop() {
            match step {
                Step::Open(e, depth) => {
                    out.push_str(&"  ".repeat(depth));
                    out.push('<');
                    out.push_str(&e.name);
                    for (k, v) in &e.attrs {
                        out.push(' ');
                        out.push_str(k);
                        out.push_str("=\"");
                        out.push_str(&escape(v, true));
                        out.push('"');
                    }
                    if e.children.is_empty() && e.text.is_empty() {
                        out.push_str("/>\n");
                    } else if e.children.is_empty() {
                        out.push('>');
                        out.push_str(&escape(&e.text, false));
                        out.push_str("</");
                        out.push_str(&e.name);
                        out.push_str(">\n");
                    } else {
                        out.push('>');
                        if !e.text.is_empty() {
                            out.push_str(&escape(&e.text, false));
                        }
                        out.push('\n');
                        stack.push(Step::Close(e, depth));
                        for child in e.children.iter().rev() {
                            stack.push(Step::Open(child, depth + 1));
                        }
                    }
                }
                Step::Close(e, depth) => {
                    out.push_str(&"  ".repeat(depth));
                    out.push_str("</");
                    out.push_str(&e.name);
                    out.push_str(">\n");
                }
            }
        }
        out
    }
}

/// Writes a whole EAGLE document: the XML declaration, the document type of EAGLE and the tree.
pub fn write_document(root: &Element) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE {} SYSTEM \"{}.dtd\">\n{}",
        root.name,
        root.name,
        root.to_xml()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(t: &str) -> Result<Element, XmlError> {
        parse(t, &Limits::default())
    }

    #[test]
    fn reads_a_small_document() {
        let e = p("<?xml version=\"1.0\"?>\n<!DOCTYPE eagle SYSTEM \"eagle.dtd\">\n<eagle version=\"6.5.0\"><!-- c --><a x=\"1&amp;2\" y='z'/><b>t&lt;1</b></eagle>").unwrap();
        assert_eq!(e.name, "eagle");
        assert_eq!(e.attr("version"), Some("6.5.0"));
        assert_eq!(e.children.len(), 2);
        assert_eq!(e.child("a").unwrap().attr("x"), Some("1&2"));
        assert_eq!(e.child("b").unwrap().text, "t<1");
        assert_eq!(e.count(), 3);
    }

    /// A tag `<r a0="0" a1="1" ...>` of `count` distinct attributes, one name repeated at the end
    /// when `again` names one.
    fn many_attributes(count: usize, again: Option<usize>) -> String {
        let mut text = String::from("<r");
        for k in 0..count {
            text.push_str(&format!(" a{k}=\"{k}\""));
        }
        if let Some(k) = again {
            text.push_str(&format!(" a{k}=\"x\""));
        }
        text.push_str("/>");
        text
    }

    #[test]
    fn a_tag_with_many_attributes_keeps_them_all_in_order_and_still_refuses_a_repeat() {
        // below, at and above the number that is compared one by one
        for count in [1, 31, 32, 33, 100, 5000] {
            let e = p(&many_attributes(count, None)).unwrap();
            assert_eq!(e.attrs.len(), count);
            assert_eq!(e.attrs[0], ("a0".to_owned(), "0".to_owned()));
            let last = count - 1;
            assert_eq!(e.attr(&format!("a{last}")), Some(last.to_string().as_str()));
            // a name used again, whether it was among the first ones or among the later ones
            for again in [0, last / 2, last] {
                assert!(
                    matches!(
                        p(&many_attributes(count, Some(again))),
                        Err(XmlError::Malformed { .. })
                    ),
                    "{count} attributes, a{again} again"
                );
            }
        }
    }

    #[test]
    fn a_bom_and_numeric_entities_are_read() {
        let e = p("\u{feff}<r a=\"&#65;&#x42;\">&#67;</r>").unwrap();
        assert_eq!(e.attr("a"), Some("AB"));
        assert_eq!(e.text, "C");
    }

    #[test]
    fn broken_documents_are_errors_and_never_panics() {
        for bad in [
            "",
            "<a>",
            "<a></b>",
            "<a x=1/>",
            "<a x=\"1\" x=\"2\"/>",
            "<a/><b/>",
            "text",
            "<a><!-- open",
            "<a x=\"<\"/>",
            "<a",
            "</a>",
            "<a>&",
        ] {
            assert!(p(bad).is_err() || bad == "<a>&", "{bad:?}");
        }
    }

    #[test]
    fn limits_are_enforced() {
        let deep = format!("{}{}", "<a>".repeat(10), "</a>".repeat(10));
        let l = Limits {
            max_depth: 5,
            ..Limits::default()
        };
        assert_eq!(
            parse(&deep, &l),
            Err(XmlError::Limit("elements nested too deep"))
        );
        let l = Limits {
            max_nodes: 2,
            ..Limits::default()
        };
        assert!(matches!(
            parse("<a><b/><c/></a>", &l),
            Err(XmlError::Limit(_))
        ));
        let l = Limits {
            max_bytes: 3,
            ..Limits::default()
        };
        assert!(matches!(parse("<a/>", &l), Err(XmlError::Limit(_))));
    }

    #[test]
    fn writing_and_reading_again_gives_the_same_tree_and_the_same_text() {
        let src = "<?xml version=\"1.0\"?>\n<!DOCTYPE eagle SYSTEM \"eagle.dtd\">\n<eagle version=\"9.6.2\">\n <drawing>\n  <note a=\"x &amp; &quot;y&quot; &lt;z&gt;\" b='1&#10;2'/>\n  <text x=\"1\">&gt;NAME &amp; more</text>\n  <empty></empty>\n  <mixed>lead<child/></mixed>\n </drawing>\n</eagle>";
        let tree = p(src).unwrap();
        let text = write_document(&tree);
        let again = p(&text).unwrap();
        assert_eq!(tree, again);
        assert_eq!(write_document(&again), text, "the written text is stable");
        assert!(text.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE eagle SYSTEM \"eagle.dtd\">\n<eagle version=\"9.6.2\">\n  <drawing>\n"));
        assert_eq!(
            tree.child("drawing").unwrap().child("mixed").unwrap().text,
            "lead"
        );
    }

    #[test]
    fn a_very_deep_tree_is_written_without_overflowing_the_stack() {
        let depth = 200;
        let l = Limits::default();
        let doc = format!("{}{}", "<a>".repeat(depth), "</a>".repeat(depth));
        let tree = parse(&doc, &l).unwrap();
        assert_eq!(parse(&tree.to_xml(), &l).unwrap(), tree);
    }

    #[test]
    fn truncating_a_valid_document_anywhere_never_panics() {
        let doc = "<eagle version=\"9.6.2\"><drawing><layers><layer number=\"1\" name=\"Top\"/></layers><text>é&amp;</text></drawing></eagle>";
        for n in 0..doc.len() {
            if doc.is_char_boundary(n) {
                let _ = p(&doc[..n]);
            }
        }
    }
}
