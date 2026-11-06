// SPDX-License-Identifier: GPL-3.0-or-later
//! The attributes of parts and the assembly variants of an EAGLE schematic or board.
//!
//! A schematic or board may define variants (`<variantdefs>`), and each of its parts (`<part>` in a
//! schematic, `<element>` in a board) may carry attributes (`<attribute>`) and, for each variant,
//! whether it is fitted and with which value (`<variant name= populate= value= technology=>`).
//! This module reads them into records that keep every attribute of every XML element as it is, so
//! that writing them back gives the same elements, and resolves a variant into a view that says
//! for each part whether it is fitted and with which value. The bill of materials and the
//! placement file use the view.
//!
//! The semantics follow the format as documented: a part is fitted in a variant unless its
//! `<variant>` of that name says `populate="no"`, and a `value` of that `<variant>` replaces the
//! value of the part. They have not been checked against the reference tool.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::Board;
use eh3d_phoenix_eagle::xml::Element;

use crate::codes::{DeepError, Finding, Severity, VARIANT_INCONSISTENT, VARIANT_UNKNOWN};

/// An XML element reduced to its attributes, in the order of the file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Record {
    /// The attributes as key and value.
    pub attrs: Vec<(String, String)>,
}

impl Record {
    /// The attributes of an element.
    pub fn from_element(e: &Element) -> Record {
        Record {
            attrs: e.attrs.clone(),
        }
    }

    /// The value of an attribute.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The element called `name` with these attributes.
    pub fn to_element(&self, name: &str) -> Element {
        Element {
            name: name.to_owned(),
            attrs: self.attrs.clone(),
            ..Element::default()
        }
    }
}

/// A part of a schematic or an element of a board with its attributes and variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartRecord {
    /// Every attribute of the `<part>` or `<element>` as it is.
    pub base: Record,
    /// The `<attribute>` elements in order.
    pub attributes: Vec<Record>,
    /// The `<variant>` elements in order.
    pub variants: Vec<Record>,
}

impl PartRecord {
    fn from_element(e: &Element) -> PartRecord {
        PartRecord {
            base: Record::from_element(e),
            attributes: e
                .children_named("attribute")
                .map(Record::from_element)
                .collect(),
            variants: e
                .children_named("variant")
                .map(Record::from_element)
                .collect(),
        }
    }

    /// The reference designator.
    pub fn name(&self) -> &str {
        self.base.get("name").unwrap_or("")
    }

    /// The value of the part.
    pub fn value(&self) -> &str {
        self.base.get("value").unwrap_or("")
    }

    /// The value of the attribute called `name` (the attribute `value` of its element).
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.get("name") == Some(name))
            .and_then(|a| a.get("value"))
    }

    fn write_into(&self, e: &mut Element) {
        e.attrs = self.base.attrs.clone();
        e.children
            .retain(|c| c.name != "attribute" && c.name != "variant");
        e.children
            .extend(self.attributes.iter().map(|a| a.to_element("attribute")));
        e.children
            .extend(self.variants.iter().map(|v| v.to_element("variant")));
    }
}

/// The variants and the part attributes of a file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VariantSet {
    /// The `<variantdef>` elements in order (`name`, and `current="yes"` for the one in use).
    pub defs: Vec<Record>,
    /// The `<attribute>` elements that belong to the whole file.
    pub global_attributes: Vec<Record>,
    /// The parts of a schematic or the elements of a board.
    pub parts: Vec<PartRecord>,
}

/// The schematic or the board of a parsed file, and whether it is the board.
pub(crate) fn container(root: &Element) -> Option<(&Element, bool)> {
    let drawing = root.child("drawing")?;
    if let Some(board) = drawing.child("board") {
        return Some((board, true));
    }
    drawing.child("schematic").map(|s| (s, false))
}

fn container_mut(root: &mut Element) -> Option<(&mut Element, bool)> {
    let drawing = root.children.iter_mut().find(|c| c.name == "drawing")?;
    let is_board = drawing.children.iter().any(|c| c.name == "board");
    let wanted = if is_board { "board" } else { "schematic" };
    let found = drawing.children.iter_mut().find(|c| c.name == wanted)?;
    Some((found, is_board))
}

/// Reads the variants and the attributes of a parsed file. A library has none.
pub fn read_variants(root: &Element) -> VariantSet {
    let Some((file, is_board)) = container(root) else {
        return VariantSet::default();
    };
    let defs: Vec<Record> = file
        .child("variantdefs")
        .map(|d| {
            d.children_named("variantdef")
                .map(Record::from_element)
                .collect()
        })
        .unwrap_or_default();
    let global_attributes: Vec<Record> = file
        .child("attributes")
        .map(|d| {
            d.children_named("attribute")
                .map(Record::from_element)
                .collect()
        })
        .unwrap_or_default();
    let (list, item) = if is_board {
        ("elements", "element")
    } else {
        ("parts", "part")
    };
    let parts: Vec<PartRecord> = file
        .child(list)
        .map(|l| {
            l.children_named(item)
                .map(PartRecord::from_element)
                .collect()
        })
        .unwrap_or_default();
    VariantSet {
        defs,
        global_attributes,
        parts,
    }
}

/// One part in a variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewPart {
    /// True when the part is fitted.
    pub populated: bool,
    /// The value in this variant.
    pub value: String,
}

/// The parts of a file as one variant sees them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantView {
    /// The variant; empty for the base design.
    pub variant: String,
    /// The parts by reference designator.
    pub parts: BTreeMap<String, ViewPart>,
}

impl VariantView {
    /// The reference designators that are not fitted, in order.
    pub fn not_fitted(&self) -> Vec<String> {
        self.parts
            .iter()
            .filter(|(_, p)| !p.populated)
            .map(|(n, _)| n.clone())
            .collect()
    }
}

impl VariantSet {
    /// The names of the variants.
    pub fn names(&self) -> Vec<String> {
        self.defs
            .iter()
            .filter_map(|d| d.get("name"))
            .map(str::to_owned)
            .collect()
    }

    /// The variant marked as the current one, when there is one.
    pub fn current(&self) -> Option<&str> {
        self.defs
            .iter()
            .find(|d| d.get("current") == Some("yes"))
            .and_then(|d| d.get("name"))
    }

    /// The parts as the variant `variant` sees them; an empty name is the base design. A variant
    /// that the file does not define is refused with the names that it does.
    pub fn view(&self, variant: &str) -> Result<VariantView, DeepError> {
        if !variant.is_empty() && !self.names().iter().any(|n| n == variant) {
            return Err(DeepError::new(
                VARIANT_UNKNOWN,
                format!(
                    "{variant:?}; the file defines: {}",
                    if self.defs.is_empty() {
                        "none".to_owned()
                    } else {
                        self.names().join(", ")
                    }
                ),
            ));
        }
        let mut parts = BTreeMap::new();
        for part in &self.parts {
            let record = (!variant.is_empty())
                .then(|| {
                    part.variants
                        .iter()
                        .find(|v| v.get("name") == Some(variant))
                })
                .flatten();
            let populated = record.is_none_or(|r| r.get("populate") != Some("no"));
            let value = record
                .and_then(|r| r.get("value"))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| part.value());
            parts
                .entry(part.name().to_owned())
                .or_insert_with(|| ViewPart {
                    populated,
                    value: value.to_owned(),
                });
        }
        Ok(VariantView {
            variant: variant.to_owned(),
            parts,
        })
    }

    /// What is inconsistent in the data: a variant defined twice, a part with the same name twice, a `<variant>` of a name
    /// that no `<variantdef>` defines, two `<variant>` of the same name in one part.
    pub fn check(&self) -> Vec<Finding> {
        let mut findings = Vec::new();
        let names = self.names();
        let mut defined: BTreeSet<&str> = BTreeSet::new();
        for name in &names {
            if !defined.insert(name.as_str()) {
                findings.push(Finding::new(
                    VARIANT_INCONSISTENT,
                    Severity::Warning,
                    Some(name),
                    "the variant is defined more than once",
                ));
            }
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for part in &self.parts {
            if !seen.insert(part.name()) {
                findings.push(Finding::new(
                    VARIANT_INCONSISTENT,
                    Severity::Warning,
                    Some(part.name()),
                    "the name is used by more than one part; the first is used",
                ));
            }
            let mut in_part: BTreeSet<&str> = BTreeSet::new();
            for variant in &part.variants {
                let name = variant.get("name").unwrap_or("");
                if !names.iter().any(|n| n == name) {
                    findings.push(Finding::new(
                        VARIANT_INCONSISTENT,
                        Severity::Warning,
                        Some(part.name()),
                        format!("the variant {name:?} is not defined"),
                    ));
                }
                if !in_part.insert(name) {
                    findings.push(Finding::new(
                        VARIANT_INCONSISTENT,
                        Severity::Warning,
                        Some(part.name()),
                        format!("the variant {name:?} is given twice; the first is used"),
                    ));
                }
            }
        }
        findings
    }
}

fn set_attr(e: &mut Element, key: &str, value: &str) {
    match e.attrs.iter_mut().find(|(k, _)| k == key) {
        Some((_, v)) => *v = value.to_owned(),
        None => e.attrs.push((key.to_owned(), value.to_owned())),
    }
}

/// Writes the variants and attributes of `set` into the tree of a file. Elements that already say
/// the same are not touched, so reading a file and writing what was read changes nothing. A part
/// that the set names and the file does not have is skipped and named in a finding.
pub fn write_variants(root: &mut Element, set: &VariantSet) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some((file, is_board)) = container_mut(root) else {
        findings.push(Finding::new(
            VARIANT_INCONSISTENT,
            Severity::Warning,
            None,
            "the file has no board and no schematic",
        ));
        return findings;
    };
    write_group(file, "variantdefs", "variantdef", &set.defs);
    write_group(file, "attributes", "attribute", &set.global_attributes);
    let (list, item) = if is_board {
        ("elements", "element")
    } else {
        ("parts", "part")
    };
    if let Some(parent) = file.children.iter_mut().find(|c| c.name == list) {
        for (index, record) in set.parts.iter().enumerate() {
            // the n-th part of a name is matched with the n-th element of that name
            let nth = set.parts[..index]
                .iter()
                .filter(|p| p.name() == record.name())
                .count();
            let target = parent
                .children
                .iter_mut()
                .filter(|c| c.name == item && c.attr("name") == Some(record.name()))
                .nth(nth);
            match target {
                Some(e) => {
                    if PartRecord::from_element(e) != *record {
                        record.write_into(e);
                    }
                }
                None => findings.push(Finding::new(
                    VARIANT_INCONSISTENT,
                    Severity::Warning,
                    Some(record.name()),
                    "the part is not in the file and is skipped",
                )),
            }
        }
    } else if !set.parts.is_empty() {
        findings.push(Finding::new(
            VARIANT_INCONSISTENT,
            Severity::Warning,
            None,
            "the file has no list of parts; the parts of the set are skipped",
        ));
    }
    findings
}

/// The children that come before `<variantdefs>` in a schematic or a board.
const BEFORE_VARIANTDEFS: [&str; 4] = ["description", "plain", "libraries", "attributes"];

/// Replaces the children of the group element `group` (for example `variantdefs`) when they differ
/// from `records`; creates the group when there is none and `records` is not empty.
fn write_group(file: &mut Element, group: &str, item: &str, records: &[Record]) {
    let wanted: Vec<Element> = records.iter().map(|r| r.to_element(item)).collect();
    if let Some(existing) = file.children.iter_mut().find(|c| c.name == group) {
        let current: Vec<&Element> = existing
            .children
            .iter()
            .filter(|c| c.name == item)
            .collect();
        if current.len() != wanted.len() || current.iter().zip(&wanted).any(|(a, b)| *a != b) {
            existing.children.retain(|c| c.name != item);
            existing.children.extend(wanted);
        }
        return;
    }
    if wanted.is_empty() {
        return;
    }
    let limit = if group == "attributes" { 3 } else { 4 };
    let at = file
        .children
        .iter()
        .rposition(|c| BEFORE_VARIANTDEFS[..limit].contains(&c.name.as_str()))
        .map_or(0, |i| i + 1);
    file.children.insert(
        at,
        Element {
            name: group.to_owned(),
            children: wanted,
            ..Element::default()
        },
    );
}

/// What a variant did to a board.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Applied {
    /// The components taken out because they are not fitted.
    pub removed: Vec<String>,
    /// The components whose value changed, as reference designator, old value and new value.
    pub revalued: Vec<(String, String, String)>,
}

/// Applies a view to a board: the components that are not fitted are taken out (with their pad
/// connections) and the values of the variant replace the values of the board.
pub fn apply_to_board(board: &mut Board, view: &VariantView) -> Applied {
    let mut applied = Applied::default();
    let mut gone = BTreeSet::new();
    for (id, component) in board.components.iter_mut() {
        let Some(part) = view.parts.get(&component.refdes) else {
            continue;
        };
        if !part.populated {
            gone.insert(*id);
            applied.removed.push(component.refdes.clone());
        } else if part.value != component.value {
            applied.revalued.push((
                component.refdes.clone(),
                component.value.clone(),
                part.value.clone(),
            ));
            component.value = part.value.clone();
        }
    }
    board.components.retain(|id, _| !gone.contains(id));
    board.pad_nets.retain(|p| !gone.contains(&p.component));
    applied.removed.sort();
    applied.revalued.sort();
    applied
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_eagle::xml::{parse, write_document, Limits};

    const SCH: &str = r#"<eagle version="9.6.2"><drawing><schematic>
<description>x</description>
<libraries/>
<attributes><attribute name="REV" value="B" constant="no"/></attributes>
<variantdefs><variantdef name="FULL" current="yes"/><variantdef name="LITE"/></variantdefs>
<parts>
<part name="R1" library="l" deviceset="R" device="" value="10k">
<attribute name="MPN" value="RC0603" x="0" y="0" size="1.27" layer="96" display="off"/>
<variant name="LITE" populate="no"/>
</part>
<part name="R2" library="l" deviceset="R" device="" value="1k">
<variant name="LITE" value="2k2"/>
</part>
<part name="C1" library="l" deviceset="C" device="" value="100n"/>
</parts>
<sheets/>
</schematic></drawing></eagle>"#;

    fn read(text: &str) -> (Element, VariantSet) {
        let root = parse(text, &Limits::default()).unwrap();
        let set = read_variants(&root);
        (root, set)
    }

    #[test]
    fn the_variants_and_attributes_are_read() {
        let (_, set) = read(SCH);
        assert_eq!(set.names(), vec!["FULL", "LITE"]);
        assert_eq!(set.current(), Some("FULL"));
        assert_eq!(set.global_attributes.len(), 1);
        assert_eq!(set.parts.len(), 3);
        assert_eq!(set.parts[0].name(), "R1");
        assert_eq!(set.parts[0].attribute("MPN"), Some("RC0603"));
        assert_eq!(set.parts[0].variants.len(), 1);
        assert!(set.check().is_empty());
    }

    #[test]
    fn a_variant_decides_what_is_fitted_and_with_which_value() {
        let (_, set) = read(SCH);
        let base = set.view("").unwrap();
        assert!(base.not_fitted().is_empty());
        assert_eq!(base.parts["R2"].value, "1k");
        let lite = set.view("LITE").unwrap();
        assert_eq!(lite.not_fitted(), vec!["R1"]);
        assert_eq!(lite.parts["R2"].value, "2k2");
        assert!(lite.parts["R2"].populated);
        assert_eq!(lite.parts["C1"].value, "100n");
        let full = set.view("FULL").unwrap();
        assert!(full.not_fitted().is_empty());
        let error = set.view("OTHER").unwrap_err();
        assert_eq!(error.code, VARIANT_UNKNOWN);
        assert!(error.detail.contains("FULL, LITE"));
    }

    #[test]
    fn reading_and_writing_what_was_read_changes_nothing() {
        let (mut root, set) = read(SCH);
        let before = write_document(&root);
        let findings = write_variants(&mut root, &set);
        assert!(findings.is_empty());
        assert_eq!(write_document(&root), before);
    }

    #[test]
    fn a_changed_set_is_written_and_read_back_the_same() {
        let (mut root, mut set) = read(SCH);
        set.defs.push(Record {
            attrs: vec![("name".to_owned(), "MINI".to_owned())],
        });
        set.parts[2].variants.push(Record {
            attrs: vec![
                ("name".to_owned(), "MINI".to_owned()),
                ("populate".to_owned(), "no".to_owned()),
            ],
        });
        set.parts[0].attributes.push(Record {
            attrs: vec![
                ("name".to_owned(), "NOTE".to_owned()),
                ("value".to_owned(), "hand".to_owned()),
            ],
        });
        let findings = write_variants(&mut root, &set);
        assert!(findings.is_empty());
        let text = write_document(&root);
        let (_, back) = read(&text);
        assert_eq!(back, set);
        assert_eq!(back.view("MINI").unwrap().not_fitted(), vec!["C1"]);
    }

    #[test]
    fn variantdefs_are_created_after_the_libraries_and_attributes() {
        let text = r#"<eagle><drawing><board><plain/><libraries/><attributes/><elements>
<element name="R1" library="l" package="p" value="1k" x="0" y="0"/></elements></board></drawing></eagle>"#;
        let (mut root, mut set) = read(text);
        assert!(set.names().is_empty());
        set.defs.push(Record {
            attrs: vec![("name".to_owned(), "A".to_owned())],
        });
        set.parts[0].variants.push(Record {
            attrs: vec![
                ("name".to_owned(), "A".to_owned()),
                ("populate".to_owned(), "no".to_owned()),
            ],
        });
        assert!(write_variants(&mut root, &set).is_empty());
        let board = root.child("drawing").unwrap().child("board").unwrap();
        let order: Vec<&str> = board.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            order,
            vec![
                "plain",
                "libraries",
                "attributes",
                "variantdefs",
                "elements"
            ]
        );
        let (_, back) = read(&write_document(&root));
        assert_eq!(back.view("A").unwrap().not_fitted(), vec!["R1"]);
    }

    #[test]
    fn a_part_that_the_file_does_not_have_is_named_not_invented() {
        let (mut root, mut set) = read(SCH);
        set.parts.push(PartRecord {
            base: Record {
                attrs: vec![("name".to_owned(), "Z9".to_owned())],
            },
            attributes: Vec::new(),
            variants: Vec::new(),
        });
        let findings = write_variants(&mut root, &set);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].subject.as_deref(), Some("Z9"));
    }

    #[test]
    fn inconsistent_data_is_named() {
        let text = r#"<eagle><drawing><schematic><variantdefs><variantdef name="A"/></variantdefs>
<parts>
<part name="R1" value="1k"><variant name="B" populate="no"/><variant name="A" value="2"/><variant name="A" value="3"/></part>
<part name="R1" value="2k"/>
</parts></schematic></drawing></eagle>"#;
        let (_, set) = read(text);
        let details: Vec<String> = set.check().into_iter().map(|f| f.detail).collect();
        assert_eq!(details.len(), 3);
        assert!(details[0].contains("\"B\" is not defined"));
        assert!(details[1].contains("given twice"));
        assert!(details[2].contains("more than one part"));
        // the first variant of a name is the one that counts
        assert_eq!(set.view("A").unwrap().parts["R1"].value, "2");
    }

    #[test]
    fn a_library_has_no_variants() {
        let (_, set) = read(r#"<eagle><drawing><library name="x"/></drawing></eagle>"#);
        assert_eq!(set, VariantSet::default());
    }
}
