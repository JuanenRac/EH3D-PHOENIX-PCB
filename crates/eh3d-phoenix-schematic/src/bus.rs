//! Buses as objects of the drawing: a bus such as `D[0..7]` or `USB{DP DM}`, the entries that take a
//! member out of it, the aliases that give a name to a group, and the checks of all three
//! (`PHOENIX-BUS-<NNN>`).
//!
//! A bus is a way of naming: its members are ordinary nets. The names of the members follow fixed
//! rules, so the same bus always expands to the same nets:
//!
//! * a vector `prefix[first..last]` has the members `prefix<first>` to `prefix<last>`, up or down,
//!   at most 256 of them (`D[0..7]` gives `D0` to `D7`);
//! * a group `name{item item ...}` has one member per item, written `name.item`; an item is a plain
//!   name or a vector (`USB{DP DM}` gives `USB.DP` and `USB.DM`, `P{D[0..1] CLK}` gives `P.D0`,
//!   `P.D1` and `P.CLK`);
//! * a bare name is an alias: it expands as the group of the alias of that name.
//!
//! A bus entry sits at a point of the drawing and gives the net there the name of one member, as a
//! net label does; that is what joins a wire to a member. Nothing here is written when a schematic
//! has no bus, so schematics without buses keep their bytes.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_model::Point;
use serde::{Deserialize, Serialize};

use crate::model::{Id, Schematic, DANGLING};

/// The most members of one vector, and of one bus after expansion.
pub const MAX_VECTOR: usize = 256;
/// The most members a bus may expand to.
pub const MAX_MEMBERS: usize = 4096;
/// What separates the name of a group from the name of an item in the name of a member.
pub const GROUP_SEPARATOR: &str = ".";

/// A bus name that is not of any of the forms above.
pub const BAD_SPEC: &str = "PHOENIX-BUS-001";
/// A member of a bus that no entry uses.
pub const MEMBER_UNUSED: &str = "PHOENIX-BUS-002";
/// A bus with two members of one name.
pub const MEMBER_REPEATED: &str = "PHOENIX-BUS-003";
/// A range of a vector that is negative or too long.
pub const RANGE_INVALID: &str = "PHOENIX-BUS-004";
/// A bus whose anchor is joined to a pin or to a wire.
pub const ON_SIMPLE_NET: &str = "PHOENIX-BUS-005";
/// An entry that names a bus or a member that does not exist.
pub const ENTRY_DANGLING: &str = "PHOENIX-BUS-006";
/// An alias with a name or members that cannot be used.
pub const ALIAS_INVALID: &str = "PHOENIX-BUS-007";

/// The codes of this module with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (BAD_SPEC, "Bus name is not valid"),
    (MEMBER_UNUSED, "Bus member has no connection"),
    (MEMBER_REPEATED, "Bus has repeated members"),
    (RANGE_INVALID, "Bus range is not valid"),
    (ON_SIMPLE_NET, "Bus is connected to a simple net"),
    (
        ENTRY_DANGLING,
        "Bus entry names a bus or a member that does not exist",
    ),
    (ALIAS_INVALID, "Bus alias is not valid"),
];

/// The titles in English, Spanish, German, French, Italian, Japanese, Chinese and European
/// Portuguese, in the order of [`REGISTRY`].
pub const TITLES: [(&str, [&str; 8]); 7] = [
    (
        BAD_SPEC,
        [
            "Bus name is not valid",
            "El nombre del bus no es válido",
            "Busname ist ungültig",
            "Le nom du bus n'est pas valide",
            "Il nome del bus non è valido",
            "バス名が正しくありません",
            "总线名称无效",
            "O nome do barramento não é válido",
        ],
    ),
    (
        MEMBER_UNUSED,
        [
            "Bus member has no connection",
            "Un miembro del bus no tiene conexión",
            "Ein Busmitglied hat keine Verbindung",
            "Un membre du bus n'a aucune connexion",
            "Un membro del bus non ha connessioni",
            "バスのメンバーが接続されていません",
            "总线成员没有连接",
            "Um membro do barramento não tem ligação",
        ],
    ),
    (
        MEMBER_REPEATED,
        [
            "Bus has repeated members",
            "El bus tiene miembros repetidos",
            "Der Bus hat doppelte Mitglieder",
            "Le bus a des membres en double",
            "Il bus ha membri ripetuti",
            "バスにメンバーの重複があります",
            "总线含有重复成员",
            "O barramento tem membros repetidos",
        ],
    ),
    (
        RANGE_INVALID,
        [
            "Bus range is not valid",
            "El intervalo del bus no es válido",
            "Der Busbereich ist ungültig",
            "La plage du bus n'est pas valide",
            "L'intervallo del bus non è valido",
            "バスの範囲が正しくありません",
            "总线范围无效",
            "O intervalo do barramento não é válido",
        ],
    ),
    (
        ON_SIMPLE_NET,
        [
            "Bus is connected to a simple net",
            "Un bus está conectado a una red simple",
            "Ein Bus ist mit einem einfachen Netz verbunden",
            "Un bus est relié à un réseau simple",
            "Un bus è collegato a una rete semplice",
            "バスが単一のネットに接続されています",
            "总线连接到了普通网络",
            "Um barramento está ligado a uma rede simples",
        ],
    ),
    (
        ENTRY_DANGLING,
        [
            "Bus entry names a bus or a member that does not exist",
            "La entrada de bus nombra un bus o un miembro que no existe",
            "Der Buseintrag nennt einen Bus oder ein Mitglied, das nicht existiert",
            "L'entrée de bus nomme un bus ou un membre qui n'existe pas",
            "L'ingresso di bus nomina un bus o un membro che non esiste",
            "バスエントリが存在しないバスまたはメンバーを指しています",
            "总线入口指向不存在的总线或成员",
            "A entrada de barramento indica um barramento ou um membro que não existe",
        ],
    ),
    (
        ALIAS_INVALID,
        [
            "Bus alias is not valid",
            "El alias de bus no es válido",
            "Der Busalias ist ungültig",
            "L'alias de bus n'est pas valide",
            "L'alias di bus non è valido",
            "バスのエイリアスが正しくありません",
            "总线别名无效",
            "O alias de barramento não é válido",
        ],
    ),
];

/// A bus placed on a sheet. Its anchor is the point where the bus is drawn and where its name
/// is written; the entries are drawn from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bus {
    /// The sheet.
    pub sheet: Id,
    /// The anchor.
    pub at: Point,
    /// The name of the bus: a vector, a group or the name of an alias.
    pub spec: String,
}

/// A member taken out of a bus at a point of its sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusEntry {
    /// The bus.
    pub bus: Id,
    /// Where the member leaves the bus; a wire that ends here is on the net of the member.
    pub at: Point,
    /// The name of the member (`D3`, `USB.DP`).
    pub member: String,
}

/// A name for a group of items: the alias `USB` with the items `DP` and `DM` expands like
/// `USB{DP DM}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusAlias {
    /// The name.
    pub name: String,
    /// The items: plain names or vectors.
    pub members: Vec<String>,
}

/// Why a bus name could not be expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecError {
    /// The text is not of any of the forms of a bus.
    Syntax(String),
    /// A vector with a negative number or more than [`MAX_VECTOR`] members.
    Range(String),
    /// Two members of one name.
    Repeated(String),
    /// More than [`MAX_MEMBERS`] members.
    TooMany,
}

impl SpecError {
    /// The code that tells this problem.
    pub fn code(&self) -> &'static str {
        match self {
            SpecError::Syntax(_) | SpecError::TooMany => BAD_SPEC,
            SpecError::Range(_) => RANGE_INVALID,
            SpecError::Repeated(_) => MEMBER_REPEATED,
        }
    }

    /// What is wrong, in English.
    pub fn detail(&self) -> String {
        match self {
            SpecError::Syntax(text) => {
                format!("{text} is not a bus such as D[0..7] or USB{{DP DM}}")
            }
            SpecError::Range(text) => format!("{text} has a range that is negative or too long"),
            SpecError::Repeated(name) => format!("the member {name} appears twice"),
            SpecError::TooMany => format!("more than {MAX_MEMBERS} members"),
        }
    }
}

fn plain_name_ok(name: &str) -> bool {
    !name.is_empty()
        && !name
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '[' | ']' | '{' | '}'))
}

/// The members of a vector `prefix[first..last]`.
fn vector(spec: &str) -> Result<Vec<String>, SpecError> {
    let syntax = || SpecError::Syntax(spec.to_owned());
    let open = spec.find('[').ok_or_else(syntax)?;
    let inner = spec
        .get(open + 1..)
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(syntax)?;
    let prefix = &spec[..open];
    if prefix.is_empty() || !plain_name_ok(prefix) {
        return Err(syntax());
    }
    let (first, last) = inner.split_once("..").ok_or_else(syntax)?;
    let first: i64 = first.trim().parse().map_err(|_| syntax())?;
    let last: i64 = last.trim().parse().map_err(|_| syntax())?;
    if first < 0 || last < 0 || first.abs_diff(last) >= MAX_VECTOR as u64 {
        return Err(SpecError::Range(spec.to_owned()));
    }
    let numbers: Vec<i64> = if first <= last {
        (first..=last).collect()
    } else {
        (last..=first).rev().collect()
    };
    Ok(numbers
        .into_iter()
        .map(|n| format!("{prefix}{n}"))
        .collect())
}

/// The members of one item of a group: a vector or a plain name.
fn item(text: &str) -> Result<Vec<String>, SpecError> {
    if text.contains('[') || text.contains(']') {
        vector(text)
    } else if plain_name_ok(text) {
        Ok(vec![text.to_owned()])
    } else {
        Err(SpecError::Syntax(text.to_owned()))
    }
}

fn group(name: &str, items: &[&str], whole: &str) -> Result<Vec<String>, SpecError> {
    if !plain_name_ok(name) || items.is_empty() {
        return Err(SpecError::Syntax(whole.to_owned()));
    }
    let mut out = Vec::new();
    for text in items {
        for member in item(text)? {
            out.push(format!("{name}{GROUP_SEPARATOR}{member}"));
        }
        if out.len() > MAX_MEMBERS {
            return Err(SpecError::TooMany);
        }
    }
    Ok(out)
}

/// The names of the members of a bus, in a fixed order. `aliases` maps the name of an alias to its
/// items.
pub fn expand_spec(
    spec: &str,
    aliases: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, SpecError> {
    let spec = spec.trim();
    let members = if let Some(open) = spec.find('{') {
        let inner = spec
            .get(open + 1..)
            .and_then(|rest| rest.strip_suffix('}'))
            .ok_or_else(|| SpecError::Syntax(spec.to_owned()))?;
        let items: Vec<&str> = inner.split_whitespace().collect();
        group(spec[..open].trim_end(), &items, spec)?
    } else if spec.contains('[') || spec.contains(']') {
        vector(spec)?
    } else if let Some(items) = aliases.get(spec) {
        let items: Vec<&str> = items.iter().map(String::as_str).collect();
        group(spec, &items, spec)?
    } else {
        return Err(SpecError::Syntax(spec.to_owned()));
    };
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for member in &members {
        if !seen.insert(member.as_str()) {
            return Err(SpecError::Repeated(member.clone()));
        }
    }
    Ok(members)
}

fn make(code: &'static str, severity: Severity, detail: String) -> Diagnostic {
    let title = REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, t)| *t);
    Diagnostic::new(0, severity, format!("{title}: {detail}"))
        .with_code(code)
        .with_category(Category::Format)
        .with_recoverability(if severity == Severity::Error {
            Recoverability::Unrecoverable
        } else {
            Recoverability::Recoverable
        })
}

impl Schematic {
    /// The aliases by name, with their items.
    pub fn alias_map(&self) -> BTreeMap<String, Vec<String>> {
        self.bus_aliases
            .values()
            .map(|a| (a.name.clone(), a.members.clone()))
            .collect()
    }

    /// The members of a bus of the drawing.
    pub fn bus_member_names(&self, bus: Id) -> Result<Vec<String>, SpecError> {
        let bus = self
            .buses
            .get(&bus)
            .ok_or_else(|| SpecError::Syntax(String::new()))?;
        expand_spec(&bus.spec, &self.alias_map())
    }

    /// What the bus entries name: the sheet, the point and the name of the member, for each entry
    /// whose bus and member exist. They act as net labels.
    pub(crate) fn bus_entry_marks(&self) -> Vec<(Id, Point, String)> {
        if self.bus_entries.is_empty() {
            return Vec::new();
        }
        let aliases = self.alias_map();
        let mut members: BTreeMap<Id, Vec<String>> = BTreeMap::new();
        let mut out = Vec::new();
        for entry in self.bus_entries.values() {
            let Some(bus) = self.buses.get(&entry.bus) else {
                continue;
            };
            let names = members
                .entry(entry.bus)
                .or_insert_with(|| expand_spec(&bus.spec, &aliases).unwrap_or_default());
            if names.contains(&entry.member) {
                out.push((bus.sheet, entry.at, entry.member.clone()));
            }
        }
        out
    }

    /// Every problem of the buses, their entries and the aliases, in a fixed order.
    pub fn bus_problems(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        if self.buses.is_empty() && self.bus_entries.is_empty() && self.bus_aliases.is_empty() {
            return out;
        }
        let aliases = self.alias_map();
        let mut alias_names: BTreeSet<&str> = BTreeSet::new();
        for (id, alias) in &self.bus_aliases {
            let name_ok = plain_name_ok(&alias.name);
            if !name_ok || alias.members.is_empty() || !alias_names.insert(alias.name.as_str()) {
                out.push(make(
                    ALIAS_INVALID,
                    Severity::Error,
                    format!(
                        "alias {} ({id}) has no usable name or items, or the name is used twice",
                        alias.name
                    ),
                ));
                continue;
            }
            if let Err(e) = expand_spec(&alias.name, &aliases) {
                out.push(make(
                    ALIAS_INVALID,
                    Severity::Error,
                    format!("alias {}: {}", alias.name, e.detail()),
                ));
            }
        }
        let mut members: BTreeMap<Id, Vec<String>> = BTreeMap::new();
        for (id, bus) in &self.buses {
            if !self.sheets.contains_key(&bus.sheet) {
                out.push(make(
                    DANGLING,
                    Severity::Error,
                    format!("bus {id} is on a sheet that does not exist"),
                ));
            }
            match expand_spec(&bus.spec, &aliases) {
                Ok(names) => {
                    members.insert(*id, names);
                }
                Err(e) => out.push(make(
                    e.code(),
                    Severity::Error,
                    format!("bus {} ({id}): {}", bus.spec, e.detail()),
                )),
            }
        }
        let mut used: BTreeSet<(Id, &str)> = BTreeSet::new();
        for (id, entry) in &self.bus_entries {
            let Some(names) = members.get(&entry.bus) else {
                if !self.buses.contains_key(&entry.bus) {
                    out.push(make(
                        ENTRY_DANGLING,
                        Severity::Error,
                        format!("entry {id} names a bus that does not exist"),
                    ));
                }
                continue;
            };
            if names.contains(&entry.member) {
                used.insert((entry.bus, entry.member.as_str()));
            } else {
                out.push(make(
                    ENTRY_DANGLING,
                    Severity::Error,
                    format!("entry {id}: {} is not a member of the bus", entry.member),
                ));
            }
        }
        for (id, names) in &members {
            let loose: Vec<&str> = names
                .iter()
                .filter(|n| !used.contains(&(*id, n.as_str())))
                .map(String::as_str)
                .collect();
            if loose.is_empty() {
                continue;
            }
            let spec = self.buses.get(id).map_or("", |b| b.spec.as_str());
            let shown: Vec<&str> = loose.iter().take(8).copied().collect();
            let more = loose.len() - shown.len();
            let tail = if more > 0 {
                format!(" and {more} more")
            } else {
                String::new()
            };
            out.push(make(
                MEMBER_UNUSED,
                Severity::Warning,
                format!("bus {spec} ({id}): {}{tail}", shown.join(", ")),
            ));
        }
        out.extend(self.bus_on_simple_nets());
        out
    }

    /// A bus whose anchor is a pin of a part or a point of a wire is joined to a simple net.
    fn bus_on_simple_nets(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        if self.buses.is_empty() {
            return out;
        }
        let mut busy: BTreeSet<(Id, i64, i64)> = BTreeSet::new();
        for (_, sheet, at) in self.pin_positions() {
            busy.insert((sheet, at.x_nm.get(), at.y_nm.get()));
        }
        for wire in self.wires.values() {
            for p in &wire.path {
                busy.insert((wire.sheet, p.x_nm.get(), p.y_nm.get()));
            }
        }
        for (id, bus) in &self.buses {
            if busy.contains(&(bus.sheet, bus.at.x_nm.get(), bus.at.y_nm.get())) {
                out.push(make(
                    ON_SIMPLE_NET,
                    Severity::Error,
                    format!("bus {} ({id}) is at a pin or on a wire", bus.spec),
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marks::bus_members;

    fn none() -> BTreeMap<String, Vec<String>> {
        BTreeMap::new()
    }

    #[test]
    fn a_vector_lists_its_members_up_or_down_and_agrees_with_the_older_function() {
        let up = expand_spec("D[0..3]", &none()).unwrap();
        assert_eq!(up, vec!["D0", "D1", "D2", "D3"]);
        assert_eq!(Some(up), bus_members("D[0..3]"));
        assert_eq!(
            expand_spec(" A[2..0] ", &none()).unwrap(),
            vec!["A2", "A1", "A0"]
        );
        assert_eq!(expand_spec("X[5..5]", &none()).unwrap(), vec!["X5"]);
    }

    #[test]
    fn a_group_prefixes_its_items_and_may_hold_vectors() {
        assert_eq!(
            expand_spec("USB{DP DM}", &none()).unwrap(),
            vec!["USB.DP", "USB.DM"]
        );
        assert_eq!(
            expand_spec("P{D[0..1] CLK}", &none()).unwrap(),
            vec!["P.D0", "P.D1", "P.CLK"]
        );
    }

    #[test]
    fn an_alias_expands_like_the_group_of_its_name() {
        let mut aliases = none();
        aliases.insert(
            "SPI".into(),
            vec!["MOSI".into(), "MISO".into(), "SCK".into()],
        );
        assert_eq!(
            expand_spec("SPI", &aliases).unwrap(),
            vec!["SPI.MOSI", "SPI.MISO", "SPI.SCK"]
        );
        assert!(matches!(
            expand_spec("UART", &aliases),
            Err(SpecError::Syntax(_))
        ));
    }

    #[test]
    fn bad_texts_are_refused_with_the_right_code() {
        for bad in [
            "D", "D[]", "D[1]", "D[a..b]", "[0..3]", "D[0..3", "D[[0..3]", "U{}", "{A B}", "U{A",
            "U{A[}",
        ] {
            let e = expand_spec(bad, &none()).unwrap_err();
            assert_eq!(e.code(), BAD_SPEC, "{bad}");
        }
        for range in ["D[-1..2]", "D[0..300]", "D[0..256]"] {
            let e = expand_spec(range, &none()).unwrap_err();
            assert_eq!(e.code(), RANGE_INVALID, "{range}");
        }
        assert!(expand_spec("D[0..255]", &none()).is_ok());
        let e = expand_spec("U{A A}", &none()).unwrap_err();
        assert_eq!(e, SpecError::Repeated("U.A".into()));
        assert_eq!(e.code(), MEMBER_REPEATED);
    }

    #[test]
    fn a_group_with_too_many_members_is_refused() {
        let items: Vec<String> = (0..20).map(|i| format!("V{i}[0..255]")).collect();
        let spec = format!("G{{{}}}", items.join(" "));
        assert_eq!(expand_spec(&spec, &none()), Err(SpecError::TooMany));
    }

    #[test]
    fn the_titles_are_in_the_registry_and_in_eight_languages() {
        assert_eq!(REGISTRY.len(), TITLES.len());
        for ((code, title), (tcode, row)) in REGISTRY.iter().zip(TITLES.iter()) {
            assert_eq!(code, tcode);
            assert_eq!(*title, row[0]);
            assert!(row.iter().all(|t| !t.is_empty()), "{code}");
            for other in &row[1..6] {
                assert_ne!(*other, row[0], "{code}");
            }
            assert!(!row[5].is_ascii() && !row[6].is_ascii(), "{code}");
        }
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
        for (code, titles) in &TITLES {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| `{code}` |")))
                .unwrap_or_else(|| panic!("{code} has no row of titles"));
            for title in titles {
                assert!(row.contains(title), "{code}: {title}");
            }
        }
    }
}
