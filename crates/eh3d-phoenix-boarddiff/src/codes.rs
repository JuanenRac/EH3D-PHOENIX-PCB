// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the comparison of two boards, `PHOENIX-DIFF-<NNN>`, with their titles
//! in the eight languages of the suite, the error type that carries one and the difference of a
//! report.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The settings of the comparison are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-DIFF-001";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-DIFF-002";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-DIFF-003";
/// A limit of the comparison was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-DIFF-004";
/// A layer is in one board only.
pub const LAYER_DIFF: &str = "PHOENIX-DIFF-005";
/// A net is in one board only.
pub const NET_DIFF: &str = "PHOENIX-DIFF-006";
/// A part is in one board only.
pub const PART_DIFF: &str = "PHOENIX-DIFF-007";
/// A part has another value.
pub const VALUE_DIFF: &str = "PHOENIX-DIFF-008";
/// A part has another footprint.
pub const FOOTPRINT_DIFF: &str = "PHOENIX-DIFF-009";
/// A part is somewhere else, turned or on the other side.
pub const MOVE_DIFF: &str = "PHOENIX-DIFF-010";
/// A pin is on another net.
pub const PIN_DIFF: &str = "PHOENIX-DIFF-011";
/// The copper of a net on a layer is different.
pub const COPPER_DIFF: &str = "PHOENIX-DIFF-012";
/// A via is in one board only.
pub const VIA_DIFF: &str = "PHOENIX-DIFF-013";
/// A zone is in one board only or has another size.
pub const ZONE_DIFF: &str = "PHOENIX-DIFF-014";
/// The outline is different.
pub const OUTLINE_DIFF: &str = "PHOENIX-DIFF-015";
/// A reference is used by more than one part.
pub const DUPLICATE_REF: &str = "PHOENIX-DIFF-016";
/// The list of differences was cut.
pub const LIST_CUT: &str = "PHOENIX-DIFF-017";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Board comparison settings are not valid",
            "La configuración de la comparación de placas no es válida",
            "Die Einstellungen des Platinenvergleichs sind ungültig",
            "Les réglages de la comparaison de cartes ne sont pas valides",
            "Le impostazioni del confronto tra schede non sono valide",
            "基板比較の設定が無効です",
            "电路板比较设置无效",
            "As definições da comparação de placas não são válidas",
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
            "Non si può leggere un file di ingresso",
            "入力ファイルを読み込めません",
            "无法读取输入文件",
            "Não é possível ler um ficheiro de entrada",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the comparison was exceeded",
            "Se superó un límite de la comparación",
            "Eine Grenze des Vergleichs wurde überschritten",
            "Une limite de la comparaison a été dépassée",
            "È stato superato un limite del confronto",
            "比較の上限を超えました",
            "超出了比较的限制",
            "Foi excedido um limite da comparação",
        ],
    ),
    (
        LAYER_DIFF,
        [
            "Layer added or removed",
            "Capa añadida o quitada",
            "Lage hinzugefügt oder entfernt",
            "Couche ajoutée ou supprimée",
            "Strato aggiunto o rimosso",
            "層が追加または削除されました",
            "层已添加或删除",
            "Camada adicionada ou removida",
        ],
    ),
    (
        NET_DIFF,
        [
            "Net added or removed",
            "Red añadida o quitada",
            "Netz hinzugefügt oder entfernt",
            "Réseau ajouté ou supprimé",
            "Rete aggiunta o rimossa",
            "ネットが追加または削除されました",
            "网络已添加或删除",
            "Rede adicionada ou removida",
        ],
    ),
    (
        PART_DIFF,
        [
            "Part added or removed",
            "Pieza añadida o quitada",
            "Bauteil hinzugefügt oder entfernt",
            "Composant ajouté ou supprimé",
            "Componente aggiunto o rimosso",
            "部品が追加または削除されました",
            "元件已添加或删除",
            "Peça adicionada ou removida",
        ],
    ),
    (
        VALUE_DIFF,
        [
            "Value of a part changed",
            "Valor de una pieza cambiado",
            "Wert eines Bauteils geändert",
            "Valeur d'un composant modifiée",
            "Valore di un componente modificato",
            "部品の値が変更されました",
            "元件的值已更改",
            "Valor de uma peça alterado",
        ],
    ),
    (
        FOOTPRINT_DIFF,
        [
            "Footprint of a part changed",
            "Huella de una pieza cambiada",
            "Footprint eines Bauteils geändert",
            "Empreinte d'un composant modifiée",
            "Footprint di un componente modificato",
            "部品のフットプリントが変更されました",
            "元件的封装已更改",
            "Pegada de uma peça alterada",
        ],
    ),
    (
        MOVE_DIFF,
        [
            "Part moved, turned or put on the other side",
            "Pieza movida, girada o puesta en la otra cara",
            "Bauteil verschoben, gedreht oder auf die andere Seite gesetzt",
            "Composant déplacé, tourné ou mis sur l'autre face",
            "Componente spostato, ruotato o messo sull'altro lato",
            "部品が移動・回転または反対面へ移されました",
            "元件已移动、旋转或移到另一面",
            "Peça movida, rodada ou colocada na outra face",
        ],
    ),
    (
        PIN_DIFF,
        [
            "Pin connected to another net",
            "Pin conectado a otra red",
            "Pin mit einem anderen Netz verbunden",
            "Broche reliée à un autre réseau",
            "Pin collegato a un'altra rete",
            "ピンが別のネットに接続されました",
            "引脚已连接到其他网络",
            "Pino ligado a outra rede",
        ],
    ),
    (
        COPPER_DIFF,
        [
            "Copper of a net on a layer changed",
            "Cobre de una red en una capa cambiado",
            "Kupfer eines Netzes auf einer Lage geändert",
            "Cuivre d'un réseau sur une couche modifié",
            "Rame di una rete su uno strato modificato",
            "層上のネットの銅が変更されました",
            "某层上网络的铜已更改",
            "Cobre de uma rede numa camada alterado",
        ],
    ),
    (
        VIA_DIFF,
        [
            "Via added or removed",
            "Vía añadida o quitada",
            "Durchkontaktierung hinzugefügt oder entfernt",
            "Via ajouté ou supprimé",
            "Via aggiunta o rimossa",
            "ビアが追加または削除されました",
            "过孔已添加或删除",
            "Via adicionada ou removida",
        ],
    ),
    (
        ZONE_DIFF,
        [
            "Zone added, removed or changed",
            "Zona añadida, quitada o cambiada",
            "Zone hinzugefügt, entfernt oder geändert",
            "Zone ajoutée, supprimée ou modifiée",
            "Zona aggiunta, rimossa o modificata",
            "ゾーンが追加・削除または変更されました",
            "敷铜区已添加、删除或更改",
            "Zona adicionada, removida ou alterada",
        ],
    ),
    (
        OUTLINE_DIFF,
        [
            "Board outline changed",
            "Contorno de la placa cambiado",
            "Platinenumriss geändert",
            "Contour de la carte modifié",
            "Contorno della scheda modificato",
            "基板外形が変更されました",
            "板框已更改",
            "Contorno da placa alterado",
        ],
    ),
    (
        DUPLICATE_REF,
        [
            "A reference is used by more than one part",
            "Una referencia la usa más de una pieza",
            "Eine Referenz wird von mehr als einem Bauteil verwendet",
            "Une référence est utilisée par plus d'un composant",
            "Un riferimento è usato da più di un componente",
            "参照が複数の部品で使われています",
            "同一位号被多个元件使用",
            "Uma referência é usada por mais de uma peça",
        ],
    ),
    (
        LIST_CUT,
        [
            "The list of differences was cut",
            "La lista de diferencias se cortó",
            "Die Liste der Unterschiede wurde gekürzt",
            "La liste des différences a été coupée",
            "L'elenco delle differenze è stato troncato",
            "差分の一覧が途中で打ち切られました",
            "差异列表已被截断",
            "A lista de diferenças foi cortada",
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

/// A difference between two boards, or a note about the comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// The code, `PHOENIX-DIFF-<NNN>`.
    pub code: &'static str,
    /// The part, net, layer, pin or place it is about.
    pub subject: String,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Difference {
    /// A difference about a subject.
    pub fn new(
        code: &'static str,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Difference {
        Difference {
            code,
            subject: subject.into(),
            detail: detail.into(),
        }
    }

    /// The line in a language: the code, the title, the subject and the detail.
    pub fn line(&self, lang: Lang) -> String {
        let mut out = format!("{}: {}", self.code, title(self.code, lang));
        if !self.subject.is_empty() {
            out.push_str(&format!(" [{}]", self.subject));
        }
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

/// An error of the comparison: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffError {
    /// The code, `PHOENIX-DIFF-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl DiffError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> DiffError {
        DiffError {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the title and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let mut out = format!("{}: {}", self.code, title(self.code, lang));
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for DiffError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-DIFF-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
        }
        assert_eq!(TITLES.len(), 17);
    }

    #[test]
    fn no_title_of_another_language_is_a_copy_of_the_english_one() {
        for (code, row) in TITLES.iter() {
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
    }

    #[test]
    fn a_line_names_the_code_the_title_the_subject_and_the_detail() {
        let d = Difference::new(VALUE_DIFF, "R1", "10k -> 22k");
        assert_eq!(
            d.line(Lang::En),
            "PHOENIX-DIFF-008: Value of a part changed [R1]: 10k -> 22k"
        );
        assert_eq!(
            d.line(Lang::Pt),
            "PHOENIX-DIFF-008: Valor de uma peça alterado [R1]: 10k -> 22k"
        );
        let bare = Difference::new(LIST_CUT, "", "");
        assert_eq!(
            bare.line(Lang::En),
            "PHOENIX-DIFF-017: The list of differences was cut"
        );
    }

    #[test]
    fn the_message_of_an_error_names_the_code_the_title_and_the_detail() {
        let e = DiffError::new(INPUT_UNREADABLE, "a.json: gone");
        assert_eq!(
            e.message(Lang::En),
            "PHOENIX-DIFF-003: An input file cannot be read: a.json: gone"
        );
        assert_eq!(e.to_string(), "PHOENIX-DIFF-003: a.json: gone");
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
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
