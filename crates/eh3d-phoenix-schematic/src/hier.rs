//! The hierarchy of sheets: sheet symbols (a block on a sheet that shows another sheet), the pins
//! of a sheet symbol, the ports of the sheet that is shown, the settings of the expansion, and the
//! checks of all of it (`PHOENIX-HIER-<NNN>`).
//!
//! * a **sheet symbol** ([`SheetBlock`]) is drawn on one sheet and refers to another sheet, the one
//!   it shows. The same sheet may be shown by several symbols with different names: each one is an
//!   instance, and the parts of the sheet are instantiated once per instance;
//! * a **pin** ([`HierPin`]) sits on a sheet symbol, and a **port** ([`SheetPort`]) on the sheet
//!   that is shown; a pin and a port of the same name are one connection;
//! * a sheet that no symbol shows is a top sheet.
//!
//! The expansion of this structure into nets and parts with hierarchical names is in
//! [`crate::hier_expand`]. Nothing here is written when a schematic has no sheet symbol.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_diagnostic::{Category, Diagnostic, Recoverability, Severity};
use eh3d_phoenix_model::{Nm, Point};
use serde::{Deserialize, Serialize};

use crate::model::{Id, Schematic};

/// A sheet symbol, pin or port that refers to something that does not exist.
pub const DANGLING_REF: &str = "PHOENIX-HIER-001";
/// A sheet that contains itself.
pub const SHEET_LOOP: &str = "PHOENIX-HIER-002";
/// A sheet symbol whose name is empty, has a slash or is used twice on its sheet.
pub const BLOCK_NAME: &str = "PHOENIX-HIER-003";
/// A pin of a sheet symbol with no port of its name on the sheet that is shown.
pub const PIN_WITHOUT_PORT: &str = "PHOENIX-HIER-004";
/// A port of a sheet with no pin of its name on a symbol that shows the sheet.
pub const PORT_WITHOUT_PIN: &str = "PHOENIX-HIER-005";
/// Two pins of one sheet symbol, or two ports of one sheet, with the same name.
pub const NAME_REPEATED: &str = "PHOENIX-HIER-006";
/// Two parts with the same reference after the sheets are expanded.
pub const REFDES_REPEATED: &str = "PHOENIX-HIER-007";
/// A hierarchy that is too deep or has too many instances.
pub const TOO_BIG: &str = "PHOENIX-HIER-008";
/// A pin and a port of one name whose directions do not fit.
pub const DIRECTION_MISMATCH: &str = "PHOENIX-HIER-009";

/// The codes of this module with their titles.
pub const REGISTRY: &[(&str, &str)] = &[
    (
        DANGLING_REF,
        "Sheet symbol, pin or port refers to something that does not exist",
    ),
    (SHEET_LOOP, "Sheet contains itself"),
    (
        BLOCK_NAME,
        "Sheet symbol name is empty, repeated on its sheet or has a slash",
    ),
    (
        PIN_WITHOUT_PORT,
        "Pin of a sheet symbol has no port in the sheet it shows",
    ),
    (
        PORT_WITHOUT_PIN,
        "Port of a sheet has no pin on the sheet symbol",
    ),
    (NAME_REPEATED, "Pin or port name is repeated"),
    (
        REFDES_REPEATED,
        "Reference designator repeated after expanding the sheets",
    ),
    (TOO_BIG, "Hierarchy is too deep or has too many instances"),
    (
        DIRECTION_MISMATCH,
        "Pin and port of one name have different directions",
    ),
];

/// The titles in English, Spanish, German, French, Italian, Japanese, Chinese and European
/// Portuguese, in the order of [`REGISTRY`].
pub const TITLES: [(&str, [&str; 8]); 9] = [
    (
        DANGLING_REF,
        [
            "Sheet symbol, pin or port refers to something that does not exist",
            "El símbolo de hoja, el pin o el puerto se refiere a algo que no existe",
            "Blattsymbol, Pin oder Port verweist auf etwas, das nicht existiert",
            "Le symbole de feuille, la broche ou le port renvoie à quelque chose qui n'existe pas",
            "Il simbolo di foglio, il pin o la porta rimanda a qualcosa che non esiste",
            "シートシンボル、ピン、またはポートが存在しないものを参照しています",
            "图纸符号、引脚或端口引用了不存在的对象",
            "O símbolo de folha, o pino ou a porta refere algo que não existe",
        ],
    ),
    (
        SHEET_LOOP,
        [
            "Sheet contains itself",
            "Una hoja se contiene a sí misma",
            "Ein Blatt enthält sich selbst",
            "Une feuille se contient elle-même",
            "Un foglio contiene se stesso",
            "シートが自分自身を含んでいます",
            "图纸包含自身",
            "Uma folha contém-se a si própria",
        ],
    ),
    (
        BLOCK_NAME,
        [
            "Sheet symbol name is empty, repeated on its sheet or has a slash",
            "El nombre del símbolo de hoja está vacío, repetido en su hoja o lleva una barra",
            "Name des Blattsymbols ist leer, auf dem Blatt doppelt oder enthält einen Schrägstrich",
            "Le nom du symbole de feuille est vide, répété sur sa feuille ou contient une barre oblique",
            "Il nome del simbolo di foglio è vuoto, ripetuto nel foglio o contiene una barra",
            "シートシンボル名が空、同じシート上で重複、またはスラッシュを含んでいます",
            "图纸符号名称为空、在图纸上重复或含有斜杠",
            "O nome do símbolo de folha está vazio, repetido na sua folha ou tem uma barra",
        ],
    ),
    (
        PIN_WITHOUT_PORT,
        [
            "Pin of a sheet symbol has no port in the sheet it shows",
            "Un pin del símbolo de hoja no tiene puerto en la hoja que muestra",
            "Ein Pin des Blattsymbols hat keinen Port auf dem gezeigten Blatt",
            "Une broche du symbole de feuille n'a pas de port dans la feuille qu'il montre",
            "Un pin del simbolo di foglio non ha porta nel foglio che mostra",
            "シートシンボルのピンに対応するポートが参照先のシートにありません",
            "图纸符号的引脚在其引用的图纸中没有对应端口",
            "Um pino do símbolo de folha não tem porta na folha que mostra",
        ],
    ),
    (
        PORT_WITHOUT_PIN,
        [
            "Port of a sheet has no pin on the sheet symbol",
            "Un puerto de la hoja no tiene pin en el símbolo de hoja",
            "Ein Port des Blatts hat keinen Pin am Blattsymbol",
            "Un port de la feuille n'a pas de broche sur le symbole de feuille",
            "Una porta del foglio non ha pin sul simbolo di foglio",
            "シートのポートに対応するピンがシートシンボルにありません",
            "图纸的端口在图纸符号上没有对应引脚",
            "Uma porta da folha não tem pino no símbolo de folha",
        ],
    ),
    (
        NAME_REPEATED,
        [
            "Pin or port name is repeated",
            "Nombre de pin o de puerto repetido",
            "Pin- oder Portname doppelt",
            "Nom de broche ou de port répété",
            "Nome di pin o di porta ripetuto",
            "ピン名またはポート名が重複しています",
            "引脚或端口名称重复",
            "Nome de pino ou de porta repetido",
        ],
    ),
    (
        REFDES_REPEATED,
        [
            "Reference designator repeated after expanding the sheets",
            "Designador de referencia repetido al expandir las hojas",
            "Bauteilkennung nach dem Auffalten der Blätter doppelt",
            "Référence en double après le dépliage des feuilles",
            "Designatore di riferimento ripetuto dopo l'espansione dei fogli",
            "シートの展開後に参照指定子が重複しています",
            "展开图纸后位号重复",
            "Designador de referência repetido depois de expandir as folhas",
        ],
    ),
    (
        TOO_BIG,
        [
            "Hierarchy is too deep or has too many instances",
            "La jerarquía es demasiado profunda o tiene demasiadas instancias",
            "Die Hierarchie ist zu tief oder hat zu viele Instanzen",
            "La hiérarchie est trop profonde ou a trop d'instances",
            "La gerarchia è troppo profonda o ha troppe istanze",
            "階層が深すぎるか、インスタンスが多すぎます",
            "层次过深或实例过多",
            "A hierarquia é demasiado profunda ou tem demasiadas instâncias",
        ],
    ),
    (
        DIRECTION_MISMATCH,
        [
            "Pin and port of one name have different directions",
            "El pin y el puerto del mismo nombre tienen direcciones distintas",
            "Pin und Port gleichen Namens haben verschiedene Richtungen",
            "La broche et le port du même nom ont des directions différentes",
            "Il pin e la porta con lo stesso nome hanno direzioni diverse",
            "同名のピンとポートの方向が異なります",
            "同名的引脚和端口方向不同",
            "O pino e a porta com o mesmo nome têm direções diferentes",
        ],
    ),
];

/// The title of a code of the hierarchy or of the buses in a language, given as the column of the
/// eight languages (0 English, 1 Spanish, 2 German, 3 French, 4 Italian, 5 Japanese, 6 Chinese,
/// 7 European Portuguese). Empty for a code that is not one of them.
pub fn code_title(code: &str, column: usize) -> &'static str {
    TITLES
        .iter()
        .chain(crate::bus::TITLES.iter())
        .find(|(c, _)| *c == code)
        .and_then(|(_, row)| row.get(column))
        .copied()
        .unwrap_or("")
}

/// The direction of a pin of a sheet symbol or of a port of a sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortDir {
    /// A signal comes in.
    Input,
    /// A signal goes out.
    Output,
    /// Both ways.
    Bidirectional,
    /// No direction is given.
    Passive,
}

impl PortDir {
    /// Every direction, in a fixed order.
    pub const ALL: [PortDir; 4] = [
        PortDir::Input,
        PortDir::Output,
        PortDir::Bidirectional,
        PortDir::Passive,
    ];

    /// The name used in files.
    pub fn as_str(self) -> &'static str {
        match self {
            PortDir::Input => "input",
            PortDir::Output => "output",
            PortDir::Bidirectional => "bidirectional",
            PortDir::Passive => "passive",
        }
    }

    /// Reads the name used in files.
    pub fn from_name(name: &str) -> Option<PortDir> {
        PortDir::ALL.into_iter().find(|d| d.as_str() == name)
    }

    /// Whether a pin of this direction fits a port of the other: equal, or one of them is
    /// bidirectional or passive.
    pub fn fits(self, other: PortDir) -> bool {
        self == other
            || matches!(self, PortDir::Bidirectional | PortDir::Passive)
            || matches!(other, PortDir::Bidirectional | PortDir::Passive)
    }
}

/// A sheet symbol: a block drawn on one sheet that shows another sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SheetBlock {
    /// The sheet it is drawn on.
    pub sheet: Id,
    /// The sheet it shows.
    pub child: Id,
    /// The name of the instance (`PSU1`); it is part of the reference of every part of the shown
    /// sheet and of the name of its local nets.
    pub name: String,
    /// The corner with the smallest coordinates.
    pub at: Point,
    /// The width of the block.
    pub width: Nm,
    /// The height of the block.
    pub height: Nm,
}

/// A pin of a sheet symbol, at a point of the sheet the symbol is drawn on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HierPin {
    /// The sheet symbol.
    pub block: Id,
    /// The name; the port of the shown sheet with the same name is the same connection.
    pub name: String,
    /// The direction.
    pub dir: PortDir,
    /// Where it is, on the sheet of the symbol.
    pub at: Point,
}

/// A port of a sheet that other sheets show, at a point of that sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SheetPort {
    /// The sheet.
    pub sheet: Id,
    /// The name; the pin of a symbol with the same name is the same connection.
    pub name: String,
    /// The direction.
    pub dir: PortDir,
    /// Where it is.
    pub at: Point,
}

/// How the reference of a part of an instance is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefdesStyle {
    /// The instance first: `U1/R1`.
    Prefix,
    /// The instance last: `R1_U1`.
    Suffix,
}

/// The settings of the expansion, read from `config/hierarchy.cfg` (which is also the default).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HierConfig {
    /// How the reference of a part of an instance is made.
    pub refdes_style: RefdesStyle,
    /// What joins the instance and the reference.
    pub refdes_separator: String,
    /// What separates the parts of the name of a net.
    pub net_separator: String,
    /// Whether a net label belongs to its own sheet.
    pub local_labels: bool,
    /// The names that are global even as labels.
    pub global_nets: BTreeSet<String>,
    /// The deepest nesting.
    pub max_depth: usize,
    /// The most instances.
    pub max_instances: usize,
}

/// The settings file of the repository, which is the default.
const BUILT_IN: &str = include_str!("../../../config/hierarchy.cfg");

/// The largest settings file that is read, in bytes.
pub const MAX_CONFIG_BYTES: usize = 1 << 20;

impl Default for HierConfig {
    /// The settings file of the repository (`config/hierarchy.cfg`).
    fn default() -> Self {
        let mut config = HierConfig::empty();
        if config.apply(BUILT_IN).is_err() {
            return HierConfig::empty();
        }
        config
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn separator_ok(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= 8
        && !value.chars().any(|c| c.is_control() || c.is_whitespace())
}

impl HierConfig {
    fn empty() -> HierConfig {
        HierConfig {
            refdes_style: RefdesStyle::Prefix,
            refdes_separator: "/".to_owned(),
            net_separator: "/".to_owned(),
            local_labels: true,
            global_nets: BTreeSet::new(),
            max_depth: 1,
            max_instances: 1,
        }
    }

    /// Reads a settings text on top of the default: a line that is left out keeps the default.
    /// Blank lines and lines starting with `#` are ignored. The error is the message to show.
    pub fn parse(text: &str) -> Result<HierConfig, String> {
        let mut config = HierConfig::default();
        config.apply(text)?;
        Ok(config)
    }

    fn apply(&mut self, text: &str) -> Result<(), String> {
        if text.len() > MAX_CONFIG_BYTES {
            return Err(format!(
                "the file is {} bytes, the limit is {MAX_CONFIG_BYTES}",
                text.len()
            ));
        }
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| format!("line {line}: not key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            let bad = || format!("line {line}: the value of {key} is not valid");
            match key {
                "refdes_style" => {
                    self.refdes_style = match value {
                        "prefix" => RefdesStyle::Prefix,
                        "suffix" => RefdesStyle::Suffix,
                        _ => return Err(bad()),
                    };
                }
                "refdes_separator" => {
                    if !separator_ok(value) {
                        return Err(bad());
                    }
                    self.refdes_separator = value.to_owned();
                }
                "net_separator" => {
                    if !separator_ok(value) {
                        return Err(bad());
                    }
                    self.net_separator = value.to_owned();
                }
                "local_labels" => self.local_labels = parse_bool(value).ok_or_else(bad)?,
                "global_nets" => {
                    self.global_nets = value
                        .split(',')
                        .map(str::trim)
                        .filter(|n| !n.is_empty())
                        .map(str::to_owned)
                        .collect();
                }
                "max_depth" => {
                    let n: usize = value.parse().map_err(|_| bad())?;
                    if !(1..=1024).contains(&n) {
                        return Err(bad());
                    }
                    self.max_depth = n;
                }
                "max_instances" => {
                    let n: usize = value.parse().map_err(|_| bad())?;
                    if !(1..=1_000_000).contains(&n) {
                        return Err(bad());
                    }
                    self.max_instances = n;
                }
                _ => return Err(format!("line {line}: unknown key {key}")),
            }
        }
        Ok(())
    }

    /// Whether a net name is global: a power port name, or one of the names of `global_nets`.
    pub fn is_global(&self, name: &str) -> bool {
        self.global_nets.contains(name)
    }
}

/// One instance of a sheet: a top sheet, or a sheet shown by a sheet symbol (once per symbol, and
/// per instance of the sheet that holds the symbol).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// The hierarchical name: empty for a top sheet, `/U1` for a sheet shown by the symbol U1,
    /// `/U1/FILT` for one shown by the symbol FILT inside it (with the separator of the settings).
    pub name: String,
    /// The names of the sheet symbols from the top sheet.
    pub path: Vec<String>,
    /// The sheet that is drawn.
    pub sheet: Id,
    /// The sheet symbol that shows it, for an instance that is not a top sheet.
    pub block: Option<Id>,
    /// The instance that holds the symbol (an index into [`Hierarchy::instances`]).
    pub parent: Option<usize>,
}

/// The tree of sheets, in a fixed order: the top sheets by id, and under each the sheets its
/// symbols show, by name of the symbol.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Hierarchy {
    /// The top sheets: those no symbol shows.
    pub roots: Vec<Id>,
    /// The instances, each one after the instance that holds it.
    pub instances: Vec<Instance>,
    /// Loops and limits found while walking the tree.
    pub problems: Vec<Diagnostic>,
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

pub(crate) fn hier_diagnostic(
    code: &'static str,
    severity: Severity,
    detail: String,
) -> Diagnostic {
    make(code, severity, detail)
}

struct Walk<'a> {
    cfg: &'a HierConfig,
    by_sheet: BTreeMap<Id, Vec<(Id, String, Id)>>,
    out: Hierarchy,
    stack: Vec<Id>,
    limit_said: bool,
}

impl Walk<'_> {
    fn visit(&mut self, sheet: Id, block: Option<Id>, parent: Option<usize>, path: Vec<String>) {
        if self.out.instances.len() >= self.cfg.max_instances || path.len() > self.cfg.max_depth {
            if !self.limit_said {
                self.limit_said = true;
                self.out.problems.push(make(
                    TOO_BIG,
                    Severity::Error,
                    format!(
                        "more than {} levels or {} instances",
                        self.cfg.max_depth, self.cfg.max_instances
                    ),
                ));
            }
            return;
        }
        let name = path.iter().fold(String::new(), |acc, p| {
            format!("{acc}{}{p}", self.cfg.net_separator)
        });
        let index = self.out.instances.len();
        self.out.instances.push(Instance {
            name,
            path: path.clone(),
            sheet,
            block,
            parent,
        });
        self.stack.push(sheet);
        let children = self.by_sheet.get(&sheet).cloned().unwrap_or_default();
        for (block_id, block_name, child) in children {
            if self.stack.contains(&child) {
                self.out.problems.push(make(
                    SHEET_LOOP,
                    Severity::Error,
                    format!("the symbol {block_name} ({block_id}) shows a sheet that holds it"),
                ));
                continue;
            }
            let mut next = path.clone();
            next.push(block_name);
            self.visit(child, Some(block_id), Some(index), next);
        }
        self.stack.pop();
    }
}

impl Schematic {
    /// True when the schematic has a sheet symbol.
    pub fn has_hierarchy(&self) -> bool {
        !self.sheet_blocks.is_empty()
    }

    /// True when the schematic holds anything of the hierarchy.
    pub(crate) fn has_hier_data(&self) -> bool {
        !self.sheet_blocks.is_empty() || !self.hier_pins.is_empty() || !self.sheet_ports.is_empty()
    }

    /// The tree of sheets. Without a sheet symbol every sheet is a top sheet with one instance.
    pub fn hierarchy(&self, cfg: &HierConfig) -> Hierarchy {
        let mut by_sheet: BTreeMap<Id, Vec<(Id, String, Id)>> = BTreeMap::new();
        for (id, block) in &self.sheet_blocks {
            if self.sheets.contains_key(&block.sheet) && self.sheets.contains_key(&block.child) {
                by_sheet.entry(block.sheet).or_default().push((
                    *id,
                    block.name.clone(),
                    block.child,
                ));
            }
        }
        for list in by_sheet.values_mut() {
            list.sort_by_key(|(id, name, _)| (name.clone(), *id));
        }
        let shown: BTreeSet<Id> = by_sheet.values().flatten().map(|(_, _, c)| *c).collect();
        let roots: Vec<Id> = self
            .sheets
            .keys()
            .filter(|id| !shown.contains(*id))
            .copied()
            .collect();
        let mut walk = Walk {
            cfg,
            by_sheet,
            out: Hierarchy {
                roots: roots.clone(),
                ..Hierarchy::default()
            },
            stack: Vec::new(),
            limit_said: false,
        };
        for root in roots {
            walk.visit(root, None, None, Vec::new());
        }
        let mut out = walk.out;
        let reached: BTreeSet<Id> = out.instances.iter().map(|i| i.sheet).collect();
        for (id, sheet) in &self.sheets {
            if !reached.contains(id)
                && !out
                    .problems
                    .iter()
                    .any(|d| d.code.as_deref() == Some(TOO_BIG))
            {
                out.problems.push(make(
                    SHEET_LOOP,
                    Severity::Error,
                    format!(
                        "the sheet {} ({id}) is in a loop of sheet symbols",
                        sheet.name
                    ),
                ));
            }
        }
        out
    }

    /// The ports of a sheet by name.
    pub(crate) fn ports_of(&self, sheet: Id) -> BTreeMap<&str, &SheetPort> {
        self.sheet_ports
            .values()
            .filter(|p| p.sheet == sheet)
            .map(|p| (p.name.as_str(), p))
            .collect()
    }

    /// Every problem of the sheet symbols, pins and ports, in a fixed order.
    pub fn hier_problems(&self, cfg: &HierConfig) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        if !self.has_hier_data() {
            return out;
        }
        let mut names: BTreeMap<(Id, &str), Id> = BTreeMap::new();
        for (id, block) in &self.sheet_blocks {
            for (what, sheet) in [("is drawn on", block.sheet), ("shows", block.child)] {
                if !self.sheets.contains_key(&sheet) {
                    out.push(make(
                        DANGLING_REF,
                        Severity::Error,
                        format!(
                            "the sheet symbol {} ({id}) {what} a sheet that does not exist",
                            block.name
                        ),
                    ));
                }
            }
            let name = block.name.trim();
            if name.is_empty()
                || name != block.name
                || name.contains('/')
                || name.chars().any(char::is_control)
            {
                out.push(make(
                    BLOCK_NAME,
                    Severity::Error,
                    format!("the sheet symbol {id} is called \"{}\"", block.name),
                ));
            } else if let Some(first) = names.insert((block.sheet, block.name.as_str()), *id) {
                out.push(make(
                    BLOCK_NAME,
                    Severity::Error,
                    format!("{} is the name of {first} and {id}", block.name),
                ));
            }
        }
        let mut pin_names: BTreeSet<(Id, &str)> = BTreeSet::new();
        for (id, pin) in &self.hier_pins {
            if !self.sheet_blocks.contains_key(&pin.block) {
                out.push(make(
                    DANGLING_REF,
                    Severity::Error,
                    format!(
                        "the pin {} ({id}) is on a sheet symbol that does not exist",
                        pin.name
                    ),
                ));
            } else if !pin_names.insert((pin.block, pin.name.as_str())) {
                out.push(make(
                    NAME_REPEATED,
                    Severity::Error,
                    format!(
                        "the pin {} is twice on the sheet symbol {}",
                        pin.name, pin.block
                    ),
                ));
            }
        }
        let mut port_names: BTreeSet<(Id, &str)> = BTreeSet::new();
        for (id, port) in &self.sheet_ports {
            if !self.sheets.contains_key(&port.sheet) {
                out.push(make(
                    DANGLING_REF,
                    Severity::Error,
                    format!(
                        "the port {} ({id}) is on a sheet that does not exist",
                        port.name
                    ),
                ));
            } else if !port_names.insert((port.sheet, port.name.as_str())) {
                out.push(make(
                    NAME_REPEATED,
                    Severity::Error,
                    format!(
                        "the port {} is twice on the sheet {}",
                        port.name, port.sheet
                    ),
                ));
            }
        }
        for (block_id, block) in &self.sheet_blocks {
            if !self.sheets.contains_key(&block.child) {
                continue;
            }
            let ports = self.ports_of(block.child);
            let pins: BTreeMap<&str, &HierPin> = self
                .hier_pins
                .values()
                .filter(|p| p.block == *block_id)
                .map(|p| (p.name.as_str(), p))
                .collect();
            for (name, pin) in &pins {
                match ports.get(name) {
                    None => out.push(make(
                        PIN_WITHOUT_PORT,
                        Severity::Error,
                        format!(
                            "the pin {name} of the sheet symbol {} has no port",
                            block.name
                        ),
                    )),
                    Some(port) if !pin.dir.fits(port.dir) => out.push(make(
                        DIRECTION_MISMATCH,
                        Severity::Warning,
                        format!(
                            "{name} of {} is {} and the port is {}",
                            block.name,
                            pin.dir.as_str(),
                            port.dir.as_str()
                        ),
                    )),
                    Some(_) => {}
                }
            }
            for name in ports.keys() {
                if !pins.contains_key(name) {
                    out.push(make(
                        PORT_WITHOUT_PIN,
                        Severity::Warning,
                        format!(
                            "the port {name} has no pin on the sheet symbol {}",
                            block.name
                        ),
                    ));
                }
            }
        }
        out.extend(self.hierarchy(cfg).problems);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_settings_are_the_file_of_the_repository() {
        let c = HierConfig::default();
        assert_eq!(c.refdes_style, RefdesStyle::Prefix);
        assert_eq!(c.refdes_separator, "/");
        assert_eq!(c.net_separator, "/");
        assert!(c.local_labels);
        assert!(c.is_global("GND") && c.is_global("+3V3") && !c.is_global("SDA"));
        assert_eq!((c.max_depth, c.max_instances), (16, 4096));
    }

    #[test]
    fn a_settings_text_changes_only_what_it_names() {
        let c = HierConfig::parse(
            "refdes_style = suffix\nrefdes_separator = _\nlocal_labels = false\n",
        )
        .unwrap();
        assert_eq!(c.refdes_style, RefdesStyle::Suffix);
        assert_eq!(c.refdes_separator, "_");
        assert!(!c.local_labels);
        assert_eq!(c.net_separator, "/");
        for bad in [
            "refdes_style = middle",
            "refdes_separator =",
            "net_separator = a b",
            "local_labels = maybe",
            "max_depth = 0",
            "max_instances = 2000000",
            "other = 1",
            "no equals sign",
        ] {
            assert!(HierConfig::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn directions_have_names_and_fit_as_documented() {
        for d in PortDir::ALL {
            assert_eq!(PortDir::from_name(d.as_str()), Some(d));
        }
        assert_eq!(PortDir::from_name("sideways"), None);
        assert!(PortDir::Input.fits(PortDir::Input));
        assert!(PortDir::Input.fits(PortDir::Bidirectional));
        assert!(PortDir::Passive.fits(PortDir::Output));
        assert!(!PortDir::Input.fits(PortDir::Output));
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
            assert!(
                !row[7].contains("arquivo") && !row[7].contains("tela"),
                "{code}"
            );
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
