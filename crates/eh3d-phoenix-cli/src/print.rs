// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `print`: a schematic (one page for each sheet), the assembly plan and the layers of
//! a board, or a panel of boards, written as SVG (one file for each page) or as one PDF.
//!
//! It is a dry run unless `--out` names a file or a folder that does not exist yet; nothing that
//! exists is ever written over, and the files that are read are never written. The settings are
//! those of `config/print.cfg`, changed by the options.
//!
//! Exit codes: 0 done, 2 the arguments, the settings, the input or the output are unusable.

use std::io::Write;
use std::path::Path;

use eh3d_phoenix_print::codes::{OPTION_INVALID, TEXT_NOT_REPRESENTABLE};
use eh3d_phoenix_print::config::{parse_layers, parse_scale, read_file, SideChoice, DEFAULT_FILE};
use eh3d_phoenix_print::output::write_new;
use eh3d_phoenix_print::{render, Content, Format, Paper, PrintConfig, PrintError, Rendered};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_viewer::load::{read_document, scene_of, Document};
use eh3d_phoenix_viewer::panel_scene::PanelSession;
use eh3d_phoenix_viewer::print_scene::{board_contents, panel_contents, schematic_contents};
use eh3d_phoenix_viewer::ViewerSettings;

pub(crate) const USAGE: &str = "\
phoenix print <schematic|board|panel> [--kind svg|pdf] [--out FOLDER|FILE] [--paper A4|A3|Letter|WxH] [--scale N] [--layers LIST] [--side top|bottom|both] [--lang CODE] [--config FILE] [<file>]
phoenix print panel [--boards A,B,...] [--panel-config FILE] ...

  print         print a schematic (one page for each sheet), a board (the assembly plan of each
                side and its layers one over the other) or a panel of boards as SVG (a file for
                each page) or as one PDF, with a border, a title block (project, title, sheet,
                revision, author, scale and paper, in the language chosen), a scale bar and,
                with `dimensions = true`, the width and the height. `esquema` and `placa` are
                accepted for `schematic` and `board`. A dry run unless --out names a file or a
                folder that does not exist yet (a file is never replaced); a prototype whose PDF
                has not been opened in real readers of PDF
  --kind        svg (default) or pdf
  --out         a folder, or a file that ends in .svg or .pdf (one page of SVG, or a PDF)
  --paper       the paper, instead of the settings: A4, A3, Letter or a size such as 400x300
  --scale N     the scale (paper over design): 2:1, 1:2, 0.5 or auto, instead of the settings
  --layers LIST the layers to print, separated by commas, instead of the settings (the names
                are those of the colours in config/print.cfg, without `colour_`)
  --side        the side of a board to print: top, bottom or both
  --boards LIST the board files of a panel of different boards, separated by commas
  --panel-config the settings of the panel (default: the defaults of the panel crate)
  --config      the settings of the printing (default: config/print.cfg when it exists)
  --lang CODE   the title block and the messages in en, es, de, fr, it, ja, zh or pt
";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Subject {
    Schematic,
    Board,
    Panel,
}

struct Options {
    subject: Subject,
    kind: Format,
    out: Option<String>,
    paper: Option<String>,
    scale: Option<String>,
    layers: Option<String>,
    side: Option<String>,
    lang: Option<Lang>,
    config: Option<String>,
    panel_config: Option<String>,
    boards: Option<Vec<String>>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, PrintError> {
    let bad = |what: &str| PrintError::new(OPTION_INVALID, what.to_owned());
    let (first, rest) = args
        .split_first()
        .ok_or_else(|| bad("say what to print: schematic, board or panel"))?;
    let subject = match first.to_ascii_lowercase().as_str() {
        "schematic" | "esquema" => Subject::Schematic,
        "board" | "placa" => Subject::Board,
        "panel" => Subject::Panel,
        other => return Err(bad(&format!("`{other}` is not schematic, board or panel"))),
    };
    let mut o = Options {
        subject,
        kind: Format::Svg,
        out: None,
        paper: None,
        scale: None,
        layers: None,
        side: None,
        lang: None,
        config: None,
        panel_config: None,
        boards: None,
        files: Vec::new(),
    };
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| -> Result<String, PrintError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(&format!("{name} needs a value")))
        };
        match *arg {
            "--kind" => {
                let text = value("--kind")?;
                o.kind = Format::parse(&text).ok_or_else(|| bad("--kind is svg or pdf"))?;
            }
            "--out" => o.out = Some(value("--out")?),
            "--paper" => o.paper = Some(value("--paper")?),
            "--scale" => o.scale = Some(value("--scale")?),
            "--layers" => o.layers = Some(value("--layers")?),
            "--side" => o.side = Some(value("--side")?),
            "--config" => o.config = Some(value("--config")?),
            "--panel-config" => o.panel_config = Some(value("--panel-config")?),
            "--lang" => {
                let tag = value("--lang")?;
                o.lang = Some(Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?);
            }
            "--boards" => {
                let list = value("--boards")?;
                let names: Vec<String> = list.split(',').map(|n| n.trim().to_owned()).collect();
                if names.iter().any(String::is_empty) {
                    return Err(bad("--boards needs board files separated by commas"));
                }
                o.boards = Some(names);
            }
            other if other.starts_with("--") => {
                return Err(bad(&format!("unknown option {other}")));
            }
            other => o.files.push(other.to_owned()),
        }
    }
    Ok(o)
}

fn build_config(options: &Options) -> Result<PrintConfig, PrintError> {
    let bad = |what: &str| PrintError::new(OPTION_INVALID, what.to_owned());
    let mut cfg = match options.config.as_deref() {
        Some(path) => read_file(Path::new(path))?,
        None if Path::new(DEFAULT_FILE).is_file() => read_file(Path::new(DEFAULT_FILE))?,
        None => PrintConfig::default(),
    };
    if let Some(text) = &options.paper {
        cfg.paper = Paper::parse(text)?;
    }
    if let Some(text) = &options.scale {
        cfg.scale = parse_scale(text)
            .ok_or_else(|| bad("--scale is auto, 1:2, 2:1 or a number from 0.001 to 1000"))?;
    }
    if let Some(text) = &options.layers {
        cfg.layers = parse_layers(text)
            .map_err(|name| bad(&format!("--layers: `{name}` is not a layer")))?;
    }
    if let Some(text) = &options.side {
        cfg.side = match text.as_str() {
            "top" => SideChoice::Top,
            "bottom" => SideChoice::Bottom,
            "both" => SideChoice::Both,
            _ => return Err(bad("--side is top, bottom or both")),
        };
    }
    cfg.validate()?;
    Ok(cfg)
}

/// The name of a file without its folder and extension.
fn stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("print")
        .to_owned()
}

/// What is to be printed: the name of the document and its pages.
fn build_contents(
    options: &Options,
    cfg: &PrintConfig,
    lang: Lang,
) -> Result<(String, Vec<Content>), PrintError> {
    let refused = |text: String| PrintError::new(OPTION_INVALID, text);
    let viewer = ViewerSettings::default();
    let described = |e: eh3d_phoenix_viewer::ViewError| refused(e.message(lang));
    if options.subject == Subject::Panel {
        let paths: Vec<String> = match (&options.boards, options.files.as_slice()) {
            (Some(list), []) => list.clone(),
            (None, [one]) => vec![one.clone()],
            _ => {
                return Err(refused(
                    "say the board of the panel, or --boards".to_owned(),
                ))
            }
        };
        let files: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
        let session = PanelSession::open(
            &files,
            options.panel_config.as_deref().map(Path::new),
            &viewer,
        )
        .map_err(described)?;
        let name = stem(&paths[0]);
        return Ok((
            name.clone(),
            panel_contents(&session.scene, cfg, lang, &name),
        ));
    }
    let [path] = options.files.as_slice() else {
        return Err(refused("say the file to print".to_owned()));
    };
    let loaded = read_document(Path::new(path), &viewer).map_err(described)?;
    let scene = scene_of(&loaded.document, &viewer).map_err(described)?;
    let name = stem(path);
    let contents = match (&loaded.document, options.subject) {
        (Document::Schematic(schematic), Subject::Schematic) => {
            schematic_contents(schematic, &scene, &viewer, cfg, lang, &name)
        }
        (Document::Board(board), Subject::Board) => board_contents(board, &scene, cfg, lang, &name),
        (Document::Board(_), _) => return Err(refused(format!("{path} is a board"))),
        (Document::Schematic(_), _) => return Err(refused(format!("{path} is a schematic"))),
    };
    Ok((name, contents))
}

fn report(
    options: &Options,
    rendered: &Rendered,
    cfg: &PrintConfig,
    lang: Lang,
    written: Option<&[std::path::PathBuf]>,
) -> String {
    let mut text = String::new();
    let kind = rendered.format.extension();
    text.push_str(&format!(
        "print: {} pages, {kind}, paper {}\n",
        rendered.pages.len(),
        cfg.paper.name()
    ));
    for (i, page) in rendered.pages.iter().enumerate() {
        text.push_str(&format!(
            "page {}: {}, {} x {} mm, scale {}\n",
            i + 1,
            page.title,
            eh3d_phoenix_print::draw::num(page.width),
            eh3d_phoenix_print::draw::num(page.height),
            eh3d_phoenix_print::page::scale_text(page.scale)
        ));
    }
    for (i, file) in rendered.files.iter().enumerate() {
        let verb = if written.is_some() {
            "wrote"
        } else {
            "would write"
        };
        let name = written
            .and_then(|paths| paths.get(i))
            .map_or_else(|| file.name.clone(), |p| p.display().to_string());
        text.push_str(&format!(
            "{verb} {name} ({} bytes, {} pages)\n",
            file.bytes.len(),
            file.pages
        ));
    }
    if rendered.boxed_chars > 0 {
        let note = PrintError::new(
            TEXT_NOT_REPRESENTABLE,
            format!("{} characters", rendered.boxed_chars),
        );
        text.push_str(&format!("warning {}\n", note.message(lang)));
    }
    if options.out.is_none() {
        text.push_str("dry run: nothing written (give --out with a new file or folder to write)\n");
    }
    text
}

pub(crate) fn print_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(Lang::En));
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let lang = options.lang.unwrap_or(Lang::En);
    let run = || -> Result<String, PrintError> {
        let cfg = build_config(&options)?;
        let (name, contents) = build_contents(&options, &cfg, lang)?;
        let rendered = render(&name, &contents, options.kind, &cfg, lang)?;
        let written = match &options.out {
            Some(path) => Some(write_new(Path::new(path), &rendered)?),
            None => None,
        };
        Ok(report(&options, &rendered, &cfg, lang, written.as_deref()))
    };
    match run() {
        Ok(text) => {
            let _ = out.write_all(text.as_bytes());
            0
        }
        Err(error) => {
            let _ = writeln!(err, "error {}", error.message(lang));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_print::output::planned_paths;

    fn run_print(args: &[&str]) -> (i32, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = print_command(args, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn the_arguments_are_read() {
        let o = parse_options(&[
            "esquema", "--kind", "PDF", "--paper", "A3", "--scale", "1:2", "--lang", "pt", "a.sch",
        ])
        .unwrap();
        assert!(o.subject == Subject::Schematic);
        assert!(o.kind == Format::Pdf);
        assert_eq!(o.paper.as_deref(), Some("A3"));
        assert_eq!(o.lang, Some(Lang::Pt));
        assert_eq!(o.files, vec!["a.sch"]);
        let cfg = build_config(&o).unwrap();
        assert_eq!(cfg.paper, Paper::A3);
        assert_eq!(cfg.scale, 0.5);
    }

    #[test]
    fn bad_arguments_are_exit_code_two_with_the_usage() {
        for args in [
            vec![],
            vec!["nothing"],
            vec!["board", "--kind", "png"],
            vec!["board", "--nonsense"],
            vec!["board", "--out"],
            vec!["board", "--lang", "xx"],
            vec!["panel", "--boards", "a,,b"],
        ] {
            let (code, out, err) = run_print(&args);
            assert_eq!(code, 2, "{args:?}");
            assert!(out.is_empty());
            assert!(err.contains("phoenix print"), "{args:?}: {err}");
        }
    }

    #[test]
    fn the_options_that_change_the_settings_are_checked() {
        for args in [
            vec!["board", "--paper", "A9", "b.json"],
            vec!["board", "--scale", "fast", "b.json"],
            vec!["board", "--layers", "wire,nope", "b.json"],
            vec!["board", "--side", "left", "b.json"],
        ] {
            let (code, _, err) = run_print(&args);
            assert_eq!(code, 2, "{args:?}");
            assert!(err.contains("PHOENIX-PRINT-"), "{args:?}: {err}");
        }
        let (code, _, err) = run_print(&["board", "--lang", "pt", "--paper", "A9", "b.json"]);
        assert_eq!(code, 2);
        assert!(err.contains("O tamanho do papel não é válido"), "{err}");
    }

    #[test]
    fn the_place_of_the_output_is_checked_without_writing() {
        let rendered = Rendered {
            format: Format::Pdf,
            files: Vec::new(),
            pages: Vec::new(),
            boxed_chars: 0,
        };
        let dir = tempfile::tempdir().unwrap();
        let new = dir.path().join("x");
        assert_eq!(planned_paths(&new, &rendered).unwrap().len(), 0);
        assert!(!new.exists());
        assert!(planned_paths(dir.path(), &rendered).is_err());
    }
}
