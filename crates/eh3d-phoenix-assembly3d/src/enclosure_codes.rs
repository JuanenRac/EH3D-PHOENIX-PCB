// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the check against an enclosure, `PHOENIX-ENCL-<NNN>`, with their titles
//! in the eight languages of the suite. The error and the finding that carry them are those of
//! the assembly (`crate::codes`), whose `title` finds these codes too.

/// The file of the enclosure is not valid.
pub const SPEC_INVALID: &str = "PHOENIX-ENCL-001";
/// An input file of the check cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-ENCL-002";
/// An option of the command is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-ENCL-003";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-ENCL-004";
/// The board or a component is outside the walls or too close to them.
pub const WALL: &str = "PHOENIX-ENCL-005";
/// The board or a component is through the lid or too close to it.
pub const LID: &str = "PHOENIX-ENCL-006";
/// The board or a component is through the floor or too close to it.
pub const FLOOR: &str = "PHOENIX-ENCL-007";
/// A component collides with a screw post.
pub const POST_COLLISION: &str = "PHOENIX-ENCL-008";
/// A screw post has no hole in the board.
pub const POST_NO_HOLE: &str = "PHOENIX-ENCL-009";
/// A screw post is outside the board outline.
pub const POST_OUTSIDE: &str = "PHOENIX-ENCL-010";
/// The least clearances to the enclosure.
pub const CLEARANCES: &str = "PHOENIX-ENCL-011";
/// A hole was left out of the STEP solid.
pub const HOLE_SKIPPED: &str = "PHOENIX-ENCL-012";
/// The STEP solid of the board is simplified.
pub const SOLID_SIMPLIFIED: &str = "PHOENIX-ENCL-013";
/// The inner contour of the enclosure cannot be used.
pub const CONTOUR_UNUSABLE: &str = "PHOENIX-ENCL-014";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SPEC_INVALID,
        [
            "Enclosure file is not valid",
            "El fichero de la carcasa no es válido",
            "Die Gehäusedatei ist ungültig",
            "Le fichier du boîtier n'est pas valide",
            "Il file dell'involucro non è valido",
            "筐体ファイルが無効です",
            "外壳文件无效",
            "O ficheiro da caixa não é válido",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "Input file of the enclosure check cannot be read",
            "No se puede leer un fichero de entrada de la comprobación de la carcasa",
            "Eine Eingabedatei der Gehäuseprüfung kann nicht gelesen werden",
            "Un fichier d'entrée de la vérification du boîtier est illisible",
            "Un file di ingresso della verifica dell'involucro non è leggibile",
            "筐体チェックの入力ファイルを読み込めません",
            "无法读取外壳检查的输入文件",
            "Não é possível ler um ficheiro de entrada da verificação da caixa",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "An option of the enclosure command is not valid",
            "Una opción del comando de la carcasa no es válida",
            "Eine Option des Gehäusebefehls ist ungültig",
            "Une option de la commande du boîtier n'est pas valide",
            "Un'opzione del comando dell'involucro non è valida",
            "筐体コマンドのオプションが無効です",
            "外壳命令的选项无效",
            "Uma opção do comando da caixa não é válida",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "Output file of the enclosure check exists or cannot be written",
            "El fichero de salida de la comprobación de la carcasa existe o no se puede escribir",
            "Die Ausgabedatei der Gehäuseprüfung existiert oder kann nicht geschrieben werden",
            "Le fichier de sortie de la vérification du boîtier existe ou ne peut pas être écrit",
            "Il file di uscita della verifica dell'involucro esiste o non può essere scritto",
            "筐体チェックの出力ファイルが存在するか書き込めません",
            "外壳检查的输出文件已存在或无法写入",
            "O ficheiro de saída da verificação da caixa existe ou não pode ser escrito",
        ],
    ),
    (
        WALL,
        [
            "Board or component is outside the walls or too close to them",
            "La placa o un componente está fuera de las paredes o demasiado cerca de ellas",
            "Platine oder Bauteil liegt außerhalb der Wände oder zu nah an ihnen",
            "La carte ou un composant est hors des parois ou trop près d'elles",
            "La scheda o un componente è fuori dalle pareti o troppo vicino ad esse",
            "基板または部品が壁の外側、または壁に近すぎます",
            "电路板或元件在壁外或离壁太近",
            "A placa ou um componente está fora das paredes ou demasiado perto delas",
        ],
    ),
    (
        LID,
        [
            "Board or component is through the lid or too close to it",
            "La placa o un componente atraviesa la tapa o está demasiado cerca de ella",
            "Platine oder Bauteil durchdringt den Deckel oder ist zu nah daran",
            "La carte ou un composant traverse le couvercle ou en est trop près",
            "La scheda o un componente attraversa il coperchio o è troppo vicino",
            "基板または部品が蓋を貫通、または蓋に近すぎます",
            "电路板或元件穿过盖子或离盖子太近",
            "A placa ou um componente atravessa a tampa ou está demasiado perto dela",
        ],
    ),
    (
        FLOOR,
        [
            "Board or component is through the floor or too close to it",
            "La placa o un componente atraviesa el suelo o está demasiado cerca de él",
            "Platine oder Bauteil durchdringt den Boden oder ist zu nah daran",
            "La carte ou un composant traverse le fond ou en est trop près",
            "La scheda o un componente attraversa il fondo o è troppo vicino",
            "基板または部品が底面を貫通、または底面に近すぎます",
            "电路板或元件穿过底面或离底面太近",
            "A placa ou um componente atravessa o fundo ou está demasiado perto dele",
        ],
    ),
    (
        POST_COLLISION,
        [
            "Component collides with a screw post",
            "Un componente colisiona con un poste de tornillo",
            "Ein Bauteil kollidiert mit einem Schraubdom",
            "Un composant entre en collision avec un plot de vis",
            "Un componente collide con un supporto per vite",
            "部品がねじ柱と干渉しています",
            "元件与螺丝柱发生碰撞",
            "Um componente colide com um poste de parafuso",
        ],
    ),
    (
        POST_NO_HOLE,
        [
            "Screw post has no hole in the board",
            "El poste de tornillo no tiene agujero en la placa",
            "Der Schraubdom hat kein Loch in der Platine",
            "Le plot de vis n'a pas de trou dans la carte",
            "Il supporto per vite non ha un foro nella scheda",
            "ねじ柱に対応する基板の穴がありません",
            "螺丝柱在电路板上没有对应的孔",
            "O poste de parafuso não tem furo na placa",
        ],
    ),
    (
        POST_OUTSIDE,
        [
            "Screw post is outside the board outline",
            "El poste de tornillo está fuera del contorno de la placa",
            "Der Schraubdom liegt außerhalb des Platinenumrisses",
            "Le plot de vis est hors du contour de la carte",
            "Il supporto per vite è fuori dal contorno della scheda",
            "ねじ柱が基板外形の外側にあります",
            "螺丝柱在电路板轮廓之外",
            "O poste de parafuso está fora do contorno da placa",
        ],
    ),
    (
        CLEARANCES,
        [
            "Least clearances to the enclosure",
            "Holguras mínimas a la carcasa",
            "Kleinste Abstände zum Gehäuse",
            "Jeux minimaux avec le boîtier",
            "Giochi minimi rispetto all'involucro",
            "筐体までの最小クリアランス",
            "到外壳的最小间隙",
            "Folgas mínimas à caixa",
        ],
    ),
    (
        HOLE_SKIPPED,
        [
            "Hole left out of the STEP solid",
            "Agujero omitido en el sólido STEP",
            "Loch im STEP-Körper ausgelassen",
            "Trou omis dans le solide STEP",
            "Foro omesso dal solido STEP",
            "STEPソリッドから穴を除外しました",
            "STEP实体中略去了孔",
            "Furo deixado de fora do sólido STEP",
        ],
    ),
    (
        SOLID_SIMPLIFIED,
        [
            "The STEP solid of the board is simplified",
            "El sólido STEP de la placa está simplificado",
            "Der STEP-Körper der Platine ist vereinfacht",
            "Le solide STEP de la carte est simplifié",
            "Il solido STEP della scheda è semplificato",
            "基板のSTEPソリッドは簡略化されています",
            "电路板的STEP实体已简化",
            "O sólido STEP da placa está simplificado",
        ],
    ),
    (
        CONTOUR_UNUSABLE,
        [
            "Inner contour of the enclosure cannot be used",
            "El contorno interior de la carcasa no se puede usar",
            "Die Innenkontur des Gehäuses ist nicht verwendbar",
            "Le contour intérieur du boîtier est inutilisable",
            "Il contorno interno dell'involucro non è utilizzabile",
            "筐体の内側輪郭を使用できません",
            "无法使用外壳的内轮廓",
            "O contorno interior da caixa não pode ser usado",
        ],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-ENCL-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
        assert_eq!(TITLES.len(), 14);
    }

    #[test]
    fn the_title_of_the_assembly_finds_these_codes_too() {
        use eh3d_phoenix_rules::Lang;
        assert_eq!(
            crate::codes::title(WALL, Lang::En),
            "Board or component is outside the walls or too close to them"
        );
        assert_eq!(
            crate::codes::title(CLEARANCES, Lang::Pt),
            "Folgas mínimas à caixa"
        );
        assert_eq!(crate::codes::title("PHOENIX-ENCL-999", Lang::En), "");
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, row) in TITLES {
            let line = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(line.contains(row[0]), "{code}: {line}");
        }
    }
}
