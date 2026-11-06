// SPDX-License-Identifier: GPL-3.0-or-later
//! The families of footprints: the dimensions of the bodies and of the leads of each family, with
//! their tolerances, the fillet goals of each density level of IPC-7351B and the way the names are
//! made, read from data files, one family per file (`config/ipc_families/*.json`).
//!
//! Every family says whether it has been verified against datasheets. The files that come with
//! the program all say that they have not: their dimensions were written by hand from the usual
//! outline drawings of each package.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::codes::{IpcError, FAMILY_INVALID, FAMILY_UNKNOWN, FILE_UNUSABLE, VARIANT_UNKNOWN};
use crate::settings::IpcSettings;

/// The format of a family file.
pub const FORMAT: &str = "phoenix-ipc-family/1";

/// The layouts a family can have: how the pads are placed.
pub const LAYOUTS: [&str; 8] = [
    "two_terminal",
    "dual",
    "sot",
    "quad",
    "grid",
    "tht_dual",
    "tht_row",
    "explicit",
];

/// The ways a name is made from the dimensions.
pub const NAMINGS: [&str; 9] = [
    "chip",
    "dual_span",
    "dual_body",
    "quad_span",
    "quad_body",
    "bga",
    "dip",
    "row",
    "explicit",
];

/// The names of the goals a variant may change for a density level.
const GOAL_NAMES: [&str; 7] = [
    "toe",
    "heel",
    "side",
    "courtyard",
    "pad_ratio",
    "drill_extra",
    "annular",
];

/// The families that come with the program, as the text of their files.
const BUILTIN: [(&str, &str); 15] = [
    ("bga", include_str!("../../../config/ipc_families/bga.json")),
    (
        "chip",
        include_str!("../../../config/ipc_families/chip.json"),
    ),
    (
        "connector",
        include_str!("../../../config/ipc_families/connector.json"),
    ),
    ("dfn", include_str!("../../../config/ipc_families/dfn.json")),
    ("dip", include_str!("../../../config/ipc_families/dip.json")),
    (
        "header",
        include_str!("../../../config/ipc_families/header.json"),
    ),
    (
        "melf",
        include_str!("../../../config/ipc_families/melf.json"),
    ),
    ("qfn", include_str!("../../../config/ipc_families/qfn.json")),
    ("qfp", include_str!("../../../config/ipc_families/qfp.json")),
    ("sod", include_str!("../../../config/ipc_families/sod.json")),
    (
        "soic",
        include_str!("../../../config/ipc_families/soic.json"),
    ),
    ("sot", include_str!("../../../config/ipc_families/sot.json")),
    (
        "sot89",
        include_str!("../../../config/ipc_families/sot89.json"),
    ),
    (
        "ssop",
        include_str!("../../../config/ipc_families/ssop.json"),
    ),
    (
        "tssop",
        include_str!("../../../config/ipc_families/tssop.json"),
    ),
];

/// A density level of IPC-7351B: A is the most copper (level M), B the nominal (N) and C the
/// least (L).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Density {
    /// Most: for the boards that are soldered by hand or by wave.
    A,
    /// Nominal.
    B,
    /// Least: for the densest boards.
    C,
}

impl Density {
    /// The three levels, in order.
    pub const ALL: [Density; 3] = [Density::A, Density::B, Density::C];

    /// The key of the level in the family files.
    pub fn key(self) -> &'static str {
        match self {
            Density::A => "A",
            Density::B => "B",
            Density::C => "C",
        }
    }

    /// The letter that ends the name of a footprint.
    pub fn letter(self) -> char {
        match self {
            Density::A => 'M',
            Density::B => 'N',
            Density::C => 'L',
        }
    }

    /// The level named by a letter, A, B or C or else M, N or L, in either case.
    pub fn parse(text: &str) -> Option<Density> {
        match text.trim().to_ascii_uppercase().as_str() {
            "A" | "M" => Some(Density::A),
            "B" | "N" => Some(Density::B),
            "C" | "L" => Some(Density::C),
            _ => None,
        }
    }
}

/// The tolerances of the fabrication and of the placement, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tolerances {
    /// The tolerance of the fabrication of the board.
    pub fab: f64,
    /// The tolerance of the placement of the component.
    pub place: f64,
}

/// The goals of a density level, in millimetres; a goal that a layout does not use is zero.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Goals {
    /// The fillet at the toe of the lead.
    pub toe: f64,
    /// The fillet at the heel of the lead.
    pub heel: f64,
    /// The fillet at the side of the lead.
    pub side: f64,
    /// The excess of the courtyard over the pads and the body.
    pub courtyard: f64,
    /// The diameter of the pad of a ball grid over the diameter of the ball.
    pub pad_ratio: f64,
    /// What is added to the diagonal of a lead to make the hole.
    pub drill_extra: f64,
    /// The width of the ring of copper around the hole.
    pub annular: f64,
}

/// The values of the drawing that every layout shares, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Drawing {
    /// The width of the silkscreen.
    pub silk_width: f64,
    /// The distance from the silkscreen to the copper.
    pub silk_gap: f64,
    /// The length of the corner marks.
    pub bracket: f64,
    /// The radius of the dot that marks the first pin.
    pub dot_r: f64,
    /// The grid to which the lands are rounded.
    pub grid: f64,
    /// The percentage of paste on an exposed pad.
    pub ep_paste: u32,
    /// The step to which a hole is rounded up.
    pub drill_step: f64,
    /// The least copper between two pads of a through-hole family.
    pub pad_gap: f64,
    /// The least ring of copper around a hole.
    pub min_ring: f64,
}

impl Default for Drawing {
    fn default() -> Self {
        Drawing {
            silk_width: 0.15,
            silk_gap: 0.20,
            bracket: 1.00,
            dot_r: 0.25,
            grid: 0.01,
            ep_paste: 60,
            drill_step: 0.05,
            pad_gap: 0.25,
            min_ring: 0.10,
        }
    }
}

/// A pad written out, in millimetres with y up.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExplicitPad {
    /// The name.
    pub name: String,
    /// The centre.
    pub x: f64,
    /// The centre.
    pub y: f64,
    /// The size along x.
    pub w: f64,
    /// The size along y.
    pub h: f64,
}

/// A variant of a family: a size or a package.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Variant {
    /// The code that goes in the name (`1608` for a chip of 0603), when the layout needs one.
    pub code: String,
    /// The dimensions, in millimetres unless the name says otherwise.
    pub values: BTreeMap<String, f64>,
    /// The places of the pads of the first side, in half pitches along the axis of the side.
    pub slots_a: Vec<i32>,
    /// The places of the pads of the second side.
    pub slots_b: Vec<i32>,
    /// The pads written out, for the layout `explicit`.
    pub pads: Vec<ExplicitPad>,
    /// The goals that this variant changes, by density level.
    pub density: BTreeMap<String, BTreeMap<String, f64>>,
    /// A note about the variant.
    pub note: String,
}

impl Variant {
    /// A dimension, when the variant has it.
    pub fn get(&self, key: &str) -> Option<f64> {
        self.values.get(key).copied()
    }

    /// A dimension, or else a value.
    pub fn get_or(&self, key: &str, default: f64) -> f64 {
        self.get(key).unwrap_or(default)
    }

    /// A dimension that the layout needs.
    pub fn need(&self, key: &str) -> Result<f64, IpcError> {
        self.get(key)
            .ok_or_else(|| IpcError::new(FAMILY_INVALID, format!("the variant has no value {key}")))
    }
}

/// A family of footprints.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    /// The format, `phoenix-ipc-family/1`.
    pub format: String,
    /// The name of the family, in lower case.
    pub family: String,
    /// What the family is.
    pub title: String,
    /// The standard whose method it follows.
    pub standard: String,
    /// True when the dimensions have been checked against datasheets.
    pub verified: bool,
    /// What is known about the dimensions.
    pub status: String,
    /// How the pads are placed, one of [`LAYOUTS`].
    pub layout: String,
    /// How the names are made, one of [`NAMINGS`].
    pub naming: String,
    /// The prefixes of the names, the first being the one that is used when none is asked for.
    pub prefixes: Vec<String>,
    /// The tolerances of the method.
    pub tolerances: Tolerances,
    /// The goals of each density level, `A`, `B` and `C`.
    pub density: BTreeMap<String, Goals>,
    /// The values of the drawing.
    #[serde(default)]
    pub drawing: Drawing,
    /// The variants, by name.
    pub variants: BTreeMap<String, Variant>,
}

/// The dimensions a layout needs in each variant.
fn required(layout: &str) -> &'static [&'static str] {
    match layout {
        "two_terminal" => &[
            "length",
            "length_tol",
            "width",
            "width_tol",
            "term",
            "term_tol",
            "height",
        ],
        "dual" => &[
            "pitch",
            "span",
            "span_tol",
            "lead",
            "lead_tol",
            "lead_w",
            "lead_w_tol",
            "body_w",
            "height",
            "pins_min",
            "pins_max",
        ],
        "sot" => &[
            "pitch",
            "span",
            "span_tol",
            "lead",
            "lead_tol",
            "lead_w",
            "lead_w_tol",
            "body_l",
            "body_w",
            "height",
        ],
        "quad" => &[
            "pins",
            "pitch",
            "body",
            "span",
            "span_tol",
            "lead",
            "lead_tol",
            "lead_w",
            "lead_w_tol",
            "height",
        ],
        "grid" => &[
            "pitch", "cols", "rows", "ball", "body_l", "body_w", "height",
        ],
        "tht_dual" => &[
            "pitch",
            "row",
            "lead_w",
            "lead_t",
            "body_w",
            "body_trim",
            "height",
            "pins_min",
            "pins_max",
        ],
        "tht_row" => &[
            "pitch", "drill", "pad_w", "pad_h", "body_w", "height", "pins_min", "pins_max",
            "rows_max",
        ],
        _ => &["pitch", "body_l", "body_w", "height"],
    }
}

fn bad(what: impl Into<String>) -> IpcError {
    IpcError::new(FAMILY_INVALID, what)
}

fn plain_name(text: &str, upper: bool) -> bool {
    !text.is_empty()
        && text.chars().all(|c| {
            c.is_ascii_digit()
                || c == '_'
                || if upper {
                    c.is_ascii_uppercase()
                } else {
                    c.is_ascii_lowercase()
                }
        })
}

impl Family {
    /// Reads a family from the text of its file and checks it.
    pub fn parse(text: &str) -> Result<Family, IpcError> {
        let family: Family = serde_json::from_str(text).map_err(|e| bad(e.to_string()))?;
        family.validate()?;
        Ok(family)
    }

    /// Checks the family: its format, its names, its numbers and what its layout needs.
    pub fn validate(&self) -> Result<(), IpcError> {
        let name = &self.family;
        if self.format != FORMAT {
            return Err(bad(format!("{name}: the format must be {FORMAT}")));
        }
        if !plain_name(name, false) {
            return Err(bad(format!("{name}: not a name in lower case")));
        }
        if !LAYOUTS.contains(&self.layout.as_str()) {
            return Err(bad(format!("{name}: unknown layout {}", self.layout)));
        }
        if !NAMINGS.contains(&self.naming.as_str()) {
            return Err(bad(format!("{name}: unknown naming {}", self.naming)));
        }
        if self.prefixes.is_empty() || !self.prefixes.iter().all(|p| plain_name(p, true)) {
            return Err(bad(format!("{name}: the prefixes are upper case words")));
        }
        for v in [self.tolerances.fab, self.tolerances.place] {
            if !v.is_finite() || v < 0.0 {
                return Err(bad(format!(
                    "{name}: a tolerance is not a number from zero"
                )));
            }
        }
        for level in Density::ALL {
            let Some(goals) = self.density.get(level.key()) else {
                return Err(bad(format!(
                    "{name}: no goals for the density {}",
                    level.key()
                )));
            };
            let all = [
                goals.toe,
                goals.heel,
                goals.side,
                goals.courtyard,
                goals.pad_ratio,
                goals.drill_extra,
                goals.annular,
            ];
            if all.iter().any(|v| !v.is_finite()) || goals.courtyard < 0.0 {
                return Err(bad(format!(
                    "{name}: bad goals for the density {}",
                    level.key()
                )));
            }
        }
        if self.density.len() != 3 {
            return Err(bad(format!("{name}: the densities are A, B and C")));
        }
        if self.variants.is_empty() {
            return Err(bad(format!("{name}: no variants")));
        }
        for (vname, variant) in &self.variants {
            self.validate_variant(vname, variant)?;
        }
        Ok(())
    }

    fn validate_variant(&self, vname: &str, variant: &Variant) -> Result<(), IpcError> {
        let at = format!("{}/{vname}", self.family);
        if !plain_name(vname, false) {
            return Err(bad(format!("{at}: not a name in lower case")));
        }
        for (key, value) in &variant.values {
            if !value.is_finite() || *value < 0.0 {
                return Err(bad(format!("{at}: {key} is not a number from zero")));
            }
        }
        for key in required(&self.layout) {
            let Some(value) = variant.get(key) else {
                return Err(bad(format!("{at}: no value {key}")));
            };
            if value <= 0.0 && !key.ends_with("_tol") && *key != "body_trim" {
                return Err(bad(format!("{at}: {key} must be above zero")));
            }
        }
        for (level, changes) in &variant.density {
            if Density::parse(level).is_none() || level.len() != 1 {
                return Err(bad(format!("{at}: the density {level} is not A, B or C")));
            }
            for (key, value) in changes {
                if !GOAL_NAMES.contains(&key.as_str()) || !value.is_finite() {
                    return Err(bad(format!("{at}: {key} is not a goal")));
                }
            }
        }
        if self.layout == "sot" && (variant.slots_a.is_empty() || variant.slots_b.is_empty()) {
            return Err(bad(format!("{at}: both sides need pads")));
        }
        if self.layout == "explicit" && variant.pads.is_empty() {
            return Err(bad(format!("{at}: no pads")));
        }
        if (self.naming == "explicit" || self.naming == "chip") && variant.code.is_empty() {
            return Err(bad(format!("{at}: no code for the name")));
        }
        for pad in &variant.pads {
            let all = [pad.x, pad.y, pad.w, pad.h];
            if pad.name.is_empty()
                || all.iter().any(|v| !v.is_finite())
                || pad.w <= 0.0
                || pad.h <= 0.0
            {
                return Err(bad(format!("{at}: pad {} is not valid", pad.name)));
            }
        }
        Ok(())
    }

    /// The variant called `name`.
    pub fn variant(&self, name: &str) -> Result<&Variant, IpcError> {
        self.variants.get(name).ok_or_else(|| {
            IpcError::new(
                VARIANT_UNKNOWN,
                format!(
                    "{}/{name}; the family has {}",
                    self.family,
                    self.variant_names().join(", ")
                ),
            )
        })
    }

    /// The names of the variants, in order.
    pub fn variant_names(&self) -> Vec<&str> {
        self.variants.keys().map(String::as_str).collect()
    }

    /// The goals of a density level for a variant: those of the family, changed by what the
    /// variant says.
    pub fn goals(&self, variant: &Variant, level: Density) -> Goals {
        let mut goals = self.density.get(level.key()).copied().unwrap_or_default();
        if let Some(changes) = variant.density.get(level.key()) {
            for (key, value) in changes {
                match key.as_str() {
                    "toe" => goals.toe = *value,
                    "heel" => goals.heel = *value,
                    "side" => goals.side = *value,
                    "courtyard" => goals.courtyard = *value,
                    "pad_ratio" => goals.pad_ratio = *value,
                    "drill_extra" => goals.drill_extra = *value,
                    "annular" => goals.annular = *value,
                    _ => {}
                }
            }
        }
        goals
    }
}

/// A set of families.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FamilySet {
    /// The families, by name.
    pub families: Vec<Family>,
}

impl FamilySet {
    /// The families that come with the program.
    pub fn builtin() -> Result<FamilySet, IpcError> {
        let mut set = FamilySet::default();
        for (name, text) in BUILTIN {
            let family = Family::parse(text).map_err(|e| bad(format!("{name}: {}", e.detail)))?;
            set.add(family)?;
        }
        Ok(set)
    }

    /// The families of the `.json` files of a folder, in the order of their names.
    pub fn from_dir(dir: &Path, settings: &IpcSettings) -> Result<FamilySet, IpcError> {
        let unusable =
            |what: String| IpcError::new(FILE_UNUSABLE, format!("{}: {what}", dir.display()));
        let mut names: Vec<std::path::PathBuf> = Vec::new();
        for entry in std::fs::read_dir(dir).map_err(|e| unusable(e.to_string()))? {
            let entry = entry.map_err(|e| unusable(e.to_string()))?;
            let path = entry.path();
            let is_json = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("json"));
            if is_json && path.is_file() {
                names.push(path);
            }
        }
        names.sort();
        if names.len() > settings.max_family_files {
            return Err(unusable(format!(
                "more than {} family files",
                settings.max_family_files
            )));
        }
        let mut set = FamilySet::default();
        for path in names {
            let size = std::fs::metadata(&path)
                .map_err(|e| unusable(e.to_string()))?
                .len();
            if size > settings.max_family_bytes {
                return Err(unusable(format!(
                    "{} is larger than {} bytes",
                    path.display(),
                    settings.max_family_bytes
                )));
            }
            let text = std::fs::read_to_string(&path).map_err(|e| unusable(e.to_string()))?;
            let family = Family::parse(&text)
                .map_err(|e| bad(format!("{}: {}", path.display(), e.detail)))?;
            set.add(family)?;
        }
        if set.families.is_empty() {
            return Err(unusable("no family files".to_owned()));
        }
        Ok(set)
    }

    fn add(&mut self, family: Family) -> Result<(), IpcError> {
        if self.families.iter().any(|f| f.family == family.family) {
            return Err(bad(format!("the family {} appears twice", family.family)));
        }
        self.families.push(family);
        Ok(())
    }

    /// The family called `name`.
    pub fn get(&self, name: &str) -> Result<&Family, IpcError> {
        self.families
            .iter()
            .find(|f| f.family == name)
            .ok_or_else(|| {
                let known: Vec<&str> = self.families.iter().map(|f| f.family.as_str()).collect();
                IpcError::new(
                    FAMILY_UNKNOWN,
                    format!("{name}; the families are {}", known.join(", ")),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_families_that_come_with_the_program_all_parse_and_say_they_are_not_verified() {
        let set = FamilySet::builtin().unwrap();
        assert_eq!(set.families.len(), BUILTIN.len());
        for family in &set.families {
            assert!(!family.verified, "{}", family.family);
            assert!(family.status.contains("not verified"), "{}", family.family);
            assert_eq!(family.standard, "IPC-7351B");
        }
        for (name, _) in BUILTIN {
            assert!(set.get(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn the_file_name_of_a_builtin_family_is_its_name() {
        for (name, text) in BUILTIN {
            assert_eq!(Family::parse(text).unwrap().family, name);
        }
    }

    #[test]
    fn densities_read_from_both_spellings() {
        assert_eq!(Density::parse("a"), Some(Density::A));
        assert_eq!(Density::parse(" M "), Some(Density::A));
        assert_eq!(Density::parse("n"), Some(Density::B));
        assert_eq!(Density::parse("L"), Some(Density::C));
        assert_eq!(Density::parse("D"), None);
        assert_eq!(Density::A.letter(), 'M');
        assert_eq!(Density::C.key(), "C");
    }

    #[test]
    fn a_variant_changes_only_the_goals_it_names() {
        let set = FamilySet::builtin().unwrap();
        let chip = set.get("chip").unwrap();
        let small = chip.variant("0201").unwrap();
        let base = chip.variant("0603").unwrap();
        let a = chip.goals(small, Density::A);
        let b = chip.goals(base, Density::A);
        assert_eq!(a.toe, 0.30);
        assert_eq!(b.toe, 0.55);
        assert_eq!(a.courtyard, b.courtyard);
    }

    #[test]
    fn unknown_names_and_broken_files_are_refused_with_their_codes() {
        let set = FamilySet::builtin().unwrap();
        assert_eq!(set.get("chop").unwrap_err().code, FAMILY_UNKNOWN);
        let chip = set.get("chip").unwrap();
        assert_eq!(chip.variant("9999").unwrap_err().code, VARIANT_UNKNOWN);
        assert_eq!(Family::parse("{").unwrap_err().code, FAMILY_INVALID);
        let (_, text) = BUILTIN[1];
        let wrong = text.replace("phoenix-ipc-family/1", "phoenix-ipc-family/9");
        assert_eq!(Family::parse(&wrong).unwrap_err().code, FAMILY_INVALID);
        let no_layout = text.replace("\"two_terminal\"", "\"nothing\"");
        assert_eq!(Family::parse(&no_layout).unwrap_err().code, FAMILY_INVALID);
        let extra = text.replace("\"verified\"", "\"checked\"");
        assert!(Family::parse(&extra).is_err());
    }

    #[test]
    fn a_folder_of_family_files_is_read_in_order_and_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let (_, chip) = BUILTIN[1];
        let (_, bga) = BUILTIN[0];
        std::fs::write(dir.path().join("b.json"), chip).unwrap();
        std::fs::write(dir.path().join("a.json"), bga).unwrap();
        std::fs::write(dir.path().join("notes.txt"), "not a family").unwrap();
        let settings = IpcSettings::default();
        let set = FamilySet::from_dir(dir.path(), &settings).unwrap();
        let names: Vec<&str> = set.families.iter().map(|f| f.family.as_str()).collect();
        assert_eq!(names, ["bga", "chip"]);
        std::fs::write(dir.path().join("c.json"), chip).unwrap();
        let e = FamilySet::from_dir(dir.path(), &settings).unwrap_err();
        assert_eq!(e.code, FAMILY_INVALID);
        let small = IpcSettings {
            max_family_files: 1,
            ..IpcSettings::default()
        };
        assert_eq!(
            FamilySet::from_dir(dir.path(), &small).unwrap_err().code,
            FILE_UNUSABLE
        );
        let empty = tempfile::tempdir().unwrap();
        assert!(FamilySet::from_dir(empty.path(), &settings).is_err());
    }
}
