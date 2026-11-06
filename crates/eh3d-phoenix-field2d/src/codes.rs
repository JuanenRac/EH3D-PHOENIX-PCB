// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the field solver, `PHOENIX-FIELD-<NNN>`, with their titles in the eight
//! languages of the suite and the error type that carries one.

use std::fmt;

use eh3d_phoenix_stackup::Language;

/// The settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-FIELD-001";
/// The geometry of the cross section is not valid.
pub const SHAPE_INVALID: &str = "PHOENIX-FIELD-002";
/// The mesh would have more cells than the limit.
pub const MESH_TOO_LARGE: &str = "PHOENIX-FIELD-003";
/// The solver did not reach its tolerance within the limit of iterations.
pub const NOT_CONVERGED: &str = "PHOENIX-FIELD-004";
/// The line cannot be taken from the stack-up.
pub const LINE_UNAVAILABLE: &str = "PHOENIX-FIELD-005";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-FIELD-006";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-FIELD-007";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-FIELD-008";
/// The field solver and the closed formula differ by more than the limit.
pub const DIFFERS_FROM_FORMULA: &str = "PHOENIX-FIELD-009";
/// A net named on the command line has no track on the board.
pub const NET_NOT_FOUND: &str = "PHOENIX-FIELD-010";
/// The solder mask over the line is left out of the model.
pub const MASK_LEFT_OUT: &str = "PHOENIX-FIELD-011";
/// The model is an approximation of the real structure.
pub const APPROXIMATE: &str = "PHOENIX-FIELD-012";
/// Two conductors are closer than one cell of the mesh.
pub const CONDUCTORS_TOUCH: &str = "PHOENIX-FIELD-013";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Field solver settings are not valid",
            "La configuración del solucionador de campo no es válida",
            "Die Einstellungen des Feldlösers sind ungültig",
            "Les réglages du solveur de champ ne sont pas valides",
            "Le impostazioni del risolutore di campo non sono valide",
            "電界ソルバーの設定が無効です",
            "场求解器设置无效",
            "As definições do resolvedor de campo não são válidas",
        ],
    ),
    (
        SHAPE_INVALID,
        [
            "The geometry of the cross section is not valid",
            "La geometría de la sección no es válida",
            "Die Geometrie des Querschnitts ist ungültig",
            "La géométrie de la section n'est pas valide",
            "La geometria della sezione non è valida",
            "断面の形状が無効です",
            "截面几何形状无效",
            "A geometria da secção não é válida",
        ],
    ),
    (
        MESH_TOO_LARGE,
        [
            "The mesh has more cells than the limit",
            "La malla tiene más celdas que el límite",
            "Das Netz hat mehr Zellen als die Grenze",
            "Le maillage a plus de cellules que la limite",
            "La maglia ha più celle del limite",
            "メッシュのセル数が上限を超えています",
            "网格单元数超过限制",
            "A malha tem mais células do que o limite",
        ],
    ),
    (
        NOT_CONVERGED,
        [
            "The solver did not reach its tolerance",
            "El solucionador no alcanzó su tolerancia",
            "Der Löser hat seine Toleranz nicht erreicht",
            "Le solveur n'a pas atteint sa tolérance",
            "Il risolutore non ha raggiunto la sua tolleranza",
            "ソルバーが許容誤差に達しませんでした",
            "求解器未达到其容差",
            "O resolvedor não atingiu a sua tolerância",
        ],
    ),
    (
        LINE_UNAVAILABLE,
        [
            "The line cannot be taken from the stack-up",
            "La línea no se puede obtener del apilado",
            "Die Leitung lässt sich nicht aus dem Lagenaufbau entnehmen",
            "La ligne ne peut pas être tirée de l'empilage",
            "La linea non si può ricavare dallo stack-up",
            "スタックアップから線路を取り出せません",
            "无法从叠层中取得该线路",
            "A linha não se pode obter do empilhamento",
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
        OUTPUT_FAILED,
        [
            "The output file exists or cannot be written",
            "El fichero de salida existe o no se puede escribir",
            "Die Ausgabedatei existiert oder lässt sich nicht schreiben",
            "Le fichier de sortie existe ou ne peut pas être écrit",
            "Il file di uscita esiste o non si può scrivere",
            "出力ファイルが存在するか、書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "An input file cannot be read",
            "No se puede leer un fichero de entrada",
            "Eine Eingabedatei lässt sich nicht lesen",
            "Un fichier d'entrée ne peut pas être lu",
            "Non si può leggere un file di ingresso",
            "入力ファイルを読み込めません",
            "无法读取输入文件",
            "Não é possível ler um ficheiro de entrada",
        ],
    ),
    (
        DIFFERS_FROM_FORMULA,
        [
            "The field solver and the closed formula differ by more than the limit",
            "El solucionador de campo y la fórmula cerrada difieren más que el límite",
            "Feldlöser und geschlossene Formel weichen stärker als die Grenze ab",
            "Le solveur de champ et la formule fermée diffèrent plus que la limite",
            "Il risolutore di campo e la formula chiusa differiscono più del limite",
            "電界ソルバーと閉形式の式の差が上限を超えています",
            "场求解器与封闭公式的差异超过限值",
            "O resolvedor de campo e a fórmula fechada diferem mais do que o limite",
        ],
    ),
    (
        NET_NOT_FOUND,
        [
            "The net has no track on the board",
            "La red no tiene pistas en la placa",
            "Das Netz hat keine Leiterbahn auf der Platine",
            "Le réseau n'a aucune piste sur la carte",
            "La rete non ha piste sulla scheda",
            "ネットに基板上の配線がありません",
            "该网络在电路板上没有走线",
            "A rede não tem pistas na placa",
        ],
    ),
    (
        MASK_LEFT_OUT,
        [
            "The solder mask over the line is left out of the model",
            "La máscara de soldadura sobre la línea no entra en el modelo",
            "Die Lötstoppmaske über der Leitung bleibt im Modell unberücksichtigt",
            "Le vernis épargne sur la ligne n'est pas dans le modèle",
            "La maschera di saldatura sulla linea non è nel modello",
            "線路上のソルダーマスクはモデルに含まれません",
            "线路上方的阻焊层未纳入模型",
            "A máscara de soldadura sobre a linha fica fora do modelo",
        ],
    ),
    (
        APPROXIMATE,
        [
            "The model is an approximation of the real structure",
            "El modelo es una aproximación de la estructura real",
            "Das Modell ist eine Näherung der realen Struktur",
            "Le modèle est une approximation de la structure réelle",
            "Il modello è un'approssimazione della struttura reale",
            "モデルは実際の構造の近似です",
            "该模型是对实际结构的近似",
            "O modelo é uma aproximação da estrutura real",
        ],
    ),
    (
        CONDUCTORS_TOUCH,
        [
            "Two conductors are closer than one cell of the mesh",
            "Dos conductores están más cerca que una celda de la malla",
            "Zwei Leiter liegen näher als eine Zelle des Netzes",
            "Deux conducteurs sont plus proches qu'une cellule du maillage",
            "Due conduttori sono più vicini di una cella della maglia",
            "2つの導体がメッシュの1セルより近接しています",
            "两个导体之间的距离小于一个网格单元",
            "Dois condutores estão mais perto do que uma célula da malha",
        ],
    ),
];

/// The title of a code in a language.
pub fn title(code: &str, language: Language) -> Option<&'static str> {
    let column = Language::ALL.iter().position(|l| *l == language)?;
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .and_then(|(_, titles)| titles.get(column).copied())
}

/// An error of the field solver: a code and the detail in English.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldError {
    /// The code, `PHOENIX-FIELD-<NNN>`.
    pub code: &'static str,
    /// What went wrong.
    pub detail: String,
}

impl FieldError {
    /// An error with a code and a detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> FieldError {
        FieldError {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, its title and then the detail in English.
    pub fn message(&self, language: Language) -> String {
        let head = title(self.code, language).unwrap_or("");
        format!("{}: {head}: {}", self.code, self.detail)
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for FieldError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_title_in_each_of_the_eight_languages() {
        assert_eq!(TITLES.len(), 13);
        for (index, (code, titles)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-FIELD-{:03}", index + 1));
            for text in titles {
                assert!(!text.trim().is_empty(), "{code}");
            }
            for language in Language::ALL {
                assert!(title(code, language).is_some());
            }
        }
    }

    #[test]
    fn a_message_carries_the_code_the_title_and_the_detail() {
        let error = FieldError::new(MESH_TOO_LARGE, "5 cells");
        let text = error.message(Language::Pt);
        assert!(text.starts_with("PHOENIX-FIELD-003: "));
        assert!(text.contains("A malha"));
        assert!(text.ends_with("5 cells"));
        assert_eq!(error.to_string(), "PHOENIX-FIELD-003: 5 cells");
    }

    #[test]
    fn an_unknown_code_has_no_title() {
        assert_eq!(title("PHOENIX-FIELD-999", Language::En), None);
    }
}
