// SPDX-License-Identifier: GPL-3.0-or-later
//! The codes of the geometric checks of Gerber files (`PHOENIX-GDF-<NNN>`), their titles in the
//! eight languages, and the report as text, JSON and CSV.

use eh3d_phoenix_rules::Lang;

use crate::geodfm::{GeoReport, GeoSeverity};

/// A conductor or a gap between conductors narrower than the profile allows: the spacing.
pub const SPACING: &str = "PHOENIX-GDF-001";
/// A conductor narrower than the profile allows.
pub const WIDTH: &str = "PHOENIX-GDF-002";
/// The ring of copper round a plated hole, in the worst case, is thinner than the profile allows.
pub const RING: &str = "PHOENIX-GDF-003";
/// Copper closer to the edge of the board than the profile allows.
pub const EDGE: &str = "PHOENIX-GDF-004";
/// A bridge of solder mask between two openings narrower than the profile allows.
pub const MASK_BRIDGE: &str = "PHOENIX-GDF-005";
/// Legend printed over a pad that is not covered by the mask.
pub const SILK_ON_PAD: &str = "PHOENIX-GDF-006";
/// A via inside the opening of a pad.
pub const VIA_IN_PAD: &str = "PHOENIX-GDF-007";
/// Copper with nothing that connects it to anything.
pub const FLOATING: &str = "PHOENIX-GDF-008";
/// A check could not be done.
pub const SKIPPED: &str = "PHOENIX-GDF-009";
/// A layer that a check needs is not among the files.
pub const LAYER_MISSING: &str = "PHOENIX-GDF-010";
/// The figures of the profile were not checked against the maker.
pub const UNVERIFIED: &str = "PHOENIX-GDF-011";

/// The codes with their titles in the eight languages (en, es, de, fr, it, ja, zh, pt).
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SPACING,
        [
            "Copper closer than the profile allows",
            "Cobre más cerca de lo que permite el perfil",
            "Kupfer näher beieinander, als das Profil zulässt",
            "Cuivre plus proche que le profil ne le permet",
            "Rame più vicino di quanto il profilo permetta",
            "銅箔の間隔がプロファイルの許容より狭い",
            "铜箔间距小于配置文件允许的值",
            "Cobre mais próximo do que o perfil permite",
        ],
    ),
    (
        WIDTH,
        [
            "Conductor narrower than the profile allows",
            "Conductor más estrecho de lo que permite el perfil",
            "Leiter schmaler, als das Profil zulässt",
            "Conducteur plus étroit que le profil ne le permet",
            "Conduttore più stretto di quanto il profilo permetta",
            "導体の幅がプロファイルの許容より狭い",
            "导线宽度小于配置文件允许的值",
            "Condutor mais estreito do que o perfil permite",
        ],
    ),
    (
        RING,
        [
            "Annular ring too thin in the worst case",
            "Anillo de cobre demasiado fino en el peor caso",
            "Restring im ungünstigsten Fall zu dünn",
            "Anneau trop mince dans le pire des cas",
            "Anello troppo sottile nel caso peggiore",
            "最悪の場合にアニュラリングが薄すぎる",
            "最坏情况下焊环过薄",
            "Anel de cobre demasiado fino no pior caso",
        ],
    ),
    (
        EDGE,
        [
            "Copper too close to the board edge",
            "Cobre demasiado cerca del borde de la placa",
            "Kupfer zu nah am Platinenrand",
            "Cuivre trop près du bord de la carte",
            "Rame troppo vicino al bordo della scheda",
            "銅箔が基板の端に近すぎる",
            "铜箔距板边过近",
            "Cobre demasiado perto do bordo da placa",
        ],
    ),
    (
        MASK_BRIDGE,
        [
            "Solder mask bridge narrower than the profile allows",
            "Puente de máscara más estrecho de lo que permite el perfil",
            "Lötstoppsteg schmaler, als das Profil zulässt",
            "Pont de vernis plus étroit que le profil ne le permet",
            "Ponte di maschera più stretto di quanto il profilo permetta",
            "ソルダーマスクのブリッジがプロファイルの許容より狭い",
            "阻焊桥宽度小于配置文件允许的值",
            "Ponte de máscara mais estreita do que o perfil permite",
        ],
    ),
    (
        SILK_ON_PAD,
        [
            "Legend printed over an exposed pad",
            "Serigrafía impresa sobre un pad descubierto",
            "Bestückungsdruck über einem freiliegenden Pad",
            "Sérigraphie imprimée sur un pad découvert",
            "Serigrafia stampata su un pad scoperto",
            "露出したパッドの上にシルクが印刷されている",
            "丝印印在裸露的焊盘上",
            "Serigrafia impressa sobre uma pastilha a descoberto",
        ],
    ),
    (
        VIA_IN_PAD,
        [
            "Via inside the opening of a pad",
            "Vía dentro de la abertura de un pad",
            "Via innerhalb der Öffnung eines Pads",
            "Via dans l'ouverture d'un pad",
            "Via dentro l'apertura di un pad",
            "パッドの開口部の中にビアがある",
            "过孔位于焊盘开窗内",
            "Via dentro da abertura de uma pastilha",
        ],
    ),
    (
        FLOATING,
        [
            "Floating copper: nothing connects it",
            "Cobre flotante: nada lo conecta",
            "Schwimmendes Kupfer: nichts verbindet es",
            "Cuivre flottant : rien ne le relie",
            "Rame flottante: nulla la collega",
            "浮いた銅箔：何にも接続されていない",
            "悬空铜箔：没有任何连接",
            "Cobre flutuante: nada o liga",
        ],
    ),
    (
        SKIPPED,
        [
            "A check could not be done",
            "No se pudo hacer una comprobación",
            "Eine Prüfung konnte nicht durchgeführt werden",
            "Une vérification n'a pas pu être faite",
            "Non è stato possibile eseguire un controllo",
            "実行できなかったチェックがある",
            "有一项检查无法完成",
            "Não foi possível fazer uma verificação",
        ],
    ),
    (
        LAYER_MISSING,
        [
            "A layer that a check needs is not among the files",
            "Falta una capa que necesita una comprobación",
            "Eine für eine Prüfung nötige Lage fehlt in den Dateien",
            "Une couche nécessaire à une vérification manque dans les fichiers",
            "Manca un livello necessario a un controllo",
            "チェックに必要なレイヤーがファイルにない",
            "文件中缺少某项检查所需的层",
            "Falta uma camada de que uma verificação precisa",
        ],
    ),
    (
        UNVERIFIED,
        [
            "The figures of the profile are not verified against the maker",
            "Las cifras del perfil no están verificadas con el fabricante",
            "Die Werte des Profils sind nicht beim Hersteller geprüft",
            "Les valeurs du profil ne sont pas vérifiées auprès du fabricant",
            "I valori del profilo non sono verificati con il produttore",
            "プロファイルの数値はメーカーに確認されていません",
            "配置文件的数值尚未与制造商核实",
            "Os valores do perfil não foram verificados junto do fabricante",
        ],
    ),
];

/// The words of the report, in the eight languages.
const WORDS: &[(&str, [&str; 8])] = &[
    (
        "pass",
        [
            "PASS",
            "SUPERADO",
            "BESTANDEN",
            "RÉUSSI",
            "SUPERATO",
            "合格",
            "通过",
            "APROVADO",
        ],
    ),
    (
        "fail",
        [
            "FAIL",
            "NO SUPERADO",
            "NICHT BESTANDEN",
            "ÉCHEC",
            "NON SUPERATO",
            "不合格",
            "未通过",
            "REPROVADO",
        ],
    ),
    (
        "profile",
        [
            "profile",
            "perfil",
            "Profil",
            "profil",
            "profilo",
            "プロファイル",
            "配置文件",
            "perfil",
        ],
    ),
    (
        "measured",
        [
            "measured", "medido", "gemessen", "mesuré", "misurato", "実測", "测得", "medido",
        ],
    ),
    (
        "limit",
        [
            "limit", "límite", "Grenze", "limite", "limite", "制限", "限值", "limite",
        ],
    ),
    (
        "layer",
        [
            "layer",
            "capa",
            "Lage",
            "couche",
            "livello",
            "レイヤー",
            "层",
            "camada",
        ],
    ),
    (
        "more",
        [
            "more not listed",
            "más sin listar",
            "weitere nicht aufgeführt",
            "autres non listés",
            "altri non elencati",
            "他に未掲載",
            "另有未列出",
            "mais não listados",
        ],
    ),
    (
        "files",
        [
            "files read",
            "ficheros leídos",
            "Dateien gelesen",
            "fichiers lus",
            "file letti",
            "読み込んだファイル",
            "已读取的文件",
            "ficheiros lidos",
        ],
    ),
];

fn column(lang: Lang) -> usize {
    Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0)
}

/// The title of a code in a language; `None` for a code that is not one of these.
pub fn title(code: &str, lang: Lang) -> Option<&'static str> {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, texts)| texts[column(lang)])
}

fn word(key: &str, lang: Lang) -> &'static str {
    WORDS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("", |(_, texts)| texts[column(lang)])
}

fn severity_name(s: GeoSeverity) -> &'static str {
    match s {
        GeoSeverity::Error => "error",
        GeoSeverity::Warning => "warning",
        GeoSeverity::Info => "info",
    }
}

fn mm(nm: i64) -> String {
    format!("{:.3}", nm as f64 / 1.0e6)
}

/// The report as text in a language: one line for the result, one for each finding.
pub fn report_text(report: &GeoReport, lang: Lang) -> String {
    let mut out = String::new();
    let result = if report.passed() {
        word("pass", lang)
    } else {
        word("fail", lang)
    };
    out.push_str(&format!(
        "{}: {} ({}: {}, {}: {})\n",
        result,
        report.profile,
        word("profile", lang),
        report.profile,
        word("files", lang),
        report.files
    ));
    for f in &report.findings {
        let title = title(f.code, lang).unwrap_or(f.code);
        out.push_str(&format!(
            "{} [{}] {} | {} {} | x={} y={} mm",
            f.code,
            severity_name(f.severity),
            title,
            word("layer", lang),
            f.layer,
            mm(f.x_nm),
            mm(f.y_nm)
        ));
        if f.limit_nm > 0 {
            out.push_str(&format!(
                " | {} {} mm | {} {} mm",
                word("measured", lang),
                mm(f.measured_nm),
                word("limit", lang),
                mm(f.limit_nm)
            ));
        }
        if !f.detail.is_empty() {
            out.push_str(&format!(" | {}", f.detail));
        }
        out.push('\n');
    }
    for (code, count) in &report.dropped {
        out.push_str(&format!(
            "{} {}: {} {}\n",
            code,
            title(code, lang).unwrap_or(code),
            count,
            word("more", lang)
        ));
    }
    out
}

/// The report as JSON: the result, the profile, the findings and what was left out.
pub fn report_json(report: &GeoReport) -> serde_json::Value {
    let mut findings = Vec::new();
    for f in &report.findings {
        let mut item = serde_json::Map::new();
        item.insert("code".to_owned(), serde_json::Value::from(f.code));
        item.insert(
            "severity".to_owned(),
            serde_json::Value::from(severity_name(f.severity)),
        );
        item.insert("layer".to_owned(), serde_json::Value::from(f.layer.clone()));
        item.insert("x_nm".to_owned(), serde_json::Value::from(f.x_nm));
        item.insert("y_nm".to_owned(), serde_json::Value::from(f.y_nm));
        item.insert(
            "measured_nm".to_owned(),
            serde_json::Value::from(f.measured_nm),
        );
        item.insert("limit_nm".to_owned(), serde_json::Value::from(f.limit_nm));
        item.insert(
            "detail".to_owned(),
            serde_json::Value::from(f.detail.clone()),
        );
        let mut titles = serde_json::Map::new();
        for lang in Lang::ALL {
            titles.insert(
                lang.tag().to_owned(),
                serde_json::Value::from(title(f.code, lang).unwrap_or("")),
            );
        }
        item.insert("title".to_owned(), serde_json::Value::Object(titles));
        findings.push(serde_json::Value::Object(item));
    }
    let mut dropped = serde_json::Map::new();
    for (code, count) in &report.dropped {
        dropped.insert((*code).to_owned(), serde_json::Value::from(*count));
    }
    let mut root = serde_json::Map::new();
    root.insert(
        "format".to_owned(),
        serde_json::Value::from("phoenix-gerber-dfm/1"),
    );
    root.insert(
        "passed".to_owned(),
        serde_json::Value::from(report.passed()),
    );
    root.insert(
        "profile".to_owned(),
        serde_json::Value::from(report.profile.clone()),
    );
    root.insert(
        "profile_verified".to_owned(),
        serde_json::Value::from(report.profile_verified),
    );
    root.insert("files".to_owned(), serde_json::Value::from(report.files));
    root.insert("findings".to_owned(), serde_json::Value::Array(findings));
    root.insert("not_listed".to_owned(), serde_json::Value::Object(dropped));
    serde_json::Value::Object(root)
}

fn csv_field(text: &str) -> String {
    if text.contains(['"', ',', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

/// The findings as CSV with a header line: code, severity, layer, x and y in millimetres, the
/// measured value and the limit in millimetres, and the title in the language.
pub fn report_csv(report: &GeoReport, lang: Lang) -> String {
    let mut out = String::from("code,severity,layer,x_mm,y_mm,measured_mm,limit_mm,title\n");
    for f in &report.findings {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            f.code,
            severity_name(f.severity),
            csv_field(&f.layer),
            mm(f.x_nm),
            mm(f.y_nm),
            mm(f.measured_nm),
            mm(f.limit_nm),
            csv_field(title(f.code, lang).unwrap_or(""))
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_its_title_in_the_eight_languages() {
        let mut seen = std::collections::BTreeSet::new();
        for (code, texts) in TITLES {
            assert!(seen.insert(*code), "{code} twice");
            assert!(code.starts_with("PHOENIX-GDF-"));
            for t in texts {
                assert!(!t.trim().is_empty(), "{code}");
            }
            // no two languages share a text except by chance of a name
            let distinct: std::collections::BTreeSet<_> = texts.iter().collect();
            assert!(distinct.len() >= 7, "{code}");
        }
        assert_eq!(TITLES.len(), 11);
        for (key, texts) in WORDS {
            assert!(texts.iter().all(|t| !t.is_empty()), "{key}");
        }
    }

    #[test]
    fn the_portuguese_is_the_one_of_portugal() {
        let pt = title(MASK_BRIDGE, Lang::Pt).unwrap();
        assert!(pt.contains("máscara"));
        assert!(title(SILK_ON_PAD, Lang::Pt).unwrap().contains("pastilha"));
        assert!(title("PHOENIX-GDF-999", Lang::En).is_none());
    }
}
