// SPDX-License-Identifier: GPL-3.0-or-later
//! The checker of the library of footprints, with the codes `PHOENIX-IPC-001` to `010`: unique
//! names that follow the naming, pads that do not overlap, the pitch, the origin at the centre of
//! the pads, the openings of the mask and of the paste, the silkscreen away from the pads and the
//! courtyard of the density level.

use std::collections::BTreeMap;

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::codes::{
    english, title, COURTYARD, MASK_PASTE, NAME_DUPLICATE, NAME_NOT_IPC, ORIGIN_OFF, PADS_OVERLAP,
    PAD_SIZE, PITCH_WRONG, SILK_ON_PAD,
};
use crate::family::FamilySet;
use crate::fp::{pads_bounds, IpcFootprint, Rect};
use crate::settings::IpcSettings;

/// One thing the checker found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The code, `PHOENIX-IPC-<NNN>`.
    pub code: &'static str,
    /// The footprint it is about.
    pub name: String,
    /// What is specific to the case.
    pub detail: String,
}

/// The result of checking a library.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckReport {
    /// How many footprints were checked.
    pub checked: usize,
    /// The findings, in the order of the footprints and then of the codes.
    pub findings: Vec<Finding>,
}

impl CheckReport {
    /// True when nothing was found.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// The findings of one footprint.
    pub fn about(&self, name: &str) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.name == name).collect()
    }

    /// The report as data.
    pub fn to_json(&self) -> Value {
        let findings: Vec<Value> = self
            .findings
            .iter()
            .map(|f| {
                json!({
                    "code": f.code,
                    "title": english(f.code),
                    "footprint": f.name,
                    "detail": f.detail,
                })
            })
            .collect();
        json!({
            "format": "phoenix-ipc-check/1",
            "checked": self.checked,
            "findings": findings.len(),
            "list": findings,
        })
    }

    /// The report as text in a language.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = format!(
            "{} footprints checked, {} findings\n",
            self.checked,
            self.findings.len()
        );
        for f in &self.findings {
            out.push_str(&format!(
                "  {}: {}: {}: {}\n",
                f.code,
                title(f.code, lang),
                f.name,
                f.detail
            ));
        }
        out
    }
}

/// The distance from a point to a box, 0 inside it.
fn point_rect(px: f64, py: f64, r: Rect) -> f64 {
    let dx = (r.0 as f64 - px).max(px - r.2 as f64).max(0.0);
    let dy = (r.1 as f64 - py).max(py - r.3 as f64).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

/// The distance from a point to a segment.
fn point_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let len2 = abx * abx + aby * aby;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / len2).clamp(0.0, 1.0)
    };
    let (qx, qy) = (a.0 + t * abx, a.1 + t * aby);
    ((p.0 - qx).powi(2) + (p.1 - qy).powi(2)).sqrt()
}

fn side(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// True when two segments cross each other.
fn crosses(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
}

/// The distance from a segment to a box, 0 when they touch.
fn segment_rect(a: (f64, f64), b: (f64, f64), r: Rect) -> f64 {
    let mut best = point_rect(a.0, a.1, r).min(point_rect(b.0, b.1, r));
    if best == 0.0 {
        return 0.0;
    }
    let (x1, y1, x2, y2) = (r.0 as f64, r.1 as f64, r.2 as f64, r.3 as f64);
    let corners = [(x1, y1), (x2, y1), (x2, y2), (x1, y2)];
    for (i, c0) in corners.iter().enumerate() {
        let c1 = corners[(i + 1) % 4];
        if crosses(a, b, *c0, c1) {
            return 0.0;
        }
        best = best.min(point_segment(*c0, a, b));
    }
    best
}

fn overlap(a: Rect, b: Rect) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

fn mm(value: i64) -> String {
    eh3d_phoenix_libedit::write::fmt_mm(value)
}

/// The findings about one footprint, without the ones that need the whole library (the names).
pub fn check_footprint(fp: &IpcFootprint, set: &FamilySet, settings: &IpcSettings) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    let mut add = |code: &'static str, detail: String| {
        out.push(Finding {
            code,
            name: fp.name.clone(),
            detail,
        });
    };

    // 010: pads with a size and holes smaller than the pads
    for p in &fp.pads {
        if p.w <= 0 || p.h <= 0 {
            add(PAD_SIZE, format!("pad {} has no size", p.name));
        } else if p.drill > 0 && p.drill >= p.w.min(p.h) {
            add(
                PAD_SIZE,
                format!(
                    "the hole of pad {} ({} mm) is not smaller than the pad",
                    p.name,
                    mm(p.drill)
                ),
            );
        }
    }

    // 003: pads that overlap
    for (i, a) in fp.pads.iter().enumerate() {
        for b in fp.pads.iter().skip(i + 1) {
            if overlap(a.rect(), b.rect()) {
                add(PADS_OVERLAP, format!("pads {} and {}", a.name, b.name));
            }
        }
    }

    // 004: the smallest distance between pad centres against the pitch
    if fp.pitch > 0 && fp.pads.len() >= 2 {
        let mut least = f64::MAX;
        for (i, a) in fp.pads.iter().enumerate() {
            for b in fp.pads.iter().skip(i + 1) {
                let d = (((a.x - b.x) as f64).powi(2) + ((a.y - b.y) as f64).powi(2)).sqrt();
                least = least.min(d);
            }
        }
        let tol = settings.pitch_tol_nm as f64;
        let pitch = fp.pitch as f64;
        let wrong = if fp.pitch_exact {
            (least - pitch).abs() > tol
        } else {
            least < pitch - tol
        };
        if wrong {
            add(
                PITCH_WRONG,
                format!(
                    "the pads are {} mm apart at the closest, the pitch is {} mm",
                    mm(least.round() as i64),
                    mm(fp.pitch)
                ),
            );
        }
    }

    // 005: the origin at the centre of the box of the pads
    if let Some(b) = pads_bounds(&fp.pads) {
        let (cx, cy) = (b.0 + b.2, b.1 + b.3);
        if cx.abs() > 2 * settings.origin_tol_nm || cy.abs() > 2 * settings.origin_tol_nm {
            add(
                ORIGIN_OFF,
                format!(
                    "the middle of the pads is at {} mm, {} mm",
                    mm(cx / 2),
                    mm(cy / 2)
                ),
            );
        }
    }

    // 006: the mask and the paste
    for p in &fp.pads {
        let bad = if p.is_smd() {
            !p.mask
                || p.paste == 0
                || p.paste > 100
                || (p.paste < 100 && p.paste > settings.ep_paste_max)
        } else {
            !p.mask || p.paste != 0
        };
        if bad {
            add(
                MASK_PASTE,
                format!(
                    "pad {}: mask {}, paste {} per cent",
                    p.name,
                    if p.mask { "open" } else { "closed" },
                    p.paste
                ),
            );
        }
    }

    // 007: the silkscreen away from the pads
    let least = settings.silk_clearance_nm as f64;
    'silk: for l in &fp.silk {
        for p in &fp.pads {
            let d = segment_rect(
                (l.x1 as f64, l.y1 as f64),
                (l.x2 as f64, l.y2 as f64),
                p.rect(),
            ) - l.width as f64 / 2.0;
            if d < least - 1.0 {
                add(
                    SILK_ON_PAD,
                    format!("a line is {} mm from pad {}", mm(d.round() as i64), p.name),
                );
                continue 'silk;
            }
        }
    }
    for dot in &fp.dots {
        for p in &fp.pads {
            let d = point_rect(dot.x as f64, dot.y as f64, p.rect()) - dot.r as f64;
            if d < least - 1.0 {
                add(
                    SILK_ON_PAD,
                    format!("a dot is {} mm from pad {}", mm(d.round() as i64), p.name),
                );
                break;
            }
        }
    }

    // 008: the courtyard of the density level
    let wanted = set
        .get(&fp.family)
        .ok()
        .and_then(|f| f.variant(&fp.variant).ok().map(|v| f.goals(v, fp.density)))
        .map(|g| (g.courtyard * 1_000_000.0).round() as i64);
    if let Some(need) = wanted {
        if fp.court_excess + settings.court_tol_nm < need {
            add(
                COURTYARD,
                format!(
                    "the excess is {} mm and the density {} asks for {} mm",
                    mm(fp.court_excess),
                    fp.density.key(),
                    mm(need)
                ),
            );
        }
    }
    if let Some(req) = fp.required_courtyard() {
        let t = settings.court_tol_nm;
        let c = fp.court;
        if (c.0 - req.0).abs() > t
            || (c.1 - req.1).abs() > t
            || (c.2 - req.2).abs() > t
            || (c.3 - req.3).abs() > t
        {
            add(
                COURTYARD,
                "the courtyard is not the box of the pads and the body grown by the excess"
                    .to_owned(),
            );
        }
    }

    // 009: the name
    let valid = !fp.name.is_empty()
        && fp
            .name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-')
        && fp.name.starts_with(&fp.prefix)
        && fp.name.ends_with(fp.density.letter());
    if !valid {
        add(
            NAME_NOT_IPC,
            format!(
                "the name must be upper case, start with {} and end with {}",
                fp.prefix,
                fp.density.letter()
            ),
        );
    }
    out
}

/// Checks a library: every footprint, and that no two have the same name.
pub fn check_library(
    footprints: &[IpcFootprint],
    set: &FamilySet,
    settings: &IpcSettings,
) -> CheckReport {
    let mut report = CheckReport {
        checked: footprints.len(),
        findings: Vec::new(),
    };
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for fp in footprints {
        let count = seen.entry(fp.name.to_ascii_uppercase()).or_insert(0);
        *count += 1;
        if *count == 2 {
            report.findings.push(Finding {
                code: NAME_DUPLICATE,
                name: fp.name.clone(),
                detail: "the name is used by more than one footprint".to_owned(),
            });
        }
        report.findings.extend(check_footprint(fp, set, settings));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::family::Density;
    use crate::generate::{generate, library, Params};

    fn set() -> FamilySet {
        FamilySet::builtin().unwrap()
    }

    fn one(family: &str, variant: &str, pins: Option<&str>) -> IpcFootprint {
        let mut params = Params::new();
        if let Some(p) = pins {
            params.insert("pins".to_owned(), p.to_owned());
        }
        generate(&set(), family, variant, Density::B, &params).unwrap()
    }

    fn codes(fp: &IpcFootprint) -> Vec<&'static str> {
        check_footprint(fp, &set(), &IpcSettings::default())
            .iter()
            .map(|f| f.code)
            .collect()
    }

    #[test]
    fn the_whole_library_passes_its_own_check() {
        let s = set();
        let fps = library(&s).unwrap();
        let report = check_library(&fps, &s, &IpcSettings::default());
        assert_eq!(report.checked, fps.len());
        assert!(report.is_clean(), "{}", report.to_text(Lang::En));
    }

    #[test]
    fn a_generated_footprint_is_clean() {
        let fp = one("soic", "narrow", Some("14"));
        assert!(codes(&fp).is_empty());
    }

    #[test]
    fn overlapping_pads_are_found() {
        let mut fp = one("chip", "0603", None);
        fp.pads[1].x = fp.pads[0].x + 100_000;
        let found = codes(&fp);
        assert!(found.contains(&PADS_OVERLAP), "{found:?}");
        assert!(found.contains(&ORIGIN_OFF), "{found:?}");
    }

    #[test]
    fn a_wrong_pitch_a_pad_without_size_and_a_big_hole_are_found() {
        let mut fp = one("soic", "narrow", Some("8"));
        fp.pitch = 1_000_000;
        assert!(codes(&fp).contains(&PITCH_WRONG));
        let mut fp = one("soic", "narrow", Some("8"));
        fp.pads[0].w = 0;
        assert!(codes(&fp).contains(&PAD_SIZE));
        let mut fp = one("header", "p2_54", Some("3"));
        fp.pads[1].drill = 2_000_000;
        assert!(codes(&fp).contains(&PAD_SIZE));
    }

    #[test]
    fn a_closed_mask_and_paste_on_a_through_hole_pad_are_found() {
        let mut fp = one("chip", "0603", None);
        fp.pads[0].mask = false;
        assert!(codes(&fp).contains(&MASK_PASTE));
        let mut fp = one("header", "p2_54", Some("3"));
        fp.pads[0].paste = 100;
        assert!(codes(&fp).contains(&MASK_PASTE));
        let mut fp = one("qfn", "qfn16_3x3_p0_5", None);
        fp.pads[16].paste = 95;
        assert!(codes(&fp).contains(&MASK_PASTE));
    }

    #[test]
    fn silkscreen_on_a_pad_is_found() {
        let mut fp = one("chip", "0603", None);
        let line = crate::fp::IpcLine {
            x1: -1_000_000,
            y1: 0,
            x2: 1_000_000,
            y2: 0,
            width: 150_000,
        };
        fp.silk.push(line);
        assert!(codes(&fp).contains(&SILK_ON_PAD));
        let mut fp = one("soic", "narrow", Some("8"));
        fp.dots[0].x = fp.pads[0].x;
        assert!(codes(&fp).contains(&SILK_ON_PAD));
    }

    #[test]
    fn a_courtyard_that_is_too_small_is_found() {
        let mut fp = one("chip", "0603", None);
        fp.court = (fp.court.0 + 100_000, fp.court.1, fp.court.2, fp.court.3);
        assert!(codes(&fp).contains(&COURTYARD));
        let mut fp = one("chip", "0603", None);
        fp.court_excess = 100_000;
        fp.court = fp.required_courtyard().unwrap();
        assert!(codes(&fp).contains(&COURTYARD));
    }

    #[test]
    fn a_name_that_does_not_follow_the_naming_is_found() {
        let mut fp = one("chip", "0603", None);
        fp.name = "resc 1608".to_owned();
        assert!(codes(&fp).contains(&NAME_NOT_IPC));
        let mut fp = one("chip", "0603", None);
        fp.name = "RESC1608X55L".to_owned();
        assert!(codes(&fp).contains(&NAME_NOT_IPC));
    }

    #[test]
    fn two_footprints_with_one_name_are_reported_once() {
        let s = set();
        let a = one("chip", "0603", None);
        let b = a.clone();
        let c = a.clone();
        let report = check_library(&[a, b, c], &s, &IpcSettings::default());
        let dups = report
            .findings
            .iter()
            .filter(|f| f.code == NAME_DUPLICATE)
            .count();
        assert_eq!(dups, 1);
        assert_eq!(report.about("RESC1608X55N").len(), 1);
    }

    #[test]
    fn the_report_is_text_in_a_language_and_data() {
        let s = set();
        let mut fp = one("chip", "0603", None);
        fp.pads[0].w = 0;
        let report = check_library(&[fp], &s, &IpcSettings::default());
        assert!(report
            .to_text(Lang::Pt)
            .contains("PHOENIX-IPC-010: Pad sem tamanho"));
        let json = report.to_json();
        assert_eq!(json["format"], "phoenix-ipc-check/1");
        assert_eq!(json["checked"], 1);
        assert_eq!(json["list"][0]["code"], "PHOENIX-IPC-010");
    }

    #[test]
    fn the_distances_to_a_box_are_those_of_the_geometry() {
        let r: Rect = (0, 0, 10, 10);
        assert_eq!(point_rect(5.0, 5.0, r), 0.0);
        assert_eq!(point_rect(13.0, 14.0, r), 5.0);
        assert_eq!(segment_rect((-5.0, 5.0), (15.0, 5.0), r), 0.0);
        assert_eq!(segment_rect((-5.0, 12.0), (15.0, 12.0), r), 2.0);
        assert_eq!(segment_rect((12.0, 0.0), (12.0, 10.0), r), 2.0);
        assert!(overlap((0, 0, 4, 4), (3, 3, 6, 6)));
        assert!(!overlap((0, 0, 4, 4), (4, 0, 6, 4)));
    }
}
