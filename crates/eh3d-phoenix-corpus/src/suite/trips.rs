// SPDX-License-Identifier: GPL-3.0-or-later
//! Round trips between formats: a board is read from one format into the model, written in
//! another format, read back, and what the writer says it keeps is compared with the board that
//! comes back, by counts and by positions with a tolerance in nanometres.

use eh3d_phoenix_board::Board;
use eh3d_phoenix_eagle_board::{read_board, signature, write_board, ITEM_LEFT_OUT};
use eh3d_phoenix_export::Profile;
use eh3d_phoenix_gerber::{round_trip, ImportSettings};
use eh3d_phoenix_kicad::{read_pcb, write_pcb, KicadSettings, WriteSettings};

use super::codes::{SuiteError, CORPUS_INCONSISTENT};
use super::entry::Entry;
use super::facts::{compare, facts, id_source, Facts};
use super::outcome::Outcome;
use super::settings::CorpusSettings;

fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Reads the start of a chain: the extension of the data file says the format.
fn start_board(entry: &Entry, bytes: &[u8]) -> Result<Result<Board, Outcome>, SuiteError> {
    let mut ids = id_source(1);
    let extension = entry
        .input
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let refused = |code: Option<&str>, why: String| {
        let mut o = Outcome::refusal(code);
        o.notes.push(format!("the start of the chain: {why}"));
        Err(o)
    };
    Ok(match extension.as_str() {
        "brd" => match read_board(&text_of(bytes), &mut ids) {
            Ok(r) => Ok(r.board),
            Err(d) => refused(d.code.as_deref(), d.message),
        },
        "kicad_pcb" => match read_pcb(bytes, &KicadSettings::default(), &mut ids) {
            Ok(r) => Ok(r.board),
            Err(d) => refused(d.code.as_deref(), d.message),
        },
        "json" => match Board::from_bytes(bytes) {
            Ok(board) => Ok(board),
            Err(e) => refused(None, e.to_string()),
        },
        other => {
            return Err(SuiteError::new(
                CORPUS_INCONSISTENT,
                format!("{}: a round trip cannot start from .{other}", entry.id),
            ))
        }
    })
}

/// One step of the chain: writes the board in a format and reads it back. The counts the writer
/// declares are compared with what is read; what it cannot say is counted in the outcome.
fn step(
    format: &str,
    board: &Board,
    ids: &mut dyn FnMut() -> eh3d_phoenix_board::Id,
    o: &mut Outcome,
    signature_ok: &mut Option<bool>,
) -> Result<Board, String> {
    match format {
        "json" => {
            let bytes = board.to_bytes().map_err(|e| format!("{e:?}"))?;
            let back = Board::from_bytes(&bytes).map_err(|e| e.to_string())?;
            let again = back.to_bytes().map_err(|e| format!("{e:?}"))?;
            if again != bytes {
                o.notes.push(
                    "json: writing the board read back does not give the same bytes".to_owned(),
                );
                o.put("json_bytes_differ", 1usize);
            }
            Ok(back)
        }
        "eagle" => {
            let written = write_board(board);
            let left_out = written
                .diagnostics
                .iter()
                .filter(|d| d.code.as_deref() == Some(ITEM_LEFT_OUT))
                .count();
            let seen = o
                .measures
                .get("eagle_left_out")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            o.put("eagle_left_out", seen + left_out as u64);
            let read = read_board(&written.text, ids).map_err(|d| d.message)?;
            let same = signature(board) == signature(&read.board);
            *signature_ok = Some(signature_ok.unwrap_or(true) && same);
            Ok(read.board)
        }
        _ => {
            let out = write_pcb(board, &WriteSettings::default());
            if out.report.has_problem() {
                o.put("kicad_writer_problems", 1usize);
            }
            let file = out
                .files
                .first()
                .ok_or_else(|| "the KiCad writer gave no file".to_owned())?;
            let read = read_pcb(file.text.as_bytes(), &KicadSettings::default(), ids)
                .map_err(|d| d.message)?;
            let back = read.board;
            let pieces: usize = back.tracks.values().map(|t| t.path.len()).sum();
            let declared = [
                ("footprints", back.components.len()),
                ("pad nets", back.pad_nets.len()),
                ("track pieces", pieces),
                ("vias", back.vias.len()),
                ("zones", back.zones.len()),
                ("nets", back.nets.len()),
            ];
            let mut mismatches = 0usize;
            for (what, got) in declared {
                let wrote = out.report.written_count(what);
                if wrote != got {
                    mismatches += 1;
                    o.notes.push(format!(
                        "kicad: the writer says {wrote} {what}, the reader finds {got}"
                    ));
                }
            }
            let seen = o
                .measures
                .get("declared_mismatches")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            o.put("declared_mismatches", seen + mismatches as u64);
            Ok(back)
        }
    }
}

/// Runs a chain of writers and readers on a board and compares what comes back.
pub fn roundtrip(
    entry: &Entry,
    bytes: &[u8],
    settings: &CorpusSettings,
) -> Result<Outcome, SuiteError> {
    let start = match start_board(entry, bytes)? {
        Ok(board) => board,
        Err(refused) => return Ok(refused),
    };
    let tolerance = entry
        .tolerance_nm
        .unwrap_or(settings.round_trip_tolerance_nm);
    let before: Facts = facts(&start);
    let mut o = Outcome::default();
    let mut ids = id_source(5000);
    let mut signature_ok: Option<bool> = None;
    let mut current = start;
    for format in &entry.via {
        match step(format, &current, &mut ids, &mut o, &mut signature_ok) {
            Ok(next) => current = next,
            Err(why) => {
                let mut refused = Outcome::refusal(None);
                refused.notes.push(format!("{format}: {why}"));
                return Ok(refused);
            }
        }
    }
    let differences = compare(&before, &facts(&current), tolerance, &entry.compare);
    o.put("steps", entry.via.len());
    o.put("differences", differences.len());
    if let Some(same) = signature_ok {
        o.put("eagle_signature_identical", same);
    }
    o.notes.extend(differences);
    Ok(o)
}

/// Writes the Gerber and Excellon files of a board with the default profile, reads them back
/// with the import and compares them with the board.
pub fn gerber_roundtrip(entry: &Entry, bytes: &[u8]) -> Result<Outcome, SuiteError> {
    let board = match Board::from_bytes(bytes) {
        Ok(board) => board,
        Err(e) => {
            let mut o = Outcome::refusal(None);
            o.notes.push(format!("{}: {e}", entry.input));
            return Ok(o);
        }
    };
    let mut settings_text = String::new();
    for (key, value) in super::read::limits_of(
        entry,
        bytes.len() as u64,
        &["compare_tolerance_nm", "coordinate_limit_nm"],
    )? {
        settings_text.push_str(&format!("{key} = {value}\n"));
    }
    let settings = ImportSettings::parse(&settings_text)
        .map_err(|e| SuiteError::new(CORPUS_INCONSISTENT, format!("{}: {e}", entry.id)))?;
    let mut o = Outcome::default();
    match round_trip(&board, &Profile::default(), &settings) {
        Ok(trip) => {
            o.put("files", trip.files);
            o.put("differences", trip.differences.len());
            o.put("imported_failed", trip.imported.failed());
            for d in &trip.differences {
                o.notes.push(format!("{}: {}", d.file, d.what));
            }
            for finding in &trip.findings {
                o.code(finding.code);
            }
        }
        Err(e) => {
            o = Outcome::refusal(Some(e.code));
            o.notes.push(e.detail);
        }
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_PARTS: &str = r#"{
      "name": "t",
      "outline": [
        {"x_nm": 0, "y_nm": 0}, {"x_nm": 20000000, "y_nm": 0},
        {"x_nm": 20000000, "y_nm": 10000000}, {"x_nm": 0, "y_nm": 10000000}
      ],
      "layers": {
        "00000000-0000-7000-8000-000000000001": {"name": "Top", "kind": "copper"},
        "00000000-0000-7000-8000-000000000002": {"name": "Bottom", "kind": "copper"}
      },
      "copper_stack": ["00000000-0000-7000-8000-000000000001", "00000000-0000-7000-8000-000000000002"],
      "nets": {"00000000-0000-7000-8000-000000000003": {"name": "GND"}},
      "footprints": {
        "00000000-0000-7000-8000-000000000004": {"name": "F", "pads": {
          "00000000-0000-7000-8000-000000000005": {"number": "1",
            "shape": {"shape": "rect", "width": 1000000, "height": 1000000},
            "at": {"x_nm": 0, "y_nm": 0}, "rotation": 0, "layers": ["00000000-0000-7000-8000-000000000001"]}
        }}
      },
      "components": {
        "00000000-0000-7000-8000-000000000006": {"refdes": "R1", "value": "1k",
          "footprint": "00000000-0000-7000-8000-000000000004",
          "at": {"x_nm": 5000000, "y_nm": 5000000}, "rotation": 0, "side": "top"},
        "00000000-0000-7000-8000-000000000007": {"refdes": "R2", "value": "1k",
          "footprint": "00000000-0000-7000-8000-000000000004",
          "at": {"x_nm": 15000000, "y_nm": 5000000}, "rotation": 0, "side": "top"}
      },
      "pad_nets": [
        {"component": "00000000-0000-7000-8000-000000000006", "pad": "00000000-0000-7000-8000-000000000005", "net": "00000000-0000-7000-8000-000000000003"},
        {"component": "00000000-0000-7000-8000-000000000007", "pad": "00000000-0000-7000-8000-000000000005", "net": "00000000-0000-7000-8000-000000000003"}
      ],
      "tracks": {
        "00000000-0000-7000-8000-000000000008": {"net": "00000000-0000-7000-8000-000000000003",
          "layer": "00000000-0000-7000-8000-000000000001", "width": 250000,
          "path": [{"kind": "line", "start": {"x_nm": 5000000, "y_nm": 5000000}, "end": {"x_nm": 15000000, "y_nm": 5000000}}]}
      },
      "vias": {},
      "zones": {}
    }"#;

    fn entry(kind: &str, via: &str, input: &str) -> Entry {
        let via = if via.is_empty() {
            String::new()
        } else {
            format!(r#""via": {via},"#)
        };
        Entry::parse(&format!(
            r#"{{"format":"phoenix-corpus/1","id":"CORPUS-T-001","title":"t","kind":"{kind}",
            "input":"{input}",{via}"how":["by hand"],"expect":{{"outcome":"read"}}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn the_board_of_the_test_is_a_good_board() {
        let board = Board::from_bytes(TWO_PARTS.as_bytes()).unwrap();
        assert!(board.is_consistent(), "{:?}", board.validate());
        assert_eq!(board.components.len(), 2);
    }

    #[test]
    fn a_board_goes_through_json_eagle_and_kicad_without_losing_what_they_keep() {
        let settings = CorpusSettings::default();
        let all: Vec<String> = [
            "components",
            "placed_pads",
            "nets",
            "pad_nets",
            "track_pieces",
            "pad_points",
            "track_points",
            "outline_points",
            "refdes_values",
            "net_names",
            "pad_net_names",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        for via in [
            r#"["json"]"#,
            r#"["eagle"]"#,
            r#"["kicad"]"#,
            r#"["eagle","kicad"]"#,
        ] {
            let mut e = entry("roundtrip", via, "t.json");
            e.compare = all.clone();
            let o = roundtrip(&e, TWO_PARTS.as_bytes(), &settings).unwrap();
            assert!(!o.refused, "{via}: {:?}", o.notes);
            assert_eq!(
                o.measures["differences"],
                serde_json::Value::from(0),
                "{via}: {:?}",
                o.notes
            );
            assert_eq!(
                o.measures
                    .get("declared_mismatches")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                0
            );
        }
    }

    #[test]
    fn a_chain_that_cannot_start_is_refused_or_an_error() {
        let settings = CorpusSettings::default();
        let e = entry("roundtrip", r#"["json"]"#, "t.json");
        let o = roundtrip(&e, b"{", &settings).unwrap();
        assert!(o.refused);
        let e = entry("roundtrip", r#"["json"]"#, "t.txt");
        assert!(roundtrip(&e, b"x", &settings).is_err());
    }

    #[test]
    fn a_board_that_is_not_json_is_refused_by_the_file_round_trip() {
        let e = entry("gerber-roundtrip", "", "t.json");
        let refused = gerber_roundtrip(&e, b"[").unwrap();
        assert!(refused.refused);
        assert!(refused.codes.is_empty());
        assert!(!refused.notes.is_empty());
    }
}
