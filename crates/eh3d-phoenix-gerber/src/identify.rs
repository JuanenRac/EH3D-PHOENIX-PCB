// SPDX-License-Identifier: GPL-3.0-or-later
//! The layer a file belongs to, told from its X2 attributes (`.FileFunction`) when it has them and
//! from its name otherwise: the extensions of EAGLE and Protel, and the names KiCad gives.

use std::collections::BTreeMap;

/// What a file is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The copper of the top side.
    CopperTop,
    /// The copper of the bottom side.
    CopperBottom,
    /// An inner copper layer, counted from 1 below the top layer.
    CopperInner(u32),
    /// The solder mask of the top side.
    MaskTop,
    /// The solder mask of the bottom side.
    MaskBottom,
    /// The solder paste of the top side.
    PasteTop,
    /// The solder paste of the bottom side.
    PasteBottom,
    /// The legend (silkscreen) of the top side.
    LegendTop,
    /// The legend (silkscreen) of the bottom side.
    LegendBottom,
    /// The board outline.
    Outline,
    /// The plated holes.
    DrillPlated,
    /// The holes that are not plated.
    DrillNonPlated,
    /// Holes, plated or not (not told).
    Drill,
}

impl Role {
    /// A short name for a summary.
    pub fn name(self) -> String {
        match self {
            Role::CopperTop => "copper top".to_owned(),
            Role::CopperBottom => "copper bottom".to_owned(),
            Role::CopperInner(n) => format!("copper inner {n}"),
            Role::MaskTop => "solder mask top".to_owned(),
            Role::MaskBottom => "solder mask bottom".to_owned(),
            Role::PasteTop => "paste top".to_owned(),
            Role::PasteBottom => "paste bottom".to_owned(),
            Role::LegendTop => "legend top".to_owned(),
            Role::LegendBottom => "legend bottom".to_owned(),
            Role::Outline => "outline".to_owned(),
            Role::DrillPlated => "drill plated".to_owned(),
            Role::DrillNonPlated => "drill not plated".to_owned(),
            Role::Drill => "drill".to_owned(),
        }
    }

    /// True for the roles of a drill file.
    pub fn is_drill(self) -> bool {
        matches!(self, Role::DrillPlated | Role::DrillNonPlated | Role::Drill)
    }
}

/// The two layers that a drill file of blind or buried vias joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrillSpan {
    /// The upper layer, counted from 1 at the top of the stack.
    pub from: u32,
    /// The lower layer.
    pub to: u32,
    /// True for buried vias, false for blind ones.
    pub buried: bool,
}

/// The span of a drill file told by its `.FileFunction` attribute (`Plated,1,2,Blind` or
/// `Plated,2,3,Buried`); `None` for any other drill file, a file of through holes included.
pub fn drill_span(attributes: &BTreeMap<String, Vec<String>>) -> Option<DrillSpan> {
    let values = attributes.get(".FileFunction")?;
    if !values.first()?.eq_ignore_ascii_case("plated") {
        return None;
    }
    let from = values.get(1)?.trim().parse::<u32>().ok()?;
    let to = values.get(2)?.trim().parse::<u32>().ok()?;
    match values.get(3)?.to_ascii_lowercase().as_str() {
        "blind" => Some(DrillSpan {
            from: from.min(to),
            to: from.max(to),
            buried: false,
        }),
        "buried" => Some(DrillSpan {
            from: from.min(to),
            to: from.max(to),
            buried: true,
        }),
        _ => None,
    }
}

/// Where the role was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The `.FileFunction` attribute of the file.
    Attribute,
    /// The name of the file.
    FileName,
}

/// A role and where it was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identified {
    /// The role.
    pub role: Role,
    /// Where it was found.
    pub source: Source,
}

/// What kind of content a file has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A Gerber file.
    Gerber,
    /// An Excellon drill file.
    Excellon,
}

/// Tells Gerber from Excellon by the first part of the text; `None` when it is neither.
pub fn sniff(text: &str) -> Option<Kind> {
    let mut end = text.len().min(4096);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let head = &text[..end];
    if head
        .lines()
        .any(|l| l.trim() == "M48" || l.trim_start().starts_with("M48,"))
    {
        return Some(Kind::Excellon);
    }
    if head.contains("%FS") || head.contains("%MO") || head.contains("%TF") || head.contains("G04")
    {
        return Some(Kind::Gerber);
    }
    None
}

fn side_of(word: &str) -> Option<bool> {
    match word.to_ascii_lowercase().as_str() {
        "top" => Some(true),
        "bot" | "bottom" => Some(false),
        _ => None,
    }
}

/// The role told by the `.FileFunction` attribute, when the attributes have one.
pub fn from_attributes(attributes: &BTreeMap<String, Vec<String>>) -> Option<Role> {
    let values = attributes.get(".FileFunction")?;
    let function = values.first()?.to_ascii_lowercase();
    let on = |index: usize| values.get(index).and_then(|v| side_of(v));
    match function.as_str() {
        "copper" => {
            let layer = values
                .get(1)
                .and_then(|v| v.trim_start_matches(['L', 'l']).parse::<u32>().ok());
            match (on(2), values.get(2).map(|v| v.to_ascii_lowercase())) {
                (Some(true), _) => Some(Role::CopperTop),
                (Some(false), _) => Some(Role::CopperBottom),
                (None, Some(position)) if position == "inr" => Some(Role::CopperInner(
                    layer.unwrap_or(2).saturating_sub(1).max(1),
                )),
                _ => None,
            }
        }
        "soldermask" => on(1).map(|top| if top { Role::MaskTop } else { Role::MaskBottom }),
        "paste" => on(1).map(|top| {
            if top {
                Role::PasteTop
            } else {
                Role::PasteBottom
            }
        }),
        "legend" => on(1).map(|top| {
            if top {
                Role::LegendTop
            } else {
                Role::LegendBottom
            }
        }),
        "profile" => Some(Role::Outline),
        "plated" => Some(Role::DrillPlated),
        "nonplated" => Some(Role::DrillNonPlated),
        _ => None,
    }
}

/// The role told by the name of a file (the extension of EAGLE and Protel, the suffix of KiCad).
pub fn from_name(file_name: &str) -> Option<Role> {
    let lower = file_name.to_ascii_lowercase();
    let base = lower.rsplit(['/', '\\']).next().unwrap_or("");
    let (stem, extension) = match base.rsplit_once('.') {
        Some((s, e)) => (s, e),
        None => (base, ""),
    };
    // KiCad: board-F_Cu.gbr, board-In1_Cu.gbr, board-Edge_Cuts.gbr, board-PTH.drl
    let normal = stem.replace(['.', ' ', '-'], "_");
    for (suffix, role) in [
        ("f_cu", Role::CopperTop),
        ("b_cu", Role::CopperBottom),
        ("f_mask", Role::MaskTop),
        ("b_mask", Role::MaskBottom),
        ("f_paste", Role::PasteTop),
        ("b_paste", Role::PasteBottom),
        ("f_silkscreen", Role::LegendTop),
        ("f_silks", Role::LegendTop),
        ("b_silkscreen", Role::LegendBottom),
        ("b_silks", Role::LegendBottom),
        ("edge_cuts", Role::Outline),
        ("npth", Role::DrillNonPlated),
        ("pth", Role::DrillPlated),
    ] {
        if normal == suffix || normal.ends_with(&format!("_{suffix}")) {
            return Some(role);
        }
    }
    if let Some(position) = normal.rfind("_in") {
        let tail = &normal[position + 3..];
        if let Some(number) = tail.strip_suffix("_cu").and_then(|n| n.parse::<u32>().ok()) {
            return Some(Role::CopperInner(number));
        }
    }
    match extension {
        // Protel
        "gtl" => Some(Role::CopperTop),
        "gbl" => Some(Role::CopperBottom),
        "gts" => Some(Role::MaskTop),
        "gbs" => Some(Role::MaskBottom),
        "gto" => Some(Role::LegendTop),
        "gbo" => Some(Role::LegendBottom),
        "gtp" => Some(Role::PasteTop),
        "gbp" => Some(Role::PasteBottom),
        "gko" | "gm1" | "gml" => Some(Role::Outline),
        // EAGLE
        "cmp" => Some(Role::CopperTop),
        "sol" => Some(Role::CopperBottom),
        "stc" => Some(Role::MaskTop),
        "sts" => Some(Role::MaskBottom),
        "plc" => Some(Role::LegendTop),
        "pls" => Some(Role::LegendBottom),
        "crc" => Some(Role::PasteTop),
        "crs" => Some(Role::PasteBottom),
        "mil" | "oln" | "dim" => Some(Role::Outline),
        "drd" | "drl" | "xln" | "exc" | "ncd" => Some(Role::Drill),
        other => inner_extension(other),
    }
}

/// `.g1` to `.g30` (Protel) and `.ly2` to `.ly30` (EAGLE) are inner copper layers.
fn inner_extension(extension: &str) -> Option<Role> {
    let (first, text) = match extension.strip_prefix("ly") {
        Some(text) => (2, text),
        None => (1, extension.strip_prefix('g')?),
    };
    let number = text.parse::<u32>().ok()?;
    if (first..=30).contains(&number) {
        Some(Role::CopperInner(number))
    } else {
        None
    }
}

/// The role of a file from its attributes first and its name second.
pub fn identify(file_name: &str, attributes: &BTreeMap<String, Vec<String>>) -> Option<Identified> {
    if let Some(role) = from_attributes(attributes) {
        return Some(Identified {
            role,
            source: Source::Attribute,
        });
    }
    from_name(file_name).map(|role| Identified {
        role,
        source: Source::FileName,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes(values: &[&str]) -> BTreeMap<String, Vec<String>> {
        let mut map = BTreeMap::new();
        map.insert(
            ".FileFunction".to_owned(),
            values.iter().map(|v| (*v).to_owned()).collect(),
        );
        map
    }

    #[test]
    fn the_attribute_tells_the_role() {
        for (values, role) in [
            (vec!["Copper", "L1", "Top"], Role::CopperTop),
            (vec!["Copper", "L4", "Bot"], Role::CopperBottom),
            (vec!["Copper", "L2", "Inr"], Role::CopperInner(1)),
            (vec!["Copper", "L3", "Inr"], Role::CopperInner(2)),
            (vec!["Soldermask", "Top"], Role::MaskTop),
            (vec!["Soldermask", "Bot"], Role::MaskBottom),
            (vec!["Paste", "Top"], Role::PasteTop),
            (vec!["Legend", "Bot"], Role::LegendBottom),
            (vec!["Profile", "NP"], Role::Outline),
            (vec!["Plated", "1", "2", "PTH"], Role::DrillPlated),
            (vec!["NonPlated", "1", "2", "NPTH"], Role::DrillNonPlated),
            (vec!["Plated", "1", "2", "Blind"], Role::DrillPlated),
            (vec!["Plated", "2", "3", "Buried"], Role::DrillPlated),
        ] {
            assert_eq!(
                from_attributes(&attributes(&values)),
                Some(role),
                "{values:?}"
            );
        }
        assert_eq!(from_attributes(&attributes(&["Other", "x"])), None);
        assert_eq!(from_attributes(&attributes(&["Soldermask"])), None);
        assert_eq!(from_attributes(&BTreeMap::new()), None);
    }

    #[test]
    fn the_names_of_the_cad_programs_tell_the_role() {
        for (name, role) in [
            ("board.GTL", Role::CopperTop),
            ("board.gbl", Role::CopperBottom),
            ("board.GTS", Role::MaskTop),
            ("board.gbs", Role::MaskBottom),
            ("board.gto", Role::LegendTop),
            ("board.gbo", Role::LegendBottom),
            ("board.gtp", Role::PasteTop),
            ("board.gbp", Role::PasteBottom),
            ("board.gko", Role::Outline),
            ("board.g2", Role::CopperInner(2)),
            ("board.cmp", Role::CopperTop),
            ("board.sol", Role::CopperBottom),
            ("board.stc", Role::MaskTop),
            ("board.sts", Role::MaskBottom),
            ("board.plc", Role::LegendTop),
            ("board.pls", Role::LegendBottom),
            ("board.crc", Role::PasteTop),
            ("board.ly3", Role::CopperInner(3)),
            ("board.drd", Role::Drill),
            ("board.DRL", Role::Drill),
            ("proj-F_Cu.gbr", Role::CopperTop),
            ("proj-B_Cu.gbr", Role::CopperBottom),
            ("proj-In2_Cu.gbr", Role::CopperInner(2)),
            ("proj-F.Cu.gbr", Role::CopperTop),
            ("proj-F_Mask.gbr", Role::MaskTop),
            ("proj-B_Paste.gbr", Role::PasteBottom),
            ("proj-F_Silkscreen.gbr", Role::LegendTop),
            ("proj-B_SilkS.gbr", Role::LegendBottom),
            ("proj-Edge_Cuts.gbr", Role::Outline),
            ("proj-PTH.drl", Role::DrillPlated),
            ("proj-NPTH.drl", Role::DrillNonPlated),
            ("out/dir/proj-F_Cu.gbr", Role::CopperTop),
        ] {
            assert_eq!(from_name(name), Some(role), "{name}");
        }
        for name in [
            "notes.txt",
            "board.gbr",
            "board",
            "board.g99",
            "board.ly1",
            "x.g0",
        ] {
            assert_eq!(from_name(name), None, "{name}");
        }
    }

    #[test]
    fn the_attribute_wins_over_the_name() {
        let attrs = attributes(&["Copper", "L1", "Top"]);
        let found = identify("board.gbl", &attrs).unwrap();
        assert_eq!(
            (found.role, found.source),
            (Role::CopperTop, Source::Attribute)
        );
        let by_name = identify("board.gbl", &BTreeMap::new()).unwrap();
        assert_eq!(
            (by_name.role, by_name.source),
            (Role::CopperBottom, Source::FileName)
        );
        assert!(identify("mystery.bin", &BTreeMap::new()).is_none());
    }

    #[test]
    fn the_kind_of_content_is_sniffed() {
        assert_eq!(sniff("M48\nMETRIC\n"), Some(Kind::Excellon));
        assert_eq!(sniff(";comment\nM48\nINCH\n"), Some(Kind::Excellon));
        assert_eq!(sniff("%FSLAX46Y46*%\n"), Some(Kind::Gerber));
        assert_eq!(sniff("G04 hello*\nD10*\n"), Some(Kind::Gerber));
        assert_eq!(sniff("just text"), None);
        // a cut in the middle of a character does not panic
        let long = "é".repeat(5000);
        assert_eq!(sniff(&long), None);
    }

    #[test]
    fn roles_have_names() {
        assert_eq!(Role::CopperInner(2).name(), "copper inner 2");
        assert!(Role::Drill.is_drill() && !Role::Outline.is_drill());
        assert_eq!(Role::MaskTop.name(), "solder mask top");
    }
}
