//! Export profiles: the choices of a manufacturer or of an assembler, read from a JSON file.
//!
//! A profile says how many decimals the Gerber coordinates have, how far the solder mask opens past
//! a pad, which style the drill files use, whether a drilled oblong pad is a slot, which columns the
//! placement and bill of materials files have and how the rotation of a part on the bottom side is
//! written. Nothing of this is fixed in the export code: the file `config/export_profiles.json`
//! holds the profiles; [`Profile::default`] is only what a missing field takes.
//!
//! A profile file is `{"format": "phoenix-export/1", "profiles": [{"name": "...", ...}]}`. Lengths
//! are integer nanometres, an unknown field is an error.

use serde::Deserialize;

use crate::package::ExportError;

/// The value of `format` that a profile file must have.
pub const FORMAT_TAG: &str = "phoenix-export/1";

/// The columns a placement file may have.
pub const PLACEMENT_KEYS: [&str; 7] = ["ref", "value", "package", "x", "y", "rotation", "side"];
/// The columns a bill of materials file may have.
pub const BOM_KEYS: [&str; 4] = ["designators", "quantity", "value", "package"];

/// How the rotation of a part on the bottom side is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BottomRotation {
    /// The rotation of the part, unchanged.
    Keep,
    /// The mirror image of the rotation, 360 degrees minus the rotation.
    Mirror,
    /// The rotation plus a half turn.
    HalfTurn,
}

/// Where the coordinates of the placement file start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// The origin of the board.
    Absolute,
    /// The lowest-left corner of the bounding box of the board outline.
    OutlineMin,
}

/// The style of the coordinates of the drill files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrillStyle {
    /// A number with a decimal point.
    Decimal,
    /// An integer of a fixed format.
    Fixed,
}

/// How much of Gerber X2 the layer files and the drill files carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum X2Level {
    /// The three file attributes this export always wrote (`.GenerationSoftware`, `.FileFunction`,
    /// `.FilePolarity`) and nothing else. The default: the files do not change.
    Legacy,
    /// Plain Gerber X1: no attribute of any kind, no comment with an attribute in the drill files.
    X1,
    /// Every file attribute (`TF`).
    File,
    /// The file attributes and the aperture attributes (`TA`, `.AperFunction`).
    Aperture,
    /// The file, aperture and object attributes (`TO`, `TD`).
    Full,
}

impl X2Level {
    /// True when the files carry the whole set of file attributes.
    pub fn has_file_attributes(self) -> bool {
        matches!(self, X2Level::File | X2Level::Aperture | X2Level::Full)
    }

    /// True when the apertures and the drill tools carry a function.
    pub fn has_aperture_attributes(self) -> bool {
        matches!(self, X2Level::Aperture | X2Level::Full)
    }

    /// True when the drawn objects carry attributes.
    pub fn has_object_attributes(self) -> bool {
        self == X2Level::Full
    }
}

/// An object attribute that the files may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectAttribute {
    /// `.N`, the net of a pad, a track, a via or a zone.
    Net,
    /// `.P`, the reference designator and the pin number of a pad.
    Pin,
    /// `.C`, the reference designator of the component of a pad.
    Component,
}

/// The kind of fiducial the pads that are fiducials are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FiducialScope {
    /// A fiducial that serves the area around it.
    Local,
    /// A fiducial that serves the whole board.
    Global,
}

/// The choices for the Gerber X2 attributes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct X2Profile {
    /// How much of X2 is written.
    pub level: X2Level,
    /// The object attributes of the level `full`, in the order they are written.
    pub object_attributes: Vec<ObjectAttribute>,
    /// Refuse (instead of escaping as a Unicode escape) a net name, a reference designator or a pin
    /// number with a character that an attribute field may not hold.
    pub strict: bool,
    /// The value of `.CreationDate`, an ISO 8601 date and time with an offset. Written only when
    /// it is given here (or on the command line): the files never read the clock.
    pub creation_date: Option<String>,
    /// The identity of the project for `.ProjectId`, a UUID; when it is missing, one is derived
    /// from the name of the board.
    pub project_id: Option<String>,
    /// The revision of `.ProjectId`.
    pub project_revision: String,
    /// The identifier of `.SameCoordinates`; empty for the attribute without an identifier.
    pub same_coordinates: String,
    /// A pad of a footprint whose name holds one of these words (any case) is a fiducial pad.
    pub fiducial_names: Vec<String>,
    /// The kind of fiducial pad those pads are.
    pub fiducial_scope: FiducialScope,
    /// A pad with one of these numbers (any case) is a heatsink pad.
    pub heatsink_pad_numbers: Vec<String>,
}

impl Default for X2Profile {
    fn default() -> Self {
        X2Profile {
            level: X2Level::Legacy,
            object_attributes: vec![
                ObjectAttribute::Net,
                ObjectAttribute::Pin,
                ObjectAttribute::Component,
            ],
            strict: false,
            creation_date: None,
            project_id: None,
            project_revision: "1".to_owned(),
            same_coordinates: "Original".to_owned(),
            fiducial_names: vec!["fiducial".to_owned()],
            fiducial_scope: FiducialScope::Local,
            heatsink_pad_numbers: vec!["EP".to_owned()],
        }
    }
}

impl X2Profile {
    /// True when `attribute` is among the object attributes to write.
    pub fn writes(&self, attribute: ObjectAttribute) -> bool {
        self.level.has_object_attributes() && self.object_attributes.contains(&attribute)
    }

    fn validate(&self, name: &str) -> Result<(), ExportError> {
        let field = |what: &str, value: &str| -> Result<(), ExportError> {
            if value.is_empty() || value.len() > 64 || crate::x2::needs_escape(value) {
                Err(bad(format!("{name}: gerber.x2.{what} is not usable")))
            } else {
                Ok(())
            }
        };
        field("project_revision", &self.project_revision)?;
        if !self.same_coordinates.is_empty() {
            field("same_coordinates", &self.same_coordinates)?;
        }
        if let Some(date) = &self.creation_date {
            if chrono::DateTime::parse_from_rfc3339(date).is_err() {
                return Err(bad(format!(
                    "{name}: gerber.x2.creation_date is not a date and time with an offset"
                )));
            }
        }
        if let Some(id) = &self.project_id {
            if uuid::Uuid::parse_str(id).is_err() {
                return Err(bad(format!("{name}: gerber.x2.project_id is not a UUID")));
            }
        }
        for (i, a) in self.object_attributes.iter().enumerate() {
            if self.object_attributes[..i].contains(a) {
                return Err(bad(format!(
                    "{name}: gerber.x2.object_attributes names an attribute twice"
                )));
            }
        }
        if self.fiducial_names.iter().any(|n| n.trim().is_empty()) {
            return Err(bad(format!(
                "{name}: gerber.x2.fiducial_names holds an empty word"
            )));
        }
        if self
            .heatsink_pad_numbers
            .iter()
            .any(|n| n.trim().is_empty())
        {
            return Err(bad(format!(
                "{name}: gerber.x2.heatsink_pad_numbers holds an empty number"
            )));
        }
        Ok(())
    }
}

/// A column of a CSV file: what it holds and what its header says.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Column {
    /// What the column holds; one of [`PLACEMENT_KEYS`] or [`BOM_KEYS`].
    pub key: String,
    /// The text of the header.
    pub header: String,
}

fn column(key: &str, header: &str) -> Column {
    Column {
        key: key.to_owned(),
        header: header.to_owned(),
    }
}

/// The choices for the Gerber files.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct GerberProfile {
    /// The extension of the layer files, without the dot.
    pub extension: String,
    /// Digits after the point (1 to 6), in millimetres.
    pub decimal_digits: u32,
    /// How far, per side, a solder mask opening is larger than its pad, in nanometres.
    pub mask_expansion_nm: i64,
    /// How far, per side, a paste opening is smaller than its pad, in nanometres.
    pub paste_shrink_nm: i64,
    /// The width of the line that draws the board outline, in nanometres.
    pub outline_width_nm: i64,
    /// The spokes of the thermal relief that the pads of the net of a pour get: 0 leaves them
    /// solid (the default), 4 clears four gaps around each pad and leaves four spokes along the
    /// axes, 2 clears a gap above and a gap below and leaves a spoke on each side. They are the
    /// `thermal_spokes`, `thermal_gap_nm` and `thermal_spoke_width_nm` of `config/zones.cfg`.
    pub zone_thermal_spokes: u32,
    /// Whether the rectangular and oblong pads get the relief too (false, the default, leaves
    /// them solid and gives it to the round pads only).
    pub zone_thermal_non_round: bool,
    /// The width of the gap of a thermal relief, in nanometres.
    pub zone_thermal_gap_nm: i64,
    /// The width of each spoke of a thermal relief, in nanometres.
    pub zone_thermal_spoke_width_nm: i64,
    /// The smallest island of copper of a pour that is written, in square nanometres (the
    /// `min_island_area_nm2` of `config/zones.cfg`): the zones that touch along a border and
    /// share a net make one island, and a smaller one is left out. 0 (the default) writes every
    /// zone.
    pub zone_island_min_area_nm2: i64,
    /// The Gerber X2 attributes.
    pub x2: X2Profile,
}

impl Default for GerberProfile {
    fn default() -> Self {
        GerberProfile {
            extension: "gbr".to_owned(),
            decimal_digits: 6,
            mask_expansion_nm: 50_000,
            paste_shrink_nm: 0,
            outline_width_nm: 100_000,
            zone_thermal_spokes: 0,
            zone_thermal_non_round: false,
            zone_thermal_gap_nm: 250_000,
            zone_thermal_spoke_width_nm: 250_000,
            zone_island_min_area_nm2: 0,
            x2: X2Profile::default(),
        }
    }
}

/// The choices for the Excellon files.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DrillProfile {
    /// The style of the coordinates.
    pub style: DrillStyle,
    /// Digits after the point of the decimal style (1 to 8).
    pub decimals: u32,
    /// Digits before the point of the fixed style (1 to 8).
    pub fixed_integer_digits: u32,
    /// Digits after the point of the fixed style (1 to 8).
    pub fixed_decimal_digits: u32,
    /// The name of the file of the plated holes.
    pub plated_file: String,
    /// The name of the file of the holes that are not plated.
    pub non_plated_file: String,
    /// Write a drilled oblong pad as a slot (`G85`).
    pub oblong_as_slot: bool,
}

impl Default for DrillProfile {
    fn default() -> Self {
        DrillProfile {
            style: DrillStyle::Decimal,
            decimals: 6,
            fixed_integer_digits: 3,
            fixed_decimal_digits: 3,
            plated_file: "drill_pth.drl".to_owned(),
            non_plated_file: "drill_npth.drl".to_owned(),
            oblong_as_slot: false,
        }
    }
}

/// The choices for the placement (pick and place) file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PlacementProfile {
    /// The name of the file.
    pub file: String,
    /// The separator of the fields.
    pub delimiter: char,
    /// Digits after the point of X and Y, in millimetres (0 to 6).
    pub coordinate_decimals: u32,
    /// Digits after the point of the rotation, in degrees (0 to 6).
    pub rotation_decimals: u32,
    /// How the rotation of a part on the bottom side is written.
    pub bottom_rotation: BottomRotation,
    /// Where the coordinates start.
    pub origin: Origin,
    /// The text for the top side.
    pub side_top: String,
    /// The text for the bottom side.
    pub side_bottom: String,
    /// The columns, in order.
    pub columns: Vec<Column>,
}

impl Default for PlacementProfile {
    fn default() -> Self {
        PlacementProfile {
            file: "placement.csv".to_owned(),
            delimiter: ',',
            coordinate_decimals: 4,
            rotation_decimals: 2,
            bottom_rotation: BottomRotation::Keep,
            origin: Origin::Absolute,
            side_top: "top".to_owned(),
            side_bottom: "bottom".to_owned(),
            columns: vec![
                column("ref", "Ref"),
                column("value", "Value"),
                column("package", "Package"),
                column("x", "X_mm"),
                column("y", "Y_mm"),
                column("rotation", "Rotation"),
                column("side", "Side"),
            ],
        }
    }
}

/// The choices for the bill of materials.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct BomProfile {
    /// The name of the file.
    pub file: String,
    /// The separator of the fields.
    pub delimiter: char,
    /// The text put between the designators of a line.
    pub designator_separator: String,
    /// The columns, in order.
    pub columns: Vec<Column>,
}

impl Default for BomProfile {
    fn default() -> Self {
        BomProfile {
            file: "bom.csv".to_owned(),
            delimiter: ',',
            designator_separator: " ".to_owned(),
            columns: vec![
                column("designators", "Designators"),
                column("quantity", "Quantity"),
                column("value", "Value"),
                column("package", "Package"),
            ],
        }
    }
}

/// One export profile.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Profile {
    /// The name the command line asks for.
    pub name: String,
    /// What the profile is for.
    pub description: String,
    /// The Gerber files.
    pub gerber: GerberProfile,
    /// The drill files.
    pub drill: DrillProfile,
    /// The placement file.
    pub placement: PlacementProfile,
    /// The bill of materials.
    pub bom: BomProfile,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            name: "generic".to_owned(),
            description: String::new(),
            gerber: GerberProfile::default(),
            drill: DrillProfile::default(),
            placement: PlacementProfile::default(),
            bom: BomProfile::default(),
        }
    }
}

fn bad(message: String) -> ExportError {
    ExportError::Profile(message)
}

/// A name that is one plain file name: letters, digits, `.`, `_` and `-`, not starting with a dot.
fn plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && !eh3d_phoenix_model::is_reserved_device_name(name)
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn check_columns(what: &str, columns: &[Column], allowed: &[&str]) -> Result<(), ExportError> {
    if columns.is_empty() {
        return Err(bad(format!("{what}: no columns")));
    }
    for (i, c) in columns.iter().enumerate() {
        if !allowed.contains(&c.key.as_str()) {
            return Err(bad(format!("{what}: unknown column '{}'", c.key)));
        }
        if columns[..i].iter().any(|other| other.key == c.key) {
            return Err(bad(format!("{what}: column '{}' twice", c.key)));
        }
    }
    Ok(())
}

fn check_delimiter(what: &str, delimiter: char) -> Result<(), ExportError> {
    if matches!(delimiter, '"' | '\n' | '\r') {
        Err(bad(format!(
            "{what}: the delimiter cannot be a quote or a line break"
        )))
    } else {
        Ok(())
    }
}

impl Profile {
    /// Checks every value against its range.
    pub fn validate(&self) -> Result<(), ExportError> {
        let name = &self.name;
        if name.trim().is_empty() {
            return Err(bad("a profile has no name".to_owned()));
        }
        let g = &self.gerber;
        if g.extension.is_empty()
            || g.extension.len() > 8
            || !g.extension.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            return Err(bad(format!("{name}: gerber.extension is not usable")));
        }
        if !(1..=6).contains(&g.decimal_digits) {
            return Err(bad(format!("{name}: gerber.decimal_digits is not 1 to 6")));
        }
        for (field, value) in [
            ("gerber.mask_expansion_nm", g.mask_expansion_nm),
            ("gerber.paste_shrink_nm", g.paste_shrink_nm),
        ] {
            if value.abs() > 1_000_000 {
                return Err(bad(format!("{name}: {field} is beyond 1 mm")));
            }
        }
        if !(1..=10_000_000).contains(&g.outline_width_nm) {
            return Err(bad(format!(
                "{name}: gerber.outline_width_nm is not 1 to 10000000"
            )));
        }
        if !matches!(g.zone_thermal_spokes, 0 | 2 | 4) {
            return Err(bad(format!(
                "{name}: gerber.zone_thermal_spokes is not 0, 2 or 4"
            )));
        }
        if !(0..=1_000_000_000_000_000).contains(&g.zone_island_min_area_nm2) {
            return Err(bad(format!(
                "{name}: gerber.zone_island_min_area_nm2 is not 0 to 1000000000000000"
            )));
        }
        for (field, value) in [
            ("gerber.zone_thermal_gap_nm", g.zone_thermal_gap_nm),
            (
                "gerber.zone_thermal_spoke_width_nm",
                g.zone_thermal_spoke_width_nm,
            ),
        ] {
            if !(1..=5_000_000).contains(&value) {
                return Err(bad(format!("{name}: {field} is not 1 to 5000000")));
            }
        }
        g.x2.validate(name)?;
        let d = &self.drill;
        for (field, value) in [
            ("drill.decimals", d.decimals),
            ("drill.fixed_integer_digits", d.fixed_integer_digits),
            ("drill.fixed_decimal_digits", d.fixed_decimal_digits),
        ] {
            if !(1..=8).contains(&value) {
                return Err(bad(format!("{name}: {field} is not 1 to 8")));
            }
        }
        let p = &self.placement;
        if p.coordinate_decimals > 6 || p.rotation_decimals > 6 {
            return Err(bad(format!("{name}: placement decimals are not 0 to 6")));
        }
        check_delimiter(&format!("{name}: placement"), p.delimiter)?;
        check_columns(&format!("{name}: placement"), &p.columns, &PLACEMENT_KEYS)?;
        let b = &self.bom;
        check_delimiter(&format!("{name}: bom"), b.delimiter)?;
        check_columns(&format!("{name}: bom"), &b.columns, &BOM_KEYS)?;
        let files = [&d.plated_file, &d.non_plated_file, &p.file, &b.file];
        for (i, file) in files.iter().enumerate() {
            if !plain_file_name(file) {
                return Err(bad(format!("{name}: '{file}' is not a plain file name")));
            }
            if files[..i].contains(file) {
                return Err(bad(format!("{name}: the file name '{file}' is used twice")));
            }
            if file.as_str() == "manifest.json" {
                return Err(bad(format!(
                    "{name}: '{file}' is reserved for the manifest"
                )));
            }
        }
        Ok(())
    }
}

/// The profiles of a profile file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSet {
    /// Always [`FORMAT_TAG`].
    pub format: String,
    /// The profiles, in the order of the file.
    pub profiles: Vec<Profile>,
}

impl ProfileSet {
    /// Reads and checks a profile file. Names must be unique.
    pub fn from_json(text: &str) -> Result<ProfileSet, ExportError> {
        let set: ProfileSet =
            serde_json::from_str(text).map_err(|e| bad(format!("the profile file: {e}")))?;
        if set.format != FORMAT_TAG {
            return Err(bad(format!(
                "the profile file has format '{}', expected '{FORMAT_TAG}'",
                set.format
            )));
        }
        for (i, profile) in set.profiles.iter().enumerate() {
            profile.validate()?;
            if set.profiles[..i].iter().any(|p| p.name == profile.name) {
                return Err(bad(format!(
                    "the profile name '{}' is used twice",
                    profile.name
                )));
            }
        }
        Ok(set)
    }

    /// The profile called `name`.
    pub fn get(&self, name: &str) -> Result<&Profile, ExportError> {
        self.profiles
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| ExportError::ProfileNotFound(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_profile_is_valid() {
        Profile::default().validate().unwrap();
    }

    #[test]
    fn a_profile_file_is_read_with_its_missing_fields_defaulted() {
        let set = ProfileSet::from_json(
            r#"{"format": "phoenix-export/1", "profiles": [
                {"name": "a"},
                {"name": "b", "placement": {"bottom_rotation": "half_turn", "side_bottom": "B"}}]}"#,
        )
        .unwrap();
        assert_eq!(
            set.get("a").unwrap(),
            &Profile {
                name: "a".to_owned(),
                ..Profile::default()
            }
        );
        let b = set.get("b").unwrap();
        assert_eq!(b.placement.bottom_rotation, BottomRotation::HalfTurn);
        assert_eq!(b.placement.side_bottom, "B");
        assert_eq!(b.placement.side_top, "top");
        assert!(matches!(set.get("c"), Err(ExportError::ProfileNotFound(_))));
    }

    #[test]
    fn bad_profile_files_are_refused() {
        for text in [
            "not json",
            r#"{"format": "other", "profiles": []}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "nonsense": 1}]}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": ""}]}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": "a"}, {"name": "a"}]}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "gerber": {"decimal_digits": 9}}]}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "drill": {"plated_file": "../x.drl"}}]}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "drill": {"plated_file": "bom.csv"}}]}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "bom": {"file": "manifest.json"}}]}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "bom": {"columns": [{"key": "price", "header": "P"}]}}]}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "placement": {"columns": []}}]}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "placement": {"bottom_rotation": "sideways"}}]}"#,
            r#"{"format": "phoenix-export/1", "profiles": [{"name": "a", "placement": {"delimiter": "\""}}]}"#,
        ] {
            let error = ProfileSet::from_json(text).unwrap_err();
            assert_eq!(error.code(), crate::codes::PROFILE_INVALID, "{text}");
        }
    }

    #[test]
    fn the_profiles_shipped_with_the_repository_are_valid() {
        let text = include_str!("../../../config/export_profiles.json");
        let set = ProfileSet::from_json(text).unwrap();
        let generic = set.get("generic").unwrap();
        assert_eq!(
            Profile {
                description: String::new(),
                ..generic.clone()
            },
            Profile::default()
        );
        assert!(set.get("jlc-like").is_ok());
    }

    #[test]
    fn the_name_of_a_windows_device_is_not_a_plain_file_name() {
        for name in ["CON", "nul.gbr", "Aux.txt", "COM1.drl", "lpt9"] {
            assert!(!plain_file_name(name), "{name}");
        }
        for name in ["top.gbr", "console.gbr", "com10.drl"] {
            assert!(plain_file_name(name), "{name}");
        }
    }
}
