// SPDX-License-Identifier: GPL-3.0-or-later
//! The entities of an EAGLE file that the design model has no place for, read into plain records
//! so that they can be listed and compared: dimensions, holes, vias with their extent, polygons
//! with their pour, thermals, orphans and rank, elements that are smashed or locked, signals with
//! their class, the net classes, the design rules inside a board, the passes of the autorouter and
//! the modules with their instances.
//!
//! The records are a view. The file itself is kept as the XML tree, so everything here is also kept
//! exactly as it is when the tree is written back, whatever the records do or do not hold.
//! The contents of `<libraries>` and `<library>` are not looked at.

use eh3d_phoenix_eagle::xml::Element;
use serde::Serialize;

/// A dimension (a measured length or angle drawn on the board).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dimension {
    /// The type (`parallel`, `horizontal`, `vertical`, `radius`, `diameter`, `angle`, `leader`).
    pub kind: String,
    /// The layer number.
    pub layer: String,
}

/// A hole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hole {
    /// The drill diameter as written.
    pub drill: String,
}

/// A via.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Via {
    /// The layers it joins as written (`1-16`, `1-2`...).
    pub extent: String,
    /// The drill diameter as written.
    pub drill: String,
    /// The shape (`round` when the file does not say).
    pub shape: String,
}

/// A polygon (a pour or a plain shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Polygon {
    /// The layer number.
    pub layer: String,
    /// The width of its outline as written.
    pub width: String,
    /// The distance to other nets when the file says one.
    pub isolate: Option<String>,
    /// `solid`, `hatch` or `cutout`.
    pub pour: String,
    /// Whether it makes thermals (`yes` when the file does not say).
    pub thermals: String,
    /// Whether it keeps islands of copper (`no` when the file does not say).
    pub orphans: String,
    /// The order in which polygons are poured (`0` when the file does not say).
    pub rank: String,
    /// The vertices.
    pub vertices: usize,
}

/// An element of a board that is smashed or locked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlacedElement {
    /// The reference designator.
    pub name: String,
    /// True when its texts were separated from it.
    pub smashed: bool,
    /// True when it cannot be moved.
    pub locked: bool,
}

/// A signal with its class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Signal {
    /// The name of the signal.
    pub name: String,
    /// The number of its net class (`0` when the file does not say).
    pub class: String,
}

/// A net class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetClassInfo {
    /// The number.
    pub number: String,
    /// The name.
    pub name: String,
    /// The width of its tracks as written.
    pub width: String,
    /// The drill of its vias as written.
    pub drill: String,
    /// The clearances to other classes.
    pub clearances: usize,
}

/// The `<designrules>` of a board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesignRulesInfo {
    /// The name of the set of rules.
    pub name: String,
    /// The number of parameters.
    pub params: usize,
}

/// A pass of the autorouter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AutorouterPass {
    /// The name of the pass.
    pub name: String,
    /// Whether the pass is active (`yes` when the file does not say).
    pub active: String,
    /// The number of parameters.
    pub params: usize,
}

/// A module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Module {
    /// The name.
    pub name: String,
    /// The prefix of the names of its parts.
    pub prefix: String,
    /// The number of parts.
    pub parts: usize,
    /// The number of sheets.
    pub sheets: usize,
    /// The number of ports.
    pub ports: usize,
}

/// An instance of a module on a sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleInstance {
    /// The name of the instance.
    pub name: String,
    /// The module it instantiates.
    pub module: String,
    /// The variant of the module, when one is chosen.
    pub variant: Option<String>,
}

/// Everything listed above, from one file.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Entities {
    /// The dimensions.
    pub dimensions: Vec<Dimension>,
    /// The holes.
    pub holes: Vec<Hole>,
    /// The vias.
    pub vias: Vec<Via>,
    /// The polygons.
    pub polygons: Vec<Polygon>,
    /// The elements that are smashed or locked.
    pub elements: Vec<PlacedElement>,
    /// The signals.
    pub signals: Vec<Signal>,
    /// The net classes.
    pub classes: Vec<NetClassInfo>,
    /// The design rules of the board.
    pub design_rules: Option<DesignRulesInfo>,
    /// The passes of the autorouter.
    pub autorouter: Vec<AutorouterPass>,
    /// The modules.
    pub modules: Vec<Module>,
    /// The instances of modules.
    pub module_instances: Vec<ModuleInstance>,
}

fn text(e: &Element, key: &str) -> String {
    e.attr(key).unwrap_or("").to_owned()
}

fn text_or(e: &Element, key: &str, default: &str) -> String {
    e.attr(key).unwrap_or(default).to_owned()
}

/// Reads the entities of a parsed file.
pub fn read_entities(root: &Element) -> Entities {
    let mut out = Entities::default();
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if e.name == "libraries" || e.name == "library" {
            continue;
        }
        stack.extend(e.children.iter().rev());
        match e.name.as_str() {
            "dimension" => out.dimensions.push(Dimension {
                kind: text_or(e, "dtype", "parallel"),
                layer: text(e, "layer"),
            }),
            "hole" => out.holes.push(Hole {
                drill: text(e, "drill"),
            }),
            "via" => out.vias.push(Via {
                extent: text_or(e, "extent", "1-16"),
                drill: text(e, "drill"),
                shape: text_or(e, "shape", "round"),
            }),
            "polygon" => out.polygons.push(Polygon {
                layer: text(e, "layer"),
                width: text(e, "width"),
                isolate: e.attr("isolate").map(str::to_owned),
                pour: text_or(e, "pour", "solid"),
                thermals: text_or(e, "thermals", "yes"),
                orphans: text_or(e, "orphans", "no"),
                rank: text_or(e, "rank", "0"),
                vertices: e.children_named("vertex").count(),
            }),
            "element" => {
                let smashed = e.attr("smashed") == Some("yes");
                let locked = e.attr("locked") == Some("yes");
                if smashed || locked {
                    out.elements.push(PlacedElement {
                        name: text(e, "name"),
                        smashed,
                        locked,
                    });
                }
            }
            "signal" => out.signals.push(Signal {
                name: text(e, "name"),
                class: text_or(e, "class", "0"),
            }),
            "class" => out.classes.push(NetClassInfo {
                number: text(e, "number"),
                name: text(e, "name"),
                width: text(e, "width"),
                drill: text(e, "drill"),
                clearances: e.children_named("clearance").count(),
            }),
            "designrules" => {
                if out.design_rules.is_none() {
                    out.design_rules = Some(DesignRulesInfo {
                        name: text(e, "name"),
                        params: e.children_named("param").count(),
                    });
                }
            }
            "pass" => out.autorouter.push(AutorouterPass {
                name: text(e, "name"),
                active: text_or(e, "active", "yes"),
                params: e.children_named("param").count(),
            }),
            "module" => out.modules.push(Module {
                name: text(e, "name"),
                prefix: text(e, "prefix"),
                parts: e
                    .child("parts")
                    .map_or(0, |p| p.children_named("part").count()),
                sheets: e
                    .child("sheets")
                    .map_or(0, |p| p.children_named("sheet").count()),
                ports: e
                    .child("ports")
                    .map_or(0, |p| p.children_named("port").count()),
            }),
            "moduleinst" => out.module_instances.push(ModuleInstance {
                name: text(e, "name"),
                module: text(e, "module"),
                variant: e.attr("modulevariant").map(str::to_owned),
            }),
            _ => {}
        }
    }
    out
}

impl Entities {
    /// A short text, one line for each kind that the file has.
    pub fn summary(&self) -> String {
        let mut out = String::new();
        let mut line = |what: &str, n: usize| {
            if n > 0 {
                out.push_str(&format!("{what}: {n}\n"));
            }
        };
        line("dimensions", self.dimensions.len());
        line("holes", self.holes.len());
        line("vias", self.vias.len());
        line("polygons", self.polygons.len());
        line("elements smashed or locked", self.elements.len());
        line("signals", self.signals.len());
        line("net classes", self.classes.len());
        line("design rules", usize::from(self.design_rules.is_some()));
        line("autorouter passes", self.autorouter.len());
        line("modules", self.modules.len());
        line("module instances", self.module_instances.len());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_eagle::xml::{parse, Limits};

    const BOARD: &str = r#"<eagle version="9.6.2"><drawing><board>
<plain>
<dimension x1="0" y1="0" x2="10" y2="0" x3="5" y3="2" textsize="1" layer="47" dtype="horizontal"/>
<dimension x1="0" y1="0" x2="10" y2="0" x3="5" y3="2" textsize="1" layer="47"/>
<hole x="1" y="1" drill="3.2"/>
</plain>
<libraries><library name="l"><packages><package name="p"><hole x="0" y="0" drill="1"/></package></packages></library></libraries>
<classes><class number="0" name="default" width="0" drill="0"/>
<class number="1" name="power" width="0.5" drill="0.4"><clearance class="0" value="0.3"/></class></classes>
<designrules name="mine"><param name="msWidth" value="6mil"/><param name="mdWireWire" value="8mil"/></designrules>
<autorouter><pass name="Default" refer="Route" active="yes"><param name="Cost" value="1"/></pass><pass name="Busses" active="no"/></autorouter>
<elements>
<element name="R1" library="l" package="p" value="1k" x="1" y="1" smashed="yes"/>
<element name="R2" library="l" package="p" value="1k" x="1" y="1" locked="yes"/>
<element name="R3" library="l" package="p" value="1k" x="1" y="1"/>
</elements>
<signals>
<signal name="GND" class="1">
<via x="1" y="1" extent="1-2" drill="0.3" shape="square"/>
<via x="2" y="2" extent="1-16" drill="0.3"/>
<polygon width="0.2" layer="1" isolate="0.3" pour="hatch" thermals="no" orphans="yes" rank="2">
<vertex x="0" y="0"/><vertex x="5" y="0"/><vertex x="5" y="5"/></polygon>
</signal>
<signal name="SIG"/>
</signals>
</board></drawing></eagle>"#;

    #[test]
    fn the_entities_of_a_board_are_read_with_their_defaults() {
        let root = parse(BOARD, &Limits::default()).unwrap();
        let e = read_entities(&root);
        assert_eq!(e.dimensions.len(), 2);
        assert_eq!(e.dimensions[0].kind, "horizontal");
        assert_eq!(e.dimensions[1].kind, "parallel");
        assert_eq!(
            e.holes,
            vec![Hole {
                drill: "3.2".to_owned()
            }]
        );
        assert_eq!(e.vias.len(), 2);
        assert_eq!(e.vias[0].extent, "1-2");
        assert_eq!(e.vias[0].shape, "square");
        assert_eq!(e.vias[1].extent, "1-16");
        assert_eq!(e.vias[1].shape, "round");
        assert_eq!(e.polygons.len(), 1);
        let p = &e.polygons[0];
        assert_eq!(
            (
                p.pour.as_str(),
                p.thermals.as_str(),
                p.orphans.as_str(),
                p.rank.as_str()
            ),
            ("hatch", "no", "yes", "2")
        );
        assert_eq!(p.isolate.as_deref(), Some("0.3"));
        assert_eq!(p.vertices, 3);
        let names: Vec<&str> = e.elements.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["R1", "R2"]);
        assert!(e.elements[0].smashed && !e.elements[0].locked);
        assert!(!e.elements[1].smashed && e.elements[1].locked);
        assert_eq!(e.signals.len(), 2);
        assert_eq!(e.signals[0].class, "1");
        assert_eq!(e.signals[1].class, "0");
        assert_eq!(e.classes.len(), 2);
        assert_eq!(e.classes[1].clearances, 1);
        assert_eq!(
            e.design_rules,
            Some(DesignRulesInfo {
                name: "mine".to_owned(),
                params: 2
            })
        );
        assert_eq!(e.autorouter.len(), 2);
        assert_eq!(e.autorouter[0].params, 1);
        assert_eq!(e.autorouter[1].active, "no");
    }

    #[test]
    fn the_contents_of_libraries_are_not_counted() {
        let root = parse(BOARD, &Limits::default()).unwrap();
        // the hole of the package is not in the list
        assert_eq!(read_entities(&root).holes.len(), 1);
    }

    #[test]
    fn modules_and_their_instances_are_read() {
        let text = r#"<eagle><drawing><schematic>
<modules><module name="PSU" prefix="P_" dx="10" dy="10">
<ports><port name="VIN" side="left" coord="2" direction="in"/><port name="GND" side="left" coord="4" direction="pwr"/></ports>
<parts><part name="C1" library="l" deviceset="C" device=""/></parts>
<sheets><sheet/></sheets></module></modules>
<sheets><sheet><moduleinsts><moduleinst name="PSU1" module="PSU" modulevariant="LITE" x="0" y="0" offset="0"/>
<moduleinst name="PSU2" module="PSU" x="0" y="0" offset="1"/></moduleinsts></sheet></sheets>
</schematic></drawing></eagle>"#;
        let root = parse(text, &Limits::default()).unwrap();
        let e = read_entities(&root);
        assert_eq!(
            e.modules,
            vec![Module {
                name: "PSU".to_owned(),
                prefix: "P_".to_owned(),
                parts: 1,
                sheets: 1,
                ports: 2
            }]
        );
        assert_eq!(e.module_instances.len(), 2);
        assert_eq!(e.module_instances[0].variant.as_deref(), Some("LITE"));
        assert_eq!(e.module_instances[1].variant, None);
    }

    #[test]
    fn the_summary_lists_only_what_the_file_has() {
        let root = parse(BOARD, &Limits::default()).unwrap();
        let summary = read_entities(&root).summary();
        assert!(summary.contains("dimensions: 2\n"));
        assert!(summary.contains("elements smashed or locked: 2\n"));
        assert!(!summary.contains("modules"));
        assert_eq!(Entities::default().summary(), "");
    }
}
