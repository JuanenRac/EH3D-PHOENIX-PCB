// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the scripts of the second version, `PHOENIX-SCRIPT-043` onwards, with
//! their titles in the eight languages of the suite. They continue the numbering of
//! [`crate::codes`], whose [`Diag`](crate::codes::Diag) carries them, and are found by the same
//! `title` function.

use eh3d_phoenix_rules::Lang;

/// The text of the script is not valid: a character, a number or a construction that is not part
/// of the language.
pub const SYNTAX: &str = "PHOENIX-SCRIPT-043";
/// The script does not start with the line `phoenix-script/2`, or names another version.
pub const HEADER: &str = "PHOENIX-SCRIPT-044";
/// A variable is used before it is defined.
pub const UNDEFINED: &str = "PHOENIX-SCRIPT-045";
/// A variable is defined twice in one block.
pub const REDEFINED: &str = "PHOENIX-SCRIPT-046";
/// A function is not in the table of functions.
pub const UNKNOWN_FUNCTION: &str = "PHOENIX-SCRIPT-047";
/// A function got too few or too many arguments.
pub const ARGUMENTS: &str = "PHOENIX-SCRIPT-048";
/// A value has a type that the operation does not take.
pub const TYPE_MISMATCH: &str = "PHOENIX-SCRIPT-049";
/// A division by zero, or a number that leaves the range of 64 bits or of the model.
pub const ARITHMETIC: &str = "PHOENIX-SCRIPT-050";
/// The script took more steps of the interpreter than `max_steps`.
pub const STEP_LIMIT: &str = "PHOENIX-SCRIPT-051";
/// A loop turned more times than `max_loop_iterations`.
pub const ITERATION_LIMIT: &str = "PHOENIX-SCRIPT-052";
/// The values of the script grew beyond `max_value_cells`, `max_list_len` or `max_text_bytes`.
pub const MEMORY_LIMIT: &str = "PHOENIX-SCRIPT-053";
/// The output grew beyond `max_output_bytes`.
pub const OUTPUT_LIMIT: &str = "PHOENIX-SCRIPT-054";
/// Blocks or expressions are nested deeper than `max_block_depth` or `max_expr_depth`.
pub const NESTING_LIMIT: &str = "PHOENIX-SCRIPT-055";
/// A data file path is not allowed: absolute, with `..`, through a link, outside `data_dir`, or no
/// `data_dir` is set.
pub const DATA_PATH: &str = "PHOENIX-SCRIPT-056";
/// A data file is too large, does not exist, exists when it should be new, or the script used
/// more files than `max_data_files`.
pub const DATA_LIMIT: &str = "PHOENIX-SCRIPT-057";
/// The function does not apply to the kind of document the script runs on, or there is none.
pub const TARGET: &str = "PHOENIX-SCRIPT-058";
/// An object named by a script does not exist (any more).
pub const NOT_FOUND: &str = "PHOENIX-SCRIPT-059";
/// An `assert` was false, or `fail` was called.
pub const ASSERTION: &str = "PHOENIX-SCRIPT-060";
/// The editor refused an operation; nothing of the script is kept.
pub const REFUSED: &str = "PHOENIX-SCRIPT-061";
/// An index is outside the list or the text.
pub const INDEX: &str = "PHOENIX-SCRIPT-062";
/// A function needs a parts base and none was given, or the part is not in it.
pub const PARTS_BASE: &str = "PHOENIX-SCRIPT-063";
/// The static check estimates more steps than `max_steps`.
pub const ESTIMATE: &str = "PHOENIX-SCRIPT-064";

/// Every code of this module with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (SYNTAX, "Syntax error in the script"),
    (
        HEADER,
        "The script does not start with the line phoenix-script/2",
    ),
    (UNDEFINED, "Variable is not defined"),
    (REDEFINED, "Variable is already defined in this block"),
    (UNKNOWN_FUNCTION, "Unknown function"),
    (ARGUMENTS, "Wrong number of arguments"),
    (TYPE_MISMATCH, "A value has the wrong type"),
    (ARITHMETIC, "Division by zero or number out of range"),
    (STEP_LIMIT, "The script took more steps than the limit"),
    (ITERATION_LIMIT, "A loop turned more times than the limit"),
    (
        MEMORY_LIMIT,
        "The values of the script grew beyond the limit",
    ),
    (OUTPUT_LIMIT, "The output grew beyond the limit"),
    (NESTING_LIMIT, "Blocks or expressions are nested too deeply"),
    (DATA_PATH, "Data file path is not allowed"),
    (
        DATA_LIMIT,
        "Data file cannot be used or a file limit was reached",
    ),
    (TARGET, "The function does not apply to this document"),
    (NOT_FOUND, "Object not found"),
    (ASSERTION, "Assertion failed"),
    (REFUSED, "The editor refused the operation"),
    (INDEX, "Index out of range"),
    (
        PARTS_BASE,
        "No parts base was given or the part is not in it",
    ),
    (ESTIMATE, "The estimated steps are over the limit"),
];

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SYNTAX,
        [
            "Syntax error in the script",
            "Error de sintaxis en el script",
            "Syntaxfehler im Skript",
            "Erreur de syntaxe dans le script",
            "Errore di sintassi nello script",
            "スクリプトの構文エラー",
            "脚本语法错误",
            "Erro de sintaxe no guião",
        ],
    ),
    (
        HEADER,
        [
            "The script does not start with the line phoenix-script/2",
            "El script no empieza con la línea phoenix-script/2",
            "Das Skript beginnt nicht mit der Zeile phoenix-script/2",
            "Le script ne commence pas par la ligne phoenix-script/2",
            "Lo script non inizia con la riga phoenix-script/2",
            "スクリプトが phoenix-script/2 の行で始まっていません",
            "脚本未以 phoenix-script/2 行开头",
            "O guião não começa com a linha phoenix-script/2",
        ],
    ),
    (
        UNDEFINED,
        [
            "Variable is not defined",
            "La variable no está definida",
            "Variable ist nicht definiert",
            "La variable n'est pas définie",
            "La variabile non è definita",
            "変数が定義されていません",
            "变量未定义",
            "A variável não está definida",
        ],
    ),
    (
        REDEFINED,
        [
            "Variable is already defined in this block",
            "La variable ya está definida en este bloque",
            "Variable ist in diesem Block bereits definiert",
            "La variable est déjà définie dans ce bloc",
            "La variabile è già definita in questo blocco",
            "この変数はこのブロックで既に定義されています",
            "该变量已在此块中定义",
            "A variável já está definida neste bloco",
        ],
    ),
    (
        UNKNOWN_FUNCTION,
        [
            "Unknown function",
            "Función desconocida",
            "Unbekannte Funktion",
            "Fonction inconnue",
            "Funzione sconosciuta",
            "不明な関数です",
            "未知函数",
            "Função desconhecida",
        ],
    ),
    (
        ARGUMENTS,
        [
            "Wrong number of arguments",
            "Número de argumentos incorrecto",
            "Falsche Anzahl von Argumenten",
            "Nombre d'arguments incorrect",
            "Numero di argomenti errato",
            "引数の数が正しくありません",
            "参数数量不正确",
            "Número de argumentos incorreto",
        ],
    ),
    (
        TYPE_MISMATCH,
        [
            "A value has the wrong type",
            "Un valor tiene un tipo incorrecto",
            "Ein Wert hat den falschen Typ",
            "Une valeur a un type incorrect",
            "Un valore ha il tipo sbagliato",
            "値の型が正しくありません",
            "值的类型不正确",
            "Um valor tem o tipo errado",
        ],
    ),
    (
        ARITHMETIC,
        [
            "Division by zero or number out of range",
            "División entre cero o número fuera de rango",
            "Division durch null oder Zahl außerhalb des Bereichs",
            "Division par zéro ou nombre hors plage",
            "Divisione per zero o numero fuori intervallo",
            "ゼロ除算、または数値が範囲外です",
            "除以零或数值超出范围",
            "Divisão por zero ou número fora do intervalo",
        ],
    ),
    (
        STEP_LIMIT,
        [
            "The script took more steps than the limit",
            "El script dio más pasos que el límite",
            "Das Skript hat mehr Schritte gebraucht als erlaubt",
            "Le script a pris plus d'étapes que la limite",
            "Lo script ha fatto più passi del limite",
            "スクリプトのステップ数が上限を超えました",
            "脚本执行的步数超过了限制",
            "O guião deu mais passos do que o limite",
        ],
    ),
    (
        ITERATION_LIMIT,
        [
            "A loop turned more times than the limit",
            "Un bucle dio más vueltas que el límite",
            "Eine Schleife lief öfter als erlaubt",
            "Une boucle a tourné plus de fois que la limite",
            "Un ciclo ha girato più volte del limite",
            "ループの回数が上限を超えました",
            "循环次数超过了限制",
            "Um ciclo deu mais voltas do que o limite",
        ],
    ),
    (
        MEMORY_LIMIT,
        [
            "The values of the script grew beyond the limit",
            "Los valores del script crecieron más allá del límite",
            "Die Werte des Skripts sind über die Grenze hinaus gewachsen",
            "Les valeurs du script ont dépassé la limite",
            "I valori dello script sono cresciuti oltre il limite",
            "スクリプトの値が上限を超えて大きくなりました",
            "脚本的值增长超过了限制",
            "Os valores do guião cresceram para além do limite",
        ],
    ),
    (
        OUTPUT_LIMIT,
        [
            "The output grew beyond the limit",
            "La salida creció más allá del límite",
            "Die Ausgabe ist über die Grenze hinaus gewachsen",
            "La sortie a dépassé la limite",
            "L'output è cresciuto oltre il limite",
            "出力が上限を超えました",
            "输出增长超过了限制",
            "A saída cresceu para além do limite",
        ],
    ),
    (
        NESTING_LIMIT,
        [
            "Blocks or expressions are nested too deeply",
            "Bloques o expresiones anidados con demasiada profundidad",
            "Blöcke oder Ausdrücke sind zu tief verschachtelt",
            "Blocs ou expressions imbriqués trop profondément",
            "Blocchi o espressioni annidati troppo in profondità",
            "ブロックまたは式のネストが深すぎます",
            "块或表达式嵌套过深",
            "Blocos ou expressões aninhados com demasiada profundidade",
        ],
    ),
    (
        DATA_PATH,
        [
            "Data file path is not allowed",
            "La ruta del fichero de datos no está permitida",
            "Der Pfad der Datendatei ist nicht erlaubt",
            "Le chemin du fichier de données n'est pas autorisé",
            "Il percorso del file di dati non è consentito",
            "データファイルのパスは許可されていません",
            "不允许使用该数据文件路径",
            "O caminho do ficheiro de dados não é permitido",
        ],
    ),
    (
        DATA_LIMIT,
        [
            "Data file cannot be used or a file limit was reached",
            "No se puede usar el fichero de datos o se alcanzó un límite de ficheros",
            "Die Datendatei kann nicht verwendet werden oder eine Dateigrenze wurde erreicht",
            "Le fichier de données est inutilisable ou une limite de fichiers est atteinte",
            "Il file di dati non è utilizzabile o è stato raggiunto un limite di file",
            "データファイルを使用できないか、ファイル数の上限に達しました",
            "无法使用该数据文件，或已达到文件数量限制",
            "O ficheiro de dados não pode ser usado ou foi atingido um limite de ficheiros",
        ],
    ),
    (
        TARGET,
        [
            "The function does not apply to this document",
            "La función no se aplica a este documento",
            "Die Funktion gilt nicht für dieses Dokument",
            "La fonction ne s'applique pas à ce document",
            "La funzione non si applica a questo documento",
            "この関数はこのドキュメントには使えません",
            "该函数不适用于此文档",
            "A função não se aplica a este documento",
        ],
    ),
    (
        NOT_FOUND,
        [
            "Object not found",
            "Objeto no encontrado",
            "Objekt nicht gefunden",
            "Objet introuvable",
            "Oggetto non trovato",
            "オブジェクトが見つかりません",
            "未找到对象",
            "Objeto não encontrado",
        ],
    ),
    (
        ASSERTION,
        [
            "Assertion failed",
            "La aserción falló",
            "Zusicherung fehlgeschlagen",
            "L'assertion a échoué",
            "Asserzione non riuscita",
            "アサーションが失敗しました",
            "断言失败",
            "A asserção falhou",
        ],
    ),
    (
        REFUSED,
        [
            "The editor refused the operation",
            "El editor rechazó la operación",
            "Der Editor hat die Operation abgelehnt",
            "L'éditeur a refusé l'opération",
            "L'editor ha rifiutato l'operazione",
            "エディターが操作を拒否しました",
            "编辑器拒绝了该操作",
            "O editor recusou a operação",
        ],
    ),
    (
        INDEX,
        [
            "Index out of range",
            "Índice fuera de rango",
            "Index außerhalb des Bereichs",
            "Indice hors limites",
            "Indice fuori intervallo",
            "インデックスが範囲外です",
            "索引超出范围",
            "Índice fora do intervalo",
        ],
    ),
    (
        PARTS_BASE,
        [
            "No parts base was given or the part is not in it",
            "No se dio una base de piezas o la pieza no está en ella",
            "Es wurde keine Teiledatenbank angegeben oder das Teil fehlt darin",
            "Aucune base de pièces n'a été donnée ou la pièce n'y est pas",
            "Non è stata indicata una base di componenti o il componente non c'è",
            "部品ベースが指定されていないか、部品がその中にありません",
            "未提供元件库，或元件不在其中",
            "Não foi indicada uma base de peças ou a peça não existe nela",
        ],
    ),
    (
        ESTIMATE,
        [
            "The estimated steps are over the limit",
            "Los pasos estimados superan el límite",
            "Die geschätzten Schritte liegen über der Grenze",
            "Les étapes estimées dépassent la limite",
            "I passi stimati superano il limite",
            "推定ステップ数が上限を超えています",
            "估计的步数超过了限制",
            "Os passos estimados ultrapassam o limite",
        ],
    ),
];

/// The title of a code of this module in a language; `None` for any other code.
pub fn title(code: &str, lang: Lang) -> Option<&'static str> {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, row)| row[column])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_title_in_each_language_and_the_english_one_is_the_registry() {
        assert_eq!(REGISTRY.len(), TITLES.len());
        for ((code, english), (tcode, row)) in REGISTRY.iter().zip(TITLES.iter()) {
            assert_eq!(code, tcode);
            assert_eq!(*english, row[0]);
            for text in row {
                assert!(!text.is_empty(), "{code}");
            }
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
    }

    #[test]
    fn the_codes_continue_the_numbering_of_the_first_version_without_gaps() {
        for (i, (code, _)) in REGISTRY.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-SCRIPT-{:03}", i + 43));
        }
        assert_eq!(
            crate::codes::REGISTRY.len() + 1,
            43,
            "the first version ends at 042"
        );
    }

    #[test]
    fn the_message_of_a_diagnostic_uses_these_titles() {
        let d = crate::codes::Diag::new(UNDEFINED, 4, 9, "x");
        assert_eq!(
            d.message(Lang::Pt),
            "PHOENIX-SCRIPT-045: A variável não está definida [4:9]: x"
        );
        assert_eq!(
            d.message(Lang::En),
            "PHOENIX-SCRIPT-045: Variable is not defined [4:9]: x"
        );
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
        let doc = include_str!("../../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
