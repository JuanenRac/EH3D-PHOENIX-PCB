// SPDX-License-Identifier: GPL-3.0-or-later
//! The layers of EAGLE: the table of the standard layers (1 to 99) with their roles, the user
//! layers (100 to 255) with a map that the settings file `config/eagle_layers.cfg` configures, and
//! the reading of the `<layers>` of a file with a finding for everything that is odd in it.
//!
//! The settings file is a plain `key = value` text. `layer.<number> = <role>` gives a user layer
//! (100 to 255) a role by its number and `name.<Name> = <role>` by its name in the file; the roles
//! are the words of [`LayerRole::token`]. A user layer without a role is kept in the file and is
//! named in a finding, and it is not exported.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_eagle::xml::Element;
use serde::Serialize;

use crate::codes::{
    DeepError, Finding, Severity, LAYER_MAP_INVALID, LAYER_NUMBER_INVALID, LAYER_RENAMED,
    LAYER_UNMAPPED,
};

/// The largest layer map settings file that is read, in bytes.
pub const MAX_MAP_BYTES: usize = 1 << 20;

/// What a layer is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerRole {
    /// A copper layer (1 to 16).
    Copper,
    /// The pads (17).
    Pads,
    /// The vias (18).
    Vias,
    /// The ratsnest of unrouted connections (19).
    Ratsnest,
    /// The board outline (20).
    Outline,
    /// The silkscreen on top (21).
    SilkTop,
    /// The silkscreen on the bottom (22).
    SilkBottom,
    /// The origins of the parts on top (23).
    OriginsTop,
    /// The origins of the parts on the bottom (24).
    OriginsBottom,
    /// The names of the parts on top (25).
    NamesTop,
    /// The names of the parts on the bottom (26).
    NamesBottom,
    /// The values of the parts on top (27).
    ValuesTop,
    /// The values of the parts on the bottom (28).
    ValuesBottom,
    /// The solder mask on top (29).
    MaskTop,
    /// The solder mask on the bottom (30).
    MaskBottom,
    /// The solder paste on top (31).
    PasteTop,
    /// The solder paste on the bottom (32).
    PasteBottom,
    /// The finish on top (33).
    FinishTop,
    /// The finish on the bottom (34).
    FinishBottom,
    /// The glue on top (35).
    GlueTop,
    /// The glue on the bottom (36).
    GlueBottom,
    /// The test points on top (37).
    TestTop,
    /// The test points on the bottom (38).
    TestBottom,
    /// The keep-out area for parts on top (39).
    KeepoutTop,
    /// The keep-out area for parts on the bottom (40).
    KeepoutBottom,
    /// The restricted area for copper on top (41).
    RestrictTop,
    /// The restricted area for copper on the bottom (42).
    RestrictBottom,
    /// The restricted area for vias (43).
    RestrictVia,
    /// The drill symbols and the plated holes (44).
    Drill,
    /// The holes that are not plated (45).
    Hole,
    /// The milling (46).
    Milling,
    /// The measures (47).
    Measures,
    /// The documentation (48).
    Document,
    /// The reference marks (49).
    Reference,
    /// The drawing imported from a DXF file (50).
    DxfImport,
    /// The documentation on top (51).
    DocuTop,
    /// The documentation on the bottom (52).
    DocuBottom,
    /// The ground name on top (53).
    GroundTop,
    /// The ground name on the bottom (54).
    GroundBottom,
    /// The nets of a schematic (91).
    SchematicNets,
    /// The busses of a schematic (92).
    SchematicBusses,
    /// The pins of a symbol (93).
    SchematicPins,
    /// The symbols (94).
    SchematicSymbols,
    /// The names in a schematic (95).
    SchematicNames,
    /// The values in a schematic (96).
    SchematicValues,
    /// The information of a schematic (97).
    SchematicInfo,
    /// The guide lines of a schematic (98).
    SchematicGuide,
    /// The order of the parts for the simulation (99).
    SpiceOrder,
    /// A user layer with no standard role.
    User,
}

/// The words of the roles in the settings file and in the reports.
const ROLE_TOKENS: &[(LayerRole, &str)] = &[
    (LayerRole::Copper, "copper"),
    (LayerRole::Pads, "pads"),
    (LayerRole::Vias, "vias"),
    (LayerRole::Ratsnest, "ratsnest"),
    (LayerRole::Outline, "outline"),
    (LayerRole::SilkTop, "silk_top"),
    (LayerRole::SilkBottom, "silk_bottom"),
    (LayerRole::OriginsTop, "origins_top"),
    (LayerRole::OriginsBottom, "origins_bottom"),
    (LayerRole::NamesTop, "names_top"),
    (LayerRole::NamesBottom, "names_bottom"),
    (LayerRole::ValuesTop, "values_top"),
    (LayerRole::ValuesBottom, "values_bottom"),
    (LayerRole::MaskTop, "mask_top"),
    (LayerRole::MaskBottom, "mask_bottom"),
    (LayerRole::PasteTop, "paste_top"),
    (LayerRole::PasteBottom, "paste_bottom"),
    (LayerRole::FinishTop, "finish_top"),
    (LayerRole::FinishBottom, "finish_bottom"),
    (LayerRole::GlueTop, "glue_top"),
    (LayerRole::GlueBottom, "glue_bottom"),
    (LayerRole::TestTop, "test_top"),
    (LayerRole::TestBottom, "test_bottom"),
    (LayerRole::KeepoutTop, "keepout_top"),
    (LayerRole::KeepoutBottom, "keepout_bottom"),
    (LayerRole::RestrictTop, "restrict_top"),
    (LayerRole::RestrictBottom, "restrict_bottom"),
    (LayerRole::RestrictVia, "restrict_via"),
    (LayerRole::Drill, "drill"),
    (LayerRole::Hole, "hole"),
    (LayerRole::Milling, "milling"),
    (LayerRole::Measures, "measures"),
    (LayerRole::Document, "document"),
    (LayerRole::Reference, "reference"),
    (LayerRole::DxfImport, "dxf"),
    (LayerRole::DocuTop, "docu_top"),
    (LayerRole::DocuBottom, "docu_bottom"),
    (LayerRole::GroundTop, "ground_top"),
    (LayerRole::GroundBottom, "ground_bottom"),
    (LayerRole::SchematicNets, "schematic_nets"),
    (LayerRole::SchematicBusses, "schematic_busses"),
    (LayerRole::SchematicPins, "schematic_pins"),
    (LayerRole::SchematicSymbols, "schematic_symbols"),
    (LayerRole::SchematicNames, "schematic_names"),
    (LayerRole::SchematicValues, "schematic_values"),
    (LayerRole::SchematicInfo, "schematic_info"),
    (LayerRole::SchematicGuide, "schematic_guide"),
    (LayerRole::SpiceOrder, "spice_order"),
    (LayerRole::User, "user"),
];

impl LayerRole {
    /// The word of the role in the settings file.
    pub fn token(self) -> &'static str {
        ROLE_TOKENS
            .iter()
            .find(|(r, _)| *r == self)
            .map_or("user", |(_, t)| *t)
    }

    /// The role of a word of the settings file.
    pub fn from_token(token: &str) -> Option<LayerRole> {
        ROLE_TOKENS
            .iter()
            .find(|(_, t)| *t == token)
            .map(|(r, _)| *r)
    }

    /// True for the roles that the export of this tool writes as a layer of its own: the copper,
    /// the outline, the silkscreen, the mask, the paste and the drills.
    pub fn is_exported(self) -> bool {
        matches!(
            self,
            LayerRole::Copper
                | LayerRole::Pads
                | LayerRole::Vias
                | LayerRole::Outline
                | LayerRole::SilkTop
                | LayerRole::SilkBottom
                | LayerRole::MaskTop
                | LayerRole::MaskBottom
                | LayerRole::PasteTop
                | LayerRole::PasteBottom
                | LayerRole::Drill
                | LayerRole::Hole
        )
    }
}

/// The standard layers from 17 on, with the names EAGLE gives them.
const STANDARD: &[(u16, &str, LayerRole)] = &[
    (17, "Pads", LayerRole::Pads),
    (18, "Vias", LayerRole::Vias),
    (19, "Unrouted", LayerRole::Ratsnest),
    (20, "Dimension", LayerRole::Outline),
    (21, "tPlace", LayerRole::SilkTop),
    (22, "bPlace", LayerRole::SilkBottom),
    (23, "tOrigins", LayerRole::OriginsTop),
    (24, "bOrigins", LayerRole::OriginsBottom),
    (25, "tNames", LayerRole::NamesTop),
    (26, "bNames", LayerRole::NamesBottom),
    (27, "tValues", LayerRole::ValuesTop),
    (28, "bValues", LayerRole::ValuesBottom),
    (29, "tStop", LayerRole::MaskTop),
    (30, "bStop", LayerRole::MaskBottom),
    (31, "tCream", LayerRole::PasteTop),
    (32, "bCream", LayerRole::PasteBottom),
    (33, "tFinish", LayerRole::FinishTop),
    (34, "bFinish", LayerRole::FinishBottom),
    (35, "tGlue", LayerRole::GlueTop),
    (36, "bGlue", LayerRole::GlueBottom),
    (37, "tTest", LayerRole::TestTop),
    (38, "bTest", LayerRole::TestBottom),
    (39, "tKeepout", LayerRole::KeepoutTop),
    (40, "bKeepout", LayerRole::KeepoutBottom),
    (41, "tRestrict", LayerRole::RestrictTop),
    (42, "bRestrict", LayerRole::RestrictBottom),
    (43, "vRestrict", LayerRole::RestrictVia),
    (44, "Drills", LayerRole::Drill),
    (45, "Holes", LayerRole::Hole),
    (46, "Milling", LayerRole::Milling),
    (47, "Measures", LayerRole::Measures),
    (48, "Document", LayerRole::Document),
    (49, "Reference", LayerRole::Reference),
    (50, "dxf", LayerRole::DxfImport),
    (51, "tDocu", LayerRole::DocuTop),
    (52, "bDocu", LayerRole::DocuBottom),
    (53, "tGND_GNDA", LayerRole::GroundTop),
    (54, "bGND_GNDA", LayerRole::GroundBottom),
    (91, "Nets", LayerRole::SchematicNets),
    (92, "Busses", LayerRole::SchematicBusses),
    (93, "Pins", LayerRole::SchematicPins),
    (94, "Symbols", LayerRole::SchematicSymbols),
    (95, "Names", LayerRole::SchematicNames),
    (96, "Values", LayerRole::SchematicValues),
    (97, "Info", LayerRole::SchematicInfo),
    (98, "Guide", LayerRole::SchematicGuide),
    (99, "SpiceOrder", LayerRole::SpiceOrder),
];

/// The first number of the user layers.
pub const FIRST_USER_LAYER: u16 = 100;
/// The last layer number.
pub const LAST_LAYER: u16 = 255;

/// The standard name and role of a layer number; `None` for a number that has no standard layer
/// (the numbers 55 to 90 and the user layers).
pub fn standard_layer(number: u16) -> Option<(String, LayerRole)> {
    match number {
        1 => Some(("Top".to_owned(), LayerRole::Copper)),
        2..=15 => Some((format!("Route{number}"), LayerRole::Copper)),
        16 => Some(("Bottom".to_owned(), LayerRole::Copper)),
        _ => STANDARD
            .iter()
            .find(|(n, _, _)| *n == number)
            .map(|(_, name, role)| ((*name).to_owned(), *role)),
    }
}

/// The whole table of the standard layers in order of number.
pub fn standard_table() -> Vec<(u16, String, LayerRole)> {
    (1..FIRST_USER_LAYER)
        .filter_map(|n| standard_layer(n).map(|(name, role)| (n, name, role)))
        .collect()
}

/// The roles that the settings give to user layers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerMap {
    /// By layer number (100 to 255).
    pub by_number: BTreeMap<u16, LayerRole>,
    /// By the name the layer has in the file.
    pub by_name: BTreeMap<String, LayerRole>,
}

impl LayerMap {
    /// Reads the settings text. An unknown key, a number outside 100 to 255, an unknown role or a
    /// repeated key is refused with its line number.
    pub fn parse(text: &str) -> Result<LayerMap, DeepError> {
        if text.len() > MAX_MAP_BYTES {
            return Err(DeepError::new(
                LAYER_MAP_INVALID,
                format!("the file is larger than {MAX_MAP_BYTES} bytes"),
            ));
        }
        let mut map = LayerMap::default();
        for (index, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let bad =
                |why: &str| DeepError::new(LAYER_MAP_INVALID, format!("line {}: {why}", index + 1));
            let Some((key, value)) = line.split_once('=') else {
                return Err(bad("there is no '='"));
            };
            let (key, value) = (key.trim(), value.trim());
            let role = LayerRole::from_token(value).ok_or_else(|| bad("the role is unknown"))?;
            if let Some(number) = key.strip_prefix("layer.") {
                let number: u16 = number
                    .parse()
                    .ok()
                    .filter(|n| (FIRST_USER_LAYER..=LAST_LAYER).contains(n))
                    .ok_or_else(|| bad("the layer number is not from 100 to 255"))?;
                if map.by_number.insert(number, role).is_some() {
                    return Err(bad("the layer number is repeated"));
                }
            } else if let Some(name) = key.strip_prefix("name.") {
                if name.is_empty() || name.len() > 64 {
                    return Err(bad("the layer name is empty or too long"));
                }
                if map.by_name.insert(name.to_owned(), role).is_some() {
                    return Err(bad("the layer name is repeated"));
                }
            } else {
                return Err(bad("the key is not layer.<number> or name.<Name>"));
            }
        }
        Ok(map)
    }
}

/// Where the role of a layer comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleSource {
    /// The standard table.
    Standard,
    /// The settings, by number.
    MappedByNumber,
    /// The settings, by name.
    MappedByName,
    /// Nothing gives a role: it is a user layer that is only kept.
    Unmapped,
}

/// The role of a layer by its number and the name it has in the file.
pub fn resolve(map: &LayerMap, number: u16, name: &str) -> (LayerRole, RoleSource) {
    if let Some((_, role)) = standard_layer(number) {
        return (role, RoleSource::Standard);
    }
    if let Some(role) = map.by_number.get(&number) {
        return (*role, RoleSource::MappedByNumber);
    }
    if let Some(role) = map.by_name.get(name) {
        return (*role, RoleSource::MappedByName);
    }
    (LayerRole::User, RoleSource::Unmapped)
}

/// A layer of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileLayer {
    /// The number.
    pub number: u16,
    /// The name in the file.
    pub name: String,
    /// What it is for.
    pub role: LayerRole,
    /// Where the role comes from.
    pub source: RoleSource,
    /// The colour index of the file, when it has one.
    pub color: Option<String>,
    /// True when the file says the layer is visible (or does not say).
    pub visible: bool,
}

/// The layers of a file and what is odd in them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLayers {
    /// The layers in the order of the file.
    pub layers: Vec<FileLayer>,
    /// Numbers out of range or repeated, standard layers with another name, user layers without a
    /// role.
    pub findings: Vec<Finding>,
}

/// Reads the `<layers>` of the `<drawing>` of a parsed file.
pub fn read_layers(root: &Element, map: &LayerMap) -> FileLayers {
    let mut layers = Vec::new();
    let mut findings = Vec::new();
    let mut seen: BTreeSet<u16> = BTreeSet::new();
    let elements: Vec<&Element> = root
        .child("drawing")
        .and_then(|d| d.child("layers"))
        .map(|x| x.children_named("layer").collect())
        .unwrap_or_default();
    for l in elements {
        let raw = l.attr("number").unwrap_or("");
        let name = l.attr("name").unwrap_or("").to_owned();
        let Some(number) = raw
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|n| (1..=LAST_LAYER).contains(n))
        else {
            findings.push(Finding::new(
                LAYER_NUMBER_INVALID,
                Severity::Warning,
                Some(&name),
                format!("the number {raw:?} is not from 1 to {LAST_LAYER}"),
            ));
            continue;
        };
        if !seen.insert(number) {
            findings.push(Finding::new(
                LAYER_NUMBER_INVALID,
                Severity::Warning,
                Some(&name),
                format!("the number {number} is used by more than one layer; the first is kept"),
            ));
            continue;
        }
        let (role, source) = resolve(map, number, &name);
        match source {
            RoleSource::Standard => {
                if let Some((standard, _)) = standard_layer(number) {
                    if standard != name {
                        findings.push(Finding::new(
                            LAYER_RENAMED,
                            Severity::Info,
                            Some(&name),
                            format!("layer {number} is {standard} in the standard table"),
                        ));
                    }
                }
            }
            RoleSource::Unmapped => findings.push(Finding::new(
                LAYER_UNMAPPED,
                Severity::Warning,
                Some(&name),
                format!("layer {number} is kept in the file and not exported"),
            )),
            RoleSource::MappedByNumber | RoleSource::MappedByName => {}
        }
        layers.push(FileLayer {
            number,
            name,
            role,
            source,
            color: l.attr("color").map(str::to_owned),
            visible: l.attr("visible").is_none_or(|v| v != "no"),
        });
    }
    FileLayers { layers, findings }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_eagle::xml::{parse, Limits};

    #[test]
    fn the_table_has_the_standard_layers_in_order_with_unique_names() {
        let table = standard_table();
        assert_eq!(table.len(), 16 + 38 + 9);
        assert_eq!(table[0], (1, "Top".to_owned(), LayerRole::Copper));
        assert_eq!(table[15], (16, "Bottom".to_owned(), LayerRole::Copper));
        let mut names = BTreeSet::new();
        for (i, (number, name, _)) in table.iter().enumerate() {
            assert!(names.insert(name.clone()), "{name}");
            if i > 0 {
                assert!(*number > table[i - 1].0);
            }
        }
        assert_eq!(standard_layer(20).unwrap().1, LayerRole::Outline);
        assert_eq!(standard_layer(7).unwrap().0, "Route7");
        assert_eq!(standard_layer(60), None);
        assert_eq!(standard_layer(100), None);
    }

    #[test]
    fn every_role_has_one_word_and_reads_back() {
        let mut seen = BTreeSet::new();
        for (role, token) in ROLE_TOKENS {
            assert!(seen.insert(*token), "{token}");
            assert_eq!(role.token(), *token);
            assert_eq!(LayerRole::from_token(token), Some(*role));
        }
        assert_eq!(LayerRole::from_token("nothing"), None);
        for (_, _, role) in standard_table() {
            assert!(ROLE_TOKENS.iter().any(|(r, _)| *r == role));
        }
        assert!(LayerRole::SilkTop.is_exported());
        assert!(!LayerRole::NamesTop.is_exported());
    }

    #[test]
    fn the_settings_are_read_and_the_wrong_lines_refused() {
        let map = LayerMap::parse(
            "# user layers\nlayer.100 = silk_top\n\nname.Milling2 = milling\nlayer.255 = user\n",
        )
        .unwrap();
        assert_eq!(map.by_number.get(&100), Some(&LayerRole::SilkTop));
        assert_eq!(map.by_name.get("Milling2"), Some(&LayerRole::Milling));
        assert_eq!(map.by_number.len(), 2);
        for text in [
            "layer.99 = user",
            "layer.256 = user",
            "layer.x = user",
            "layer.100 = nothing",
            "layer.100 user",
            "other = user",
            "layer.100 = user\nlayer.100 = user",
            "name. = user",
            "name.A = user\nname.A = user",
        ] {
            let error = LayerMap::parse(text).unwrap_err();
            assert_eq!(error.code, LAYER_MAP_INVALID, "{text:?}");
        }
        let big = "#".repeat(MAX_MAP_BYTES + 1);
        assert!(LayerMap::parse(&big).is_err());
    }

    #[test]
    fn the_role_comes_from_the_table_then_the_number_then_the_name() {
        let map = LayerMap::parse("layer.100 = silk_top\nname.Extra = hole\n").unwrap();
        assert_eq!(
            resolve(&map, 21, "x"),
            (LayerRole::SilkTop, RoleSource::Standard)
        );
        assert_eq!(
            resolve(&map, 100, "x"),
            (LayerRole::SilkTop, RoleSource::MappedByNumber)
        );
        assert_eq!(
            resolve(&map, 101, "Extra"),
            (LayerRole::Hole, RoleSource::MappedByName)
        );
        assert_eq!(
            resolve(&map, 102, "Other"),
            (LayerRole::User, RoleSource::Unmapped)
        );
    }

    const FILE: &str = r#"<eagle version="9.6.2"><drawing><layers>
        <layer number="1" name="Top" color="4" fill="1" visible="yes" active="yes"/>
        <layer number="25" name="Names" color="7" fill="1" visible="no" active="yes"/>
        <layer number="100" name="Mine" color="2" fill="1" visible="yes" active="yes"/>
        <layer number="101" name="Free" color="3" fill="1" visible="yes" active="yes"/>
        <layer number="300" name="Far" color="3" fill="1" visible="yes" active="yes"/>
        <layer number="1" name="Again" color="3" fill="1" visible="yes" active="yes"/>
        <layer number="x" name="Bad" color="3" fill="1" visible="yes" active="yes"/>
        </layers></drawing></eagle>"#;

    #[test]
    fn the_layers_of_a_file_are_read_with_a_finding_for_each_oddity() {
        let root = parse(FILE, &Limits::default()).unwrap();
        let map = LayerMap::parse("layer.100 = outline\n").unwrap();
        let read = read_layers(&root, &map);
        let numbers: Vec<u16> = read.layers.iter().map(|l| l.number).collect();
        assert_eq!(numbers, vec![1, 25, 100, 101]);
        assert_eq!(read.layers[2].role, LayerRole::Outline);
        assert_eq!(read.layers[2].source, RoleSource::MappedByNumber);
        assert_eq!(read.layers[3].source, RoleSource::Unmapped);
        assert!(!read.layers[1].visible);
        assert!(read.layers[0].visible);
        let codes: Vec<&str> = read.findings.iter().map(|f| f.code).collect();
        assert_eq!(
            codes,
            vec![
                LAYER_RENAMED,
                LAYER_UNMAPPED,
                LAYER_NUMBER_INVALID,
                LAYER_NUMBER_INVALID,
                LAYER_NUMBER_INVALID
            ]
        );
        assert_eq!(
            read.findings[0].detail,
            "layer 25 is tNames in the standard table"
        );
    }

    #[test]
    fn a_file_without_layers_gives_none() {
        let root = parse("<eagle><drawing/></eagle>", &Limits::default()).unwrap();
        let read = read_layers(&root, &LayerMap::default());
        assert!(read.layers.is_empty() && read.findings.is_empty());
    }

    #[test]
    fn the_example_settings_of_the_repository_are_valid() {
        let text = include_str!("../../../config/eagle_layers.cfg");
        let map = LayerMap::parse(text).unwrap();
        assert!(map.by_number.is_empty() || map.by_number.keys().all(|n| *n >= 100));
    }
}
