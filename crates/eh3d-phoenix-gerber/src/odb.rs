// SPDX-License-Identifier: GPL-3.0-or-later
//! A basic reader of the matrix file of an ODB++ job: the steps and the layers of the job, in
//! order, with the kind, the context and the polarity of each layer.
//!
//! An ODB++ job is a folder (`matrix/matrix`, `steps/<step>/layers/<layer>/features`, ...) that
//! is often sent as a compressed archive. This module reads only the text of `matrix/matrix`,
//! which is a list of blocks `STEP { ... }` and `LAYER { ... }` of lines `KEY=VALUE`, and it
//! reads it strictly: whatever it does not know it refuses, with the code `PHOENIX-GBR-020` and
//! the line, and does not guess.
//!
//! Refused: a compressed file or an archive (the files of the job are not unpacked here); a file
//! that is not text; a file larger than the limit or with more lines, layers or steps than the
//! limits; a block other than `STEP` and `LAYER`, a block inside a block, a line outside a
//! block and a block that is not closed; a layer with no row, with a row that is not a positive
//! whole number or that is repeated, with no name or with a name that is repeated (in any case),
//! with a context other than `BOARD` and `MISC`, with a kind that is not in the list of
//! [`OdbLayerKind`] and with a polarity other than `POSITIVE` and `NEGATIVE`; and a job with no
//! layer. The keys that are not named here (colour, old name, the start and the end of a drill
//! span, the type of a dielectric) are skipped.
//!
//! Not read: the features of the layers, the components, the nets, the stack-up of the job and
//! the other files of the folder. Nothing is written and nothing is merged into a board.

use crate::codes::{GbrError, INPUT_TOO_LARGE, LIMIT_EXCEEDED, ODB_UNSUPPORTED};
use crate::identify::Role;

/// The limits of the reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OdbLimits {
    /// The most bytes of the file.
    pub max_bytes: usize,
    /// The most lines of the file.
    pub max_lines: usize,
    /// The most characters of a line.
    pub max_line_chars: usize,
    /// The most layers.
    pub max_layers: usize,
    /// The most steps.
    pub max_steps: usize,
}

impl Default for OdbLimits {
    fn default() -> OdbLimits {
        OdbLimits {
            max_bytes: 1 << 20,
            max_lines: 100_000,
            max_line_chars: 1024,
            max_layers: 256,
            max_steps: 64,
        }
    }
}

/// Whether a layer belongs to the board or is only for the job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OdbContext {
    /// A layer of the board.
    Board,
    /// A layer that is not part of the board (a drawing, a coupon).
    Misc,
}

/// The kind of a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OdbLayerKind {
    /// A signal copper layer.
    Signal,
    /// A power or ground plane.
    PowerGround,
    /// A copper layer with signals and planes.
    Mixed,
    /// A solder mask.
    SolderMask,
    /// A solder paste.
    SolderPaste,
    /// A legend (silkscreen).
    SilkScreen,
    /// Holes.
    Drill,
    /// A route (an outline or a slot).
    Rout,
    /// A drawing.
    Document,
    /// The components of a side.
    Component,
    /// A core or a prepreg.
    Dielectric,
}

impl OdbLayerKind {
    /// The kind of a word of the file (in any case), if it is known.
    pub fn from_word(word: &str) -> Option<OdbLayerKind> {
        Some(match word.to_ascii_uppercase().as_str() {
            "SIGNAL" => OdbLayerKind::Signal,
            "POWER_GROUND" => OdbLayerKind::PowerGround,
            "MIXED" => OdbLayerKind::Mixed,
            "SOLDER_MASK" => OdbLayerKind::SolderMask,
            "SOLDER_PASTE" => OdbLayerKind::SolderPaste,
            "SILK_SCREEN" => OdbLayerKind::SilkScreen,
            "DRILL" => OdbLayerKind::Drill,
            "ROUT" => OdbLayerKind::Rout,
            "DOCUMENT" => OdbLayerKind::Document,
            "COMPONENT" => OdbLayerKind::Component,
            "DIELECTRIC" => OdbLayerKind::Dielectric,
            _ => return None,
        })
    }

    /// True for the kinds that are copper.
    pub fn is_copper(self) -> bool {
        matches!(
            self,
            OdbLayerKind::Signal | OdbLayerKind::PowerGround | OdbLayerKind::Mixed
        )
    }
}

/// A layer of the matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OdbLayer {
    /// The row of the layer, from 1: its place in the order of the job.
    pub row: u32,
    /// The name of the layer.
    pub name: String,
    /// The context.
    pub context: OdbContext,
    /// The kind.
    pub kind: OdbLayerKind,
    /// True for a positive layer (the default), false for a negative one.
    pub positive: bool,
}

/// The matrix of a job.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OdbMatrix {
    /// The names of the steps, in the order of the file.
    pub steps: Vec<String>,
    /// The layers, by increasing row.
    pub layers: Vec<OdbLayer>,
}

impl OdbMatrix {
    /// The copper layers of the board, from the top to the bottom.
    pub fn copper_layers(&self) -> Vec<&OdbLayer> {
        self.layers
            .iter()
            .filter(|l| l.context == OdbContext::Board && l.kind.is_copper())
            .collect()
    }

    /// The role that each layer of the board has in the files of the suite, told from its kind
    /// and its place: the first copper layer is the top, the last the bottom and the ones
    /// between are inner layers counted from 1; a mask, a paste or a legend above the first
    /// copper layer is of the top and below the last one of the bottom; every drill is `Drill`
    /// (the matrix does not say whether it is plated). A layer that has no role (a route, a
    /// drawing, the components, a dielectric, a layer outside the board, or a mask, a paste or
    /// a legend of a board with no copper) has `None`. The order is that of the layers.
    pub fn roles(&self) -> Vec<(String, Option<Role>)> {
        let copper = self.copper_layers();
        let first = copper.first().map(|l| l.row);
        self.layers
            .iter()
            .map(|layer| {
                let role = if layer.context != OdbContext::Board {
                    None
                } else if layer.kind.is_copper() {
                    let index = copper.iter().position(|c| c.row == layer.row).unwrap_or(0);
                    if copper.len() < 2 || index == 0 {
                        Some(Role::CopperTop)
                    } else if index + 1 == copper.len() {
                        Some(Role::CopperBottom)
                    } else {
                        Some(Role::CopperInner(index as u32))
                    }
                } else {
                    match (layer.kind, first) {
                        (OdbLayerKind::Drill, _) => Some(Role::Drill),
                        (OdbLayerKind::SolderMask, Some(f)) => Some(if layer.row < f {
                            Role::MaskTop
                        } else {
                            Role::MaskBottom
                        }),
                        (OdbLayerKind::SolderPaste, Some(f)) => Some(if layer.row < f {
                            Role::PasteTop
                        } else {
                            Role::PasteBottom
                        }),
                        (OdbLayerKind::SilkScreen, Some(f)) => Some(if layer.row < f {
                            Role::LegendTop
                        } else {
                            Role::LegendBottom
                        }),
                        _ => None,
                    }
                };
                (layer.name.clone(), role)
            })
            .collect()
    }
}

fn refuse(line: usize, what: impl Into<String>) -> GbrError {
    GbrError::at(ODB_UNSUPPORTED, line, what)
}

/// The fields of a block of layer being read.
#[derive(Default)]
struct Pending {
    row: Option<u32>,
    name: Option<String>,
    context: Option<OdbContext>,
    kind: Option<OdbLayerKind>,
    positive: Option<bool>,
}

fn magic_refusal(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        Some("a gzip file is not read; give the matrix file of the unpacked job")
    } else if bytes.starts_with(b"PK\x03\x04") {
        Some("a zip file is not read; give the matrix file of the unpacked job")
    } else if bytes.len() > 262 && &bytes[257..262] == b"ustar" {
        Some("a tar file is not read; give the matrix file of the unpacked job")
    } else if bytes.starts_with(b"BZh") || bytes.starts_with(&[0xfd, b'7', b'z', b'X', b'Z']) {
        Some("a compressed file is not read; give the matrix file of the unpacked job")
    } else {
        None
    }
}

/// Reads the matrix file of an ODB++ job.
pub fn parse_matrix(bytes: &[u8], limits: &OdbLimits) -> Result<OdbMatrix, GbrError> {
    if bytes.len() > limits.max_bytes {
        return Err(GbrError::new(
            INPUT_TOO_LARGE,
            format!("the matrix has more than {} bytes", limits.max_bytes),
        ));
    }
    if let Some(what) = magic_refusal(bytes) {
        return Err(refuse(0, what));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| refuse(0, "the file is not UTF-8 text"))?;
    if text
        .chars()
        .any(|c| c.is_control() && !"\t\r\n".contains(c))
    {
        return Err(refuse(0, "the file holds a control character"));
    }
    let mut matrix = OdbMatrix::default();
    // The block being read: its name, the line where it opened and its fields.
    let mut open: Option<(String, usize, Pending)> = None;
    let mut count = 0usize;
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        count += 1;
        if count > limits.max_lines {
            return Err(GbrError::at(
                LIMIT_EXCEEDED,
                number,
                format!("more than {} lines", limits.max_lines),
            ));
        }
        if raw.chars().count() > limits.max_line_chars {
            return Err(GbrError::at(
                LIMIT_EXCEEDED,
                number,
                format!("a line of more than {} characters", limits.max_line_chars),
            ));
        }
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(head) = line.strip_suffix('{') {
            let name = head.trim().to_ascii_uppercase();
            if open.is_some() {
                return Err(refuse(number, "a block inside a block"));
            }
            if name != "STEP" && name != "LAYER" {
                return Err(refuse(
                    number,
                    format!("the block {} is not read", head.trim()),
                ));
            }
            open = Some((name, number, Pending::default()));
        } else if line == "}" {
            let Some((name, _, pending)) = open.take() else {
                return Err(refuse(number, "a closing brace with no block"));
            };
            close_block(&mut matrix, &name, pending, number, limits)?;
        } else if let Some((_, _, pending)) = open.as_mut() {
            let Some((key, value)) = line.split_once('=') else {
                return Err(refuse(number, "a line that is not KEY=VALUE"));
            };
            read_field(pending, key.trim(), value.trim(), number)?;
        } else {
            return Err(refuse(number, "a line outside a block"));
        }
    }
    if let Some((name, opened, _)) = open {
        return Err(refuse(
            opened,
            format!("the block {name} is not closed before the end of the file"),
        ));
    }
    if matrix.layers.is_empty() {
        return Err(refuse(0, "the matrix has no layer"));
    }
    matrix.layers.sort_by_key(|l| l.row);
    Ok(matrix)
}

fn read_field(pending: &mut Pending, key: &str, value: &str, line: usize) -> Result<(), GbrError> {
    match key.to_ascii_uppercase().as_str() {
        "ROW" => {
            let row = value
                .parse::<u32>()
                .ok()
                .filter(|r| *r >= 1)
                .ok_or_else(|| refuse(line, "ROW is not a positive whole number"))?;
            pending.row = Some(row);
        }
        "NAME" => {
            if value.is_empty() {
                return Err(refuse(line, "NAME is empty"));
            }
            pending.name = Some(value.to_owned());
        }
        "CONTEXT" => {
            pending.context = Some(match value.to_ascii_uppercase().as_str() {
                "BOARD" => OdbContext::Board,
                "MISC" => OdbContext::Misc,
                _ => {
                    return Err(refuse(
                        line,
                        format!("the context {value} is not BOARD or MISC"),
                    ))
                }
            });
        }
        "TYPE" => {
            pending.kind = Some(
                OdbLayerKind::from_word(value)
                    .ok_or_else(|| refuse(line, format!("the layer type {value} is not read")))?,
            );
        }
        "POLARITY" => {
            pending.positive = Some(match value.to_ascii_uppercase().as_str() {
                "POSITIVE" => true,
                "NEGATIVE" => false,
                _ => {
                    return Err(refuse(
                        line,
                        format!("the polarity {value} is not POSITIVE or NEGATIVE"),
                    ))
                }
            });
        }
        _ => {}
    }
    Ok(())
}

fn close_block(
    matrix: &mut OdbMatrix,
    name: &str,
    pending: Pending,
    line: usize,
    limits: &OdbLimits,
) -> Result<(), GbrError> {
    if name == "STEP" {
        let step = pending
            .name
            .ok_or_else(|| refuse(line, "a STEP has no NAME"))?;
        if matrix.steps.len() >= limits.max_steps {
            return Err(GbrError::at(
                LIMIT_EXCEEDED,
                line,
                format!("more than {} steps", limits.max_steps),
            ));
        }
        matrix.steps.push(step);
        return Ok(());
    }
    if matrix.layers.len() >= limits.max_layers {
        return Err(GbrError::at(
            LIMIT_EXCEEDED,
            line,
            format!("more than {} layers", limits.max_layers),
        ));
    }
    let row = pending
        .row
        .ok_or_else(|| refuse(line, "a LAYER has no ROW"))?;
    let layer_name = pending
        .name
        .ok_or_else(|| refuse(line, "a LAYER has no NAME"))?;
    let kind = pending
        .kind
        .ok_or_else(|| refuse(line, "a LAYER has no TYPE"))?;
    if matrix.layers.iter().any(|l| l.row == row) {
        return Err(refuse(line, format!("the row {row} is given twice")));
    }
    if matrix
        .layers
        .iter()
        .any(|l| l.name.eq_ignore_ascii_case(&layer_name))
    {
        return Err(refuse(
            line,
            format!("the layer name {layer_name} is given twice"),
        ));
    }
    matrix.layers.push(OdbLayer {
        row,
        name: layer_name,
        context: pending.context.unwrap_or(OdbContext::Board),
        kind,
        positive: pending.positive.unwrap_or(true),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(row: u32, name: &str, kind: &str) -> String {
        format!("LAYER {{\n  ROW={row}\n  CONTEXT=BOARD\n  TYPE={kind}\n  NAME={name}\n  POLARITY=POSITIVE\n}}\n")
    }

    fn four_layers() -> String {
        let mut text = String::from("STEP {\n  COL=1\n  NAME=pcb\n}\n");
        for (row, name, kind) in [
            (1, "silk_top", "SILK_SCREEN"),
            (2, "paste_top", "SOLDER_PASTE"),
            (3, "mask_top", "SOLDER_MASK"),
            (4, "top", "SIGNAL"),
            (5, "gnd", "POWER_GROUND"),
            (6, "pwr", "POWER_GROUND"),
            (7, "bottom", "SIGNAL"),
            (8, "mask_bot", "SOLDER_MASK"),
            (9, "paste_bot", "SOLDER_PASTE"),
            (10, "silk_bot", "SILK_SCREEN"),
            (11, "drill", "DRILL"),
            (12, "outline", "ROUT"),
        ] {
            text.push_str(&layer(row, name, kind));
        }
        text
    }

    fn refused(text: &str) -> GbrError {
        parse_matrix(text.as_bytes(), &OdbLimits::default()).unwrap_err()
    }

    #[test]
    fn a_matrix_is_read_with_its_steps_and_layers_in_order_of_row() {
        let m = parse_matrix(four_layers().as_bytes(), &OdbLimits::default()).unwrap();
        assert_eq!(m.steps, vec!["pcb".to_owned()]);
        assert_eq!(m.layers.len(), 12);
        assert_eq!(m.layers[3].name, "top");
        assert_eq!(m.layers[3].kind, OdbLayerKind::Signal);
        assert!(m.layers.iter().all(|l| l.positive));
        assert_eq!(m.copper_layers().len(), 4);
    }

    #[test]
    fn rows_given_out_of_order_are_sorted() {
        let text = format!("{}{}", layer(2, "b", "SIGNAL"), layer(1, "a", "SIGNAL"));
        let m = parse_matrix(text.as_bytes(), &OdbLimits::default()).unwrap();
        assert_eq!(m.layers[0].name, "a");
        assert_eq!(m.layers[1].name, "b");
    }

    #[test]
    fn the_roles_are_told_from_the_kind_and_the_place() {
        let m = parse_matrix(four_layers().as_bytes(), &OdbLimits::default()).unwrap();
        let roles = m.roles();
        let role = |name: &str| roles.iter().find(|(n, _)| n == name).unwrap().1;
        assert_eq!(role("top"), Some(Role::CopperTop));
        assert_eq!(role("gnd"), Some(Role::CopperInner(1)));
        assert_eq!(role("pwr"), Some(Role::CopperInner(2)));
        assert_eq!(role("bottom"), Some(Role::CopperBottom));
        assert_eq!(role("mask_top"), Some(Role::MaskTop));
        assert_eq!(role("mask_bot"), Some(Role::MaskBottom));
        assert_eq!(role("paste_top"), Some(Role::PasteTop));
        assert_eq!(role("paste_bot"), Some(Role::PasteBottom));
        assert_eq!(role("silk_top"), Some(Role::LegendTop));
        assert_eq!(role("silk_bot"), Some(Role::LegendBottom));
        assert_eq!(role("drill"), Some(Role::Drill));
        assert_eq!(role("outline"), None);
    }

    #[test]
    fn a_layer_outside_the_board_has_no_role() {
        let text = "LAYER {\n ROW=1\n CONTEXT=MISC\n TYPE=SIGNAL\n NAME=coupon\n}\nLAYER {\n ROW=2\n TYPE=SIGNAL\n NAME=top\n POLARITY=NEGATIVE\n}\n";
        let m = parse_matrix(text.as_bytes(), &OdbLimits::default()).unwrap();
        assert_eq!(m.roles()[0].1, None);
        assert_eq!(m.roles()[1].1, Some(Role::CopperTop));
        assert!(!m.layers[1].positive);
        assert_eq!(m.copper_layers().len(), 1);
    }

    #[test]
    fn keys_that_are_not_named_are_skipped_and_the_words_are_in_any_case() {
        let text =
            "layer {\n row = 1\n color = 0x00ff00\n type = signal\n name = top\n old_name =\n}\n";
        let m = parse_matrix(text.as_bytes(), &OdbLimits::default()).unwrap();
        assert_eq!(m.layers[0].kind, OdbLayerKind::Signal);
    }

    #[test]
    fn archives_and_files_that_are_not_text_are_refused() {
        let limits = OdbLimits::default();
        for bytes in [
            &[0x1f_u8, 0x8b, 8, 0][..],
            b"PK\x03\x04rest",
            b"BZh91AY",
            &[0xfd, b'7', b'z', b'X', b'Z', 0],
            &[0xff, 0xfe, 0x41],
            b"LAYER {\n\0\n}\n",
        ] {
            let e = parse_matrix(bytes, &limits).unwrap_err();
            assert_eq!(e.code, ODB_UNSUPPORTED, "{bytes:?}");
        }
        let mut tar = vec![b'a'; 300];
        tar[257..262].copy_from_slice(b"ustar");
        assert_eq!(
            parse_matrix(&tar, &limits).unwrap_err().code,
            ODB_UNSUPPORTED
        );
    }

    #[test]
    fn what_the_reader_does_not_know_is_refused_with_its_line() {
        for (text, line, word) in [
            ("ARRAY {\n}\n", 1, "ARRAY"),
            ("LAYER {\n LAYER {\n", 2, "inside"),
            ("ROW=1\n", 1, "outside"),
            ("}\n", 1, "closing"),
            ("LAYER {\n ROW 1\n}\n", 2, "KEY=VALUE"),
            ("LAYER {\n ROW=0\n}\n", 2, "ROW"),
            ("LAYER {\n ROW=x\n}\n", 2, "ROW"),
            ("LAYER {\n NAME=\n}\n", 2, "NAME"),
            ("LAYER {\n CONTEXT=SOME\n}\n", 2, "SOME"),
            ("LAYER {\n TYPE=FLEX\n}\n", 2, "FLEX"),
            ("LAYER {\n POLARITY=UP\n}\n", 2, "UP"),
            ("LAYER {\n ROW=1\n", 1, "not closed"),
        ] {
            let e = refused(text);
            assert_eq!(e.code, ODB_UNSUPPORTED, "{text}");
            assert_eq!(e.line, line, "{text}: {e}");
            assert!(e.detail.contains(word), "{text}: {e}");
        }
    }

    #[test]
    fn a_layer_must_have_a_row_a_name_and_a_type_and_no_repeats() {
        let ok = layer(1, "a", "SIGNAL");
        for (text, word) in [
            ("LAYER {\n NAME=a\n TYPE=SIGNAL\n}\n".to_owned(), "ROW"),
            ("LAYER {\n ROW=1\n TYPE=SIGNAL\n}\n".to_owned(), "NAME"),
            ("LAYER {\n ROW=1\n NAME=a\n}\n".to_owned(), "TYPE"),
            (format!("{ok}{}", layer(1, "b", "SIGNAL")), "row 1"),
            (format!("{ok}{}", layer(2, "A", "SIGNAL")), "name A"),
            ("STEP {\n COL=1\n}\n".to_owned(), "NAME"),
            ("STEP {\n NAME=a\n}\n".to_owned(), "no layer"),
        ] {
            let e = refused(&text);
            assert_eq!(e.code, ODB_UNSUPPORTED, "{text}");
            assert!(e.detail.contains(word), "{text}: {e}");
        }
    }

    #[test]
    fn the_limits_are_kept() {
        let text = four_layers();
        let small = |f: fn(&mut OdbLimits)| {
            let mut l = OdbLimits::default();
            f(&mut l);
            parse_matrix(text.as_bytes(), &l).unwrap_err().code
        };
        assert_eq!(small(|l| l.max_bytes = 10), INPUT_TOO_LARGE);
        assert_eq!(small(|l| l.max_lines = 5), LIMIT_EXCEEDED);
        assert_eq!(small(|l| l.max_line_chars = 4), LIMIT_EXCEEDED);
        assert_eq!(small(|l| l.max_layers = 3), LIMIT_EXCEEDED);
        assert_eq!(small(|l| l.max_steps = 0), LIMIT_EXCEEDED);
    }

    #[test]
    fn a_dielectric_is_read_and_is_not_copper() {
        let text = format!(
            "{}{}{}",
            layer(1, "top", "SIGNAL"),
            layer(2, "core", "DIELECTRIC"),
            layer(3, "bottom", "SIGNAL")
        );
        let m = parse_matrix(text.as_bytes(), &OdbLimits::default()).unwrap();
        assert_eq!(m.layers[1].kind, OdbLayerKind::Dielectric);
        assert_eq!(m.copper_layers().len(), 2);
        assert_eq!(m.roles()[1].1, None);
    }
}
