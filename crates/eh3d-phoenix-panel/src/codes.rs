// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-PANEL-<NNN>` of the panelization, with their titles in the eight
//! languages of the suite, the error type that carries one and the note that a plan keeps.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The panel settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-PANEL-001";
/// The outline of the board cannot be used for a panel.
pub const OUTLINE_UNUSABLE: &str = "PHOENIX-PANEL-002";
/// The panel is larger than the limit or has more boards than the limit.
pub const PANEL_TOO_LARGE: &str = "PHOENIX-PANEL-003";
/// The tabs do not fit on the edge of a board.
pub const TABS_DONT_FIT: &str = "PHOENIX-PANEL-004";
/// A fiducial or a tooling hole does not fit in the frame.
pub const MARK_DOESNT_FIT: &str = "PHOENIX-PANEL-005";
/// Two marks overlap.
pub const MARKS_OVERLAP: &str = "PHOENIX-PANEL-006";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-PANEL-007";
/// An output file exists or is not allowed.
pub const OUTPUT_NOT_ALLOWED: &str = "PHOENIX-PANEL-008";
/// The outline is not a rectangle; its bounding box is used.
pub const OUTLINE_NOT_RECTANGLE: &str = "PHOENIX-PANEL-009";
/// The gap is ignored because the boards are separated by V-score lines.
pub const GAP_IGNORED: &str = "PHOENIX-PANEL-010";
/// The marks were left out because the panel has no frame.
pub const MARKS_SKIPPED: &str = "PHOENIX-PANEL-011";
/// The mouse bite holes do not fit in the tab.
pub const BITES_DONT_FIT: &str = "PHOENIX-PANEL-012";
/// The list of boards or their placement is not valid.
pub const BOARDS_INVALID: &str = "PHOENIX-PANEL-013";
/// Two boards of the panel overlap or are closer than the gap.
pub const BOARDS_OVERLAP: &str = "PHOENIX-PANEL-014";
/// A V-score line passes too close to a pad, a track, a via or a zone.
pub const VSCORE_NEAR_COPPER: &str = "PHOENIX-PANEL-015";
/// The V-score lines cannot separate every board from its neighbours.
pub const VSCORE_CANNOT_SEPARATE: &str = "PHOENIX-PANEL-016";
/// A local fiducial does not fit on its board.
pub const LOCAL_MARK_DOESNT_FIT: &str = "PHOENIX-PANEL-017";
/// The boards of the panel do not have the same layers.
pub const LAYERS_DIFFER: &str = "PHOENIX-PANEL-018";
/// The files of the panel read back differ from the plan.
pub const ROUNDTRIP_MISMATCH: &str = "PHOENIX-PANEL-019";
/// A board has no tab that holds it to the panel.
pub const BOARD_NOT_JOINED: &str = "PHOENIX-PANEL-020";
/// The layer files of the panel could not be written.
pub const EXPORT_FAILED: &str = "PHOENIX-PANEL-021";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Panel settings are not valid",
            "La configuración del panel no es válida",
            "Die Einstellungen des Nutzens sind ungültig",
            "Les réglages du panneau ne sont pas valides",
            "Le impostazioni del pannello non sono valide",
            "パネル設定が無効です",
            "拼板设置无效",
            "As definições do painel não são válidas",
        ],
    ),
    (
        OUTLINE_UNUSABLE,
        [
            "Board outline cannot be used for a panel",
            "El contorno de la placa no sirve para un panel",
            "Die Platinenkontur ist für einen Nutzen nicht brauchbar",
            "Le contour de la carte est inutilisable pour un panneau",
            "Il contorno della scheda non è utilizzabile per un pannello",
            "基板外形はパネルに使用できません",
            "电路板外形无法用于拼板",
            "O contorno da placa não pode ser usado num painel",
        ],
    ),
    (
        PANEL_TOO_LARGE,
        [
            "Panel too large or too many boards",
            "Panel demasiado grande o demasiadas placas",
            "Nutzen zu groß oder zu viele Platinen",
            "Panneau trop grand ou trop de cartes",
            "Pannello troppo grande o troppe schede",
            "パネルが大きすぎるか基板が多すぎます",
            "拼板过大或电路板过多",
            "Painel demasiado grande ou placas a mais",
        ],
    ),
    (
        TABS_DONT_FIT,
        [
            "Tabs do not fit on the edge",
            "Las pestañas no caben en el borde",
            "Die Stege passen nicht auf die Kante",
            "Les ponts ne tiennent pas sur le bord",
            "Le linguette non entrano nel bordo",
            "タブが辺に収まりません",
            "连接桥放不进板边",
            "As patilhas não cabem na aresta",
        ],
    ),
    (
        MARK_DOESNT_FIT,
        [
            "Mark does not fit in the frame",
            "La marca no cabe en el marco",
            "Die Marke passt nicht in den Rahmen",
            "Le repère ne tient pas dans le cadre",
            "Il riferimento non entra nella cornice",
            "マークがフレームに収まりません",
            "标记放不进边框",
            "A marca não cabe na moldura",
        ],
    ),
    (
        MARKS_OVERLAP,
        [
            "Marks overlap",
            "Las marcas se solapan",
            "Marken überlappen sich",
            "Les repères se chevauchent",
            "I riferimenti si sovrappongono",
            "マークが重なっています",
            "标记相互重叠",
            "As marcas sobrepõem-se",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "Command option not valid",
            "Opción de comando no válida",
            "Ungültige Befehlsoption",
            "Option de commande non valide",
            "Opzione del comando non valida",
            "コマンドのオプションが無効です",
            "命令选项无效",
            "Opção do comando não válida",
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
            "不允许该输出文件",
            "O ficheiro de saída não é permitido",
        ],
    ),
    (
        OUTLINE_NOT_RECTANGLE,
        [
            "Outline is not a rectangle",
            "El contorno no es un rectángulo",
            "Die Kontur ist kein Rechteck",
            "Le contour n'est pas un rectangle",
            "Il contorno non è un rettangolo",
            "外形は長方形ではありません",
            "外形不是矩形",
            "O contorno não é um retângulo",
        ],
    ),
    (
        GAP_IGNORED,
        [
            "Gap ignored for V-score",
            "Separación ignorada con V-score",
            "Abstand beim V-Ritzen ignoriert",
            "Écart ignoré pour la rainure en V",
            "Distanza ignorata con il V-score",
            "Vカットでは間隔を無視しました",
            "V形槽工艺忽略间距",
            "Espaçamento ignorado no V-score",
        ],
    ),
    (
        MARKS_SKIPPED,
        [
            "Marks skipped: the panel has no frame",
            "Marcas omitidas: el panel no tiene marco",
            "Marken übersprungen: der Nutzen hat keinen Rahmen",
            "Repères omis : le panneau n'a pas de cadre",
            "Riferimenti omessi: il pannello non ha cornice",
            "マークを省略しました：パネルにフレームがありません",
            "已跳过标记：拼板没有边框",
            "Marcas omitidas: o painel não tem moldura",
        ],
    ),
    (
        BITES_DONT_FIT,
        [
            "Mouse bite holes do not fit the tab",
            "Los agujeros de rotura no caben en la pestaña",
            "Die Bohrungen der Sollbruchstelle passen nicht in den Steg",
            "Les trous de rupture ne tiennent pas dans le pont",
            "I fori di rottura non entrano nella linguetta",
            "ミシン目の穴がタブに収まりません",
            "邮票孔放不进连接桥",
            "Os furos de rutura não cabem na patilha",
        ],
    ),
    (
        BOARDS_INVALID,
        [
            "Board list or placement is not valid",
            "La lista de placas o su colocación no es válida",
            "Platinenliste oder Platzierung ungültig",
            "La liste des cartes ou leur placement n'est pas valide",
            "L'elenco delle schede o la loro posizione non è valido",
            "基板リストまたは配置が無効です",
            "电路板列表或位置无效",
            "A lista de placas ou a sua posição não é válida",
        ],
    ),
    (
        BOARDS_OVERLAP,
        [
            "Boards overlap or are too close",
            "Las placas se solapan o están demasiado cerca",
            "Platinen überlappen sich oder sind zu nah",
            "Les cartes se chevauchent ou sont trop proches",
            "Le schede si sovrappongono o sono troppo vicine",
            "基板が重なっているか近すぎます",
            "电路板重叠或距离过近",
            "As placas sobrepõem-se ou estão demasiado próximas",
        ],
    ),
    (
        VSCORE_NEAR_COPPER,
        [
            "V-score line too close to copper",
            "Línea de V-score demasiado cerca del cobre",
            "V-Ritzlinie zu nah am Kupfer",
            "Ligne de rainure en V trop proche du cuivre",
            "Linea di V-score troppo vicina al rame",
            "Vカット線が銅箔に近すぎます",
            "V形槽线距铜箔过近",
            "Linha de V-score demasiado perto do cobre",
        ],
    ),
    (
        VSCORE_CANNOT_SEPARATE,
        [
            "V-score lines cannot separate the boards",
            "Las líneas de V-score no pueden separar las placas",
            "V-Ritzlinien können die Platinen nicht trennen",
            "Les rainures en V ne peuvent pas séparer les cartes",
            "Le linee di V-score non possono separare le schede",
            "Vカット線では基板を分離できません",
            "V形槽线无法分离这些电路板",
            "As linhas de V-score não conseguem separar as placas",
        ],
    ),
    (
        LOCAL_MARK_DOESNT_FIT,
        [
            "Local fiducial does not fit on the board",
            "El fiducial local no cabe en la placa",
            "Die lokale Passmarke passt nicht auf die Platine",
            "Le repère local ne tient pas sur la carte",
            "Il fiducial locale non entra nella scheda",
            "ローカルフィデューシャルが基板に収まりません",
            "局部基准点放不进电路板",
            "O fiducial local não cabe na placa",
        ],
    ),
    (
        LAYERS_DIFFER,
        [
            "Boards of the panel have different layers",
            "Las placas del panel tienen capas distintas",
            "Die Platinen des Nutzens haben unterschiedliche Lagen",
            "Les cartes du panneau ont des couches différentes",
            "Le schede del pannello hanno strati diversi",
            "パネルの基板で層構成が異なります",
            "拼板中的电路板层结构不同",
            "As placas do painel têm camadas diferentes",
        ],
    ),
    (
        ROUNDTRIP_MISMATCH,
        [
            "Panel files differ from the plan when read back",
            "Los ficheros del panel difieren del plan al releerlos",
            "Die Dateien des Nutzens weichen beim Zurücklesen vom Plan ab",
            "Les fichiers du panneau diffèrent du plan à la relecture",
            "I file del pannello differiscono dal piano alla rilettura",
            "読み戻したパネルファイルが計画と異なります",
            "回读的拼板文件与计划不一致",
            "Os ficheiros do painel diferem do plano ao serem relidos",
        ],
    ),
    (
        BOARD_NOT_JOINED,
        [
            "Board is not joined to the panel",
            "La placa no está unida al panel",
            "Die Platine ist nicht mit dem Nutzen verbunden",
            "La carte n'est pas reliée au panneau",
            "La scheda non è collegata al pannello",
            "基板がパネルに接続されていません",
            "电路板未与拼板连接",
            "A placa não está ligada ao painel",
        ],
    ),
    (
        EXPORT_FAILED,
        [
            "The layers of the panel could not be written",
            "No se pudieron escribir las capas del panel",
            "Die Lagen des Nutzens konnten nicht geschrieben werden",
            "Les couches du panneau n'ont pas pu être écrites",
            "Non è stato possibile scrivere gli strati del pannello",
            "パネルの層を書き出せませんでした",
            "无法写出拼板的各层",
            "Não foi possível escrever as camadas do painel",
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

/// An error of the panelization: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelError {
    /// The code, `PHOENIX-PANEL-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl PanelError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> PanelError {
        PanelError {
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

impl fmt::Display for PanelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for PanelError {}

/// A note that a plan keeps for what it changed or left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// The code.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Note {
    /// The text in a language.
    pub fn message(&self, lang: Lang) -> String {
        let mut out = format!("{}: {}", self.code, title(self.code, lang));
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-PANEL-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
        }
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
    fn the_message_names_the_code_the_title_and_the_detail() {
        let e = PanelError::new(TABS_DONT_FIT, "2 tabs of 3 mm on 4 mm");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-PANEL-004: As patilhas não cabem na aresta: 2 tabs of 3 mm on 4 mm"
        );
        assert_eq!(e.to_string(), "PHOENIX-PANEL-004: 2 tabs of 3 mm on 4 mm");
        let note = Note {
            code: GAP_IGNORED,
            detail: String::new(),
        };
        assert_eq!(
            note.message(Lang::En),
            "PHOENIX-PANEL-010: Gap ignored for V-score"
        );
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
