// SPDX-License-Identifier: GPL-3.0-or-later
//! The stack-up of a board as a page of HTML: the layers from the top to the bottom with their
//! kind, name, material and thickness, how deep each one starts, a bar whose length follows the
//! thickness, and the total thickness with the number of copper layers.
//!
//! The words of the page are in the eight languages of the suite; the numbers, the names of the
//! layers and the names of the materials are written as they are in the stack-up (the names are
//! escaped, so that a name from a file cannot add markup). The page holds no script, no style
//! sheet from outside and no image, and the same stack-up gives the same bytes every time.

use std::fmt::Write as _;

use crate::error::{Language, StackupError};
use crate::stackup::{Layer, Stackup};

/// One layer of the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackRow {
    /// The place from the top, counted from 1.
    pub position: usize,
    /// The kind: `copper`, `core`, `prepreg` or `solder_mask`.
    pub kind: &'static str,
    /// The name of the layer.
    pub name: String,
    /// The name of the material; empty for copper.
    pub material: String,
    /// The thickness in nanometres.
    pub thickness_nm: i64,
    /// How far below the top of the board the layer starts, in nanometres.
    pub from_top_nm: i64,
}

/// The layers as rows, from the top, with the depth at which each starts.
pub fn stackup_rows(stackup: &Stackup) -> Result<Vec<StackRow>, StackupError> {
    let mut rows = Vec::with_capacity(stackup.layers.len());
    let mut depth: i64 = 0;
    for (index, layer) in stackup.layers.iter().enumerate() {
        let thickness_nm = layer.thickness_nm()?;
        rows.push(StackRow {
            position: index + 1,
            kind: kind_word(layer),
            name: layer.name().to_owned(),
            material: layer.material().unwrap_or("").to_owned(),
            thickness_nm,
            from_top_nm: depth,
        });
        depth = depth.saturating_add(thickness_nm);
    }
    Ok(rows)
}

fn kind_word(layer: &Layer) -> &'static str {
    match layer {
        Layer::Copper { .. } => "copper",
        Layer::Core { .. } => "core",
        Layer::Prepreg { .. } => "prepreg",
        Layer::SolderMask { .. } => "solder_mask",
    }
}

/// Nanometres as micrometres with three decimals, without a rounding of any kind.
fn micrometres(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let magnitude = nm.unsigned_abs();
    format!("{sign}{}.{:03}", magnitude / 1000, magnitude % 1000)
}

/// Text made safe to put in a page: the five characters that can open markup or end an attribute
/// are written as references.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// The words of the page in one language.
struct Words {
    code: &'static str,
    title: &'static str,
    position: &'static str,
    kind: &'static str,
    name: &'static str,
    material: &'static str,
    thickness: &'static str,
    from_top: &'static str,
    total: &'static str,
    copper_layers: &'static str,
    copper: &'static str,
    core: &'static str,
    prepreg: &'static str,
    solder_mask: &'static str,
}

fn words(language: Language) -> Words {
    match language {
        Language::En => Words {
            code: "en",
            title: "Stack-up",
            position: "No.",
            kind: "Kind",
            name: "Layer",
            material: "Material",
            thickness: "Thickness (um)",
            from_top: "From the top (um)",
            total: "Total thickness (um)",
            copper_layers: "Copper layers",
            copper: "copper",
            core: "core",
            prepreg: "prepreg",
            solder_mask: "solder mask",
        },
        Language::Es => Words {
            code: "es",
            title: "Apilado",
            position: "N.º",
            kind: "Tipo",
            name: "Capa",
            material: "Material",
            thickness: "Espesor (um)",
            from_top: "Desde arriba (um)",
            total: "Espesor total (um)",
            copper_layers: "Capas de cobre",
            copper: "cobre",
            core: "núcleo",
            prepreg: "preimpregnado",
            solder_mask: "máscara de soldadura",
        },
        Language::De => Words {
            code: "de",
            title: "Lagenaufbau",
            position: "Nr.",
            kind: "Art",
            name: "Lage",
            material: "Material",
            thickness: "Dicke (um)",
            from_top: "Von oben (um)",
            total: "Gesamtdicke (um)",
            copper_layers: "Kupferlagen",
            copper: "Kupfer",
            core: "Kern",
            prepreg: "Prepreg",
            solder_mask: "Lötstopplack",
        },
        Language::Fr => Words {
            code: "fr",
            title: "Empilage",
            position: "N°",
            kind: "Type",
            name: "Couche",
            material: "Matériau",
            thickness: "Épaisseur (um)",
            from_top: "Depuis le haut (um)",
            total: "Épaisseur totale (um)",
            copper_layers: "Couches de cuivre",
            copper: "cuivre",
            core: "noyau",
            prepreg: "préimprégné",
            solder_mask: "vernis épargne",
        },
        Language::It => Words {
            code: "it",
            title: "Stack-up",
            position: "N.",
            kind: "Tipo",
            name: "Strato",
            material: "Materiale",
            thickness: "Spessore (um)",
            from_top: "Dall'alto (um)",
            total: "Spessore totale (um)",
            copper_layers: "Strati di rame",
            copper: "rame",
            core: "core",
            prepreg: "prepreg",
            solder_mask: "maschera di saldatura",
        },
        Language::Ja => Words {
            code: "ja",
            title: "層構成",
            position: "番号",
            kind: "種類",
            name: "層",
            material: "材料",
            thickness: "厚さ (um)",
            from_top: "上面から (um)",
            total: "総厚 (um)",
            copper_layers: "銅層の数",
            copper: "銅",
            core: "コア",
            prepreg: "プリプレグ",
            solder_mask: "ソルダーマスク",
        },
        Language::Zh => Words {
            code: "zh",
            title: "叠层",
            position: "序号",
            kind: "类型",
            name: "层",
            material: "材料",
            thickness: "厚度 (um)",
            from_top: "距顶面 (um)",
            total: "总厚度 (um)",
            copper_layers: "铜层数",
            copper: "铜",
            core: "芯板",
            prepreg: "半固化片",
            solder_mask: "阻焊层",
        },
        Language::Pt => Words {
            code: "pt",
            title: "Empilhamento",
            position: "N.º",
            kind: "Tipo",
            name: "Camada",
            material: "Material",
            thickness: "Espessura (um)",
            from_top: "A partir de cima (um)",
            total: "Espessura total (um)",
            copper_layers: "Camadas de cobre",
            copper: "cobre",
            core: "núcleo",
            prepreg: "pré-impregnado",
            solder_mask: "máscara de solda",
        },
    }
}

fn kind_text<'a>(w: &'a Words, kind: &str) -> &'a str {
    match kind {
        "copper" => w.copper,
        "core" => w.core,
        "prepreg" => w.prepreg,
        _ => w.solder_mask,
    }
}

/// The report as a complete page of HTML in a language.
pub fn stackup_html(stackup: &Stackup, language: Language) -> Result<String, StackupError> {
    let rows = stackup_rows(stackup)?;
    let w = words(language);
    let total: i64 = rows
        .iter()
        .fold(0_i64, |t, r| t.saturating_add(r.thickness_nm));
    let thickest = rows
        .iter()
        .map(|r| r.thickness_nm)
        .max()
        .unwrap_or(0)
        .max(1);
    let copper = rows.iter().filter(|r| r.kind == "copper").count();
    let name = escape(&stackup.name);
    let mut out = String::new();
    let _ = writeln!(out, "<!DOCTYPE html>");
    let _ = writeln!(out, "<html lang=\"{}\">", w.code);
    let _ = writeln!(
        out,
        "<head><meta charset=\"utf-8\"><title>{} - {name}</title></head>",
        w.title
    );
    let _ = writeln!(out, "<body>");
    let _ = writeln!(out, "<h1>{} - {name}</h1>", w.title);
    let _ = writeln!(
        out,
        "<table border=\"1\" cellspacing=\"0\" cellpadding=\"4\">"
    );
    let _ = writeln!(
        out,
        "<tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th></th></tr>",
        w.position, w.kind, w.name, w.material, w.thickness, w.from_top
    );
    for row in &rows {
        // A bar is at least 1 per cent wide, so that a thin layer can still be seen.
        let share = (i128::from(row.thickness_nm) * 100 / i128::from(thickest)).clamp(1, 100);
        let _ = writeln!(
            out,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><div style=\"background:#888;height:10px;width:{share}%\"></div></td></tr>",
            row.position,
            kind_text(&w, row.kind),
            escape(&row.name),
            escape(&row.material),
            micrometres(row.thickness_nm),
            micrometres(row.from_top_nm)
        );
    }
    let _ = writeln!(out, "</table>");
    let _ = writeln!(out, "<p>{}: {}</p>", w.total, micrometres(total));
    let _ = writeln!(out, "<p>{}: {copper}</p>", w.copper_layers);
    let _ = writeln!(out, "</body>");
    let _ = writeln!(out, "</html>");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stackup::CopperThickness;

    fn copper(name: &str, um: f64) -> Layer {
        Layer::Copper {
            name: name.to_owned(),
            thickness: CopperThickness::Um(um),
            roughness_rms_nm: 0,
        }
    }

    fn core(name: &str, nm: i64) -> Layer {
        Layer::Core {
            name: name.to_owned(),
            thickness_nm: nm,
            material: "FR-4 <TG150>".to_owned(),
        }
    }

    fn four_layers() -> Stackup {
        Stackup {
            name: "four & \"co\"".to_owned(),
            layers: vec![
                copper("F.Cu", 35.0),
                Layer::Prepreg {
                    name: "pp1".to_owned(),
                    thickness_nm: 100_000,
                    material: "FR-4".to_owned(),
                },
                copper("In1", 17.5),
                core("core", 1_000_000),
                copper("In2", 17.5),
                Layer::Prepreg {
                    name: "pp2".to_owned(),
                    thickness_nm: 100_000,
                    material: "FR-4".to_owned(),
                },
                copper("B.Cu", 35.0),
            ],
        }
    }

    #[test]
    fn the_rows_start_where_the_one_above_ends() {
        let rows = stackup_rows(&four_layers()).unwrap();
        assert_eq!(rows.len(), 7);
        assert_eq!(rows[0].from_top_nm, 0);
        for pair in rows.windows(2) {
            assert_eq!(
                pair[1].from_top_nm,
                pair[0].from_top_nm + pair[0].thickness_nm
            );
        }
        assert_eq!(rows[0].kind, "copper");
        assert_eq!(rows[1].kind, "prepreg");
        assert_eq!(rows[3].kind, "core");
        assert_eq!(rows[3].material, "FR-4 <TG150>");
        assert_eq!(rows[6].position, 7);
    }

    #[test]
    fn micrometres_are_exact() {
        assert_eq!(micrometres(0), "0.000");
        assert_eq!(micrometres(1), "0.001");
        assert_eq!(micrometres(35_000), "35.000");
        assert_eq!(micrometres(1_234_567), "1234.567");
        assert_eq!(micrometres(-1_500), "-1.500");
        assert_eq!(micrometres(i64::MIN), "-9223372036854775.808");
    }

    #[test]
    fn the_page_has_the_totals_and_the_names_are_escaped() {
        let page = stackup_html(&four_layers(), Language::En).unwrap();
        assert!(page.starts_with("<!DOCTYPE html>\n<html lang=\"en\">"));
        assert!(page.contains("four &amp; &quot;co&quot;"));
        assert!(page.contains("FR-4 &lt;TG150&gt;"));
        assert!(!page.contains("<TG150>"));
        // 35 + 100 + 17.5 + 1000 + 17.5 + 100 + 35 micrometres
        assert!(page.contains("Total thickness (um): 1305.000"), "{page}");
        assert!(page.contains("Copper layers: 4"));
        assert_eq!(page.matches("<tr><td>").count(), 7);
        assert!(!page.contains("<script"));
    }

    #[test]
    fn every_language_has_its_own_words_and_the_same_numbers() {
        let mut pages = std::collections::BTreeSet::new();
        for language in Language::ALL {
            let page = stackup_html(&four_layers(), language).unwrap();
            assert!(page.contains("1305.000"), "{language:?}");
            assert!(page.contains("35.000"), "{language:?}");
            assert_eq!(page.matches("<tr><td>").count(), 7, "{language:?}");
            assert!(pages.insert(page), "{language:?} repeats another language");
        }
        assert_eq!(pages.len(), 8);
        let codes: std::collections::BTreeSet<&str> =
            Language::ALL.iter().map(|l| words(*l).code).collect();
        assert_eq!(codes.len(), 8);
    }

    #[test]
    fn the_bars_follow_the_thickness_and_never_vanish() {
        let page = stackup_html(&four_layers(), Language::En).unwrap();
        assert!(page.contains("width:100%"));
        // 35 um of copper beside 1000 um of core is 3 per cent; 17.5 um is 1 per cent.
        assert!(page.contains("width:3%"));
        assert!(page.contains("width:1%"));
    }

    #[test]
    fn a_stack_with_no_layer_gives_an_empty_table() {
        let empty = Stackup {
            name: "none".to_owned(),
            layers: Vec::new(),
        };
        let page = stackup_html(&empty, Language::Pt).unwrap();
        assert!(page.contains("Espessura total (um): 0.000"));
        assert!(page.contains("Camadas de cobre: 0"));
    }

    #[test]
    fn a_thickness_that_cannot_be_read_is_an_error() {
        let bad = Stackup {
            name: "bad".to_owned(),
            layers: vec![copper("F.Cu", -1.0)],
        };
        assert!(stackup_rows(&bad).is_err());
        assert!(stackup_html(&bad, Language::En).is_err());
    }
}
