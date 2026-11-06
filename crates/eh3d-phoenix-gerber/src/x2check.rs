// SPDX-License-Identifier: GPL-3.0-or-later
//! A basic check of the structure of Gerber X2 and Excellon files, with the stable codes
//! `PHOENIX-GX2-<NNN>`.
//!
//! What is checked, for a Gerber file: the file ends with `M02` and nothing follows it; the
//! format and the units come before the first drawing command; an aperture that is selected was
//! defined and no aperture is defined twice or below `D10`; a region is opened and closed; a
//! `TD` deletes an attribute that was set; the attribute commands have a name, characters that a
//! field may hold and, for the standard names this suite writes, the shape of their values; the
//! file has `.FileFunction` (an error when missing) and the other file attributes this suite
//! writes (a warning when missing). For a drill file: the same for the end command (`M30`), the
//! tools and the attributes, which are comments that begin with `#@!`.
//!
//! This is a subset chosen for this project, not the whole specification, and no manufacturer's
//! tool has been used to compare it.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_export::x2::{DRILL_FUNCTIONS, GERBER_FUNCTIONS};

use crate::codes::{Finding, Severity};

/// The end command is missing, is not the last command, or something follows it.
pub const END_COMMAND: &str = "PHOENIX-GX2-001";
/// A required file attribute (`.FileFunction`) is missing.
pub const REQUIRED_MISSING: &str = "PHOENIX-GX2-002";
/// A recommended file attribute is missing.
pub const RECOMMENDED_MISSING: &str = "PHOENIX-GX2-003";
/// An aperture or a tool is used without a definition.
pub const UNDEFINED_APERTURE: &str = "PHOENIX-GX2-004";
/// `TD` deletes an attribute that was not set.
pub const DELETE_WITHOUT_SET: &str = "PHOENIX-GX2-005";
/// An attribute has no name, or a value of a shape its name does not allow.
pub const ATTRIBUTE_INVALID: &str = "PHOENIX-GX2-006";
/// A character that an attribute field may not hold.
pub const FIELD_CHARACTER: &str = "PHOENIX-GX2-007";
/// A drawing command comes before the coordinate format and the units.
pub const DRAW_BEFORE_FORMAT: &str = "PHOENIX-GX2-008";
/// A region is opened and not closed, or closed and not opened.
pub const REGION_STRUCTURE: &str = "PHOENIX-GX2-009";
/// An aperture is defined twice or numbered below 10.
pub const APERTURE_DEFINITION: &str = "PHOENIX-GX2-010";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        END_COMMAND,
        [
            "End command missing or not last",
            "Falta el comando de fin o no es el último",
            "Der Endbefehl fehlt oder steht nicht am Schluss",
            "Commande de fin absente ou pas en dernier",
            "Comando di fine mancante o non ultimo",
            "終了コマンドがないか最後ではありません",
            "缺少结束命令或结束命令不在最后",
            "O comando de fim falta ou não é o último",
        ],
    ),
    (
        REQUIRED_MISSING,
        [
            "Required file attribute missing",
            "Falta un atributo de fichero obligatorio",
            "Ein erforderliches Dateiattribut fehlt",
            "Attribut de fichier obligatoire absent",
            "Attributo di file obbligatorio mancante",
            "必須のファイル属性がありません",
            "缺少必需的文件属性",
            "Falta um atributo de ficheiro obrigatório",
        ],
    ),
    (
        RECOMMENDED_MISSING,
        [
            "Recommended file attribute missing",
            "Falta un atributo de fichero recomendado",
            "Ein empfohlenes Dateiattribut fehlt",
            "Attribut de fichier recommandé absent",
            "Attributo di file raccomandato mancante",
            "推奨されるファイル属性がありません",
            "缺少建议的文件属性",
            "Falta um atributo de ficheiro recomendado",
        ],
    ),
    (
        UNDEFINED_APERTURE,
        [
            "Aperture or tool used without a definition",
            "Apertura o herramienta usada sin definición",
            "Blende oder Werkzeug ohne Definition verwendet",
            "Ouverture ou outil utilisé sans définition",
            "Apertura o utensile usato senza definizione",
            "定義のないアパーチャまたはツールが使われています",
            "使用了未定义的光圈或刀具",
            "Abertura ou ferramenta usada sem definição",
        ],
    ),
    (
        DELETE_WITHOUT_SET,
        [
            "Attribute deleted that was not set",
            "Atributo borrado sin haber sido definido",
            "Attribut gelöscht, das nicht gesetzt war",
            "Attribut supprimé sans avoir été défini",
            "Attributo eliminato senza essere stato impostato",
            "設定されていない属性を削除しています",
            "删除了未设置的属性",
            "Atributo eliminado sem ter sido definido",
        ],
    ),
    (
        ATTRIBUTE_INVALID,
        [
            "Attribute name or value not valid",
            "Nombre o valor de atributo no válido",
            "Attributname oder -wert ungültig",
            "Nom ou valeur d'attribut non valide",
            "Nome o valore dell'attributo non valido",
            "属性の名前または値が無効です",
            "属性名称或值无效",
            "Nome ou valor de atributo não válido",
        ],
    ),
    (
        FIELD_CHARACTER,
        [
            "Character not allowed in an attribute field",
            "Carácter no permitido en un campo de atributo",
            "Zeichen im Attributfeld nicht erlaubt",
            "Caractère non autorisé dans un champ d'attribut",
            "Carattere non consentito in un campo dell'attributo",
            "属性フィールドに使えない文字があります",
            "属性字段中含有不允许的字符",
            "Carácter não permitido num campo de atributo",
        ],
    ),
    (
        DRAW_BEFORE_FORMAT,
        [
            "Drawing command before the format and units",
            "Orden de dibujo antes del formato y las unidades",
            "Zeichenbefehl vor Format und Einheiten",
            "Commande de tracé avant le format et les unités",
            "Comando di disegno prima di formato e unità",
            "フォーマットと単位の前に描画コマンドがあります",
            "在格式和单位之前出现绘图命令",
            "Comando de desenho antes do formato e das unidades",
        ],
    ),
    (
        REGION_STRUCTURE,
        [
            "Region not opened or not closed",
            "Región sin abrir o sin cerrar",
            "Region nicht geöffnet oder nicht geschlossen",
            "Région non ouverte ou non fermée",
            "Regione non aperta o non chiusa",
            "リージョンが開始または終了されていません",
            "区域未开始或未结束",
            "Região não aberta ou não fechada",
        ],
    ),
    (
        APERTURE_DEFINITION,
        [
            "Aperture defined twice or numbered below 10",
            "Apertura definida dos veces o con número menor que 10",
            "Blende doppelt definiert oder Nummer unter 10",
            "Ouverture définie deux fois ou numérotée en dessous de 10",
            "Apertura definita due volte o con numero inferiore a 10",
            "アパーチャが二重に定義されているか、番号が10未満です",
            "光圈被重复定义或编号小于10",
            "Abertura definida duas vezes ou com número inferior a 10",
        ],
    ),
];

/// The values of `.FileFunction` that the specification of X2 names as its first value.
const FILE_FUNCTIONS: &[&str] = &[
    "Copper",
    "Plated",
    "NonPlated",
    "Profile",
    "Soldermask",
    "Legend",
    "Component",
    "Paste",
    "Glue",
    "Carbonmask",
    "Goldmask",
    "Heatsinkmask",
    "Peelablemask",
    "Silvermask",
    "Tinmask",
    "Depthrout",
    "Vcut",
    "Viafill",
    "Pads",
    "Other",
    "Drillmap",
    "FabricationDrawing",
    "Vcutmap",
    "AssemblyDrawing",
    "ArrayDrawing",
    "OtherDrawing",
];

const FILE_NAMES: &[&str] = &[
    ".FileFunction",
    ".FilePolarity",
    ".Part",
    ".ProjectId",
    ".GenerationSoftware",
    ".SameCoordinates",
    ".CreationDate",
    ".MD5",
];
const APERTURE_NAMES: &[&str] = &[".AperFunction"];
const OBJECT_NAMES: &[&str] = &[
    ".N", ".P", ".C", ".CRot", ".CMfr", ".CMPN", ".CVal", ".CMnt", ".CFtp", ".CPgN", ".CPgD",
    ".CHgt", ".CLbN", ".CLbD", ".CSup",
];

/// True when the findings hold an error.
pub fn has_errors(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.severity == Severity::Error)
}

fn is_top_or_bottom(value: Option<&&str>) -> bool {
    matches!(value, Some(&"Top") | Some(&"Bot"))
}

fn is_number(value: Option<&&str>) -> bool {
    value.is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
}

/// The first character of a field that a field may not hold, with the reason; `None` when the
/// field is fine. A backslash must begin a `\uXXXX` escape.
fn field_problem(field: &str) -> Option<String> {
    let bytes = field.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' {
            let escape = field.get(i + 1..i + 6).is_some_and(|rest| {
                rest.starts_with('u') && rest[1..].bytes().all(|h| h.is_ascii_hexdigit())
            });
            if !escape {
                return Some("a backslash that does not begin a \\uXXXX escape".to_owned());
            }
            i += 6;
            continue;
        }
        if !(b' '..=b'~').contains(&b) {
            return Some(match field[i..].chars().next() {
                Some(c) => format!("the character U+{:04X}", u32::from(c)),
                None => "a character".to_owned(),
            });
        }
        i += 1;
    }
    None
}

/// What shape the values of `.FileFunction` have, or why they do not fit.
fn file_function_problem(values: &[&str]) -> Option<(Severity, String)> {
    let first = *values.first()?;
    let wrong = |what: &str| Some((Severity::Error, format!(".FileFunction {first}: {what}")));
    match first {
        "Copper" => {
            let layer_ok = values
                .get(1)
                .is_some_and(|v| v.strip_prefix('L').is_some_and(|n| is_number(Some(&n))));
            let kind_ok = matches!(values.get(2), Some(&"Top") | Some(&"Inr") | Some(&"Bot"));
            if layer_ok && kind_ok {
                None
            } else {
                wrong("expected L<number> and Top, Inr or Bot")
            }
        }
        "Plated" => {
            if is_number(values.get(1))
                && is_number(values.get(2))
                && matches!(
                    values.get(3),
                    Some(&"PTH") | Some(&"Blind") | Some(&"Buried")
                )
            {
                None
            } else {
                wrong("expected two layer numbers and PTH, Blind or Buried")
            }
        }
        "NonPlated" => {
            if is_number(values.get(1))
                && is_number(values.get(2))
                && values.get(3) == Some(&"NPTH")
            {
                None
            } else {
                wrong("expected two layer numbers and NPTH")
            }
        }
        "Soldermask" | "Legend" | "Paste" | "Glue" | "Carbonmask" | "Goldmask" | "Heatsinkmask"
        | "Peelablemask" | "Silvermask" | "Tinmask" => {
            if is_top_or_bottom(values.get(1)) {
                None
            } else {
                wrong("expected Top or Bot")
            }
        }
        "Profile" => {
            if matches!(values.get(1), Some(&"P") | Some(&"NP")) {
                None
            } else {
                wrong("expected P or NP")
            }
        }
        other if FILE_FUNCTIONS.contains(&other) => None,
        other => Some((
            Severity::Warning,
            format!(".FileFunction {other} is not one this check knows"),
        )),
    }
}

/// Checks one attribute command and pushes what it finds.
fn check_attribute(
    out: &mut Vec<Finding>,
    line: usize,
    kind: &str,
    name: &str,
    values: &[&str],
    drill: bool,
) {
    let error = |out: &mut Vec<Finding>, code, severity, detail: String| {
        out.push(Finding::new(code, severity, line, detail));
    };
    if name.is_empty() {
        error(
            out,
            ATTRIBUTE_INVALID,
            Severity::Error,
            format!("{kind} without a name"),
        );
        return;
    }
    for field in std::iter::once(&name).chain(values.iter()) {
        if let Some(problem) = field_problem(field) {
            error(
                out,
                FIELD_CHARACTER,
                Severity::Error,
                format!("{kind}{name}: {problem}"),
            );
        }
    }
    let home = if FILE_NAMES.contains(&name) {
        Some("TF")
    } else if APERTURE_NAMES.contains(&name) {
        Some("TA")
    } else if OBJECT_NAMES.contains(&name) {
        Some("TO")
    } else {
        None
    };
    if let Some(home) = home {
        if home != kind {
            error(
                out,
                ATTRIBUTE_INVALID,
                Severity::Error,
                format!("{name} belongs to {home}, not to {kind}"),
            );
            return;
        }
    }
    let problem: Option<(Severity, String)> = match name {
        ".FileFunction" => file_function_problem(values),
        ".FilePolarity" => (!matches!(values, ["Positive"] | ["Negative"])).then(|| {
            (
                Severity::Error,
                ".FilePolarity is Positive or Negative".to_owned(),
            )
        }),
        ".Part" => (!values.first().is_some_and(|v| {
            matches!(
                *v,
                "Single" | "Array" | "FabricationPanel" | "Coupon" | "Other"
            )
        }))
        .then(|| {
            (
                Severity::Error,
                ".Part is Single, Array, FabricationPanel, Coupon or Other".to_owned(),
            )
        }),
        ".ProjectId" => (values.len() != 3).then(|| {
            (
                Severity::Error,
                ".ProjectId has a name, a UUID and a revision".to_owned(),
            )
        }),
        ".GenerationSoftware" => (values.len() < 2).then(|| {
            (
                Severity::Error,
                ".GenerationSoftware has a vendor and an application".to_owned(),
            )
        }),
        ".AperFunction" => values.first().and_then(|first| {
            let known = if drill {
                matches!(*first, "Plated" | "NonPlated")
                    && values.get(2).is_some_and(|f| DRILL_FUNCTIONS.contains(f))
            } else {
                GERBER_FUNCTIONS.contains(first)
            };
            (!known).then(|| {
                (
                    Severity::Warning,
                    format!(
                        ".AperFunction {} is not one this check knows",
                        values.join(",")
                    ),
                )
            })
        }),
        ".N" | ".C" => values
            .is_empty()
            .then(|| (Severity::Error, format!("{name} needs a value"))),
        ".P" => (values.len() < 2).then(|| {
            (
                Severity::Error,
                ".P has a reference designator and a pin".to_owned(),
            )
        }),
        _ => None,
    };
    if let Some((severity, detail)) = problem {
        error(out, ATTRIBUTE_INVALID, severity, detail);
    }
}

/// The attributes still to be checked at the end of a file: what the file set, by name.
type FileAttributes = BTreeMap<String, Vec<String>>;

fn check_file_attributes(out: &mut Vec<Finding>, attributes: &FileAttributes, drill: bool) {
    let Some(function) = attributes.get(".FileFunction") else {
        out.push(Finding::new(
            REQUIRED_MISSING,
            Severity::Error,
            0,
            ".FileFunction",
        ));
        return;
    };
    let mut wanted = vec![".GenerationSoftware", ".Part"];
    let has_polarity = function
        .first()
        .is_some_and(|f| matches!(f.as_str(), "Copper" | "Soldermask" | "Legend" | "Paste"));
    if !drill && has_polarity {
        wanted.push(".FilePolarity");
    }
    for name in wanted {
        if !attributes.contains_key(name) {
            out.push(Finding::new(
                RECOMMENDED_MISSING,
                Severity::Warning,
                0,
                name,
            ));
        }
    }
}

/// The attributes set and not yet deleted.
#[derive(Default)]
struct InForce {
    aperture: BTreeSet<String>,
    object: BTreeSet<String>,
}

impl InForce {
    fn delete(&mut self, out: &mut Vec<Finding>, line: usize, name: &str) {
        if name.is_empty() {
            if self.aperture.is_empty() && self.object.is_empty() {
                out.push(Finding::new(
                    DELETE_WITHOUT_SET,
                    Severity::Warning,
                    line,
                    "TD with no attribute in force",
                ));
            }
            self.aperture.clear();
            self.object.clear();
        } else {
            let a = self.aperture.remove(name);
            let o = self.object.remove(name);
            if !a && !o {
                out.push(Finding::new(
                    DELETE_WITHOUT_SET,
                    Severity::Warning,
                    line,
                    format!("TD{name}"),
                ));
            }
        }
    }
}

struct Block {
    line: usize,
    extended: bool,
    text: String,
}

/// Splits a Gerber file into its word commands (up to `*`) and extended commands (between `%`).
fn blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let (mut line, mut start) = (1usize, 1usize);
    let mut in_extended = false;
    let mut buffer = String::new();
    for ch in text.chars() {
        match ch {
            '\n' => line += 1,
            '\r' => {}
            '%' => {
                if in_extended {
                    out.push(Block {
                        line: start,
                        extended: true,
                        text: std::mem::take(&mut buffer),
                    });
                } else {
                    buffer.clear();
                }
                in_extended = !in_extended;
            }
            '*' if !in_extended => {
                let word = std::mem::take(&mut buffer);
                if !word.trim().is_empty() {
                    out.push(Block {
                        line: start,
                        extended: false,
                        text: word.trim().to_owned(),
                    });
                }
            }
            _ => {
                if buffer.is_empty() {
                    if ch.is_whitespace() {
                        continue;
                    }
                    start = line;
                }
                buffer.push(ch);
            }
        }
    }
    out
}

fn digits_of(text: &str) -> Option<u32> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Splits `.Name,v1,v2` into the name and the values.
fn name_and_values(body: &str) -> (&str, Vec<&str>) {
    let mut parts = body.split(',');
    let name = parts.next().unwrap_or("");
    (name, parts.collect())
}

fn check_gerber(text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut attributes = FileAttributes::new();
    let mut in_force = InForce::default();
    let mut defined: BTreeSet<u32> = BTreeSet::new();
    let (mut format_seen, mut units_seen) = (false, false);
    let (mut ended, mut after_end, mut draw_reported) = (false, false, false);
    let (mut region_open, mut none_selected_reported) = (false, false);
    let mut current: Option<u32> = None;
    let mut last_line = 0usize;
    for block in blocks(text) {
        last_line = block.line;
        if ended {
            if !after_end {
                after_end = true;
                out.push(Finding::new(
                    END_COMMAND,
                    Severity::Error,
                    block.line,
                    "content after M02",
                ));
            }
            continue;
        }
        if block.extended {
            let command = block.text.split('*').next().unwrap_or("");
            match command.get(..2) {
                Some(kind @ ("TF" | "TA" | "TO")) => {
                    let (name, values) = name_and_values(&command[2..]);
                    check_attribute(&mut out, block.line, kind, name, &values, false);
                    match kind {
                        "TF" => {
                            attributes.insert(
                                name.to_owned(),
                                values.iter().map(|v| (*v).to_owned()).collect(),
                            );
                        }
                        "TA" => {
                            in_force.aperture.insert(name.to_owned());
                        }
                        _ => {
                            in_force.object.insert(name.to_owned());
                        }
                    }
                }
                Some("TD") => in_force.delete(&mut out, block.line, &command[2..]),
                Some("FS") => format_seen = true,
                Some("MO") => units_seen = true,
                Some("AD") => {
                    if let Some(number) = command.strip_prefix("ADD").and_then(digits_of) {
                        if number < 10 || !defined.insert(number) {
                            out.push(Finding::new(
                                APERTURE_DEFINITION,
                                Severity::Error,
                                block.line,
                                format!("D{number}"),
                            ));
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        let word = block.text.as_str();
        if word.starts_with("G04") {
            continue;
        }
        if word == "M02" {
            ended = true;
        } else if word == "G36" {
            if region_open {
                out.push(Finding::new(
                    REGION_STRUCTURE,
                    Severity::Error,
                    block.line,
                    "G36 inside a region",
                ));
            }
            region_open = true;
        } else if word == "G37" {
            if !region_open {
                out.push(Finding::new(
                    REGION_STRUCTURE,
                    Severity::Error,
                    block.line,
                    "G37 without G36",
                ));
            }
            region_open = false;
        } else if let Some(number) = word
            .strip_prefix('D')
            .filter(|rest| rest.bytes().all(|b| b.is_ascii_digit()))
            .and_then(digits_of)
        {
            if number >= 10 {
                if !defined.contains(&number) {
                    out.push(Finding::new(
                        UNDEFINED_APERTURE,
                        Severity::Error,
                        block.line,
                        format!("D{number}"),
                    ));
                }
                current = Some(number);
            }
        } else if word.starts_with('X') || word.starts_with('Y') || word.contains("D0") {
            if !draw_reported && !(format_seen && units_seen) {
                draw_reported = true;
                out.push(Finding::new(
                    DRAW_BEFORE_FORMAT,
                    Severity::Error,
                    block.line,
                    "a drawing command comes before FS and MO",
                ));
            }
            let draws = word.ends_with("D03") || (word.ends_with("D01") && !region_open);
            if draws && current.is_none() && !none_selected_reported {
                none_selected_reported = true;
                out.push(Finding::new(
                    UNDEFINED_APERTURE,
                    Severity::Error,
                    block.line,
                    "an object is drawn with no aperture selected",
                ));
            }
        }
    }
    if !ended {
        out.push(Finding::new(
            END_COMMAND,
            Severity::Error,
            last_line,
            "no M02",
        ));
    }
    if region_open {
        out.push(Finding::new(
            REGION_STRUCTURE,
            Severity::Error,
            last_line,
            "a region is not closed",
        ));
    }
    check_file_attributes(&mut out, &attributes, false);
    out
}

fn check_drill(text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut attributes = FileAttributes::new();
    let mut in_force = InForce::default();
    let mut tools: BTreeSet<u32> = BTreeSet::new();
    let (mut ended, mut after_end) = (false, false);
    let mut last_line = 0usize;
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        last_line = line;
        if ended {
            if !after_end {
                after_end = true;
                out.push(Finding::new(
                    END_COMMAND,
                    Severity::Error,
                    line,
                    "content after M30",
                ));
            }
            continue;
        }
        if let Some(comment) = trimmed.strip_prefix(';') {
            let Some(command) = comment.trim().strip_prefix("#@!") else {
                continue;
            };
            let command = command.trim();
            match command.get(..2) {
                Some(kind @ ("TF" | "TA" | "TO")) => {
                    let (name, values) = name_and_values(&command[2..]);
                    check_attribute(&mut out, line, kind, name, &values, true);
                    match kind {
                        "TF" => {
                            attributes.insert(
                                name.to_owned(),
                                values.iter().map(|v| (*v).to_owned()).collect(),
                            );
                        }
                        "TA" => {
                            in_force.aperture.insert(name.to_owned());
                        }
                        _ => {
                            in_force.object.insert(name.to_owned());
                        }
                    }
                }
                Some("TD") => in_force.delete(&mut out, line, &command[2..]),
                _ => {}
            }
            continue;
        }
        if trimmed == "M30" {
            ended = true;
        } else if let Some(rest) = trimmed.strip_prefix('T') {
            if let Some(number) = digits_of(rest) {
                let after = rest.trim_start_matches(|c: char| c.is_ascii_digit());
                if after.starts_with('C') {
                    tools.insert(number);
                } else if number != 0 && !tools.contains(&number) {
                    out.push(Finding::new(
                        UNDEFINED_APERTURE,
                        Severity::Error,
                        line,
                        format!("T{number}"),
                    ));
                }
            }
        }
    }
    if !ended {
        out.push(Finding::new(
            END_COMMAND,
            Severity::Error,
            last_line,
            "no M30",
        ));
    }
    check_file_attributes(&mut out, &attributes, true);
    out
}

/// Checks the text of a Gerber or of an Excellon file (told by its first line, `M48`) and
/// returns what it finds, by line.
pub fn check_text(text: &str) -> Vec<Finding> {
    let first = text.lines().map(str::trim).find(|l| !l.is_empty());
    let mut found = if first == Some("M48") {
        check_drill(text)
    } else {
        check_gerber(text)
    };
    found.sort_by_key(|f| (f.line, f.code));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_rules::Lang;

    const GOOD: &str = "%TF.GenerationSoftware,EH3D,PHOENIX-PCB,0.0.1*%\n\
%TF.FileFunction,Copper,L1,Top*%\n%TF.FilePolarity,Positive*%\n%TF.Part,Single*%\n\
%FSLAX46Y46*%\n%MOMM*%\n%TA.AperFunction,Conductor*%\n%ADD10C,0.25*%\n%TD.AperFunction*%\n\
%TO.N,GND*%\nD10*\nX1000000Y2000000D02*\nX3000000Y2000000D01*\n%TD.N*%\nM02*\n";

    fn codes(findings: &[Finding]) -> Vec<&'static str> {
        findings.iter().map(|f| f.code).collect()
    }

    #[test]
    fn a_well_formed_file_has_no_finding() {
        assert_eq!(check_text(GOOD), Vec::new());
    }

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-GX2-{:03}", i + 1));
            for text in row {
                assert!(!text.is_empty(), "{code}");
            }
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
    }

    #[test]
    fn the_documented_table_has_every_code_of_the_check() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, row) in TITLES {
            let line = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(line.contains(row[0]), "{code}: {line}");
        }
    }

    #[test]
    fn the_titles_are_found_by_the_common_function() {
        let finding = Finding::new(END_COMMAND, Severity::Error, 3, "no M02");
        assert_eq!(
            finding.message(Lang::Pt),
            "error PHOENIX-GX2-001: O comando de fim falta ou não é o último (line 3): no M02"
        );
    }

    #[test]
    fn the_end_command_must_be_there_and_last() {
        let no_end = GOOD.replace("M02*\n", "");
        assert_eq!(codes(&check_text(&no_end)), vec![END_COMMAND]);
        let after = format!("{GOOD}D10*\n");
        assert_eq!(codes(&check_text(&after)), vec![END_COMMAND]);
    }

    #[test]
    fn the_file_function_is_required_and_the_rest_is_recommended() {
        let no_function = GOOD.replace("%TF.FileFunction,Copper,L1,Top*%\n", "");
        assert_eq!(codes(&check_text(&no_function)), vec![REQUIRED_MISSING]);
        let bare = GOOD
            .replace("%TF.FilePolarity,Positive*%\n", "")
            .replace("%TF.Part,Single*%\n", "");
        let found = check_text(&bare);
        assert_eq!(
            codes(&found),
            vec![RECOMMENDED_MISSING, RECOMMENDED_MISSING]
        );
        assert!(found.iter().all(|f| f.severity == Severity::Warning));
        assert!(!has_errors(&found));
    }

    #[test]
    fn apertures_must_be_defined_once_and_before_use() {
        let undefined = GOOD.replace("D10*\nX1", "D11*\nX1");
        assert_eq!(codes(&check_text(&undefined)), vec![UNDEFINED_APERTURE]);
        let twice = GOOD.replace("%ADD10C,0.25*%\n", "%ADD10C,0.25*%\n%ADD10C,0.30*%\n");
        assert_eq!(codes(&check_text(&twice)), vec![APERTURE_DEFINITION]);
        let low = GOOD.replace("%ADD10C,0.25*%", "%ADD5C,0.25*%");
        assert!(codes(&check_text(&low)).contains(&APERTURE_DEFINITION));
        let none = GOOD.replace("D10*\nX1", "X1");
        assert_eq!(codes(&check_text(&none)), vec![UNDEFINED_APERTURE]);
    }

    #[test]
    fn a_delete_needs_an_attribute_in_force() {
        let twice = GOOD.replace("%TD.N*%\n", "%TD.N*%\n%TD.N*%\n");
        assert_eq!(codes(&check_text(&twice)), vec![DELETE_WITHOUT_SET]);
        let unknown = GOOD.replace("%TD.N*%\n", "%TD.N*%\n%TD.P*%\n");
        assert_eq!(codes(&check_text(&unknown)), vec![DELETE_WITHOUT_SET]);
    }

    #[test]
    fn attribute_values_have_the_shape_of_their_name() {
        let bad_polarity = GOOD.replace("Positive", "Plus");
        assert_eq!(codes(&check_text(&bad_polarity)), vec![ATTRIBUTE_INVALID]);
        let bad_copper = GOOD.replace("Copper,L1,Top", "Copper,Top");
        assert_eq!(codes(&check_text(&bad_copper)), vec![ATTRIBUTE_INVALID]);
        let bad_pin = GOOD
            .replace("%TO.N,GND*%", "%TO.P,R1*%")
            .replace("%TD.N*%", "%TD.P*%");
        assert_eq!(codes(&check_text(&bad_pin)), vec![ATTRIBUTE_INVALID]);
        let wrong_home = GOOD.replace("%TO.N,GND*%", "%TA.N,GND*%");
        assert_eq!(codes(&check_text(&wrong_home)), vec![ATTRIBUTE_INVALID]);
        let unknown = GOOD.replace("Conductor", "Teleporter");
        let found = check_text(&unknown);
        assert_eq!(codes(&found), vec![ATTRIBUTE_INVALID]);
        assert!(!has_errors(&found));
    }

    #[test]
    fn a_field_may_not_hold_what_it_cannot_and_an_escape_is_fine() {
        let escaped = GOOD.replace("GND", "A\\u002CB");
        assert_eq!(check_text(&escaped), Vec::new());
        let backslash = GOOD.replace("GND", "A\\B");
        assert_eq!(codes(&check_text(&backslash)), vec![FIELD_CHARACTER]);
        let accent = GOOD.replace("GND", "TENSIÓN");
        assert_eq!(codes(&check_text(&accent)), vec![FIELD_CHARACTER]);
    }

    #[test]
    fn drawing_needs_the_format_and_regions_must_close() {
        let early = GOOD.replace("%FSLAX46Y46*%\n%MOMM*%\n", "");
        assert!(codes(&check_text(&early)).contains(&DRAW_BEFORE_FORMAT));
        let open = GOOD.replace("D10*\n", "D10*\nG36*\n");
        assert_eq!(codes(&check_text(&open)), vec![REGION_STRUCTURE]);
        let closed_alone = GOOD.replace("D10*\n", "D10*\nG37*\n");
        assert_eq!(codes(&check_text(&closed_alone)), vec![REGION_STRUCTURE]);
    }

    const DRILL: &str = "M48\n; #@! TF.GenerationSoftware,EH3D,PHOENIX-PCB,0.0.1\n\
; #@! TF.FileFunction,Plated,1,2,PTH,Drill\n; #@! TF.Part,Single\nMETRIC,TZ\n\
; #@! TA.AperFunction,Plated,PTH,ViaDrill\nT01C0.3\n; #@! TD.AperFunction\n%\nT01\nX1.0Y2.0\nM30\n";

    #[test]
    fn a_well_formed_drill_file_has_no_finding() {
        assert_eq!(check_text(DRILL), Vec::new());
    }

    #[test]
    fn a_drill_file_is_checked_for_its_end_tools_and_attributes() {
        let no_end = DRILL.replace("M30\n", "");
        assert_eq!(codes(&check_text(&no_end)), vec![END_COMMAND]);
        let no_tool = DRILL.replace("T01\nX1", "T02\nX1");
        assert_eq!(codes(&check_text(&no_tool)), vec![UNDEFINED_APERTURE]);
        let no_function = DRILL.replace("; #@! TF.FileFunction,Plated,1,2,PTH,Drill\n", "");
        assert_eq!(codes(&check_text(&no_function)), vec![REQUIRED_MISSING]);
        let no_set = DRILL.replace("; #@! TA.AperFunction,Plated,PTH,ViaDrill\n", "");
        assert_eq!(codes(&check_text(&no_set)), vec![DELETE_WITHOUT_SET]);
        let bad = DRILL.replace("PTH,ViaDrill", "PTH,Nothing");
        let found = check_text(&bad);
        assert_eq!(codes(&found), vec![ATTRIBUTE_INVALID]);
        assert!(!has_errors(&found));
    }
}
