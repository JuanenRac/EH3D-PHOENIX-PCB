// SPDX-License-Identifier: GPL-3.0-or-later
//! Generator of microstrip RF structures for EH3D-PHOENIX-PCB.
//!
//! From the stack-up of the board and a frequency, the generator draws the copper of a structure
//! and returns it as one reversible operation of the board together with a report:
//!
//! * [`kinds`]: the types of structure (lines, mitered bends, curves, tapers, open and
//!   short-circuited stubs, the single-stub, quarter-wave and L matches, stepped-impedance
//!   low-pass and coupled-line band-pass filters, the Wilkinson divider, the branch-line coupler,
//!   the patch and dipole antennas and an edge SMA launch with a via fence) and the parameters of
//!   each;
//! * [`formulas`]: the published formulas, each with its source and the range in which the program
//!   trusts it;
//! * [`env`]: the stack-up, the layer and the frequency, and the track width for an impedance (the
//!   synthesis of `eh3d-phoenix-stackup`);
//! * [`layout`]: the geometry in integer nanometres and its conversion into the operations of the
//!   board (one batch, applied all or nothing, whose inverse restores the board exactly);
//! * [`report`]: the dimensions, the formulas, the warnings, the ports and the rule check, as text
//!   in the eight languages of the suite and as JSON;
//! * [`settings`] and [`codes`]: every default and limit, read from `config/rfgen.cfg`, and the
//!   codes `PHOENIX-RFGEN-001` to `PHOENIX-RFGEN-019` with their titles in the eight languages.
//!
//! Lengths are integer nanometres in the model; the one place where a length in metres becomes
//! nanometres is [`layout::nm_from_m`], which rounds to the nearest nanometre with halves away
//! from zero. Floating point is used only inside the formulas, on inputs that were checked.
//!
//! **State: prototype.** The formulas are published ones and the tests use values of textbooks, but
//! nothing here has been compared with an electromagnetic simulator or with a measurement of a
//! real board. Discontinuities (steps, T junctions, the open ends of coupled lines, vias) are not
//! modelled, and a result is a first draft to be simulated, not a finished design.
//!
//! Nothing here opens a file, a window or the network, and nothing panics on its input.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod env;
pub mod formulas;
pub mod kinds;
pub mod layout;
pub mod params;
pub mod report;
pub mod settings;
pub mod shapes;

use eh3d_phoenix_board::{Board, Id, Layer, LayerKind, OpError, Operation};
use eh3d_phoenix_rules::{check_board, Severity};
use eh3d_phoenix_stackup::{MaterialSample, Stackup};
use serde::Deserialize;

pub use codes::{title, RfgenError};
pub use eh3d_phoenix_rules::Lang;
pub use env::Environment;
pub use kinds::{Kind, NetNames};
pub use layout::Placement;
pub use params::{ParamSpec, Params};
pub use report::Report;
pub use settings::RfgenSettings;

use codes::{BOARD_REFUSED, INPUT_UNREADABLE, RULES_REFUSE};
use layout::{to_operation, IdSource};
use report::{Counts, PortReport, RuleSummary};

/// The largest stack-up file that is read, in bytes.
pub const MAX_STACKUP_BYTES: usize = 4 * 1024 * 1024;

/// What to draw, where, and with which nets.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The type of structure.
    pub kind: Kind,
    /// The parameters of that type.
    pub params: Params,
    /// The names of the nets.
    pub nets: NetNames,
    /// Where the structure goes.
    pub placement: Placement,
}

impl Request {
    /// A request for a type with its parameters, the default nets and the origin of the board.
    pub fn new(kind: Kind, params: Params) -> Request {
        Request {
            kind,
            params,
            nets: NetNames::default(),
            placement: Placement::default(),
        }
    }
}

/// A structure drawn on a board.
#[derive(Debug, Clone, PartialEq)]
pub struct Generated {
    /// The operation that puts the structure on the board: one batch, all or nothing.
    pub operation: Operation,
    /// The board with the operation applied.
    pub board: Board,
    /// The report.
    pub report: Report,
}

impl Generated {
    /// Applies the operation to a board that is the one it was made for, and returns the operation
    /// that undoes it. On an error the board is unchanged.
    pub fn apply(&self, board: &mut Board) -> Result<Operation, RfgenError> {
        board.apply(&self.operation).map_err(refused)
    }
}

fn refused(e: OpError) -> RfgenError {
    RfgenError::new(BOARD_REFUSED, e.to_string())
}

/// The content of a stack-up file: the layers and the materials. A job file of `phoenix stackup`
/// is accepted too; its lines are ignored.
#[derive(Debug, Deserialize)]
struct StackupFile {
    stackup: Stackup,
    materials: Vec<MaterialSample>,
    #[serde(default, rename = "lines")]
    _lines: Option<serde_json::Value>,
}

/// Reads a stack-up file (JSON with `stackup` and `materials`).
pub fn parse_stackup_file(text: &str) -> Result<(Stackup, Vec<MaterialSample>), RfgenError> {
    if text.len() > MAX_STACKUP_BYTES {
        return Err(RfgenError::new(
            INPUT_UNREADABLE,
            format!("the file is larger than {MAX_STACKUP_BYTES} bytes"),
        ));
    }
    let file: StackupFile =
        serde_json::from_str(text).map_err(|e| RfgenError::new(INPUT_UNREADABLE, e.to_string()))?;
    Ok((file.stackup, file.materials))
}

/// A new board with one copper layer for each copper layer of the stack-up, in the order of the
/// stack-up, with the copper stack set. The ids come from `new_id`.
pub fn new_board(
    env: &Environment,
    name: &str,
    new_id: &mut dyn FnMut() -> Id,
) -> Result<Board, RfgenError> {
    let mut board = Board {
        name: name.to_owned(),
        ..Board::default()
    };
    let mut stack = Vec::new();
    for layer in env.stackup.layers.iter().filter(|l| l.is_copper()) {
        let id = new_id();
        board.layers.insert(
            id,
            Layer {
                name: layer.name().to_owned(),
                kind: LayerKind::Copper,
                side: None,
            },
        );
        stack.push(id);
    }
    board
        .apply(&Operation::SetCopperStack { stack })
        .map_err(refused)?;
    Ok(board)
}

/// Draws a structure and puts it on a copy of `board`.
///
/// The geometry is built from the environment and the parameters, converted to one batch of
/// operations and applied to a copy of the board; the rule check runs on the board before and
/// after. With `rules_gate` on in the settings, a structure that adds an error to the rule check
/// is refused. Ids that the board already uses are skipped.
pub fn generate(
    env: &Environment,
    board: &Board,
    request: &Request,
    new_id: &mut dyn FnMut() -> Id,
) -> Result<Generated, RfgenError> {
    let (layout, mut report) = kinds::build(request.kind, env, &request.params, &request.nets)?;
    for port in &layout.ports {
        let at = request.placement.apply(port.at)?;
        report.ports.push(PortReport {
            name: port.name.clone(),
            x_nm: at.x_nm.get(),
            y_nm: at.y_nm.get(),
            angle_deg: request.placement.angle(port.angle_deg),
            z0_ohm: port.z0_ohm,
        });
    }
    let mut ids = IdSource::new(board, new_id, env.settings.id_attempts);
    let operation = to_operation(
        &layout,
        board,
        &env.layer,
        &env.substrate.plane_layer,
        &request.placement,
        &env.settings,
        &mut ids,
    )?;
    let mut after = board.clone();
    after.apply(&operation).map_err(refused)?;
    report.counts = Counts {
        polygons: layout.polys.len(),
        arcs: layout.arcs.len(),
        vias: layout.vias.len(),
        parts: layout.parts.len(),
        nets_added: after.nets.len().saturating_sub(board.nets.len()),
    };

    let rules = env.settings.rule_values()?;
    let before_report = check_board(board, &rules);
    let after_report = check_board(&after, &rules);
    let errors_before = before_report.count(Severity::Error);
    let errors_after = after_report.count(Severity::Error);
    let findings: Vec<String> = after_report
        .confirmed
        .iter()
        .map(|f| format!("[{}] {} {}", f.severity.as_str(), f.code, f.location))
        .collect();
    let summary = RuleSummary {
        errors_before,
        errors_after,
        warnings_after: after_report.count(Severity::Warning),
        inconclusive: after_report.inconclusive.len(),
        passes: errors_after <= errors_before,
        findings,
    };
    if env.settings.rules_gate && !summary.passes {
        let first = summary.findings.first().cloned().unwrap_or_default();
        return Err(RfgenError::new(
            RULES_REFUSE,
            format!(
                "{} errors before and {} after; the first finding is {first}",
                summary.errors_before, summary.errors_after
            ),
        ));
    }
    report.rules = Some(summary);
    Ok(Generated {
        operation,
        board: after,
        report,
    })
}

/// The types of structure with their parameters, as text for `phoenix rfgen --list`.
pub fn list_text() -> String {
    let mut out = String::new();
    for kind in Kind::ALL {
        out.push_str(&format!("{:<13} {}\n", kind.name(), kind.summary()));
        for spec in kind.params() {
            let unit = if spec.unit.is_empty() {
                String::new()
            } else {
                format!(" [{}]", spec.unit)
            };
            out.push_str(&format!(
                "    --{:<18}{} {} (default: {})\n",
                spec.name, unit, spec.help, spec.default
            ));
        }
    }
    out
}

/// The same list as JSON.
pub fn list_json() -> serde_json::Value {
    let kinds: Vec<serde_json::Value> = Kind::ALL
        .iter()
        .map(|kind| {
            let params: Vec<serde_json::Value> = kind
                .params()
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "name": p.name,
                        "unit": p.unit,
                        "default": p.default,
                        "help": p.help,
                    })
                })
                .collect();
            serde_json::json!({
                "name": kind.name(),
                "summary": kind.summary(),
                "params": params,
            })
        })
        .collect();
    serde_json::json!({ "kinds": kinds })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::fixtures::{materials, two_layer};
    use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, StepClock};
    use eh3d_phoenix_stackup::StackupSettings;

    fn source() -> impl FnMut() -> Id {
        let mut generator = IdGenerator::new(
            StepClock::new(1_700_000_000_000, 1),
            DeterministicRandom::new(11),
        );
        move || generator.generate().unwrap()
    }

    fn env() -> Environment {
        Environment::new(
            &two_layer(),
            &materials(),
            None,
            1.0,
            RfgenSettings::default(),
            StackupSettings::default(),
        )
        .unwrap()
    }

    #[test]
    fn a_new_board_has_the_copper_layers_of_the_stackup_in_order() {
        let mut next = source();
        let board = new_board(&env(), "test", &mut next).unwrap();
        assert_eq!(board.layers.len(), 2);
        assert_eq!(board.copper_stack.len(), 2);
        let names: Vec<&str> = board
            .copper_stack
            .iter()
            .map(|id| board.layers[id].name.as_str())
            .collect();
        assert_eq!(names, vec!["top", "bottom"]);
        assert!(board.is_consistent(), "{:?}", board.validate());
    }

    #[test]
    fn a_generated_line_goes_on_the_board_and_comes_off_again() {
        let e = env();
        let mut next = source();
        let board = new_board(&e, "test", &mut next).unwrap();
        let request = Request::new(Kind::Line, Params::new());
        let generated = generate(&e, &board, &request, &mut next).unwrap();
        assert_eq!(generated.board.zones.len(), 1);
        assert!(generated.board.is_consistent());
        let mut copy = board.clone();
        let inverse = generated.apply(&mut copy).unwrap();
        assert_eq!(copy, generated.board);
        copy.apply(&inverse).unwrap();
        assert_eq!(copy, board);
        let rules = generated.report.rules.as_ref().unwrap();
        assert!(rules.passes);
        assert_eq!(generated.report.counts.polygons, 1);
        assert_eq!(generated.report.counts.nets_added, 1);
        assert_eq!(generated.report.ports.len(), 2);
    }

    #[test]
    fn the_same_request_gives_the_same_bytes() {
        let e = env();
        let request = Request::new(Kind::Bend, Params::new());
        let mut a = source();
        let board_a = new_board(&e, "test", &mut a).unwrap();
        let first = generate(&e, &board_a, &request, &mut a).unwrap();
        let mut b = source();
        let board_b = new_board(&e, "test", &mut b).unwrap();
        let second = generate(&e, &board_b, &request, &mut b).unwrap();
        assert_eq!(
            first.board.to_bytes().unwrap(),
            second.board.to_bytes().unwrap()
        );
        assert_eq!(first.report.to_json(), second.report.to_json());
    }

    #[test]
    fn a_second_structure_on_the_same_board_gets_new_ids() {
        let e = env();
        let mut next = source();
        let board = new_board(&e, "test", &mut next).unwrap();
        let mut again = source();
        let first = generate(
            &e,
            &board,
            &Request::new(Kind::Line, Params::new()),
            &mut again,
        )
        .unwrap();
        // the same generator would give the same ids again: they are skipped
        let mut third = source();
        let second = generate(
            &e,
            &first.board,
            &Request {
                placement: Placement::new(0, 20_000_000, 0).unwrap(),
                ..Request::new(Kind::Line, Params::new())
            },
            &mut third,
        )
        .unwrap();
        assert_eq!(second.board.zones.len(), 2);
        assert_eq!(second.board.nets.len(), 1);
        assert!(second.board.is_consistent());
    }

    #[test]
    fn the_rule_gate_refuses_a_structure_that_adds_an_error() {
        let settings = RfgenSettings {
            // the line is wider than a track the rules would allow only if the limit is huge
            rule_min_via_diameter_nm: 1_000_000,
            ..RfgenSettings::default()
        };
        let e = Environment::new(
            &two_layer(),
            &materials(),
            None,
            2.4,
            settings,
            StackupSettings::default(),
        )
        .unwrap();
        let mut next = source();
        let board = new_board(&e, "test", &mut next).unwrap();
        // a short-circuited stub has a via of 0.6 mm: below the 1 mm limit
        let request = Request::new(Kind::StubShort, Params::new());
        let refused = generate(&e, &board, &request, &mut next).unwrap_err();
        assert_eq!(refused.code, RULES_REFUSE);
        // with the gate off the structure is made and the report says what is wrong
        let open = RfgenSettings {
            rules_gate: false,
            rule_min_via_diameter_nm: 1_000_000,
            ..RfgenSettings::default()
        };
        let e = Environment::new(
            &two_layer(),
            &materials(),
            None,
            2.4,
            open,
            StackupSettings::default(),
        )
        .unwrap();
        let made = generate(&e, &board, &request, &mut next).unwrap();
        let rules = made.report.rules.as_ref().unwrap();
        assert!(!rules.passes);
        assert_eq!(rules.errors_after, 1);
        assert_eq!(rules.findings.len(), 1);
    }

    #[test]
    fn a_layer_the_board_does_not_have_is_an_error() {
        let e = env();
        let mut next = source();
        let mut board = new_board(&e, "test", &mut next).unwrap();
        for layer in board.layers.values_mut() {
            layer.name = format!("x-{}", layer.name);
        }
        let request = Request::new(Kind::Line, Params::new());
        let error = generate(&e, &board, &request, &mut next).unwrap_err();
        assert_eq!(error.code, codes::LAYER_NOT_FOUND);
    }

    #[test]
    fn a_stackup_file_is_read_with_or_without_lines() {
        let text = r#"{
          "stackup": {"name": "s", "layers": [
            {"kind": "copper", "name": "top", "thickness": {"um": 35.0}},
            {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4"},
            {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}]},
          "materials": [{"name": "fr4", "dk_points": [[1.0, 4.4]], "df_points": [[1.0, 0.02]]}]
        }"#;
        let (stackup, materials) = parse_stackup_file(text).unwrap();
        assert_eq!(stackup.layers.len(), 3);
        assert_eq!(materials.len(), 1);
        let with_lines = text.replace(
            "\"materials\"",
            "\"lines\": [{\"name\": \"a\"}], \"materials\"",
        );
        assert!(parse_stackup_file(&with_lines).is_ok());
        assert_eq!(parse_stackup_file("{").unwrap_err().code, INPUT_UNREADABLE);
        let big = " ".repeat(MAX_STACKUP_BYTES + 1);
        assert_eq!(parse_stackup_file(&big).unwrap_err().code, INPUT_UNREADABLE);
    }

    #[test]
    fn the_list_names_every_type_and_parameter() {
        let text = list_text();
        for kind in Kind::ALL {
            assert!(text.contains(kind.name()), "{}", kind.name());
            for spec in kind.params() {
                assert!(text.contains(&format!("--{}", spec.name)));
            }
        }
        let json = list_json();
        assert_eq!(json["kinds"].as_array().unwrap().len(), 16);
        assert_eq!(json["kinds"][0]["name"], "line");
    }
}
