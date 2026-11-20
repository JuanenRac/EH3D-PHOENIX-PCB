// SPDX-License-Identifier: GPL-3.0-or-later
//! The report of a file that was read, as text in the eight languages of the suite: what the file
//! is (revision, unit), how many layers, holes, parts, nets and copper objects it holds, the
//! stack-up with the thickness, material, dielectric constant and loss tangent of each layer, and
//! what the reader left out. The same file always gives the same bytes.

use std::fmt::Write as _;

use eh3d_phoenix_rules::Lang;

use crate::read_model::IpcDoc;
use crate::stack_summary::{StackKind, StackSummary};

/// The words of the report in one language.
struct Words {
    title: &'static str,
    revision: &'static str,
    unit: &'static str,
    unknown: &'static str,
    layers: &'static str,
    copper: &'static str,
    steps: &'static str,
    holes: &'static str,
    plated: &'static str,
    not_plated: &'static str,
    slots: &'static str,
    parts: &'static str,
    packages: &'static str,
    padstacks: &'static str,
    nets: &'static str,
    pins: &'static str,
    pads: &'static str,
    lines: &'static str,
    arcs: &'static str,
    polylines: &'static str,
    regions: &'static str,
    bom: &'static str,
    stackup: &'static str,
    no_stackup: &'static str,
    total: &'static str,
    declared: &'static str,
    left_out: &'static str,
    layer: &'static str,
    kind: &'static str,
    material: &'static str,
    thickness: &'static str,
    findings: &'static str,
    none: &'static str,
    copper_word: &'static str,
    core_word: &'static str,
    prepreg_word: &'static str,
    mask_word: &'static str,
    not_given: &'static str,
}

fn words(lang: Lang) -> Words {
    match lang {
        Lang::En => Words {
            title: "IPC-2581 file",
            revision: "Revision",
            unit: "Unit of the lengths",
            unknown: "not given",
            layers: "Layers",
            copper: "of them copper",
            steps: "Steps",
            holes: "Holes",
            plated: "plated",
            not_plated: "not plated",
            slots: "Slots",
            parts: "Placed parts",
            packages: "Packages",
            padstacks: "Padstacks",
            nets: "Nets",
            pins: "Pins on the nets",
            pads: "Pads",
            lines: "Lines",
            arcs: "Arcs",
            polylines: "Polylines",
            regions: "Regions",
            bom: "Lines of the bill of materials",
            stackup: "Stack-up",
            no_stackup: "The file has no stack-up",
            total: "Sum of the thicknesses (um)",
            declared: "Thickness declared by the file (um)",
            left_out: "Layers of the stack-up that are not copper, dielectric or mask",
            layer: "Layer",
            kind: "Kind",
            material: "Material",
            thickness: "Thickness (um)",
            findings: "What the reader left out or could not read",
            none: "nothing",
            copper_word: "copper",
            core_word: "core",
            prepreg_word: "prepreg",
            mask_word: "solder mask",
            not_given: "-",
        },
        Lang::Es => Words {
            title: "Archivo IPC-2581",
            revision: "Revisión",
            unit: "Unidad de las longitudes",
            unknown: "no indicada",
            layers: "Capas",
            copper: "de ellas de cobre",
            steps: "Pasos (step)",
            holes: "Taladros",
            plated: "metalizados",
            not_plated: "sin metalizar",
            slots: "Ranuras",
            parts: "Componentes colocados",
            packages: "Encapsulados",
            padstacks: "Padstacks",
            nets: "Redes",
            pins: "Pines en las redes",
            pads: "Pads",
            lines: "Líneas",
            arcs: "Arcos",
            polylines: "Polilíneas",
            regions: "Regiones",
            bom: "Líneas de la lista de materiales",
            stackup: "Apilado",
            no_stackup: "El archivo no tiene apilado",
            total: "Suma de los espesores (um)",
            declared: "Espesor declarado por el archivo (um)",
            left_out: "Capas del apilado que no son cobre, dieléctrico ni máscara",
            layer: "Capa",
            kind: "Tipo",
            material: "Material",
            thickness: "Espesor (um)",
            findings: "Lo que el lector omitió o no pudo leer",
            none: "nada",
            copper_word: "cobre",
            core_word: "núcleo",
            prepreg_word: "prepreg",
            mask_word: "máscara de soldadura",
            not_given: "-",
        },
        Lang::De => Words {
            title: "IPC-2581-Datei",
            revision: "Revision",
            unit: "Einheit der Längen",
            unknown: "nicht angegeben",
            layers: "Lagen",
            copper: "davon Kupfer",
            steps: "Schritte (Step)",
            holes: "Bohrungen",
            plated: "durchkontaktiert",
            not_plated: "nicht durchkontaktiert",
            slots: "Langlöcher",
            parts: "Platzierte Bauteile",
            packages: "Gehäuse",
            padstacks: "Padstacks",
            nets: "Netze",
            pins: "Pins an den Netzen",
            pads: "Pads",
            lines: "Linien",
            arcs: "Bögen",
            polylines: "Polylinien",
            regions: "Flächen",
            bom: "Zeilen der Stückliste",
            stackup: "Lagenaufbau",
            no_stackup: "Die Datei hat keinen Lagenaufbau",
            total: "Summe der Dicken (um)",
            declared: "In der Datei angegebene Dicke (um)",
            left_out:
                "Lagen des Aufbaus, die weder Kupfer noch Dielektrikum noch Lötstopplack sind",
            layer: "Lage",
            kind: "Art",
            material: "Material",
            thickness: "Dicke (um)",
            findings: "Was der Leser ausgelassen hat oder nicht lesen konnte",
            none: "nichts",
            copper_word: "Kupfer",
            core_word: "Kern",
            prepreg_word: "Prepreg",
            mask_word: "Lötstopplack",
            not_given: "-",
        },
        Lang::Fr => Words {
            title: "Fichier IPC-2581",
            revision: "Révision",
            unit: "Unité des longueurs",
            unknown: "non indiquée",
            layers: "Couches",
            copper: "dont cuivre",
            steps: "Étapes (step)",
            holes: "Perçages",
            plated: "métallisés",
            not_plated: "non métallisés",
            slots: "Fentes",
            parts: "Composants placés",
            packages: "Boîtiers",
            padstacks: "Padstacks",
            nets: "Réseaux",
            pins: "Broches sur les réseaux",
            pads: "Pastilles",
            lines: "Lignes",
            arcs: "Arcs",
            polylines: "Polylignes",
            regions: "Régions",
            bom: "Lignes de la nomenclature",
            stackup: "Empilage",
            no_stackup: "Le fichier n'a pas d'empilage",
            total: "Somme des épaisseurs (um)",
            declared: "Épaisseur déclarée par le fichier (um)",
            left_out:
                "Couches de l'empilage qui ne sont ni cuivre, ni diélectrique, ni vernis épargne",
            layer: "Couche",
            kind: "Type",
            material: "Matériau",
            thickness: "Épaisseur (um)",
            findings: "Ce que le lecteur a laissé de côté ou n'a pas pu lire",
            none: "rien",
            copper_word: "cuivre",
            core_word: "âme",
            prepreg_word: "préimprégné",
            mask_word: "vernis épargne",
            not_given: "-",
        },
        Lang::It => Words {
            title: "File IPC-2581",
            revision: "Revisione",
            unit: "Unità delle lunghezze",
            unknown: "non indicata",
            layers: "Strati",
            copper: "di cui rame",
            steps: "Passi (step)",
            holes: "Fori",
            plated: "metallizzati",
            not_plated: "non metallizzati",
            slots: "Asole",
            parts: "Componenti posizionati",
            packages: "Contenitori",
            padstacks: "Padstack",
            nets: "Reti",
            pins: "Pin sulle reti",
            pads: "Piazzole",
            lines: "Linee",
            arcs: "Archi",
            polylines: "Polilinee",
            regions: "Regioni",
            bom: "Righe della distinta base",
            stackup: "Stack-up",
            no_stackup: "Il file non ha uno stack-up",
            total: "Somma degli spessori (um)",
            declared: "Spessore dichiarato dal file (um)",
            left_out: "Strati dello stack-up che non sono rame, dielettrico né maschera",
            layer: "Strato",
            kind: "Tipo",
            material: "Materiale",
            thickness: "Spessore (um)",
            findings: "Ciò che il lettore ha omesso o non ha potuto leggere",
            none: "niente",
            copper_word: "rame",
            core_word: "core",
            prepreg_word: "prepreg",
            mask_word: "maschera di saldatura",
            not_given: "-",
        },
        Lang::Ja => Words {
            title: "IPC-2581 ファイル",
            revision: "リビジョン",
            unit: "長さの単位",
            unknown: "指定なし",
            layers: "層",
            copper: "うち銅層",
            steps: "ステップ",
            holes: "穴",
            plated: "めっきあり",
            not_plated: "めっきなし",
            slots: "スロット",
            parts: "配置された部品",
            packages: "パッケージ",
            padstacks: "パッドスタック",
            nets: "ネット",
            pins: "ネット上のピン",
            pads: "パッド",
            lines: "線",
            arcs: "円弧",
            polylines: "ポリライン",
            regions: "領域",
            bom: "部品表の行",
            stackup: "スタックアップ",
            no_stackup: "ファイルにスタックアップがありません",
            total: "厚さの合計 (um)",
            declared: "ファイルが宣言する厚さ (um)",
            left_out: "銅・誘電体・ソルダーマスクのいずれでもないスタックアップの層",
            layer: "層",
            kind: "種類",
            material: "材料",
            thickness: "厚さ (um)",
            findings: "リーダーが除外した、または読めなかったもの",
            none: "なし",
            copper_word: "銅",
            core_word: "コア",
            prepreg_word: "プリプレグ",
            mask_word: "ソルダーマスク",
            not_given: "-",
        },
        Lang::Zh => Words {
            title: "IPC-2581 文件",
            revision: "版本",
            unit: "长度单位",
            unknown: "未给出",
            layers: "层",
            copper: "其中铜层",
            steps: "步骤 (step)",
            holes: "孔",
            plated: "已金属化",
            not_plated: "未金属化",
            slots: "槽",
            parts: "已放置的元件",
            packages: "封装",
            padstacks: "焊盘叠",
            nets: "网络",
            pins: "网络上的引脚",
            pads: "焊盘",
            lines: "线",
            arcs: "圆弧",
            polylines: "折线",
            regions: "区域",
            bom: "物料清单的行数",
            stackup: "叠层",
            no_stackup: "文件没有叠层",
            total: "厚度之和 (um)",
            declared: "文件声明的厚度 (um)",
            left_out: "叠层中既不是铜、介质也不是阻焊的层",
            layer: "层",
            kind: "类型",
            material: "材料",
            thickness: "厚度 (um)",
            findings: "读取器忽略或无法读取的内容",
            none: "无",
            copper_word: "铜",
            core_word: "芯板",
            prepreg_word: "半固化片",
            mask_word: "阻焊",
            not_given: "-",
        },
        Lang::Pt => Words {
            title: "Ficheiro IPC-2581",
            revision: "Revisão",
            unit: "Unidade dos comprimentos",
            unknown: "não indicada",
            layers: "Camadas",
            copper: "das quais de cobre",
            steps: "Passos (step)",
            holes: "Furos",
            plated: "metalizados",
            not_plated: "não metalizados",
            slots: "Ranhuras",
            parts: "Componentes colocados",
            packages: "Encapsulamentos",
            padstacks: "Padstacks",
            nets: "Redes",
            pins: "Pinos nas redes",
            pads: "Pastilhas",
            lines: "Linhas",
            arcs: "Arcos",
            polylines: "Polilinhas",
            regions: "Regiões",
            bom: "Linhas da lista de materiais",
            stackup: "Empilhamento",
            no_stackup: "O ficheiro não tem empilhamento",
            total: "Soma das espessuras (um)",
            declared: "Espessura declarada pelo ficheiro (um)",
            left_out: "Camadas do empilhamento que não são cobre, dielétrico nem máscara",
            layer: "Camada",
            kind: "Tipo",
            material: "Material",
            thickness: "Espessura (um)",
            findings: "O que o leitor deixou de fora ou não conseguiu ler",
            none: "nada",
            copper_word: "cobre",
            core_word: "núcleo",
            prepreg_word: "pré-impregnado",
            mask_word: "máscara de soldadura",
            not_given: "-",
        },
    }
}

/// Nanometres as micrometres with three decimals, without any rounding.
pub fn micrometres(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let magnitude = nm.unsigned_abs();
    format!("{sign}{}.{:03}", magnitude / 1000, magnitude % 1000)
}

fn kind_word(w: &Words, kind: StackKind) -> &'static str {
    match kind {
        StackKind::Copper => w.copper_word,
        StackKind::Core => w.core_word,
        StackKind::Prepreg => w.prepreg_word,
        StackKind::SolderMask => w.mask_word,
    }
}

/// The report of a file in a language. `stack` is the stack-up made from the file, when there is
/// one.
pub fn report_text(doc: &IpcDoc, stack: Option<&StackSummary>, lang: Lang) -> String {
    let w = words(lang);
    let c = doc.counts();
    let mut out = String::new();
    let _ = writeln!(out, "{}", w.title);
    let revision = if doc.revision.is_empty() {
        w.unknown
    } else {
        doc.revision.as_str()
    };
    let _ = writeln!(out, "{}: {revision}", w.revision);
    let unit = doc.units.map_or(w.unknown, |u| u.name());
    let _ = writeln!(out, "{}: {unit}", w.unit);
    let _ = writeln!(
        out,
        "{}: {} ({}: {})",
        w.layers, c.layers, w.copper, c.copper_layers
    );
    let _ = writeln!(out, "{}: {}", w.steps, c.steps);
    let _ = writeln!(
        out,
        "{}: {} ({}: {}, {}: {})",
        w.holes, c.holes, w.plated, c.holes_plated, w.not_plated, c.holes_non_plated
    );
    let counts = [
        (w.slots, c.slots),
        (w.parts, c.components),
        (w.packages, c.packages),
        (w.padstacks, c.padstacks),
        (w.nets, c.nets),
        (w.pins, c.net_pins),
        (w.pads, c.pads),
        (w.lines, c.lines),
        (w.arcs, c.arcs),
        (w.polylines, c.polylines),
        (w.regions, c.regions),
        (w.bom, c.bom_items),
    ];
    for (label, n) in counts {
        let _ = writeln!(out, "{label}: {n}");
    }
    let _ = writeln!(out, "{}:", w.stackup);
    match stack {
        None => {
            let _ = writeln!(out, "  {}", w.no_stackup);
        }
        Some(s) => {
            let _ = writeln!(
                out,
                "  {} | {} | {} | {} | Dk | Df",
                w.layer, w.kind, w.material, w.thickness
            );
            for r in &s.rows {
                let thickness = r.thickness_nm.map_or(w.not_given.to_owned(), micrometres);
                let dk = r.dk.map_or(w.not_given.to_owned(), |v| v.to_string());
                let df = r.df.map_or(w.not_given.to_owned(), |v| v.to_string());
                let material = if r.material.is_empty() {
                    w.not_given
                } else {
                    r.material.as_str()
                };
                let _ = writeln!(
                    out,
                    "  {:>2} {} | {} | {material} | {thickness} | {dk} | {df}",
                    r.position,
                    r.name,
                    kind_word(&w, r.kind)
                );
            }
            let _ = writeln!(out, "  {}: {}", w.total, micrometres(s.total_nm));
            if let Some(d) = s.declared_nm {
                let _ = writeln!(out, "  {}: {}", w.declared, micrometres(d));
            }
            if s.left_out > 0 {
                let _ = writeln!(out, "  {}: {}", w.left_out, s.left_out);
            }
        }
    }
    let _ = writeln!(out, "{}:", w.findings);
    let mut any = false;
    for f in &doc.findings {
        let _ = writeln!(out, "  {}", f.line(lang));
        any = true;
    }
    if let Some(s) = stack {
        for f in &s.findings {
            let _ = writeln!(out, "  {}", f.line(lang));
            any = true;
        }
    }
    if !any {
        let _ = writeln!(out, "  {}", w.none);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::{read_str, ReadOptions};
    use crate::stack_summary::stack_summary;

    const FILE: &str = r#"<IPC-2581 revision="C"><Ecad name="e"><CadHeader units="MILLIMETER"/><CadData>
<Layer name="T" layerFunction="SIGNAL" side="TOP" polarity="POSITIVE"/>
<Layer name="D" layerFunction="DIELCORE" side="INTERNAL" polarity="POSITIVE"/>
<Layer name="B" layerFunction="SIGNAL" side="BOTTOM" polarity="POSITIVE"/>
<Stackup name="S" overallThickness="1.6">
<StackupGroup name="G"><StackupLayer layerOrGroupRef="T" thickness="0.035"/><StackupLayer layerOrGroupRef="D" thickness="1.53"/><StackupLayer layerOrGroupRef="B" thickness="0.035"/></StackupGroup></Stackup>
<Step name="p"><Datum x="0" y="0"/><Mystery k="1"/></Step></CadData></Ecad></IPC-2581>"#;

    #[test]
    fn micrometres_are_exact() {
        assert_eq!(micrometres(35_000), "35.000");
        assert_eq!(micrometres(200_500), "200.500");
        assert_eq!(micrometres(-1_234), "-1.234");
        assert_eq!(micrometres(0), "0.000");
    }

    #[test]
    fn the_report_has_the_counts_the_stack_and_the_findings_in_every_language() {
        let doc = read_str(FILE, &ReadOptions::default()).unwrap();
        let stack = stack_summary(&doc, 0);
        let en = report_text(&doc, stack.as_ref(), Lang::En);
        assert!(en.contains("Revision: C"), "{en}");
        assert!(en.contains("Unit of the lengths: MILLIMETER"), "{en}");
        assert!(en.contains("Layers: 3 (of them copper: 2)"), "{en}");
        assert!(en.contains("1530.000"), "{en}");
        assert!(en.contains("Sum of the thicknesses (um): 1600.000"), "{en}");
        assert!(en.contains("PHOENIX-IPC2581-012"), "{en}");
        assert!(en.contains("[Mystery]"), "{en}");
        // dielectric without a constant: reported
        assert!(en.contains("PHOENIX-IPC2581-014"), "{en}");
        let mut seen = std::collections::BTreeSet::new();
        for lang in Lang::ALL {
            let text = report_text(&doc, stack.as_ref(), lang);
            assert!(text.contains("1530.000"), "{lang:?}");
            assert!(text.contains("PHOENIX-IPC2581-012"), "{lang:?}");
            assert!(seen.insert(text), "{lang:?} repeats another language");
        }
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn a_file_without_a_stackup_or_findings_says_so() {
        let doc = read_str(
            "<IPC-2581 revision=\"B\"><Ecad name=\"e\"><CadHeader units=\"INCH\"/></Ecad></IPC-2581>",
            &ReadOptions::default(),
        )
        .unwrap();
        let en = report_text(&doc, None, Lang::En);
        assert!(en.contains("The file has no stack-up"), "{en}");
        assert!(en.ends_with("  nothing\n"), "{en}");
        let pt = report_text(&doc, None, Lang::Pt);
        assert!(pt.contains("O ficheiro não tem empilhamento"), "{pt}");
        assert!(pt.contains("Espessura") || pt.contains("Camadas"), "{pt}");
    }
}
