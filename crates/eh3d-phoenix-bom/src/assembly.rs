// SPDX-License-Identifier: GPL-3.0-or-later
//! The bill of materials of the assembly: the same parts as [`crate::bom`], but for each face of
//! the board (the top face, the bottom face and the whole board) and with the quantities of one
//! board and of a panel of several boards. A manufacturer that fits the two faces in two passes
//! asks for one list for each pass.
//!
//! Each face is a bill made by [`build_bom`] on a copy of the board that holds only the
//! components of that face, so the grouping by value, footprint, maker and part number, the
//! parts that are not fitted and the prices are those of the plain bill. The schematic is not used
//! for the faces (its parts have no face); the whole board uses it when it is given. The parts
//! that are not fitted are listed apart, with the status `dnp`, so that the assembler sees them.
//! The cost of the bare boards belongs to the plain bill and is left out here.
//!
//! The text is the same every time: lines in the order of their first reference, money with six
//! decimals worked out in whole millionths.

use std::collections::BTreeMap;

use eh3d_phoenix_board::Side;
use serde_json::{json, Value};

use crate::bom::{build_bom, natural_key, BomInputs, BomLine};
use crate::codes::{BomError, Finding, NOT_FITTED, PCB_ESTIMATE, PCB_NOT_COSTED};
use crate::money::format_micro;
use crate::text::{csv_raw, csv_text};

/// The format tag of the JSON file.
pub const FORMAT: &str = "phoenix-assembly-bom/1";

/// The columns of the CSV file.
pub const COLUMNS: [&str; 12] = [
    "references",
    "quantity_board",
    "quantity_panel",
    "value",
    "footprint",
    "manufacturer",
    "mpn",
    "supplier",
    "supplier_code",
    "unit_price",
    "quantity_to_buy",
    "status",
];

/// A group of parts that are not fitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnpLine {
    /// The value.
    pub value: String,
    /// The name of the footprint; empty when there is none.
    pub footprint: String,
    /// The maker; empty when it is not known.
    pub manufacturer: String,
    /// The part number; empty when it is not known.
    pub mpn: String,
    /// The references, in natural order.
    pub references: Vec<String>,
}

/// The bill of one face.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyFace {
    /// `top`, `bottom` or `board`.
    pub name: &'static str,
    /// The lines of the parts that are fitted.
    pub lines: Vec<BomLine>,
    /// The parts that are not fitted.
    pub not_fitted: Vec<DnpLine>,
    /// What was found.
    pub findings: Vec<Finding>,
}

impl AssemblyFace {
    /// The number of parts that are fitted on one board.
    pub fn parts(&self) -> u64 {
        self.lines.iter().map(|l| l.qty).sum()
    }
}

/// The bills of the three views of a board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyBom {
    /// The top face, the bottom face and the whole board, in that order.
    pub faces: Vec<AssemblyFace>,
    /// The boards in a panel (1 when there is no panel).
    pub boards_per_panel: u64,
}

fn face_of(
    inputs: &BomInputs,
    name: &'static str,
    side: Option<Side>,
) -> Result<AssemblyFace, BomError> {
    let mut board = inputs.board.clone();
    let schematic = match side {
        Some(side) => {
            board.components.retain(|_, c| c.side == side);
            None
        }
        None => inputs.schematic,
    };
    let bom = build_bom(&BomInputs {
        board: &board,
        schematic,
        parts: inputs.parts,
        prices: inputs.prices,
        settings: inputs.settings,
        stackup: inputs.stackup,
    })?;
    type Key = (String, String, String, String);
    let mut groups: BTreeMap<Key, Vec<String>> = BTreeMap::new();
    for finding in &bom.findings {
        if finding.code != NOT_FITTED {
            continue;
        }
        let Some(reference) = finding.subject.as_deref() else {
            continue;
        };
        let component = board.components.values().find(|c| c.refdes == reference);
        let (value, footprint) = component.map_or_else(
            || (String::new(), String::new()),
            |c| {
                (
                    c.value.clone(),
                    board
                        .footprints
                        .get(&c.footprint)
                        .map_or_else(String::new, |f| f.name.clone()),
                )
            },
        );
        let (manufacturer, mpn) = inputs
            .parts
            .find(reference, &value, &footprint)
            .map(|d| (d.manufacturer.clone(), d.mpn.clone()))
            .unwrap_or_default();
        groups
            .entry((value, footprint, manufacturer, mpn))
            .or_default()
            .push(reference.to_owned());
    }
    let mut not_fitted: Vec<DnpLine> = groups
        .into_iter()
        .map(|((value, footprint, manufacturer, mpn), mut references)| {
            references.sort_by_key(|r| natural_key(r));
            DnpLine {
                value,
                footprint,
                manufacturer,
                mpn,
                references,
            }
        })
        .collect();
    not_fitted.sort_by_key(|l| natural_key(&l.references[0]));
    let findings = bom
        .findings
        .into_iter()
        .filter(|f| f.code != PCB_NOT_COSTED && f.code != PCB_ESTIMATE)
        .collect();
    Ok(AssemblyFace {
        name,
        lines: bom.lines,
        not_fitted,
        findings,
    })
}

/// Builds the bills of the top face, the bottom face and the whole board. `boards_per_panel` is
/// the number of boards in a panel (at least 1).
pub fn build_assembly(inputs: &BomInputs, boards_per_panel: u64) -> Result<AssemblyBom, BomError> {
    let boards_per_panel = boards_per_panel.max(1);
    Ok(AssemblyBom {
        faces: vec![
            face_of(inputs, "top", Some(Side::Top))?,
            face_of(inputs, "bottom", Some(Side::Bottom))?,
            face_of(inputs, "board", None)?,
        ],
        boards_per_panel,
    })
}

fn row(cells: [String; 12], numeric: &[usize]) -> String {
    let text: Vec<String> = cells
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if numeric.contains(&i) {
                csv_raw(c)
            } else {
                csv_text(c)
            }
        })
        .collect();
    let mut line = text.join(",");
    line.push('\n');
    line
}

/// The CSV file of one face: a header line, a line for each group of parts that are fitted and a
/// line with the status `dnp` for each group that is not.
pub fn assembly_csv(face: &AssemblyFace, boards_per_panel: u64) -> String {
    let panel = boards_per_panel.max(1);
    let mut out = COLUMNS.join(",");
    out.push('\n');
    for line in &face.lines {
        let (supplier, code, unit, buy) = match &line.purchase {
            Some(p) => (
                p.supplier.clone(),
                p.supplier_code.clone(),
                format_micro(i128::from(p.unit_price_micro)),
                p.buy_qty.to_string(),
            ),
            None => (String::new(), String::new(), String::new(), String::new()),
        };
        let status = if line.footprint.is_empty() {
            "no_footprint"
        } else {
            "fitted"
        };
        out.push_str(&row(
            [
                line.references.join(" "),
                line.qty.to_string(),
                (line.qty * panel).to_string(),
                line.value.clone(),
                line.footprint.clone(),
                line.manufacturer.clone(),
                line.mpn.clone(),
                supplier,
                code,
                unit,
                buy,
                status.to_owned(),
            ],
            &[1, 2, 9, 10],
        ));
    }
    for line in &face.not_fitted {
        let qty = line.references.len() as u64;
        out.push_str(&row(
            [
                line.references.join(" "),
                qty.to_string(),
                (qty * panel).to_string(),
                line.value.clone(),
                line.footprint.clone(),
                line.manufacturer.clone(),
                line.mpn.clone(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "dnp".to_owned(),
            ],
            &[1, 2],
        ));
    }
    out
}

fn line_json(line: &BomLine, panel: u64) -> Value {
    json!({
        "references": line.references,
        "quantity_board": line.qty,
        "quantity_panel": line.qty * panel,
        "value": line.value,
        "footprint": line.footprint,
        "manufacturer": line.manufacturer,
        "mpn": line.mpn,
        "supplier": line.purchase.as_ref().map(|p| p.supplier.clone()),
        "supplier_code": line.purchase.as_ref().map(|p| p.supplier_code.clone()),
        "unit_price": line.purchase.as_ref().map(|p| format_micro(i128::from(p.unit_price_micro))),
        "quantity_to_buy": line.purchase.as_ref().map(|p| p.buy_qty),
    })
}

/// The three bills as JSON (`phoenix-assembly-bom/1`), ending with a line break.
pub fn assembly_json(bom: &AssemblyBom) -> String {
    let panel = bom.boards_per_panel;
    let faces: Vec<Value> = bom
        .faces
        .iter()
        .map(|face| {
            let dnp: Vec<Value> = face
                .not_fitted
                .iter()
                .map(|l| {
                    json!({
                        "references": l.references,
                        "quantity_board": l.references.len(),
                        "quantity_panel": l.references.len() as u64 * panel,
                        "value": l.value,
                        "footprint": l.footprint,
                        "manufacturer": l.manufacturer,
                        "mpn": l.mpn,
                    })
                })
                .collect();
            let findings: Vec<Value> = face
                .findings
                .iter()
                .map(|f| {
                    json!({
                        "code": f.code,
                        "severity": f.severity.as_str(),
                        "subject": f.subject,
                        "detail": f.detail,
                    })
                })
                .collect();
            json!({
                "face": face.name,
                "parts_board": face.parts(),
                "parts_panel": face.parts() * panel,
                "lines": face.lines.iter().map(|l| line_json(l, panel)).collect::<Vec<_>>(),
                "not_fitted": dnp,
                "findings": findings,
            })
        })
        .collect();
    let value = json!({
        "format": FORMAT,
        "boards_per_panel": panel,
        "faces": faces,
    });
    let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parts::PartsFile;
    use crate::prices::PriceBook;
    use crate::settings::CostSettings;
    use eh3d_phoenix_board::{Board, Component, Footprint, Id};
    use eh3d_phoenix_model::{ObjectId, Point, Udeg};

    fn id(n: u32) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
    }

    fn part(refdes: &str, value: &str, side: Side) -> Component {
        Component {
            refdes: refdes.to_owned(),
            value: value.to_owned(),
            symbol: None,
            footprint: id(10),
            at: Point::from_nm(0, 0).unwrap(),
            rotation: Udeg::new(0).unwrap(),
            side,
        }
    }

    fn board() -> Board {
        let mut b = Board::default();
        b.footprints.insert(
            id(10),
            Footprint {
                name: "R0603".to_owned(),
                pads: Default::default(),
            },
        );
        for (n, refdes, value, side) in [
            (20, "R1", "10k", Side::Top),
            (21, "R2", "10k", Side::Top),
            (22, "R3", "1k", Side::Bottom),
            (23, "R10", "10k", Side::Bottom),
            (24, "R4", "4k7", Side::Top),
        ] {
            b.components.insert(id(n), part(refdes, value, side));
        }
        b
    }

    fn inputs<'a>(
        board: &'a Board,
        parts: &'a PartsFile,
        prices: &'a PriceBook,
        settings: &'a CostSettings,
    ) -> BomInputs<'a> {
        BomInputs {
            board,
            schematic: None,
            parts,
            prices,
            settings,
            stackup: None,
        }
    }

    #[test]
    fn each_face_groups_its_own_parts_and_the_panel_multiplies_them() {
        let b = board();
        let parts = PartsFile::parse("match,fitted\nR4,no\n", 10).unwrap();
        let prices = PriceBook::default();
        let settings = CostSettings::default();
        let bom = build_assembly(&inputs(&b, &parts, &prices, &settings), 4).unwrap();
        let names: Vec<&str> = bom.faces.iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["top", "bottom", "board"]);
        let top = &bom.faces[0];
        assert_eq!(top.lines.len(), 1);
        assert_eq!(top.lines[0].references, vec!["R1", "R2"]);
        assert_eq!(top.parts(), 2);
        assert_eq!(top.not_fitted.len(), 1);
        assert_eq!(top.not_fitted[0].references, vec!["R4"]);
        assert_eq!(top.not_fitted[0].value, "4k7");
        assert_eq!(top.not_fitted[0].footprint, "R0603");
        let bottom = &bom.faces[1];
        assert_eq!(bottom.lines.len(), 2);
        assert_eq!(bottom.lines[0].references, vec!["R3"]);
        assert_eq!(bottom.lines[1].references, vec!["R10"]);
        assert!(bottom.not_fitted.is_empty());
        let whole = &bom.faces[2];
        assert_eq!(whole.parts(), 4);
        assert_eq!(bom.boards_per_panel, 4);
    }

    #[test]
    fn the_csv_has_the_quantities_of_a_board_and_of_a_panel_and_marks_the_parts_not_fitted() {
        let b = board();
        let parts = PartsFile::parse("match,fitted\nR4,no\n", 10).unwrap();
        let prices = PriceBook::default();
        let settings = CostSettings::default();
        let bom = build_assembly(&inputs(&b, &parts, &prices, &settings), 4).unwrap();
        let csv = assembly_csv(&bom.faces[0], bom.boards_per_panel);
        assert_eq!(
            csv,
            "references,quantity_board,quantity_panel,value,footprint,manufacturer,mpn,supplier,supplier_code,unit_price,quantity_to_buy,status\n\
             R1 R2,2,8,10k,R0603,,,,,,,fitted\n\
             R4,1,4,4k7,R0603,,,,,,,dnp\n"
        );
        // no panel: the quantity of the panel is the quantity of the board
        let single = assembly_csv(&bom.faces[0], 0);
        assert!(single.contains("R1 R2,2,2,10k"));
    }

    #[test]
    fn the_json_is_valid_and_has_a_section_for_each_face() {
        let b = board();
        let parts = PartsFile::default();
        let prices = PriceBook::default();
        let settings = CostSettings::default();
        let bom = build_assembly(&inputs(&b, &parts, &prices, &settings), 2).unwrap();
        let text = assembly_json(&bom);
        assert!(text.ends_with("}\n"));
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["format"], FORMAT);
        assert_eq!(value["boards_per_panel"], 2);
        assert_eq!(value["faces"].as_array().unwrap().len(), 3);
        assert_eq!(value["faces"][0]["face"], "top");
        assert_eq!(value["faces"][0]["parts_board"], 3);
        assert_eq!(value["faces"][0]["parts_panel"], 6);
        assert_eq!(value["faces"][1]["lines"][0]["references"][0], "R3");
        assert_eq!(text, assembly_json(&bom));
    }
}
