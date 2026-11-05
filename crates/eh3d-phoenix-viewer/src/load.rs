// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading a file for the viewer: an EAGLE board (`.brd`) or schematic (`.sch`), or the JSON of
//! the model of a board or of a schematic (`design/board.json`, `design/schematic.json`).

use std::collections::BTreeMap;
use std::path::Path;

use eh3d_phoenix_board::Board;
use eh3d_phoenix_eagle_board::{read_board, read_schematic};
use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, ObjectId, StepClock};
use eh3d_phoenix_schematic::Schematic;

use crate::codes::{
    ViewError, EAGLE_UNREADABLE, FILE_NOT_RECOGNISED, FILE_UNUSABLE, NOTHING_TO_DRAW,
};
use crate::scene::Scene;
use crate::settings::ViewerSettings;

/// What a file held.
#[derive(Debug, Clone, PartialEq)]
pub enum Document {
    /// A board.
    Board(Box<Board>),
    /// A schematic.
    Schematic(Box<Schematic>),
}

/// A document and how much of its file the reader could not use.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// The document.
    pub document: Document,
    /// The parts of an EAGLE file that were not read or were read in a simpler form.
    pub unread: usize,
}

/// Ids for the objects read from a file: the same file always gives the same ids.
pub fn id_source() -> Result<impl FnMut() -> ObjectId, ViewError> {
    let mut generator = IdGenerator::new(
        StepClock::new(1_700_000_000_000, 1),
        DeterministicRandom::new(1),
    );
    let mut last = generator
        .generate()
        .map_err(|e| ViewError::new(FILE_UNUSABLE, e.to_string()))?;
    Ok(move || {
        if let Ok(next) = generator.generate() {
            last = next;
        }
        last
    })
}

fn eagle_board(text: &str) -> Result<Loaded, ViewError> {
    let mut new_id = id_source()?;
    let read =
        read_board(text, &mut new_id).map_err(|d| ViewError::new(EAGLE_UNREADABLE, d.message))?;
    Ok(Loaded {
        document: Document::Board(Box::new(read.board)),
        unread: read.diagnostics.len(),
    })
}

fn eagle_schematic(text: &str) -> Result<Loaded, ViewError> {
    let mut new_id = id_source()?;
    let read = read_schematic(text, &BTreeMap::new(), &mut new_id)
        .map_err(|d| ViewError::new(EAGLE_UNREADABLE, d.message))?;
    Ok(Loaded {
        document: Document::Schematic(Box::new(read.schematic)),
        unread: read.diagnostics.len(),
    })
}

/// Reads the text of a file. `extension` is the extension of its name, in any case, without the
/// dot; the content decides when it says nothing.
pub fn parse_document(extension: &str, text: &str) -> Result<Loaded, ViewError> {
    match extension.to_ascii_lowercase().as_str() {
        "brd" => return eagle_board(text),
        "sch" => return eagle_schematic(text),
        _ => {}
    }
    if text.trim_start().starts_with('<') {
        return eagle_board(text)
            .or_else(|_| eagle_schematic(text))
            .map_err(|_| {
                ViewError::new(
                    FILE_NOT_RECOGNISED,
                    "an XML file without an EAGLE board or schematic",
                )
            });
    }
    if let Ok(board) = Board::from_bytes(text.as_bytes()) {
        return Ok(Loaded {
            document: Document::Board(Box::new(board)),
            unread: 0,
        });
    }
    if let Ok(schematic) = Schematic::from_bytes(text.as_bytes()) {
        return Ok(Loaded {
            document: Document::Schematic(Box::new(schematic)),
            unread: 0,
        });
    }
    Err(ViewError::new(
        FILE_NOT_RECOGNISED,
        "neither the JSON of a board nor the JSON of a schematic",
    ))
}

/// Reads a file, refusing one larger than the settings allow.
pub fn read_document(path: &Path, settings: &ViewerSettings) -> Result<Loaded, ViewError> {
    let size = std::fs::metadata(path)
        .map_err(|e| ViewError::new(FILE_UNUSABLE, e.to_string()))?
        .len();
    if size > settings.max_file_bytes {
        return Err(ViewError::new(
            FILE_UNUSABLE,
            format!(
                "the file is {size} bytes, the limit is {}",
                settings.max_file_bytes
            ),
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| ViewError::new(FILE_UNUSABLE, e.to_string()))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| ViewError::new(FILE_NOT_RECOGNISED, "the file is not UTF-8 text"))?;
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    parse_document(extension, &text)
}

/// Draws a document, and refuses one that has nothing to draw.
pub fn scene_of(document: &Document, settings: &ViewerSettings) -> Result<Scene, ViewError> {
    let scene = match document {
        Document::Board(board) => Scene::from_board(board, settings)?,
        Document::Schematic(schematic) => Scene::from_schematic(schematic, settings)?,
    };
    if scene.prims.is_empty() {
        return Err(ViewError::new(NOTHING_TO_DRAW, ""));
    }
    Ok(scene)
}

/// Reads and draws a file.
pub fn open(path: &Path, settings: &ViewerSettings) -> Result<(Scene, usize), ViewError> {
    let loaded = read_document(path, settings)?;
    let scene = scene_of(&loaded.document, settings)?;
    Ok((scene, loaded.unread))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;
    use crate::scene::SceneKind;
    use eh3d_phoenix_eagle_board::{write_board, write_schematic};

    fn temp_file(name: &str, content: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        (dir, path)
    }

    #[test]
    fn the_json_of_a_board_and_of_a_schematic_are_told_apart() {
        let board = sample::board();
        let bytes = board.to_bytes().unwrap();
        let loaded = parse_document("json", std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(loaded.document, Document::Board(Box::new(board)));
        let schematic = sample::schematic();
        let bytes = schematic.to_bytes().unwrap();
        let loaded = parse_document("JSON", std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(loaded.document, Document::Schematic(Box::new(schematic)));
    }

    #[test]
    fn something_else_is_refused_with_its_code() {
        for (ext, text) in [
            ("json", "{\"hello\": 1}"),
            ("txt", "plain words"),
            ("json", ""),
            ("xml", "<html></html>"),
        ] {
            let error = parse_document(ext, text).unwrap_err();
            assert_eq!(error.code, FILE_NOT_RECOGNISED, "{ext} {text}");
        }
        let error = parse_document("brd", "<html></html>").unwrap_err();
        assert_eq!(error.code, EAGLE_UNREADABLE);
        let error = parse_document("sch", "not xml at all").unwrap_err();
        assert_eq!(error.code, EAGLE_UNREADABLE);
    }

    #[test]
    fn an_eagle_board_written_by_this_code_is_read_and_drawn() {
        let board = sample::board();
        let written = write_board(&board);
        let loaded = parse_document("brd", &written.text).unwrap();
        let Document::Board(read) = &loaded.document else {
            panic!("not a board");
        };
        assert_eq!(read.tracks.len(), board.tracks.len());
        let scene = scene_of(&loaded.document, &ViewerSettings::default()).unwrap();
        assert_eq!(scene.kind, SceneKind::Board);
        assert!(!scene.prims.is_empty());
        // the content decides when the extension says nothing
        let again = parse_document("", &written.text).unwrap();
        assert!(matches!(again.document, Document::Board(_)));
    }

    #[test]
    fn an_eagle_schematic_written_by_this_code_is_read_and_drawn() {
        let board = sample::board();
        let schematic = sample::schematic();
        let written = write_schematic(&board, &schematic);
        let loaded = parse_document("sch", &written.text).unwrap();
        assert!(matches!(loaded.document, Document::Schematic(_)));
        let again = parse_document("xml", &written.text).unwrap();
        assert!(matches!(again.document, Document::Schematic(_)));
        let scene = scene_of(&loaded.document, &ViewerSettings::default()).unwrap();
        assert_eq!(scene.kind, SceneKind::Schematic);
    }

    #[test]
    fn a_file_is_read_from_its_path_and_a_missing_or_large_one_is_refused() {
        let board = sample::board();
        let (_dir, path) = temp_file("board.json", &board.to_bytes().unwrap());
        let (scene, unread) = open(&path, &ViewerSettings::default()).unwrap();
        assert_eq!(unread, 0);
        assert_eq!(scene.prims.len(), 15);
        let missing = path.with_file_name("nothing.json");
        assert_eq!(
            open(&missing, &ViewerSettings::default()).unwrap_err().code,
            FILE_UNUSABLE
        );
        let tiny = ViewerSettings::parse("max_file_bytes = 10").unwrap();
        assert_eq!(open(&path, &tiny).unwrap_err().code, FILE_UNUSABLE);
        let (_dir2, binary) = temp_file("binary.json", &[0xff, 0xfe, 0x00, 0x80]);
        assert_eq!(
            open(&binary, &ViewerSettings::default()).unwrap_err().code,
            FILE_NOT_RECOGNISED
        );
    }

    #[test]
    fn a_document_with_nothing_to_draw_is_refused() {
        let empty = Document::Board(Box::default());
        let error = scene_of(&empty, &ViewerSettings::default()).unwrap_err();
        assert_eq!(error.code, NOTHING_TO_DRAW);
        let empty = Document::Schematic(Box::default());
        assert_eq!(
            scene_of(&empty, &ViewerSettings::default())
                .unwrap_err()
                .code,
            NOTHING_TO_DRAW
        );
    }

    #[test]
    fn the_ids_of_a_file_are_the_same_every_time() {
        let mut first = id_source().unwrap();
        let mut second = id_source().unwrap();
        for _ in 0..5 {
            assert_eq!(first(), second());
        }
        assert_ne!(first(), first());
    }
}
