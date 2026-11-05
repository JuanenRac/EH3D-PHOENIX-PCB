// SPDX-License-Identifier: GPL-3.0-or-later
//! The checks of the assembled scene: the boxes of two components that overlap, a component that
//! stands higher (or hangs lower) than the limit, and a component whose box is not inside the
//! outline of the board. All of it is integer arithmetic on the boxes; nothing is judged from the
//! real shape of a component, so a box that is bigger than the part reports a collision the part
//! would not have, and a collision between real parts that are inside each other's box is found.

use crate::codes::{Finding, Severity, COLLISION, LIMIT_EXCEEDED, OUTSIDE_OUTLINE, TOO_TALL};
use crate::geom::{point_in_polygon, quads_overlap};
use crate::scene::{millimetres, Part3d, Scene3d};
use crate::settings::AssemblySettings;

/// Two components whose boxes overlap, by their place in the scene, with the overlap in height.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collision {
    /// The place of the first component in the scene (the one with the lesser reference).
    pub first: usize,
    /// The place of the second component.
    pub second: usize,
    /// How much the two overlap in height, in nanometres.
    pub height_overlap_nm: i64,
}

/// The result of the checks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Checks {
    /// Every pair of components that collide, by reference.
    pub collisions: Vec<Collision>,
    /// The places of the components taller than the limit.
    pub too_tall: Vec<usize>,
    /// The places of the components not inside the outline.
    pub outside: Vec<usize>,
    /// One finding for each of the above (up to the limit of the settings).
    pub findings: Vec<Finding>,
}

/// How far a component stands above the top of the board (or hangs below its bottom), in
/// nanometres; 0 for one that does not reach out.
pub fn reach_nm(part: &Part3d, thickness_nm: i64) -> i64 {
    if part.bottom {
        (-part.z_min).max(0)
    } else {
        (part.z_max - thickness_nm).max(0)
    }
}

/// Runs the checks on a scene.
pub fn check_scene(scene: &Scene3d, settings: &AssemblySettings) -> Checks {
    let parts = &scene.parts;
    let bounds: Vec<_> = parts.iter().map(Part3d::plane_bounds).collect();

    // the boxes that overlap: sorted by the least x, each one is compared with the next ones
    // that start before it ends
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by_key(|i| (bounds[*i].0 .0, *i));
    let mut collisions = Vec::new();
    for (rank, &i) in order.iter().enumerate() {
        let ((_, low_i), (right_i, high_i)) = bounds[i];
        for &j in &order[rank + 1..] {
            let ((left_j, low_j), (_, high_j)) = bounds[j];
            if left_j >= right_i {
                break;
            }
            if high_i.min(high_j) <= low_i.max(low_j) {
                continue;
            }
            let (a, b) = (&parts[i], &parts[j]);
            let height_overlap = a.z_max.min(b.z_max) - a.z_min.max(b.z_min);
            if height_overlap <= settings.min_overlap_nm {
                continue;
            }
            if quads_overlap(&a.corners, &b.corners, settings.min_overlap_nm) {
                let (first, second) = if (&a.reference, i) <= (&b.reference, j) {
                    (i, j)
                } else {
                    (j, i)
                };
                collisions.push(Collision {
                    first,
                    second,
                    height_overlap_nm: height_overlap,
                });
            }
        }
    }
    collisions.sort_by_key(|c| {
        (
            parts[c.first].reference.clone(),
            parts[c.second].reference.clone(),
            c.first,
            c.second,
        )
    });

    let mut too_tall = Vec::new();
    if settings.max_component_height_nm > 0 {
        for (index, part) in parts.iter().enumerate() {
            if reach_nm(part, scene.thickness_nm) > settings.max_component_height_nm {
                too_tall.push(index);
            }
        }
    }
    let mut outside = Vec::new();
    if settings.check_outline && scene.outline.len() >= 3 {
        for (index, part) in parts.iter().enumerate() {
            if !part
                .corners
                .iter()
                .all(|c| point_in_polygon(*c, &scene.outline))
            {
                outside.push(index);
            }
        }
    }

    let mut findings = Vec::new();
    let limit = settings.max_findings;
    for c in collisions.iter().take(limit) {
        let (a, b) = (&parts[c.first], &parts[c.second]);
        findings.push(Finding::new(
            COLLISION,
            Severity::Error,
            Some(&format!("{} and {}", a.reference, b.reference)),
            format!("overlap in height {} mm", millimetres(c.height_overlap_nm)),
        ));
    }
    for &index in too_tall.iter().take(limit) {
        let part = &parts[index];
        findings.push(Finding::new(
            TOO_TALL,
            Severity::Error,
            Some(&part.reference),
            format!(
                "{} mm {} the board, the limit is {} mm",
                millimetres(reach_nm(part, scene.thickness_nm)),
                if part.bottom { "below" } else { "above" },
                millimetres(settings.max_component_height_nm)
            ),
        ));
    }
    for &index in outside.iter().take(limit) {
        findings.push(Finding::new(
            OUTSIDE_OUTLINE,
            Severity::Warning,
            Some(&parts[index].reference),
            "a corner of the box is outside the outline",
        ));
    }
    for (what, count) in [
        ("collisions", collisions.len()),
        ("components taller than the limit", too_tall.len()),
        ("components outside the outline", outside.len()),
    ] {
        if count > limit {
            findings.push(Finding::new(
                LIMIT_EXCEEDED,
                Severity::Warning,
                None,
                format!("{count} {what}, {limit} listed"),
            ));
        }
    }
    Checks {
        collisions,
        too_tall,
        outside,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::rect_corners;
    use crate::scene::PartSource;

    const MM: i64 = 1_000_000;

    fn part(
        reference: &str,
        min: (i64, i64),
        max: (i64, i64),
        z: (i64, i64),
        bottom: bool,
    ) -> Part3d {
        Part3d {
            reference: reference.to_owned(),
            footprint: "F".to_owned(),
            bottom,
            corners: rect_corners(min, max),
            z_min: z.0,
            z_max: z.1,
            source: PartSource::Table,
        }
    }

    fn scene(parts: Vec<Part3d>) -> Scene3d {
        Scene3d {
            name: "b".to_owned(),
            outline: vec![(0, 0), (20 * MM, 0), (20 * MM, 20 * MM), (0, 20 * MM)],
            thickness_nm: 1_600_000,
            parts,
            findings: Vec::new(),
        }
    }

    const T: i64 = 1_600_000;

    #[test]
    fn two_boxes_that_overlap_in_the_plane_and_in_height_collide() {
        let s = scene(vec![
            part("R2", (4 * MM, 4 * MM), (8 * MM, 8 * MM), (T, T + MM), false),
            part(
                "R1",
                (6 * MM, 6 * MM),
                (10 * MM, 10 * MM),
                (T, T + MM),
                false,
            ),
            part(
                "R3",
                (12 * MM, 12 * MM),
                (14 * MM, 14 * MM),
                (T, T + MM),
                false,
            ),
        ]);
        let checks = check_scene(&s, &AssemblySettings::default());
        assert_eq!(checks.collisions.len(), 1);
        let c = &checks.collisions[0];
        assert_eq!(
            (
                s.parts[c.first].reference.as_str(),
                s.parts[c.second].reference.as_str()
            ),
            ("R1", "R2")
        );
        assert_eq!(c.height_overlap_nm, MM);
        assert_eq!(checks.findings[0].subject.as_deref(), Some("R1 and R2"));
        assert_eq!(checks.findings[0].code, COLLISION);
    }

    #[test]
    fn boxes_that_only_touch_or_sit_one_over_the_other_do_not_collide() {
        let s = scene(vec![
            part("A", (2 * MM, 2 * MM), (6 * MM, 6 * MM), (T, T + MM), false),
            part("B", (6 * MM, 2 * MM), (9 * MM, 6 * MM), (T, T + MM), false),
            part(
                "C",
                (2 * MM, 2 * MM),
                (6 * MM, 6 * MM),
                (T + MM, T + 2 * MM),
                false,
            ),
            part("D", (2 * MM, 2 * MM), (6 * MM, 6 * MM), (-MM, 0), true),
        ]);
        let checks = check_scene(&s, &AssemblySettings::default());
        assert!(checks.collisions.is_empty(), "{:?}", checks.collisions);
    }

    #[test]
    fn a_part_that_goes_through_the_board_collides_with_one_on_the_other_side() {
        let s = scene(vec![
            part(
                "TH",
                (2 * MM, 2 * MM),
                (4 * MM, 4 * MM),
                (-2 * MM, T + MM),
                false,
            ),
            part("BT", (3 * MM, 3 * MM), (5 * MM, 5 * MM), (-MM, 0), true),
        ]);
        let checks = check_scene(&s, &AssemblySettings::default());
        assert_eq!(checks.collisions.len(), 1);
        assert_eq!(checks.collisions[0].height_overlap_nm, MM);
    }

    #[test]
    fn the_height_is_measured_from_the_face_of_the_board_on_each_side() {
        let s = scene(vec![
            part("TOP", (MM, MM), (2 * MM, 2 * MM), (T, T + 30 * MM), false),
            part(
                "LOW",
                (4 * MM, 4 * MM),
                (5 * MM, 5 * MM),
                (T, T + 10 * MM),
                false,
            ),
            part(
                "BOT",
                (7 * MM, 7 * MM),
                (8 * MM, 8 * MM),
                (-26 * MM, 0),
                true,
            ),
        ]);
        assert_eq!(reach_nm(&s.parts[0], T), 30 * MM);
        let checks = check_scene(&s, &AssemblySettings::default());
        assert_eq!(checks.too_tall, vec![0, 2]);
        assert_eq!(checks.findings[0].code, TOO_TALL);
        assert!(checks.findings[0]
            .detail
            .contains("30.000000 mm above the board"));
        assert!(checks.findings[1]
            .detail
            .contains("26.000000 mm below the board"));
        let none = AssemblySettings {
            max_component_height_nm: 0,
            ..AssemblySettings::default()
        };
        assert!(check_scene(&s, &none).too_tall.is_empty());
    }

    #[test]
    fn a_box_that_leaves_the_outline_is_reported_unless_the_check_is_off() {
        let s = scene(vec![
            part("IN", (MM, MM), (3 * MM, 3 * MM), (T, T + MM), false),
            part(
                "OUT",
                (18 * MM, 18 * MM),
                (22 * MM, 19 * MM),
                (T, T + MM),
                false,
            ),
        ]);
        let checks = check_scene(&s, &AssemblySettings::default());
        assert_eq!(checks.outside, vec![1]);
        assert_eq!(checks.findings[0].code, OUTSIDE_OUTLINE);
        let off = AssemblySettings {
            check_outline: false,
            ..AssemblySettings::default()
        };
        assert!(check_scene(&s, &off).outside.is_empty());
    }

    #[test]
    fn the_findings_of_one_kind_stop_at_the_limit_with_a_note() {
        let parts: Vec<Part3d> = (0..4)
            .map(|i| {
                part(
                    &format!("P{i}"),
                    (MM, MM),
                    (3 * MM, 3 * MM),
                    (T, T + MM),
                    false,
                )
            })
            .collect();
        let settings = AssemblySettings {
            max_findings: 2,
            ..AssemblySettings::default()
        };
        let checks = check_scene(&scene(parts), &settings);
        assert_eq!(checks.collisions.len(), 6);
        assert_eq!(checks.findings.len(), 3);
        assert_eq!(checks.findings[2].code, LIMIT_EXCEEDED);
        assert!(checks.findings[2]
            .detail
            .starts_with("6 collisions, 2 listed"));
    }
}
