// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `dxf-export`: a board or a panel of boards as a DXF drawing (release 12, in ASCII)
//! with the outline, the drilled holes, the V-score lines, the fiducials and the outlines of the
//! pads on layers of their own, and the silkscreen if asked. It is a dry run unless `--out` names a
//! file that does not exist yet; a file that exists is never written over.
//!
//! Exit codes: 0 done, 2 the arguments, the settings, the input or the output are unusable.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, PadRef, PadShape, Segment, Side, SilkItem};
use eh3d_phoenix_dxf::codes::{
    DxfError, Finding, Severity, EXPORT_SIMPLIFIED, INPUT_UNREADABLE, OPTION_INVALID, OUTPUT_FAILED,
};
use eh3d_phoenix_dxf::{write_dxf, Drawing, DxfSettings, Seg, P};
use eh3d_phoenix_panel::{panel_openings, HoleKind, PanelPlan};
use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_viewer::panel_scene::PanelSession;
use eh3d_phoenix_viewer::ViewerSettings;
use serde_json::json;

use crate::dxf_import::load_dxf_settings;
use crate::export::read_board_file;

pub(crate) const USAGE: &str = "\
phoenix dxf-export <board|panel> [--out FILE.dxf] [--config FILE] [--panel-config FILE] [--boards A,B,...] [--silk] [--json] [--lang CODE] <file>

  dxf-export    a board or a panel of boards as a DXF drawing (release 12, in ASCII, millimetres,
                the same text every time). A board gives its outline (a closed polyline), its
                drilled holes (circles), the outlines of its pads and, with --silk, its
                silkscreen; a panel gives the frame and the routed outline, the V-score lines, the
                tabs (crosses), the holes of the boards, the mouse bites, the tooling holes and the
                fiducials. The names of the layers come from config/dxf.cfg. `placa` is accepted
                for `board`. A dry run unless --out names a file that does not exist yet (a file
                is never replaced). A prototype: no CAD program has opened the files it writes
  --out         the DXF file to write (a new file)
  --config      the names of the layers and the text settings (default: config/dxf.cfg when it
                exists)
  --silk        the silkscreen of a board: strokes as polylines and texts as texts (a text is
                written without its rotation and its mirror)
  --boards LIST the board files of a panel of different boards, separated by commas
  --panel-config the settings of the panel (default: the defaults of the panel crate)
  --json        print the report as JSON. --lang CODE writes the titles in en, es, de, fr, it,
                ja, zh or pt
";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Subject {
    Board,
    Panel,
}

struct Options {
    subject: Subject,
    out: Option<String>,
    config: Option<String>,
    panel_config: Option<String>,
    boards: Option<Vec<String>>,
    silk: bool,
    json: bool,
    lang: Lang,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, DxfError> {
    let bad = |what: &str| DxfError::new(OPTION_INVALID, what.to_owned());
    let (first, rest) = args
        .split_first()
        .ok_or_else(|| bad("say what to export: board or panel"))?;
    let subject = match first.to_ascii_lowercase().as_str() {
        "board" | "placa" => Subject::Board,
        "panel" => Subject::Panel,
        other => return Err(bad(&format!("`{other}` is not board or panel"))),
    };
    let mut o = Options {
        subject,
        out: None,
        config: None,
        panel_config: None,
        boards: None,
        silk: false,
        json: false,
        lang: Lang::En,
        files: Vec::new(),
    };
    let mut iter = rest.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, DxfError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--json" => o.json = true,
            "--silk" => o.silk = true,
            "--out" => o.out = Some(value("--out needs a file")?),
            "--config" => o.config = Some(value("--config needs a file")?),
            "--panel-config" => o.panel_config = Some(value("--panel-config needs a file")?),
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                o.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--boards" => {
                let list = value("--boards needs board files")?;
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

/// A point turned about the origin by `udeg` micro-degrees: exact for the right angles, rounded
/// to the nanometre for any other.
fn turn(p: P, udeg: i64) -> P {
    match udeg.rem_euclid(360_000_000) {
        0 => p,
        90_000_000 => (-p.1, p.0),
        180_000_000 => (-p.0, -p.1),
        270_000_000 => (p.1, -p.0),
        a => {
            let radians = (a as f64 / 1_000_000.0).to_radians();
            let (s, c) = radians.sin_cos();
            let (x, y) = (p.0 as f64, p.1 as f64);
            (
                (x * c - y * s).round() as i64,
                (x * s + y * c).round() as i64,
            )
        }
    }
}

/// The closed outline of a pad of the shape `shape` about `centre`, turned by `angle`, or a circle.
fn pad_outline(drawing: &mut Drawing, layer: &str, shape: &PadShape, centre: P, angle: i64) {
    let place = |p: P| {
        let q = turn(p, angle);
        (centre.0 + q.0, centre.1 + q.1)
    };
    match *shape {
        PadShape::Circle { diameter } => {
            let radius = diameter.get() / 2;
            if radius > 0 {
                drawing.circle(layer, centre, radius);
            }
        }
        PadShape::Rect { width, height } => {
            let (w, h) = (width.get() / 2, height.get() / 2);
            let corners = [(-w, -h), (w, -h), (w, h), (-w, h)].map(place);
            drawing.polygon(layer, &corners);
        }
        PadShape::Oblong { width, height } => {
            let (w, h) = (width.get(), height.get());
            // two straight sides (the pieces 0 and 2) and two half turns about the ends (the
            // pieces 1 and 3), walked counter-clockwise
            let ends: [(P, P); 4] = if w >= h {
                let (r, s) = (h / 2, w / 2 - h / 2);
                [
                    ((-s, -r), (s, -r)),
                    ((s, -r), (s, r)),
                    ((s, r), (-s, r)),
                    ((-s, r), (-s, -r)),
                ]
            } else {
                let (r, s) = (w / 2, h / 2 - w / 2);
                [
                    ((r, -s), (r, s)),
                    ((r, s), (-r, s)),
                    ((-r, s), (-r, -s)),
                    ((-r, -s), (r, -s)),
                ]
            };
            let segs: Vec<Seg> = ends
                .iter()
                .enumerate()
                .map(|(i, &(a, b))| Seg {
                    a: place(a),
                    b: place(b),
                    sweep_udeg: if i.is_multiple_of(2) { 0 } else { 180_000_000 },
                })
                .collect();
            drawing.path(layer, segs, true);
        }
    }
}

/// The drawing of a board: the outline and its interior cutouts, the drilled holes, the outlines of the pads and, if asked,
/// the silkscreen.
pub(crate) fn board_drawing(board: &Board, settings: &DxfSettings, silk: bool) -> Drawing {
    let mut d = Drawing::new();
    // the arcs of the outline are written as the chords they are cut into
    let outline: Vec<P> = board
        .outline_polygon(&eh3d_phoenix_board::ArcSettings::current())
        .iter()
        .map(|p| (p.x_nm.get(), p.y_nm.get()))
        .collect();
    d.polygon(&settings.layer_outline, &outline);
    for cutout in board.outline_cutouts.iter().filter(|c| c.len() >= 3) {
        let ring: Vec<P> = cutout
            .iter()
            .map(|p| (p.x_nm.get(), p.y_nm.get()))
            .collect();
        d.polygon(&settings.layer_outline, &ring);
    }
    for via in board.vias.values() {
        let radius = via.drill.get() / 2;
        if radius > 0 {
            d.circle(
                &settings.layer_drills,
                (via.at.x_nm.get(), via.at.y_nm.get()),
                radius,
            );
        }
    }
    for (component_id, component) in &board.components {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for (pad_id, pad) in &footprint.pads {
            let Some(at) = pad_position(
                board,
                PadRef {
                    component: *component_id,
                    pad: *pad_id,
                },
            ) else {
                continue;
            };
            let centre = (at.x_nm.get(), at.y_nm.get());
            if let Some(drill) = pad.drill {
                let radius = drill.get() / 2;
                if radius > 0 {
                    d.circle(&settings.layer_drills, centre, radius);
                }
            }
            let own = pad.rotation.get();
            let angle = component.rotation.get()
                + if component.side == Side::Bottom {
                    -own
                } else {
                    own
                };
            pad_outline(&mut d, &settings.layer_pads, &pad.shape, centre, angle);
        }
    }
    if silk {
        for item in board.silkscreen.values() {
            match item {
                SilkItem::Text {
                    at, text, height, ..
                } => d.text(
                    &settings.layer_silk,
                    (at.x_nm.get(), at.y_nm.get()),
                    height.get(),
                    text,
                ),
                SilkItem::Stroke { path, .. } => {
                    let segs: Vec<Seg> = path
                        .iter()
                        .map(|s| match *s {
                            Segment::Line { start, end } => Seg::line(
                                (start.x_nm.get(), start.y_nm.get()),
                                (end.x_nm.get(), end.y_nm.get()),
                            ),
                            Segment::Arc { start, end, sweep } => Seg {
                                a: (start.x_nm.get(), start.y_nm.get()),
                                b: (end.x_nm.get(), end.y_nm.get()),
                                sweep_udeg: sweep.get(),
                            },
                        })
                        .collect();
                    d.path(&settings.layer_silk, segs, false);
                }
            }
        }
    }
    if settings.write_text && !outline.is_empty() {
        let low = outline.iter().fold((i64::MAX, i64::MAX), |acc, p| {
            (acc.0.min(p.0), acc.1.min(p.1))
        });
        d.text(
            &settings.layer_text,
            (low.0, low.1 - 2 * settings.text_height_nm),
            settings.text_height_nm,
            &board.name,
        );
    }
    d
}

/// The drawing of a panel: the frame and the routed outline, the V-score lines, the tabs, the holes
/// of the boards, the mouse bites, the tooling holes and the fiducials.
pub(crate) fn panel_drawing(plan: &PanelPlan, names: &[String], settings: &DxfSettings) -> Drawing {
    let mut d = Drawing::new();
    let point = |p: eh3d_phoenix_panel::Pt| (p.x, p.y);
    let (w, h) = (plan.width, plan.height);
    d.polygon(&settings.layer_outline, &[(0, 0), (w, 0), (w, h), (0, h)]);
    for line in &plan.profile {
        d.line(&settings.layer_outline, point(line.from), point(line.to));
    }
    for line in &plan.score {
        d.line(&settings.layer_vscore, point(line.from), point(line.to));
    }
    for tab in &plan.tabs {
        d.cross(&settings.layer_tabs, point(*tab), settings.cross_size_nm);
    }
    for hole in &plan.holes {
        let layer = match hole.kind {
            HoleKind::Plated | HoleKind::NonPlated => &settings.layer_drills,
            HoleKind::MouseBite => &settings.layer_mouse_bites,
            HoleKind::Tooling => &settings.layer_tooling,
        };
        let radius = hole.diameter / 2;
        if radius > 0 {
            d.circle(layer, point(hole.at), radius);
        }
    }
    for mark in plan.fiducials.iter().chain(plan.local_fiducials.iter()) {
        let radius = mark.copper_diameter / 2;
        if radius > 0 {
            d.circle(&settings.layer_fiducials, point(mark.at), radius);
        }
        d.cross(
            &settings.layer_fiducials,
            point(mark.at),
            settings.cross_size_nm,
        );
    }
    if settings.write_text {
        for copy in &plan.copies {
            if let Some(name) = names.get(copy.board) {
                d.text(
                    &settings.layer_text,
                    point(copy.origin),
                    settings.text_height_nm,
                    name,
                );
            }
        }
    }
    d
}

/// The openings that the boards have inside their outlines, each a closed polygon on the layer
/// of the outline.
pub(crate) fn add_openings(
    drawing: &mut Drawing,
    openings: &[Vec<eh3d_phoenix_panel::Pt>],
    settings: &DxfSettings,
) {
    for ring in openings.iter().filter(|r| r.len() >= 3) {
        let points: Vec<(i64, i64)> = ring.iter().map(|p| (p.x, p.y)).collect();
        drawing.polygon(&settings.layer_outline, &points);
    }
}

fn export_note() -> Finding {
    Finding::new(
        EXPORT_SIMPLIFIED,
        Severity::Info,
        None,
        "outlines of the pads, holes, lines and marks; copper, mask, paste, widths and the rotation of texts are not in the file",
    )
}

/// How many entities each layer holds.
fn layer_counts(drawing: &Drawing) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for e in &drawing.entities {
        *counts.entry(e.layer.clone()).or_insert(0) += 1;
    }
    counts
}

fn stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map_or_else(|| path.to_owned(), |s| s.to_string_lossy().into_owned())
}

fn fail(err: &mut dyn Write, error: &DxfError, lang: Lang) -> i32 {
    let _ = writeln!(err, "error: {}", error.message(lang));
    2
}

fn drawing_of(options: &Options, settings: &DxfSettings) -> Result<Drawing, DxfError> {
    let unreadable = |what: String| DxfError::new(INPUT_UNREADABLE, what);
    if options.subject == Subject::Board {
        let [path] = options.files.as_slice() else {
            return Err(DxfError::new(OPTION_INVALID, "give exactly one board file"));
        };
        let board = read_board_file(path).map_err(unreadable)?;
        return Ok(board_drawing(&board, settings, options.silk));
    }
    let paths: Vec<String> = match (&options.boards, options.files.as_slice()) {
        (Some(list), []) => list.clone(),
        (None, [one]) => vec![one.clone()],
        _ => {
            return Err(DxfError::new(
                OPTION_INVALID,
                "say the board of the panel, or --boards",
            ))
        }
    };
    let files: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let session = PanelSession::open(
        &files,
        options.panel_config.as_deref().map(Path::new),
        &ViewerSettings::default(),
    )
    .map_err(|e| unreadable(e.message(options.lang)))?;
    let names: Vec<String> = paths.iter().map(|p| stem(p)).collect();
    let mut drawing = panel_drawing(&session.plan, &names, settings);
    let openings = panel_openings(&session.boards, &session.plan)
        .map_err(|e| DxfError::new(INPUT_UNREADABLE, e.to_string()))?;
    add_openings(&mut drawing, &openings, settings);
    Ok(drawing)
}

/// The command `dxf-export`.
pub(crate) fn dxf_export_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(e) => {
            let _ = err.write_all(USAGE.as_bytes());
            return fail(err, &e, Lang::En);
        }
    };
    let lang = options.lang;
    if let Some(path) = options.out.as_deref() {
        let refuse = |why: String| DxfError::new(OUTPUT_FAILED, why);
        let inputs = options.files.iter().chain(options.boards.iter().flatten());
        let problem = if Path::new(path).exists() {
            Some(refuse(format!("{path} exists; give a new file")))
        } else if !path.to_ascii_lowercase().ends_with(".dxf") {
            Some(refuse(format!("{path}: only .dxf is written here")))
        } else if inputs
            .into_iter()
            .any(|i| crate::design::same_file(path, i))
        {
            Some(refuse(format!("{path} is an input file")))
        } else {
            None
        };
        if let Some(e) = problem {
            return fail(err, &e, lang);
        }
    }
    let (settings, _) = match load_dxf_settings(options.config.as_deref()) {
        Ok(loaded) => loaded,
        Err(e) => return fail(err, &e, lang),
    };
    let drawing = match drawing_of(&options, &settings) {
        Ok(drawing) => drawing,
        Err(e) => return fail(err, &e, lang),
    };
    let text = write_dxf(&drawing);
    let mut written: Option<String> = None;
    if let Some(path) = options.out.as_deref() {
        let result = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .and_then(|mut file| file.write_all(text.as_bytes()));
        if let Err(e) = result {
            return fail(
                err,
                &DxfError::new(OUTPUT_FAILED, format!("{path}: {e}")),
                lang,
            );
        }
        written = Some(path.to_owned());
    }
    let counts = layer_counts(&drawing);
    let note = export_note();
    if options.json {
        let value = json!({
            "schema": "phoenix-dxf-export/1",
            "entities": drawing.entities.len(),
            "layers": counts,
            "bytes": text.len(),
            "notes": [{"code": note.code, "severity": note.severity.as_str(), "detail": note.detail}],
            "written": written,
        });
        let _ = writeln!(out, "{value}");
    } else {
        let _ = writeln!(out, "entities: {}", drawing.entities.len());
        for (layer, count) in &counts {
            let _ = writeln!(out, "layer {layer}: {count}");
        }
        let _ = writeln!(out, "{}", note.line(lang));
        match &written {
            Some(path) => {
                let _ = writeln!(out, "written: {path}");
            }
            None => {
                let _ = writeln!(
                    out,
                    "dry run: nothing was written (--out FILE.dxf writes the drawing)"
                );
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    use eh3d_phoenix_board::{Component, Footprint, Id, Layer, LayerKind, Net, Pad, Via};
    use eh3d_phoenix_dxf::{read_dxf, Shape as DxfShape};
    use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};
    use eh3d_phoenix_panel::{plan, BoardShape, PanelSettings, Pt};

    const MM: i64 = 1_000_000;

    fn id(n: u16) -> Id {
        ObjectId::from_parts(1_700_000_000_000, n, u64::from(n)).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    /// A board 40 by 30 mm with a via and a part of three pads: a round one with a drill, a
    /// rectangular one and an oblong one.
    fn board(rotation_deg: i64) -> Board {
        let top = id(1);
        let mut b = Board {
            name: "demo".to_owned(),
            outline: vec![
                pt(0, 0),
                pt(40 * MM, 0),
                pt(40 * MM, 30 * MM),
                pt(0, 30 * MM),
            ],
            ..Board::default()
        };
        b.layers.insert(
            top,
            Layer {
                name: "Top".to_owned(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        b.nets.insert(
            id(2),
            Net {
                name: "N".to_owned(),
            },
        );
        b.vias.insert(
            id(3),
            Via {
                net: id(2),
                at: pt(5 * MM, 5 * MM),
                drill: Nm::new(400_000).unwrap(),
                diameter: Nm::new(800_000).unwrap(),
                from_layer: top,
                to_layer: top,
            },
        );
        let mut pads = BTreeMap::new();
        let mut add = |n: u16, number: &str, shape: PadShape, x: i64, drill: Option<i64>| {
            pads.insert(
                id(n),
                Pad {
                    number: number.to_owned(),
                    shape,
                    at: pt(x * MM, 0),
                    rotation: Udeg::ZERO,
                    drill: drill.map(|d| Nm::new(d).unwrap()),
                    layers: vec![top],
                    roundness: 0,
                    plated: true,
                },
            );
        };
        add(
            10,
            "1",
            PadShape::Circle {
                diameter: Nm::new(1_600_000).unwrap(),
            },
            -3,
            Some(800_000),
        );
        add(
            11,
            "2",
            PadShape::Rect {
                width: Nm::new(2 * MM).unwrap(),
                height: Nm::new(MM).unwrap(),
            },
            0,
            None,
        );
        add(
            12,
            "3",
            PadShape::Oblong {
                width: Nm::new(3 * MM).unwrap(),
                height: Nm::new(MM).unwrap(),
            },
            3,
            None,
        );
        b.footprints.insert(
            id(20),
            Footprint {
                name: "F".to_owned(),
                pads,
            },
        );
        b.components.insert(
            id(30),
            Component {
                refdes: "U1".to_owned(),
                value: String::new(),
                symbol: None,
                footprint: id(20),
                at: pt(10 * MM, 10 * MM),
                rotation: Udeg::new(rotation_deg * 1_000_000).unwrap(),
                side: Side::Top,
            },
        );
        b
    }

    fn on<'a>(d: &'a Drawing, layer: &'a str) -> Vec<&'a DxfShape> {
        d.on_layer(layer).map(|e| &e.shape).collect()
    }

    #[test]
    fn a_board_gives_its_outline_its_holes_and_the_outlines_of_its_pads() {
        let settings = DxfSettings::default();
        let d = board_drawing(&board(0), &settings, false);
        let outline = on(&d, "OUTLINE");
        assert_eq!(outline.len(), 1);
        match outline[0] {
            DxfShape::Path { segs, closed } => {
                assert!(*closed);
                assert_eq!(segs.len(), 4);
                assert_eq!(segs[1].a, (40 * MM, 0));
            }
            other => panic!("{other:?}"),
        }
        // the via and the drill of the round pad
        let drills = on(&d, "DRILLS");
        assert_eq!(drills.len(), 2);
        assert!(drills.contains(&&DxfShape::Circle {
            centre: (5 * MM, 5 * MM),
            radius: 200_000
        }));
        assert!(drills.contains(&&DxfShape::Circle {
            centre: (7 * MM, 10 * MM),
            radius: 400_000
        }));
        let pads = on(&d, "PADS");
        assert_eq!(pads.len(), 3);
        assert!(pads.contains(&&DxfShape::Circle {
            centre: (7 * MM, 10 * MM),
            radius: 800_000
        }));
        // the rectangle 2 by 1 about (10, 10)
        match pads.iter().find_map(|s| match s {
            DxfShape::Path { segs, .. }
                if segs.len() == 4 && segs.iter().all(|g| g.sweep_udeg == 0) =>
            {
                Some(segs.clone())
            }
            _ => None,
        }) {
            Some(segs) => {
                assert_eq!(segs[0].a, (9 * MM, 9_500_000));
                assert_eq!(segs[1].a, (11 * MM, 9_500_000));
                assert_eq!(segs[2].a, (11 * MM, 10_500_000));
                assert_eq!(segs[3].a, (9 * MM, 10_500_000));
            }
            None => panic!("no rectangle"),
        }
        // the oblong 3 by 1 about (13, 10): two lines and two half turns
        let oblong = pads
            .iter()
            .find_map(|s| match s {
                DxfShape::Path { segs, .. } if segs.iter().any(|g| g.sweep_udeg != 0) => {
                    Some(segs.clone())
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(oblong.len(), 4);
        assert_eq!(
            oblong[0],
            Seg::line((12 * MM, 9_500_000), (14 * MM, 9_500_000))
        );
        assert_eq!(
            oblong[1],
            Seg {
                a: (14 * MM, 9_500_000),
                b: (14 * MM, 10_500_000),
                sweep_udeg: 180_000_000
            }
        );
        assert_eq!(
            oblong[2],
            Seg::line((14 * MM, 10_500_000), (12 * MM, 10_500_000))
        );
        assert_eq!(oblong[3].sweep_udeg, 180_000_000);
        assert!(on(&d, "SILKSCREEN").is_empty());
    }

    #[test]
    fn a_turned_part_turns_its_pads_and_their_outlines() {
        let d = board_drawing(&board(90), &DxfSettings::default(), false);
        // the round pad at (-3, 0) goes to (0, -3) about (10, 10)
        assert!(on(&d, "PADS").contains(&&DxfShape::Circle {
            centre: (10 * MM, 7 * MM),
            radius: 800_000
        }));
        // the rectangle 2 by 1 turned to 1 by 2: its first corner (-1, -0.5) goes to (0.5, -1)
        let rect = on(&d, "PADS")
            .into_iter()
            .find_map(|s| match s {
                DxfShape::Path { segs, .. }
                    if segs.len() == 4 && segs.iter().all(|g| g.sweep_udeg == 0) =>
                {
                    Some(segs.clone())
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(rect[0].a, (10_500_000, 9 * MM));
    }

    #[test]
    fn the_names_of_the_layers_and_the_text_come_from_the_settings() {
        let settings =
            DxfSettings::parse("layer_outline = EDGE\nlayer_drills = HOLES\nwrite_text = true\n")
                .unwrap();
        let d = board_drawing(&board(0), &settings, false);
        assert_eq!(on(&d, "EDGE").len(), 1);
        assert_eq!(on(&d, "HOLES").len(), 2);
        assert!(on(&d, "OUTLINE").is_empty());
        match on(&d, "TEXT")[0] {
            DxfShape::Text { at, height, text } => {
                assert_eq!(text, "demo");
                assert_eq!(*at, (0, -2 * MM));
                assert_eq!(*height, MM);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_openings_of_the_boards_of_a_panel_are_closed_polygons_on_the_outline_layer() {
        let dxf = DxfSettings::default();
        let mut d = Drawing::new();
        let ring: Vec<Pt> = [(1, 1), (3, 1), (3, 2)]
            .iter()
            .map(|(x, y)| Pt {
                x: x * MM,
                y: y * MM,
            })
            .collect();
        let short = vec![Pt { x: 0, y: 0 }, Pt { x: MM, y: 0 }];
        add_openings(&mut d, &[ring, short], &dxf);
        // the ring with two points is not an opening
        assert_eq!(on(&d, "OUTLINE").len(), 1);
        match on(&d, "OUTLINE")[0] {
            DxfShape::Path { segs, closed } => {
                assert!(*closed);
                assert_eq!(segs.len(), 3);
                assert_eq!(segs[2].a, (3 * MM, 2 * MM));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_file_written_is_read_back_by_the_reader_of_the_suite() {
        let settings = DxfSettings::default();
        let d = board_drawing(&board(0), &settings, false);
        let read = read_dxf(&write_dxf(&d), &settings).unwrap();
        assert_eq!(read.drawing, d);
        assert!(read.report.findings.is_empty());
    }

    #[test]
    fn a_panel_gives_its_frame_scores_tabs_holes_and_fiducials() {
        let shape = BoardShape {
            origin: Pt { x: 0, y: 0 },
            width: 30 * MM,
            height: 20 * MM,
            holes: Vec::new(),
        };
        let settings = PanelSettings::parse("columns = 2\nrows = 1\n").unwrap();
        let laid = plan(&shape, &settings).unwrap();
        let dxf = DxfSettings::parse("write_text = true\n").unwrap();
        let d = panel_drawing(&laid, &["demo".to_owned()], &dxf);
        // the frame is one polyline of the panel size; the routed outline adds a line each
        match on(&d, "OUTLINE")[0] {
            DxfShape::Path { segs, closed } => {
                assert!(*closed);
                assert_eq!(segs[1].a, (laid.width, 0));
                assert_eq!(segs[2].a, (laid.width, laid.height));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(on(&d, "OUTLINE").len(), 1 + laid.profile.len());
        assert_eq!(on(&d, "VSCORE").len(), laid.score.len());
        assert_eq!(on(&d, "TABS").len(), 2 * laid.tabs.len());
        let circles = |layer: &str| {
            on(&d, layer)
                .iter()
                .filter(|s| matches!(s, DxfShape::Circle { .. }))
                .count()
        };
        assert_eq!(circles("TOOLING"), laid.holes_of(HoleKind::Tooling));
        assert_eq!(circles("MOUSEBITES"), laid.holes_of(HoleKind::MouseBite));
        assert_eq!(
            circles("DRILLS"),
            laid.holes_of(HoleKind::Plated) + laid.holes_of(HoleKind::NonPlated)
        );
        assert_eq!(
            circles("FIDUCIALS"),
            laid.fiducials.len() + laid.local_fiducials.len()
        );
        assert_eq!(on(&d, "TEXT").len(), laid.copies.len());
        assert_eq!(laid.copies.len(), 2);
    }

    #[test]
    fn the_options_are_read_and_the_wrong_ones_refused() {
        let o = parse_options(&[
            "placa", "--out", "a.dxf", "--silk", "--lang", "es", "b.json",
        ])
        .unwrap();
        assert!(o.subject == Subject::Board && o.silk);
        assert_eq!(o.out.as_deref(), Some("a.dxf"));
        assert_eq!(o.lang, Lang::Es);
        assert_eq!(o.files, vec!["b.json".to_owned()]);
        let p = parse_options(&["panel", "--boards", "a.json, b.json"]).unwrap();
        assert_eq!(
            p.boards,
            Some(vec!["a.json".to_owned(), "b.json".to_owned()])
        );
        for bad in [
            vec![],
            vec!["schematic"],
            vec!["board", "--nope"],
            vec!["board", "--out"],
            vec!["panel", "--boards", "a,,b"],
            vec!["board", "--lang", "xx"],
        ] {
            let e = parse_options(&bad).err().unwrap();
            assert_eq!(e.code, OPTION_INVALID, "{bad:?}");
        }
    }
}
