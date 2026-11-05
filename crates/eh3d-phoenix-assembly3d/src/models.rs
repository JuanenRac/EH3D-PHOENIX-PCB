// SPDX-License-Identifier: GPL-3.0-or-later
//! The table that says which model a footprint has: a CSV file with a header line and the
//! columns `footprint`, `model`, `dx_mm`, `dy_mm`, `dz_mm` and `rot_z_deg`.
//!
//! `model` is the name of a STEP file, relative to the folder of the table (no absolute path and
//! no `..`), or `box:LxWxH` in millimetres, a plain box that the user gives (centred on the
//! origin of the footprint in the board plane, standing on the board from height 0). `dx_mm`,
//! `dy_mm` and `dz_mm` move the model, and `rot_z_deg` turns it around the vertical axis of the
//! footprint before it is moved; it must be a multiple of 90, so that the box stays exact. Each
//! of the four may be left empty for 0.

use std::collections::BTreeMap;

use eh3d_phoenix_bom::text::parse_table;

use crate::codes::{AssemblyError, TABLE_INVALID};
use crate::step::decimal_to_nm;

/// Where the geometry of a model comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSource {
    /// A STEP file, by the name written in the table.
    File(String),
    /// A plain box given in the table: length along x, width along y and height, in nanometres.
    Box {
        /// Along x.
        length_nm: i64,
        /// Along y.
        width_nm: i64,
        /// Along z.
        height_nm: i64,
    },
}

/// The model of one footprint and how it sits on the footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntry {
    /// The geometry.
    pub source: ModelSource,
    /// The shift along x, in nanometres.
    pub dx_nm: i64,
    /// The shift along y, in nanometres.
    pub dy_nm: i64,
    /// The shift along z, in nanometres.
    pub dz_nm: i64,
    /// The turns of 90 degrees around the vertical axis, from 0 to 3.
    pub quarter_turns: u8,
}

/// The models by the name of the footprint.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelTable {
    /// The entries.
    pub entries: BTreeMap<String, ModelEntry>,
}

fn invalid(what: String) -> AssemblyError {
    AssemblyError::new(TABLE_INVALID, what)
}

/// Whether a name of a file is a plain relative path with no `..`.
pub fn safe_relative_name(name: &str) -> bool {
    if name.is_empty() || name.contains('\0') || name.starts_with(['/', '\\']) {
        return false;
    }
    let drive = name.as_bytes().get(1) == Some(&b':');
    if drive {
        return false;
    }
    name.split(['/', '\\'])
        .all(|part| !part.is_empty() && part != "..")
}

impl ModelTable {
    /// Reads the table, with no more than `max_rows` lines.
    pub fn parse(text: &str, max_rows: usize) -> Result<ModelTable, AssemblyError> {
        let table = parse_table(text).map_err(invalid)?;
        let footprint_col = table
            .column("footprint")
            .ok_or_else(|| invalid("the column footprint is missing".to_owned()))?;
        let model_col = table
            .column("model")
            .ok_or_else(|| invalid("the column model is missing".to_owned()))?;
        if table.rows.len() > max_rows {
            return Err(invalid(format!("more than {max_rows} lines")));
        }
        let mut entries = BTreeMap::new();
        for (line, cells) in &table.rows {
            let get = |index: Option<usize>| -> String {
                index
                    .and_then(|i| cells.get(i))
                    .cloned()
                    .unwrap_or_default()
            };
            let footprint = get(Some(footprint_col));
            if footprint.is_empty() {
                return Err(invalid(format!("line {line}: footprint is empty")));
            }
            let model = get(Some(model_col));
            let source = if let Some(size) = model.strip_prefix("box:") {
                let parts: Vec<&str> = size.split(['x', 'X']).collect();
                let [l, w, h] = parts.as_slice() else {
                    return Err(invalid(format!("line {line}: a box is box:LxWxH")));
                };
                let dim = |text: &str| {
                    decimal_to_nm(text, 1_000_000)
                        .filter(|v| *v > 0)
                        .ok_or_else(|| invalid(format!("line {line}: the size {text} of the box")))
                };
                ModelSource::Box {
                    length_nm: dim(l)?,
                    width_nm: dim(w)?,
                    height_nm: dim(h)?,
                }
            } else if safe_relative_name(&model) {
                ModelSource::File(model)
            } else {
                return Err(invalid(format!(
                    "line {line}: the model is a relative file name with no .. or box:LxWxH"
                )));
            };
            let shift = |name: &str| -> Result<i64, AssemblyError> {
                let text = get(table.column(name));
                if text.is_empty() {
                    return Ok(0);
                }
                decimal_to_nm(&text, 1_000_000)
                    .ok_or_else(|| invalid(format!("line {line}: {name} is not a number")))
            };
            let degrees = get(table.column("rot_z_deg"));
            let quarter_turns = if degrees.is_empty() {
                0
            } else {
                let value: i64 = degrees.parse().map_err(|_| {
                    invalid(format!("line {line}: rot_z_deg is not a whole number"))
                })?;
                if value % 90 != 0 {
                    return Err(invalid(format!(
                        "line {line}: rot_z_deg {value} is not a multiple of 90"
                    )));
                }
                u8::try_from((value / 90).rem_euclid(4)).unwrap_or(0)
            };
            let entry = ModelEntry {
                source,
                dx_nm: shift("dx_mm")?,
                dy_nm: shift("dy_mm")?,
                dz_nm: shift("dz_mm")?,
                quarter_turns,
            };
            if entries.insert(footprint.clone(), entry).is_some() {
                return Err(invalid(format!(
                    "line {line}: footprint {footprint} is given twice"
                )));
            }
        }
        Ok(ModelTable { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "footprint,model,dx_mm,dy_mm,dz_mm,rot_z_deg\n\
R0603,models/r0603.step,0.1,-0.2,0,90\n\
SOT23,box:2.9x1.3x1.1,,,0.05,\n\
DIP8,box:9.5X6.4X3.3,0,0,0,-90\n";

    #[test]
    fn the_table_is_read_line_by_line() {
        let t = ModelTable::parse(TABLE, 10).unwrap();
        assert_eq!(t.entries.len(), 3);
        let r = &t.entries["R0603"];
        assert_eq!(r.source, ModelSource::File("models/r0603.step".to_owned()));
        assert_eq!(
            (r.dx_nm, r.dy_nm, r.dz_nm, r.quarter_turns),
            (100_000, -200_000, 0, 1)
        );
        let s = &t.entries["SOT23"];
        assert_eq!(
            s.source,
            ModelSource::Box {
                length_nm: 2_900_000,
                width_nm: 1_300_000,
                height_nm: 1_100_000
            }
        );
        assert_eq!((s.dz_nm, s.quarter_turns), (50_000, 0));
        assert_eq!(t.entries["DIP8"].quarter_turns, 3);
    }

    #[test]
    fn what_is_wrong_is_refused_with_its_line() {
        for (text, needle) in [
            ("model\nx.step\n", "footprint is missing"),
            ("footprint\nR\n", "model is missing"),
            ("footprint,model\n,x.step\n", "footprint is empty"),
            ("footprint,model\nR,../x.step\n", "relative file name"),
            ("footprint,model\nR,/etc/x.step\n", "relative file name"),
            ("footprint,model\nR,C:x.step\n", "relative file name"),
            ("footprint,model\nR,box:1x2\n", "box:LxWxH"),
            ("footprint,model\nR,box:1x0x2\n", "the size 0"),
            (
                "footprint,model,rot_z_deg\nR,x.step,45\n",
                "not a multiple of 90",
            ),
            (
                "footprint,model,rot_z_deg\nR,x.step,a\n",
                "not a whole number",
            ),
            (
                "footprint,model,dx_mm\nR,x.step,a\n",
                "dx_mm is not a number",
            ),
            ("footprint,model\nR,x.step\nR,y.step\n", "given twice"),
        ] {
            let e = ModelTable::parse(text, 10).unwrap_err();
            assert_eq!(e.code, TABLE_INVALID, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
        }
        assert!(ModelTable::parse(TABLE, 2).is_err());
    }

    #[test]
    fn a_name_of_a_file_is_safe_only_when_it_stays_in_the_folder() {
        for good in ["a.step", "dir/a.step", "dir\\a.step", "a b.step"] {
            assert!(safe_relative_name(good), "{good}");
        }
        for bad in ["", "/a", "\\a", "C:\\a", "a/../b", "..", "a//b", "a\0b"] {
            assert!(!safe_relative_name(bad), "{bad:?}");
        }
    }
}
