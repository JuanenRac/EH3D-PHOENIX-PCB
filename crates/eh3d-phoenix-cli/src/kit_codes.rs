// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the starter kit, `PHOENIX-KIT-<NNN>`, with their titles in the eight
//! languages of the suite.

use eh3d_phoenix_rules::Lang;

/// The catalogue of the kit cannot be found or read.
pub const CATALOGUE_UNREADABLE: &str = "PHOENIX-KIT-001";
/// The catalogue is not a catalogue of this format, or it names a path that is not acceptable.
pub const CATALOGUE_INVALID: &str = "PHOENIX-KIT-002";
/// A file of the kit is missing, is not a plain file or is larger than its limit.
pub const FILE_UNUSABLE: &str = "PHOENIX-KIT-003";
/// A file of the kit does not have the bytes that the catalogue records.
pub const FILE_CHANGED: &str = "PHOENIX-KIT-004";
/// A file to be copied is already there, so nothing is copied.
pub const DESTINATION_EXISTS: &str = "PHOENIX-KIT-005";
/// The catalogue has no piece with this identifier.
pub const UNKNOWN_PIECE: &str = "PHOENIX-KIT-006";
/// An option or a line of the settings is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-KIT-007";
/// A copy cannot be written.
pub const COPY_FAILED: &str = "PHOENIX-KIT-008";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        CATALOGUE_UNREADABLE,
        [
            "The catalogue of the kit cannot be read",
            "No se puede leer el catálogo del kit",
            "Der Katalog des Kits kann nicht gelesen werden",
            "Le catalogue du kit ne peut pas être lu",
            "Impossibile leggere il catalogo del kit",
            "キットのカタログを読み取れません",
            "无法读取套件目录",
            "Não é possível ler o catálogo do kit",
        ],
    ),
    (
        CATALOGUE_INVALID,
        [
            "The catalogue of the kit is not valid",
            "El catálogo del kit no es válido",
            "Der Katalog des Kits ist ungültig",
            "Le catalogue du kit n'est pas valide",
            "Il catalogo del kit non è valido",
            "キットのカタログが不正です",
            "套件目录无效",
            "O catálogo do kit não é válido",
        ],
    ),
    (
        FILE_UNUSABLE,
        [
            "A file of the kit cannot be used",
            "No se puede usar un fichero del kit",
            "Eine Datei des Kits kann nicht verwendet werden",
            "Un fichier du kit ne peut pas être utilisé",
            "Un file del kit non può essere usato",
            "キットのファイルを使用できません",
            "套件中的某个文件无法使用",
            "Não é possível usar um ficheiro do kit",
        ],
    ),
    (
        FILE_CHANGED,
        [
            "A file of the kit does not match the catalogue",
            "Un fichero del kit no coincide con el catálogo",
            "Eine Datei des Kits stimmt nicht mit dem Katalog überein",
            "Un fichier du kit ne correspond pas au catalogue",
            "Un file del kit non corrisponde al catalogo",
            "キットのファイルがカタログと一致しません",
            "套件中的某个文件与目录不一致",
            "Um ficheiro do kit não corresponde ao catálogo",
        ],
    ),
    (
        DESTINATION_EXISTS,
        [
            "A destination file already exists; nothing is copied",
            "Ya existe un fichero de destino; no se copia nada",
            "Eine Zieldatei ist bereits vorhanden; nichts wird kopiert",
            "Un fichier de destination existe déjà ; rien n'est copié",
            "Un file di destinazione esiste già; non viene copiato nulla",
            "コピー先のファイルが既に存在するため、何もコピーしません",
            "目标文件已存在，未复制任何内容",
            "Já existe um ficheiro de destino; nada é copiado",
        ],
    ),
    (
        UNKNOWN_PIECE,
        [
            "The kit has no piece with this identifier",
            "El kit no tiene ninguna pieza con ese identificador",
            "Das Kit hat kein Teil mit dieser Kennung",
            "Le kit n'a aucun élément avec cet identifiant",
            "Il kit non ha elementi con questo identificativo",
            "この識別子の項目はキットにありません",
            "套件中没有该标识的条目",
            "O kit não tem nenhuma peça com este identificador",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "An option or a setting of the kit is not valid",
            "Una opción o un ajuste del kit no es válido",
            "Eine Option oder Einstellung des Kits ist ungültig",
            "Une option ou un réglage du kit n'est pas valide",
            "Un'opzione o un'impostazione del kit non è valida",
            "キットのオプションまたは設定が不正です",
            "套件的选项或设置无效",
            "Uma opção ou definição do kit não é válida",
        ],
    ),
    (
        COPY_FAILED,
        [
            "A file of the kit could not be copied",
            "No se pudo copiar un fichero del kit",
            "Eine Datei des Kits konnte nicht kopiert werden",
            "Un fichier du kit n'a pas pu être copié",
            "Non è stato possibile copiare un file del kit",
            "キットのファイルをコピーできませんでした",
            "无法复制套件中的某个文件",
            "Não foi possível copiar um ficheiro do kit",
        ],
    ),
];

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-KIT-{:03}", i + 1));
            for text in &row[1..] {
                assert!(!text.is_empty(), "{code}");
                assert_ne!(*text, row[0], "{code}");
            }
        }
        assert_eq!(title("PHOENIX-KIT-999", Lang::En), "");
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, row) in TITLES {
            let translated = doc
                .lines()
                .filter(|l| l.starts_with(&format!("| {code} |")))
                .any(|l| row.iter().all(|t| l.contains(t)));
            assert!(translated, "{code} is not in the table of the languages");
        }
    }
}
