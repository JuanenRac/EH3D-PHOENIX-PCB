// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the reader of user programs, `PHOENIX-ULP-<NNN>`, with their titles in
//! the eight languages of the suite, and the diagnostic that carries one with the place in the
//! program (line and column, both counted from one).
//!
//! Codes 001 to 003 and 018 and 024 belong to the settings, the command and the files; 004 to 017
//! are what the reader finds in a program; 019 to 023 are the notes of the compatibility report;
//! 025 to 041 are what stops a run (the limits, the errors of the evaluator and the check of the
//! output).

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

/// The run took more steps than the limit.
pub const RUN_STEP_LIMIT: &str = "PHOENIX-ULP-025";
/// The variables of the run hold more memory than the limit.
pub const RUN_MEMORY_LIMIT: &str = "PHOENIX-ULP-026";
/// Statements, expressions or calls are nested deeper than the limit in the run.
pub const RUN_DEPTH_LIMIT: &str = "PHOENIX-ULP-027";
/// The program wrote more than the limit.
pub const RUN_OUTPUT_LIMIT: &str = "PHOENIX-ULP-028";
/// A value has a type that the operation does not accept.
pub const RUN_TYPE: &str = "PHOENIX-ULP-029";
/// Division by zero.
pub const RUN_DIVISION: &str = "PHOENIX-ULP-030";
/// A number is out of range.
pub const RUN_RANGE: &str = "PHOENIX-ULP-031";
/// A name is not defined.
pub const RUN_NAME: &str = "PHOENIX-ULP-032";
/// An index is out of range.
pub const RUN_INDEX: &str = "PHOENIX-ULP-033";
/// The program uses something that the sandbox does not allow.
pub const RUN_NOT_ALLOWED: &str = "PHOENIX-ULP-034";
/// The evaluator does not run this yet.
pub const RUN_NOT_WRITTEN: &str = "PHOENIX-ULP-035";
/// The program needs a board or a schematic that was not given.
pub const RUN_NO_MODEL: &str = "PHOENIX-ULP-036";
/// A call has the wrong arguments.
pub const RUN_ARGUMENTS: &str = "PHOENIX-ULP-037";
/// The program tries to change the design, which a run only reads.
pub const RUN_READ_ONLY: &str = "PHOENIX-ULP-038";
/// The program has problems of reading and was not run.
pub const RUN_HAS_PROBLEMS: &str = "PHOENIX-ULP-039";
/// A format text is not valid for its arguments.
pub const RUN_FORMAT: &str = "PHOENIX-ULP-040";
/// The output of the run differs from the expected output.
pub const RUN_OUTPUT_DIFFERS: &str = "PHOENIX-ULP-041";

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
    (
        RUN_STEP_LIMIT,
        [
            "The run took more steps than the limit",
            "La ejecución dio más pasos que el límite",
            "Die Ausführung hat mehr Schritte gebraucht als die Grenze",
            "L'exécution a fait plus de pas que la limite",
            "L'esecuzione ha eseguito più passi del limite",
            "実行のステップ数が上限を超えました",
            "运行的步数超出限制",
            "A execução deu mais passos do que o limite",
        ],
    ),
    (
        RUN_MEMORY_LIMIT,
        [
            "The variables of the run hold more memory than the limit",
            "Las variables de la ejecución ocupan más memoria que el límite",
            "Die Variablen der Ausführung belegen mehr Speicher als die Grenze",
            "Les variables de l'exécution occupent plus de mémoire que la limite",
            "Le variabili dell'esecuzione occupano più memoria del limite",
            "実行の変数がメモリ上限を超えました",
            "运行的变量占用内存超出限制",
            "As variáveis da execução ocupam mais memória do que o limite",
        ],
    ),
    (
        RUN_DEPTH_LIMIT,
        [
            "Statements, expressions or calls are nested deeper than the limit in the run",
            "En la ejecución, las sentencias, expresiones o llamadas se anidan más que el límite",
            "In der Ausführung sind Anweisungen, Ausdrücke oder Aufrufe tiefer verschachtelt als die Grenze",
            "Dans l'exécution, les instructions, expressions ou appels sont imbriqués plus profondément que la limite",
            "Nell'esecuzione istruzioni, espressioni o chiamate sono annidate più del limite",
            "実行中に文・式・呼び出しの入れ子が上限を超えました",
            "运行中语句、表达式或调用的嵌套超出限制",
            "Na execução, as instruções, expressões ou chamadas estão aninhadas mais fundo do que o limite",
        ],
    ),
    (
        RUN_OUTPUT_LIMIT,
        [
            "The program wrote more than the limit",
            "El programa escribió más que el límite",
            "Das Programm hat mehr geschrieben als die Grenze",
            "Le programme a écrit plus que la limite",
            "Il programma ha scritto più del limite",
            "プログラムの出力が上限を超えました",
            "程序写出的内容超出限制",
            "O programa escreveu mais do que o limite",
        ],
    ),
    (
        RUN_TYPE,
        [
            "A value has a type that the operation does not accept",
            "Un valor tiene un tipo que la operación no acepta",
            "Ein Wert hat einen Typ, den die Operation nicht akzeptiert",
            "Une valeur a un type que l'opération n'accepte pas",
            "Un valore ha un tipo che l'operazione non accetta",
            "値の型がこの操作では使えません",
            "某个值的类型不被该操作接受",
            "Um valor tem um tipo que a operação não aceita",
        ],
    ),
    (
        RUN_DIVISION,
        [
            "Division by zero",
            "División por cero",
            "Division durch null",
            "Division par zéro",
            "Divisione per zero",
            "ゼロ除算です",
            "除以零",
            "Divisão por zero",
        ],
    ),
    (
        RUN_RANGE,
        [
            "A number is out of range",
            "Un número está fuera de rango",
            "Eine Zahl liegt außerhalb des Bereichs",
            "Un nombre est hors limites",
            "Un numero è fuori intervallo",
            "数値が範囲外です",
            "数值超出范围",
            "Um número está fora do intervalo",
        ],
    ),
    (
        RUN_NAME,
        [
            "A name is not defined",
            "Un nombre no está definido",
            "Ein Name ist nicht definiert",
            "Un nom n'est pas défini",
            "Un nome non è definito",
            "名前が定義されていません",
            "名称未定义",
            "Um nome não está definido",
        ],
    ),
    (
        RUN_INDEX,
        [
            "An index is out of range",
            "Un índice está fuera de rango",
            "Ein Index liegt außerhalb des Bereichs",
            "Un indice est hors limites",
            "Un indice è fuori intervallo",
            "添字が範囲外です",
            "索引超出范围",
            "Um índice está fora do intervalo",
        ],
    ),
    (
        RUN_NOT_ALLOWED,
        [
            "The program uses something that the sandbox does not allow",
            "El programa usa algo que el entorno aislado no permite",
            "Das Programm verwendet etwas, das die Sandbox nicht erlaubt",
            "Le programme utilise quelque chose que le bac à sable n'autorise pas",
            "Il programma usa qualcosa che la sandbox non consente",
            "プログラムがサンドボックスで許可されていないものを使っています",
            "程序使用了沙箱不允许的内容",
            "O programa usa algo que o ambiente isolado não permite",
        ],
    ),
    (
        RUN_NOT_WRITTEN,
        [
            "The evaluator does not run this yet",
            "El evaluador todavía no ejecuta esto",
            "Der Auswerter führt dies noch nicht aus",
            "L'évaluateur n'exécute pas encore cela",
            "Il valutatore non esegue ancora questo",
            "評価器はまだこれを実行できません",
            "求值器尚不能运行此内容",
            "O avaliador ainda não executa isto",
        ],
    ),
    (
        RUN_NO_MODEL,
        [
            "The program needs a board or a schematic that was not given",
            "El programa necesita una placa o un esquema que no se dio",
            "Das Programm braucht eine Platine oder einen Schaltplan, der nicht angegeben wurde",
            "Le programme a besoin d'une carte ou d'un schéma qui n'a pas été donné",
            "Il programma richiede una scheda o uno schema che non è stato indicato",
            "プログラムに必要な基板または回路図が指定されていません",
            "程序需要的电路板或原理图未提供",
            "O programa precisa de uma placa ou de um esquema que não foi indicado",
        ],
    ),
    (
        RUN_ARGUMENTS,
        [
            "A call has the wrong arguments",
            "Una llamada tiene argumentos incorrectos",
            "Ein Aufruf hat falsche Argumente",
            "Un appel a des arguments incorrects",
            "Una chiamata ha argomenti errati",
            "呼び出しの引数が正しくありません",
            "调用的参数不正确",
            "Uma chamada tem argumentos errados",
        ],
    ),
    (
        RUN_READ_ONLY,
        [
            "The program tries to change the design, which a run only reads",
            "El programa intenta cambiar el diseño, que una ejecución solo lee",
            "Das Programm versucht, den Entwurf zu ändern, den eine Ausführung nur liest",
            "Le programme essaie de modifier la conception, qu'une exécution ne fait que lire",
            "Il programma cerca di modificare il progetto, che un'esecuzione si limita a leggere",
            "プログラムが設計を変更しようとしましたが、実行は読み取り専用です",
            "程序试图修改设计，而运行只读取设计",
            "O programa tenta alterar o projeto, que uma execução apenas lê",
        ],
    ),
    (
        RUN_HAS_PROBLEMS,
        [
            "The program has problems of reading and was not run",
            "El programa tiene problemas de lectura y no se ejecutó",
            "Das Programm hat Lesefehler und wurde nicht ausgeführt",
            "Le programme a des problèmes de lecture et n'a pas été exécuté",
            "Il programma ha problemi di lettura e non è stato eseguito",
            "プログラムに読み取りの問題があるため実行しませんでした",
            "程序存在读取问题，未运行",
            "O programa tem problemas de leitura e não foi executado",
        ],
    ),
    (
        RUN_FORMAT,
        [
            "A format text is not valid for its arguments",
            "Un texto de formato no es válido para sus argumentos",
            "Ein Formattext passt nicht zu seinen Argumenten",
            "Un texte de format n'est pas valide pour ses arguments",
            "Un testo di formato non è valido per i suoi argomenti",
            "書式テキストが引数に対して無効です",
            "格式文本与其参数不匹配",
            "Um texto de formato não é válido para os seus argumentos",
        ],
    ),
    (
        RUN_OUTPUT_DIFFERS,
        [
            "The output of the run differs from the expected output",
            "La salida de la ejecución difiere de la esperada",
            "Die Ausgabe der Ausführung weicht von der erwarteten ab",
            "La sortie de l'exécution diffère de la sortie attendue",
            "L'output dell'esecuzione differisce da quello atteso",
            "実行の出力が期待される出力と異なります",
            "运行输出与预期输出不同",
            "A saída da execução difere da esperada",
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
        assert_eq!(TITLES.len(), 41);
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
