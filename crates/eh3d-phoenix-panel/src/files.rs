// SPDX-License-Identifier: GPL-3.0-or-later
//! The files of a panel as text: the outline that is routed and the V-score lines as Gerber X2,
//! the fiducials (copper and solder mask opening) as Gerber X2 and the holes as Excellon. The
//! same plan always gives the same bytes; nothing reads the clock.

use std::collections::BTreeMap;

use crate::codes::{PanelError, PANEL_TOO_LARGE};
use crate::layout::{Hole, HoleKind, Line, Mark, PanelPlan, Pt};
use crate::settings::PanelSettings;

/// A file of the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelFile {
    /// The file name, without a folder.
    pub name: String,
    /// The text.
    pub text: String,
}

/// A length in millimetres with at least one decimal and no trailing zeros, exact.
pub fn mm(nm: i64) -> String {
    let magnitude = nm.unsigned_abs();
    let mut text = format!("{}.{:06}", magnitude / 1_000_000, magnitude % 1_000_000);
    while text.ends_with('0') && !text.ends_with(".0") {
        text.pop();
    }
    if nm < 0 && magnitude != 0 {
        text.insert(0, '-');
    }
    text
}

fn gerber_header(function: &str) -> String {
    format!(
        "%TF.GenerationSoftware,EH3D,PHOENIX-PCB,{}*%\n%TF.FileFunction,{function}*%\n%FSLAX46Y46*%\n%MOMM*%\n",
        env!("CARGO_PKG_VERSION")
    )
}

fn gerber_lines(function: &str, width_nm: i64, lines: &[Line]) -> String {
    let mut out = gerber_header(function);
    out.push_str(&format!("%ADD10C,{}*%\n%LPD*%\nD10*\n", mm(width_nm)));
    for line in lines {
        out.push_str(&format!(
            "X{}Y{}D02*\nX{}Y{}D01*\n",
            line.from.x, line.from.y, line.to.x, line.to.y
        ));
    }
    out.push_str("M02*\n");
    out
}

/// The openings of the boards inside the panel, each a closed contour, drawn with the same
/// aperture after the lines of the profile.
fn gerber_profile(width_nm: i64, lines: &[Line], cutouts: &[Vec<Pt>]) -> String {
    let mut out = gerber_lines("Profile,NP", width_nm, lines);
    out.truncate(out.len() - "M02*\n".len());
    for ring in cutouts.iter().filter(|r| r.len() >= 3) {
        for (i, from) in ring.iter().enumerate() {
            let to = ring[(i + 1) % ring.len()];
            out.push_str(&format!(
                "X{}Y{}D02*\nX{}Y{}D01*\n",
                from.x, from.y, to.x, to.y
            ));
        }
    }
    out.push_str("M02*\n");
    out
}

fn gerber_flashes(function: &str, diameter_nm: i64, centres: &[Pt]) -> String {
    let mut out = gerber_header(function);
    out.push_str(&format!("%ADD10C,{}*%\n%LPD*%\nD10*\n", mm(diameter_nm)));
    for at in centres {
        out.push_str(&format!("X{}Y{}D03*\n", at.x, at.y));
    }
    out.push_str("M02*\n");
    out
}

fn drill_file(function: &str, holes: &[Hole]) -> Result<String, PanelError> {
    let mut sorted: Vec<Hole> = holes.to_vec();
    sorted.sort_by_key(|h| (h.diameter, h.at.y, h.at.x));
    let mut diameters: Vec<i64> = sorted.iter().map(|h| h.diameter).collect();
    diameters.dedup();
    if diameters.len() > 99 {
        return Err(PanelError::new(
            PANEL_TOO_LARGE,
            "more than 99 drill diameters",
        ));
    }
    let numbers: BTreeMap<i64, usize> = diameters
        .iter()
        .enumerate()
        .map(|(i, d)| (*d, i + 1))
        .collect();
    let mut out = String::from("M48\n");
    out.push_str(&format!(
        "; #@! TF.GenerationSoftware,EH3D,PHOENIX-PCB,{}\n",
        env!("CARGO_PKG_VERSION")
    ));
    out.push_str(&format!("; #@! TF.FileFunction,{function}\nMETRIC,TZ\n"));
    for (diameter, number) in &numbers {
        out.push_str(&format!("T{number}C{}\n", mm(*diameter)));
    }
    out.push_str("%\n");
    let mut current = 0usize;
    for hole in &sorted {
        let number = numbers[&hole.diameter];
        if number != current {
            out.push_str(&format!("T{number}\n"));
            current = number;
        }
        out.push_str(&format!("X{}Y{}\n", mm(hole.at.x), mm(hole.at.y)));
    }
    out.push_str("M30\n");
    Ok(out)
}

/// Every file of the panel, in a fixed order: the outline, the V-score lines (when there are
/// any), the fiducials (copper and mask, top and bottom as chosen) and the drill files (plated
/// and not plated, when there is a hole of the kind).
pub fn panel_files(
    plan: &PanelPlan,
    settings: &PanelSettings,
) -> Result<Vec<PanelFile>, PanelError> {
    panel_files_with_cutouts(plan, settings, &[])
}

/// The same files, with the openings that the boards have inside their outlines (`cutouts`, each a
/// closed contour in the coordinates of the panel, as `compose` moves them) drawn in the profile
/// after the lines of the panel.
pub fn panel_files_with_cutouts(
    plan: &PanelPlan,
    settings: &PanelSettings,
    cutouts: &[Vec<Pt>],
) -> Result<Vec<PanelFile>, PanelError> {
    let mut files = vec![PanelFile {
        name: "panel_profile.gbr".to_owned(),
        text: gerber_profile(settings.outline_width_nm, &plan.profile, cutouts),
    }];
    if !plan.score.is_empty() {
        files.push(PanelFile {
            name: "panel_vscore.gbr".to_owned(),
            text: gerber_lines("Other,VScore", settings.outline_width_nm, &plan.score),
        });
    }
    if !plan.fiducials.is_empty() {
        let centres: Vec<Pt> = plan.fiducials.iter().map(|m: &Mark| m.at).collect();
        let copper = plan.fiducials[0].copper_diameter;
        let mask = plan.fiducials[0].mask_diameter;
        let sides: &[(&str, &str)] = if settings.fiducials_both_sides {
            &[("top", "Top"), ("bot", "Bot")]
        } else {
            &[("top", "Top")]
        };
        for (suffix, side) in sides {
            files.push(PanelFile {
                name: format!("panel_fiducial_copper_{suffix}.gbr"),
                text: gerber_flashes(
                    &format!("Other,PanelFiducialCopper{side}"),
                    copper,
                    &centres,
                ),
            });
            files.push(PanelFile {
                name: format!("panel_fiducial_mask_{suffix}.gbr"),
                text: gerber_flashes(&format!("Other,PanelFiducialMask{side}"), mask, &centres),
            });
        }
    }
    let plated: Vec<Hole> = plan
        .holes
        .iter()
        .filter(|h| h.kind == HoleKind::Plated)
        .copied()
        .collect();
    let not_plated: Vec<Hole> = plan
        .holes
        .iter()
        .filter(|h| h.kind != HoleKind::Plated)
        .copied()
        .collect();
    if !plated.is_empty() {
        files.push(PanelFile {
            name: "panel_drill_pth.drl".to_owned(),
            text: drill_file("Plated,1,2,PTH", &plated)?,
        });
    }
    if !not_plated.is_empty() {
        files.push(PanelFile {
            name: "panel_drill_npth.drl".to_owned(),
            text: drill_file("NonPlated,1,2,NPTH", &not_plated)?,
        });
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{plan, BoardShape};

    const MM: i64 = 1_000_000;

    fn shape() -> BoardShape {
        BoardShape {
            origin: Pt { x: 0, y: 0 },
            width: 30 * MM,
            height: 20 * MM,
            holes: vec![
                (
                    true,
                    Pt {
                        x: 2 * MM,
                        y: 2 * MM,
                    },
                    800_000,
                ),
                (
                    false,
                    Pt {
                        x: 28 * MM,
                        y: 18 * MM,
                    },
                    3 * MM,
                ),
            ],
        }
    }

    #[test]
    fn lengths_are_written_exactly() {
        assert_eq!(mm(0), "0.0");
        assert_eq!(mm(1_500_000), "1.5");
        assert_eq!(mm(-250_000), "-0.25");
        assert_eq!(mm(3_000_000), "3.0");
        assert_eq!(mm(1), "0.000001");
    }

    #[test]
    fn the_files_of_the_default_panel() {
        let settings = PanelSettings::default();
        let plan = plan(&shape(), &settings).unwrap();
        let files = panel_files(&plan, &settings).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "panel_profile.gbr",
                "panel_fiducial_copper_top.gbr",
                "panel_fiducial_mask_top.gbr",
                "panel_drill_pth.drl",
                "panel_drill_npth.drl"
            ]
        );
        let profile = &files[0].text;
        assert!(profile.contains("%TF.FileFunction,Profile,NP*%\n%FSLAX46Y46*%\n%MOMM*%\n"));
        assert!(profile.contains("%ADD10C,0.1*%"));
        assert!(profile.ends_with("M02*\n"));
        assert_eq!(profile.matches("D01*").count(), plan.profile.len());
        let copper = &files[1].text;
        assert_eq!(copper.matches("D03*").count(), 3);
        assert!(copper.contains("%ADD10C,1.0*%"));
        assert!(files[2].text.contains("%ADD10C,2.0*%"));
        let plated = &files[3].text;
        assert!(plated.starts_with("M48\n; #@! TF.GenerationSoftware,EH3D,PHOENIX-PCB,"));
        assert!(plated.contains("; #@! TF.FileFunction,Plated,1,2,PTH\nMETRIC,TZ\nT1C0.8\n%\nT1\n"));
        assert!(plated.ends_with("M30\n"));
        let not_plated = &files[4].text;
        // mouse bites (0.5), tooling holes (3.0) and the holes of the boards (3.0)
        assert!(not_plated.contains("T1C0.5\nT2C3.0\n%\n"), "{not_plated}");
    }

    #[test]
    fn the_openings_of_the_boards_are_closed_contours_in_the_profile() {
        let settings = PanelSettings::default();
        let plan = plan(&shape(), &settings).unwrap();
        let ring = vec![
            Pt { x: MM, y: MM },
            Pt { x: 3 * MM, y: MM },
            Pt {
                x: 3 * MM,
                y: 2 * MM,
            },
        ];
        let none = panel_files_with_cutouts(&plan, &settings, &[]).unwrap();
        assert_eq!(none, panel_files(&plan, &settings).unwrap());
        // a ring with fewer than three points is not an opening
        let short = vec![Pt { x: 0, y: 0 }, Pt { x: MM, y: 0 }];
        let same = panel_files_with_cutouts(&plan, &settings, &[short]).unwrap();
        assert_eq!(none, same);
        let files = panel_files_with_cutouts(&plan, &settings, &[ring]).unwrap();
        let text = &files[0].text;
        // three more lines, the last one back to the first point, before the end of the file
        assert_eq!(text.matches("D01*").count(), plan.profile.len() + 3);
        assert!(text.ends_with("X3000000Y2000000D02*\nX1000000Y1000000D01*\nM02*\n"));
        assert!(text.contains("X1000000Y1000000D02*\nX3000000Y1000000D01*\n"));
        assert_eq!(text.matches("M02*").count(), 1);
    }

    #[test]
    fn a_v_score_panel_has_a_score_file_and_both_sides_have_fiducials() {
        let settings =
            PanelSettings::parse("joint = v_score\nfiducials_both_sides = true\n").unwrap();
        let plan = plan(&shape(), &settings).unwrap();
        let files = panel_files(&plan, &settings).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"panel_vscore.gbr"));
        assert!(names.contains(&"panel_fiducial_copper_bot.gbr"));
        assert!(names.contains(&"panel_fiducial_mask_bot.gbr"));
        let score = files.iter().find(|f| f.name == "panel_vscore.gbr").unwrap();
        assert!(score.text.contains("%TF.FileFunction,Other,VScore*%"));
        assert_eq!(score.text.matches("D01*").count(), plan.score.len());
    }

    #[test]
    fn the_same_plan_gives_the_same_bytes() {
        let settings = PanelSettings::default();
        let a = panel_files(&plan(&shape(), &settings).unwrap(), &settings).unwrap();
        let b = panel_files(&plan(&shape(), &settings).unwrap(), &settings).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn a_panel_without_holes_has_no_drill_file() {
        let bare = BoardShape {
            holes: Vec::new(),
            ..shape()
        };
        let settings =
            PanelSettings::parse("joint = v_score\ntooling_hole_count = 0\nfiducial_count = 0\n")
                .unwrap();
        let plan = plan(&bare, &settings).unwrap();
        let files = panel_files(&plan, &settings).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["panel_profile.gbr", "panel_vscore.gbr"]);
    }

    #[test]
    fn too_many_diameters_are_refused() {
        let holes: Vec<Hole> = (1..=100)
            .map(|d| Hole {
                kind: HoleKind::NonPlated,
                at: Pt { x: 0, y: 0 },
                diameter: d * 1000,
            })
            .collect();
        assert_eq!(
            drill_file("NonPlated,1,2,NPTH", &holes).unwrap_err().code,
            PANEL_TOO_LARGE
        );
    }
}
