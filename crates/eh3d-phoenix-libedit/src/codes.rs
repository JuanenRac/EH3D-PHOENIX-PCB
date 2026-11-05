// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the library editor, `PHOENIX-LIB-<NNN>`, with their titles in the eight
//! languages of the suite, and the error type that carries one.
//!
//! Codes 001 to 014 are the findings of the check of a library; codes 020 and above are errors of
//! reading, editing, saving and generating.

use eh3d_phoenix_rules::Lang;

/// A pin of a symbol has no name.
pub const PIN_NO_NAME: &str = "PHOENIX-LIB-001";
/// Two pins of a symbol have the same name.
pub const PIN_DUPLICATE: &str = "PHOENIX-LIB-002";
/// Two pads of a footprint have the same name.
pub const PAD_DUPLICATE: &str = "PHOENIX-LIB-003";
/// A pad has no copper layer or no size.
pub const PAD_NO_COPPER: &str = "PHOENIX-LIB-004";
/// The drill of a pad is larger than the pad.
pub const DRILL_TOO_BIG: &str = "PHOENIX-LIB-005";
/// Silkscreen is drawn over a pad.
pub const SILK_OVER_PAD: &str = "PHOENIX-LIB-006";
/// A pin of the symbol of a device has no pad.
pub const DEVICE_PIN_NO_PAD: &str = "PHOENIX-LIB-007";
/// A connection names a gate, a pin or a pad that does not exist.
pub const CONNECT_BROKEN: &str = "PHOENIX-LIB-008";
/// A device uses a footprint that the library does not have.
pub const DEVICE_NO_PACKAGE: &str = "PHOENIX-LIB-009";
/// A gate uses a symbol that the library does not have.
pub const GATE_NO_SYMBOL: &str = "PHOENIX-LIB-010";
/// A footprint has no origin inside its pads.
pub const NO_ORIGIN: &str = "PHOENIX-LIB-011";
/// A footprint has no text for the name or the value.
pub const NO_NAME_TEXT: &str = "PHOENIX-LIB-012";
/// A device set has no gates or no devices.
pub const SET_EMPTY: &str = "PHOENIX-LIB-013";
/// A device set has no reference prefix.
pub const SET_NO_PREFIX: &str = "PHOENIX-LIB-014";

/// A library file cannot be read or is not an EAGLE library.
pub const FILE_UNUSABLE: &str = "PHOENIX-LIB-020";
/// A name is empty or has characters that are not allowed.
pub const NAME_INVALID: &str = "PHOENIX-LIB-021";
/// A name is already used.
pub const NAME_USED: &str = "PHOENIX-LIB-022";
/// A symbol, footprint, device, gate, pad or item does not exist.
pub const NOT_FOUND: &str = "PHOENIX-LIB-023";
/// A value is out of its range or does not fit the item.
pub const VALUE_INVALID: &str = "PHOENIX-LIB-024";
/// A library cannot be written, or the target already exists.
pub const WRITE_FAILED: &str = "PHOENIX-LIB-025";
/// The written library does not read back the same.
pub const ROUND_TRIP: &str = "PHOENIX-LIB-026";
/// A file of operations is not valid.
pub const OPERATIONS_INVALID: &str = "PHOENIX-LIB-027";
/// A parameter of a footprint generator is not valid.
pub const PARAM_INVALID: &str = "PHOENIX-LIB-028";
/// A settings file is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-LIB-029";
/// A replacement by the master needs the confirmation of the owner.
pub const NOT_CONFIRMED: &str = "PHOENIX-LIB-030";
/// An option of the command is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-LIB-031";
/// The backup of the original cannot be made.
pub const BACKUP_FAILED: &str = "PHOENIX-LIB-032";
/// A footprint or a symbol is still used by a device.
pub const IN_USE: &str = "PHOENIX-LIB-033";

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (PIN_NO_NAME, "Pin of a symbol without a name"),
    (PIN_DUPLICATE, "Pins of a symbol with the same name"),
    (PAD_DUPLICATE, "Pads of a footprint with the same name"),
    (PAD_NO_COPPER, "Pad without a copper layer or a size"),
    (DRILL_TOO_BIG, "Drill larger than the pad"),
    (SILK_OVER_PAD, "Silkscreen drawn over a pad"),
    (DEVICE_PIN_NO_PAD, "Pin of a device without a pad"),
    (
        CONNECT_BROKEN,
        "Connection to a gate, pin or pad that does not exist",
    ),
    (
        DEVICE_NO_PACKAGE,
        "Device uses a footprint that the library does not have",
    ),
    (
        GATE_NO_SYMBOL,
        "Gate uses a symbol that the library does not have",
    ),
    (NO_ORIGIN, "Footprint without an origin inside its pads"),
    (NO_NAME_TEXT, "Footprint without a name or value text"),
    (SET_EMPTY, "Device set without gates or devices"),
    (SET_NO_PREFIX, "Device set without a reference prefix"),
    (FILE_UNUSABLE, "Library file cannot be used"),
    (NAME_INVALID, "Name is not valid"),
    (NAME_USED, "Name is already used"),
    (NOT_FOUND, "Item does not exist"),
    (VALUE_INVALID, "Value is not valid for the item"),
    (WRITE_FAILED, "Library cannot be written"),
    (ROUND_TRIP, "Written library does not read back the same"),
    (OPERATIONS_INVALID, "File of operations is not valid"),
    (PARAM_INVALID, "Footprint parameter is not valid"),
    (SETTINGS_INVALID, "Library editor settings are not valid"),
    (NOT_CONFIRMED, "Replacement by the master is not confirmed"),
    (OPTION_INVALID, "Command option is not valid"),
    (BACKUP_FAILED, "Backup of the original cannot be made"),
    (IN_USE, "Item is still used by a device"),
];

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: [(&str, [&str; 8]); 28] = [
    (
        PIN_NO_NAME,
        [
            "Pin of a symbol without a name",
            "Pin de un símbolo sin nombre",
            "Pin eines Symbols ohne Namen",
            "Broche d'un symbole sans nom",
            "Pin di un simbolo senza nome",
            "名前のないシンボルのピン",
            "没有名称的符号引脚",
            "Pino de um símbolo sem nome",
        ],
    ),
    (
        PIN_DUPLICATE,
        [
            "Pins of a symbol with the same name",
            "Pines de un símbolo con el mismo nombre",
            "Pins eines Symbols mit gleichem Namen",
            "Broches d'un symbole avec le même nom",
            "Pin di un simbolo con lo stesso nome",
            "同じ名前を持つシンボルのピン",
            "符号中名称相同的引脚",
            "Pinos de um símbolo com o mesmo nome",
        ],
    ),
    (
        PAD_DUPLICATE,
        [
            "Pads of a footprint with the same name",
            "Pads de una huella con el mismo nombre",
            "Pads eines Footprints mit gleichem Namen",
            "Pastilles d'une empreinte avec le même nom",
            "Pad di un footprint con lo stesso nome",
            "同じ名前を持つフットプリントのパッド",
            "封装中名称相同的焊盘",
            "Pads de uma pegada com o mesmo nome",
        ],
    ),
    (
        PAD_NO_COPPER,
        [
            "Pad without a copper layer or a size",
            "Pad sin capa de cobre o sin tamaño",
            "Pad ohne Kupferlage oder ohne Größe",
            "Pastille sans couche de cuivre ou sans taille",
            "Pad senza strato di rame o senza dimensione",
            "銅箔層またはサイズのないパッド",
            "没有铜层或没有尺寸的焊盘",
            "Pad sem camada de cobre ou sem tamanho",
        ],
    ),
    (
        DRILL_TOO_BIG,
        [
            "Drill larger than the pad",
            "Taladro mayor que el pad",
            "Bohrung größer als das Pad",
            "Perçage plus grand que la pastille",
            "Foro più grande del pad",
            "パッドより大きいドリル穴",
            "钻孔大于焊盘",
            "Furo maior do que o pad",
        ],
    ),
    (
        SILK_OVER_PAD,
        [
            "Silkscreen drawn over a pad",
            "Serigrafía dibujada sobre un pad",
            "Bestückungsdruck über einem Pad",
            "Sérigraphie dessinée sur une pastille",
            "Serigrafia disegnata sopra un pad",
            "パッドの上に描かれたシルク",
            "丝印画在焊盘上",
            "Serigrafia desenhada sobre um pad",
        ],
    ),
    (
        DEVICE_PIN_NO_PAD,
        [
            "Pin of a device without a pad",
            "Pin de un dispositivo sin pad",
            "Pin eines Bauteils ohne Pad",
            "Broche d'un composant sans pastille",
            "Pin di un dispositivo senza pad",
            "パッドのないデバイスのピン",
            "器件中没有焊盘的引脚",
            "Pino de um dispositivo sem pad",
        ],
    ),
    (
        CONNECT_BROKEN,
        [
            "Connection to a gate, pin or pad that does not exist",
            "Conexión a una puerta, un pin o un pad que no existe",
            "Verbindung zu einem Gate, Pin oder Pad, das nicht existiert",
            "Connexion vers une porte, une broche ou une pastille inexistante",
            "Connessione a una porta, un pin o un pad che non esiste",
            "存在しないゲート、ピン、パッドへの接続",
            "连接到不存在的门、引脚或焊盘",
            "Ligação a uma porta, um pino ou um pad que não existe",
        ],
    ),
    (
        DEVICE_NO_PACKAGE,
        [
            "Device uses a footprint that the library does not have",
            "El dispositivo usa una huella que la biblioteca no tiene",
            "Das Bauteil verwendet einen Footprint, den die Bibliothek nicht hat",
            "Le composant utilise une empreinte que la bibliothèque n'a pas",
            "Il dispositivo usa un footprint che la libreria non ha",
            "デバイスがライブラリにないフットプリントを使っています",
            "器件使用了库中没有的封装",
            "O dispositivo usa uma pegada que a biblioteca não tem",
        ],
    ),
    (
        GATE_NO_SYMBOL,
        [
            "Gate uses a symbol that the library does not have",
            "La puerta usa un símbolo que la biblioteca no tiene",
            "Das Gate verwendet ein Symbol, das die Bibliothek nicht hat",
            "La porte utilise un symbole que la bibliothèque n'a pas",
            "La porta usa un simbolo che la libreria non ha",
            "ゲートがライブラリにないシンボルを使っています",
            "门使用了库中没有的符号",
            "A porta usa um símbolo que a biblioteca não tem",
        ],
    ),
    (
        NO_ORIGIN,
        [
            "Footprint without an origin inside its pads",
            "Huella sin origen dentro de sus pads",
            "Footprint ohne Ursprung innerhalb seiner Pads",
            "Empreinte sans origine à l'intérieur de ses pastilles",
            "Footprint senza origine all'interno dei suoi pad",
            "パッドの内側に原点がないフットプリント",
            "封装的原点不在其焊盘范围内",
            "Pegada sem origem dentro dos seus pads",
        ],
    ),
    (
        NO_NAME_TEXT,
        [
            "Footprint without a name or value text",
            "Huella sin texto de nombre o de valor",
            "Footprint ohne Namens- oder Werttext",
            "Empreinte sans texte de nom ou de valeur",
            "Footprint senza testo del nome o del valore",
            "名前または値のテキストがないフットプリント",
            "封装没有名称或值的文本",
            "Pegada sem texto de nome ou de valor",
        ],
    ),
    (
        SET_EMPTY,
        [
            "Device set without gates or devices",
            "Conjunto de dispositivos sin puertas o sin dispositivos",
            "Bauteilsatz ohne Gates oder ohne Bauteile",
            "Ensemble de composants sans portes ou sans composants",
            "Insieme di dispositivi senza porte o senza dispositivi",
            "ゲートまたはデバイスのないデバイスセット",
            "没有门或没有器件的器件集",
            "Conjunto de dispositivos sem portas ou sem dispositivos",
        ],
    ),
    (
        SET_NO_PREFIX,
        [
            "Device set without a reference prefix",
            "Conjunto de dispositivos sin prefijo de referencia",
            "Bauteilsatz ohne Referenzpräfix",
            "Ensemble de composants sans préfixe de référence",
            "Insieme di dispositivi senza prefisso di riferimento",
            "参照プレフィックスのないデバイスセット",
            "没有参考前缀的器件集",
            "Conjunto de dispositivos sem prefixo de referência",
        ],
    ),
    (
        FILE_UNUSABLE,
        [
            "Library file cannot be used",
            "No se puede usar el archivo de biblioteca",
            "Die Bibliotheksdatei kann nicht verwendet werden",
            "Le fichier de bibliothèque ne peut pas être utilisé",
            "Il file della libreria non può essere usato",
            "ライブラリファイルを使用できません",
            "无法使用该库文件",
            "Não é possível usar o ficheiro da biblioteca",
        ],
    ),
    (
        NAME_INVALID,
        [
            "Name is not valid",
            "El nombre no es válido",
            "Der Name ist ungültig",
            "Le nom n'est pas valide",
            "Il nome non è valido",
            "名前が正しくありません",
            "名称无效",
            "O nome não é válido",
        ],
    ),
    (
        NAME_USED,
        [
            "Name is already used",
            "El nombre ya está en uso",
            "Der Name wird bereits verwendet",
            "Le nom est déjà utilisé",
            "Il nome è già in uso",
            "この名前はすでに使われています",
            "名称已被使用",
            "O nome já está a ser usado",
        ],
    ),
    (
        NOT_FOUND,
        [
            "Item does not exist",
            "El elemento no existe",
            "Das Element existiert nicht",
            "L'élément n'existe pas",
            "L'elemento non esiste",
            "要素が存在しません",
            "该元素不存在",
            "O elemento não existe",
        ],
    ),
    (
        VALUE_INVALID,
        [
            "Value is not valid for the item",
            "El valor no es válido para el elemento",
            "Der Wert ist für das Element ungültig",
            "La valeur n'est pas valide pour l'élément",
            "Il valore non è valido per l'elemento",
            "この要素には無効な値です",
            "该值对此元素无效",
            "O valor não é válido para o elemento",
        ],
    ),
    (
        WRITE_FAILED,
        [
            "Library cannot be written",
            "No se puede escribir la biblioteca",
            "Die Bibliothek kann nicht geschrieben werden",
            "La bibliothèque ne peut pas être écrite",
            "La libreria non può essere scritta",
            "ライブラリを書き込めません",
            "无法写入库",
            "Não é possível escrever a biblioteca",
        ],
    ),
    (
        ROUND_TRIP,
        [
            "Written library does not read back the same",
            "La biblioteca escrita no se vuelve a leer igual",
            "Die geschriebene Bibliothek wird nicht gleich zurückgelesen",
            "La bibliothèque écrite ne se relit pas à l'identique",
            "La libreria scritta non viene riletta uguale",
            "書き込んだライブラリを読み戻すと同じになりません",
            "写出的库读回后不一致",
            "A biblioteca escrita não volta a ser lida igual",
        ],
    ),
    (
        OPERATIONS_INVALID,
        [
            "File of operations is not valid",
            "El archivo de operaciones no es válido",
            "Die Datei der Operationen ist ungültig",
            "Le fichier des opérations n'est pas valide",
            "Il file delle operazioni non è valido",
            "操作ファイルが正しくありません",
            "操作文件无效",
            "O ficheiro de operações não é válido",
        ],
    ),
    (
        PARAM_INVALID,
        [
            "Footprint parameter is not valid",
            "Un parámetro de la huella no es válido",
            "Ein Parameter des Footprints ist ungültig",
            "Un paramètre de l'empreinte n'est pas valide",
            "Un parametro del footprint non è valido",
            "フットプリントのパラメータが正しくありません",
            "封装参数无效",
            "Um parâmetro da pegada não é válido",
        ],
    ),
    (
        SETTINGS_INVALID,
        [
            "Library editor settings are not valid",
            "La configuración del editor de bibliotecas no es válida",
            "Die Einstellungen des Bibliothekseditors sind ungültig",
            "Les réglages de l'éditeur de bibliothèques ne sont pas valides",
            "Le impostazioni dell'editor di librerie non sono valide",
            "ライブラリエディタの設定が正しくありません",
            "库编辑器的设置无效",
            "As definições do editor de bibliotecas não são válidas",
        ],
    ),
    (
        NOT_CONFIRMED,
        [
            "Replacement by the master is not confirmed",
            "El reemplazo por el maestro no está confirmado",
            "Der Ersatz durch den Master ist nicht bestätigt",
            "Le remplacement par le maître n'est pas confirmé",
            "La sostituzione con il master non è confermata",
            "マスターへの置き換えが確認されていません",
            "尚未确认用主库项替换",
            "A substituição pelo mestre não está confirmada",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "Command option is not valid",
            "Una opción del comando no es válida",
            "Eine Option des Befehls ist ungültig",
            "Une option de la commande n'est pas valide",
            "Un'opzione del comando non è valida",
            "コマンドのオプションが正しくありません",
            "命令选项无效",
            "Uma opção do comando não é válida",
        ],
    ),
    (
        BACKUP_FAILED,
        [
            "Backup of the original cannot be made",
            "No se puede hacer la copia de seguridad del original",
            "Die Sicherung des Originals kann nicht erstellt werden",
            "La sauvegarde de l'original ne peut pas être faite",
            "Non è possibile fare il backup dell'originale",
            "元のファイルのバックアップを作成できません",
            "无法备份原文件",
            "Não é possível fazer a cópia de segurança do original",
        ],
    ),
    (
        IN_USE,
        [
            "Item is still used by a device",
            "El elemento todavía lo usa un dispositivo",
            "Das Element wird noch von einem Bauteil verwendet",
            "L'élément est encore utilisé par un composant",
            "L'elemento è ancora usato da un dispositivo",
            "この要素はまだデバイスで使われています",
            "该元素仍被器件使用",
            "O elemento ainda é usado por um dispositivo",
        ],
    ),
];

/// The column of a language in the title table.
pub fn column(lang: Lang) -> usize {
    Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0)
}

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column(lang)])
}

/// An error of the library editor: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibError {
    /// The code, `PHOENIX-LIB-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case: a name, an index or the text of a reader.
    pub detail: String,
}

impl std::fmt::Display for LibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for LibError {}

impl LibError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> LibError {
        LibError {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the title and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let title = title(self.code, lang);
        if self.detail.is_empty() {
            format!("{}: {title}", self.code)
        } else {
            format!("{}: {title}: {}", self.code, self.detail)
        }
    }
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
            for (i, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {i}");
            }
            assert!(code.starts_with("PHOENIX-LIB-"));
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
    fn the_japanese_and_chinese_titles_are_in_their_scripts() {
        for (code, row) in TITLES.iter() {
            assert!(!row[5].is_ascii(), "{code} ja");
            assert!(!row[6].is_ascii(), "{code} zh");
        }
    }

    #[test]
    fn the_portuguese_titles_are_those_of_portugal() {
        for (code, row) in TITLES.iter() {
            let text = row[7];
            assert!(!text.contains("arquivo"), "{code}");
            assert!(!text.contains("tela"), "{code}");
            assert!(!text.contains("Salvar"), "{code}");
        }
        assert!(title(FILE_UNUSABLE, Lang::Pt).contains("ficheiro"));
    }

    #[test]
    fn codes_are_unique_and_in_order() {
        for pair in REGISTRY.windows(2) {
            assert!(pair[0].0 < pair[1].0, "{} {}", pair[0].0, pair[1].0);
        }
    }

    #[test]
    fn the_message_names_the_code_the_title_and_the_detail() {
        let e = LibError::new(NAME_USED, "R0603");
        assert_eq!(
            e.message(Lang::Pt),
            "PHOENIX-LIB-022: O nome já está a ser usado: R0603"
        );
        assert_eq!(e.to_string(), "PHOENIX-LIB-022: R0603");
        let bare = LibError::new(NOT_FOUND, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-LIB-023: Item does not exist"
        );
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
