// SPDX-License-Identifier: GPL-3.0-or-later
//! The reading mode of the command `ipc2581`: `phoenix ipc2581 --read FILE` reads an IPC-2581 file
//! (revisions A, B and C, as XML), reports what it holds in the eight languages and can write the
//! stack-up as a page of HTML (`--html`) and as a job file for `phoenix stackup` (`--stackup-out`).
//!
//! Exit codes: 0 done, 2 the arguments, the input or the output are unusable. Nothing is written
//! unless `--html` or `--stackup-out` names a file, and a file that exists is never replaced
//! unless `--overwrite` is given. The file that is read is not trusted: its size, depth, elements
//! and items are limited by the settings (`read_max_bytes`, `read_max_depth`,
//! `read_max_elements`, `read_max_items`). A ZIP archive is read in memory (nothing is written to a
//! folder) with the limits `zip_max_total_bytes`, `zip_max_entries` and `zip_max_ratio`: the entry
//! that reads as an IPC-2581 file is the one reported.

use std::collections::BTreeMap;
use std::io::{Read, Write};

use eh3d_phoenix_ipc2581::codes::{Finding, INPUT_UNREADABLE, READ_LIMIT, READ_STACKUP_INCOMPLETE};
use eh3d_phoenix_ipc2581::zipread::looks_like_zip;
use eh3d_phoenix_ipc2581::{
    read_archive, read_bytes, report_text, stack_summary, Counts, Ipc2581Error, Ipc2581Settings,
    ReadOptions, StackKind, StackSummary, ZipLimits,
};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_stackup::{
    stackup_html, CopperThickness, Language, Layer, MaterialSample, Stackup,
};
use serde::Serialize;

use crate::ipc2581::write_file;

/// The longest name of a layer or a material that the stack-up model accepts, in characters.
const NAME_CHARS: usize = 64;

/// What the command reads from the arguments.
pub(crate) struct ReadRequest<'a> {
    /// The file to read.
    pub path: &'a str,
    /// The page of HTML to write.
    pub html: Option<&'a str>,
    /// The job file for `phoenix stackup` to write.
    pub stackup_out: Option<&'a str>,
    /// Replace the files that exist.
    pub overwrite: bool,
    /// Print JSON.
    pub json: bool,
    /// The language of the messages.
    pub lang: Lang,
}

/// The stack-up of a file in the model of `eh3d-phoenix-stackup`: the layers, the materials with
/// the constants the file gives, and what could not be carried over.
pub(crate) struct Mapped {
    /// The layers from the top to the bottom.
    pub stackup: Stackup,
    /// The materials the layers refer to.
    pub materials: Vec<MaterialSample>,
    /// What is missing or was changed, in English.
    pub notes: Vec<String>,
}

fn cut(text: &str) -> String {
    text.chars().take(NAME_CHARS).collect()
}

/// Makes the stack-up and the materials from a stack-up read from a file. A layer without a
/// thickness is left out and said; a dielectric without a constant has no material sample (so a
/// job that uses it is refused until the constant is written); a loss tangent that is missing is
/// written as zero and said. The file gives a dielectric constant and a loss tangent without a
/// frequency, so each material gets one point at `reference_ghz`, a place to start from that the
/// user corrects in the job file.
pub(crate) fn to_stackup(summary: &StackSummary, reference_ghz: f64) -> Mapped {
    let mut layers = Vec::new();
    let mut materials: Vec<MaterialSample> = Vec::new();
    let mut notes = Vec::new();
    let mut taken: BTreeMap<String, (Option<u64>, Option<u64>)> = BTreeMap::new();
    for row in &summary.rows {
        let Some(thickness_nm) = row.thickness_nm else {
            notes.push(format!(
                "layer {} has no thickness and is left out",
                row.name
            ));
            continue;
        };
        let name = cut(&row.name);
        let base = if row.material.is_empty() {
            "unnamed".to_owned()
        } else {
            cut(&row.material)
        };
        let material = if row.kind == StackKind::Copper {
            String::new()
        } else {
            // two layers with the same material name but other constants are two materials
            let key = (row.dk.map(f64::to_bits), row.df.map(f64::to_bits));
            let mut candidate = base.clone();
            let mut n = 1usize;
            while taken.get(&candidate).is_some_and(|k| *k != key) {
                n += 1;
                let suffix = format!(" #{n}");
                let keep = NAME_CHARS.saturating_sub(suffix.chars().count());
                candidate = format!("{}{suffix}", base.chars().take(keep).collect::<String>());
            }
            if taken.insert(candidate.clone(), key).is_none() {
                match row.dk {
                    Some(dk) => {
                        let df = row.df.unwrap_or_else(|| {
                            notes.push(format!(
                                "material {candidate} has no loss tangent in the file; 0 is written"
                            ));
                            0.0
                        });
                        materials.push(MaterialSample {
                            name: candidate.clone(),
                            dk_points: vec![(reference_ghz, dk)],
                            df_points: vec![(reference_ghz, df)],
                            thickness_nm: Some(thickness_nm),
                        });
                    }
                    None => notes.push(format!(
                        "material {candidate} has no dielectric constant in the file"
                    )),
                }
            }
            candidate
        };
        layers.push(match row.kind {
            StackKind::Copper => Layer::Copper {
                name,
                thickness: CopperThickness::Um(thickness_nm as f64 / 1000.0),
                roughness_rms_nm: 0,
            },
            StackKind::Core => Layer::Core {
                name,
                thickness_nm,
                material,
            },
            StackKind::Prepreg => Layer::Prepreg {
                name,
                thickness_nm,
                material,
            },
            StackKind::SolderMask => Layer::SolderMask {
                name,
                thickness_nm,
                material,
            },
        });
    }
    Mapped {
        stackup: Stackup {
            name: if summary.name.is_empty() {
                "stack-up of the file".to_owned()
            } else {
                cut(&summary.name)
            },
            layers,
        },
        materials,
        notes,
    }
}

#[derive(Serialize)]
struct JobTemplate<'a> {
    stackup: &'a Stackup,
    materials: &'a [MaterialSample],
    lines: Vec<u8>,
}

#[derive(Serialize)]
struct ReadJson<'a> {
    archive_entry: Option<&'a str>,
    revision: &'a str,
    units: Option<&'static str>,
    software: &'a str,
    counts: Counts,
    stackup: Option<&'a StackSummary>,
    findings: &'a [Finding],
}

fn language_of(lang: Lang) -> Language {
    Language::from_tag(lang.tag()).unwrap_or(Language::En)
}

fn read_file(path: &str, limit: usize) -> Result<Vec<u8>, Ipc2581Error> {
    let failed = |e: std::io::Error| Ipc2581Error::new(INPUT_UNREADABLE, format!("{path}: {e}"));
    let file = std::fs::File::open(path).map_err(failed)?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(failed)?;
    if bytes.len() > limit {
        return Err(Ipc2581Error::new(
            READ_LIMIT,
            format!("{path} is larger than the {limit} bytes that read_max_bytes allows"),
        ));
    }
    Ok(bytes)
}

/// Reads the file and prints the report.
pub(crate) fn read_command(
    request: &ReadRequest<'_>,
    settings: &Ipc2581Settings,
    out: &mut dyn Write,
) -> Result<i32, Ipc2581Error> {
    let file_limit = settings.read_max_bytes.max(settings.zip_max_total_bytes);
    let bytes = read_file(request.path, file_limit)?;
    let options = ReadOptions::from(settings);
    let (entry, doc) = if looks_like_zip(&bytes) {
        let (name, doc) = read_archive(&bytes, &options, &ZipLimits::from(settings))?;
        (Some(name), doc)
    } else if bytes.len() > settings.read_max_bytes {
        return Err(Ipc2581Error::new(
            READ_LIMIT,
            format!(
                "{} is larger than the {} bytes that read_max_bytes allows",
                request.path, settings.read_max_bytes
            ),
        ));
    } else {
        (None, read_bytes(&bytes, &options)?)
    };
    drop(bytes);
    let summary = stack_summary(&doc, 0);
    let reference_ghz = settings.read_reference_mhz as f64 / 1000.0;
    let mapped = summary.as_ref().map(|s| to_stackup(s, reference_ghz));
    if let (Some(path), Some(mapped)) = (request.html, &mapped) {
        let page = stackup_html(&mapped.stackup, language_of(request.lang))
            .map_err(|e| Ipc2581Error::new(INPUT_UNREADABLE, e.to_string()))?;
        write_file(path, &page, request.overwrite, request.path)?;
    }
    if let (Some(path), Some(mapped)) = (request.stackup_out, &mapped) {
        let job = JobTemplate {
            stackup: &mapped.stackup,
            materials: &mapped.materials,
            lines: Vec::new(),
        };
        let mut text = serde_json::to_string_pretty(&job).unwrap_or_default();
        text.push('\n');
        write_file(path, &text, request.overwrite, request.path)?;
    }
    if (request.html.is_some() || request.stackup_out.is_some()) && mapped.is_none() {
        return Err(Ipc2581Error::new(
            READ_STACKUP_INCOMPLETE,
            "the file has no stack-up to write",
        ));
    }
    let mut text = if request.json {
        let report = ReadJson {
            archive_entry: entry.as_deref(),
            revision: &doc.revision,
            units: doc.units.map(|u| u.name()),
            software: &doc.software,
            counts: doc.counts(),
            stackup: summary.as_ref(),
            findings: &doc.findings,
        };
        let mut json = serde_json::to_string_pretty(&report).unwrap_or_default();
        json.push('\n');
        json
    } else {
        let mut text = String::new();
        if let Some(name) = &entry {
            text.push_str(&format!("ZIP: {name}\n"));
        }
        text.push_str(&report_text(&doc, summary.as_ref(), request.lang));
        if let Some(mapped) = &mapped {
            for note in &mapped.notes {
                text.push_str(&format!("note: {note}\n"));
            }
        }
        for path in request.html.iter().chain(request.stackup_out.iter()) {
            text.push_str(&format!("written: {path}\n"));
        }
        text
    };
    if text.is_empty() {
        text.push('\n');
    }
    let _ = out.write_all(text.as_bytes());
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_ipc2581::{read_str, StackRow};

    fn row(name: &str, kind: StackKind, thickness: Option<i64>, material: &str) -> StackRow {
        StackRow {
            position: 1,
            name: name.to_owned(),
            kind,
            thickness_nm: thickness,
            material: material.to_owned(),
            dk: None,
            df: None,
            conductivity_s_per_m: None,
        }
    }

    fn summary(rows: Vec<StackRow>) -> StackSummary {
        StackSummary {
            name: "S".to_owned(),
            rows,
            total_nm: 0,
            declared_nm: None,
            left_out: 0,
            complete: true,
            findings: Vec::new(),
        }
    }

    #[test]
    fn the_kinds_become_the_layers_of_the_model_with_exact_thicknesses() {
        let mut core = row("D", StackKind::Core, Some(1_530_000), "FR-4");
        core.dk = Some(4.3);
        core.df = Some(0.019);
        let mapped = to_stackup(
            &summary(vec![
                row("T", StackKind::SolderMask, Some(20_000), "Mask"),
                row("L1", StackKind::Copper, Some(35_000), ""),
                core,
                row("L2", StackKind::Copper, Some(35_500), ""),
            ]),
            2.5,
        );
        assert_eq!(mapped.stackup.name, "S");
        assert_eq!(mapped.stackup.layers.len(), 4);
        assert_eq!(
            mapped.stackup.layers[1],
            Layer::Copper {
                name: "L1".to_owned(),
                thickness: CopperThickness::Um(35.0),
                roughness_rms_nm: 0
            }
        );
        assert_eq!(
            mapped.stackup.layers[2],
            Layer::Core {
                name: "D".to_owned(),
                thickness_nm: 1_530_000,
                material: "FR-4".to_owned()
            }
        );
        assert_eq!(mapped.materials.len(), 1);
        assert_eq!(mapped.materials[0].name, "FR-4");
        assert_eq!(mapped.materials[0].dk_points, [(2.5, 4.3)]);
        assert_eq!(mapped.materials[0].df_points, [(2.5, 0.019)]);
        // the mask has no constant: no sample, and it is said
        assert!(
            mapped.notes.iter().any(|n| n.contains("Mask")),
            "{:?}",
            mapped.notes
        );
    }

    #[test]
    fn a_missing_value_is_said_and_never_made_up() {
        let mut prepreg = row("P", StackKind::Prepreg, Some(100_000), "Glass");
        prepreg.dk = Some(4.0);
        let mapped = to_stackup(
            &summary(vec![row("L1", StackKind::Copper, None, ""), prepreg]),
            1.0,
        );
        assert_eq!(mapped.stackup.layers.len(), 1);
        assert!(
            mapped.notes[0].contains("L1 has no thickness"),
            "{:?}",
            mapped.notes
        );
        assert!(
            mapped.notes[1].contains("no loss tangent"),
            "{:?}",
            mapped.notes
        );
        assert_eq!(mapped.materials[0].df_points[0].1, 0.0);
    }

    #[test]
    fn two_materials_with_one_name_and_other_constants_are_two_materials() {
        let mut a = row("D1", StackKind::Core, Some(200_000), "FR-4");
        a.dk = Some(4.2);
        a.df = Some(0.02);
        let mut b = row("D2", StackKind::Core, Some(300_000), "FR-4");
        b.dk = Some(4.6);
        b.df = Some(0.02);
        let mut c = row("D3", StackKind::Core, Some(300_000), "FR-4");
        c.dk = Some(4.2);
        c.df = Some(0.02);
        let mapped = to_stackup(&summary(vec![a, b, c]), 1.0);
        let names: Vec<&str> = mapped.materials.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["FR-4", "FR-4 #2"]);
        let used: Vec<&str> = mapped
            .stackup
            .layers
            .iter()
            .filter_map(|l| match l {
                Layer::Core { material, .. } => Some(material.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(used, ["FR-4", "FR-4 #2", "FR-4"]);
    }

    #[test]
    fn long_names_are_cut_to_what_the_model_accepts() {
        let long = "n".repeat(100);
        let mapped = to_stackup(
            &summary(vec![row(&long, StackKind::Copper, Some(1_000), "")]),
            1.0,
        );
        assert_eq!(mapped.stackup.layers[0].name().chars().count(), NAME_CHARS);
    }

    #[test]
    fn the_stackup_of_a_file_read_maps_end_to_end() {
        let doc = read_str(
            "<IPC-2581 revision=\"C\"><Ecad name=\"e\"><CadHeader units=\"MILLIMETER\"><Spec name=\"K\"><General type=\"MATERIAL\"><Property text=\"Core\"/></General><Dielectric type=\"DIELECTRIC_CONSTANT\"><Property value=\"4.4\"/></Dielectric><Dielectric type=\"LOSS_TANGENT\"><Property value=\"0.018\"/></Dielectric></Spec></CadHeader><CadData><Layer name=\"A\" layerFunction=\"SIGNAL\" side=\"TOP\" polarity=\"POSITIVE\"/><Layer name=\"B\" layerFunction=\"DIELCORE\" side=\"INTERNAL\" polarity=\"POSITIVE\"/><Layer name=\"C\" layerFunction=\"SIGNAL\" side=\"BOTTOM\" polarity=\"POSITIVE\"/><Stackup name=\"s\"><StackupGroup name=\"g\"><StackupLayer layerOrGroupRef=\"A\" thickness=\"0.035\"/><StackupLayer layerOrGroupRef=\"B\" thickness=\"1.5\"><SpecRef id=\"K\"/></StackupLayer><StackupLayer layerOrGroupRef=\"C\" thickness=\"0.035\"/></StackupGroup></Stackup></CadData></Ecad></IPC-2581>",
            &ReadOptions::default(),
        )
        .unwrap();
        let mapped = to_stackup(&stack_summary(&doc, 0).unwrap(), 1.0);
        assert!(mapped.notes.is_empty(), "{:?}", mapped.notes);
        assert_eq!(mapped.stackup.total_thickness_nm().unwrap(), 1_570_000);
        assert_eq!(mapped.materials[0].name, "Core");
    }
}
