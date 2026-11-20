// SPDX-License-Identifier: GPL-3.0-or-later
//! A short library of laminates for a quick first estimate. The figures are not kept here: every
//! entry is a record of the materials database of the suite (`eh3d-materials`, in
//! EH3D-COMMON), which is the only source of them. The file `config/laminate_links.json` says
//! which record each short identifier is and at which frequency the dielectric constant and the
//! loss tangent are read; the rest (glass transition, decomposition, expansion, thermal
//! conductivity, thicknesses) is copied from the record.
//!
//! **Nothing here is verified.** Every entry has `verified: false`, as the figures of the
//! database are the ones the data sheets state, not measurements, and a data sheet states them
//! for a stated frequency, resin content and test method that a real board does not share. A
//! figure that the record does not give is left undefined (`None`), never guessed; the figure of a
//! frequency the record has no point for is `None` as well. Use the library to get a first
//! estimate and replace the figures with the ones of the laminate the fabricator quotes.
//!
//! Frequencies are in GHz, temperatures in degrees Celsius, expansion in ppm per degree Celsius
//! and thicknesses in micrometres. The glass transition is the one the database orders materials
//! by (DSC, else DMA, else TMA); the database does not keep a lower bound ("over 280"), so
//! `tg_is_lower_bound` is false for every entry.
//!
//! This module is compiled only with the feature `materials`.

use std::sync::OnceLock;

use eh3d_materials::model::{DkPoint, Kind, Material};
use eh3d_materials::Database;
use serde::Deserialize;

use crate::material::MaterialSample;

/// The data file that says which record of the database each short identifier is.
pub const LINKS_TEXT: &str = include_str!("../../../config/laminate_links.json");

/// The format the file must declare.
pub const LINKS_FORMAT: &str = "phoenix-laminate-links/1";

/// The kind of a laminate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Glass-reinforced epoxy (FR-4 and its variants).
    Epoxy,
    /// Hydrocarbon or PTFE laminates for high frequencies.
    HighFrequency,
    /// Polyimide laminates and films, rigid or flexible.
    Polyimide,
    /// A dielectric on a metal base (insulated metal substrate).
    MetalBase,
    /// A conductor, listed for its conductivity.
    Conductor,
}

/// The figures a data sheet states for one laminate. `None` is a figure the record does not
/// give.
#[derive(Debug, Clone, PartialEq)]
pub struct Laminate {
    /// The identifier used by the library: lowercase, digits and hyphens.
    pub id: String,
    /// The identifier of the record in the materials database (empty for a conductor).
    pub common_id: String,
    /// The maker.
    pub maker: String,
    /// The name of the product as the sheet writes it.
    pub name: String,
    /// The kind of laminate.
    pub family: Family,
    /// The dielectric constant.
    pub dk: Option<f64>,
    /// The frequency the dielectric constant is stated at, in GHz.
    pub dk_ghz: Option<f64>,
    /// The loss tangent.
    pub df: Option<f64>,
    /// The frequency the loss tangent is stated at, in GHz.
    pub df_ghz: Option<f64>,
    /// The glass transition temperature.
    pub tg_c: Option<f64>,
    /// Whether `tg_c` is a lower bound. The database does not say it, so it is false.
    pub tg_is_lower_bound: bool,
    /// The decomposition temperature.
    pub td_c: Option<f64>,
    /// The expansion in the plane (X), below the glass transition.
    pub cte_x_ppm: Option<f64>,
    /// The expansion in the plane (Y), below the glass transition.
    pub cte_y_ppm: Option<f64>,
    /// The expansion through the thickness (Z), below the glass transition.
    pub cte_z_ppm: Option<f64>,
    /// The expansion through the thickness (Z), above the glass transition.
    pub cte_z_above_tg_ppm: Option<f64>,
    /// The thermal conductivity in watts per metre and kelvin.
    pub thermal_w_per_mk: Option<f64>,
    /// The conductivity in siemens per metre (a conductor).
    pub conductivity_s_per_m: Option<f64>,
    /// The commercial thicknesses of a core, in micrometres, from the thinnest.
    pub core_thicknesses_um: Vec<u32>,
    /// Whether the figures were checked against a second source. Always false here.
    pub verified: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConductor {
    name: String,
    conductivity_s_per_m: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLink {
    id: String,
    #[serde(default)]
    common_id: Option<String>,
    #[serde(default)]
    dk_ghz: Option<f64>,
    #[serde(default)]
    df_ghz: Option<f64>,
    #[serde(default)]
    conductor: Option<RawConductor>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLinks {
    format: String,
    #[serde(default)]
    note: String,
    entries: Vec<RawLink>,
}

/// The value of the point of a list at a frequency, when the list has a point there.
fn point_at(list: &[DkPoint], ghz: Option<f64>) -> Option<(f64, f64)> {
    let ghz = ghz?;
    list.iter()
        .find(|p| p.freq_ghz == Some(ghz))
        .map(|p| (ghz, p.value))
}

fn family_of(kind: Kind) -> Family {
    match kind {
        Kind::FlexLaminate | Kind::Coverlay => Family::Polyimide,
        Kind::MetalCore => Family::MetalBase,
        Kind::Ptfe | Kind::CeramicFilled => Family::HighFrequency,
        Kind::CopperFoil => Family::Conductor,
        Kind::Laminate | Kind::Prepreg | Kind::BondingFilm | Kind::Other => Family::Epoxy,
    }
}

/// The laminate of a record of the database, read at the frequencies the link states.
fn from_record(id: &str, record: &Material, dk_ghz: Option<f64>, df_ghz: Option<f64>) -> Laminate {
    let dk = point_at(&record.dk, dk_ghz);
    let df = point_at(&record.df, df_ghz);
    Laminate {
        id: id.to_owned(),
        common_id: record.id.clone(),
        maker: record.manufacturer.clone(),
        name: record.name.clone(),
        family: family_of(record.kind),
        dk: dk.map(|p| p.1),
        dk_ghz: dk.map(|p| p.0),
        df: df.map(|p| p.1),
        df_ghz: df.map(|p| p.0),
        tg_c: record.tg_c(),
        tg_is_lower_bound: false,
        td_c: record.td_c,
        cte_x_ppm: record.cte_x_ppm,
        cte_y_ppm: record.cte_y_ppm,
        cte_z_ppm: record.cte_z_below_tg_ppm,
        cte_z_above_tg_ppm: record.cte_z_above_tg_ppm,
        thermal_w_per_mk: record.thermal_conductivity_w_mk,
        conductivity_s_per_m: None,
        core_thicknesses_um: record
            .thicknesses_mm
            .iter()
            .filter(|mm| mm.is_finite() && **mm > 0.0)
            .map(|mm| (mm * 1000.0).round() as u32)
            .collect(),
        verified: false,
    }
}

/// Build the library from a text of links and a database. A bad file, or a link to a record the
/// database does not have, is an error that says what is wrong.
pub fn build(links_text: &str, database: &Database) -> Result<Vec<Laminate>, String> {
    let raw: RawLinks =
        serde_json::from_str(links_text).map_err(|e| format!("laminate links: {e}"))?;
    if raw.format != LINKS_FORMAT {
        return Err(format!(
            "laminate links: the format is {:?}, not {LINKS_FORMAT:?}",
            raw.format
        ));
    }
    let _ = raw.note;
    let mut out = Vec::new();
    for link in raw.entries {
        if let Some(c) = link.conductor {
            out.push(Laminate {
                id: link.id,
                common_id: String::new(),
                maker: String::new(),
                name: c.name,
                family: Family::Conductor,
                dk: None,
                dk_ghz: None,
                df: None,
                df_ghz: None,
                tg_c: None,
                tg_is_lower_bound: false,
                td_c: None,
                cte_x_ppm: None,
                cte_y_ppm: None,
                cte_z_ppm: None,
                cte_z_above_tg_ppm: None,
                thermal_w_per_mk: None,
                conductivity_s_per_m: Some(c.conductivity_s_per_m),
                core_thicknesses_um: Vec::new(),
                verified: false,
            });
            continue;
        }
        let common = link
            .common_id
            .ok_or_else(|| format!("laminate links: {} has no record", link.id))?;
        let record = database
            .get(&common)
            .ok_or_else(|| format!("laminate links: {} is not in the database", common))?;
        out.push(from_record(&link.id, record, link.dk_ghz, link.df_ghz));
    }
    Ok(out)
}

/// The library: the entries of `config/laminate_links.json` read from the embedded database.
/// An empty slice when the database or the file cannot be read (the tests of the module fail
/// then).
pub fn laminates() -> &'static [Laminate] {
    static CELL: OnceLock<Vec<Laminate>> = OnceLock::new();
    CELL.get_or_init(|| {
        Database::embedded()
            .ok()
            .and_then(|db| build(LINKS_TEXT, &db).ok())
            .unwrap_or_default()
    })
}

/// The laminate with an identifier, in any case.
pub fn find(id: &str) -> Option<&'static Laminate> {
    laminates()
        .iter()
        .find(|l| l.id.eq_ignore_ascii_case(id.trim()))
}

impl Laminate {
    /// The material sample of the laminate: the dielectric constant and the loss tangent at the
    /// frequencies the link states them at. `None` when the record has no constant, no loss
    /// tangent or no point at that frequency for one of them, because a sample needs all of them;
    /// a figure is never completed with a guess.
    pub fn sample(&self) -> Option<MaterialSample> {
        let sample = MaterialSample {
            name: format!("{} {}", self.maker, self.name).trim().to_owned(),
            dk_points: vec![(self.dk_ghz?, self.dk?)],
            df_points: vec![(self.df_ghz?, self.df?)],
            thickness_nm: None,
        };
        sample.validate().ok()?;
        Some(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identifiers_are_unique_sorted_and_made_of_lowercase_digits_and_hyphens() {
        let ids: Vec<&str> = laminates().iter().map(|l| l.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted);
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
        for id in ids {
            assert!(
                !id.is_empty()
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "{id}"
            );
        }
    }

    #[test]
    fn nothing_in_the_library_is_verified() {
        assert!(laminates().len() >= 15);
        assert!(laminates().iter().all(|l| !l.verified));
    }

    #[test]
    fn every_figure_is_in_a_plausible_range() {
        for l in laminates() {
            let id = &l.id;
            for (name, value, low, high) in [
                ("dk", l.dk, 1.0, 100.0),
                ("df", l.df, 0.0, 0.1),
                ("tg", l.tg_c, 20.0, 400.0),
                ("td", l.td_c, 100.0, 600.0),
                ("cte_x", l.cte_x_ppm, 0.0, 100.0),
                ("cte_y", l.cte_y_ppm, 0.0, 100.0),
                ("cte_z", l.cte_z_ppm, 0.0, 300.0),
                ("cte_z_above", l.cte_z_above_tg_ppm, 0.0, 600.0),
                ("thermal", l.thermal_w_per_mk, 0.0, 500.0),
                ("dk_ghz", l.dk_ghz, 0.0001, 200.0),
                ("df_ghz", l.df_ghz, 0.0001, 200.0),
            ] {
                if let Some(v) = value {
                    assert!((low..=high).contains(&v), "{id} {name} {v}");
                }
            }
            // a frequency is stated only with its figure
            assert_eq!(l.dk.is_some(), l.dk_ghz.is_some(), "{id}");
            assert_eq!(l.df.is_some(), l.df_ghz.is_some(), "{id}");
            assert!(!l.tg_is_lower_bound, "{id}");
            assert!(
                l.core_thicknesses_um.windows(2).all(|w| w[0] < w[1]),
                "{id}"
            );
        }
    }

    #[test]
    fn every_entry_is_the_record_of_the_database_it_links_to() {
        let db = Database::embedded().unwrap();
        for l in laminates() {
            if l.family == Family::Conductor {
                assert!(l.common_id.is_empty());
                continue;
            }
            let record = db.get(&l.common_id).unwrap();
            assert_eq!(l.maker, record.manufacturer, "{}", l.id);
            assert_eq!(l.td_c, record.td_c, "{}", l.id);
            assert_eq!(l.tg_c, record.tg_c(), "{}", l.id);
            assert_eq!(
                l.thermal_w_per_mk, record.thermal_conductivity_w_mk,
                "{}",
                l.id
            );
            if let (Some(dk), Some(ghz)) = (l.dk, l.dk_ghz) {
                assert!(
                    record
                        .dk
                        .iter()
                        .any(|p| p.value == dk && p.freq_ghz == Some(ghz)),
                    "{}",
                    l.id
                );
            }
        }
    }

    #[test]
    fn a_lookup_finds_the_entry_in_any_case() {
        let r = find(" Rogers-RO4350B ").unwrap();
        assert_eq!(
            (r.dk, r.df, r.td_c),
            (Some(3.48), Some(0.0037), Some(390.0))
        );
        assert_eq!(r.common_id, "rogers/ro4350b");
        assert_eq!(r.core_thicknesses_um.last(), Some(&1524));
        assert!(find("nothing").is_none());
        assert!(find("").is_none());
        let copper = find("copper-foil").unwrap();
        assert_eq!(copper.family, Family::Conductor);
        assert_eq!(copper.conductivity_s_per_m, Some(59_590_000.0));
    }

    #[test]
    fn a_sample_needs_the_constant_the_loss_and_both_frequencies() {
        let s = find("rogers-ro3003").unwrap().sample().unwrap();
        assert_eq!(s.name, "Rogers RO3003");
        assert_eq!(s.dk_points, [(10.0, 3.0)]);
        assert_eq!(s.df_points, [(10.0, 0.0013)]);
        // a conductor has neither constant nor loss
        assert!(find("copper-foil").unwrap().sample().is_none());
        let with_sample = laminates().iter().filter(|l| l.sample().is_some()).count();
        assert!(with_sample >= 14, "{with_sample}");
    }

    #[test]
    fn the_high_frequency_laminates_are_lossier_in_the_order_the_sheets_state() {
        let df = |id: &str| find(id).unwrap().df.unwrap();
        assert!(df("rogers-ro3003") < df("rogers-ro3006"));
        assert!(df("rogers-ro3006") < df("rogers-ro3010"));
        assert!(df("rogers-ro4003c") < df("rogers-ro4350b"));
        assert!(df("rogers-ro4350b") < df("isola-is400"));
    }

    #[test]
    fn a_bad_links_file_is_refused_with_what_is_wrong() {
        let db = Database::embedded().unwrap();
        assert!(build("{", &db).is_err());
        let wrong_format = r#"{"format":"x","entries":[]}"#;
        assert!(build(wrong_format, &db).unwrap_err().contains("format"));
        let unknown = format!(
            r#"{{"format":"{LINKS_FORMAT}","entries":[{{"id":"a","common_id":"nobody/nothing"}}]}}"#
        );
        assert!(build(&unknown, &db).unwrap_err().contains("nobody/nothing"));
        let empty = format!(r#"{{"format":"{LINKS_FORMAT}","entries":[{{"id":"a"}}]}}"#);
        assert!(build(&empty, &db).unwrap_err().contains("no record"));
    }
}
