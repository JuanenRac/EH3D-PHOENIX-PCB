// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-BZONE-<NNN>` of the zones in the board editor, with their titles in
//! the eight languages of the suite. They are the codes of the operations on zones, of the fill
//! that the editor keeps, of the copy and paste of groups and of the zone data that a target
//! format cannot hold. The errors of the editor keep their own codes (`PHOENIX-BEDIT-<NNN>`).

use eh3d_phoenix_rules::Lang;

/// A zone that the board does not have.
pub const ZONE_NOT_FOUND: &str = "PHOENIX-BZONE-001";
/// An outline that cannot be a zone.
pub const ZONE_OUTLINE: &str = "PHOENIX-BZONE-002";
/// A property of a zone that is out of its range.
pub const ZONE_PROPERTY: &str = "PHOENIX-BZONE-003";
/// A cutout that is not valid.
pub const ZONE_CUTOUT: &str = "PHOENIX-BZONE-004";
/// A fill that breaks a clearance or joins nets.
pub const FILL_BREAKS_RULES: &str = "PHOENIX-BZONE-005";
/// A part of a fill that is dropped or joined to nothing.
pub const FILL_ISLAND: &str = "PHOENIX-BZONE-006";
/// A zone with no fill.
pub const FILL_MISSING: &str = "PHOENIX-BZONE-007";
/// A group that cannot be pasted.
pub const PASTE_REFUSED: &str = "PHOENIX-BZONE-008";
/// A clipboard that cannot be used.
pub const CLIPBOARD_INVALID: &str = "PHOENIX-BZONE-009";
/// Zone data that a target format cannot hold.
pub const NOT_REPRESENTABLE: &str = "PHOENIX-BZONE-010";
/// A file of zone data that cannot be read.
pub const ZONE_DATA_INVALID: &str = "PHOENIX-BZONE-011";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        ZONE_NOT_FOUND,
        [
            "Zone is not on the board",
            "La zona no está en la placa",
            "Die Zone ist nicht auf der Platine",
            "La zone n'est pas sur la carte",
            "La zona non è sulla scheda",
            "ゾーンが基板にありません",
            "板上没有该区域",
            "A zona não está na placa",
        ],
    ),
    (
        ZONE_OUTLINE,
        [
            "Outline of the zone cannot be used",
            "El contorno de la zona no se puede usar",
            "Der Umriss der Zone ist nicht verwendbar",
            "Le contour de la zone n'est pas utilisable",
            "Il contorno della zona non è utilizzabile",
            "ゾーンの輪郭は使用できません",
            "区域轮廓无法使用",
            "O contorno da zona não pode ser usado",
        ],
    ),
    (
        ZONE_PROPERTY,
        [
            "Property of the zone is out of range",
            "Una propiedad de la zona está fuera de rango",
            "Eine Eigenschaft der Zone liegt außerhalb des Bereichs",
            "Une propriété de la zone est hors limites",
            "Una proprietà della zona è fuori intervallo",
            "ゾーンのプロパティが範囲外です",
            "区域属性超出范围",
            "Uma propriedade da zona está fora do intervalo",
        ],
    ),
    (
        ZONE_CUTOUT,
        [
            "Cutout of the zone is not valid",
            "El recorte de la zona no es válido",
            "Der Ausschnitt der Zone ist ungültig",
            "La découpe de la zone n'est pas valide",
            "Il ritaglio della zona non è valido",
            "ゾーンの切り抜きが無効です",
            "区域挖空无效",
            "O recorte da zona não é válido",
        ],
    ),
    (
        FILL_BREAKS_RULES,
        [
            "Fill breaks a clearance or joins nets",
            "El relleno rompe una separación o une redes",
            "Die Füllung verletzt einen Abstand oder verbindet Netze",
            "Le remplissage viole un dégagement ou relie des réseaux",
            "Il riempimento viola una distanza o unisce reti",
            "塗りつぶしがクリアランスを破るかネットを短絡します",
            "填充违反间距或使网络短路",
            "O preenchimento viola uma distância ou une redes",
        ],
    ),
    (
        FILL_ISLAND,
        [
            "Part of the fill is dropped or joined to nothing",
            "Una parte del relleno se descarta o no está unida a nada",
            "Ein Teil der Füllung entfällt oder hängt an nichts",
            "Une partie du remplissage est écartée ou reliée à rien",
            "Una parte del riempimento è scartata o non è collegata a nulla",
            "塗りつぶしの一部が破棄されたか、何にも接続されていません",
            "部分填充被丢弃或未连接任何对象",
            "Uma parte do preenchimento é descartada ou não está ligada a nada",
        ],
    ),
    (
        FILL_MISSING,
        [
            "Zone has no fill",
            "La zona no tiene relleno",
            "Die Zone hat keine Füllung",
            "La zone n'a pas de remplissage",
            "La zona non ha riempimento",
            "ゾーンに塗りつぶしがありません",
            "区域没有填充",
            "A zona não tem preenchimento",
        ],
    ),
    (
        PASTE_REFUSED,
        [
            "Group cannot be pasted",
            "El grupo no se puede pegar",
            "Die Gruppe kann nicht eingefügt werden",
            "Le groupe ne peut pas être collé",
            "Il gruppo non può essere incollato",
            "グループを貼り付けできません",
            "无法粘贴该组",
            "O grupo não pode ser colado",
        ],
    ),
    (
        CLIPBOARD_INVALID,
        [
            "Clipboard cannot be used",
            "El portapapeles no se puede usar",
            "Die Zwischenablage ist nicht verwendbar",
            "Le presse-papiers n'est pas utilisable",
            "Gli appunti non sono utilizzabili",
            "クリップボードを使用できません",
            "剪贴板无法使用",
            "A área de transferência não pode ser usada",
        ],
    ),
    (
        NOT_REPRESENTABLE,
        [
            "Zone data cannot be written to the format",
            "Los datos de la zona no se pueden escribir en el formato",
            "Die Zonendaten lassen sich im Format nicht ausdrücken",
            "Les données de la zone ne peuvent pas être écrites dans le format",
            "I dati della zona non si possono scrivere nel formato",
            "ゾーンのデータをこの形式で書き出せません",
            "区域数据无法写入该格式",
            "Os dados da zona não podem ser escritos no formato",
        ],
    ),
    (
        ZONE_DATA_INVALID,
        [
            "File of zone data is not valid",
            "El fichero de datos de zonas no es válido",
            "Die Zonendatendatei ist ungültig",
            "Le fichier de données de zones n'est pas valide",
            "Il file dei dati delle zone non è valido",
            "ゾーンデータのファイルが無効です",
            "区域数据文件无效",
            "O ficheiro de dados de zonas não é válido",
        ],
    ),
];

/// The codes of this module with their titles in English.
pub fn registry() -> Vec<(&'static str, &'static str)> {
    TITLES.iter().map(|(code, row)| (*code, row[0])).collect()
}

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_table_has_every_code_of_the_zones() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in registry() {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }

    #[test]
    fn every_code_has_a_title_in_every_language() {
        assert_eq!(TITLES.len(), 11);
        for (code, row) in TITLES {
            assert!(code.starts_with("PHOENIX-BZONE-"));
            assert_eq!(row.len(), 8);
            for lang in Lang::ALL {
                assert!(!title(code, lang).is_empty(), "{code}");
            }
        }
        assert_eq!(title("PHOENIX-BZONE-999", Lang::En), "");
    }

    #[test]
    fn the_portuguese_titles_are_those_of_portugal() {
        assert!(title(ZONE_NOT_FOUND, Lang::Pt).contains("placa"));
        assert!(title(CLIPBOARD_INVALID, Lang::Pt).contains("área de transferência"));
        for (code, _) in TITLES {
            assert!(!title(code, Lang::Pt).contains("arquivo"), "{code}");
        }
    }
}
