// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `view`: a viewer and editor of a board or a schematic. With the `gui` feature it
//! opens a window; without it the command explains how to build the window, and `--summary` and
//! `--bench` work on any build.
//!
//! Exit codes: 0 done, 2 the arguments, the settings or the input are unusable (or the window
//! was asked for in a build without it).

use std::io::Write;
use std::path::Path;
#[cfg(feature = "gui")]
use std::path::PathBuf;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_viewer::bench;
use eh3d_phoenix_viewer::codes::{ViewError, SETTINGS_INVALID};
use eh3d_phoenix_viewer::{open, Viewer, ViewerSettings};

pub(crate) const USAGE: &str = "\
phoenix view [--settings FILE] [--lang CODE] [<board.brd | schematic.sch | board.json>]
phoenix view --summary [--settings FILE] [--lang CODE] <file>
phoenix view --bench [--settings FILE] [--lang CODE]

  view          a viewer of a board or a schematic: layers with a colour each, zoom, pan, fit,
                a net highlighted by a click, a measure between two points and the position of
                the pointer in millimetres; in the window a board is also edited (move, turn,
                delete, vertices, widths, vias and routing, with undo and redo and the findings
                of the rules) and a schematic is checked and synchronised with a board, always
                saved under another name. The window needs a build with the gui feature
                (cargo run --release -p eh3d-phoenix-cli --features gui -- view FILE); it is a
                prototype that has not been seen on a screen
  --summary     print what the file holds (primitives, items, nets, primitives per layer)
  --bench       time the preparation of the primitives and the selection by position on three
                synthetic boards (the sizes come from the settings); drawing on a screen is
                not measured
  --settings    a settings file (config/viewer.cfg shows every key); --lang is one of en, es, de,
                fr, it, ja, zh, pt
";

struct Options {
    settings: Option<String>,
    lang: Lang,
    bench: bool,
    summary: bool,
    file: Option<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, String> {
    let mut options = Options {
        settings: None,
        lang: Lang::En,
        bench: false,
        summary: false,
        file: None,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match *arg {
            "--bench" => options.bench = true,
            "--summary" => options.summary = true,
            "--settings" => {
                let path = iter.next().ok_or("--settings needs a file")?;
                options.settings = Some((*path).to_owned());
            }
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                options.lang = Lang::from_tag(tag).ok_or("unknown language code")?;
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => {
                if options.file.is_some() {
                    return Err("only one file can be given".to_owned());
                }
                options.file = Some(other.to_owned());
            }
        }
    }
    if options.bench && (options.file.is_some() || options.summary) {
        return Err("--bench takes no file and cannot be combined with --summary".to_owned());
    }
    if options.summary && options.file.is_none() {
        return Err("--summary needs a file".to_owned());
    }
    Ok(options)
}

fn fail(err: &mut dyn Write, lang: Lang, error: &ViewError) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

fn load_settings(path: Option<&str>) -> Result<ViewerSettings, ViewError> {
    match path {
        None => Ok(ViewerSettings::default()),
        Some(path) => {
            let text = crate::check::read_text(path)
                .map_err(|message| ViewError::new(SETTINGS_INVALID, message))?;
            ViewerSettings::parse(&text)
        }
    }
}

fn window(file: Option<&str>, lang: Lang, settings: ViewerSettings, err: &mut dyn Write) -> i32 {
    #[cfg(feature = "gui")]
    {
        match eh3d_phoenix_viewer::gui::run(file.map(PathBuf::from), lang, settings) {
            Ok(()) => 0,
            Err(error) => fail(err, lang, &error),
        }
    }
    #[cfg(not(feature = "gui"))]
    {
        let _ = (file, settings);
        let error = ViewError::new(
            eh3d_phoenix_viewer::codes::WINDOW_NOT_BUILT,
            "build the program with the gui feature: cargo run --release -p eh3d-phoenix-cli \
             --features gui -- view FILE (--summary and --bench work without it)",
        );
        fail(err, lang, &error)
    }
}

pub(crate) fn view(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let lang = options.lang;
    let settings = match load_settings(options.settings.as_deref()) {
        Ok(settings) => settings,
        Err(error) => return fail(err, lang, &error),
    };
    if options.bench {
        return match bench::run(&settings) {
            Ok(rows) => {
                let _ = out.write_all(bench::report(&rows, lang).as_bytes());
                0
            }
            Err(error) => fail(err, lang, &error),
        };
    }
    if let (true, Some(file)) = (options.summary, options.file.as_deref()) {
        return match open(Path::new(file), &settings) {
            Ok((scene, unread)) => {
                let viewer = Viewer::new(scene, settings);
                let _ = out.write_all(viewer.summary(lang, unread).as_bytes());
                0
            }
            Err(error) => fail(err, lang, &error),
        };
    }
    window(options.file.as_deref(), lang, settings, err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_viewer::bench::synthetic_board;

    fn run_view(args: &[&str]) -> (i32, String, String) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = view(args, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    fn settings_file(dir: &Path) -> String {
        let path = dir.join("viewer.cfg");
        std::fs::write(
            &path,
            "bench_small = 100\nbench_medium = 300\nbench_large = 900\nbench_picks = 40\nbench_linear_picks = 10\n",
        )
        .unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn the_benchmark_prints_three_boards_and_the_note_that_drawing_is_not_measured() {
        let dir = tempfile::tempdir().unwrap();
        let settings = settings_file(dir.path());
        let (code, out, err) = run_view(&["--bench", "--settings", &settings]);
        assert_eq!(code, 0, "{err}");
        assert!(out.contains("small") && out.contains("medium") && out.contains("large"));
        assert!(out.contains("Drawing on a screen was not measured"));
        let (code, out, _) = run_view(&["--bench", "--settings", &settings, "--lang", "pt"]);
        assert_eq!(code, 0);
        assert!(out.contains("O desenho no ecrã não foi medido"));
    }

    #[test]
    fn the_summary_describes_a_board_file() {
        let dir = tempfile::tempdir().unwrap();
        let settings = ViewerSettings::parse("bench_seed = 3").unwrap();
        let board = synthetic_board(200, &settings).unwrap();
        let file = dir.path().join("board.json");
        std::fs::write(&file, board.to_bytes().unwrap()).unwrap();
        let name = file.to_string_lossy().into_owned();
        let (code, out, err) = run_view(&["--summary", &name]);
        assert_eq!(code, 0, "{err}");
        assert!(out.starts_with("Board: 200 primitives"), "{out}");
        assert!(out.contains("  Top: "));
        let (code, out, _) = run_view(&["--summary", "--lang", "de", &name]);
        assert_eq!(code, 0);
        assert!(out.starts_with("Platine: 200 Primitive"), "{out}");
    }

    #[test]
    fn a_file_that_cannot_be_used_is_refused_with_its_code_in_the_language() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir
            .path()
            .join("nothing.brd")
            .to_string_lossy()
            .into_owned();
        let (code, out, err) = run_view(&["--summary", &missing]);
        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert!(err.contains("PHOENIX-VIEW-001"), "{err}");
        let (code, _, err) = run_view(&["--summary", "--lang", "es", &missing]);
        assert_eq!(code, 2);
        assert!(err.contains("No se puede leer el archivo"), "{err}");
    }

    #[test]
    fn bad_arguments_and_bad_settings_are_exit_code_two() {
        for args in [
            vec!["--bench", "file.brd"],
            vec!["--bench", "--summary"],
            vec!["--summary"],
            vec!["--nonsense"],
            vec!["--lang", "xx"],
            vec!["--lang"],
            vec!["--settings"],
            vec!["a.brd", "b.brd"],
        ] {
            let (code, out, err) = run_view(&args);
            assert_eq!(code, 2, "{args:?}");
            assert!(out.is_empty());
            assert!(err.contains("phoenix view"), "{args:?}");
        }
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.cfg");
        std::fs::write(&bad, "nonsense").unwrap();
        let name = bad.to_string_lossy().into_owned();
        let (code, _, err) = run_view(&["--bench", "--settings", &name]);
        assert_eq!(code, 2);
        assert!(err.contains("PHOENIX-VIEW-005"), "{err}");
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn without_the_gui_feature_the_window_explains_how_to_build_it() {
        let (code, out, err) = run_view(&["some.brd"]);
        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert!(err.contains("PHOENIX-VIEW-008"), "{err}");
        assert!(err.contains("--features gui"), "{err}");
        let (_, _, err) = run_view(&["--lang", "fr"]);
        assert!(err.contains("La fenêtre n'est pas incluse"), "{err}");
    }
}
