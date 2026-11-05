// SPDX-License-Identifier: GPL-3.0-or-later
//! The assembly on small boards made by hand, with the numbers worked out on paper.

use std::cell::Cell;

use eh3d_phoenix_assembly3d::codes::{
    COLLISION, EXPORT_SIMPLIFIED, LIMIT_EXCEEDED, MODEL_MISSING, OUTSIDE_OUTLINE, STEP_INVALID,
    STEP_REPLACED, THICKNESS_DEFAULT, TOO_TALL,
};
use eh3d_phoenix_assembly3d::{
    build_scene, check_scene, export_note, scene_json, scene_obj, scene_step, scene_text,
    AssemblyError, AssemblySettings, ModelTable, PartSource, Scene3d, SolidKind, StepSolid,
};
use eh3d_phoenix_board::{Board, Component, Footprint, Id, Layer, LayerKind, Side};
use eh3d_phoenix_model::{ObjectId, Point, Udeg};
use eh3d_phoenix_rules::Lang;

const MM: i64 = 1_000_000;

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

fn component(refdes: &str, footprint: u32, at: (i64, i64), degrees: i64, side: Side) -> Component {
    Component {
        refdes: refdes.to_owned(),
        value: String::new(),
        symbol: None,
        footprint: id(footprint),
        at: pt(at.0, at.1),
        rotation: Udeg::new(degrees * 1_000_000).unwrap(),
        side,
    }
}

/// A board of 20 mm by 20 mm with two footprints, R0603 (10) and DIP8 (11).
fn board(components: Vec<Component>) -> Board {
    let mut b = Board {
        name: "asm".to_owned(),
        outline: vec![
            pt(0, 0),
            pt(20 * MM, 0),
            pt(20 * MM, 20 * MM),
            pt(0, 20 * MM),
        ],
        ..Board::default()
    };
    b.layers.insert(
        id(1),
        Layer {
            name: "F.Cu".to_owned(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    for (n, name) in [(10, "R0603"), (11, "DIP8")] {
        b.footprints.insert(
            id(n),
            Footprint {
                name: name.to_owned(),
                pads: Default::default(),
            },
        );
    }
    for (index, c) in components.into_iter().enumerate() {
        b.components.insert(id(100 + index as u32), c);
    }
    b
}

const TABLE: &str = "footprint,model,dx_mm,dy_mm,dz_mm,rot_z_deg\n\
R0603,box:1.6x0.8x0.5,,,,\n\
DIP8,dip8.step,-4.75,-3.2,,\n";

fn dip_solid() -> StepSolid {
    StepSolid {
        min: [0, 0, 0],
        max: [9_500_000, 6_400_000, 3_300_000],
        kind: SolidKind::Box,
        unknown: Vec::new(),
    }
}

fn resolver_ok(name: &str) -> Result<StepSolid, AssemblyError> {
    match name {
        "dip8.step" => Ok(dip_solid()),
        other => Err(AssemblyError::new(
            STEP_INVALID,
            format!("{other}: no such model"),
        )),
    }
}

fn scene_of(components: Vec<Component>, thickness: Option<i64>) -> Scene3d {
    let table = ModelTable::parse(TABLE, 100).unwrap();
    build_scene(
        &board(components),
        thickness,
        &table,
        &AssemblySettings::default(),
        &mut resolver_ok,
    )
    .unwrap()
}

#[test]
fn a_box_of_the_table_stands_on_the_board_with_its_height_from_the_stackup() {
    let scene = scene_of(
        vec![component("R1", 10, (5 * MM, 5 * MM), 0, Side::Top)],
        Some(1_200_000),
    );
    assert_eq!(scene.thickness_nm, 1_200_000);
    assert!(scene.findings.is_empty());
    let r1 = &scene.parts[0];
    let bounds = r1.plane_bounds();
    assert_eq!(bounds, ((4_200_000, 4_600_000), (5_800_000, 5_400_000)));
    assert_eq!((r1.z_min, r1.z_max), (1_200_000, 1_700_000));
    assert_eq!(r1.source, PartSource::Table);
    assert!(!r1.bottom);
}

#[test]
fn without_a_stackup_the_default_thickness_is_used_and_said() {
    let scene = scene_of(
        vec![component("R1", 10, (5 * MM, 5 * MM), 0, Side::Top)],
        None,
    );
    assert_eq!(scene.thickness_nm, 1_600_000);
    assert_eq!(scene.findings.len(), 1);
    assert_eq!(scene.findings[0].code, THICKNESS_DEFAULT);
    assert_eq!(scene.findings[0].detail, "1.600000 mm");
    let zero = scene_of(vec![], Some(0));
    assert_eq!(zero.findings[0].code, THICKNESS_DEFAULT);
}

#[test]
fn a_step_box_is_moved_by_the_table_and_turned_with_the_component() {
    let scene = scene_of(
        vec![component("U1", 11, (15 * MM, 15 * MM), 90, Side::Top)],
        Some(1_600_000),
    );
    let u1 = &scene.parts[0];
    assert_eq!(u1.source, PartSource::StepBox);
    // 9.5 by 6.4 mm centred by the table, turned a quarter: 6.4 along x and 9.5 along y
    assert_eq!(
        u1.plane_bounds(),
        ((11_800_000, 10_250_000), (18_200_000, 19_750_000))
    );
    assert_eq!((u1.z_min, u1.z_max), (1_600_000, 4_900_000));
    assert!(scene.findings.is_empty());
}

#[test]
fn a_component_on_the_bottom_is_mirrored_along_x_and_hangs_below_the_board() {
    let table = ModelTable::parse("footprint,model,dx_mm\nR0603,box:1.6x0.8x0.5,1\n", 10).unwrap();
    let b = board(vec![
        component("T1", 10, (5 * MM, 5 * MM), 0, Side::Top),
        component("B1", 10, (5 * MM, 5 * MM), 0, Side::Bottom),
    ]);
    let scene = build_scene(
        &b,
        Some(1_600_000),
        &table,
        &AssemblySettings::default(),
        &mut resolver_ok,
    )
    .unwrap();
    let top = scene.parts.iter().find(|p| p.reference == "T1").unwrap();
    let bottom = scene.parts.iter().find(|p| p.reference == "B1").unwrap();
    assert_eq!(top.plane_bounds().0 .0, 5_200_000);
    assert_eq!(top.plane_bounds().1 .0, 6_800_000);
    assert_eq!(bottom.plane_bounds().0 .0, 3_200_000);
    assert_eq!(bottom.plane_bounds().1 .0, 4_800_000);
    assert_eq!((bottom.z_min, bottom.z_max), (-500_000, 0));
    assert!(bottom.bottom);
}

#[test]
fn a_footprint_with_no_model_is_left_out_with_one_finding_that_lists_its_references() {
    let table = ModelTable::parse("footprint,model\nDIP8,dip8.step\n", 10).unwrap();
    let comps: Vec<Component> = (1..=12)
        .map(|n| component(&format!("R{n}"), 10, (MM * n, 5 * MM), 0, Side::Top))
        .collect();
    let b = board(comps);
    let scene = build_scene(
        &b,
        Some(1_600_000),
        &table,
        &AssemblySettings::default(),
        &mut resolver_ok,
    )
    .unwrap();
    assert!(scene.parts.is_empty());
    let f: Vec<_> = scene
        .findings
        .iter()
        .filter(|f| f.code == MODEL_MISSING)
        .collect();
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].subject.as_deref(), Some("R0603"));
    assert_eq!(
        f[0].detail,
        "R1, R10, R11, R12, R2, R3, R4, R5, R6, R7 and 2 more"
    );
}

#[test]
fn a_step_that_is_not_a_box_is_replaced_by_its_box_with_a_finding_and_read_once() {
    let calls = Cell::new(0);
    let mut resolve = |name: &str| -> Result<StepSolid, AssemblyError> {
        calls.set(calls.get() + 1);
        let mut solid = dip_solid();
        solid.kind = SolidKind::Curved;
        assert_eq!(name, "dip8.step");
        Ok(solid)
    };
    let table = ModelTable::parse(TABLE, 100).unwrap();
    let b = board(vec![
        component("U1", 11, (5 * MM, 5 * MM), 0, Side::Top),
        component("U2", 11, (15 * MM, 15 * MM), 0, Side::Top),
    ]);
    let scene = build_scene(
        &b,
        Some(1_600_000),
        &table,
        &AssemblySettings::default(),
        &mut resolve,
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(scene.parts.len(), 2);
    assert_eq!(
        scene.parts[0].source,
        PartSource::StepEnclosing(SolidKind::Curved)
    );
    let f: Vec<_> = scene
        .findings
        .iter()
        .filter(|f| f.code == STEP_REPLACED)
        .collect();
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].subject.as_deref(), Some("dip8.step"));
    assert_eq!(f[0].detail, "Curved");
}

#[test]
fn a_model_that_cannot_be_read_leaves_its_components_out_with_one_finding() {
    let table = ModelTable::parse("footprint,model\nDIP8,missing.step\n", 10).unwrap();
    let b = board(vec![
        component("U1", 11, (5 * MM, 5 * MM), 0, Side::Top),
        component("U2", 11, (15 * MM, 15 * MM), 0, Side::Top),
    ]);
    let scene = build_scene(
        &b,
        Some(1_600_000),
        &table,
        &AssemblySettings::default(),
        &mut resolver_ok,
    )
    .unwrap();
    assert!(scene.parts.is_empty());
    let f: Vec<_> = scene
        .findings
        .iter()
        .filter(|f| f.code == STEP_INVALID)
        .collect();
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].subject.as_deref(), Some("missing.step"));
}

#[test]
fn too_many_components_are_refused() {
    let table = ModelTable::parse(TABLE, 100).unwrap();
    let settings = AssemblySettings {
        max_components: 1,
        ..AssemblySettings::default()
    };
    let b = board(vec![
        component("R1", 10, (5 * MM, 5 * MM), 0, Side::Top),
        component("R2", 10, (9 * MM, 5 * MM), 0, Side::Top),
    ]);
    let e = build_scene(&b, None, &table, &settings, &mut resolver_ok).unwrap_err();
    assert_eq!(e.code, LIMIT_EXCEEDED);
}

#[test]
fn the_checks_find_the_collision_the_tall_part_and_the_part_off_the_board() {
    let scene = scene_of(
        vec![
            component("R1", 10, (5 * MM, 5 * MM), 0, Side::Top),
            component("R2", 10, (5_500_000, 5 * MM), 0, Side::Top),
            component("R3", 10, (19_700_000, 5 * MM), 0, Side::Top),
            component("U1", 11, (12 * MM, 12 * MM), 0, Side::Top),
        ],
        Some(1_600_000),
    );
    let settings = AssemblySettings {
        max_component_height_nm: 3_000_000,
        ..AssemblySettings::default()
    };
    let checks = check_scene(&scene, &settings);
    assert_eq!(checks.collisions.len(), 1);
    let c = &checks.collisions[0];
    assert_eq!(scene.parts[c.first].reference, "R1");
    assert_eq!(scene.parts[c.second].reference, "R2");
    assert_eq!(c.height_overlap_nm, 500_000);
    // U1 stands 3.3 mm above the board and the limit is 3 mm
    assert_eq!(checks.too_tall.len(), 1);
    assert_eq!(scene.parts[checks.too_tall[0]].reference, "U1");
    // R3 reaches 20.5 mm on an outline that ends at 20
    assert_eq!(checks.outside.len(), 1);
    assert_eq!(scene.parts[checks.outside[0]].reference, "R3");
    let codes: Vec<&str> = checks.findings.iter().map(|f| f.code).collect();
    assert_eq!(codes, vec![COLLISION, TOO_TALL, OUTSIDE_OUTLINE]);
}

#[test]
fn the_exports_are_the_same_every_time_and_hold_the_board_and_every_component() {
    let scene = scene_of(
        vec![
            component("R1", 10, (5 * MM, 5 * MM), 0, Side::Top),
            component("U1", 11, (15 * MM, 15 * MM), 90, Side::Bottom),
        ],
        Some(1_600_000),
    );
    let obj = scene_obj(&scene);
    assert_eq!(obj, scene_obj(&scene));
    for name in ["o board", "o R1", "o U1"] {
        assert_eq!(obj.lines().filter(|l| *l == name).count(), 1, "{name}");
    }
    assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), 24);
    assert_eq!(obj.lines().filter(|l| l.starts_with("f ")).count(), 18);
    let step = scene_step(&scene);
    assert_eq!(step, scene_step(&scene));
    assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 3);
    assert!(step.contains("MANIFOLD_SOLID_BREP('board',"));
    let note = export_note(&scene);
    assert_eq!(note.code, EXPORT_SIMPLIFIED);
    assert!(note.detail.starts_with("2 components as boxes"));
}

#[test]
fn the_reports_name_the_components_the_checks_and_the_findings() {
    let scene = scene_of(
        vec![
            component("R1", 10, (5 * MM, 5 * MM), 0, Side::Top),
            component("R2", 10, (5_500_000, 5 * MM), 0, Side::Top),
            component("U9", 12, (1, 1), 0, Side::Top),
        ],
        None,
    );
    let checks = check_scene(&scene, &AssemblySettings::default());
    let note = [export_note(&scene)];
    let text = scene_text(&scene, &checks, &note, Lang::Pt);
    assert!(text.starts_with("board asm: 1.600000 mm thick, 2 components with a box\n"));
    assert!(text.contains("checks: 1 collisions, 0 too tall, 0 outside the outline\n"));
    assert!(text.contains("error: PHOENIX-3D-010: Os componentes colidem [R1 and R2]"));
    assert!(text.contains("info: PHOENIX-3D-013: A espessura da placa é desconhecida"));
    let json = scene_json(&scene, &checks, &note);
    assert_eq!(json, scene_json(&scene, &checks, &note));
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["format"], "phoenix-assembly3d/1");
    assert_eq!(value["board"]["thickness_nm"], 1_600_000);
    assert_eq!(value["parts"][0]["reference"], "R1");
    assert_eq!(value["parts"][0]["z_max_nm"], 2_100_000);
    assert_eq!(value["collisions"][0]["second"], "R2");
    assert_eq!(value["parts"][0]["corners_nm"][0][0], 4_200_000);
    assert!(json.ends_with("}\n"));
}
