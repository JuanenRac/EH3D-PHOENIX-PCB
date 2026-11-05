// SPDX-License-Identifier: GPL-3.0-or-later
//! The report that a generator returns with its geometry: the dimensions it calculated, the
//! formulas it used with the source and the range of each, the warnings, the ports, what was put
//! on the board and the result of the rule check. It is written as text in the eight languages of
//! the suite (the words of the report and the titles of the codes; names of formulas, sources and
//! details stay in English) and as JSON.

use std::fmt::Write as _;

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::codes::{title, WARN_RANGE};

/// A number the generator calculated.
#[derive(Debug, Clone, PartialEq)]
pub struct Dimension {
    /// The name.
    pub name: String,
    /// The value.
    pub value: f64,
    /// The unit, or an empty text.
    pub unit: String,
}

/// A formula that was used.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaUse {
    /// What it gives.
    pub name: String,
    /// Where it is published.
    pub source: String,
    /// The range in which the program trusts it.
    pub range: String,
    /// Whether every input was inside that range.
    pub in_range: bool,
}

/// Something to know about the result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// The code (`PHOENIX-RFGEN-...`, or one of the stack-up crate).
    pub code: String,
    /// The detail, in English.
    pub detail: String,
}

/// A port of the structure on the board.
#[derive(Debug, Clone, PartialEq)]
pub struct PortReport {
    /// The name.
    pub name: String,
    /// The x coordinate in nanometres.
    pub x_nm: i64,
    /// The y coordinate in nanometres.
    pub y_nm: i64,
    /// The direction in which the signal leaves the structure, in degrees.
    pub angle_deg: i32,
    /// The impedance of the line at the port in ohm.
    pub z0_ohm: f64,
}

/// What was put on the board.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Polygons of copper.
    pub polygons: usize,
    /// Tracks that are arcs.
    pub arcs: usize,
    /// Vias.
    pub vias: usize,
    /// Parts with pads.
    pub parts: usize,
    /// Nets that were added.
    pub nets_added: usize,
}

/// The result of the rule check of the board with the structure.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleSummary {
    /// Errors of the board before the structure.
    pub errors_before: usize,
    /// Errors of the board with the structure.
    pub errors_after: usize,
    /// Warnings of the board with the structure.
    pub warnings_after: usize,
    /// Findings that the rules could not judge.
    pub inconclusive: usize,
    /// Whether the structure adds no error.
    pub passes: bool,
    /// The proven findings of the board with the structure, one line each.
    pub findings: Vec<String>,
}

/// The report of one structure.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The type of structure.
    pub kind: String,
    /// The frequency in GHz.
    pub freq_ghz: f64,
    /// The name of the copper layer.
    pub layer: String,
    /// The calculated dimensions.
    pub dimensions: Vec<Dimension>,
    /// The formulas used.
    pub formulas: Vec<FormulaUse>,
    /// The warnings.
    pub warnings: Vec<Warning>,
    /// The ports.
    pub ports: Vec<PortReport>,
    /// What was put on the board.
    pub counts: Counts,
    /// The rule check, once it ran.
    pub rules: Option<RuleSummary>,
}

impl Report {
    /// An empty report.
    pub fn new(kind: &str, freq_ghz: f64, layer: &str) -> Report {
        Report {
            kind: kind.to_owned(),
            freq_ghz,
            layer: layer.to_owned(),
            dimensions: Vec::new(),
            formulas: Vec::new(),
            warnings: Vec::new(),
            ports: Vec::new(),
            counts: Counts::default(),
            rules: None,
        }
    }

    /// Adds a dimension.
    pub fn dim(&mut self, name: &str, value: f64, unit: &str) {
        self.dimensions.push(Dimension {
            name: name.to_owned(),
            value,
            unit: unit.to_owned(),
        });
    }

    /// Adds a dimension given in metres, shown in millimetres.
    pub fn dim_mm(&mut self, name: &str, metres: f64) {
        self.dim(name, metres * 1.0e3, "mm");
    }

    /// Records a formula and, for each place where its inputs left its range, a warning.
    pub fn formula(&mut self, name: &str, source: &str, range: &str, problems: &[String]) {
        match self.formulas.iter_mut().find(|f| f.name == name) {
            Some(known) => known.in_range &= problems.is_empty(),
            None => self.formulas.push(FormulaUse {
                name: name.to_owned(),
                source: source.to_owned(),
                range: range.to_owned(),
                in_range: problems.is_empty(),
            }),
        }
        for problem in problems {
            self.warn(WARN_RANGE, &format!("{name}: {problem}"));
        }
    }

    /// Adds a warning, unless the same one is already there.
    pub fn warn(&mut self, code: &str, detail: &str) {
        let warning = Warning {
            code: code.to_owned(),
            detail: detail.to_owned(),
        };
        if !self.warnings.contains(&warning) {
            self.warnings.push(warning);
        }
    }

    /// The value of a dimension by name.
    pub fn value(&self, name: &str) -> Option<f64> {
        self.dimensions
            .iter()
            .find(|d| d.name == name)
            .map(|d| d.value)
    }

    /// The report as JSON. The same report always gives the same text.
    pub fn to_json(&self) -> Value {
        let dimensions: Vec<Value> = self
            .dimensions
            .iter()
            .map(|d| json!({ "name": d.name, "value": d.value, "unit": d.unit }))
            .collect();
        let formulas: Vec<Value> = self
            .formulas
            .iter()
            .map(|f| {
                json!({
                    "name": f.name,
                    "source": f.source,
                    "range": f.range,
                    "in_range": f.in_range,
                })
            })
            .collect();
        let warnings: Vec<Value> = self
            .warnings
            .iter()
            .map(|w| json!({ "code": w.code, "detail": w.detail }))
            .collect();
        let ports: Vec<Value> = self
            .ports
            .iter()
            .map(|p| {
                json!({
                    "name": p.name,
                    "x_nm": p.x_nm,
                    "y_nm": p.y_nm,
                    "angle_deg": p.angle_deg,
                    "z0_ohm": p.z0_ohm,
                })
            })
            .collect();
        let rules = match &self.rules {
            None => Value::Null,
            Some(r) => json!({
                "errors_before": r.errors_before,
                "errors_after": r.errors_after,
                "warnings_after": r.warnings_after,
                "inconclusive": r.inconclusive,
                "passes": r.passes,
                "findings": r.findings,
            }),
        };
        json!({
            "state": "prototype",
            "kind": self.kind,
            "freq_ghz": self.freq_ghz,
            "layer": self.layer,
            "dimensions": dimensions,
            "formulas": formulas,
            "warnings": warnings,
            "ports": ports,
            "counts": {
                "polygons": self.counts.polygons,
                "arcs": self.counts.arcs,
                "vias": self.counts.vias,
                "parts": self.counts.parts,
                "nets_added": self.counts.nets_added,
            },
            "rules": rules,
        })
    }

    /// The report as text in a language.
    pub fn to_text(&self, lang: Lang) -> String {
        let w = words(lang);
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}: {} | {} GHz | {}",
            w.structure, self.kind, self.freq_ghz, self.layer
        );
        let _ = writeln!(out, "{}:", w.dimensions);
        for d in &self.dimensions {
            if d.unit.is_empty() {
                let _ = writeln!(out, "  {} = {:.6}", d.name, d.value);
            } else {
                let _ = writeln!(out, "  {} = {:.6} {}", d.name, d.value, d.unit);
            }
        }
        let _ = writeln!(out, "{}:", w.formulas);
        for f in &self.formulas {
            let flag = if f.in_range { "ok" } else { "OUT OF RANGE" };
            let _ = writeln!(out, "  {} [{}] | {} | {}", f.name, flag, f.source, f.range);
        }
        if !self.warnings.is_empty() {
            let _ = writeln!(out, "{}:", w.warnings);
            for warning in &self.warnings {
                let head = title(&warning.code, lang);
                if head.is_empty() {
                    let _ = writeln!(out, "  {}: {}", warning.code, warning.detail);
                } else {
                    let _ = writeln!(out, "  {}: {}: {}", warning.code, head, warning.detail);
                }
            }
        }
        if !self.ports.is_empty() {
            let _ = writeln!(out, "{}:", w.ports);
            for p in &self.ports {
                let _ = writeln!(
                    out,
                    "  {} at ({}, {}) nm, {} deg, {:.2} ohm",
                    p.name, p.x_nm, p.y_nm, p.angle_deg, p.z0_ohm
                );
            }
        }
        let _ = writeln!(
            out,
            "{}: {} {}, {} {}, {} {}, {} {}, {} {}",
            w.board,
            self.counts.polygons,
            w.polygons,
            self.counts.arcs,
            w.arcs,
            self.counts.vias,
            w.vias,
            self.counts.parts,
            w.parts,
            self.counts.nets_added,
            w.nets
        );
        if let Some(r) = &self.rules {
            let verdict = if r.passes { w.passes } else { w.fails };
            let _ = writeln!(
                out,
                "{}: {} ({} -> {} {}, {} {}, {} {})",
                w.rules,
                verdict,
                r.errors_before,
                r.errors_after,
                w.errors,
                r.warnings_after,
                w.warnings,
                r.inconclusive,
                w.inconclusive
            );
            for line in &r.findings {
                let _ = writeln!(out, "  {line}");
            }
        }
        let _ = writeln!(out, "{}", w.prototype);
        out
    }
}

/// The words of the text report in a language.
struct Words {
    structure: &'static str,
    dimensions: &'static str,
    formulas: &'static str,
    warnings: &'static str,
    ports: &'static str,
    board: &'static str,
    polygons: &'static str,
    arcs: &'static str,
    vias: &'static str,
    parts: &'static str,
    nets: &'static str,
    rules: &'static str,
    passes: &'static str,
    fails: &'static str,
    errors: &'static str,
    inconclusive: &'static str,
    prototype: &'static str,
}

fn words(lang: Lang) -> Words {
    match lang {
        Lang::En => Words {
            structure: "structure",
            dimensions: "dimensions",
            formulas: "formulas",
            warnings: "warnings",
            ports: "ports",
            board: "on the board",
            polygons: "polygons",
            arcs: "arcs",
            vias: "vias",
            parts: "parts",
            nets: "new nets",
            rules: "rules",
            passes: "no new error",
            fails: "new errors",
            errors: "errors",
            inconclusive: "not judged",
            prototype: "Prototype: the formulas are published ones, but the result was not checked with an electromagnetic simulator or measured.",
        },
        Lang::Es => Words {
            structure: "estructura",
            dimensions: "dimensiones",
            formulas: "fórmulas",
            warnings: "avisos",
            ports: "puertos",
            board: "en la placa",
            polygons: "polígonos",
            arcs: "arcos",
            vias: "vías",
            parts: "piezas",
            nets: "redes nuevas",
            rules: "reglas",
            passes: "ningún error nuevo",
            fails: "errores nuevos",
            errors: "errores",
            inconclusive: "sin juzgar",
            prototype: "Prototipo: las fórmulas son publicadas, pero el resultado no se comprobó con un simulador electromagnético ni se midió.",
        },
        Lang::De => Words {
            structure: "Struktur",
            dimensions: "Abmessungen",
            formulas: "Formeln",
            warnings: "Warnungen",
            ports: "Tore",
            board: "auf der Platine",
            polygons: "Polygone",
            arcs: "Bögen",
            vias: "Durchkontaktierungen",
            parts: "Bauteile",
            nets: "neue Netze",
            rules: "Regeln",
            passes: "kein neuer Fehler",
            fails: "neue Fehler",
            errors: "Fehler",
            inconclusive: "nicht beurteilt",
            prototype: "Prototyp: Die Formeln sind veröffentlicht, das Ergebnis wurde aber weder mit einem elektromagnetischen Simulator geprüft noch gemessen.",
        },
        Lang::Fr => Words {
            structure: "structure",
            dimensions: "dimensions",
            formulas: "formules",
            warnings: "avertissements",
            ports: "accès",
            board: "sur la carte",
            polygons: "polygones",
            arcs: "arcs",
            vias: "vias",
            parts: "composants",
            nets: "nouveaux réseaux",
            rules: "règles",
            passes: "aucune nouvelle erreur",
            fails: "nouvelles erreurs",
            errors: "erreurs",
            inconclusive: "non jugés",
            prototype: "Prototype : les formules sont publiées, mais le résultat n'a été ni vérifié avec un simulateur électromagnétique ni mesuré.",
        },
        Lang::It => Words {
            structure: "struttura",
            dimensions: "dimensioni",
            formulas: "formule",
            warnings: "avvisi",
            ports: "porte",
            board: "sulla scheda",
            polygons: "poligoni",
            arcs: "archi",
            vias: "via",
            parts: "componenti",
            nets: "nuove reti",
            rules: "regole",
            passes: "nessun nuovo errore",
            fails: "nuovi errori",
            errors: "errori",
            inconclusive: "non giudicati",
            prototype: "Prototipo: le formule sono pubblicate, ma il risultato non è stato verificato con un simulatore elettromagnetico né misurato.",
        },
        Lang::Ja => Words {
            structure: "構造",
            dimensions: "寸法",
            formulas: "式",
            warnings: "警告",
            ports: "ポート",
            board: "基板上",
            polygons: "ポリゴン",
            arcs: "円弧",
            vias: "ビア",
            parts: "部品",
            nets: "新しいネット",
            rules: "ルール",
            passes: "新しいエラーなし",
            fails: "新しいエラーあり",
            errors: "エラー",
            inconclusive: "未判定",
            prototype: "試作段階：式は公表されたものですが、結果は電磁界シミュレーターでの確認も測定もされていません。",
        },
        Lang::Zh => Words {
            structure: "结构",
            dimensions: "尺寸",
            formulas: "公式",
            warnings: "警告",
            ports: "端口",
            board: "电路板上",
            polygons: "多边形",
            arcs: "圆弧",
            vias: "过孔",
            parts: "元件",
            nets: "新网络",
            rules: "规则",
            passes: "没有新错误",
            fails: "有新错误",
            errors: "错误",
            inconclusive: "未判定",
            prototype: "原型：公式均已发表，但结果未经电磁仿真器验证，也未经测量。",
        },
        Lang::Pt => Words {
            structure: "estrutura",
            dimensions: "dimensões",
            formulas: "fórmulas",
            warnings: "avisos",
            ports: "portas",
            board: "na placa",
            polygons: "polígonos",
            arcs: "arcos",
            vias: "vias",
            parts: "componentes",
            nets: "novas redes",
            rules: "regras",
            passes: "nenhum erro novo",
            fails: "erros novos",
            errors: "erros",
            inconclusive: "não avaliados",
            prototype: "Protótipo: as fórmulas são publicadas, mas o resultado não foi verificado com um simulador eletromagnético nem medido.",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::WARN_FEATURE;

    fn sample() -> Report {
        let mut r = Report::new("line", 2.4, "top");
        r.dim("width", 3.0, "mm");
        r.dim_mm("length", 0.0412);
        r.formula(
            "guided wavelength",
            "Hammerstad and Jensen",
            "0.01 <= W/h <= 100",
            &[],
        );
        r.formula(
            "miter",
            "Douville and James",
            "W/h >= 0.25",
            &["W/h = 0.100 is below 0.25".to_string()],
        );
        r.ports.push(PortReport {
            name: "P1".into(),
            x_nm: 0,
            y_nm: 0,
            angle_deg: 180,
            z0_ohm: 50.0,
        });
        r.counts = Counts {
            polygons: 1,
            ..Counts::default()
        };
        r.rules = Some(RuleSummary {
            passes: true,
            ..RuleSummary::default()
        });
        r
    }

    #[test]
    fn a_formula_out_of_range_adds_one_warning_with_the_range_code() {
        let r = sample();
        assert_eq!(r.formulas.len(), 2);
        assert!(r.formulas[0].in_range && !r.formulas[1].in_range);
        assert_eq!(r.warnings.len(), 1);
        assert_eq!(r.warnings[0].code, WARN_RANGE);
        assert!(r.warnings[0].detail.starts_with("miter: W/h"));
    }

    #[test]
    fn the_same_warning_is_not_added_twice() {
        let mut r = Report::new("line", 1.0, "top");
        r.warn(WARN_FEATURE, "a");
        r.warn(WARN_FEATURE, "a");
        r.warn(WARN_FEATURE, "b");
        assert_eq!(r.warnings.len(), 2);
    }

    #[test]
    fn the_values_are_found_by_name_and_metres_become_millimetres() {
        let r = sample();
        assert_eq!(r.value("width"), Some(3.0));
        assert!((r.value("length").unwrap() - 41.2).abs() < 1.0e-9);
        assert_eq!(r.value("other"), None);
    }

    #[test]
    fn the_json_names_the_state_and_keeps_the_order() {
        let v = sample().to_json();
        assert_eq!(v["state"], "prototype");
        assert_eq!(v["kind"], "line");
        assert_eq!(v["dimensions"][0]["name"], "width");
        assert_eq!(v["formulas"][1]["in_range"], false);
        assert_eq!(v["ports"][0]["angle_deg"], 180);
        assert_eq!(v["rules"]["passes"], true);
        assert_eq!(sample().to_json(), sample().to_json());
        let empty = Report::new("line", 1.0, "top");
        assert!(empty.to_json()["rules"].is_null());
    }

    #[test]
    fn the_text_has_every_section_in_every_language() {
        for lang in Lang::ALL {
            let text = sample().to_text(lang);
            assert!(text.contains("width = 3.000000 mm"), "{text}");
            assert!(text.contains("miter [OUT OF RANGE]"), "{text}");
            assert!(text.contains("PHOENIX-RFGEN-017"), "{text}");
            assert!(text.contains(title(WARN_RANGE, lang)), "{text}");
            assert!(text.contains("P1 at (0, 0) nm"), "{text}");
        }
        let es = sample().to_text(Lang::Es);
        assert!(es.contains("dimensiones:") && es.contains("Prototipo"));
        let pt = sample().to_text(Lang::Pt);
        assert!(pt.contains("Protótipo") && pt.contains("na placa"));
    }

    #[test]
    fn no_word_of_another_language_is_a_copy_of_the_english_one() {
        let en = words(Lang::En);
        for lang in Lang::ALL {
            if lang == Lang::En {
                continue;
            }
            let other = words(lang);
            // a word can be the same in two languages (dimensions in French), but not all of them
            assert!(
                other.dimensions != en.dimensions
                    || other.prototype != en.prototype
                    || other.passes != en.passes,
                "{lang:?}"
            );
            assert_ne!(other.prototype, en.prototype, "{lang:?}");
        }
    }
}
