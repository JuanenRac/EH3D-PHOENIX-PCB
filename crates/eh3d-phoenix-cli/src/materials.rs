// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `phoenix materials` (list and show the materials database of EH3D-COMMON) and the
//! materials that `phoenix stackup --material ID` takes from it.
//!
//! Without the feature `materials` the database is not part of the program: the command and the
//! option answer with an error that says so.

use eh3d_phoenix_stackup::{MaterialSample, StackupError};
use std::io::Write;

#[cfg(feature = "materials")]
mod database {
    use super::*;
    use eh3d_materials::model::{DkPoint, Material};
    use eh3d_materials::summary::representative;
    use eh3d_materials::{interpolate, Criteria, Database, Kind};
    use eh3d_phoenix_stackup::materials_adapter::sample_from_database;
    use serde_json::{json, Value};
    use std::collections::HashSet;

    const USAGE: &str = "\
phoenix materials list [--manufacturer NAME] [--kind KIND] [--min-tg C] [--min-dk X] [--max-dk X]
                       [--min-df X] [--max-df X] [--at-ghz F] [--phoenix] [--json]
phoenix materials show <id> [--at-ghz F] [--json]

  materials   the board materials database of the suite (laminates, prepregs, flex, metal core,
              PTFE and ceramic filled boards), every value as its datasheet publishes it;
              list filters by manufacturer, kind (laminate, prepreg, bonding_film, flex_laminate,
              coverlay, metal_core, ptfe, ceramic_filled, copper_foil, other), glass transition,
              Dk and Df (taken at --at-ghz between the published points, else the first listed)
              and, with --phoenix, keeps the materials that serve the design tool; show prints one
              record by its identifier, for example isola/370hr or rogers/ro4350b
";

    /// The options of `materials list`.
    struct ListOptions {
        criteria: Criteria,
        phoenix: bool,
        json: bool,
    }

    fn number(value: Option<&str>) -> Option<f64> {
        value?.parse::<f64>().ok().filter(|v| v.is_finite())
    }

    fn kind_from(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.as_str() == name)
    }

    fn parse_list(args: &[&str]) -> Option<ListOptions> {
        let mut options = ListOptions {
            criteria: Criteria::default(),
            phoenix: false,
            json: false,
        };
        let mut iter = args.iter().copied();
        while let Some(arg) = iter.next() {
            match arg {
                "--json" => options.json = true,
                "--phoenix" => options.phoenix = true,
                "--manufacturer" => options.criteria.manufacturer = Some(iter.next()?.to_string()),
                "--kind" => options.criteria.kind = Some(kind_from(iter.next()?)?),
                "--min-tg" => options.criteria.min_tg_c = Some(number(iter.next())?),
                "--min-dk" => options.criteria.min_dk = Some(number(iter.next())?),
                "--max-dk" => options.criteria.max_dk = Some(number(iter.next())?),
                "--min-df" => options.criteria.min_df = Some(number(iter.next())?),
                "--max-df" => options.criteria.max_df = Some(number(iter.next())?),
                "--at-ghz" => options.criteria.at_ghz = Some(number(iter.next())?),
                _ => return None,
            }
        }
        Some(options)
    }

    /// A value of a list of points and the frequency it is at: the one interpolated at `at_ghz`
    /// when given (nothing when it is outside the published points), else the one nearest 1 GHz.
    fn value_of(list: &[DkPoint], at_ghz: Option<f64>) -> Option<(f64, Option<f64>)> {
        match at_ghz {
            Some(freq) => interpolate("", list, freq).ok().map(|v| (v, Some(freq))),
            None => representative(list).map(|p| (p.value, p.freq_ghz)),
        }
    }

    fn cell(value: Option<f64>, decimals: usize) -> String {
        match value {
            Some(v) => format!("{v:.decimals$}"),
            None => "-".to_string(),
        }
    }

    fn row(id: &str, kind: &str, tg: &str, dk: &str, df: &str) -> String {
        format!("{id:<38} {kind:<15} {tg:>6} {dk:>6} {df:>8}\n")
    }

    fn open(err: &mut dyn Write) -> Option<Database> {
        match Database::embedded() {
            Ok(database) => Some(database),
            Err(error) => {
                let _ = writeln!(err, "error {}: {error}", error.code().code());
                None
            }
        }
    }

    fn list(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
        let Some(options) = parse_list(args) else {
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        };
        let Some(db) = open(err) else {
            return 2;
        };
        let phoenix_ids: Option<HashSet<&str>> = options.phoenix.then(|| {
            db.materials_useful_for_phoenix()
                .into_iter()
                .map(|m| m.id.as_str())
                .collect()
        });
        let found: Vec<&Material> = db
            .search(&options.criteria)
            .into_iter()
            .filter(|m| {
                phoenix_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(m.id.as_str()))
            })
            .collect();
        let at = options.criteria.at_ghz;
        if options.json {
            let rows: Vec<Value> = found
                .iter()
                .map(|m| {
                    let dk = value_of(&m.dk, at);
                    let df = value_of(&m.df, at);
                    json!({
                        "id": m.id,
                        "manufacturer": m.manufacturer,
                        "name": m.name,
                        "kind": m.kind.as_str(),
                        "tg_c": m.tg_c(),
                        "dk": dk.map(|p| p.0),
                        "dk_freq_ghz": dk.and_then(|p| p.1),
                        "df": df.map(|p| p.0),
                        "df_freq_ghz": df.and_then(|p| p.1),
                    })
                })
                .collect();
            let text = serde_json::to_string_pretty(&rows).unwrap_or_default();
            let _ = writeln!(out, "{text}");
            return 0;
        }
        let _ = out.write_all(row("id", "kind", "Tg C", "Dk", "Df").as_bytes());
        for m in &found {
            let line = row(
                &m.id,
                m.kind.as_str(),
                &cell(m.tg_c(), 0),
                &cell(value_of(&m.dk, at).map(|p| p.0), 2),
                &cell(value_of(&m.df, at).map(|p| p.0), 4),
            );
            let _ = out.write_all(line.as_bytes());
        }
        let _ = writeln!(out, "{} materials", found.len());
        0
    }

    fn points_text(list: &[DkPoint]) -> String {
        if list.is_empty() {
            return "-".to_string();
        }
        let parts: Vec<String> = list
            .iter()
            .map(|p| match p.freq_ghz {
                Some(freq) => format!("{} at {freq} GHz", p.value),
                None => format!("{} (no frequency)", p.value),
            })
            .collect();
        parts.join("; ")
    }

    fn numbers_text(values: &[f64]) -> String {
        if values.is_empty() {
            return "-".to_string();
        }
        let parts: Vec<String> = values.iter().map(|v| v.to_string()).collect();
        parts.join(", ")
    }

    fn show(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
        let mut id: Option<&str> = None;
        let mut at: Option<f64> = None;
        let mut json = false;
        let mut iter = args.iter().copied();
        while let Some(arg) = iter.next() {
            match arg {
                "--json" => json = true,
                "--at-ghz" => match number(iter.next()) {
                    Some(freq) => at = Some(freq),
                    None => {
                        let _ = err.write_all(USAGE.as_bytes());
                        return 2;
                    }
                },
                other if !other.starts_with("--") && id.is_none() => id = Some(other),
                _ => {
                    let _ = err.write_all(USAGE.as_bytes());
                    return 2;
                }
            }
        }
        let Some(id) = id else {
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        };
        let Some(db) = open(err) else {
            return 2;
        };
        let Some(m) = db.get(id) else {
            let _ = writeln!(err, "error: unknown material {id}");
            return 2;
        };
        let for_phoenix = db
            .materials_useful_for_phoenix()
            .iter()
            .any(|u| u.id == m.id);
        if json {
            let mut value = serde_json::to_value(m).unwrap_or_default();
            if let (Some(object), Some(freq)) = (value.as_object_mut(), at) {
                object.insert(
                    "at_ghz".to_string(),
                    json!({
                        "freq_ghz": freq,
                        "dk": m.dk_at(freq).ok(),
                        "df": m.df_at(freq).ok(),
                    }),
                );
            }
            let text = serde_json::to_string_pretty(&value).unwrap_or_default();
            let _ = writeln!(out, "{text}");
            return 0;
        }
        let copper: Vec<String> = m
            .copper
            .iter()
            .map(|c| match (c.weight_oz, c.thickness_um) {
                (Some(oz), Some(um)) => format!("{oz} oz ({um} um)"),
                (Some(oz), None) => format!("{oz} oz"),
                (None, Some(um)) => format!("{um} um"),
                (None, None) => "-".to_string(),
            })
            .collect();
        let sources: Vec<&str> = m.all_sources().map(|s| s.pdf.as_str()).collect();
        let mut text = String::new();
        let mut put = |label: &str, value: String| {
            text.push_str(&format!("{label:<22} {value}\n"));
        };
        put("id", m.id.clone());
        put("manufacturer", m.manufacturer.clone());
        put("name", m.name.clone());
        put("kind", m.kind.as_str().to_string());
        put("Tg (DSC)", cell(m.tg_dsc_c, 0));
        put("Tg (DMA)", cell(m.tg_dma_c, 0));
        put("Tg (TMA)", cell(m.tg_tma_c, 0));
        put("Td", cell(m.td_c, 0));
        put("Dk", points_text(&m.dk));
        put("Df", points_text(&m.df));
        put("thicknesses (mm)", numbers_text(&m.thicknesses_mm));
        put(
            "copper",
            if copper.is_empty() {
                "-".to_string()
            } else {
                copper.join(", ")
            },
        );
        put("thermal conductivity", cell(m.thermal_conductivity_w_mk, 2));
        let useful = if for_phoenix { "yes" } else { "no" };
        put("useful for PHOENIX-PCB", useful.to_string());
        put("sources", sources.join(", "));
        if let Some(freq) = at {
            let dk = m.dk_at(freq).ok();
            let df = m.df_at(freq).ok();
            put(
                &format!("at {freq} GHz"),
                format!("Dk {}, Df {}", cell(dk, 3), cell(df, 5)),
            );
        }
        let _ = out.write_all(text.as_bytes());
        0
    }

    pub(super) fn command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
        match args.split_first() {
            Some((&"list", tail)) => list(tail, out, err),
            Some((&"show", tail)) => show(tail, out, err),
            _ => {
                let _ = err.write_all(USAGE.as_bytes());
                2
            }
        }
    }

    pub(super) fn samples(ids: &[&str]) -> Result<Vec<MaterialSample>, StackupError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let database = Database::embedded().map_err(|error| StackupError::InvalidJob {
            reason: error.to_string(),
        })?;
        ids.iter()
            .map(|id| sample_from_database(&database, id))
            .collect()
    }
}

/// `phoenix materials list|show`. Returns the exit code.
#[cfg(feature = "materials")]
pub(crate) fn materials_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    database::command(args, out, err)
}

/// `phoenix materials ...` in a build without the database.
#[cfg(not(feature = "materials"))]
pub(crate) fn materials_command(_args: &[&str], _out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let _ = writeln!(
        err,
        "error: this build has no materials database (the feature `materials` is off)"
    );
    2
}

/// The materials named with `--material`, as samples for the job.
#[cfg(feature = "materials")]
pub(crate) fn samples(ids: &[&str]) -> Result<Vec<MaterialSample>, StackupError> {
    database::samples(ids)
}

/// The materials named with `--material` in a build without the database.
#[cfg(not(feature = "materials"))]
pub(crate) fn samples(ids: &[&str]) -> Result<Vec<MaterialSample>, StackupError> {
    if ids.is_empty() {
        Ok(Vec::new())
    } else {
        Err(StackupError::InvalidJob {
            reason: "this build has no materials database (the feature `materials` is off)"
                .to_string(),
        })
    }
}
