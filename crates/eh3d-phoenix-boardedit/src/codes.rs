// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-BEDIT-<NNN>` of the board editor, with their titles in the eight
//! languages of the suite, and the error that carries one.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// A settings file that is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-BEDIT-001";
/// A file of operations that cannot be read.
pub const OPERATIONS_INVALID: &str = "PHOENIX-BEDIT-002";
/// A result that would overwrite an input file, or that has a format the editor does not write.
pub const OUTPUT_NOT_ALLOWED: &str = "PHOENIX-BEDIT-003";
/// An operation that the board refuses or that refers to something the board does not have.
pub const OPERATION_REFUSED: &str = "PHOENIX-BEDIT-004";
/// An operation that would add a rule error.
pub const REJECTED_BY_DRC: &str = "PHOENIX-BEDIT-005";
/// A component that is locked.
pub const COMPONENT_LOCKED: &str = "PHOENIX-BEDIT-006";
/// A shape that cannot be made: a route without length, a point out of range, a stretch that
/// does not exist.
pub const GEOMETRY_INVALID: &str = "PHOENIX-BEDIT-007";
/// A net that cannot be tuned to the length asked for.
pub const TUNE_IMPOSSIBLE: &str = "PHOENIX-BEDIT-008";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Settings file is not valid",
            "El fichero de configuración no es válido",
            "Die Einstellungsdatei ist ungültig",
            "Le fichier de réglages n'est pas valide",
            "Il file delle impostazioni non è valido",
            "設定ファイルが無効です",
            "设置文件无效",
            "O ficheiro de definições não é válido",
        ],
    ),
    (
        OPERATIONS_INVALID,
        [
            "File of operations is not valid",
            "El fichero de operaciones no es válido",
            "Die Operationsdatei ist ungültig",
            "Le fichier d'opérations n'est pas valide",
            "Il file delle operazioni non è valido",
            "操作ファイルが無効です",
            "操作文件无效",
            "O ficheiro de operações não é válido",
        ],
    ),
    (
        OUTPUT_NOT_ALLOWED,
        [
            "Output file is not allowed",
            "El fichero de salida no está permitido",
            "Die Ausgabedatei ist nicht erlaubt",
            "Le fichier de sortie n'est pas autorisé",
            "Il file di uscita non è consentito",
            "出力ファイルは許可されていません",
            "不允许使用该输出文件",
            "O ficheiro de saída não é permitido",
        ],
    ),
    (
        OPERATION_REFUSED,
        [
            "Operation was refused",
            "La operación fue rechazada",
            "Die Operation wurde abgelehnt",
            "L'opération a été refusée",
            "L'operazione è stata rifiutata",
            "操作は拒否されました",
            "操作被拒绝",
            "A operação foi recusada",
        ],
    ),
    (
        REJECTED_BY_DRC,
        [
            "Operation adds a rule error",
            "La operación añade un error de reglas",
            "Die Operation fügt einen Regelfehler hinzu",
            "L'opération ajoute une erreur de règles",
            "L'operazione aggiunge un errore di regole",
            "操作により規則エラーが追加されます",
            "该操作会增加规则错误",
            "A operação acrescenta um erro de regras",
        ],
    ),
    (
        COMPONENT_LOCKED,
        [
            "Component is locked",
            "El componente está bloqueado",
            "Das Bauteil ist gesperrt",
            "Le composant est verrouillé",
            "Il componente è bloccato",
            "部品はロックされています",
            "元件已锁定",
            "O componente está bloqueado",
        ],
    ),
    (
        GEOMETRY_INVALID,
        [
            "Shape cannot be made",
            "No se puede crear la forma",
            "Die Form kann nicht erzeugt werden",
            "La forme ne peut pas être créée",
            "La forma non può essere creata",
            "形状を作成できません",
            "无法创建该形状",
            "Não é possível criar a forma",
        ],
    ),
    (
        TUNE_IMPOSSIBLE,
        [
            "Net cannot be tuned to the length asked for",
            "La red no se puede ajustar a la longitud pedida",
            "Das Netz kann nicht auf die verlangte Länge abgeglichen werden",
            "Le réseau ne peut pas être ajusté à la longueur demandée",
            "La rete non può essere regolata alla lunghezza richiesta",
            "ネットを指定の長さに調整できません",
            "无法将网络调整到所需长度",
            "A rede não pode ser ajustada ao comprimento pedido",
        ],
    ),
];

/// The codes of this crate with their titles in English.
pub fn registry() -> Vec<(&'static str, &'static str)> {
    TITLES.iter().map(|(code, row)| (*code, row[0])).collect()
}

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

/// An error of the editor with its code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardEditError {
    /// The stable code.
    pub code: &'static str,
    /// What is wrong, in English.
    pub detail: String,
}

impl BoardEditError {
    /// An error with a code and a detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> BoardEditError {
        BoardEditError {
            code,
            detail: detail.into(),
        }
    }

    /// The error in a language: the code, the title and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let mut shown = title(self.code, lang);
        if shown.is_empty() {
            shown = crate::pair_codes::title(self.code, lang);
        }
        if shown.is_empty() {
            shown = crate::shove_codes::title(self.code, lang);
        }
        format!("{} {}: {}", self.code, shown, self.detail)
    }
}

impl fmt::Display for BoardEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for BoardEditError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in registry() {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }

    #[test]
    fn every_code_has_a_title_in_every_language() {
        for (code, row) in TITLES {
            assert!(code.starts_with("PHOENIX-BEDIT-"));
            for lang in Lang::ALL {
                assert!(!title(code, lang).is_empty(), "{code}");
            }
            assert_eq!(row.len(), 8);
        }
        assert_eq!(title("PHOENIX-BEDIT-999", Lang::En), "");
    }

    #[test]
    fn an_error_shows_its_code_first() {
        let e = BoardEditError::new(OPERATION_REFUSED, "bad");
        assert_eq!(e.to_string(), "PHOENIX-BEDIT-004: bad");
        assert!(e.message(Lang::Pt).contains("recusada"));
    }
}
