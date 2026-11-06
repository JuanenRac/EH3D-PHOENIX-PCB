// SPDX-License-Identifier: GPL-3.0-or-later
//! The s-expressions of the KiCad files that the writers make.
//!
//! The text is deterministic: the same tree always gives the same bytes. Numbers are written from
//! integers (nanometres and micro-degrees) with the fewest decimals that are exact, text is
//! escaped, the indentation is one tab for each level, and the identifiers that KiCad wants on
//! almost every item are derived from the ids of the model and a role, never from a clock or from
//! chance.

use eh3d_phoenix_model::ObjectId;

/// One node of a tree of s-expressions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sx {
    /// A word written as it is (`smd`, `1.27`, `F.Cu`).
    Atom(String),
    /// A text written between quotes, with the escapes KiCad reads.
    Str(String),
    /// A list.
    List(Vec<Sx>),
}

/// A word.
pub fn atom(text: &str) -> Sx {
    Sx::Atom(text.to_owned())
}

/// A quoted text.
pub fn string(text: &str) -> Sx {
    Sx::Str(text.to_owned())
}

/// A list that starts with the word `head`.
pub fn list(head: &str, items: Vec<Sx>) -> Sx {
    let mut all = Vec::with_capacity(items.len() + 1);
    all.push(atom(head));
    all.extend(items);
    Sx::List(all)
}

/// A number from a value in millionths (nanometres written as millimetres, micro-degrees written
/// as degrees).
pub fn num(value: i64) -> Sx {
    Sx::Atom(fixed6(value))
}

/// A whole number.
pub fn int(value: i64) -> Sx {
    Sx::Atom(value.to_string())
}

/// `(head x y)` with two lengths.
pub fn pair(head: &str, x: i64, y: i64) -> Sx {
    list(head, vec![num(x), num(y)])
}

/// `(at x y)` or, when the angle is not zero, `(at x y angle)`.
pub fn at(x: i64, y: i64, angle_udeg: i64) -> Sx {
    if angle_udeg == 0 {
        pair("at", x, y)
    } else {
        list("at", vec![num(x), num(y), num(angle_udeg)])
    }
}

/// `(at x y angle)` with the angle always written (the form that KiCad writes for symbols and
/// pins).
pub fn at3(x: i64, y: i64, angle_udeg: i64) -> Sx {
    list("at", vec![num(x), num(y), num(angle_udeg)])
}

/// A value in millionths as a decimal text with the fewest decimals that are exact.
pub fn fixed6(value: i64) -> String {
    let abs = value.unsigned_abs();
    let whole = abs / 1_000_000;
    let fraction = abs % 1_000_000;
    let mut text = String::new();
    if value < 0 {
        text.push('-');
    }
    text.push_str(&whole.to_string());
    if fraction != 0 {
        let digits = format!("{fraction:06}");
        text.push('.');
        text.push_str(digits.trim_end_matches('0'));
    }
    text
}

/// A text between quotes with `\`, `"` and the line breaks escaped. Any other control character
/// becomes a space.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if other.is_control() => out.push(' '),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push('\t');
    }
}

fn put(out: &mut String, node: &Sx, depth: usize) {
    match node {
        Sx::Atom(text) => out.push_str(text),
        Sx::Str(text) => out.push_str(&quote(text)),
        Sx::List(items) => {
            out.push('(');
            let mut inline = true;
            for (index, item) in items.iter().enumerate() {
                if matches!(item, Sx::List(_)) {
                    inline = false;
                }
                if inline {
                    if index > 0 {
                        out.push(' ');
                    }
                    put(out, item, depth);
                } else {
                    out.push('\n');
                    indent(out, depth + 1);
                    put(out, item, depth + 1);
                }
            }
            if !inline {
                out.push('\n');
                indent(out, depth);
            }
            out.push(')');
        }
    }
}

/// The text of a tree, ending with a line break.
pub fn render(root: &Sx) -> String {
    let mut out = String::new();
    put(&mut out, root, 0);
    out.push('\n');
    out
}

fn mix(seed: u64, role: &str, bytes: &[u8]) -> u64 {
    let mut h = seed ^ 0xcbf2_9ce4_8422_2325;
    for b in role
        .bytes()
        .chain(std::iter::once(0u8))
        .chain(bytes.iter().copied())
    {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ (h >> 31)
}

/// A UUID (version 4 layout) that depends only on `role` and `bytes`.
pub fn uuid_from(role: &str, bytes: &[u8]) -> String {
    let a = mix(0x9e37_79b9_7f4a_7c15, role, bytes);
    let b = mix(0xd1b5_4a32_d192_ed03, role, bytes);
    let mut raw = [0u8; 16];
    raw[..8].copy_from_slice(&a.to_be_bytes());
    raw[8..].copy_from_slice(&b.to_be_bytes());
    raw[6] = (raw[6] & 0x0f) | 0x40;
    raw[8] = (raw[8] & 0x3f) | 0x80;
    let hex: Vec<String> = raw.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        hex[0..4].concat(),
        hex[4..6].concat(),
        hex[6..8].concat(),
        hex[8..10].concat(),
        hex[10..16].concat()
    )
}

/// The UUID of an item of the model in a role (the same id in two roles gives two UUIDs).
pub fn uuid_for(id: &ObjectId, role: &str) -> String {
    uuid_from(role, id.as_bytes())
}

/// `(uuid "...")` for an item of the model in a role.
pub fn uuid_node(id: &ObjectId, role: &str) -> Sx {
    list("uuid", vec![string(&uuid_for(id, role))])
}

/// `(uuid "...")` for something that has no id in the model, from a text.
pub fn uuid_node_text(role: &str, text: &str) -> Sx {
    list("uuid", vec![string(&uuid_from(role, text.as_bytes()))])
}

/// `(effects (font (size h h) (thickness t)))`, with `hide` and a justification when asked.
pub fn effects(size: i64, thickness: i64, hide: bool, justify: &[&str]) -> Sx {
    let mut items = vec![list(
        "font",
        vec![
            pair("size", size, size),
            list("thickness", vec![num(thickness)]),
        ],
    )];
    if !justify.is_empty() {
        items.push(list("justify", justify.iter().map(|w| atom(w)).collect()));
    }
    if hide {
        items.push(list("hide", vec![atom("yes")]));
    }
    list("effects", items)
}

/// Tools of the tests: a reader of the text the writers make, built on its own so that the tests
/// do not depend on the readers of the crate.
#[cfg(test)]
pub(crate) mod testkit {
    use super::Sx;

    fn push(stack: &mut [Vec<Sx>], node: Sx) -> Result<(), String> {
        match stack.last_mut() {
            Some(top) => {
                top.push(node);
                Ok(())
            }
            None => Err("an item outside any list".to_owned()),
        }
    }

    /// Parses one tree; an error says what is wrong with the text.
    pub(crate) fn parse(text: &str) -> Result<Sx, String> {
        let chars: Vec<char> = text.chars().collect();
        let mut pos = 0usize;
        let mut stack: Vec<Vec<Sx>> = Vec::new();
        let mut root: Option<Sx> = None;
        while pos < chars.len() {
            let c = chars[pos];
            if c.is_whitespace() {
                pos += 1;
            } else if c == '(' {
                stack.push(Vec::new());
                pos += 1;
            } else if c == ')' {
                let items = stack.pop().ok_or("a closing parenthesis too many")?;
                let node = Sx::List(items);
                if stack.is_empty() {
                    if root.is_some() {
                        return Err("two trees".to_owned());
                    }
                    root = Some(node);
                } else {
                    push(&mut stack, node)?;
                }
                pos += 1;
            } else if c == '"' {
                pos += 1;
                let mut s = String::new();
                loop {
                    let d = *chars.get(pos).ok_or("a text that is not closed")?;
                    pos += 1;
                    if d == '"' {
                        break;
                    }
                    if d == '\\' {
                        let e = *chars.get(pos).ok_or("an escape that is not finished")?;
                        pos += 1;
                        s.push(match e {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            other => other,
                        });
                    } else {
                        s.push(d);
                    }
                }
                push(&mut stack, Sx::Str(s))?;
            } else {
                let start = pos;
                while pos < chars.len()
                    && !chars[pos].is_whitespace()
                    && chars[pos] != '('
                    && chars[pos] != ')'
                    && chars[pos] != '"'
                {
                    pos += 1;
                }
                let word: String = chars[start..pos].iter().collect();
                push(&mut stack, Sx::Atom(word))?;
            }
        }
        if !stack.is_empty() {
            return Err("a list that is not closed".to_owned());
        }
        root.ok_or_else(|| "no tree".to_owned())
    }

    /// The text of a word or a quoted text.
    pub(crate) fn text(node: &Sx) -> Option<&str> {
        match node {
            Sx::Atom(t) | Sx::Str(t) => Some(t.as_str()),
            Sx::List(_) => None,
        }
    }

    /// The items of a list, the head included.
    pub(crate) fn items(node: &Sx) -> &[Sx] {
        match node {
            Sx::List(items) => items.as_slice(),
            _ => &[],
        }
    }

    /// The head word of a list.
    pub(crate) fn head(node: &Sx) -> Option<&str> {
        items(node).first().and_then(text)
    }

    /// The text of the `index`th item after the head.
    pub(crate) fn arg(node: &Sx, index: usize) -> Option<&str> {
        items(node).get(index + 1).and_then(text)
    }

    /// Every list item whose head is `name`.
    pub(crate) fn children<'a>(node: &'a Sx, name: &str) -> Vec<&'a Sx> {
        items(node)
            .iter()
            .filter(|i| head(i) == Some(name))
            .collect()
    }

    /// The first list item whose head is `name`.
    pub(crate) fn child<'a>(node: &'a Sx, name: &str) -> Option<&'a Sx> {
        children(node, name).into_iter().next()
    }
}

#[cfg(test)]
mod tests {
    use super::testkit::{arg, child, children, head, parse};
    use super::*;

    #[test]
    fn numbers_have_the_fewest_exact_decimals() {
        assert_eq!(fixed6(0), "0");
        assert_eq!(fixed6(1_000_000), "1");
        assert_eq!(fixed6(1_270_000), "1.27");
        assert_eq!(fixed6(-1_270_000), "-1.27");
        assert_eq!(fixed6(-500_000), "-0.5");
        assert_eq!(fixed6(1), "0.000001");
        assert_eq!(fixed6(-1), "-0.000001");
        assert_eq!(fixed6(123_456_789), "123.456789");
        assert_eq!(fixed6(10_000_000), "10");
        assert_eq!(fixed6(i64::MIN + 1), "-9223372036854.775807");
    }

    #[test]
    fn text_is_escaped_and_reads_back() {
        let raw = "a \"b\" \\ c\nd\te";
        let quoted = quote(raw);
        assert_eq!(quoted, "\"a \\\"b\\\" \\\\ c\\nd\\te\"");
        let tree = parse(&format!("(x {quoted})")).unwrap();
        assert_eq!(arg(&tree, 0), Some(raw));
        assert_eq!(quote("a\u{1}b"), "\"a b\"");
    }

    #[test]
    fn a_tree_renders_with_tabs_and_reads_back_equal() {
        let tree = list(
            "root",
            vec![
                atom("flag"),
                string("name"),
                list("at", vec![num(1_500_000), num(-2_000_000)]),
                list("nested", vec![list("deep", vec![int(3)])]),
            ],
        );
        let text = render(&tree);
        assert_eq!(
            text,
            "(root flag \"name\"\n\t(at 1.5 -2)\n\t(nested\n\t\t(deep 3)\n\t)\n)\n"
        );
        assert_eq!(parse(&text).unwrap(), tree);
        assert_eq!(render(&tree), text);
    }

    #[test]
    fn ids_give_the_same_uuids_and_roles_give_different_ones() {
        let id = ObjectId::from_parts(1_700_000_000_000, 1, 7).unwrap();
        let other = ObjectId::from_parts(1_700_000_000_000, 2, 8).unwrap();
        let a = uuid_for(&id, "pad");
        assert_eq!(a, uuid_for(&id, "pad"));
        assert_ne!(a, uuid_for(&id, "via"));
        assert_ne!(a, uuid_for(&other, "pad"));
        assert_eq!(a.len(), 36);
        let parts: Vec<&str> = a.split('-').collect();
        let lengths: Vec<usize> = parts.iter().map(|p| p.len()).collect();
        assert_eq!(lengths, vec![8, 4, 4, 4, 12]);
        assert!(parts[2].starts_with('4'));
        assert!(matches!(&parts[3][..1], "8" | "9" | "a" | "b"));
        assert!(a.chars().all(|c| c == '-' || c.is_ascii_hexdigit()));
    }

    #[test]
    fn effects_and_positions_have_their_shape() {
        let e = effects(1_270_000, 150_000, true, &["left", "bottom"]);
        let tree = parse(&render(&e)).unwrap();
        assert_eq!(head(&tree), Some("effects"));
        assert!(child(&tree, "hide").is_some());
        assert_eq!(children(&tree, "justify").len(), 1);
        assert_eq!(render(&at(1_000_000, 2_000_000, 0)), "(at 1 2)\n");
        assert_eq!(
            render(&at(1_000_000, 2_000_000, 90_000_000)),
            "(at 1 2 90)\n"
        );
    }

    #[test]
    fn damaged_text_is_an_error_in_the_test_reader() {
        for bad in ["(a", "a)", "(a))", "\"x", "", "(a) (b)"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }
}
