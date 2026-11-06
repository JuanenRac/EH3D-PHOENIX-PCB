//! The codes of the pick and place file, the stencil and the fabrication package:
//! `PHOENIX-PNP-<NNN>`, `PHOENIX-STENCIL-<NNN>` and `PHOENIX-FABPACK-<NNN>`, with their titles in the
//! eight languages of the suite, the finding of a run and the error that carries a code.

use std::fmt;

use crate::messages::Lang;

/// The pick and place settings are not valid.
pub const PNP_SETTINGS_INVALID: &str = "PHOENIX-PNP-001";
/// A component has no footprint on the board.
pub const PNP_NO_FOOTPRINT: &str = "PHOENIX-PNP-002";
/// A footprint has no pads, so the origin of the component is its centre.
pub const PNP_NO_PADS: &str = "PHOENIX-PNP-003";
/// A reference designator is used by two components.
pub const PNP_DUPLICATE: &str = "PHOENIX-PNP-004";
/// A component has no reference designator.
pub const PNP_NO_REFDES: &str = "PHOENIX-PNP-005";
/// A part that is not fitted was left out or marked.
pub const PNP_NOT_FITTED: &str = "PHOENIX-PNP-006";
/// The centre of a part is outside the bounding box of the board outline.
pub const PNP_OUTSIDE: &str = "PHOENIX-PNP-007";
/// A command option is not valid.
pub const PNP_OPTION_INVALID: &str = "PHOENIX-PNP-008";
/// An output file exists or cannot be written.
pub const PNP_OUTPUT_FAILED: &str = "PHOENIX-PNP-009";
/// The rotation of parts was corrected for the machine.
pub const PNP_ROTATION_CORRECTED: &str = "PHOENIX-PNP-010";
/// The board has no components.
pub const PNP_NO_COMPONENTS: &str = "PHOENIX-PNP-011";

/// The stencil settings are not valid.
pub const STENCIL_SETTINGS_INVALID: &str = "PHOENIX-STENCIL-001";
/// A paste layer does not say which side of the board it is on.
pub const STENCIL_NO_SIDE: &str = "PHOENIX-STENCIL-002";
/// An opening is below the minimum aspect ratio.
pub const STENCIL_ASPECT: &str = "PHOENIX-STENCIL-003";
/// An opening is below the minimum area ratio.
pub const STENCIL_AREA: &str = "PHOENIX-STENCIL-004";
/// A large pad was split into windows.
pub const STENCIL_WINDOWS: &str = "PHOENIX-STENCIL-005";
/// A rotated pad was written as a region.
pub const STENCIL_REGION: &str = "PHOENIX-STENCIL-006";
/// The board has no paste openings.
pub const STENCIL_NO_OPENINGS: &str = "PHOENIX-STENCIL-007";
/// A command option is not valid.
pub const STENCIL_OPTION_INVALID: &str = "PHOENIX-STENCIL-008";
/// An output file exists or cannot be written.
pub const STENCIL_OUTPUT_FAILED: &str = "PHOENIX-STENCIL-009";
/// The reduction took an opening away.
pub const STENCIL_VANISHED: &str = "PHOENIX-STENCIL-010";
/// A pad is too small to be split into windows.
pub const STENCIL_NO_SPLIT: &str = "PHOENIX-STENCIL-011";

/// The profile of the package is not valid or not found.
pub const FABPACK_PROFILE: &str = "PHOENIX-FABPACK-001";
/// The package could not be assembled.
pub const FABPACK_FAILED: &str = "PHOENIX-FABPACK-002";
/// The manifest read back does not agree with the files.
pub const FABPACK_MISMATCH: &str = "PHOENIX-FABPACK-003";
/// The output exists or cannot be written.
pub const FABPACK_OUTPUT_FAILED: &str = "PHOENIX-FABPACK-004";
/// A command option is not valid.
pub const FABPACK_OPTION_INVALID: &str = "PHOENIX-FABPACK-005";
/// A file or the package is too large for the format of the package.
pub const FABPACK_TOO_LARGE: &str = "PHOENIX-FABPACK-006";
/// What the files of the package cannot say.
pub const FABPACK_NOTES: &str = "PHOENIX-FABPACK-007";
/// An input file cannot be read.
pub const FABPACK_INPUT_UNREADABLE: &str = "PHOENIX-FABPACK-008";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        PNP_SETTINGS_INVALID,
        [
            "Pick and place settings are not valid",
            "La configuración de pick and place no es válida",
            "Die Einstellungen der Bestückungsdatei sind ungültig",
            "Les réglages du fichier de placement ne sont pas valides",
            "Le impostazioni del file di posizionamento non sono valide",
            "ピックアンドプレースの設定が無効です",
            "贴片坐标设置无效",
            "As definições do ficheiro de posições não são válidas",
        ],
    ),
    (
        PNP_NO_FOOTPRINT,
        [
            "Component has no footprint on the board",
            "El componente no tiene huella en la placa",
            "Das Bauteil hat keinen Footprint auf der Platine",
            "Le composant n'a pas d'empreinte sur la carte",
            "Il componente non ha impronta sulla scheda",
            "部品にフットプリントがありません",
            "元件在电路板上没有封装",
            "O componente não tem pegada na placa",
        ],
    ),
    (
        PNP_NO_PADS,
        [
            "Footprint has no pads, the origin is the centre",
            "La huella no tiene pads, el centro es el origen",
            "Der Footprint hat keine Pads, der Ursprung ist die Mitte",
            "L'empreinte n'a pas de pastilles, le centre est l'origine",
            "L'impronta non ha pad, il centro è l'origine",
            "フットプリントにパッドがないため原点を中心とします",
            "封装没有焊盘，以原点作为中心",
            "A pegada não tem pastilhas, o centro é a origem",
        ],
    ),
    (
        PNP_DUPLICATE,
        [
            "Reference designator is used twice",
            "El designador de referencia se usa dos veces",
            "Die Referenzbezeichnung wird zweimal verwendet",
            "La référence est utilisée deux fois",
            "Il designatore di riferimento è usato due volte",
            "参照記号が重複しています",
            "位号被使用了两次",
            "O designador de referência é usado duas vezes",
        ],
    ),
    (
        PNP_NO_REFDES,
        [
            "Component has no reference designator",
            "El componente no tiene designador de referencia",
            "Das Bauteil hat keine Referenzbezeichnung",
            "Le composant n'a pas de référence",
            "Il componente non ha designatore di riferimento",
            "部品に参照記号がありません",
            "元件没有位号",
            "O componente não tem designador de referência",
        ],
    ),
    (
        PNP_NOT_FITTED,
        [
            "Part is not fitted",
            "La pieza no se monta",
            "Das Teil wird nicht bestückt",
            "La pièce n'est pas montée",
            "Il componente non è montato",
            "部品は実装されません",
            "该元件不贴装",
            "A peça não é montada",
        ],
    ),
    (
        PNP_OUTSIDE,
        [
            "Part is outside the board outline",
            "La pieza está fuera del contorno de la placa",
            "Das Teil liegt außerhalb der Platinenkontur",
            "La pièce est hors du contour de la carte",
            "Il componente è fuori dal contorno della scheda",
            "部品が基板外形の外にあります",
            "元件位于板框之外",
            "A peça está fora do contorno da placa",
        ],
    ),
    (
        PNP_OPTION_INVALID,
        [
            "Option is not valid",
            "La opción no es válida",
            "Die Option ist ungültig",
            "L'option n'est pas valide",
            "L'opzione non è valida",
            "オプションが無効です",
            "选项无效",
            "A opção não é válida",
        ],
    ),
    (
        PNP_OUTPUT_FAILED,
        [
            "Output file exists or cannot be written",
            "El archivo de salida existe o no se puede escribir",
            "Die Ausgabedatei existiert oder kann nicht geschrieben werden",
            "Le fichier de sortie existe ou ne peut pas être écrit",
            "Il file di uscita esiste o non può essere scritto",
            "出力ファイルが存在するか、書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída existe ou não pode ser escrito",
        ],
    ),
    (
        PNP_ROTATION_CORRECTED,
        [
            "Rotation corrected for the machine",
            "Giro corregido para la máquina",
            "Drehung für die Maschine korrigiert",
            "Rotation corrigée pour la machine",
            "Rotazione corretta per la macchina",
            "機械用に回転を補正しました",
            "已按机器修正旋转角度",
            "Rotação corrigida para a máquina",
        ],
    ),
    (
        PNP_NO_COMPONENTS,
        [
            "Board has no components",
            "La placa no tiene componentes",
            "Die Platine hat keine Bauteile",
            "La carte n'a pas de composants",
            "La scheda non ha componenti",
            "基板に部品がありません",
            "电路板上没有元件",
            "A placa não tem componentes",
        ],
    ),
    (
        STENCIL_SETTINGS_INVALID,
        [
            "Stencil settings are not valid",
            "La configuración de la plantilla no es válida",
            "Die Schablonen-Einstellungen sind ungültig",
            "Les réglages du pochoir ne sont pas valides",
            "Le impostazioni dello stencil non sono valide",
            "ステンシルの設定が無効です",
            "钢网设置无效",
            "As definições da stencil não são válidas",
        ],
    ),
    (
        STENCIL_NO_SIDE,
        [
            "Paste layer has no side",
            "La capa de pasta no indica su cara",
            "Die Pastenlage hat keine Seite",
            "La couche de pâte n'a pas de face",
            "Lo strato della pasta non ha il lato",
            "ペースト層に面の指定がありません",
            "锡膏层没有指明面",
            "A camada de pasta não indica a face",
        ],
    ),
    (
        STENCIL_ASPECT,
        [
            "Opening is below the minimum aspect ratio",
            "La apertura está por debajo de la relación de aspecto mínima",
            "Die Öffnung unterschreitet das Mindest-Seitenverhältnis",
            "L'ouverture est sous le rapport d'aspect minimal",
            "L'apertura è sotto il rapporto di aspetto minimo",
            "開口がアスペクト比の下限を下回っています",
            "开孔低于最小宽厚比",
            "A abertura está abaixo da relação de aspeto mínima",
        ],
    ),
    (
        STENCIL_AREA,
        [
            "Opening is below the minimum area ratio",
            "La apertura está por debajo de la relación de área mínima",
            "Die Öffnung unterschreitet das Mindest-Flächenverhältnis",
            "L'ouverture est sous le rapport de surface minimal",
            "L'apertura è sotto il rapporto di area minimo",
            "開口が面積比の下限を下回っています",
            "开孔低于最小面积比",
            "A abertura está abaixo da relação de área mínima",
        ],
    ),
    (
        STENCIL_WINDOWS,
        [
            "Large pad split into windows",
            "Pad grande dividido en ventanas",
            "Großes Pad in Fenster unterteilt",
            "Grande pastille divisée en fenêtres",
            "Pad grande diviso in finestre",
            "大きなパッドを窓に分割しました",
            "大焊盘已分为多个小窗",
            "Pastilha grande dividida em janelas",
        ],
    ),
    (
        STENCIL_REGION,
        [
            "Rotated pad written as a region",
            "Pad girado escrito como región",
            "Gedrehtes Pad als Region geschrieben",
            "Pastille tournée écrite comme région",
            "Pad ruotato scritto come regione",
            "回転したパッドをリージョンとして出力しました",
            "旋转的焊盘以区域输出",
            "Pastilha rodada escrita como região",
        ],
    ),
    (
        STENCIL_NO_OPENINGS,
        [
            "Board has no paste openings",
            "La placa no tiene aperturas de pasta",
            "Die Platine hat keine Pastenöffnungen",
            "La carte n'a pas d'ouvertures de pâte",
            "La scheda non ha aperture di pasta",
            "基板にペースト開口がありません",
            "电路板没有锡膏开孔",
            "A placa não tem aberturas de pasta",
        ],
    ),
    (
        STENCIL_OPTION_INVALID,
        [
            "Option is not valid",
            "La opción no es válida",
            "Die Option ist ungültig",
            "L'option n'est pas valide",
            "L'opzione non è valida",
            "オプションが無効です",
            "选项无效",
            "A opção não é válida",
        ],
    ),
    (
        STENCIL_OUTPUT_FAILED,
        [
            "Output file exists or cannot be written",
            "El archivo de salida existe o no se puede escribir",
            "Die Ausgabedatei existiert oder kann nicht geschrieben werden",
            "Le fichier de sortie existe ou ne peut pas être écrit",
            "Il file di uscita esiste o non può essere scritto",
            "出力ファイルが存在するか、書き込めません",
            "输出文件已存在或无法写入",
            "O ficheiro de saída existe ou não pode ser escrito",
        ],
    ),
    (
        STENCIL_VANISHED,
        [
            "Reduction took an opening away",
            "La reducción eliminó una apertura",
            "Die Verkleinerung hat eine Öffnung entfernt",
            "La réduction a supprimé une ouverture",
            "La riduzione ha eliminato un'apertura",
            "縮小により開口がなくなりました",
            "缩减使一个开孔消失",
            "A redução eliminou uma abertura",
        ],
    ),
    (
        STENCIL_NO_SPLIT,
        [
            "Pad is too small to be split into windows",
            "El pad es demasiado pequeño para dividirlo en ventanas",
            "Das Pad ist zu klein für eine Unterteilung in Fenster",
            "La pastille est trop petite pour être divisée en fenêtres",
            "Il pad è troppo piccolo per essere diviso in finestre",
            "パッドが小さすぎて窓に分割できません",
            "焊盘太小，无法分为小窗",
            "A pastilha é pequena demais para ser dividida em janelas",
        ],
    ),
    (
        FABPACK_PROFILE,
        [
            "Export profile is not valid or not found",
            "El perfil de exportación no es válido o no existe",
            "Das Exportprofil ist ungültig oder fehlt",
            "Le profil d'export n'est pas valide ou introuvable",
            "Il profilo di esportazione non è valido o manca",
            "エクスポートプロファイルが無効か見つかりません",
            "导出配置无效或未找到",
            "O perfil de exportação não é válido ou não existe",
        ],
    ),
    (
        FABPACK_FAILED,
        [
            "Package could not be assembled",
            "No se pudo montar el paquete",
            "Das Paket konnte nicht zusammengestellt werden",
            "Le paquet n'a pas pu être assemblé",
            "Non è stato possibile assemblare il pacchetto",
            "パッケージを作成できませんでした",
            "无法生成制造包",
            "Não foi possível montar o pacote",
        ],
    ),
    (
        FABPACK_MISMATCH,
        [
            "Manifest read back differs from the files",
            "El manifiesto leído no coincide con los archivos",
            "Das gelesene Manifest weicht von den Dateien ab",
            "Le manifeste relu diffère des fichiers",
            "Il manifesto riletto differisce dai file",
            "読み戻したマニフェストがファイルと一致しません",
            "回读的清单与文件不一致",
            "O manifesto lido difere dos ficheiros",
        ],
    ),
    (
        FABPACK_OUTPUT_FAILED,
        [
            "Output exists or cannot be written",
            "La salida existe o no se puede escribir",
            "Die Ausgabe existiert oder kann nicht geschrieben werden",
            "La sortie existe ou ne peut pas être écrite",
            "L'uscita esiste o non può essere scritta",
            "出力先が存在するか、書き込めません",
            "输出已存在或无法写入",
            "A saída existe ou não pode ser escrita",
        ],
    ),
    (
        FABPACK_OPTION_INVALID,
        [
            "Option is not valid",
            "La opción no es válida",
            "Die Option ist ungültig",
            "L'option n'est pas valide",
            "L'opzione non è valida",
            "オプションが無効です",
            "选项无效",
            "A opção não é válida",
        ],
    ),
    (
        FABPACK_TOO_LARGE,
        [
            "File or package is too large for the package format",
            "El archivo o el paquete es demasiado grande para el formato",
            "Datei oder Paket ist zu groß für das Paketformat",
            "Le fichier ou le paquet est trop grand pour le format",
            "Il file o il pacchetto è troppo grande per il formato",
            "ファイルまたはパッケージが形式の上限を超えています",
            "文件或包超出格式限制",
            "O ficheiro ou o pacote é demasiado grande para o formato",
        ],
    ),
    (
        FABPACK_NOTES,
        [
            "What the files of the package cannot say",
            "Lo que los archivos del paquete no pueden expresar",
            "Was die Dateien des Pakets nicht ausdrücken können",
            "Ce que les fichiers du paquet ne peuvent pas exprimer",
            "Ciò che i file del pacchetto non possono esprimere",
            "パッケージのファイルでは表現できないこと",
            "包内文件无法表达的内容",
            "O que os ficheiros do pacote não conseguem exprimir",
        ],
    ),
    (
        FABPACK_INPUT_UNREADABLE,
        [
            "Input file cannot be read",
            "No se puede leer el archivo de entrada",
            "Die Eingabedatei kann nicht gelesen werden",
            "Le fichier d'entrée ne peut pas être lu",
            "Il file di input non può essere letto",
            "入力ファイルを読み込めません",
            "无法读取输入文件",
            "Não é possível ler o ficheiro de entrada",
        ],
    ),
];

fn column(lang: Lang) -> usize {
    Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0)
}

/// The title of a code in a language; the code itself when it is unknown.
pub fn title(code: &str, lang: Lang) -> String {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or_else(|| code.to_owned(), |(_, row)| row[column(lang)].to_owned())
}

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Information.
    Info,
    /// Something the user should look at.
    Warning,
    /// The result cannot be trusted.
    Error,
}

impl Severity {
    /// The word written before a finding.
    pub fn word(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// One thing found while making a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The code.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// What it is about (a reference designator, a face), empty when it is about the whole run.
    pub subject: String,
    /// The detail.
    pub detail: String,
}

impl Finding {
    /// A finding.
    pub fn new(
        code: &'static str,
        severity: Severity,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Finding {
        Finding {
            code,
            severity,
            subject: subject.into(),
            detail: detail.into(),
        }
    }

    /// The line shown to the user: severity, code, title, subject and detail.
    pub fn line(&self, lang: Lang) -> String {
        let mut line = format!(
            "{}: {}: {}",
            self.severity.word(),
            self.code,
            title(self.code, lang)
        );
        if !self.subject.is_empty() {
            line.push_str(&format!(" [{}]", self.subject));
        }
        if !self.detail.is_empty() {
            line.push_str(&format!(": {}", self.detail));
        }
        line
    }
}

/// An error that carries a code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabError {
    /// The code.
    pub code: &'static str,
    /// The detail.
    pub detail: String,
}

impl FabError {
    /// An error.
    pub fn new(code: &'static str, detail: impl Into<String>) -> FabError {
        FabError {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the title and the detail.
    pub fn message(&self, lang: Lang) -> String {
        if self.detail.is_empty() {
            format!("{}: {}", self.code, title(self.code, lang))
        } else {
            format!("{}: {}: {}", self.code, title(self.code, lang), self.detail)
        }
    }
}

impl fmt::Display for FabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.detail.is_empty() {
            f.write_str(self.code)
        } else {
            write!(f, "{}: {}", self.code, self.detail)
        }
    }
}

impl std::error::Error for FabError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_inside_its_family_and_has_a_title_in_each_language() {
        let mut last: Option<(String, u32)> = None;
        for (code, row) in TITLES {
            let (family, number) = code.rsplit_once('-').unwrap();
            let number: u32 = number.parse().unwrap();
            match &last {
                Some((f, n)) if f == family => assert_eq!(number, n + 1, "{code}"),
                _ => assert_eq!(number, 1, "{code}"),
            }
            last = Some((family.to_owned(), number));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
        }
    }

    #[test]
    fn no_title_of_another_language_is_empty_or_a_copy_of_a_code() {
        for (code, row) in TITLES {
            for text in &row[1..] {
                assert!(!text.starts_with("PHOENIX-"), "{code}");
            }
        }
    }

    #[test]
    fn the_message_names_the_code_the_title_and_the_detail() {
        let e = FabError::new(PNP_NO_FOOTPRINT, "R1");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-PNP-002: O componente não tem pegada na placa: R1"
        );
        assert_eq!(e.to_string(), "PHOENIX-PNP-002: R1");
        assert_eq!(
            FabError::new(PNP_NO_FOOTPRINT, "").message(Lang::En),
            "PHOENIX-PNP-002: Component has no footprint on the board"
        );
        assert_eq!(title("PHOENIX-X-999", Lang::En), "PHOENIX-X-999");
    }

    #[test]
    fn a_finding_line_names_severity_code_title_subject_and_detail() {
        let f = Finding::new(PNP_NOT_FITTED, Severity::Info, "C9", "left out");
        assert_eq!(
            f.line(Lang::En),
            "info: PHOENIX-PNP-006: Part is not fitted [C9]: left out"
        );
        assert!(Severity::Error > Severity::Warning);
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
