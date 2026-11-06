// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the bill of materials, `PHOENIX-BOM-<NNN>`, with their titles in the
//! eight languages of the suite, the error type that carries one and the finding of a report.

use std::fmt;

use eh3d_phoenix_rules::Lang;

/// The cost settings are not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-BOM-001";
/// An input file is larger than the limit.
pub const FILE_TOO_LARGE: &str = "PHOENIX-BOM-002";
/// The price file is not valid.
pub const PRICES_INVALID: &str = "PHOENIX-BOM-003";
/// The file of part data is not valid.
pub const PARTS_INVALID: &str = "PHOENIX-BOM-004";
/// A part has no price.
pub const NO_PRICE: &str = "PHOENIX-BOM-005";
/// A part has no footprint.
pub const NO_FOOTPRINT: &str = "PHOENIX-BOM-006";
/// A price in another currency was ignored.
pub const CURRENCY_IGNORED: &str = "PHOENIX-BOM-007";
/// The quantity to buy was raised to the minimum of the supplier.
pub const MINIMUM_RAISED: &str = "PHOENIX-BOM-008";
/// A part that is not fitted is left out.
pub const NOT_FITTED: &str = "PHOENIX-BOM-009";
/// A component refers to a footprint that is not on the board.
pub const FOOTPRINT_MISSING: &str = "PHOENIX-BOM-010";
/// A part of the schematic is not placed on the board.
pub const NOT_PLACED: &str = "PHOENIX-BOM-011";
/// The board cannot give a PCB cost.
pub const PCB_NOT_COSTED: &str = "PHOENIX-BOM-012";
/// Two prices for one quantity of one part disagree.
pub const PRICE_CONFLICT: &str = "PHOENIX-BOM-013";
/// A command option is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-BOM-014";
/// An output file exists or cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-BOM-015";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-BOM-016";
/// A limit was exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-BOM-017";
/// The PCB cost is an estimate from example tariffs.
pub const PCB_ESTIMATE: &str = "PHOENIX-BOM-018";
/// A quantity of an imported bill differs from the number of its references.
pub const IMPORT_QUANTITY: &str = "PHOENIX-BOM-019";
/// A reference is in two lines of an imported bill.
pub const IMPORT_DUPLICATE: &str = "PHOENIX-BOM-020";
/// An imported bill and the board disagree about a part.
pub const IMPORT_DISAGREES: &str = "PHOENIX-BOM-021";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Cost settings are not valid",
            "La configuración de costes no es válida",
            "Die Kosteneinstellungen sind ungültig",
            "Les réglages des coûts ne sont pas valides",
            "Le impostazioni dei costi non sono valide",
            "コスト設定が無効です",
            "成本设置无效",
            "As definições de custos não são válidas",
        ],
    ),
    (
        FILE_TOO_LARGE,
        [
            "Input file is larger than the limit",
            "El archivo de entrada supera el límite",
            "Die Eingabedatei ist größer als das Limit",
            "Le fichier d'entrée dépasse la limite",
            "Il file di input supera il limite",
            "入力ファイルが上限を超えています",
            "输入文件超过限制",
            "O ficheiro de entrada excede o limite",
        ],
    ),
    (
        PRICES_INVALID,
        [
            "Price file is not valid",
            "El archivo de precios no es válido",
            "Die Preisdatei ist ungültig",
            "Le fichier de prix n'est pas valide",
            "Il file dei prezzi non è valido",
            "価格ファイルが無効です",
            "价格文件无效",
            "O ficheiro de preços não é válido",
        ],
    ),
    (
        PARTS_INVALID,
        [
            "File of part data is not valid",
            "El archivo de datos de piezas no es válido",
            "Die Datei der Bauteildaten ist ungültig",
            "Le fichier des données de pièces n'est pas valide",
            "Il file dei dati dei componenti non è valido",
            "部品データファイルが無効です",
            "元件数据文件无效",
            "O ficheiro de dados das peças não é válido",
        ],
    ),
    (
        NO_PRICE,
        [
            "Part has no price",
            "La pieza no tiene precio",
            "Das Bauteil hat keinen Preis",
            "La pièce n'a pas de prix",
            "Il componente non ha un prezzo",
            "部品に価格がありません",
            "元件没有价格",
            "A peça não tem preço",
        ],
    ),
    (
        NO_FOOTPRINT,
        [
            "Part has no footprint",
            "La pieza no tiene huella",
            "Das Bauteil hat keinen Footprint",
            "La pièce n'a pas d'empreinte",
            "Il componente non ha un'impronta",
            "部品にフットプリントがありません",
            "元件没有封装",
            "A peça não tem pegada",
        ],
    ),
    (
        CURRENCY_IGNORED,
        [
            "Price in another currency was ignored",
            "Se ignoró un precio en otra moneda",
            "Ein Preis in einer anderen Währung wurde ignoriert",
            "Un prix dans une autre devise a été ignoré",
            "Un prezzo in un'altra valuta è stato ignorato",
            "別の通貨の価格を無視しました",
            "已忽略其他币种的价格",
            "Um preço noutra moeda foi ignorado",
        ],
    ),
    (
        MINIMUM_RAISED,
        [
            "Quantity to buy was raised to the minimum of the supplier",
            "La cantidad a comprar se subió al mínimo del proveedor",
            "Die Kaufmenge wurde auf das Minimum des Lieferanten erhöht",
            "La quantité à acheter a été portée au minimum du fournisseur",
            "La quantità da acquistare è stata portata al minimo del fornitore",
            "購入数を仕入先の最小数量に引き上げました",
            "购买数量已提高到供应商的最低数量",
            "A quantidade a comprar foi aumentada para o mínimo do fornecedor",
        ],
    ),
    (
        NOT_FITTED,
        [
            "Part that is not fitted is left out",
            "Se omite una pieza que no se monta",
            "Ein nicht bestücktes Bauteil wird weggelassen",
            "Une pièce non montée est omise",
            "Un componente non montato viene omesso",
            "実装しない部品を除外しました",
            "已排除不贴装的元件",
            "Uma peça que não é montada foi omitida",
        ],
    ),
    (
        FOOTPRINT_MISSING,
        [
            "Component refers to a footprint that is not on the board",
            "El componente remite a una huella que no está en la placa",
            "Das Bauteil verweist auf einen Footprint, der nicht auf der Platine ist",
            "Le composant renvoie à une empreinte absente de la carte",
            "Il componente rimanda a un'impronta non presente sulla scheda",
            "部品が基板にないフットプリントを参照しています",
            "元件引用了电路板上不存在的封装",
            "O componente refere uma pegada que não está na placa",
        ],
    ),
    (
        NOT_PLACED,
        [
            "Part of the schematic is not placed on the board",
            "Una pieza del esquema no está colocada en la placa",
            "Ein Teil des Schaltplans ist nicht auf der Platine platziert",
            "Une pièce du schéma n'est pas placée sur la carte",
            "Un componente dello schema non è posizionato sulla scheda",
            "回路図の部品が基板に配置されていません",
            "原理图中的元件未放置在电路板上",
            "Uma peça do esquema não está colocada na placa",
        ],
    ),
    (
        PCB_NOT_COSTED,
        [
            "Board cannot give a PCB cost",
            "La placa no permite calcular un coste de PCB",
            "Die Platine lässt keine Leiterplattenkosten zu",
            "La carte ne permet pas de calculer un coût de PCB",
            "La scheda non permette di calcolare un costo del PCB",
            "基板からPCBコストを算出できません",
            "电路板无法给出PCB成本",
            "A placa não permite calcular um custo de PCB",
        ],
    ),
    (
        PRICE_CONFLICT,
        [
            "Two prices for one quantity of one part disagree",
            "Dos precios para una misma cantidad de una pieza no coinciden",
            "Zwei Preise für eine Menge eines Bauteils widersprechen sich",
            "Deux prix pour une même quantité d'une pièce sont en désaccord",
            "Due prezzi per la stessa quantità di un componente non coincidono",
            "同じ部品・数量に対する2つの価格が一致しません",
            "同一元件同一数量的两个价格不一致",
            "Dois preços para a mesma quantidade de uma peça não coincidem",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "A command option is not valid",
            "Una opción del comando no es válida",
            "Eine Option des Befehls ist ungültig",
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
            "El archivo de salida existe o no se puede escribir",
            "Die Ausgabedatei existiert oder kann nicht geschrieben werden",
            "Le fichier de sortie existe ou ne peut pas être écrit",
            "Il file di output esiste o non può essere scritto",
            "出力ファイルが既に存在するか書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída já existe ou não pode ser escrito",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "An input file cannot be read",
            "No se puede leer un archivo de entrada",
            "Eine Eingabedatei kann nicht gelesen werden",
            "Un fichier d'entrée est illisible",
            "Un file di input non è leggibile",
            "入力ファイルを読み込めません",
            "无法读取输入文件",
            "Não é possível ler um ficheiro de entrada",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the bill of materials was exceeded",
            "Se superó un límite de la lista de materiales",
            "Ein Limit der Stückliste wurde überschritten",
            "Une limite de la nomenclature a été dépassée",
            "Un limite della distinta base è stato superato",
            "部品表の上限を超えました",
            "超出了物料清单的限制",
            "Foi excedido um limite da lista de materiais",
        ],
    ),
    (
        PCB_ESTIMATE,
        [
            "The PCB cost is an estimate from the tariffs of the settings",
            "El coste de la PCB es una estimación con las tarifas de la configuración",
            "Die Leiterplattenkosten sind eine Schätzung mit den Tarifen der Einstellungen",
            "Le coût du PCB est une estimation avec les tarifs des réglages",
            "Il costo del PCB è una stima con le tariffe delle impostazioni",
            "PCBコストは設定の料金表による見積りです",
            "PCB成本是按设置中的费率得出的估算值",
            "O custo da PCB é uma estimativa com as tarifas das definições",
        ],
    ),
    (
        IMPORT_QUANTITY,
        [
            "A quantity of the imported bill differs from its references",
            "Una cantidad de la lista importada no coincide con sus referencias",
            "Eine Menge der importierten Stückliste weicht von ihren Referenzen ab",
            "Une quantité de la nomenclature importée diffère de ses références",
            "Una quantità della distinta importata non corrisponde ai suoi riferimenti",
            "インポートした部品表の数量が参照の数と一致しません",
            "导入的物料清单中的数量与其位号数不一致",
            "Uma quantidade da lista importada difere das suas referências",
        ],
    ),
    (
        IMPORT_DUPLICATE,
        [
            "A reference is in two lines of the imported bill",
            "Una referencia está en dos líneas de la lista importada",
            "Eine Referenz steht in zwei Zeilen der importierten Stückliste",
            "Une référence figure sur deux lignes de la nomenclature importée",
            "Un riferimento è in due righe della distinta importata",
            "同じ参照がインポートした部品表の 2 行にあります",
            "同一位号出现在导入物料清单的两行中",
            "Uma referência está em duas linhas da lista importada",
        ],
    ),
    (
        IMPORT_DISAGREES,
        [
            "The imported bill and the board disagree about a part",
            "La lista importada y la placa no coinciden en una pieza",
            "Die importierte Stückliste und die Platine widersprechen sich bei einem Bauteil",
            "La nomenclature importée et la carte divergent sur un composant",
            "La distinta importata e la scheda non concordano su un componente",
            "インポートした部品表と基板で部品が一致しません",
            "导入的物料清单与电路板在某个元件上不一致",
            "A lista importada e a placa não concordam sobre uma peça",
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

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Something the report did on its own that the user may want to know.
    Info,
    /// Something to look at.
    Warning,
    /// Something that makes the report wrong or incomplete.
    Error,
}

impl Severity {
    /// The word used in the reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// A finding of a report: a registered code, what it is about and what is specific to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The code, `PHOENIX-BOM-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// The part or the line it is about.
    pub subject: Option<String>,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl Finding {
    /// A finding about a part or a line.
    pub fn new(
        code: &'static str,
        severity: Severity,
        subject: Option<&str>,
        detail: impl Into<String>,
    ) -> Finding {
        Finding {
            code,
            severity,
            subject: subject.map(str::to_owned),
            detail: detail.into(),
        }
    }

    /// The line in a language: the severity, the code, the title, the subject and the detail.
    pub fn line(&self, lang: Lang) -> String {
        let mut out = format!(
            "{}: {}: {}",
            self.severity.as_str(),
            self.code,
            title(self.code, lang)
        );
        if let Some(subject) = &self.subject {
            out.push_str(&format!(" [{subject}]"));
        }
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

/// An error of the bill of materials: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomError {
    /// The code, `PHOENIX-BOM-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl BomError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> BomError {
        BomError {
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

impl fmt::Display for BomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for BomError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-BOM-{:03}", i + 1));
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
        let e = BomError::new(NO_PRICE, "R1");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-BOM-005: A peça não tem preço: R1"
        );
        assert_eq!(e.to_string(), "PHOENIX-BOM-005: R1");
        let bare = BomError::new(NO_PRICE, "");
        assert_eq!(bare.message(Lang::En), "PHOENIX-BOM-005: Part has no price");
    }

    #[test]
    fn a_finding_line_names_severity_code_title_subject_and_detail() {
        let f = Finding::new(NO_PRICE, Severity::Warning, Some("R1, R2"), "10k R0603");
        assert_eq!(
            f.line(Lang::En),
            "warning: PHOENIX-BOM-005: Part has no price [R1, R2]: 10k R0603"
        );
        assert!(Severity::Error > Severity::Info);
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
