// SPDX-License-Identifier: GPL-3.0-or-later
//! The XML reader and writer of the EAGLE files, checked on small documents with the tree, the
//! byte offset of each refusal and the text that is written worked out by hand.

use eh3d_phoenix_eagle::xml::parse;
use eh3d_phoenix_eagle::{write_document, Element, Limits, XmlError};

fn read(text: &str) -> Result<Element, XmlError> {
    parse(text, &Limits::default())
}

fn malformed(at: usize, what: &'static str) -> XmlError {
    XmlError::Malformed { at, what }
}

#[test]
fn elements_attributes_and_children_are_read_in_order() {
    let root = read(r#"<a x="1" y='two'><b>hi</b><b/><c z="&lt;&amp;&#65;&#x42;"/></a>"#).unwrap();
    assert_eq!(root.name, "a");
    assert_eq!(
        root.attrs,
        vec![
            ("x".to_string(), "1".to_string()),
            ("y".to_string(), "two".to_string())
        ]
    );
    assert_eq!(root.attr("x"), Some("1"));
    assert_eq!(root.attr("y"), Some("two"));
    assert_eq!(root.attr("missing"), None);
    assert_eq!(root.children.len(), 3);
    assert_eq!(root.children_named("b").count(), 2);
    assert_eq!(root.children_named("z").count(), 0);
    assert_eq!(root.child("b").unwrap().text, "hi");
    // the second b is empty
    assert_eq!(root.children[1].text, "");
    // the entities of the five names and the numbers, in decimal and in hexadecimal
    assert_eq!(root.child("c").unwrap().attr("z"), Some("<&AB"));
    // a, two b and c
    assert_eq!(root.count(), 4);
}

#[test]
fn white_space_between_elements_is_layout_and_text_next_to_children_is_trimmed() {
    let spaced = read("<a> <b/> </a>").unwrap();
    assert_eq!(spaced.text, "");
    assert_eq!(spaced.children.len(), 1);
    let mixed = read("<a>\n  <b>x</b>\n  text\n</a>").unwrap();
    assert_eq!(mixed.text, "text");
    // text of a leaf is kept as it is, with its spaces inside
    let leaf = read("<a>  two  words </a>").unwrap();
    assert_eq!(leaf.text, "  two  words ");
}

#[test]
fn text_has_its_entities_resolved() {
    let root = read("<a>1 &lt; 2 &amp;&amp; 3 &gt; 2</a>").unwrap();
    assert_eq!(root.text, "1 < 2 && 3 > 2");
}

#[test]
fn a_bom_a_declaration_a_doctype_a_comment_and_cdata_are_handled() {
    let doc = "\u{feff}<?xml version=\"1.0\"?>\n<!DOCTYPE eagle SYSTEM \"eagle.dtd\">\n\
        <!-- a < comment -->\n<eagle><note><![CDATA[a < b & c]]></note></eagle>\n";
    let root = read(doc).unwrap();
    assert_eq!(root.name, "eagle");
    assert_eq!(root.child("note").unwrap().text, "a < b & c");
}

#[test]
fn a_doctype_with_an_internal_subset_is_skipped() {
    let root = read("<!DOCTYPE a [ <!ENTITY x \"y>\"> ]><a/>").unwrap();
    assert_eq!(root.name, "a");
    assert!(root.children.is_empty());
}

#[test]
fn what_is_not_an_entity_is_kept_as_it_was_written() {
    let root = read(r#"<a p="a&bogus;b" q="x&y" r="&#xZZ;" s="&abcdefghijklmnop;" t="&#9731;"/>"#)
        .unwrap();
    assert_eq!(root.attr("p"), Some("a&bogus;b"));
    assert_eq!(root.attr("q"), Some("x&y"));
    assert_eq!(root.attr("r"), Some("&#xZZ;"));
    assert_eq!(root.attr("s"), Some("&abcdefghijklmnop;"));
    // 9731 is U+2603
    assert_eq!(root.attr("t"), Some("\u{2603}"));
}

#[test]
fn names_may_have_a_namespace_dots_dashes_and_letters_beyond_ascii() {
    let root = read(r#"<ns:tag-1.x_y a:b="1"><é/></ns:tag-1.x_y>"#).unwrap();
    assert_eq!(root.name, "ns:tag-1.x_y");
    assert_eq!(root.attr("a:b"), Some("1"));
    assert_eq!(root.children[0].name, "é");
    // a space before the end of an empty element
    assert_eq!(read("<a />").unwrap().name, "a");
}

#[test]
fn a_refusal_names_the_byte_where_it_stopped() {
    // </a> closes b: the reader is after the > of the closing tag, at byte 10
    assert_eq!(
        read("<a><b></a>"),
        Err(malformed(10, "a closing tag that matches the open element"))
    );
    // an element that is never closed: the reader is at the end, byte 7
    assert_eq!(
        read("<a><b/>"),
        Err(malformed(7, "a closing tag for every open element"))
    );
    assert_eq!(read(""), Err(malformed(0, "a root element")));
    assert_eq!(read("<a/><b/>"), Err(malformed(8, "a single root element")));
    assert_eq!(
        read("x<a/>"),
        Err(malformed(0, "only white space outside the root element"))
    );
    assert_eq!(read("<>"), Err(malformed(1, "a name")));
    assert_eq!(read("<a></a"), Err(malformed(6, "> after a closing tag")));
}

#[test]
fn a_bad_attribute_is_refused_where_it_is() {
    assert_eq!(
        read(r#"<a x="1" x="2"/>"#),
        Err(malformed(14, "an attribute name used once"))
    );
    assert_eq!(
        read("<a x=1/>"),
        Err(malformed(5, "a quoted attribute value"))
    );
    assert_eq!(
        read(r#"<a x="<"/>"#),
        Err(malformed(6, "no < inside an attribute value"))
    );
    assert_eq!(
        read("<a x/>"),
        Err(malformed(4, "= after an attribute name"))
    );
}

#[test]
fn the_limits_are_checked_in_bytes_in_depth_and_in_elements() {
    let small = Limits {
        max_bytes: 5,
        ..Limits::default()
    };
    assert_eq!(
        parse("<a></a>", &small),
        Err(XmlError::Limit("input is larger than the byte limit"))
    );
    assert!(parse("<a/>", &small).is_ok());
    let shallow = Limits {
        max_depth: 2,
        ..Limits::default()
    };
    assert!(parse("<a><b></b></a>", &shallow).is_ok());
    assert_eq!(
        parse("<a><b><c></c></b></a>", &shallow),
        Err(XmlError::Limit("elements nested too deep"))
    );
    let few = Limits {
        max_nodes: 2,
        ..Limits::default()
    };
    assert_eq!(
        parse("<a><b/><c/></a>", &few),
        Err(XmlError::Limit("too many elements"))
    );
    let three = Limits {
        max_nodes: 3,
        ..Limits::default()
    };
    assert!(parse("<a><b/><c/></a>", &three).is_ok());
}

#[test]
fn the_text_written_has_two_spaces_of_indent_and_escapes_its_markup() {
    let root = read(r#"<a x="1&amp;2"><b>t&lt;u</b><c/></a>"#).unwrap();
    assert_eq!(
        root.to_xml(),
        "<a x=\"1&amp;2\">\n  <b>t&lt;u</b>\n  <c/>\n</a>\n"
    );
    assert_eq!(
        write_document(&root),
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE a SYSTEM \"a.dtd\">\n\
         <a x=\"1&amp;2\">\n  <b>t&lt;u</b>\n  <c/>\n</a>\n"
    );
}

#[test]
fn the_characters_that_an_attribute_cannot_hold_are_written_as_references_and_read_back() {
    let tricky = Element {
        name: "a".to_string(),
        attrs: vec![("k".to_string(), "line1\nline2\t\"q\" <&>\r".to_string())],
        text: String::new(),
        children: Vec::new(),
    };
    let text = tricky.to_xml();
    assert_eq!(
        text,
        "<a k=\"line1&#10;line2&#9;&quot;q&quot; &lt;&amp;&gt;&#13;\"/>\n"
    );
    assert_eq!(read(&text).unwrap(), tricky);
}

#[test]
fn a_leaf_with_text_is_written_on_one_line_and_a_tree_comes_back_the_same() {
    let tree = Element {
        name: "root".to_string(),
        attrs: vec![("n".to_string(), "1".to_string())],
        text: String::new(),
        children: vec![
            Element {
                name: "leaf".to_string(),
                attrs: Vec::new(),
                text: "a & b\r".to_string(),
                children: Vec::new(),
            },
            Element {
                name: "empty".to_string(),
                attrs: Vec::new(),
                text: String::new(),
                children: Vec::new(),
            },
        ],
    };
    let text = tree.to_xml();
    assert_eq!(
        text,
        "<root n=\"1\">\n  <leaf>a &amp; b&#13;</leaf>\n  <empty/>\n</root>\n"
    );
    assert_eq!(read(&text).unwrap(), tree);
    assert_eq!(read(&write_document(&tree)).unwrap(), tree);
}
