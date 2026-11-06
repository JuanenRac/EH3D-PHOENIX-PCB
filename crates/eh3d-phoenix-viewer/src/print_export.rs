// SPDX-License-Identifier: GPL-3.0-or-later
//! The export of the document that is open in the window to SVG and PDF, without a window: the
//! name proposed, the pages made from the document and the writing to a place that is new, with
//! the settings of `config/print.cfg` when that file exists.

use std::path::{Path, PathBuf};

use eh3d_phoenix_print::config::{read_file, DEFAULT_FILE};
use eh3d_phoenix_print::output::write_new;
use eh3d_phoenix_print::{render, Content, Format, PrintConfig, PrintError};
use eh3d_phoenix_rules::Lang;

use crate::print_scene::{board_contents, schematic_contents};
use crate::workspace::{Doc, Workspace};

/// The name of the document: the name of the file it was read from, else the name inside it.
pub fn project_name(w: &Workspace) -> String {
    w.input
        .as_deref()
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| match &w.doc {
            Doc::Board(session) => session.board.name.clone(),
            Doc::Schematic(session) => session.schematic.name.clone(),
        })
}

/// A name that is safe as the name of a file.
fn file_name(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "print".to_owned()
    } else {
        cleaned
    }
}

/// The pages of the document as it is now.
pub fn contents_of(w: &Workspace, cfg: &PrintConfig, lang: Lang) -> Vec<Content> {
    let project = project_name(w);
    match &w.doc {
        Doc::Board(session) => board_contents(&session.board, &w.viewer.scene, cfg, lang, &project),
        Doc::Schematic(session) => schematic_contents(
            &session.schematic,
            &w.viewer.scene,
            &w.viewer.settings,
            cfg,
            lang,
            &project,
        ),
    }
}

/// The place proposed: a PDF next to the file that was read, or a folder for the SVG files, which
/// are several when the document has several pages.
pub fn suggested_path(input: Option<&Path>, format: Format) -> PathBuf {
    let stem = input
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .map_or_else(|| "document".to_owned(), file_name);
    let name = match format {
        Format::Pdf => format!("{stem}_print.pdf"),
        Format::Svg => format!("{stem}_print"),
    };
    match input.and_then(|p| p.parent()) {
        Some(dir) => dir.join(name),
        None => PathBuf::from(name),
    }
}

/// The settings of the printing: the file `config/print.cfg` when it exists, else the defaults.
pub fn default_config() -> Result<PrintConfig, PrintError> {
    let file = Path::new(DEFAULT_FILE);
    if file.is_file() {
        read_file(file)
    } else {
        Ok(PrintConfig::default())
    }
}

/// Writes the document to a place that is new and returns the files.
pub fn export(
    w: &Workspace,
    out: &Path,
    format: Format,
    cfg: &PrintConfig,
    lang: Lang,
) -> Result<Vec<PathBuf>, PrintError> {
    let contents = contents_of(w, cfg, lang);
    let rendered = render(&file_name(&project_name(w)), &contents, format, cfg, lang)?;
    write_new(out, &rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{Document, Loaded};
    use crate::sample;
    use crate::settings::ViewerSettings;
    use eh3d_phoenix_print::pdfcheck::check_pdf;
    use eh3d_phoenix_route::RouteSettings;

    fn workspace(document: Document, input: Option<PathBuf>) -> Workspace {
        Workspace::new(
            Loaded {
                document,
                unread: 0,
            },
            input,
            ViewerSettings::default(),
            RouteSettings::default(),
        )
        .unwrap()
    }

    #[test]
    fn the_names_proposed_follow_the_file_and_the_format() {
        let input = Path::new("/work/my board.brd");
        assert_eq!(
            suggested_path(Some(input), Format::Pdf),
            Path::new("/work/my_board_print.pdf")
        );
        assert_eq!(
            suggested_path(Some(input), Format::Svg),
            Path::new("/work/my_board_print")
        );
        assert_eq!(
            suggested_path(None, Format::Pdf),
            Path::new("document_print.pdf")
        );
        assert_eq!(file_name(""), "print");
        assert_eq!(file_name("a b/c"), "a_b_c");
    }

    #[test]
    fn a_board_is_exported_to_a_pdf_that_passes_its_own_check() {
        let w = workspace(Document::Board(Box::new(sample::board())), None);
        assert_eq!(project_name(&w), "sample");
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("board.pdf");
        let cfg = PrintConfig::default();
        let written = export(&w, &out, Format::Pdf, &cfg, Lang::Pt).unwrap();
        assert_eq!(written, vec![out.clone()]);
        let info = check_pdf(&std::fs::read(&out).unwrap()).unwrap();
        assert_eq!(info.pages, 3);
        // a file that exists is never written over
        let error = export(&w, &out, Format::Pdf, &cfg, Lang::En).unwrap_err();
        assert_eq!(error.code, eh3d_phoenix_print::codes::OUTPUT_REFUSED);
    }

    #[test]
    fn a_schematic_is_exported_to_a_folder_of_svg_files() {
        let w = workspace(
            Document::Schematic(Box::new(sample::schematic())),
            Some(PathBuf::from("demo.json")),
        );
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("sheets");
        let written = export(&w, &out, Format::Svg, &PrintConfig::default(), Lang::En).unwrap();
        let names: Vec<String> = written
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["sheet-01.svg", "sheet-02.svg"]);
        let text = std::fs::read_to_string(&written[0]).unwrap();
        assert!(text.starts_with("<?xml"));
        assert!(text.contains("Schematic - Sheet 1"));
    }
}
