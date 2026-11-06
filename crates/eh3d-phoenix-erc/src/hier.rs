// SPDX-License-Identifier: GPL-3.0-or-later
//! The rule check of a schematic that has sheet symbols: the hierarchy is drawn out first (every
//! instance of every sheet, with references such as `U1/R1` and nets such as `/U1/VOUT`), and the
//! checks run on that flat schematic. A schematic without sheet symbols is checked as it is.

use eh3d_phoenix_schematic::{EditError, HierConfig, Schematic};

use crate::checks::{check, check_with_designators};
use crate::config::ErcConfig;
use crate::report::Report;

/// Checks a schematic, drawing out its hierarchy first when it has sheet symbols. The error is a
/// refusal to expand (the supply of ids ran out).
pub fn check_hier(
    schematic: &Schematic,
    hier: &HierConfig,
    config: &ErcConfig,
) -> Result<Report, EditError> {
    if !schematic.has_hierarchy() {
        return Ok(check(schematic, config));
    }
    let expanded = schematic.expand(hier)?;
    Ok(check_with_designators(
        &expanded.schematic,
        config,
        Some(&expanded.designators),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::{Nm, Point};
    use eh3d_phoenix_schematic::{
        EditOp, Editor, Id, PinDef, PinKind, PortDir, Schematic, SymbolDef,
    };

    fn pt(x_mm: i64, y_mm: i64) -> Point {
        Point::from_nm(x_mm * 1_000_000, y_mm * 1_000_000).unwrap()
    }

    fn two_pin() -> SymbolDef {
        let pin = |number: &str, y: i64| PinDef {
            number: number.into(),
            at: pt(0, y),
            kind: Some(PinKind::Passive),
        };
        SymbolDef {
            name: "R".into(),
            pins: vec![pin("1", 5), pin("2", -5)],
        }
    }

    fn run(ed: &mut Editor, op: EditOp) -> Vec<Id> {
        ed.apply(&op).unwrap().created
    }

    /// A sheet with one resistor whose pin 1 is the port IN and pin 2 the port OUT, shown twice
    /// by the sheet symbols A and B on a top sheet that joins A.OUT to B.IN. With `spare` the
    /// sheet also has a resistor whose pins touch nothing.
    fn design(spare: bool) -> Schematic {
        let mut ed = Editor::new(Schematic::default());
        let sheet = |ed: &mut Editor, name: &str| {
            run(
                ed,
                EditOp::AddSheet {
                    name: name.into(),
                    width: Nm::new(297_000_000).unwrap(),
                    height: Nm::new(210_000_000).unwrap(),
                },
            )[0]
        };
        let top = sheet(&mut ed, "Top");
        let child = sheet(&mut ed, "Stage");
        let place = |ed: &mut Editor, on: Id, at: Point, refdes: &str| {
            run(
                ed,
                EditOp::PlaceSymbol {
                    symbol: "R".into(),
                    define: Some(two_pin()),
                    sheet: on,
                    at,
                    rotation: None,
                    mirrored: false,
                    refdes: Some(refdes.into()),
                    value: Some("1k".into()),
                },
            );
        };
        place(&mut ed, child, pt(50, 80), "R1");
        if spare {
            place(&mut ed, child, pt(100, 80), "R2");
        }
        for (name, dir, at) in [
            ("IN", PortDir::Input, pt(50, 85)),
            ("OUT", PortDir::Output, pt(50, 75)),
        ] {
            run(
                &mut ed,
                EditOp::AddSheetPort {
                    sheet: child,
                    name: name.into(),
                    dir,
                    at,
                },
            );
        }
        let mut blocks = Vec::new();
        for (name, x) in [("A", 20), ("B", 80)] {
            blocks.push(
                run(
                    &mut ed,
                    EditOp::AddSheetBlock {
                        sheet: top,
                        child,
                        name: name.into(),
                        at: pt(x, 50),
                        width: Nm::new(20_000_000).unwrap(),
                        height: Nm::new(20_000_000).unwrap(),
                    },
                )[0],
            );
        }
        for (block, x) in blocks.iter().zip([20, 80]) {
            for (name, dir, dx) in [("IN", PortDir::Input, 0), ("OUT", PortDir::Output, 20)] {
                run(
                    &mut ed,
                    EditOp::AddHierPin {
                        block: *block,
                        name: name.into(),
                        dir,
                        at: pt(x + dx, 60),
                    },
                );
            }
        }
        // A.OUT at (40, 60) to B.IN at (80, 60)
        run(
            &mut ed,
            EditOp::AddWire {
                sheet: top,
                path: vec![pt(40, 60), pt(80, 60)],
            },
        );
        ed.into_schematic()
    }

    fn unconnected(report: &Report) -> Vec<String> {
        report
            .confirmed
            .iter()
            .filter(|f| f.code == "PHOENIX-ERC-001")
            .map(|f| format!("{} {}", f.location, f.detail))
            .collect()
    }

    #[test]
    fn the_pins_of_every_instance_are_connected_through_the_ports() {
        let s = design(false);
        assert!(s.is_consistent(), "{:?}", s.validate());
        let report = check_hier(&s, &HierConfig::default(), &ErcConfig::default()).unwrap();
        // every pin is on a net: the ports name the nets of the pins that nothing wires above
        let loose = unconnected(&report);
        assert!(loose.is_empty(), "{loose:?}");
        // A.OUT and B.IN are wired, so the net /A/OUT is one net with the two resistors
        let x = s.expand(&HierConfig::default()).unwrap();
        let net = x
            .schematic
            .nets
            .values()
            .find(|n| n.pins.len() == 2)
            .expect("a net with two pins");
        let mut refs: Vec<&str> = net
            .pins
            .iter()
            .map(|p| x.schematic.parts[&p.part].refdes.as_str())
            .collect();
        refs.sort_unstable();
        assert_eq!(refs, vec!["A/R1", "B/R1"]);
    }

    #[test]
    fn a_finding_names_the_part_by_its_hierarchical_reference() {
        let s = design(true);
        let report = check_hier(&s, &HierConfig::default(), &ErcConfig::default()).unwrap();
        let loose = unconnected(&report);
        for expected in ["A/R2", "B/R2"] {
            assert!(
                loose.iter().any(|l| l.contains(expected)),
                "{expected} in {loose:?}"
            );
        }
        // the two pins of each spare resistor
        assert_eq!(loose.iter().filter(|l| l.contains("/R2")).count(), 4);
    }

    #[test]
    fn a_schematic_without_sheet_symbols_is_checked_as_it_is() {
        let s = Schematic::default();
        let a = check_hier(&s, &HierConfig::default(), &ErcConfig::default()).unwrap();
        assert_eq!(a, check(&s, &ErcConfig::default()));
    }
}
