// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks of a library: pins without a number or repeated, pads with the same name, pads
//! without copper, a drill larger than its pad, silkscreen over a pad, devices with pins that
//! reach no pad, connections to things that do not exist, and footprints without an origin. The
//! report is deterministic: the findings are sorted, and the same library always gives the same
//! text and the same JSON.

use std::collections::BTreeMap;

use eh3d_phoenix_rules::Lang;
use serde::Serialize;

use crate::codes::{
    title, CONNECT_BROKEN, DEVICE_NO_PACKAGE, DEVICE_PIN_NO_PAD, DRILL_TOO_BIG, GATE_NO_SYMBOL,
    NO_NAME_TEXT, NO_ORIGIN, PAD_DUPLICATE, PAD_NO_COPPER, PIN_DUPLICATE, PIN_NO_NAME, SET_EMPTY,
    SET_NO_PREFIX, SILK_OVER_PAD,
};
use crate::geom::{auto_diameter, pad_shape, stroke_distance, Bounds};
use crate::model::{Item, Library, Package, Symbol};
use crate::settings::LibSettings;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The library will not work as it is.
    Error,
    /// Probably a mistake.
    Warning,
}

impl Severity {
    /// The word for the severity.
    pub fn word(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// One finding of the check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// The code, `PHOENIX-LIB-<NNN>`.
    pub code: &'static str,
    /// The severity.
    pub severity: Severity,
    /// `package`, `symbol` or `deviceset`.
    pub kind: &'static str,
    /// The name of the footprint, symbol or device set.
    pub name: String,
    /// What is specific to the case (a pad, a pin, a device).
    pub detail: String,
}

impl Finding {
    fn kind_rank(&self) -> u8 {
        match self.kind {
            "package" => 0,
            "symbol" => 1,
            _ => 2,
        }
    }
}

/// The result of checking a library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckReport {
    /// How many footprints were checked.
    pub packages: usize,
    /// How many symbols were checked.
    pub symbols: usize,
    /// How many device sets were checked.
    pub devicesets: usize,
    /// The findings, sorted by kind, name, code and detail.
    pub findings: Vec<Finding>,
}

impl CheckReport {
    /// How many findings are errors.
    pub fn errors(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }

    /// How many findings are warnings.
    pub fn warnings(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count()
    }

    /// The findings of one footprint, symbol or device set.
    pub fn about(&self, kind: &str, name: &str) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.kind == kind && f.name == name)
            .collect()
    }

    /// The report as JSON, with a line feed at the end.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// The report as text in a language: one line per finding with its code, its title and its
    /// detail, then the totals.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = String::new();
        for f in &self.findings {
            out.push_str(&format!(
                "{} {} {} {}: {}",
                f.severity.word(),
                f.code,
                f.kind,
                f.name,
                title(f.code, lang)
            ));
            if !f.detail.is_empty() {
                out.push_str(&format!(" ({})", f.detail));
            }
            out.push('\n');
        }
        out.push_str(&format!(
            "{} footprints, {} symbols, {} device sets: {} errors, {} warnings\n",
            self.packages,
            self.symbols,
            self.devicesets,
            self.errors(),
            self.warnings()
        ));
        out
    }
}

struct Collector {
    findings: Vec<Finding>,
}

impl Collector {
    fn add(
        &mut self,
        code: &'static str,
        severity: Severity,
        kind: &'static str,
        name: &str,
        detail: impl Into<String>,
    ) {
        self.findings.push(Finding {
            code,
            severity,
            kind,
            name: name.to_owned(),
            detail: detail.into(),
        });
    }
}

fn check_package(p: &Package, s: &LibSettings, c: &mut Collector) {
    // the names of the pads
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, item) in p.pads() {
        if let Some(name) = item.name() {
            *seen.entry(name).or_insert(0) += 1;
        }
    }
    for (name, count) in &seen {
        if *count > 1 {
            c.add(
                PAD_DUPLICATE,
                Severity::Error,
                "package",
                &p.name,
                format!("{name} x{count}"),
            );
        }
    }
    for (_, item) in p.pads() {
        let name = item.name().unwrap_or("");
        match item {
            Item::Smd(m) => {
                if !s.copper_layers.contains(&m.layer) {
                    c.add(
                        PAD_NO_COPPER,
                        Severity::Error,
                        "package",
                        &p.name,
                        format!("{name}: layer {}", m.layer),
                    );
                } else if m.dx <= 0 || m.dy <= 0 {
                    c.add(
                        PAD_NO_COPPER,
                        Severity::Error,
                        "package",
                        &p.name,
                        format!("{name}: no size"),
                    );
                }
            }
            Item::Pad(t) => {
                let copper = if t.diameter > 0 {
                    t.diameter
                } else {
                    auto_diameter(t.drill)
                };
                if t.drill <= 0 {
                    c.add(
                        PAD_NO_COPPER,
                        Severity::Error,
                        "package",
                        &p.name,
                        format!("{name}: no drill"),
                    );
                } else if t.drill > copper {
                    c.add(
                        DRILL_TOO_BIG,
                        Severity::Error,
                        "package",
                        &p.name,
                        format!("{name}: drill {} nm, pad {} nm", t.drill, copper),
                    );
                }
            }
            _ => {}
        }
    }
    // silkscreen over the pads
    let shapes: Vec<(String, crate::geom::Shape)> = p
        .pads()
        .filter_map(|(_, i)| pad_shape(i).map(|shape| (i.name().unwrap_or("").to_owned(), shape)))
        .collect();
    for item in &p.items {
        let on_silk = item.layer().is_some_and(|l| s.silk_layers.contains(&l));
        if !on_silk {
            continue;
        }
        for (name, shape) in &shapes {
            if let Some(distance) = stroke_distance(item, shape) {
                if distance < s.silk_clearance_nm as f64 || distance == 0.0 {
                    c.add(
                        SILK_OVER_PAD,
                        Severity::Warning,
                        "package",
                        &p.name,
                        format!("{} over pad {name}", item.kind()),
                    );
                }
            }
        }
    }
    // the origin
    let mut bounds: Option<Bounds> = None;
    for (_, item) in p.pads() {
        if let Some((x, y)) = item.pad_centre() {
            match bounds.as_mut() {
                Some(b) => b.include(x, y),
                None => bounds = Some(Bounds::at(x, y)),
            }
        }
    }
    match bounds {
        Some(b) => {
            if !b.contains(0, 0, s.origin_tolerance_nm) {
                c.add(
                    NO_ORIGIN,
                    Severity::Warning,
                    "package",
                    &p.name,
                    format!(
                        "pads from ({}, {}) to ({}, {}) nm",
                        b.min_x, b.min_y, b.max_x, b.max_y
                    ),
                );
            }
        }
        None => c.add(NO_ORIGIN, Severity::Warning, "package", &p.name, "no pads"),
    }
    // the texts of the name and the value
    if s.require_name_texts {
        let has = |word: &str| {
            p.items.iter().any(|i| match i {
                Item::Text(t) => t.content.eq_ignore_ascii_case(word),
                _ => false,
            })
        };
        let mut missing = Vec::new();
        if !has(">NAME") {
            missing.push(">NAME");
        }
        if !has(">VALUE") {
            missing.push(">VALUE");
        }
        if !missing.is_empty() {
            c.add(
                NO_NAME_TEXT,
                Severity::Warning,
                "package",
                &p.name,
                missing.join(" "),
            );
        }
    }
}

fn check_symbol(sym: &Symbol, c: &mut Collector) {
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, pin) in sym.pins() {
        if pin.name.is_empty() {
            c.add(
                PIN_NO_NAME,
                Severity::Error,
                "symbol",
                &sym.name,
                format!("pin at ({}, {}) nm", pin.x, pin.y),
            );
        } else {
            *seen.entry(pin.name.as_str()).or_insert(0) += 1;
        }
    }
    for (name, count) in &seen {
        if *count > 1 {
            c.add(
                PIN_DUPLICATE,
                Severity::Error,
                "symbol",
                &sym.name,
                format!("{name} x{count}"),
            );
        }
    }
}

fn check_deviceset(lib: &Library, set: &crate::model::DeviceSet, c: &mut Collector) {
    if set.gates.is_empty() || set.devices.is_empty() {
        c.add(
            SET_EMPTY,
            Severity::Warning,
            "deviceset",
            &set.name,
            format!("{} gates, {} devices", set.gates.len(), set.devices.len()),
        );
    }
    if set.prefix.is_empty() {
        c.add(SET_NO_PREFIX, Severity::Warning, "deviceset", &set.name, "");
    }
    for gate in &set.gates {
        if lib.symbol(&gate.symbol).is_none() {
            c.add(
                GATE_NO_SYMBOL,
                Severity::Error,
                "deviceset",
                &set.name,
                format!("gate {}: symbol {}", gate.name, gate.symbol),
            );
        }
    }
    for dev in &set.devices {
        let label = format!("device {:?}", dev.name);
        let package = match &dev.package {
            None => None,
            Some(name) => match lib.package(name) {
                Some(p) => Some(p),
                None => {
                    c.add(
                        DEVICE_NO_PACKAGE,
                        Severity::Error,
                        "deviceset",
                        &set.name,
                        format!("{label}: footprint {name}"),
                    );
                    None
                }
            },
        };
        for conn in &dev.connects {
            let gate = set.gates.iter().find(|g| g.name == conn.gate);
            let symbol = gate.and_then(|g| lib.symbol(&g.symbol));
            let broken = |c: &mut Collector, what: String| {
                c.add(
                    CONNECT_BROKEN,
                    Severity::Error,
                    "deviceset",
                    &set.name,
                    format!("{label}: {what}"),
                );
            };
            if gate.is_none() {
                broken(c, format!("gate {}", conn.gate));
                continue;
            }
            if let Some(symbol) = symbol {
                if !symbol.pins().any(|(_, p)| p.name == conn.pin) {
                    broken(c, format!("pin {} of gate {}", conn.pin, conn.gate));
                }
            }
            if let Some(package) = package {
                for pad in conn.pad.split_whitespace() {
                    if package.pad_index(pad).is_none() {
                        broken(c, format!("pad {pad} of {}", package.name));
                    }
                }
            }
        }
        if let Some(package) = package {
            for gate in &set.gates {
                let Some(symbol) = lib.symbol(&gate.symbol) else {
                    continue;
                };
                for (_, pin) in symbol.pins() {
                    let connected = dev
                        .connects
                        .iter()
                        .any(|k| k.gate == gate.name && k.pin == pin.name);
                    if !connected {
                        c.add(
                            DEVICE_PIN_NO_PAD,
                            Severity::Warning,
                            "deviceset",
                            &set.name,
                            format!(
                                "{label}: pin {} of gate {} (footprint {})",
                                pin.name, gate.name, package.name
                            ),
                        );
                    }
                }
            }
        }
    }
}

/// Checks a library.
pub fn check_library(lib: &Library, settings: &LibSettings) -> CheckReport {
    let mut c = Collector {
        findings: Vec::new(),
    };
    for p in &lib.packages {
        check_package(p, settings, &mut c);
    }
    for s in &lib.symbols {
        check_symbol(s, &mut c);
    }
    for d in &lib.devicesets {
        check_deviceset(lib, d, &mut c);
    }
    c.findings.sort_by(|a, b| {
        (a.kind_rank(), &a.name, a.code, &a.detail).cmp(&(
            b.kind_rank(),
            &b.name,
            b.code,
            &b.detail,
        ))
    });
    c.findings.dedup();
    CheckReport {
        packages: lib.packages.len(),
        symbols: lib.symbols.len(),
        devicesets: lib.devicesets.len(),
        findings: c.findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Gate, Pad, Pin, Smd, Text, Wire};
    use crate::ops::{apply_all, Operation, Target};
    use crate::read::tests::sample;
    use std::sync::Arc;

    fn codes_of(report: &CheckReport) -> Vec<&'static str> {
        report.findings.iter().map(|f| f.code).collect()
    }

    fn fix(lib: &mut Library, ops: &[Operation]) {
        *lib = apply_all(lib, ops).unwrap();
    }

    #[test]
    fn the_sample_has_only_the_findings_that_it_deserves() {
        let report = check_library(&sample(), &LibSettings::default());
        // R0603 has a polygon the model keeps, pads on both sides of the origin and both texts;
        // HDR1X2 has no texts and its origin is on pad 1; the empty device has no package.
        let found: Vec<(String, &str)> = report
            .findings
            .iter()
            .map(|f| (f.name.clone(), f.code))
            .collect();
        assert_eq!(found, vec![("HDR1X2".to_owned(), NO_NAME_TEXT)]);
        assert_eq!(report.errors(), 0);
        assert_eq!(report.warnings(), 1);
        assert_eq!(
            (report.packages, report.symbols, report.devicesets),
            (2, 1, 1)
        );
    }

    #[test]
    fn pads_with_the_same_name_a_missing_layer_and_a_big_drill_are_found() {
        let mut lib = sample();
        let t = Target::package("HDR1X2");
        fix(
            &mut lib,
            &[
                Operation::AddItem {
                    target: t.clone(),
                    item: Item::Pad(Pad {
                        name: "1".into(),
                        x: 5_080_000,
                        drill: 2_000_000,
                        diameter: 1_000_000,
                        ..Pad::default()
                    }),
                    at: None,
                },
                Operation::AddItem {
                    target: t,
                    item: Item::Smd(Smd {
                        name: "7".into(),
                        x: 7_620_000,
                        dx: 500_000,
                        dy: 500_000,
                        layer: 2,
                        ..Smd::default()
                    }),
                    at: None,
                },
            ],
        );
        let report = check_library(&lib, &LibSettings::default());
        let about = report.about("package", "HDR1X2");
        let codes: Vec<&str> = about.iter().map(|f| f.code).collect();
        assert!(codes.contains(&PAD_DUPLICATE), "{codes:?}");
        assert!(codes.contains(&DRILL_TOO_BIG), "{codes:?}");
        assert!(codes.contains(&PAD_NO_COPPER), "{codes:?}");
        let dup = about.iter().find(|f| f.code == PAD_DUPLICATE).unwrap();
        assert_eq!(dup.detail, "1 x2");
        assert_eq!(dup.severity, Severity::Error);
    }

    #[test]
    fn silkscreen_over_a_pad_is_found_and_silkscreen_beside_it_is_not() {
        let mut lib = sample();
        let t = Target::package("R0603");
        let line = |y: i64| {
            Item::Wire(Wire {
                x1: -300_000,
                y1: y,
                x2: 300_000,
                y2: y,
                width: 150_000,
                layer: 21,
                ..Wire::default()
            })
        };
        // the pads are 0.8 by 0.9 mm centred at 0.8 mm: a line at x from -0.3 to 0.3 beside
        // them does not touch, a line from -1.2 to -0.4 crosses pad 1
        fix(
            &mut lib,
            &[Operation::AddItem {
                target: t.clone(),
                item: line(0),
                at: None,
            }],
        );
        assert!(!codes_of(&check_library(&lib, &LibSettings::default())).contains(&SILK_OVER_PAD));
        fix(
            &mut lib,
            &[Operation::AddItem {
                target: t,
                item: Item::Wire(Wire {
                    x1: -1_200_000,
                    y1: 0,
                    x2: -400_000,
                    y2: 0,
                    width: 150_000,
                    layer: 21,
                    ..Wire::default()
                }),
                at: None,
            }],
        );
        let report = check_library(&lib, &LibSettings::default());
        let hits: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.code == SILK_OVER_PAD)
            .collect();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].detail.ends_with("pad 1"), "{}", hits[0].detail);
        // a clearance in the settings makes the line near pad 2 count too
        let strict = LibSettings {
            silk_clearance_nm: 600_000,
            ..LibSettings::default()
        };
        let near = check_library(&lib, &strict);
        assert!(
            near.findings
                .iter()
                .filter(|f| f.code == SILK_OVER_PAD)
                .count()
                > 1
        );
    }

    #[test]
    fn a_footprint_without_pads_or_far_from_its_origin_has_no_origin() {
        let mut lib = sample();
        fix(
            &mut lib,
            &[Operation::CreatePackage {
                name: "EMPTY".into(),
            }],
        );
        let far = Operation::AddItem {
            target: Target::package("EMPTY"),
            item: Item::Smd(Smd {
                name: "1".into(),
                x: 10_000_000,
                y: 10_000_000,
                dx: 1_000_000,
                dy: 1_000_000,
                ..Smd::default()
            }),
            at: None,
        };
        let report = check_library(&lib, &LibSettings::default());
        let empty = report.about("package", "EMPTY");
        assert!(empty
            .iter()
            .any(|f| f.code == NO_ORIGIN && f.detail == "no pads"));
        fix(&mut lib, &[far]);
        let report = check_library(&lib, &LibSettings::default());
        let empty = report.about("package", "EMPTY");
        assert!(empty
            .iter()
            .any(|f| f.code == NO_ORIGIN && f.detail.starts_with("pads from")));
        // within the tolerance of the settings the origin is accepted
        let loose = LibSettings {
            origin_tolerance_nm: 20_000_000,
            ..LibSettings::default()
        };
        let report = check_library(&lib, &loose);
        assert!(!report
            .about("package", "EMPTY")
            .iter()
            .any(|f| f.code == NO_ORIGIN));
    }

    #[test]
    fn the_texts_of_the_name_and_value_are_asked_for_unless_the_settings_say_no() {
        let lib = sample();
        let report = check_library(&lib, &LibSettings::default());
        let f = &report.about("package", "HDR1X2")[0];
        assert_eq!((f.code, f.detail.as_str()), (NO_NAME_TEXT, ">NAME >VALUE"));
        let relaxed = LibSettings {
            require_name_texts: false,
            ..LibSettings::default()
        };
        assert!(check_library(&lib, &relaxed).findings.is_empty());
        let mut lib = lib;
        fix(
            &mut lib,
            &[Operation::AddItem {
                target: Target::package("HDR1X2"),
                item: Item::Text(Text {
                    layer: 25,
                    size: 1_000_000,
                    content: ">name".into(),
                    ..Text::default()
                }),
                at: None,
            }],
        );
        let report = check_library(&lib, &LibSettings::default());
        assert_eq!(report.about("package", "HDR1X2")[0].detail, ">VALUE");
    }

    #[test]
    fn pins_without_a_name_or_repeated_are_found() {
        let mut lib = sample();
        let mut sym = lib.symbol("R").unwrap().clone();
        sym.items.push(Item::Pin(Pin {
            name: String::new(),
            ..Pin::default()
        }));
        sym.items.push(Item::Pin(Pin {
            name: "1".into(),
            ..Pin::default()
        }));
        lib.symbols[0] = Arc::new(sym);
        let report = check_library(&lib, &LibSettings::default());
        let about = report.about("symbol", "R");
        let codes: Vec<&str> = about.iter().map(|f| f.code).collect();
        assert!(codes.contains(&PIN_NO_NAME) && codes.contains(&PIN_DUPLICATE));
        assert_eq!(
            about
                .iter()
                .find(|f| f.code == PIN_DUPLICATE)
                .unwrap()
                .detail,
            "1 x2"
        );
    }

    #[test]
    fn a_device_with_pins_that_reach_no_pad_and_broken_references_are_found() {
        let mut lib = sample();
        fix(
            &mut lib,
            &[Operation::Disconnect {
                set: "R".into(),
                device: "0603".into(),
                gate: "G$1".into(),
                pin: "2".into(),
            }],
        );
        let report = check_library(&lib, &LibSettings::default());
        let about = report.about("deviceset", "R");
        let missing = about.iter().find(|f| f.code == DEVICE_PIN_NO_PAD).unwrap();
        assert!(missing.detail.contains("pin 2") && missing.detail.contains("R0603"));
        // break the references by hand: a gate with an unknown symbol, a device with an unknown
        // footprint and a connection to a pad that is not there
        let mut set = lib.deviceset("R").unwrap().clone();
        set.gates.push(Gate {
            name: "G$2".into(),
            symbol: "GONE".into(),
            ..Gate::default()
        });
        set.devices[0].package = Some("GONE".into());
        set.devices[0].connects[0].pad = "99".into();
        lib.devicesets[0] = Arc::new(set);
        let report = check_library(&lib, &LibSettings::default());
        let codes = codes_of(&report);
        assert!(codes.contains(&GATE_NO_SYMBOL));
        assert!(codes.contains(&DEVICE_NO_PACKAGE));
        // with the footprint gone, the pad cannot be judged, so the broken connection is
        // reported once the footprint is back
        let mut set = lib.deviceset("R").unwrap().clone();
        set.devices[0].package = Some("R0603".into());
        lib.devicesets[0] = Arc::new(set);
        let report = check_library(&lib, &LibSettings::default());
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == CONNECT_BROKEN && f.detail.contains("pad 99")));
    }

    #[test]
    fn an_empty_device_set_and_a_missing_prefix_are_found() {
        let mut lib = sample();
        fix(
            &mut lib,
            &[Operation::CreateDeviceSet {
                name: "EMPTYSET".into(),
                prefix: String::new(),
            }],
        );
        let report = check_library(&lib, &LibSettings::default());
        let codes: Vec<&str> = report
            .about("deviceset", "EMPTYSET")
            .iter()
            .map(|f| f.code)
            .collect();
        assert_eq!(codes, vec![SET_EMPTY, SET_NO_PREFIX]);
    }

    #[test]
    fn the_report_is_sorted_and_the_same_every_time() {
        let mut lib = sample();
        fix(
            &mut lib,
            &[
                Operation::CreateSymbol { name: "A".into() },
                Operation::CreatePackage { name: "B".into() },
            ],
        );
        let a = check_library(&lib, &LibSettings::default());
        let b = check_library(&lib, &LibSettings::default());
        assert_eq!(a, b);
        assert_eq!(a.to_json(), b.to_json());
        assert_eq!(a.to_text(Lang::Es), b.to_text(Lang::Es));
        let order: Vec<(&str, &str)> = a
            .findings
            .iter()
            .map(|f| (f.kind, f.name.as_str()))
            .collect();
        let mut sorted = order.clone();
        sorted.sort_by_key(|(kind, name)| (if *kind == "package" { 0 } else { 1 }, *name));
        assert_eq!(order, sorted);
    }

    #[test]
    fn the_text_uses_the_language_and_the_totals() {
        let report = check_library(&sample(), &LibSettings::default());
        let es = report.to_text(Lang::Es);
        assert!(es.contains("PHOENIX-LIB-012"));
        assert!(es.contains("Huella sin texto de nombre o de valor"));
        assert!(es.contains("0 errors, 1 warnings"));
        let pt = report.to_text(Lang::Pt);
        assert!(pt.contains("Pegada sem texto de nome ou de valor"));
        let json = report.to_json();
        assert!(json.contains("\"code\": \"PHOENIX-LIB-012\""));
        assert!(json.contains("\"severity\": \"warning\""));
        assert!(json.ends_with("}\n"));
    }

    #[test]
    fn a_pad_of_a_long_shape_is_checked_against_its_own_size() {
        let mut lib = sample();
        // HDR1X2 pad 2 is long and turned by a quarter: with the automatic diameter of 1.5 mm
        // it is 3 mm along y and 1.5 mm along x, so a line 0.5 mm beyond its end does not
        // touch it and one at its end would
        fix(
            &mut lib,
            &[Operation::AddItem {
                target: Target::package("HDR1X2"),
                item: Item::Wire(Wire {
                    x1: 2_540_000,
                    y1: 2_000_000,
                    x2: 3_540_000,
                    y2: 2_000_000,
                    width: 100_000,
                    layer: 21,
                    ..Wire::default()
                }),
                at: None,
            }],
        );
        let report = check_library(&lib, &LibSettings::default());
        assert!(
            !codes_of(&report).contains(&SILK_OVER_PAD),
            "{:?}",
            report.findings
        );
    }
}
