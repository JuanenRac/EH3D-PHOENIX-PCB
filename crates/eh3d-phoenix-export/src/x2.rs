//! The attributes of Gerber X2 that the export writes: file attributes (`TF`), aperture
//! attributes (`TA`, `.AperFunction`) and object attributes (`TO`, `TD`), and their twins in the
//! drill files, which carry them in comments that begin with `#@!`.
//!
//! The writer of EH3D-CORE-LIBS writes the file attributes only, so the aperture and object
//! attributes are put into the text it gives back: the commands of the file carry a marker line
//! (an empty step and repeat, `%SR*%`) where an object attribute has to change, the apertures are
//! numbered by shape and function, and [`replace_markers`], [`insert_aperture_attributes`] and
//! [`insert_tool_attributes`] turn them into attribute commands. Nothing reads the clock or makes
//! a random number: the same board and profile give the same bytes.
//!
//! A text that goes into an attribute field may not hold a comma, an asterisk, a percent sign, a
//! backslash, a control character or a character outside the printable ASCII range. [`Fields`]
//! writes such a character as `\uXXXX` (four hexadecimal digits) or, in strict mode, refuses the
//! text; a character beyond the first plane of Unicode has no such escape and is always refused.

use std::collections::BTreeMap;
use std::fmt;

use uuid::Uuid;

use crate::gerber::GENERATION_SOFTWARE;
use crate::profile::{FiducialScope, X2Level, X2Profile};

/// The value of `.N` for a pad or object that is on no net.
pub const NO_NET: &str = "N/C";

/// The marker line the commands carry where an object attribute has to change.
const MARKER: &str = "%SR*%\n";

/// A name and the values of a file attribute: `(".FileFunction", ["Copper", "L1", "Top"])`.
pub type AttributeList = Vec<(&'static str, Vec<String>)>;

/// The functions of an aperture of a Gerber file that this export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApFunction {
    /// The pad of a via.
    ViaPad,
    /// The pad of a through-hole component.
    ComponentPad,
    /// The copper-defined pad of a surface-mount component.
    SmdPad,
    /// A heatsink or exposed pad.
    HeatsinkPad,
    /// A fiducial that serves the area around it.
    FiducialLocal,
    /// A fiducial that serves the whole board.
    FiducialGlobal,
    /// A track or a zone of copper that carries current.
    Conductor,
    /// The board outline.
    Profile,
}

impl ApFunction {
    /// The values of `.AperFunction`.
    pub fn values(self) -> &'static [&'static str] {
        match self {
            ApFunction::ViaPad => &["ViaPad"],
            ApFunction::ComponentPad => &["ComponentPad"],
            ApFunction::SmdPad => &["SMDPad", "CuDef"],
            ApFunction::HeatsinkPad => &["HeatsinkPad"],
            ApFunction::FiducialLocal => &["FiducialPad", "Local"],
            ApFunction::FiducialGlobal => &["FiducialPad", "Global"],
            ApFunction::Conductor => &["Conductor"],
            ApFunction::Profile => &["Profile"],
        }
    }

    /// The function of a fiducial of a kind.
    pub fn fiducial(scope: FiducialScope) -> ApFunction {
        match scope {
            FiducialScope::Local => ApFunction::FiducialLocal,
            FiducialScope::Global => ApFunction::FiducialGlobal,
        }
    }
}

/// The functions of a drill tool that this export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DrillFunction {
    /// The hole of a via.
    Via,
    /// The plated hole of a component pad.
    Component,
    /// A hole that is not plated and belongs to no pad on copper.
    Mechanical,
}

impl DrillFunction {
    /// The values of `.AperFunction` in a drill file.
    pub fn values(self) -> &'static [&'static str] {
        match self {
            DrillFunction::Via => &["Plated", "PTH", "ViaDrill"],
            DrillFunction::Component => &["Plated", "PTH", "ComponentDrill"],
            DrillFunction::Mechanical => &["NonPlated", "NPTH", "MechanicalDrill"],
        }
    }
}

/// The first values of `.AperFunction` of a Gerber file that the specification of X2 names.
pub const GERBER_FUNCTIONS: &[&str] = &[
    "ViaPad",
    "ComponentPad",
    "SMDPad",
    "BGAPad",
    "ConnectorPad",
    "HeatsinkPad",
    "TestPad",
    "CastellatedPad",
    "FiducialPad",
    "ThermalReliefPad",
    "WasherPad",
    "AntiPad",
    "OtherPad",
    "Conductor",
    "NonConductor",
    "CopperBalancing",
    "Border",
    "OtherCopper",
    "ComponentMain",
    "ComponentOutline",
    "ComponentPin",
    "Profile",
    "Material",
    "Other",
];

/// The values of `.AperFunction` of a drill tool that the specification of X2 names.
pub const DRILL_FUNCTIONS: &[&str] = &[
    "ViaDrill",
    "BackDrill",
    "ComponentDrill",
    "MechanicalDrill",
    "CastellatedDrill",
    "OtherDrill",
];

/// Why a text cannot go into an attribute field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// The text.
    pub text: String,
    /// The character that cannot be written.
    pub character: char,
    /// True when the mode is strict; false when the character has no escape.
    pub strict: bool,
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the attribute text '{}' holds the character U+{:04X}, which {}",
            self.text,
            u32::from(self.character),
            if self.strict {
                "an attribute field may not hold (strict mode)"
            } else {
                "has no escape"
            }
        )
    }
}

impl std::error::Error for FieldError {}

fn is_plain(c: char) -> bool {
    matches!(c, ' '..='~') && !matches!(c, ',' | '*' | '%' | '\\')
}

/// True when `text` holds a character that an attribute field may not hold as it is.
pub fn needs_escape(text: &str) -> bool {
    !text.chars().all(is_plain)
}

/// Writes every character that a field may not hold as `\uXXXX`. A character beyond the first
/// plane of Unicode has no such escape and is refused.
pub fn escape_field(text: &str) -> Result<String, FieldError> {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if is_plain(c) {
            out.push(c);
        } else if u32::from(c) <= 0xFFFF {
            out.push_str(&format!("\\u{:04X}", u32::from(c)));
        } else {
            return Err(FieldError {
                text: text.to_owned(),
                character: c,
                strict: false,
            });
        }
    }
    Ok(out)
}

/// The text a field stands for: every `\uXXXX` is the character it names. A backslash that does
/// not begin a valid escape stays as it is.
pub fn unescape_field(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find("\\u") {
        out.push_str(&rest[..index]);
        let digits = rest.get(index + 2..index + 6);
        let character = digits
            .filter(|d| d.bytes().all(|b| b.is_ascii_hexdigit()))
            .and_then(|d| u32::from_str_radix(d, 16).ok())
            .and_then(char::from_u32);
        match character {
            Some(c) => {
                out.push(c);
                rest = &rest[index + 6..];
            }
            None => {
                out.push_str("\\u");
                rest = &rest[index + 2..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Makes the texts of the attribute fields of one file, and counts those that were escaped.
#[derive(Debug, Clone, Copy, Default)]
pub struct Fields {
    strict: bool,
    escaped: usize,
}

impl Fields {
    /// A maker; `strict` refuses a text that would need an escape.
    pub fn new(strict: bool) -> Fields {
        Fields { strict, escaped: 0 }
    }

    /// How many texts were escaped so far.
    pub fn escaped(&self) -> usize {
        self.escaped
    }

    /// The text as it goes into a field.
    pub fn text(&mut self, text: &str) -> Result<String, FieldError> {
        if !needs_escape(text) {
            return Ok(text.to_owned());
        }
        if self.strict {
            let character = text.chars().find(|c| !is_plain(*c)).unwrap_or(' ');
            return Err(FieldError {
                text: text.to_owned(),
                character,
                strict: true,
            });
        }
        let escaped = escape_field(text)?;
        self.escaped += 1;
        Ok(escaped)
    }
}

/// A UUID built from the name of a project: the first 128 bits of the SHA-256 of the name, with
/// the version bits 5 and the variant bits of RFC 4122 set. The same name always gives the same
/// UUID.
pub fn derived_project_id(name: &str) -> Uuid {
    let digest = eh3d_container::sha256_hex(format!("phoenix-pcb-project:{name}").as_bytes());
    let bits = digest
        .get(..32)
        .and_then(|h| u128::from_str_radix(h, 16).ok())
        .unwrap_or(0);
    let bits = (bits & !(0xF_u128 << 76)) | (5_u128 << 76);
    let bits = (bits & !(0x3_u128 << 62)) | (2_u128 << 62);
    Uuid::from_u128(bits)
}

/// What the attributes of a file are made from.
#[derive(Debug, Clone)]
pub struct FileInput<'a> {
    /// The choices of the profile.
    pub x2: &'a X2Profile,
    /// The name of the board.
    pub board_name: &'a str,
    /// The values of `.FileFunction`, when the file has a known function.
    pub function: Option<Vec<String>>,
    /// The value of `.FilePolarity`, when the kind of file has one.
    pub polarity: Option<&'static str>,
}

/// The file attributes of a file, in order, for the level of the profile: three attributes at the
/// level `legacy`, none at `x1`, and at the other levels also the creation date (only when the
/// profile gives it), the project, the coordinate system and the part.
pub fn file_attributes(
    input: &FileInput<'_>,
    fields: &mut Fields,
) -> Result<AttributeList, FieldError> {
    let level = input.x2.level;
    let mut list: AttributeList = Vec::new();
    if level == X2Level::X1 {
        return Ok(list);
    }
    list.push((
        ".GenerationSoftware",
        GENERATION_SOFTWARE
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
    ));
    if level.has_file_attributes() {
        if let Some(date) = &input.x2.creation_date {
            list.push((".CreationDate", vec![date.clone()]));
        }
        let name = if input.board_name.trim().is_empty() {
            "board"
        } else {
            input.board_name
        };
        let id = input
            .x2
            .project_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(|| derived_project_id(name));
        list.push((
            ".ProjectId",
            vec![
                fields.text(name)?,
                id.hyphenated().to_string(),
                fields.text(&input.x2.project_revision)?,
            ],
        ));
        list.push((
            ".SameCoordinates",
            if input.x2.same_coordinates.is_empty() {
                Vec::new()
            } else {
                vec![input.x2.same_coordinates.clone()]
            },
        ));
    }
    if let Some(function) = &input.function {
        list.push((".FileFunction", function.clone()));
    }
    if let Some(polarity) = input.polarity {
        list.push((".FilePolarity", vec![polarity.to_owned()]));
    }
    if level.has_file_attributes() {
        list.push((".Part", vec!["Single".to_owned()]));
    }
    Ok(list)
}

/// The object attributes of one drawn object, as the raw texts (not yet made into fields).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectSet {
    /// `.N`: the name of the net, or [`NO_NET`].
    pub net: Option<String>,
    /// `.P`: the reference designator and the pin number.
    pub pin: Option<(String, String)>,
    /// `.C`: the reference designator.
    pub component: Option<String>,
}

/// The commands that turn the object attributes in force, `previous`, into `next`: an attribute
/// that is new or has changed is set with `TO`, one that is no longer wanted is deleted with `TD`.
/// The result is empty when nothing changes.
pub fn object_changes(
    previous: &ObjectSet,
    next: &ObjectSet,
    fields: &mut Fields,
) -> Result<String, FieldError> {
    let mut out = String::new();
    if previous.net != next.net {
        match &next.net {
            Some(net) => out.push_str(&format!("%TO.N,{}*%\n", fields.text(net)?)),
            None => out.push_str("%TD.N*%\n"),
        }
    }
    if previous.pin != next.pin {
        match &next.pin {
            Some((refdes, pin)) => out.push_str(&format!(
                "%TO.P,{},{}*%\n",
                fields.text(refdes)?,
                fields.text(pin)?
            )),
            None => out.push_str("%TD.P*%\n"),
        }
    }
    if previous.component != next.component {
        match &next.component {
            Some(refdes) => out.push_str(&format!("%TO.C,{}*%\n", fields.text(refdes)?)),
            None => out.push_str("%TD.C*%\n"),
        }
    }
    Ok(out)
}

/// The marker command: the writer writes it as the line `%SR*%`, which [`replace_markers`]
/// replaces.
pub fn marker() -> eh3d_parser_gerber::Operation {
    eh3d_parser_gerber::Operation::StepRepeat(None)
}

/// Replaces the marker lines of `text`, one by one and in order, with `replacements`. `None` when
/// the number of markers is not the number of replacements.
pub fn replace_markers(text: &str, replacements: &[String]) -> Option<String> {
    let mut out =
        String::with_capacity(text.len() + replacements.iter().map(String::len).sum::<usize>());
    let mut next = replacements.iter();
    for line in text.split_inclusive('\n') {
        if line == MARKER {
            out.push_str(next.next()?);
        } else {
            out.push_str(line);
        }
    }
    next.next().is_none().then_some(out)
}

/// Puts `TA.AperFunction` before the definition of each aperture of `text` that has a function
/// and `TD.AperFunction` after the last of them, so that the attribute is in force for the
/// definitions it is meant for and for no other command.
pub fn insert_aperture_attributes(text: &str, functions: &BTreeMap<u32, ApFunction>) -> String {
    let mut out = String::with_capacity(text.len() + 32 * functions.len());
    let mut current: Option<ApFunction> = None;
    for line in text.split_inclusive('\n') {
        if let Some(rest) = line.strip_prefix("%ADD") {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            let function = digits
                .parse::<u32>()
                .ok()
                .and_then(|n| functions.get(&n).copied());
            if function != current {
                match function {
                    Some(f) => {
                        out.push_str(&format!("%TA.AperFunction,{}*%\n", f.values().join(",")))
                    }
                    None => out.push_str("%TD.AperFunction*%\n"),
                }
                current = function;
            }
        } else if current.take().is_some() {
            out.push_str("%TD.AperFunction*%\n");
        }
        out.push_str(line);
    }
    out
}

/// True for a tool definition of a drill file: `T`, two digits and `C`.
fn tool_number(line: &str) -> Option<u32> {
    let rest = line.strip_prefix('T')?;
    let digits = rest.get(..2)?;
    if !digits.bytes().all(|b| b.is_ascii_digit()) || rest.as_bytes().get(2) != Some(&b'C') {
        return None;
    }
    digits.parse().ok()
}

/// Puts `; #@! TA.AperFunction,...` before the definition of each tool of a drill file that has a
/// function and `; #@! TD.AperFunction` after the last definition.
pub fn insert_tool_attributes(text: &str, functions: &BTreeMap<u32, DrillFunction>) -> String {
    let mut out = String::with_capacity(text.len() + 48 * functions.len());
    let mut in_tools = false;
    for line in text.split_inclusive('\n') {
        match tool_number(line) {
            Some(number) => {
                in_tools = true;
                if let Some(f) = functions.get(&number) {
                    out.push_str(&format!("; #@! TA.AperFunction,{}\n", f.values().join(",")));
                }
            }
            None => {
                if in_tools {
                    out.push_str("; #@! TD.AperFunction\n");
                    in_tools = false;
                }
            }
        }
        out.push_str(line);
    }
    out
}

/// The comment lines of a drill file for its file attributes (`; #@! TF.FileFunction,...`), without
/// the leading semicolon, which the writer adds.
pub fn drill_comments(list: &AttributeList) -> Vec<String> {
    list.iter()
        .map(|(name, values)| {
            let mut line = format!(" #@! TF{name}");
            for v in values {
                line.push(',');
                line.push_str(v);
            }
            line
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_not_changed_and_the_rest_is_escaped() {
        assert!(!needs_escape("VCC_3V3/A"));
        assert_eq!(escape_field("VCC").unwrap(), "VCC");
        assert_eq!(escape_field("a,b").unwrap(), "a\\u002Cb");
        assert_eq!(escape_field("50%*").unwrap(), "50\\u0025\\u002A");
        assert_eq!(escape_field("a\\b").unwrap(), "a\\u005Cb");
        assert_eq!(escape_field("Tensión").unwrap(), "Tensi\\u00F3n");
        assert!(escape_field("a\u{1F600}").is_err());
    }

    #[test]
    fn an_escaped_text_reads_back_as_the_text() {
        for text in ["a,b", "x\\y", "Tensión 5V", "100%", "日本語", "plain"] {
            assert_eq!(unescape_field(&escape_field(text).unwrap()), text);
        }
        assert_eq!(unescape_field("\\u12"), "\\u12");
        assert_eq!(unescape_field("\\uZZZZ!"), "\\uZZZZ!");
    }

    #[test]
    fn strict_mode_refuses_what_the_other_mode_escapes() {
        let mut lax = Fields::new(false);
        assert_eq!(lax.text("A,B").unwrap(), "A\\u002CB");
        assert_eq!(lax.text("A").unwrap(), "A");
        assert_eq!(lax.escaped(), 1);
        let mut strict = Fields::new(true);
        let error = strict.text("A,B").unwrap_err();
        assert!(error.strict && error.character == ',');
        assert_eq!(strict.text("AB").unwrap(), "AB");
        assert_eq!(strict.escaped(), 0);
    }

    #[test]
    fn the_project_identity_is_a_stable_uuid_of_the_name() {
        let a = derived_project_id("Board");
        assert_eq!(a, derived_project_id("Board"));
        assert_ne!(a, derived_project_id("Other"));
        let text = a.hyphenated().to_string();
        assert_eq!(text.len(), 36);
        assert_eq!(text.as_bytes()[14], b'5');
        assert!(matches!(text.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
    }

    #[test]
    fn the_file_attributes_follow_the_level() {
        let function = Some(vec!["Copper".to_owned(), "L1".to_owned(), "Top".to_owned()]);
        let make = |level, date: Option<&str>| {
            let x2 = X2Profile {
                level,
                creation_date: date.map(str::to_owned),
                ..X2Profile::default()
            };
            let input = FileInput {
                x2: &x2,
                board_name: "Demo",
                function: function.clone(),
                polarity: Some("Positive"),
            };
            file_attributes(&input, &mut Fields::new(false))
                .unwrap()
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            make(X2Level::Legacy, Some("2000-01-01T00:00:00+00:00")),
            vec![".GenerationSoftware", ".FileFunction", ".FilePolarity"]
        );
        assert!(make(X2Level::X1, None).is_empty());
        assert_eq!(
            make(X2Level::File, None),
            vec![
                ".GenerationSoftware",
                ".ProjectId",
                ".SameCoordinates",
                ".FileFunction",
                ".FilePolarity",
                ".Part"
            ]
        );
        assert_eq!(
            make(X2Level::Full, Some("2000-01-01T00:00:00+00:00"))[1],
            ".CreationDate"
        );
    }

    #[test]
    fn object_attributes_change_only_what_differs() {
        let mut fields = Fields::new(false);
        let a = ObjectSet {
            net: Some("GND".to_owned()),
            pin: Some(("R1".to_owned(), "1".to_owned())),
            component: Some("R1".to_owned()),
        };
        let none = ObjectSet::default();
        assert_eq!(
            object_changes(&none, &a, &mut fields).unwrap(),
            "%TO.N,GND*%\n%TO.P,R1,1*%\n%TO.C,R1*%\n"
        );
        let b = ObjectSet {
            pin: None,
            component: None,
            ..a.clone()
        };
        assert_eq!(
            object_changes(&a, &b, &mut fields).unwrap(),
            "%TD.P*%\n%TD.C*%\n"
        );
        assert_eq!(object_changes(&b, &b, &mut fields).unwrap(), "");
    }

    #[test]
    fn markers_are_replaced_in_order_and_counted() {
        let text = "a\n%SR*%\nb\n%SR*%\nc\n";
        let out = replace_markers(text, &["X\n".to_owned(), String::new()]).unwrap();
        assert_eq!(out, "a\nX\nb\nc\n");
        assert!(replace_markers(text, &["X\n".to_owned()]).is_none());
        assert!(replace_markers("a\n", &["X\n".to_owned()]).is_none());
    }

    #[test]
    fn aperture_attributes_surround_the_definitions_they_belong_to() {
        let text = "%MOMM*%\n%ADD10C,0.1*%\n%ADD11R,1X1*%\n%ADD12C,0.2*%\nD10*\nM02*\n";
        let functions = BTreeMap::from([(10, ApFunction::Conductor), (11, ApFunction::SmdPad)]);
        assert_eq!(
            insert_aperture_attributes(text, &functions),
            "%MOMM*%\n%TA.AperFunction,Conductor*%\n%ADD10C,0.1*%\n\
             %TA.AperFunction,SMDPad,CuDef*%\n%ADD11R,1X1*%\n\
             %TD.AperFunction*%\n%ADD12C,0.2*%\nD10*\nM02*\n"
        );
    }

    #[test]
    fn tool_attributes_go_before_the_tool_definitions() {
        let text = "M48\nMETRIC,TZ\nT01C0.3\nT02C0.8\n%\nT01\nX1Y1\nM30\n";
        let functions = BTreeMap::from([(1, DrillFunction::Via), (2, DrillFunction::Component)]);
        assert_eq!(
            insert_tool_attributes(text, &functions),
            "M48\nMETRIC,TZ\n; #@! TA.AperFunction,Plated,PTH,ViaDrill\nT01C0.3\n\
             ; #@! TA.AperFunction,Plated,PTH,ComponentDrill\nT02C0.8\n\
             ; #@! TD.AperFunction\n%\nT01\nX1Y1\nM30\n"
        );
    }

    #[test]
    fn a_drill_comment_holds_the_attribute() {
        let list: AttributeList =
            vec![(".FileFunction", vec!["Plated".to_owned(), "1".to_owned()])];
        assert_eq!(drill_comments(&list), vec![" #@! TF.FileFunction,Plated,1"]);
    }
}
