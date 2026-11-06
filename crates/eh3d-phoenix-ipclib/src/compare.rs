// SPDX-License-Identifier: GPL-3.0-or-later
//! The comparison of a footprint of the owner with the one the library generates: pad by pad, with
//! the differences of position, of size and of hole, and the pads that only one of them has. It
//! is a report; nothing is replaced. The generated footprint is also tried turned by quarter
//! turns, because the owner may have drawn the same package the other way round.

use std::path::Path;

use eh3d_phoenix_kicad::{read_footprint, sequential_ids, KicadSettings};
use eh3d_phoenix_libedit::read::read_library;
use eh3d_phoenix_libedit::LibSettings;
use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::codes::{
    english, title, IpcError, COMPARE_DIFFERS, COMPARE_EXTRA, COMPARE_MISSING, FILE_UNUSABLE,
    PARAM_INVALID,
};
use crate::pads::{from_kicad, from_package, PadGeom};

/// What differs for a pad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadDiff {
    /// `PHOENIX-IPC-040`, `041` or `042`.
    pub code: &'static str,
    /// The name of the pad.
    pub name: String,
    /// The owner's pad minus the generated one, in nanometres.
    pub dx: i64,
    /// The owner's pad minus the generated one.
    pub dy: i64,
    /// The owner's pad minus the generated one.
    pub dw: i64,
    /// The owner's pad minus the generated one.
    pub dh: i64,
    /// The owner's hole minus the generated one.
    pub dd: i64,
}

impl PadDiff {
    fn exceeds(&self, tol: i64) -> bool {
        [self.dx, self.dy, self.dw, self.dh, self.dd]
            .iter()
            .any(|d| d.abs() > tol)
    }
}

/// The result of a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareReport {
    /// The name of the footprint of the owner.
    pub owner: String,
    /// The name of the generated footprint.
    pub generated: String,
    /// The turn of the generated footprint that fits best, in degrees.
    pub turn_deg: u32,
    /// The tolerance, in nanometres.
    pub tolerance_nm: i64,
    /// How many pads the owner's footprint has.
    pub owner_pads: usize,
    /// How many pads the generated footprint has.
    pub generated_pads: usize,
    /// How many pads agree within the tolerance.
    pub matching: usize,
    /// The pads that differ or are missing.
    pub differences: Vec<PadDiff>,
}

impl CompareReport {
    /// True when every pad agrees and there are no pads left over.
    pub fn is_same(&self) -> bool {
        self.differences.is_empty()
    }

    /// The report as data.
    pub fn to_json(&self) -> Value {
        let list: Vec<Value> = self
            .differences
            .iter()
            .map(|d| {
                json!({
                    "code": d.code,
                    "title": english(d.code),
                    "pad": d.name,
                    "dx": d.dx,
                    "dy": d.dy,
                    "dw": d.dw,
                    "dh": d.dh,
                    "ddrill": d.dd,
                })
            })
            .collect();
        json!({
            "format": "phoenix-ipc-compare/1",
            "unit": "nm",
            "owner": self.owner,
            "generated": self.generated,
            "turn": self.turn_deg,
            "tolerance": self.tolerance_nm,
            "owner_pads": self.owner_pads,
            "generated_pads": self.generated_pads,
            "matching": self.matching,
            "differences": list,
        })
    }

    /// The report as text in a language.
    pub fn to_text(&self, lang: Lang) -> String {
        let mm = eh3d_phoenix_libedit::write::fmt_mm;
        let mut out = format!(
            "{} against {} (turned {} degrees): {} of {} pads agree within {} mm, {} differences\n",
            self.owner,
            self.generated,
            self.turn_deg,
            self.matching,
            self.generated_pads,
            mm(self.tolerance_nm),
            self.differences.len()
        );
        for d in &self.differences {
            out.push_str(&format!(
                "  {}: {}: pad {}: dx {} dy {} dw {} dh {} ddrill {} (mm)\n",
                d.code,
                title(d.code, lang),
                d.name,
                mm(d.dx),
                mm(d.dy),
                mm(d.dw),
                mm(d.dh),
                mm(d.dd)
            ));
        }
        out
    }
}

/// The pads turned by quarter turns counter-clockwise around the origin.
pub fn turned(pads: &[PadGeom], quarters: u32) -> Vec<PadGeom> {
    pads.iter()
        .map(|p| {
            let (x, y, w, h) = match quarters % 4 {
                0 => (p.x, p.y, p.w, p.h),
                1 => (-p.y, p.x, p.h, p.w),
                2 => (-p.x, -p.y, p.w, p.h),
                _ => (p.y, -p.x, p.h, p.w),
            };
            PadGeom {
                name: p.name.clone(),
                x,
                y,
                w,
                h,
                drill: p.drill,
                copper: p.copper,
            }
        })
        .collect()
}

fn missing(q: &PadGeom) -> PadDiff {
    PadDiff {
        code: COMPARE_MISSING,
        name: q.name.clone(),
        dx: 0,
        dy: 0,
        dw: 0,
        dh: 0,
        dd: 0,
    }
}

fn extra(o: &PadGeom) -> PadDiff {
    PadDiff {
        code: COMPARE_EXTRA,
        name: o.name.clone(),
        dx: 0,
        dy: 0,
        dw: 0,
        dh: 0,
        dd: 0,
    }
}

/// Pairs the pads by name, taking for each generated pad the nearest pad of the owner that has
/// the same name and is not taken; returns the differences and how many pads agree.
fn align(owner: &[PadGeom], generated: &[PadGeom], tol: i64) -> (Vec<PadDiff>, usize) {
    let mut used = vec![false; owner.len()];
    let mut diffs = Vec::new();
    let mut matching = 0;
    for q in generated {
        let mut best: Option<usize> = None;
        let mut best_d = i64::MAX;
        for (i, o) in owner.iter().enumerate() {
            if used[i] || o.name != q.name {
                continue;
            }
            let d = (o.x - q.x).abs() + (o.y - q.y).abs();
            if d < best_d {
                best_d = d;
                best = Some(i);
            }
        }
        match best {
            None => diffs.push(missing(q)),
            Some(i) => {
                used[i] = true;
                let o = &owner[i];
                let d = PadDiff {
                    code: COMPARE_DIFFERS,
                    name: q.name.clone(),
                    dx: o.x - q.x,
                    dy: o.y - q.y,
                    dw: o.w - q.w,
                    dh: o.h - q.h,
                    dd: o.drill - q.drill,
                };
                if d.exceeds(tol) {
                    diffs.push(d);
                } else {
                    matching += 1;
                }
            }
        }
    }
    for (i, o) in owner.iter().enumerate() {
        if !used[i] {
            diffs.push(extra(o));
        }
    }
    (diffs, matching)
}

/// Compares the pads of a footprint of the owner with the generated ones, trying the four turns
/// of the generated footprint and keeping the one where most pads agree (the first on a tie).
pub fn compare_pads(
    owner_name: &str,
    owner: &[PadGeom],
    generated_name: &str,
    generated: &[PadGeom],
    tol: i64,
) -> CompareReport {
    let mut best: Option<(u32, Vec<PadDiff>, usize)> = None;
    for quarters in 0..4_u32 {
        let (diffs, matching) = align(owner, &turned(generated, quarters), tol);
        let better = match &best {
            None => true,
            Some((_, old, old_matching)) => {
                matching > *old_matching || (matching == *old_matching && diffs.len() < old.len())
            }
        };
        if better {
            best = Some((quarters, diffs, matching));
        }
    }
    let (quarters, differences, matching) = best.unwrap_or((0, Vec::new(), 0));
    CompareReport {
        owner: owner_name.to_owned(),
        generated: generated_name.to_owned(),
        turn_deg: quarters * 90,
        tolerance_nm: tol,
        owner_pads: owner.len(),
        generated_pads: generated.len(),
        matching,
        differences,
    }
}

/// Reads one footprint of the owner from an EAGLE library (`name` is needed) or from a
/// `.kicad_mod` file, as its name and its pads.
pub fn load_footprint(
    path: &Path,
    name: Option<&str>,
    max_bytes: u64,
) -> Result<(String, Vec<PadGeom>), IpcError> {
    let unusable =
        |what: String| IpcError::new(FILE_UNUSABLE, format!("{}: {what}", path.display()));
    let size = std::fs::metadata(path)
        .map_err(|e| unusable(e.to_string()))?
        .len();
    if size > max_bytes {
        return Err(unusable(format!("larger than {max_bytes} bytes")));
    }
    let bytes = std::fs::read(path).map_err(|e| unusable(e.to_string()))?;
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "lbr" => {
            let lib = read_library(&bytes, &LibSettings::default().limits())
                .map_err(|e| unusable(e.to_string()))?;
            let Some(wanted) = name else {
                return Err(IpcError::new(
                    PARAM_INVALID,
                    "the name of the footprint is needed for a library",
                ));
            };
            let Some(package) = lib.package(wanted) else {
                return Err(unusable(format!(
                    "no footprint {wanted}; the library has {}",
                    lib.names(eh3d_phoenix_libedit::Kind::Package).join(", ")
                )));
            };
            Ok((package.name.clone(), from_package(package)))
        }
        "kicad_mod" => {
            let mut ids = sequential_ids().map_err(|e| unusable(e.to_string()))?;
            let read = read_footprint(&bytes, &KicadSettings::default(), &mut ids)
                .map_err(|d| unusable(d.message))?;
            Ok((read.footprint.name.clone(), from_kicad(&read.footprint)))
        }
        _ => Err(unusable(
            "only .lbr libraries and .kicad_mod files are compared".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(name: &str, x: i64, y: i64, w: i64, h: i64) -> PadGeom {
        PadGeom {
            name: name.to_owned(),
            x,
            y,
            w,
            h,
            drill: 0,
            copper: true,
        }
    }

    fn pair() -> Vec<PadGeom> {
        vec![
            pad("1", -800_000, 0, 800_000, 900_000),
            pad("2", 800_000, 0, 800_000, 900_000),
        ]
    }

    #[test]
    fn the_same_pads_agree() {
        let report = compare_pads("A", &pair(), "B", &pair(), 50_000);
        assert!(report.is_same());
        assert_eq!((report.matching, report.turn_deg), (2, 0));
        assert_eq!((report.owner_pads, report.generated_pads), (2, 2));
        assert!(report.to_text(Lang::En).contains("2 of 2 pads agree"));
    }

    #[test]
    fn a_pad_that_moved_or_grew_beyond_the_tolerance_is_reported_with_the_difference() {
        let mut owner = pair();
        owner[1].x += 200_000;
        owner[0].w += 30_000;
        let report = compare_pads("A", &owner, "B", &pair(), 50_000);
        assert_eq!(report.matching, 1);
        assert_eq!(report.differences.len(), 1);
        let d = &report.differences[0];
        assert_eq!(
            (d.code, d.name.as_str(), d.dx),
            (COMPARE_DIFFERS, "2", 200_000)
        );
        assert!(report
            .to_text(Lang::Es)
            .contains("PHOENIX-IPC-040: El pad difiere"));
        assert_eq!(report.to_json()["differences"][0]["dx"], 200_000);
    }

    #[test]
    fn pads_that_only_one_footprint_has_are_reported() {
        let mut owner = pair();
        owner.remove(1);
        owner.push(pad("9", 0, 2_000_000, 500_000, 500_000));
        let report = compare_pads("A", &owner, "B", &pair(), 50_000);
        let codes: Vec<(&str, &str)> = report
            .differences
            .iter()
            .map(|d| (d.code, d.name.as_str()))
            .collect();
        assert_eq!(codes, [(COMPARE_MISSING, "2"), (COMPARE_EXTRA, "9")]);
        assert_eq!(report.matching, 1);
    }

    #[test]
    fn a_package_drawn_the_other_way_round_is_matched_by_a_turn() {
        let owner = [
            pad("1", 0, 800_000, 900_000, 800_000),
            pad("2", 0, -800_000, 900_000, 800_000),
        ];
        let report = compare_pads("A", &owner, "B", &pair(), 50_000);
        assert!(report.is_same());
        assert_eq!(report.turn_deg, 270);
    }

    #[test]
    fn a_footprint_is_read_from_a_kicad_file_and_a_library_needs_its_name() {
        let dir = tempfile::tempdir().unwrap();
        let set = crate::family::FamilySet::builtin().unwrap();
        let fp = crate::generate::generate(
            &set,
            "chip",
            "0603",
            crate::family::Density::B,
            &crate::generate::Params::new(),
        )
        .unwrap();
        let mod_path = dir.path().join("a.kicad_mod");
        crate::fp::save_kicad_mod(&fp, &mod_path).unwrap();
        let (name, pads) = load_footprint(&mod_path, None, 1 << 20).unwrap();
        assert_eq!(name, fp.name);
        let report = compare_pads(&name, &pads, &fp.name, &crate::pads::from_ipc(&fp), 5_000);
        assert!(report.is_same(), "{}", report.to_text(Lang::En));
        let lbr_path = dir.path().join("a.lbr");
        crate::fp::save_lbr(&[fp], &lbr_path).unwrap();
        let e = load_footprint(&lbr_path, None, 1 << 20).unwrap_err();
        assert_eq!(e.code, PARAM_INVALID);
        let (name, pads) = load_footprint(&lbr_path, Some("RESC1608X55N"), 1 << 20).unwrap();
        assert_eq!((name.as_str(), pads.len()), ("RESC1608X55N", 2));
        let e = load_footprint(&lbr_path, Some("nothing"), 1 << 20).unwrap_err();
        assert_eq!(e.code, FILE_UNUSABLE);
        let other = dir.path().join("a.txt");
        std::fs::write(&other, "x").unwrap();
        assert_eq!(
            load_footprint(&other, None, 1 << 20).unwrap_err().code,
            FILE_UNUSABLE
        );
    }
}
