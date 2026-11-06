// SPDX-License-Identifier: GPL-3.0-or-later
//! The loss report of an EAGLE file (`phoenix eagle-check`): what a rewrite keeps, what it
//! simplifies and what it leaves out.
//!
//! There are two ways to write an EAGLE file back. Rewriting the XML tree keeps everything: the
//! report proves it by writing the tree, reading it again and comparing, and by comparing the text
//! of the nodes that no record covers. Reading the file into the design model of a board or a
//! schematic and writing that model keeps only what the model holds: the report names, with a
//! stable code, every kind of element that this second way simplifies or leaves out, and the kinds
//! of element that nothing in this tool interprets (they are kept byte for byte in the tree).
//!
//! The list of what the design model reads is written from the readers of `eh3d-phoenix-eagle-board`
//! as they stand; it is approximate for libraries, which have no reader into a model of their own.

use std::collections::BTreeMap;

use eh3d_phoenix_eagle::xml::{parse, write_document, Element, Limits};
use serde::Serialize;

use crate::codes::{
    DeepError, Finding, Severity, KEPT_OPAQUE, LEFT_OUT_BY_MODEL, NOT_EAGLE_FILE, REWRITE_CHANGED,
    SIMPLIFIED_BY_MODEL,
};
use crate::entities::{read_entities, Entities};
use crate::layers::{read_layers, FileLayer, LayerMap, RoleSource};
use crate::variants::{container, read_variants};

/// The kind of file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Board,
    Schematic,
    Library,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Board => "board",
            Kind::Schematic => "schematic",
            Kind::Library => "library",
        }
    }
}

/// The names of the elements that the records of this crate cover (the design model leaves them
/// out, but they are listed and kept).
const COVERED: &[&str] = &[
    "attribute",
    "variantdefs",
    "variantdef",
    "variant",
    "dimension",
    "hole",
    "designrules",
    "autorouter",
    "classes",
    "class",
    "clearance",
    "modules",
    "moduleinst",
];

const BOARD_KEPT: &[&str] = &[
    "eagle",
    "drawing",
    "layers",
    "board",
    "plain",
    "libraries",
    "library",
    "packages",
    "package",
    "pad",
    "smd",
    "elements",
    "element",
    "signals",
    "signal",
    "contactref",
];

const SCHEMATIC_KEPT: &[&str] = &[
    "eagle",
    "drawing",
    "layers",
    "schematic",
    "libraries",
    "library",
    "symbols",
    "symbol",
    "pin",
    "devicesets",
    "deviceset",
    "gates",
    "gate",
    "devices",
    "device",
    "parts",
    "part",
    "modules",
    "module",
    "ports",
    "port",
    "sheets",
    "sheet",
    "instances",
    "instance",
    "moduleinsts",
    "moduleinst",
    "nets",
    "net",
    "segment",
    "pinref",
    "wire",
    "vertex",
];

const LIBRARY_KEPT: &[&str] = &[
    "eagle",
    "drawing",
    "layers",
    "library",
    "packages",
    "package",
    "pad",
    "smd",
    "symbols",
    "symbol",
    "pin",
    "devicesets",
    "deviceset",
    "gates",
    "gate",
    "devices",
    "device",
    "technologies",
];

fn layer_number(e: &Element) -> Option<u16> {
    e.attr("layer").and_then(|n| n.trim().parse().ok())
}

/// True when the design model reads the element.
fn is_kept(kind: Kind, parent: &str, e: &Element) -> bool {
    let name = e.name.as_str();
    match kind {
        Kind::Board => match name {
            "layer" => e
                .attr("number")
                .and_then(|n| n.trim().parse::<u16>().ok())
                .is_some_and(|n| matches!(n, 1..=16 | 20..=22 | 29..=32 | 51 | 52)),
            "wire" => {
                parent == "signal"
                    || (parent == "plain" && layer_number(e).is_some_and(|n| matches!(n, 20..=22)))
            }
            "via" | "polygon" => parent == "signal",
            "vertex" => parent == "polygon",
            _ => BOARD_KEPT.contains(&name),
        },
        Kind::Schematic => name == "layer" || SCHEMATIC_KEPT.contains(&name),
        Kind::Library => name == "layer" || LIBRARY_KEPT.contains(&name),
    }
}

fn is_set(e: &Element, key: &str, value: &str) -> bool {
    e.attr(key) == Some(value)
}

/// What the design model simplifies in an element that it reads.
fn simplified(kind: Kind, parent: &str, e: &Element) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    match (kind, e.name.as_str()) {
        (Kind::Board, "pad") | (Kind::Library, "pad") => {
            if e.attr("shape")
                .is_some_and(|s| s != "round" && s != "square")
            {
                parts.push("shape");
            }
        }
        (Kind::Board, "smd") | (Kind::Library, "smd") => {
            if e.attr("roundness").is_some_and(|r| r != "0" && r != "100") {
                parts.push("roundness");
            }
            for key in ["stop", "cream", "thermals"] {
                if is_set(e, key, "no") {
                    parts.push(key);
                }
            }
        }
        (Kind::Board, "element") => {
            if is_set(e, "smashed", "yes") {
                parts.push("smashed");
            }
            if is_set(e, "locked", "yes") {
                parts.push("locked");
            }
        }
        (Kind::Board, "signal") => {
            if e.attr("class").is_some_and(|c| c != "0") {
                parts.push("class");
            }
        }
        (Kind::Board, "via") => {
            if e.attr("shape").is_some_and(|s| s != "round") {
                parts.push("shape");
            }
        }
        (Kind::Board, "polygon") if parent == "signal" => {
            if e.attr("pour").is_some_and(|p| p != "solid") {
                parts.push("pour");
            }
            if is_set(e, "thermals", "no") {
                parts.push("thermals");
            }
            if is_set(e, "orphans", "yes") {
                parts.push("orphans");
            }
            if e.attr("rank").is_some_and(|r| r != "0") {
                parts.push("rank");
            }
            if e.attr("spacing").is_some() {
                parts.push("spacing");
            }
        }
        (Kind::Schematic, "module") if e.child("parts").is_some() || e.child("sheets").is_some() => {
            parts.push("contents");
        }
        _ => {}
    }
    (!parts.is_empty()).then(|| format!("{} not kept", parts.join(", ")))
}

/// The report of one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EagleCheck {
    /// `board`, `schematic` or `library`.
    pub kind: String,
    /// The version the file declares.
    pub version: String,
    /// The number of elements.
    pub nodes: usize,
    /// True when writing the tree and reading it again gives the same tree.
    pub tree_lossless: bool,
    /// The nodes under the board or schematic that nothing reads or covers.
    pub opaque_nodes: usize,
    /// How many of the opaque nodes have the same text after the rewrite.
    pub opaque_identical: usize,
    /// The layers of the file.
    pub layers: Vec<FileLayer>,
    /// The names of the variants.
    pub variants: Vec<String>,
    /// The entities the design model has no place for.
    pub entities: Entities,
    /// Everything that is kept, simplified or left out, in order.
    pub findings: Vec<Finding>,
}

/// The nodes under the schematic or board that no record covers, as text.
fn opaque_texts(root: &Element) -> Vec<String> {
    let Some((file, _)) = container(root) else {
        return Vec::new();
    };
    file.children
        .iter()
        .filter(|c| !COVERED.contains(&c.name.as_str()))
        .filter(|c| {
            !matches!(
                c.name.as_str(),
                "attributes" | "libraries" | "elements" | "parts"
            )
        })
        .filter(|c| !matches!(c.name.as_str(), "signals" | "sheets" | "plain"))
        .map(Element::to_xml)
        .collect()
}

/// Reads a file and reports what a rewrite keeps, simplifies and leaves out.
pub fn check_eagle(text: &str, map: &LayerMap) -> Result<EagleCheck, DeepError> {
    let root = parse(text, &Limits::default())
        .map_err(|e| DeepError::new(NOT_EAGLE_FILE, e.to_string()))?;
    if root.name != "eagle" {
        return Err(DeepError::new(
            NOT_EAGLE_FILE,
            "the root element is not eagle",
        ));
    }
    let Some(drawing) = root.child("drawing") else {
        return Err(DeepError::new(NOT_EAGLE_FILE, "there is no drawing"));
    };
    let kind = if drawing.child("board").is_some() {
        Kind::Board
    } else if drawing.child("schematic").is_some() {
        Kind::Schematic
    } else if drawing.child("library").is_some() {
        Kind::Library
    } else {
        return Err(DeepError::new(
            NOT_EAGLE_FILE,
            "the drawing has no board, schematic or library",
        ));
    };

    let mut findings = Vec::new();
    let rewritten = write_document(&root);
    let again = parse(&rewritten, &Limits::default());
    let tree_lossless = again.as_ref().is_ok_and(|a| *a == root);
    let before = opaque_texts(&root);
    let after = again.as_ref().map(opaque_texts).unwrap_or_default();
    let opaque_identical = before
        .iter()
        .zip(after.iter())
        .filter(|(a, b)| a == b)
        .count();
    if !tree_lossless {
        findings.push(Finding::new(
            REWRITE_CHANGED,
            Severity::Error,
            None,
            "writing the tree and reading it again gives another tree",
        ));
    }

    let layers = read_layers(&root, map);
    findings.extend(layers.findings.clone());
    let set = read_variants(&root);
    findings.extend(set.check());

    // what the design model reads, simplifies or leaves out
    let mut left_out: BTreeMap<String, usize> = BTreeMap::new();
    let mut reasons: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut stack: Vec<(&Element, &str)> = vec![(&root, "")];
    while let Some((e, parent)) = stack.pop() {
        if !is_kept(kind, parent, e) {
            *left_out.entry(e.name.clone()).or_insert(0) += 1;
            continue;
        }
        if let Some(why) = simplified(kind, parent, e) {
            *reasons.entry((e.name.clone(), why)).or_insert(0) += 1;
        }
        for child in e.children.iter().rev() {
            stack.push((child, e.name.as_str()));
        }
    }
    for ((name, why), n) in &reasons {
        findings.push(Finding::new(
            SIMPLIFIED_BY_MODEL,
            Severity::Info,
            Some(name),
            format!("{n} element(s): {why}"),
        ));
    }
    for (name, n) in &left_out {
        if COVERED.contains(&name.as_str()) {
            findings.push(Finding::new(
                LEFT_OUT_BY_MODEL,
                Severity::Warning,
                Some(name),
                format!("{n} element(s): kept in the tree, read into the records of this crate, left out by the design model"),
            ));
        } else {
            findings.push(Finding::new(
                KEPT_OPAQUE,
                Severity::Info,
                Some(name),
                format!(
                    "{n} element(s): kept byte for byte in the tree, left out by the design model"
                ),
            ));
        }
    }

    Ok(EagleCheck {
        kind: kind.name().to_owned(),
        version: root.attr("version").unwrap_or("").to_owned(),
        nodes: root.count(),
        tree_lossless,
        opaque_nodes: before.len(),
        opaque_identical,
        layers: layers.layers,
        variants: set.names(),
        entities: read_entities(&root),
        findings,
    })
}

impl EagleCheck {
    /// The report as text in English, one finding per line after a short header.
    pub fn to_text(&self, lang: eh3d_phoenix_rules::Lang) -> String {
        let mut out = format!(
            "{} (version {}), {} elements\n",
            self.kind, self.version, self.nodes
        );
        out.push_str(&format!(
            "rewrite of the tree: {} ({} of {} opaque nodes with the same text)\n",
            if self.tree_lossless {
                "identical"
            } else {
                "CHANGED"
            },
            self.opaque_identical,
            self.opaque_nodes
        ));
        let user = self
            .layers
            .iter()
            .filter(|l| l.source != RoleSource::Standard)
            .count();
        out.push_str(&format!(
            "layers: {} ({} user layers)\n",
            self.layers.len(),
            user
        ));
        if !self.variants.is_empty() {
            out.push_str(&format!("variants: {}\n", self.variants.join(", ")));
        }
        out.push_str(&self.entities.summary());
        out.push_str(&crate::codes::lines(&self.findings, lang));
        out
    }

    /// The report as JSON.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// True when a finding is an error.
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{LAYER_UNMAPPED, VARIANT_INCONSISTENT};

    const BOARD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE eagle SYSTEM "eagle.dtd">
<eagle version="9.6.2"><drawing>
<layers>
<layer number="1" name="Top" color="4" fill="1" visible="yes" active="yes"/>
<layer number="16" name="Bottom" color="1" fill="1" visible="yes" active="yes"/>
<layer number="100" name="Mine" color="2" fill="1" visible="yes" active="yes"/>
</layers>
<board>
<description>note</description>
<plain>
<wire x1="0" y1="0" x2="10" y2="0" width="0" layer="20"/>
<wire x1="0" y1="0" x2="10" y2="0" width="0.1" layer="100"/>
<text x="1" y="1" size="1" layer="21">hello</text>
<dimension x1="0" y1="0" x2="10" y2="0" x3="5" y3="2" textsize="1" layer="47"/>
</plain>
<libraries><library name="l"><packages><package name="p">
<smd name="1" x="0" y="0" dx="1" dy="1" layer="1" roundness="50"/>
<pad name="2" x="2" y="0" drill="0.8" diameter="1.6" shape="octagon"/>
<wire x1="0" y1="0" x2="1" y2="0" width="0.1" layer="21"/>
</package></packages></library></libraries>
<variantdefs><variantdef name="A"/></variantdefs>
<designrules name="r"><param name="msWidth" value="6mil"/></designrules>
<autorouter><pass name="Default"/></autorouter>
<elements>
<element name="R1" library="l" package="p" value="1k" x="1" y="1" smashed="yes"><attribute name="MPN" value="x"/><variant name="A" populate="no"/><variant name="B"/></element>
</elements>
<signals><signal name="GND" class="1">
<contactref element="R1" pad="1"/>
<wire x1="0" y1="0" x2="1" y2="0" width="0.2" layer="1"/>
<polygon width="0.2" layer="1" pour="hatch" rank="2"><vertex x="0" y="0"/><vertex x="1" y="0"/><vertex x="1" y="1"/></polygon>
</signal></signals>
</board></drawing></eagle>"#;

    fn find<'a>(check: &'a EagleCheck, code: &str, subject: &str) -> Option<&'a Finding> {
        check
            .findings
            .iter()
            .find(|f| f.code == code && f.subject.as_deref() == Some(subject))
    }

    #[test]
    fn the_tree_rewrite_is_proven_lossless_and_the_model_loss_is_named() {
        let check = check_eagle(BOARD, &LayerMap::default()).unwrap();
        assert_eq!(check.kind, "board");
        assert_eq!(check.version, "9.6.2");
        assert!(check.tree_lossless);
        assert_eq!(check.opaque_nodes, check.opaque_identical);
        assert_eq!(check.opaque_nodes, 1);
        assert_eq!(check.variants, vec!["A"]);
        assert!(!check.has_errors());
        // read by the model but simplified
        let element = find(&check, SIMPLIFIED_BY_MODEL, "element").unwrap();
        assert_eq!(element.detail, "1 element(s): smashed not kept");
        assert_eq!(
            find(&check, SIMPLIFIED_BY_MODEL, "signal").unwrap().detail,
            "1 element(s): class not kept"
        );
        assert_eq!(
            find(&check, SIMPLIFIED_BY_MODEL, "polygon").unwrap().detail,
            "1 element(s): pour, rank not kept"
        );
        // left out, covered by the records
        for name in [
            "dimension",
            "variantdefs",
            "designrules",
            "autorouter",
            "attribute",
            "variant",
        ] {
            let f = find(&check, LEFT_OUT_BY_MODEL, name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(f.severity, Severity::Warning);
        }
        assert_eq!(
            find(&check, LEFT_OUT_BY_MODEL, "variant")
                .unwrap()
                .detail
                .split(' ')
                .next(),
            Some("2")
        );
        // left out, covered by nothing
        for name in ["description", "text"] {
            assert_eq!(
                find(&check, KEPT_OPAQUE, name).unwrap().severity,
                Severity::Info
            );
        }
        // the wire on the user layer of the plain drawing, and the hole-free package items
        assert_eq!(
            find(&check, KEPT_OPAQUE, "wire")
                .unwrap()
                .detail
                .split(' ')
                .next(),
            Some("2")
        );
        // a user layer without a role, and a variant that is not defined
        assert!(find(&check, LAYER_UNMAPPED, "Mine").is_some());
        assert!(find(&check, VARIANT_INCONSISTENT, "R1").is_some());
    }

    #[test]
    fn the_map_gives_a_user_layer_a_role() {
        let map = LayerMap::parse("layer.100 = silk_top\n").unwrap();
        let check = check_eagle(BOARD, &map).unwrap();
        assert!(find(&check, LAYER_UNMAPPED, "Mine").is_none());
    }

    #[test]
    fn a_schematic_and_a_library_are_told_apart() {
        let sch = r#"<eagle version="9.6.2"><drawing><schematic><description>d</description>
<libraries/><parts><part name="R1" library="l" deviceset="R" device="" value="1k"><attribute name="X" value="1"/></part></parts>
<modules><module name="M" prefix="M_" dx="1" dy="1"><parts/><sheets/></module></modules>
<sheets><sheet><plain><text x="0" y="0" size="1" layer="97">t</text></plain></sheet></sheets></schematic></drawing></eagle>"#;
        let check = check_eagle(sch, &LayerMap::default()).unwrap();
        assert_eq!(check.kind, "schematic");
        assert!(check.tree_lossless);
        assert_eq!(
            find(&check, SIMPLIFIED_BY_MODEL, "module").unwrap().detail,
            "1 element(s): contents not kept"
        );
        assert!(find(&check, LEFT_OUT_BY_MODEL, "attribute").is_some());
        assert!(find(&check, KEPT_OPAQUE, "plain").is_some());
        let lbr = r#"<eagle version="9.6.2"><drawing><library name="x"><packages/></library></drawing></eagle>"#;
        assert_eq!(
            check_eagle(lbr, &LayerMap::default()).unwrap().kind,
            "library"
        );
    }

    #[test]
    fn what_is_not_an_eagle_file_gets_a_code() {
        for text in [
            "",
            "not xml",
            "<other/>",
            "<eagle/>",
            "<eagle><drawing/></eagle>",
        ] {
            let error = check_eagle(text, &LayerMap::default()).unwrap_err();
            assert_eq!(error.code, NOT_EAGLE_FILE, "{text:?}");
        }
    }

    #[test]
    fn the_report_is_the_same_every_time_in_text_and_in_json() {
        let a = check_eagle(BOARD, &LayerMap::default()).unwrap();
        let b = check_eagle(BOARD, &LayerMap::default()).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.to_json(), b.to_json());
        let text = a.to_text(eh3d_phoenix_rules::Lang::Pt);
        assert!(text.starts_with("board (version 9.6.2), "));
        assert!(text.contains("rewrite of the tree: identical"));
        assert!(text.contains("PHOENIX-EAGLEX-019"));
        assert!(text.contains("O elemento fica na árvore"));
        let value: serde_json::Value = serde_json::from_str(&a.to_json()).unwrap();
        assert_eq!(value["kind"], "board");
    }
}
