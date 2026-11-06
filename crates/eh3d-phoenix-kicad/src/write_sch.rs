// SPDX-License-Identifier: GPL-3.0-or-later
//! The schematic model written as KiCad 8 `.kicad_sch` files.
//!
//! The first sheet of the model is the root file; every other sheet is a file of its own that the
//! root refers to with a sheet symbol. The y axis of the model points up and the one of KiCad
//! points down, so every position gets its y turned; the pins inside a symbol keep their y, as in
//! the files of KiCad. The nets of a schematic are not stored by KiCad: they follow from the wires,
//! labels and power symbols, so the report says how many nets rest on the drawing alone. A part
//! that has no place in the drawing is put on a grid and its nets are written as labels on its
//! pins.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_model::Point;
use eh3d_phoenix_schematic::{Id, Part, Schematic, Sheet};

use crate::settings::KicadSettings;
use crate::write_codes::{
    Export, ExportFile, ExportReport, CHANGED, DANGLING, DEFAULTED, LEFT_OUT, ROUND_TRIP,
};
use crate::write_lib::{plan_pins, prefixes, property, symbol_names, symbol_node, PinPlan};
use crate::write_pcb::{check_syntax, clean_name, file_stem};
use crate::write_settings::WriteSettings;
use crate::write_sexpr::{
    at3, atom, effects, int, list, num, pair, render, string, uuid_for, uuid_from, uuid_node, Sx,
};

/// The papers that KiCad names, in nanometres (width, height).
const PAPERS: [(&str, i64, i64); 6] = [
    ("A5", 210_000_000, 148_000_000),
    ("A4", 297_000_000, 210_000_000),
    ("A3", 420_000_000, 297_000_000),
    ("A2", 594_000_000, 420_000_000),
    ("A1", 841_000_000, 594_000_000),
    ("A0", 1_189_000_000, 841_000_000),
];

fn paper_node(sheet: Option<&Sheet>, settings: &WriteSettings) -> Sx {
    let Some(sheet) = sheet else {
        return list("paper", vec![string(&settings.paper)]);
    };
    let (w, h) = (sheet.width.get(), sheet.height.get());
    match PAPERS.iter().find(|(_, pw, ph)| *pw == w && *ph == h) {
        Some((name, _, _)) => list("paper", vec![string(name)]),
        None => list("paper", vec![string("User"), num(w), num(h)]),
    }
}

/// Where a part goes and how it is turned.
struct Place {
    at: (i64, i64),
    quarters: i64,
    mirrored: bool,
}

/// The items that go on one sheet.
#[derive(Default)]
struct Bucket<'a> {
    parts: Vec<(&'a Id, &'a Part)>,
    wires: Vec<(&'a Id, &'a [Point])>,
    labels: Vec<(&'a Id, Point, &'a str)>,
    junctions: Vec<(&'a Id, Point)>,
    ports: Vec<(&'a Id, Point, &'a str)>,
}

struct Env<'a> {
    schematic: &'a Schematic,
    footprints: &'a BTreeMap<Id, String>,
    settings: &'a WriteSettings,
    names: BTreeMap<Id, String>,
    prefixes: BTreeMap<Id, String>,
    plans: BTreeMap<Id, Vec<PinPlan>>,
    root_uuid: String,
}

fn stroke_node(width: i64) -> Sx {
    list(
        "stroke",
        vec![
            list("width", vec![num(width)]),
            list("type", vec![atom("default")]),
        ],
    )
}

fn xy(p: Point) -> Sx {
    pair("xy", p.x_nm.get(), -p.y_nm.get())
}

fn sx(p: Point) -> i64 {
    p.x_nm.get()
}

fn sy(p: Point) -> i64 {
    -p.y_nm.get()
}

fn key2(a: &Id, b: &Id) -> Vec<u8> {
    let mut key = a.as_bytes().to_vec();
    key.extend_from_slice(b.as_bytes());
    key
}

fn yes_no(flag: bool) -> Sx {
    atom(if flag { "yes" } else { "no" })
}

fn power_symbol_node(real: &str, clean: &str, settings: &WriteSettings) -> Sx {
    let pitch = settings.pin_pitch_nm;
    let font = effects(
        settings.text_size_nm,
        settings.text_thickness_nm,
        false,
        &[],
    );
    list(
        "symbol",
        vec![
            string(&format!("power:{clean}")),
            list("power", vec![]),
            list("pin_names", vec![list("offset", vec![int(0)])]),
            list("exclude_from_sim", vec![atom("no")]),
            list("in_bom", vec![atom("no")]),
            list("on_board", vec![atom("yes")]),
            property("Reference", "#PWR", 0, 0, true, settings),
            property("Value", real, 0, pitch, false, settings),
            list(
                "symbol",
                vec![
                    string(&format!("{clean}_0_1")),
                    list(
                        "polyline",
                        vec![
                            list("pts", vec![pair("xy", 0, 0), pair("xy", 0, pitch / 2)]),
                            stroke_node(settings.symbol_line_nm),
                            list("fill", vec![list("type", vec![atom("none")])]),
                        ],
                    ),
                ],
            ),
            list(
                "symbol",
                vec![
                    string(&format!("{clean}_1_1")),
                    list(
                        "pin",
                        vec![
                            atom("power_in"),
                            atom("line"),
                            at3(0, 0, 0),
                            list("length", vec![num(0)]),
                            list("hide", vec![atom("yes")]),
                            list("name", vec![string(real), font.clone()]),
                            list("number", vec![string("1"), font]),
                        ],
                    ),
                ],
            ),
        ],
    )
}

impl Env<'_> {
    fn instances(&self, name: &str, path: &str, reference: &str) -> Sx {
        list(
            "instances",
            vec![list(
                "project",
                vec![
                    string(name),
                    list(
                        "path",
                        vec![
                            string(path),
                            list("reference", vec![string(reference)]),
                            list("unit", vec![int(1)]),
                        ],
                    ),
                ],
            )],
        )
    }

    /// A part of the model as a placed symbol; `place` is already in the orientation of KiCad.
    fn part_node(
        &self,
        part_id: &Id,
        part: &Part,
        place: &Place,
        path: &str,
        report: &mut ExportReport,
    ) -> Option<Sx> {
        let settings = self.settings;
        let at = place.at;
        let Some(short) = self.names.get(&part.symbol) else {
            report.note(
                DANGLING,
                "parts of a symbol the schematic does not have",
                "the part is left out",
            );
            return None;
        };
        let size = settings.text_size_nm;
        let footprint_text = match &part.footprint {
            Some(id) => match self.footprints.get(id) {
                Some(name) => {
                    let cleaned = clean_name(name, report);
                    format!("{}:{}", settings.lib_nickname, cleaned)
                }
                None => {
                    report.note(
                        DANGLING,
                        "parts that name a footprint the schematic cannot find",
                        "the footprint field is left empty",
                    );
                    String::new()
                }
            },
            None => String::new(),
        };
        let mut items = vec![
            list(
                "lib_id",
                vec![string(&format!("{}:{}", settings.lib_nickname, short))],
            ),
            at3(at.0, at.1, place.quarters * 90_000_000),
        ];
        if place.mirrored {
            items.push(list("mirror", vec![atom("y")]));
        }
        items.push(list("unit", vec![int(1)]));
        items.push(list("exclude_from_sim", vec![yes_no(false)]));
        items.push(list("in_bom", vec![yes_no(true)]));
        items.push(list("on_board", vec![yes_no(true)]));
        items.push(list("dnp", vec![yes_no(false)]));
        items.push(uuid_node(part_id, "schematic-part"));
        items.push(property(
            "Reference",
            &part.refdes,
            at.0,
            at.1 - 2 * size,
            false,
            settings,
        ));
        items.push(property(
            "Value",
            &part.value,
            at.0,
            at.1 + 2 * size,
            false,
            settings,
        ));
        items.push(property(
            "Footprint",
            &footprint_text,
            at.0,
            at.1,
            true,
            settings,
        ));
        items.push(property("Datasheet", "~", at.0, at.1, true, settings));
        if let Some(plans) = self.plans.get(&part.symbol) {
            for plan in plans {
                items.push(list(
                    "pin",
                    vec![
                        string(&plan.number),
                        list(
                            "uuid",
                            vec![string(&uuid_from(
                                "schematic-pin",
                                &key2(part_id, &plan.id),
                            ))],
                        ),
                    ],
                ));
            }
        }
        items.push(self.instances(&self.schematic.name, path, &part.refdes));
        Some(list("symbol", items))
    }
}

fn quarters_of(udeg: i64) -> (i64, bool) {
    let quarter = 90_000_000;
    let quarters = (udeg + quarter / 2).div_euclid(quarter).rem_euclid(4);
    (quarters, udeg.rem_euclid(quarter) != 0)
}

/// Writes a schematic as a `.kicad_sch` file for each sheet. `footprints` gives the name of each
/// footprint the parts refer to.
pub fn write_schematic(
    schematic: &Schematic,
    footprints: &BTreeMap<Id, String>,
    settings: &WriteSettings,
) -> Export {
    let mut report = ExportReport::default();
    let names = symbol_names(schematic, &mut report);
    let mut plans: BTreeMap<Id, Vec<PinPlan>> = BTreeMap::new();
    for (id, symbol) in &schematic.symbols {
        plans.insert(
            *id,
            plan_pins(symbol, &schematic.pin_kinds, settings, &mut report),
        );
    }
    let env = Env {
        schematic,
        footprints,
        settings,
        names,
        prefixes: prefixes(schematic),
        plans,
        root_uuid: uuid_from("schematic", schematic.name.as_bytes()),
    };

    let sheets: Vec<(&Id, &Sheet)> = schematic.sheets.iter().collect();
    let count = sheets.len().max(1);
    let mut stray = 0usize;
    let mut slot = |sheet: &Id| -> usize {
        match sheets.iter().position(|(id, _)| *id == sheet) {
            Some(index) => index,
            None => {
                if !sheets.is_empty() {
                    stray += 1;
                }
                0
            }
        }
    };
    let mut buckets: Vec<Bucket> = (0..count).map(|_| Bucket::default()).collect();
    let mut unplaced: Vec<(&Id, &Part)> = Vec::new();
    for (id, part) in &schematic.parts {
        match &part.placement {
            Some(placement) => {
                let index = slot(&placement.sheet);
                buckets[index].parts.push((id, part));
            }
            None => unplaced.push((id, part)),
        }
    }
    for (id, wire) in &schematic.wires {
        let index = slot(&wire.sheet);
        buckets[index].wires.push((id, wire.path.as_slice()));
    }
    for (id, label) in &schematic.labels {
        let index = slot(&label.sheet);
        buckets[index]
            .labels
            .push((id, label.at, label.name.as_str()));
    }
    for (id, junction) in &schematic.junctions {
        let index = slot(&junction.sheet);
        buckets[index].junctions.push((id, junction.at));
    }
    for (id, port) in &schematic.power_ports {
        let index = slot(&port.sheet);
        buckets[index].ports.push((id, port.at, port.name.as_str()));
    }
    if stray > 0 {
        report.note_many(
            CHANGED,
            "items on a sheet the schematic does not have",
            "written on the first sheet",
            stray,
        );
    }

    // the file names
    let root_stem = file_stem(&schematic.name, "schematic");
    let mut file_names: Vec<String> = vec![format!("{root_stem}.kicad_sch")];
    let mut used: BTreeSet<String> = BTreeSet::new();
    used.insert(file_names[0].clone());
    for (_, sheet) in sheets.iter().skip(1) {
        let mut name = format!("{root_stem}_{}.kicad_sch", file_stem(&sheet.name, "sheet"));
        if used.contains(&name) {
            name = format!(
                "{root_stem}_{}_{}.kicad_sch",
                file_stem(&sheet.name, "sheet"),
                file_names.len()
            );
            report.note(
                CHANGED,
                "sheets with a name that another sheet has",
                "the number of the sheet was added to the file name",
            );
        }
        used.insert(name.clone());
        file_names.push(name);
    }

    let unplaced_index: BTreeMap<&Id, usize> = unplaced
        .iter()
        .enumerate()
        .map(|(index, (id, _))| (*id, index))
        .collect();
    let mut power_count = 0usize;
    let mut files: Vec<ExportFile> = Vec::new();
    let mut written_parts_root = 0usize;
    for (index, bucket) in buckets.iter().enumerate() {
        let sheet = sheets.get(index).map(|(_, s)| *s);
        let path = if index == 0 {
            format!("/{}", env.root_uuid)
        } else {
            let block = uuid_for(sheets[index].0, "sheet-block");
            format!("/{}/{}", env.root_uuid, block)
        };
        let mut body: Vec<Sx> = Vec::new();
        let mut used_symbols: BTreeSet<Id> = BTreeSet::new();
        for (part_id, part) in &bucket.parts {
            let Some(placement) = &part.placement else {
                continue;
            };
            let (quarters, rounded) = quarters_of(placement.rotation.get());
            if rounded {
                report.note(
                    CHANGED,
                    "turns that are not a right angle",
                    "rounded to the nearest right angle",
                );
            }
            let place = Place {
                at: (sx(placement.at), sy(placement.at)),
                quarters,
                mirrored: schematic.mirrored.contains(*part_id),
            };
            if let Some(node) = env.part_node(part_id, part, &place, &path, &mut report) {
                used_symbols.insert(part.symbol);
                body.push(node);
                report.wrote("parts", 1);
                if index == 0 {
                    written_parts_root += 1;
                }
            }
        }
        if index == 0 {
            for (part_id, part) in &unplaced {
                let slot_index = unplaced_index.get(part_id).copied().unwrap_or(0) as i64;
                let place = Place {
                    at: (
                        (slot_index % 8 + 1) * settings.grid_nm,
                        (slot_index / 8 + 1) * settings.grid_nm,
                    ),
                    quarters: 0,
                    mirrored: false,
                };
                if let Some(node) = env.part_node(part_id, part, &place, &path, &mut report) {
                    used_symbols.insert(part.symbol);
                    body.push(node);
                    report.wrote("parts", 1);
                    written_parts_root += 1;
                }
            }
            if !unplaced.is_empty() {
                report.note_many(
                    DEFAULTED,
                    "parts without a place in the drawing",
                    "put on a grid at the top of the first sheet",
                    unplaced.len(),
                );
            }
            body.extend(unplaced_labels(
                &env,
                &unplaced,
                &unplaced_index,
                &mut report,
            ));
        }
        // wires, junctions, labels
        for (wire_id, points) in &bucket.wires {
            if points.len() < 2 {
                report.note(
                    LEFT_OUT,
                    "wires with fewer than two points",
                    "the wire is left out",
                );
                continue;
            }
            for (k, pair_of_points) in points.windows(2).enumerate() {
                body.push(list(
                    "wire",
                    vec![
                        list("pts", vec![xy(pair_of_points[0]), xy(pair_of_points[1])]),
                        stroke_node(0),
                        uuid_node(wire_id, &format!("wire:{k}")),
                    ],
                ));
                report.wrote("wire pieces", 1);
            }
        }
        for (junction_id, at) in &bucket.junctions {
            body.push(list(
                "junction",
                vec![
                    pair("at", sx(*at), sy(*at)),
                    list("diameter", vec![num(0)]),
                    list("color", vec![int(0), int(0), int(0), int(0)]),
                    uuid_node(junction_id, "junction"),
                ],
            ));
            report.wrote("junctions", 1);
        }
        for (label_id, at, name) in &bucket.labels {
            body.push(label_node(
                name,
                sx(*at),
                sy(*at),
                uuid_node(label_id, "label"),
                settings,
            ));
            report.wrote("labels", 1);
        }
        let mut power_names: BTreeMap<String, String> = BTreeMap::new();
        for (_, _, name) in &bucket.ports {
            let mut scratch = ExportReport::default();
            let clean = clean_name(name, &mut scratch);
            report.merge(&scratch);
            power_names.insert((*name).to_owned(), clean);
        }
        for (port_id, at, name) in &bucket.ports {
            power_count += 1;
            let clean = power_names.get(*name).cloned().unwrap_or_default();
            let reference = format!("#PWR{power_count:02}");
            let size = settings.text_size_nm;
            let (x, y) = (sx(*at), sy(*at));
            body.push(list(
                "symbol",
                vec![
                    list("lib_id", vec![string(&format!("power:{clean}"))]),
                    at3(x, y, 0),
                    list("unit", vec![int(1)]),
                    list("exclude_from_sim", vec![yes_no(false)]),
                    list("in_bom", vec![yes_no(false)]),
                    list("on_board", vec![yes_no(true)]),
                    list("dnp", vec![yes_no(false)]),
                    uuid_node(port_id, "power-port"),
                    property("Reference", &reference, x, y - 2 * size, true, settings),
                    property("Value", name, x, y + 2 * size, false, settings),
                    list(
                        "pin",
                        vec![
                            string("1"),
                            list(
                                "uuid",
                                vec![string(&uuid_from("power-pin", port_id.as_bytes()))],
                            ),
                        ],
                    ),
                    env.instances(&schematic.name, &path, &reference),
                ],
            ));
            report.wrote("power ports", 1);
        }

        // the sheet symbols of the root
        if index == 0 {
            for (k, (sheet_id, sub)) in sheets.iter().enumerate().skip(1) {
                body.push(sheet_block(
                    &env,
                    sheet_id,
                    sub,
                    &file_names[k],
                    (k as i64 - 1) * 2 * settings.grid_nm,
                    k + 1,
                ));
            }
        }

        // the library of the symbols used on this sheet
        let mut lib: Vec<Sx> = Vec::new();
        for symbol_id in &used_symbols {
            if let (Some(short), Some(plan)) = (env.names.get(symbol_id), env.plans.get(symbol_id))
            {
                let prefix = env
                    .prefixes
                    .get(symbol_id)
                    .cloned()
                    .unwrap_or_else(|| "U".to_owned());
                lib.push(symbol_node(
                    &format!("{}:{}", settings.lib_nickname, short),
                    short,
                    &prefix,
                    plan,
                    settings,
                ));
            }
        }
        for (real, clean) in &power_names {
            lib.push(power_symbol_node(real, clean, settings));
        }

        let uuid = if index == 0 {
            env.root_uuid.clone()
        } else {
            uuid_for(sheets[index].0, "sheet-file")
        };
        let mut root: Vec<Sx> = vec![
            list("version", vec![int(settings.sch_version as i64)]),
            list("generator", vec![string(&settings.generator)]),
            list("uuid", vec![string(&uuid)]),
            paper_node(sheet, settings),
            list("lib_symbols", lib),
        ];
        root.extend(body);
        root.push(list(
            "sheet_instances",
            vec![list(
                "path",
                vec![
                    string("/"),
                    list("page", vec![string(&(index + 1).to_string())]),
                ],
            )],
        ));
        let text = render(&list("kicad_sch", root));
        check_syntax(&text, "a schematic file", &mut report);
        files.push(ExportFile {
            name: file_names[index].clone(),
            text,
        });
    }

    let on_drawing = schematic
        .nets
        .values()
        .filter(|net| {
            net.pins.iter().any(|pin| {
                schematic
                    .parts
                    .get(&pin.part)
                    .is_some_and(|p| p.placement.is_some())
            })
        })
        .count();
    if on_drawing > 0 {
        report.note_many(
            LEFT_OUT,
            "nets of placed parts (KiCad derives nets from the wires and labels)",
            "the net list of the model is not written; the drawing holds the connections",
            on_drawing,
        );
    }
    if sheets.len() > 1 {
        report.note(
            CHANGED,
            "labels of a sheet",
            "labels are local to their sheet; connections between sheets are not written",
        );
    }
    report.wrote("sheets", count);
    check_root(&files[0].text, written_parts_root, &mut report);
    Export { files, report }
}

/// A label with the text, the place and the id given.
fn label_node(name: &str, x: i64, y: i64, uuid: Sx, settings: &WriteSettings) -> Sx {
    list(
        "label",
        vec![
            string(name),
            at3(x, y, 0),
            effects(
                settings.text_size_nm,
                settings.text_thickness_nm,
                false,
                &["left", "bottom"],
            ),
            uuid,
        ],
    )
}

/// Labels on the pins of the parts that have no place, one for each pin of a net that has a name.
fn unplaced_labels(
    env: &Env,
    unplaced: &[(&Id, &Part)],
    index: &BTreeMap<&Id, usize>,
    report: &mut ExportReport,
) -> Vec<Sx> {
    let settings = env.settings;
    let mut out = Vec::new();
    for (net_id, net) in &env.schematic.nets {
        for pin in &net.pins {
            let Some(slot) = index.get(&pin.part) else {
                continue;
            };
            let Some((_, part)) = unplaced.get(*slot) else {
                continue;
            };
            let Some(plan) = env
                .plans
                .get(&part.symbol)
                .and_then(|plans| plans.iter().find(|p| p.id == pin.pin))
            else {
                crate::write_lib::dangling_pin(report);
                continue;
            };
            if net.name.is_empty() {
                report.note(
                    LEFT_OUT,
                    "nets without a name that join parts without a place",
                    "no label can be written",
                );
                continue;
            }
            let slot_index = *slot as i64;
            let origin_x = (slot_index % 8 + 1) * settings.grid_nm;
            let origin_y = (slot_index / 8 + 1) * settings.grid_nm;
            let mut key = key2(net_id, &pin.part);
            key.extend_from_slice(pin.pin.as_bytes());
            out.push(label_node(
                &net.name,
                origin_x + plan.x,
                origin_y - plan.y,
                list("uuid", vec![string(&uuid_from("pin-label", &key))]),
                settings,
            ));
            report.wrote("labels", 1);
            report.note(
                CHANGED,
                "nets of parts without a place, written as labels on their pins",
                "the label joins the pin to the net by name",
            );
        }
    }
    out
}

/// The symbol of a sheet inside the root file.
fn sheet_block(env: &Env, sheet_id: &Id, sheet: &Sheet, file: &str, x: i64, page: usize) -> Sx {
    let settings = env.settings;
    let size = settings.text_size_nm;
    let y = -3 * settings.grid_nm;
    let (w, h) = (settings.grid_nm, settings.grid_nm / 2);
    let font = |justify: &[&str]| effects(size, settings.text_thickness_nm, false, justify);
    let block_uuid = uuid_for(sheet_id, "sheet-block");
    list(
        "sheet",
        vec![
            pair("at", x, y),
            pair("size", w, h),
            list("fields_autoplaced", vec![atom("yes")]),
            list(
                "stroke",
                vec![
                    list("width", vec![num(settings.symbol_line_nm)]),
                    list("type", vec![atom("solid")]),
                ],
            ),
            list(
                "fill",
                vec![list("color", vec![int(0), int(0), int(0), atom("0.0")])],
            ),
            list("uuid", vec![string(&block_uuid)]),
            list(
                "property",
                vec![
                    string("Sheetname"),
                    string(&sheet.name),
                    at3(x, y - size, 0),
                    font(&["left", "bottom"]),
                ],
            ),
            list(
                "property",
                vec![
                    string("Sheetfile"),
                    string(file),
                    at3(x, y + h + size, 0),
                    font(&["left", "top"]),
                ],
            ),
            list(
                "instances",
                vec![list(
                    "project",
                    vec![
                        string(&env.schematic.name),
                        list(
                            "path",
                            vec![
                                string(&format!("/{}", env.root_uuid)),
                                list("page", vec![string(&page.to_string())]),
                            ],
                        ),
                    ],
                )],
            ),
        ],
    )
}

/// Reads the root file back with the schematic reader of the crate and compares the number of
/// parts with what was written on the root sheet.
fn check_root(text: &str, parts: usize, report: &mut ExportReport) {
    let mut ids = match crate::sequential_ids() {
        Ok(ids) => ids,
        Err(error) => {
            report.note(ROUND_TRIP, "the check could not start", &error.to_string());
            return;
        }
    };
    match crate::schematic::read_schematic(text.as_bytes(), &KicadSettings::default(), &mut ids) {
        Ok(read) => {
            let got = read.schematic.parts.len();
            if got != parts {
                report.note(
                    ROUND_TRIP,
                    "parts of the first sheet",
                    &format!("{parts} were written and {got} were read back"),
                );
            }
        }
        Err(diagnostic) => report.note(ROUND_TRIP, "the schematic file", &diagnostic.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write_lib::sample::{sample_schematic, sid, spt};
    use crate::write_sexpr::testkit::{arg, child, children, head, items, parse};
    use eh3d_phoenix_model::Nm;

    fn write(s: &Schematic) -> Export {
        write_schematic(s, &BTreeMap::new(), &WriteSettings::default())
    }

    #[test]
    fn a_schematic_is_written_with_its_parts_wires_and_marks() {
        let s = sample_schematic();
        let out = write(&s);
        assert_eq!(out.files.len(), 1);
        assert_eq!(out.files[0].name, "Demo sheet.kicad_sch");
        let tree = parse(&out.files[0].text).unwrap();
        assert_eq!(head(&tree), Some("kicad_sch"));
        assert_eq!(arg(child(&tree, "version").unwrap(), 0), Some("20231120"));
        assert_eq!(arg(child(&tree, "paper").unwrap(), 0), Some("A4"));
        let symbols = children(&tree, "symbol");
        // R1, R2, R3 and the power port
        assert_eq!(symbols.len(), 4);
        assert_eq!(children(&tree, "wire").len(), 1);
        assert_eq!(children(&tree, "junction").len(), 1);
        // the label SIG and the labels of the net on the pin of R3
        assert_eq!(children(&tree, "label").len(), 2);
        let lib = child(&tree, "lib_symbols").unwrap();
        let lib_names: Vec<&str> = children(lib, "symbol")
            .iter()
            .filter_map(|n| arg(n, 0))
            .collect();
        assert_eq!(lib_names, vec!["phoenix:R", "power:GND"]);
        assert!(
            !out.report.has_problem(),
            "{}",
            out.report.text(crate::error::Language::En)
        );
    }

    #[test]
    fn positions_turn_y_and_a_part_keeps_its_pins_and_reference() {
        let s = sample_schematic();
        let tree = parse(&write(&s).files[0].text).unwrap();
        let symbols = children(&tree, "symbol");
        let r1 = symbols
            .iter()
            .find(|n| {
                children(n, "property")
                    .iter()
                    .any(|p| arg(p, 0) == Some("Reference") && arg(p, 1) == Some("R1"))
            })
            .unwrap();
        let at = child(r1, "at").unwrap();
        assert_eq!(arg(at, 0), Some("50.8"));
        assert_eq!(arg(at, 1), Some("-100"));
        assert_eq!(children(r1, "pin").len(), 2);
        assert_eq!(arg(child(r1, "lib_id").unwrap(), 0), Some("phoenix:R"));
        let wire = children(&tree, "wire")[0];
        let pts = child(wire, "pts").unwrap();
        let first = children(pts, "xy")[0];
        assert_eq!(arg(first, 0), Some("55.88"));
        assert_eq!(arg(first, 1), Some("-100"));
    }

    #[test]
    fn a_part_without_a_place_goes_on_the_grid_and_its_net_becomes_labels() {
        let s = sample_schematic();
        let out = write(&s);
        let tree = parse(&out.files[0].text).unwrap();
        let labels = children(&tree, "label");
        let names: Vec<&str> = labels.iter().filter_map(|l| arg(l, 0)).collect();
        assert_eq!(names, vec!["SIG", "SIG"]);
        assert!(out.report.count_of(DEFAULTED) >= 1);
        assert!(out
            .report
            .notes()
            .iter()
            .any(|n| n.category.contains("without a place")));
    }

    #[test]
    fn the_output_is_deterministic_and_the_uuids_are_distinct() {
        let s = sample_schematic();
        let a = write(&s);
        let b = write(&s.clone());
        assert_eq!(a.files, b.files);
        let tree = parse(&a.files[0].text).unwrap();
        let mut uuids: Vec<String> = Vec::new();
        collect(&tree, &mut uuids);
        let unique: BTreeSet<&String> = uuids.iter().collect();
        assert_eq!(unique.len(), uuids.len());
    }

    fn collect(node: &Sx, out: &mut Vec<String>) {
        if head(node) == Some("uuid") {
            if let Some(text) = arg(node, 0) {
                out.push(text.to_owned());
            }
        }
        for item in items(node) {
            collect(item, out);
        }
    }

    #[test]
    fn the_root_file_reads_back_with_the_reader_of_the_crate() {
        let s = sample_schematic();
        let out = write(&s);
        let mut ids = crate::sequential_ids().unwrap();
        let read = crate::schematic::read_schematic(
            out.files[0].text.as_bytes(),
            &KicadSettings::default(),
            &mut ids,
        )
        .unwrap();
        assert_eq!(read.schematic.parts.len(), 3);
        let refs: BTreeSet<&str> = read
            .schematic
            .parts
            .values()
            .map(|p| p.refdes.as_str())
            .collect();
        assert_eq!(refs, BTreeSet::from(["R1", "R2", "R3"]));
    }

    #[test]
    fn a_second_sheet_is_a_file_of_its_own_and_the_root_refers_to_it() {
        let mut s = sample_schematic();
        let second = sid(11);
        s.sheets.insert(
            second,
            Sheet {
                name: "Power".into(),
                width: Nm::new(420_000_000).unwrap(),
                height: Nm::new(297_000_000).unwrap(),
            },
        );
        s.wires.insert(
            sid(34),
            eh3d_phoenix_schematic::Wire {
                sheet: second,
                path: vec![spt(0, 0), spt(10_000_000, 0), spt(10_000_000, 10_000_000)],
            },
        );
        let out = write(&s);
        assert_eq!(out.files.len(), 2);
        assert_eq!(out.files[1].name, "Demo sheet_Power.kicad_sch");
        let root = parse(&out.files[0].text).unwrap();
        let blocks = children(&root, "sheet");
        assert_eq!(blocks.len(), 1);
        let file = children(blocks[0], "property")
            .into_iter()
            .find(|p| arg(p, 0) == Some("Sheetfile"))
            .and_then(|p| arg(p, 1).map(str::to_owned));
        assert_eq!(file.as_deref(), Some("Demo sheet_Power.kicad_sch"));
        let sub = parse(&out.files[1].text).unwrap();
        assert_eq!(arg(child(&sub, "paper").unwrap(), 0), Some("A3"));
        assert_eq!(children(&sub, "wire").len(), 2);
        assert!(out
            .report
            .notes()
            .iter()
            .any(|n| n.category == "labels of a sheet"));
        assert!(
            !out.report.has_problem(),
            "{}",
            out.report.text(crate::error::Language::En)
        );
    }

    #[test]
    fn odd_papers_are_user_papers_and_unknown_sheets_are_counted() {
        let mut s = sample_schematic();
        let sheet = s.sheets.values_mut().next().unwrap();
        sheet.width = Nm::new(300_000_000).unwrap();
        s.wires.insert(
            sid(35),
            eh3d_phoenix_schematic::Wire {
                sheet: sid(77),
                path: vec![spt(0, 0), spt(1_000_000, 0)],
            },
        );
        let out = write(&s);
        let tree = parse(&out.files[0].text).unwrap();
        let paper = child(&tree, "paper").unwrap();
        assert_eq!(arg(paper, 0), Some("User"));
        assert_eq!(arg(paper, 1), Some("300"));
        assert_eq!(children(&tree, "wire").len(), 2);
        assert!(
            out.report.count_of(CHANGED) >= 2,
            "{}",
            out.report.text(crate::error::Language::En)
        );
    }

    #[test]
    fn a_part_of_a_symbol_that_is_missing_is_left_out_and_counted() {
        let mut s = sample_schematic();
        s.parts.get_mut(&sid(21)).unwrap().symbol = sid(99);
        let out = write(&s);
        assert_eq!(out.report.count_of(DANGLING), 1);
        assert_eq!(out.report.written_count("parts"), 2);
        assert!(
            !out.report.has_problem(),
            "{}",
            out.report.text(crate::error::Language::En)
        );
    }

    #[test]
    fn turns_are_rounded_to_right_angles_and_mirrors_are_written() {
        let mut s = sample_schematic();
        let p = s.parts.get_mut(&sid(20)).unwrap();
        if let Some(placement) = p.placement.as_mut() {
            placement.rotation = eh3d_phoenix_model::Udeg::new(100_000_000).unwrap();
        }
        s.mirrored.insert(sid(21));
        let out = write(&s);
        let tree = parse(&out.files[0].text).unwrap();
        let mirrored = children(&tree, "symbol")
            .iter()
            .filter(|n| child(n, "mirror").is_some())
            .count();
        assert_eq!(mirrored, 1);
        let turned = children(&tree, "symbol")
            .iter()
            .filter(|n| child(n, "at").and_then(|a| arg(a, 2)) == Some("90"))
            .count();
        assert_eq!(turned, 1);
        assert!(out
            .report
            .notes()
            .iter()
            .any(|n| n.category.contains("right angle")));
    }
}
