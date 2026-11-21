// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the reader of user programs, `PHOENIX-ULP-<NNN>`, with their titles in
//! the eight languages of the suite, and the diagnostic that carries one with the place in the
//! program (line and column, both counted from one).
//!
//! Codes 001 to 003 and 018 and 024 belong to the settings, the command and the files; 004 to 017
//! are what the reader finds in a program; 019 to 023 are the notes of the compatibility report.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The settings of the reader are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-ULP-001";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-ULP-002";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-ULP-003";
/// The program is larger than the limit.
pub const SOURCE_TOO_LARGE: &str = "PHOENIX-ULP-004";
/// The program has more tokens than the limit.
pub const TOO_MANY_TOKENS: &str = "PHOENIX-ULP-005";
/// Blocks or expressions are nested deeper than the limit.
pub const TOO_DEEP: &str = "PHOENIX-ULP-006";
/// The program has more statements than the limit.
pub const TOO_MANY_STATEMENTS: &str = "PHOENIX-ULP-007";
/// A text in quotes is not closed.
pub const STRING_OPEN: &str = "PHOENIX-ULP-008";
/// A comment is not closed.
pub const COMMENT_OPEN: &str = "PHOENIX-ULP-009";
/// A character is not valid where it stands.
pub const CHAR_INVALID: &str = "PHOENIX-ULP-010";
/// A number is not valid.
pub const NUMBER_INVALID: &str = "PHOENIX-ULP-011";
/// An escape sequence of a text is not valid.
pub const ESCAPE_INVALID: &str = "PHOENIX-ULP-012";
/// A directive is not known.
pub const DIRECTIVE_UNKNOWN: &str = "PHOENIX-ULP-013";
/// A directive is not well formed.
pub const DIRECTIVE_INVALID: &str = "PHOENIX-ULP-014";
/// A token is not expected where it stands.
pub const UNEXPECTED: &str = "PHOENIX-ULP-015";
/// A token that is needed is missing.
pub const EXPECTED: &str = "PHOENIX-ULP-016";
/// A declaration is not valid.
pub const DECLARATION_INVALID: &str = "PHOENIX-ULP-017";
/// The list of allowed names is not valid.
pub const SUPPORT_INVALID: &str = "PHOENIX-ULP-018";
/// A function or member is not on the list.
pub const NAME_UNKNOWN: &str = "PHOENIX-ULP-019";
/// A name the sandbox never allows.
pub const ITEM_DENIED: &str = "PHOENIX-ULP-020";
/// A name that is not covered yet.
pub const ITEM_PENDING: &str = "PHOENIX-ULP-021";
/// A name that is allowed with limits.
pub const ITEM_LIMITED: &str = "PHOENIX-ULP-022";
/// The list of problems was cut.
pub const LIST_CUT: &str = "PHOENIX-ULP-023";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-ULP-024";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "The settings of the reader of user programs are not valid",
            "La configuración del lector de programas de usuario no es válida",
            "Die Einstellungen des Lesers für Benutzerprogramme sind ungültig",
            "Les réglages du lecteur de programmes utilisateur ne sont pas valides",
            "Le impostazioni del lettore dei programmi utente non sono valide",
            "ユーザープログラム読み取りの設定が無効です",
            "用户程序读取器的设置无效",
            "As definições do leitor de programas de utilizador não são válidas",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "A command option is not valid",
            "Una opción del comando no es válida",
            "Eine Befehlsoption ist ungültig",
            "Une option de la commande n'est pas valide",
            "Un'opzione del comando non è valida",
            "コマンドのオプションが無効です",
            "命令选项无效",
            "Uma opção do comando não é válida",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "An input file cannot be read",
            "No se puede leer un archivo de entrada",
            "Eine Eingabedatei kann nicht gelesen werden",
            "Un fichier d'entrée ne peut pas être lu",
            "Non è possibile leggere un file di ingresso",
            "入力ファイルを読み取れません",
            "无法读取输入文件",
            "Não é possível ler um ficheiro de entrada",
        ],
    ),
    (
        SOURCE_TOO_LARGE,
        [
            "The program is larger than the limit",
            "El programa es más grande que el límite",
            "Das Programm ist größer als die Grenze",
            "Le programme dépasse la limite de taille",
            "Il programma supera il limite di dimensione",
            "プログラムのサイズが上限を超えています",
            "程序超出大小限制",
            "O programa é maior do que o limite",
        ],
    ),
    (
        TOO_MANY_TOKENS,
        [
            "The program has more tokens than the limit",
            "El programa tiene más componentes léxicos que el límite",
            "Das Programm hat mehr Token als die Grenze",
            "Le programme a plus de lexèmes que la limite",
            "Il programma ha più token del limite",
            "プログラムのトークン数が上限を超えています",
            "程序的词法单元数超出限制",
            "O programa tem mais itens léxicos do que o limite",
        ],
    ),
    (
        TOO_DEEP,
        [
            "Blocks or expressions are nested deeper than the limit",
            "Los bloques o expresiones están anidados más allá del límite",
            "Blöcke oder Ausdrücke sind tiefer verschachtelt als erlaubt",
            "Des blocs ou des expressions sont imbriqués au-delà de la limite",
            "Blocchi o espressioni sono annidati oltre il limite",
            "ブロックまたは式のネストが上限を超えています",
            "代码块或表达式的嵌套超出限制",
            "Blocos ou expressões estão aninhados além do limite",
        ],
    ),
    (
        TOO_MANY_STATEMENTS,
        [
            "The program has more statements than the limit",
            "El programa tiene más sentencias que el límite",
            "Das Programm hat mehr Anweisungen als die Grenze",
            "Le programme a plus d'instructions que la limite",
            "Il programma ha più istruzioni del limite",
            "プログラムの文の数が上限を超えています",
            "程序的语句数超出限制",
            "O programa tem mais instruções do que o limite",
        ],
    ),
    (
        STRING_OPEN,
        [
            "A text in quotes is not closed",
            "Un texto entre comillas no está cerrado",
            "Ein Text in Anführungszeichen ist nicht geschlossen",
            "Un texte entre guillemets n'est pas fermé",
            "Un testo tra virgolette non è chiuso",
            "引用符で囲んだ文字列が閉じられていません",
            "引号中的文本没有闭合",
            "Um texto entre aspas não está fechado",
        ],
    ),
    (
        COMMENT_OPEN,
        [
            "A comment is not closed",
            "Un comentario no está cerrado",
            "Ein Kommentar ist nicht geschlossen",
            "Un commentaire n'est pas fermé",
            "Un commento non è chiuso",
            "コメントが閉じられていません",
            "注释没有闭合",
            "Um comentário não está fechado",
        ],
    ),
    (
        CHAR_INVALID,
        [
            "A character is not valid where it stands",
            "Un carácter no es válido en ese lugar",
            "Ein Zeichen ist an dieser Stelle ungültig",
            "Un caractère n'est pas valide à cet endroit",
            "Un carattere non è valido in quel punto",
            "この位置では使えない文字があります",
            "此处出现了无效字符",
            "Um carácter não é válido nesse local",
        ],
    ),
    (
        NUMBER_INVALID,
        [
            "A number is not valid",
            "Un número no es válido",
            "Eine Zahl ist ungültig",
            "Un nombre n'est pas valide",
            "Un numero non è valido",
            "数値が無効です",
            "数字无效",
            "Um número não é válido",
        ],
    ),
    (
        ESCAPE_INVALID,
        [
            "An escape sequence of a text is not valid",
            "Una secuencia de escape de un texto no es válida",
            "Eine Escape-Sequenz in einem Text ist ungültig",
            "Une séquence d'échappement d'un texte n'est pas valide",
            "Una sequenza di escape di un testo non è valida",
            "文字列のエスケープシーケンスが無効です",
            "文本中的转义序列无效",
            "Uma sequência de escape de um texto não é válida",
        ],
    ),
    (
        DIRECTIVE_UNKNOWN,
        [
            "A directive is not known",
            "Una directiva no se conoce",
            "Eine Direktive ist unbekannt",
            "Une directive n'est pas connue",
            "Una direttiva non è conosciuta",
            "不明なディレクティブです",
            "未知的指令",
            "Uma diretiva não é conhecida",
        ],
    ),
    (
        DIRECTIVE_INVALID,
        [
            "A directive is not well formed",
            "Una directiva está mal formada",
            "Eine Direktive ist fehlerhaft aufgebaut",
            "Une directive est mal formée",
            "Una direttiva è malformata",
            "ディレクティブの形式が正しくありません",
            "指令格式不正确",
            "Uma diretiva está mal formada",
        ],
    ),
    (
        UNEXPECTED,
        [
            "A token is not expected where it stands",
            "Un elemento no se espera en ese lugar",
            "Ein Element wird an dieser Stelle nicht erwartet",
            "Un élément n'est pas attendu à cet endroit",
            "Un elemento non è previsto in quel punto",
            "この位置に置けない要素があります",
            "此处出现了意外的元素",
            "Um elemento não é esperado nesse local",
        ],
    ),
    (
        EXPECTED,
        [
            "A token that is needed is missing",
            "Falta un elemento necesario",
            "Ein erforderliches Element fehlt",
            "Un élément nécessaire est absent",
            "Manca un elemento necessario",
            "必要な要素が欠けています",
            "缺少必需的元素",
            "Falta um elemento necessário",
        ],
    ),
    (
        DECLARATION_INVALID,
        [
            "A declaration is not valid",
            "Una declaración no es válida",
            "Eine Deklaration ist ungültig",
            "Une déclaration n'est pas valide",
            "Una dichiarazione non è valida",
            "宣言が無効です",
            "声明无效",
            "Uma declaração não é válida",
        ],
    ),
    (
        SUPPORT_INVALID,
        [
            "The list of allowed names is not valid",
            "La lista de nombres permitidos no es válida",
            "Die Liste der erlaubten Namen ist ungültig",
            "La liste des noms autorisés n'est pas valide",
            "L'elenco dei nomi consentiti non è valido",
            "許可する名前の一覧が無効です",
            "允许名称列表无效",
            "A lista de nomes permitidos não é válida",
        ],
    ),
    (
        NAME_UNKNOWN,
        [
            "A function or member is not on the list",
            "Una función o miembro no está en la lista",
            "Eine Funktion oder ein Member steht nicht in der Liste",
            "Une fonction ou un membre n'est pas dans la liste",
            "Una funzione o un membro non è nell'elenco",
            "関数またはメンバーが一覧にありません",
            "某个函数或成员不在列表中",
            "Uma função ou membro não está na lista",
        ],
    ),
    (
        ITEM_DENIED,
        [
            "The sandbox never allows this",
            "El entorno aislado nunca permite esto",
            "Die Sandbox erlaubt dies nie",
            "Le bac à sable n'autorise jamais cela",
            "La sandbox non lo consente mai",
            "サンドボックスはこれを一切許可しません",
            "沙箱永远不允许这一项",
            "O ambiente isolado nunca permite isto",
        ],
    ),
    (
        ITEM_PENDING,
        [
            "Not covered yet",
            "Todavía no está cubierto",
            "Noch nicht abgedeckt",
            "Pas encore couvert",
            "Non ancora coperto",
            "まだ対応していません",
            "尚未覆盖",
            "Ainda não está coberto",
        ],
    ),
    (
        ITEM_LIMITED,
        [
            "Allowed with limits",
            "Permitido con límites",
            "Mit Einschränkungen erlaubt",
            "Autorisé avec des limites",
            "Consentito con limiti",
            "制限付きで許可されます",
            "有限制地允许",
            "Permitido com limites",
        ],
    ),
    (
        LIST_CUT,
        [
            "The list of problems was cut",
            "La lista de problemas se cortó",
            "Die Liste der Probleme wurde gekürzt",
            "La liste des problèmes a été coupée",
            "L'elenco dei problemi è stato troncato",
            "問題の一覧が途中で打ち切られました",
            "问题列表已被截断",
            "A lista de problemas foi cortada",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "An output file exists or cannot be written",
            "Un archivo de salida ya existe o no se puede escribir",
            "Eine Ausgabedatei existiert bereits oder kann nicht geschrieben werden",
            "Un fichier de sortie existe déjà ou ne peut pas être écrit",
            "Un file di uscita esiste già o non può essere scritto",
            "出力ファイルが既に存在するか、書き込めません",
            "输出文件已存在或无法写入",
            "Um ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
];

/// The title of a code in a language; the code itself when it is not registered.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = match lang {
        Lang::En => 0,
        Lang::Es => 1,
        Lang::De => 2,
        Lang::Fr => 3,
        Lang::It => 4,
        Lang::Ja => 5,
        Lang::Zh => 6,
        Lang::Pt => 7,
    };
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

/// A diagnostic: a registered code, the place in the program (line and column from one, zero when
/// the problem is not in a place) and what is specific to the case, in English.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    /// The code, `PHOENIX-ULP-<NNN>`.
    pub code: &'static str,
    /// The line, from one; zero when there is none.
    pub line: usize,
    /// The column, from one; zero when there is none.
    pub col: usize,
    /// What is specific to the case.
    pub detail: String,
}

/// An error of the reader is a diagnostic.
pub type UlpError = Diag;

impl Diag {
    /// A diagnostic without a place.
    pub fn new(code: &'static str, detail: impl Into<String>) -> Diag {
        Diag::at(code, 0, 0, detail)
    }

    /// A diagnostic at a line and a column.
    pub fn at(code: &'static str, line: usize, col: usize, detail: impl Into<String>) -> Diag {
        Diag {
            code,
            line,
            col,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the place, the title and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let mut out = self.code.to_owned();
        if self.line > 0 {
            out.push_str(&format!(" [{}:{}]", self.line, self.col));
        }
        out.push_str(": ");
        out.push_str(title(self.code, lang));
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(
                f,
                "{} [{}:{}]: {}",
                self.code, self.line, self.col, self.detail
            )
        } else {
            write!(f, "{}: {}", self.code, self.detail)
        }
    }
}

impl std::error::Error for Diag {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-ULP-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
        }
        assert_eq!(TITLES.len(), 24);
    }

    #[test]
    fn no_title_of_another_language_is_a_copy_of_the_english_one() {
        for (code, row) in TITLES {
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
    }

    #[test]
    fn a_message_names_the_code_the_place_the_title_and_the_detail() {
        let d = Diag::at(EXPECTED, 3, 14, "`;`");
        assert_eq!(
            d.message(Lang::En),
            "PHOENIX-ULP-016 [3:14]: A token that is needed is missing: `;`"
        );
        assert_eq!(
            d.message(Lang::Pt),
            "PHOENIX-ULP-016 [3:14]: Falta um elemento necessário: `;`"
        );
        let bare = Diag::new(LIST_CUT, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-ULP-023: The list of problems was cut"
        );
        assert_eq!(bare.to_string(), "PHOENIX-ULP-023: ");
    }

    #[test]
    fn a_code_that_is_not_registered_has_no_title() {
        assert_eq!(title("PHOENIX-ULP-999", Lang::En), "");
    }
}
