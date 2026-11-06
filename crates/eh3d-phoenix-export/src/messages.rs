//! The titles of the codes of this crate in the eight languages of the suite: English, Spanish,
//! German, French, Italian, Japanese, Chinese and European Portuguese.
//!
//! The English column is the one of [`crate::codes::REGISTRY`]; a test keeps them equal.

/// A language of the suite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    /// English.
    En,
    /// Spanish.
    Es,
    /// German.
    De,
    /// French.
    Fr,
    /// Italian.
    It,
    /// Japanese.
    Ja,
    /// Chinese.
    Zh,
    /// European Portuguese.
    Pt,
}

impl Lang {
    /// Every language, in the order of the columns of the title table.
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Zh,
        Lang::Pt,
    ];

    /// The language of a two-letter tag (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`).
    pub fn from_tag(tag: &str) -> Option<Lang> {
        match tag {
            "en" => Some(Lang::En),
            "es" => Some(Lang::Es),
            "de" => Some(Lang::De),
            "fr" => Some(Lang::Fr),
            "it" => Some(Lang::It),
            "ja" => Some(Lang::Ja),
            "zh" => Some(Lang::Zh),
            "pt" => Some(Lang::Pt),
            _ => None,
        }
    }

    fn column(self) -> usize {
        match self {
            Lang::En => 0,
            Lang::Es => 1,
            Lang::De => 2,
            Lang::Fr => 3,
            Lang::It => 4,
            Lang::Ja => 5,
            Lang::Zh => 6,
            Lang::Pt => 7,
        }
    }
}

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: [(&str, [&str; 8]); 20] = [
    (
        "PHOENIX-EXPORT-001",
        [
            "Pad rotation is not a right angle",
            "El giro del pad no es un ángulo recto",
            "Die Drehung des Pads ist kein rechter Winkel",
            "La rotation du pad n'est pas un angle droit",
            "La rotazione del pad non è un angolo retto",
            "パッドの回転が直角ではありません",
            "焊盘旋转角度不是直角",
            "A rotação da pastilha não é um ângulo reto",
        ],
    ),
    (
        "PHOENIX-EXPORT-002",
        [
            "Zone written as its outline",
            "Zona escrita como su contorno",
            "Zone als Umriss geschrieben",
            "Zone écrite comme son contour",
            "Zona scritta come il suo contorno",
            "ゾーンは輪郭として出力されました",
            "铺铜区域以轮廓形式输出",
            "Zona escrita como o seu contorno",
        ],
    ),
    (
        "PHOENIX-EXPORT-003",
        [
            "Copper stack order not given",
            "No se indica el orden de las capas de cobre",
            "Reihenfolge der Kupferlagen nicht angegeben",
            "Ordre des couches de cuivre non indiqué",
            "Ordine degli strati di rame non indicato",
            "銅層の順序が指定されていません",
            "未给出铜层顺序",
            "A ordem das camadas de cobre não foi indicada",
        ],
    ),
    (
        "PHOENIX-EXPORT-004",
        [
            "Arc left out",
            "Arco omitido",
            "Bogen ausgelassen",
            "Arc omis",
            "Arco omesso",
            "円弧は省略されました",
            "圆弧已省略",
            "Arco omitido",
        ],
    ),
    (
        "PHOENIX-EXPORT-005",
        [
            "File could not be written",
            "No se pudo escribir el archivo",
            "Datei konnte nicht geschrieben werden",
            "Le fichier n'a pas pu être écrit",
            "Impossibile scrivere il file",
            "ファイルを書き込めませんでした",
            "无法写入文件",
            "Não foi possível escrever o ficheiro",
        ],
    ),
    (
        "PHOENIX-EXPORT-006",
        [
            "Non-round pad drilled as a round hole",
            "Pad no redondo taladrado como agujero redondo",
            "Nicht rundes Pad als rundes Loch gebohrt",
            "Pad non rond percé comme un trou rond",
            "Pad non rotondo forato come foro rotondo",
            "丸くないパッドが丸穴として穿孔されました",
            "非圆形焊盘按圆孔钻孔",
            "Pastilha não redonda furada como furo redondo",
        ],
    ),
    (
        "PHOENIX-EXPORT-007",
        [
            "Package could not be assembled",
            "No se pudo montar el paquete",
            "Paket konnte nicht zusammengestellt werden",
            "Le paquet n'a pas pu être assemblé",
            "Impossibile assemblare il pacchetto",
            "パッケージを組み立てられませんでした",
            "无法组装软件包",
            "Não foi possível montar o pacote",
        ],
    ),
    (
        "PHOENIX-EXPORT-008",
        [
            "Silkscreen side not given",
            "No se indica la cara de la serigrafía",
            "Seite des Bestückungsdrucks nicht angegeben",
            "Face de la sérigraphie non indiquée",
            "Lato della serigrafia non indicato",
            "シルクの面が指定されていません",
            "未给出丝印所在面",
            "A face da serigrafia não foi indicada",
        ],
    ),
    (
        "PHOENIX-EXPORT-009",
        [
            "Silkscreen characters without a glyph",
            "Caracteres de serigrafía sin glifo",
            "Zeichen des Bestückungsdrucks ohne Glyphe",
            "Caractères de sérigraphie sans glyphe",
            "Caratteri della serigrafia senza glifo",
            "グリフのないシルク文字",
            "丝印字符缺少字形",
            "Carateres da serigrafia sem glifo",
        ],
    ),
    (
        "PHOENIX-EXPORT-010",
        [
            "Silkscreen item left out",
            "Elemento de serigrafía omitido",
            "Element des Bestückungsdrucks ausgelassen",
            "Élément de sérigraphie omis",
            "Elemento della serigrafia omesso",
            "シルク要素は省略されました",
            "丝印元素已省略",
            "Elemento da serigrafia omitido",
        ],
    ),
    (
        "PHOENIX-EXPORT-011",
        [
            "Export profile is not valid",
            "El perfil de exportación no es válido",
            "Das Exportprofil ist ungültig",
            "Le profil d'exportation n'est pas valide",
            "Il profilo di esportazione non è valido",
            "エクスポートプロファイルが無効です",
            "导出配置无效",
            "O perfil de exportação não é válido",
        ],
    ),
    (
        "PHOENIX-EXPORT-012",
        [
            "Export profile not found",
            "No se encuentra el perfil de exportación",
            "Exportprofil nicht gefunden",
            "Profil d'exportation introuvable",
            "Profilo di esportazione non trovato",
            "エクスポートプロファイルが見つかりません",
            "未找到导出配置",
            "O perfil de exportação não foi encontrado",
        ],
    ),
    (
        "PHOENIX-EXPORT-013",
        [
            "Output could not be written",
            "No se pudo escribir la salida",
            "Ausgabe konnte nicht geschrieben werden",
            "La sortie n'a pas pu être écrite",
            "Impossibile scrivere l'output",
            "出力を書き込めませんでした",
            "无法写入输出",
            "Não foi possível escrever a saída",
        ],
    ),
    (
        "PHOENIX-EXPORT-014",
        [
            "Written file differs from the model",
            "El archivo escrito difiere del modelo",
            "Die geschriebene Datei weicht vom Modell ab",
            "Le fichier écrit diffère du modèle",
            "Il file scritto differisce dal modello",
            "書き出したファイルがモデルと異なります",
            "输出文件与模型不一致",
            "O ficheiro escrito difere do modelo",
        ],
    ),
    (
        "PHOENIX-EXPORT-015",
        [
            "Board has no components",
            "La placa no tiene componentes",
            "Die Platine hat keine Bauteile",
            "La carte n'a aucun composant",
            "La scheda non ha componenti",
            "基板に部品がありません",
            "电路板没有元件",
            "A placa não tem componentes",
        ],
    ),
    (
        "PHOENIX-EXPORT-016",
        [
            "Slot pad not on a right angle",
            "El pad de la ranura no está en ángulo recto",
            "Langloch-Pad nicht im rechten Winkel",
            "Pad de fente pas à angle droit",
            "Pad di asola non ad angolo retto",
            "スロットパッドが直角ではありません",
            "槽孔焊盘不在直角方向",
            "A pastilha da ranhura não está em ângulo reto",
        ],
    ),
    (
        "PHOENIX-EXPORT-017",
        [
            "Layer side not given",
            "No se indica la cara de la capa",
            "Seite der Lage nicht angegeben",
            "Face de la couche non indiquée",
            "Lato dello strato non indicato",
            "層の面が指定されていません",
            "未给出层所在面",
            "A face da camada não foi indicada",
        ],
    ),
    (
        "PHOENIX-EXPORT-018",
        [
            "Written file could not be read back",
            "No se pudo releer el archivo escrito",
            "Die geschriebene Datei konnte nicht zurückgelesen werden",
            "Le fichier écrit n'a pas pu être relu",
            "Impossibile rileggere il file scritto",
            "書き出したファイルを読み戻せませんでした",
            "无法回读已写入的文件",
            "Não foi possível reler o ficheiro escrito",
        ],
    ),
    (
        "PHOENIX-EXPORT-019",
        [
            "Attribute text written with escapes",
            "Texto de atributo escrito con escapes",
            "Attributtext mit Escape-Folgen geschrieben",
            "Texte d'attribut écrit avec des échappements",
            "Testo dell'attributo scritto con sequenze di escape",
            "属性テキストをエスケープして出力しました",
            "属性文本已使用转义序列输出",
            "Texto de atributo escrito com sequências de escape",
        ],
    ),
    (
        "PHOENIX-EXPORT-020",
        [
            "Attribute text not allowed",
            "Texto de atributo no permitido",
            "Attributtext nicht zulässig",
            "Texte d'attribut non autorisé",
            "Testo dell'attributo non consentito",
            "属性テキストは使用できません",
            "属性文本不被允许",
            "Texto de atributo não permitido",
        ],
    ),
];

/// The title of `code` in `lang`, or `"?"` for a code this crate does not have.
pub fn title(code: &str, lang: Lang) -> &'static str {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("?", |(_, titles)| titles[lang.column()])
}

/// A message made by [`crate::codes::diagnostic`] (the English title, then `: ` and the detail) with
/// its title in `lang`. The detail stays as it is. A message that does not start with the title of
/// `code` is returned unchanged.
pub fn localize(code: &str, message: &str, lang: Lang) -> String {
    let english = title(code, Lang::En);
    match message.strip_prefix(english) {
        Some(rest) if english != "?" => format!("{}{rest}", title(code, lang)),
        _ => message.to_owned(),
    }
}

/// The line that says how many files were written.
pub fn written(count: usize, lang: Lang) -> String {
    match lang {
        Lang::En => format!("{count} files written"),
        Lang::Es => format!("{count} archivos escritos"),
        Lang::De => format!("{count} Dateien geschrieben"),
        Lang::Fr => format!("{count} fichiers écrits"),
        Lang::It => format!("{count} file scritti"),
        Lang::Ja => format!("{count} 個のファイルを書き出しました"),
        Lang::Zh => format!("已写入 {count} 个文件"),
        Lang::Pt => format!("{count} ficheiros escritos"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::REGISTRY;

    #[test]
    fn every_code_has_its_title_in_the_eight_languages() {
        assert_eq!(TITLES.len(), REGISTRY.len());
        for (code, english) in REGISTRY {
            assert_eq!(title(code, Lang::En), *english, "{code}");
            for lang in Lang::ALL {
                assert!(!title(code, lang).is_empty(), "{code}");
            }
        }
    }

    #[test]
    fn the_tags_name_the_eight_languages() {
        for (tag, lang) in [
            ("en", Lang::En),
            ("es", Lang::Es),
            ("de", Lang::De),
            ("fr", Lang::Fr),
            ("it", Lang::It),
            ("ja", Lang::Ja),
            ("zh", Lang::Zh),
            ("pt", Lang::Pt),
        ] {
            assert_eq!(Lang::from_tag(tag), Some(lang));
        }
        assert_eq!(Lang::from_tag("xx"), None);
        let message = "Arc left out: an arc of a track";
        assert_eq!(
            localize("PHOENIX-EXPORT-004", message, Lang::De),
            "Bogen ausgelassen: an arc of a track"
        );
        assert_eq!(localize("PHOENIX-EXPORT-004", "other", Lang::De), "other");
        assert_eq!(localize("nothing", "?", Lang::De), "?");
        assert!(written(3, Lang::Pt).contains("ficheiros"));
        assert_eq!(title("PHOENIX-EXPORT-999", Lang::En), "?");
    }
}
