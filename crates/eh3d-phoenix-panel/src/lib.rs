// SPDX-License-Identifier: GPL-3.0-or-later
//! Panelization of a board (a prototype).
//!
//! * [`settings`]: the sizes and counts of the panel, read from `config/panel.cfg`;
//! * [`layout`]: a matrix of boards with a gap and a frame, the tabs with a row of mouse bite
//!   holes or the V-score lines, the fiducials and the tooling holes, in integer nanometres;
//! * [`files`]: the outline and the V-score lines as Gerber X2, the fiducials as Gerber X2 and the
//!   holes (those of the boards, the mouse bites and the tooling holes) as Excellon;
//! * [`codes`]: the stable codes `PHOENIX-PANEL-<NNN>` with their titles in the eight languages.
//!
//! The panel is a plan and some text; it does not copy the copper, the mask or the legend of the
//! boards into one file per layer, and nothing has been checked by a manufacturer.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod files;
pub mod layout;
pub mod settings;

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

pub use codes::{title, Note, PanelError, TITLES};
pub use files::{panel_files, PanelFile};
pub use layout::{plan, BoardShape, Hole, HoleKind, Line, Mark, PanelPlan, Pt};
pub use settings::{Frame, Joint, PanelSettings};

fn mm_text(nm: i64) -> String {
    files::mm(nm)
}

/// The summary of a plan as text, with its notes in a language.
pub fn summary_text(
    plan: &PanelPlan,
    settings: &PanelSettings,
    files: &[PanelFile],
    lang: Lang,
) -> String {
    let mut out = format!(
        "panel {} mm by {} mm, {} x {} boards\n",
        mm_text(plan.width),
        mm_text(plan.height),
        plan.columns,
        plan.rows
    );
    match settings.joint {
        Joint::MouseBites => out.push_str(&format!(
            "joint: mouse bites, {} tabs, {} holes\n",
            plan.tabs.len(),
            plan.holes_of(HoleKind::MouseBite)
        )),
        Joint::VScore => out.push_str(&format!("joint: V-score, {} lines\n", plan.score.len())),
    }
    out.push_str(&format!(
        "holes: {} plated, {} not plated, {} tooling; fiducials: {}\n",
        plan.holes_of(HoleKind::Plated),
        plan.holes_of(HoleKind::NonPlated),
        plan.holes_of(HoleKind::Tooling),
        plan.fiducials.len()
    ));
    for file in files {
        out.push_str(&format!("file {} ({} bytes)\n", file.name, file.text.len()));
    }
    for note in &plan.notes {
        out.push_str(&format!("note {}\n", note.message(lang)));
    }
    out
}

/// The summary of a plan as JSON (lengths in nanometres).
pub fn summary_json(plan: &PanelPlan, settings: &PanelSettings, files: &[PanelFile]) -> Value {
    let boards: Vec<Value> = plan
        .origins
        .iter()
        .map(|p| json!({ "x_nm": p.x, "y_nm": p.y }))
        .collect();
    let marks: Vec<Value> = plan
        .fiducials
        .iter()
        .map(|m| json!({ "x_nm": m.at.x, "y_nm": m.at.y, "copper_nm": m.copper_diameter, "mask_nm": m.mask_diameter }))
        .collect();
    let notes: Vec<Value> = plan
        .notes
        .iter()
        .map(|n| json!({ "code": n.code, "detail": n.detail }))
        .collect();
    let names: Vec<Value> = files.iter().map(|f| json!(f.name)).collect();
    let joint = match settings.joint {
        Joint::MouseBites => "mouse_bites",
        Joint::VScore => "v_score",
    };
    json!({
        "width_nm": plan.width,
        "height_nm": plan.height,
        "columns": plan.columns,
        "rows": plan.rows,
        "joint": joint,
        "boards": boards,
        "tabs": plan.tabs.len(),
        "score_lines": plan.score.len(),
        "profile_lines": plan.profile.len(),
        "holes": {
            "plated": plan.holes_of(HoleKind::Plated),
            "not_plated": plan.holes_of(HoleKind::NonPlated),
            "mouse_bites": plan.holes_of(HoleKind::MouseBite),
            "tooling": plan.holes_of(HoleKind::Tooling),
        },
        "fiducials": marks,
        "files": names,
        "notes": notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape() -> BoardShape {
        BoardShape {
            origin: Pt { x: 0, y: 0 },
            width: 30_000_000,
            height: 20_000_000,
            holes: vec![(
                true,
                Pt {
                    x: 2_000_000,
                    y: 2_000_000,
                },
                800_000,
            )],
        }
    }

    #[test]
    fn the_summary_names_the_size_the_joint_and_the_files() {
        let settings = PanelSettings::default();
        let plan = plan(&shape(), &settings).unwrap();
        let files = panel_files(&plan, &settings).unwrap();
        let text = summary_text(&plan, &settings, &files, Lang::En);
        assert!(
            text.starts_with("panel 82.0 mm by 62.0 mm, 2 x 2 boards\n"),
            "{text}"
        );
        assert!(text.contains("joint: mouse bites, "), "{text}");
        assert!(
            text.contains("holes: 4 plated, 0 not plated, 4 tooling; fiducials: 3\n"),
            "{text}"
        );
        assert!(text.contains("file panel_profile.gbr ("), "{text}");
        let value = summary_json(&plan, &settings, &files);
        assert_eq!(value["columns"], 2);
        assert_eq!(value["joint"], "mouse_bites");
        assert_eq!(value["boards"].as_array().unwrap().len(), 4);
        assert_eq!(value["holes"]["plated"], 4);
        assert_eq!(value["fiducials"][0]["x_nm"], 16_000_000);
        assert_eq!(value["width_nm"], 82_000_000);
    }

    #[test]
    fn the_notes_come_in_the_language_asked() {
        let settings = PanelSettings::parse("frame = none\n").unwrap();
        let plan = plan(&shape(), &settings).unwrap();
        let text = summary_text(&plan, &settings, &[], Lang::De);
        assert!(
            text.contains("note PHOENIX-PANEL-011: Marken übersprungen"),
            "{text}"
        );
    }
}
