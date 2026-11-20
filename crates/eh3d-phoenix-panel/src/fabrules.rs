// SPDX-License-Identifier: GPL-3.0-or-later
//! The rules of a board maker for a panel, as data, and the check of a plan against them.
//!
//! The file `config/panel_fab.cfg` holds the figures: for V-cuts the least size of the panel, the
//! least thickness of the board, the least distance from copper to the cut and from cut to cut;
//! for routed tabs the least gap, the diameter, the distance between the edges and the number of
//! the break-away holes, and the least tab; the fiducials, the tooling edge (wider for assembly)
//! and the tooling holes. A figure of `0` is not checked. Every set of rules is marked
//! `verified = false` until someone has compared it with the maker, and a finding says
//! "the maker" only in that sense: the figures are public guiding numbers, not a promise.
//!
//! [`check`] judges [`PanelFacts`], which [`facts_from`] takes from the plan and the settings of
//! the panel. It does not move anything: the panel is laid out by the settings, and this says where
//! the layout falls short of a maker. The report is written in the eight languages and as JSON.

use std::fmt::Write as _;

use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

use crate::codes::{
    title, PanelError, FAB_FIDUCIAL, FAB_ROUTE_GAP, FAB_STAMP_HOLES, FAB_TAB_NARROW, FAB_TOOL_EDGE,
    FAB_TOOL_HOLE, FAB_VCUT_LIMITS, FAB_VCUT_SPACING, SETTINGS_INVALID,
};
use crate::layout::PanelPlan;
use crate::settings::{Frame, Joint, PanelSettings};

/// The largest length a figure may take, in nanometres.
const LENGTH_LIMIT_NM: i64 = 1 << 30;
/// The largest count a figure may take.
const COUNT_LIMIT: u32 = 1000;

/// The figures of one maker. A zero is not checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabRules {
    /// The name that `--fab-rules` gives.
    pub name: String,
    /// True when the figures were compared with the maker (every set that comes with the program
    /// says false).
    pub verified: bool,
    /// The angle of the groove of a V-cut, in degrees (said in the report, not judged).
    pub v_cut_angle_deg: u32,
    /// The least distance from copper to the V-cut.
    pub v_cut_copper_min_nm: i64,
    /// The least thickness of a board that is V-cut.
    pub v_cut_thickness_min_nm: i64,
    /// The least panel, on both sides, that is V-cut.
    pub v_cut_panel_min_nm: i64,
    /// The least distance between two parallel V-cut lines.
    pub v_cut_line_gap_min_nm: i64,
    /// The least routed gap between boards.
    pub route_gap_min_nm: i64,
    /// The least diameter of a break-away hole.
    pub stamp_hole_min_nm: i64,
    /// The largest diameter of a break-away hole.
    pub stamp_hole_max_nm: i64,
    /// The least distance between the edges of two break-away holes.
    pub stamp_spacing_min_nm: i64,
    /// The fewest break-away holes in a tab.
    pub stamp_holes_min: u32,
    /// The most break-away holes in a tab.
    pub stamp_holes_max: u32,
    /// The least tab with no holes.
    pub tab_min_nm: i64,
    /// The least tab with break-away holes.
    pub tab_min_stamped_nm: i64,
    /// The fewest fiducials on the panel.
    pub fiducial_count_min: u32,
    /// The copper diameter of a fiducial.
    pub fiducial_diameter_nm: i64,
    /// The least diameter of the solder mask opening of a fiducial.
    pub fiducial_mask_nm: i64,
    /// The least width of the tooling edge.
    pub tool_edge_nm: i64,
    /// The least width of the tooling edge for assembly.
    pub tool_edge_smt_nm: i64,
    /// The diameter of a tooling hole.
    pub tool_hole_diameter_nm: i64,
    /// The fewest tooling holes for assembly.
    pub tool_hole_count_min: u32,
}

impl FabRules {
    /// Rules that check nothing.
    pub fn empty(name: &str) -> FabRules {
        FabRules {
            name: name.to_owned(),
            verified: false,
            v_cut_angle_deg: 0,
            v_cut_copper_min_nm: 0,
            v_cut_thickness_min_nm: 0,
            v_cut_panel_min_nm: 0,
            v_cut_line_gap_min_nm: 0,
            route_gap_min_nm: 0,
            stamp_hole_min_nm: 0,
            stamp_hole_max_nm: 0,
            stamp_spacing_min_nm: 0,
            stamp_holes_min: 0,
            stamp_holes_max: 0,
            tab_min_nm: 0,
            tab_min_stamped_nm: 0,
            fiducial_count_min: 0,
            fiducial_diameter_nm: 0,
            fiducial_mask_nm: 0,
            tool_edge_nm: 0,
            tool_edge_smt_nm: 0,
            tool_hole_diameter_nm: 0,
            tool_hole_count_min: 0,
        }
    }
}

/// The file: what is the same for every set, and the sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabConfig {
    /// The panel goes to assembly (the tooling edge and the tooling holes are judged for it).
    pub smt_assembly: bool,
    /// The thickness of the board, which the board file does not give.
    pub board_thickness_nm: i64,
    /// The sets, in the order of the file.
    pub rules: Vec<FabRules>,
}

impl Default for FabConfig {
    fn default() -> Self {
        FabConfig {
            smt_assembly: false,
            board_thickness_nm: 1_600_000,
            rules: Vec::new(),
        }
    }
}

fn invalid(line: usize, what: &str) -> PanelError {
    PanelError::new(SETTINGS_INVALID, format!("fab rules line {line}: {what}"))
}

fn length(line: usize, key: &str, value: &str) -> Result<i64, PanelError> {
    let n: i64 = value
        .parse()
        .map_err(|_| invalid(line, &format!("the value of {key} is not a number")))?;
    if !(0..=LENGTH_LIMIT_NM).contains(&n) {
        return Err(invalid(
            line,
            &format!("{key} is not between 0 and {LENGTH_LIMIT_NM}"),
        ));
    }
    Ok(n)
}

fn count(line: usize, key: &str, value: &str) -> Result<u32, PanelError> {
    let n: u32 = value
        .parse()
        .map_err(|_| invalid(line, &format!("the value of {key} is not a number")))?;
    if n > COUNT_LIMIT {
        return Err(invalid(
            line,
            &format!("{key} is not between 0 and {COUNT_LIMIT}"),
        ));
    }
    Ok(n)
}

fn flag(line: usize, key: &str, value: &str) -> Result<bool, PanelError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(invalid(line, &format!("{key} is true or false"))),
    }
}

fn set_rule(r: &mut FabRules, line: usize, key: &str, v: &str) -> Result<(), PanelError> {
    match key {
        "verified" => r.verified = flag(line, key, v)?,
        "v_cut_angle_deg" => r.v_cut_angle_deg = count(line, key, v)?,
        "v_cut_copper_min_nm" => r.v_cut_copper_min_nm = length(line, key, v)?,
        "v_cut_thickness_min_nm" => r.v_cut_thickness_min_nm = length(line, key, v)?,
        "v_cut_panel_min_nm" => r.v_cut_panel_min_nm = length(line, key, v)?,
        "v_cut_line_gap_min_nm" => r.v_cut_line_gap_min_nm = length(line, key, v)?,
        "route_gap_min_nm" => r.route_gap_min_nm = length(line, key, v)?,
        "stamp_hole_min_nm" => r.stamp_hole_min_nm = length(line, key, v)?,
        "stamp_hole_max_nm" => r.stamp_hole_max_nm = length(line, key, v)?,
        "stamp_spacing_min_nm" => r.stamp_spacing_min_nm = length(line, key, v)?,
        "stamp_holes_min" => r.stamp_holes_min = count(line, key, v)?,
        "stamp_holes_max" => r.stamp_holes_max = count(line, key, v)?,
        "tab_min_nm" => r.tab_min_nm = length(line, key, v)?,
        "tab_min_stamped_nm" => r.tab_min_stamped_nm = length(line, key, v)?,
        "fiducial_count_min" => r.fiducial_count_min = count(line, key, v)?,
        "fiducial_diameter_nm" => r.fiducial_diameter_nm = length(line, key, v)?,
        "fiducial_mask_nm" => r.fiducial_mask_nm = length(line, key, v)?,
        "tool_edge_nm" => r.tool_edge_nm = length(line, key, v)?,
        "tool_edge_smt_nm" => r.tool_edge_smt_nm = length(line, key, v)?,
        "tool_hole_diameter_nm" => r.tool_hole_diameter_nm = length(line, key, v)?,
        "tool_hole_count_min" => r.tool_hole_count_min = count(line, key, v)?,
        _ => return Err(invalid(line, &format!("unknown key {key}"))),
    }
    Ok(())
}

fn check_ranges(r: &FabRules) -> Result<(), PanelError> {
    let bad =
        |what: String| PanelError::new(SETTINGS_INVALID, format!("fab rules {}: {what}", r.name));
    if r.stamp_hole_max_nm != 0 && r.stamp_hole_max_nm < r.stamp_hole_min_nm {
        return Err(bad(
            "stamp_hole_max_nm is below stamp_hole_min_nm".to_owned()
        ));
    }
    if r.stamp_holes_max != 0 && r.stamp_holes_max < r.stamp_holes_min {
        return Err(bad("stamp_holes_max is below stamp_holes_min".to_owned()));
    }
    if r.tool_edge_smt_nm != 0 && r.tool_edge_smt_nm < r.tool_edge_nm {
        return Err(bad("tool_edge_smt_nm is below tool_edge_nm".to_owned()));
    }
    Ok(())
}

impl FabConfig {
    /// Parses the text of `config/panel_fab.cfg`: lines `key = value` for the whole file, then
    /// sections `[rules NAME]`. An unknown key, a value out of range or a name used twice is
    /// refused with its line number and the name of the key.
    pub fn parse(text: &str) -> Result<FabConfig, PanelError> {
        let mut config = FabConfig::default();
        let mut current: Option<FabRules> = None;
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(head) = trimmed.strip_prefix('[') {
                let inner = head
                    .strip_suffix(']')
                    .ok_or_else(|| invalid(line, "a section is [rules NAME]"))?;
                let name = inner
                    .strip_prefix("rules ")
                    .map(str::trim)
                    .ok_or_else(|| invalid(line, "a section is [rules NAME]"))?;
                let plain = !name.is_empty()
                    && name.len() <= 32
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
                if !plain {
                    return Err(invalid(line, "a name is 1 to 32 letters, digits, - or _"));
                }
                if let Some(done) = current.take() {
                    check_ranges(&done)?;
                    config.rules.push(done);
                }
                if config.rules.iter().any(|r| r.name == name) {
                    return Err(invalid(line, &format!("the rules '{name}' appear twice")));
                }
                current = Some(FabRules::empty(name));
                continue;
            }
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| invalid(line, "not key = value"))?;
            let (key, value) = (key.trim(), value.trim());
            match current.as_mut() {
                Some(rules) => set_rule(rules, line, key, value)?,
                None => match key {
                    "smt_assembly" => config.smt_assembly = flag(line, key, value)?,
                    "board_thickness_nm" => {
                        config.board_thickness_nm = length(line, key, value)?;
                        if config.board_thickness_nm == 0 {
                            return Err(invalid(line, "board_thickness_nm is not above 0"));
                        }
                    }
                    _ => return Err(invalid(line, &format!("unknown key {key}"))),
                },
            }
        }
        if let Some(done) = current.take() {
            check_ranges(&done)?;
            config.rules.push(done);
        }
        Ok(config)
    }

    /// The rules of a name.
    pub fn find(&self, name: &str) -> Option<&FabRules> {
        self.rules.iter().find(|r| r.name == name)
    }

    /// The names of the sets, in the order of the file.
    pub fn names(&self) -> Vec<&str> {
        self.rules.iter().map(|r| r.name.as_str()).collect()
    }
}

/// What the check needs to know of a panel. Lengths are in nanometres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelFacts {
    /// The width of the panel.
    pub width_nm: i64,
    /// The height of the panel.
    pub height_nm: i64,
    /// The thickness of the board.
    pub thickness_nm: i64,
    /// True when the boards are separated by V-cuts.
    pub v_score: bool,
    /// The least distance between two parallel V-cut lines, when there are two.
    pub score_gap_nm: Option<i64>,
    /// How close the settings let a V-cut pass to copper.
    pub v_score_clearance_nm: i64,
    /// The routed gap between boards.
    pub route_gap_nm: i64,
    /// The width of a tab.
    pub tab_width_nm: i64,
    /// The diameter of a break-away hole.
    pub stamp_diameter_nm: i64,
    /// The distance between the centres of two break-away holes.
    pub stamp_pitch_nm: i64,
    /// The break-away holes in a tab.
    pub stamp_holes: u32,
    /// The width of the narrowest rail of the frame, 0 without a frame.
    pub rail_nm: i64,
    /// The fiducials of the frame.
    pub fiducials: u32,
    /// The copper diameter of a fiducial.
    pub fiducial_diameter_nm: i64,
    /// The solder mask opening of a fiducial.
    pub fiducial_mask_nm: i64,
    /// The tooling holes.
    pub tool_holes: u32,
    /// The diameter of a tooling hole.
    pub tool_hole_diameter_nm: i64,
}

fn least_gap(values: &mut Vec<i64>) -> Option<i64> {
    values.sort_unstable();
    values.dedup();
    values.windows(2).map(|w| w[1] - w[0]).min()
}

/// Takes the facts from the plan of a panel and its settings. `thickness_nm` is the thickness of
/// the board.
pub fn facts_from(plan: &PanelPlan, settings: &PanelSettings, thickness_nm: i64) -> PanelFacts {
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for line in &plan.score {
        if line.from.x == line.to.x {
            xs.push(line.from.x);
        } else if line.from.y == line.to.y {
            ys.push(line.from.y);
        }
    }
    let gaps = [least_gap(&mut xs), least_gap(&mut ys)];
    PanelFacts {
        width_nm: plan.width,
        height_nm: plan.height,
        thickness_nm,
        v_score: settings.joint == Joint::VScore,
        score_gap_nm: gaps.into_iter().flatten().min(),
        v_score_clearance_nm: settings.v_score_clearance_nm,
        route_gap_nm: settings.gap_nm,
        tab_width_nm: settings.tab_width_nm,
        stamp_diameter_nm: settings.mouse_bite_drill_nm,
        stamp_pitch_nm: settings.mouse_bite_pitch_nm,
        stamp_holes: settings.mouse_bite_holes,
        rail_nm: if settings.frame == Frame::None {
            0
        } else {
            settings.frame_width_nm
        },
        fiducials: settings.fiducial_count,
        fiducial_diameter_nm: settings.fiducial_diameter_nm,
        fiducial_mask_nm: settings.fiducial_mask_diameter_nm,
        tool_holes: settings.tooling_hole_count,
        tool_hole_diameter_nm: settings.tooling_hole_diameter_nm,
    }
}

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FabSeverity {
    /// Worth a look; the maker may still take the panel.
    Warning,
    /// The maker would refuse or change the panel.
    Error,
}

impl FabSeverity {
    /// The word of the reports.
    pub fn as_str(self) -> &'static str {
        match self {
            FabSeverity::Warning => "warning",
            FabSeverity::Error => "error",
        }
    }
}

/// One thing the layout does not meet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabFinding {
    /// The code, `PHOENIX-PANEL-<NNN>`.
    pub code: &'static str,
    /// How serious it is.
    pub severity: FabSeverity,
    /// What is specific to the case; in English.
    pub detail: String,
}

impl FabFinding {
    /// The line in a language.
    pub fn line(&self, lang: Lang) -> String {
        format!(
            "{}: {}: {}: {}",
            self.severity.as_str(),
            self.code,
            title(self.code, lang),
            self.detail
        )
    }
}

fn mm(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let n = nm.unsigned_abs();
    let (whole, frac) = (n / 1_000_000, n % 1_000_000);
    if frac == 0 {
        format!("{sign}{whole}")
    } else {
        let text = format!("{frac:06}");
        format!("{sign}{whole}.{}", text.trim_end_matches('0'))
    }
}

fn found(out: &mut Vec<FabFinding>, code: &'static str, severity: FabSeverity, detail: String) {
    out.push(FabFinding {
        code,
        severity,
        detail,
    });
}

/// Judges the facts of a panel against a set of rules; `smt` says the panel goes to assembly.
/// The findings come in a fixed order: V-cuts, routed tabs, fiducials, the tooling edge, the
/// tooling holes.
pub fn check(facts: &PanelFacts, rules: &FabRules, smt: bool) -> Vec<FabFinding> {
    use FabSeverity::{Error, Warning};
    let mut out = Vec::new();
    if facts.v_score {
        let least = facts.width_nm.min(facts.height_nm);
        if rules.v_cut_panel_min_nm > 0 && least < rules.v_cut_panel_min_nm {
            found(
                &mut out,
                FAB_VCUT_LIMITS,
                Error,
                format!(
                    "the panel is {} mm by {} mm and the least that is V-cut is {} mm on both sides",
                    mm(facts.width_nm),
                    mm(facts.height_nm),
                    mm(rules.v_cut_panel_min_nm)
                ),
            );
        }
        if rules.v_cut_thickness_min_nm > 0 && facts.thickness_nm < rules.v_cut_thickness_min_nm {
            found(
                &mut out,
                FAB_VCUT_LIMITS,
                Error,
                format!(
                    "the board is {} mm thick and the least that is V-cut is {} mm",
                    mm(facts.thickness_nm),
                    mm(rules.v_cut_thickness_min_nm)
                ),
            );
        }
        if rules.v_cut_copper_min_nm > 0 && facts.v_score_clearance_nm < rules.v_cut_copper_min_nm {
            found(
                &mut out,
                FAB_VCUT_LIMITS,
                Warning,
                format!(
                    "v_score_clearance_nm lets copper come {} mm from the cut and the maker asks {} mm",
                    mm(facts.v_score_clearance_nm),
                    mm(rules.v_cut_copper_min_nm)
                ),
            );
        }
        if let Some(gap) = facts.score_gap_nm {
            if rules.v_cut_line_gap_min_nm > 0 && gap < rules.v_cut_line_gap_min_nm {
                found(
                    &mut out,
                    FAB_VCUT_SPACING,
                    Error,
                    format!(
                        "two parallel cuts are {} mm apart and the least is {} mm",
                        mm(gap),
                        mm(rules.v_cut_line_gap_min_nm)
                    ),
                );
            }
        }
    } else {
        if rules.route_gap_min_nm > 0 && facts.route_gap_nm < rules.route_gap_min_nm {
            found(
                &mut out,
                FAB_ROUTE_GAP,
                Error,
                format!(
                    "the gap is {} mm and the least is {} mm",
                    mm(facts.route_gap_nm),
                    mm(rules.route_gap_min_nm)
                ),
            );
        }
        let holes = facts.stamp_holes;
        if holes > 0 {
            let d = facts.stamp_diameter_nm;
            if rules.stamp_hole_min_nm > 0 && d < rules.stamp_hole_min_nm {
                found(
                    &mut out,
                    FAB_STAMP_HOLES,
                    Error,
                    format!(
                        "the holes are {} mm and the least is {} mm",
                        mm(d),
                        mm(rules.stamp_hole_min_nm)
                    ),
                );
            }
            if rules.stamp_hole_max_nm > 0 && d > rules.stamp_hole_max_nm {
                found(
                    &mut out,
                    FAB_STAMP_HOLES,
                    Warning,
                    format!(
                        "the holes are {} mm and the most is {} mm",
                        mm(d),
                        mm(rules.stamp_hole_max_nm)
                    ),
                );
            }
            let spacing = facts.stamp_pitch_nm - d;
            if rules.stamp_spacing_min_nm > 0 && holes > 1 && spacing < rules.stamp_spacing_min_nm {
                found(
                    &mut out,
                    FAB_STAMP_HOLES,
                    Error,
                    format!(
                        "the edges of two holes are {} mm apart and the least is {} mm",
                        mm(spacing),
                        mm(rules.stamp_spacing_min_nm)
                    ),
                );
            }
            let below = rules.stamp_holes_min > 0 && holes < rules.stamp_holes_min;
            let above = rules.stamp_holes_max > 0 && holes > rules.stamp_holes_max;
            if below || above {
                found(
                    &mut out,
                    FAB_STAMP_HOLES,
                    Warning,
                    format!(
                        "{holes} holes in a tab and the maker asks {} to {}",
                        rules.stamp_holes_min, rules.stamp_holes_max
                    ),
                );
            }
        }
        let least_tab = if holes > 0 && rules.tab_min_stamped_nm > 0 {
            rules.tab_min_stamped_nm
        } else {
            rules.tab_min_nm
        };
        if least_tab > 0 && facts.tab_width_nm < least_tab {
            found(
                &mut out,
                FAB_TAB_NARROW,
                Error,
                format!(
                    "the tab is {} mm and the least {} is {} mm",
                    mm(facts.tab_width_nm),
                    if holes > 0 {
                        "with holes"
                    } else {
                        "without holes"
                    },
                    mm(least_tab)
                ),
            );
        }
    }
    if facts.fiducials < rules.fiducial_count_min {
        found(
            &mut out,
            FAB_FIDUCIAL,
            Error,
            format!(
                "{} fiducials on the panel and the maker asks {}",
                facts.fiducials, rules.fiducial_count_min
            ),
        );
    }
    if facts.fiducials > 0 {
        if rules.fiducial_diameter_nm > 0
            && facts.fiducial_diameter_nm != rules.fiducial_diameter_nm
        {
            found(
                &mut out,
                FAB_FIDUCIAL,
                Warning,
                format!(
                    "the copper of a fiducial is {} mm and the maker asks {} mm",
                    mm(facts.fiducial_diameter_nm),
                    mm(rules.fiducial_diameter_nm)
                ),
            );
        }
        if rules.fiducial_mask_nm > 0 && facts.fiducial_mask_nm < rules.fiducial_mask_nm {
            found(
                &mut out,
                FAB_FIDUCIAL,
                Warning,
                format!(
                    "the solder mask opening of a fiducial is {} mm and the least is {} mm",
                    mm(facts.fiducial_mask_nm),
                    mm(rules.fiducial_mask_nm)
                ),
            );
        }
    }
    let edge = if smt && rules.tool_edge_smt_nm > 0 {
        rules.tool_edge_smt_nm
    } else {
        rules.tool_edge_nm
    };
    if edge > 0 && facts.rail_nm < edge {
        found(
            &mut out,
            FAB_TOOL_EDGE,
            Error,
            format!(
                "the narrowest rail is {} mm and the least {}tooling edge is {} mm",
                mm(facts.rail_nm),
                if smt { "assembly " } else { "" },
                mm(edge)
            ),
        );
    }
    if smt && facts.tool_holes < rules.tool_hole_count_min {
        found(
            &mut out,
            FAB_TOOL_HOLE,
            Error,
            format!(
                "{} tooling holes and assembly asks {}",
                facts.tool_holes, rules.tool_hole_count_min
            ),
        );
    }
    if facts.tool_holes > 0
        && rules.tool_hole_diameter_nm > 0
        && facts.tool_hole_diameter_nm != rules.tool_hole_diameter_nm
    {
        found(
            &mut out,
            FAB_TOOL_HOLE,
            Warning,
            format!(
                "a tooling hole is {} mm and the maker asks {} mm",
                mm(facts.tool_hole_diameter_nm),
                mm(rules.tool_hole_diameter_nm)
            ),
        );
    }
    out
}

/// The words of the report in the eight languages (English, Spanish, German, French, Italian,
/// Japanese, Chinese and European Portuguese).
const WORDS: [[&str; 8]; 3] = [
    [
        "Panel rules",
        "Reglas de panel",
        "Nutzenregeln",
        "Règles de panneau",
        "Regole di pannello",
        "パネル規則",
        "拼板规则",
        "Regras de painel",
    ],
    [
        "the figures are not verified with the maker",
        "las cifras no están verificadas con el fabricante",
        "die Werte sind beim Hersteller nicht überprüft",
        "les valeurs ne sont pas vérifiées auprès du fabricant",
        "i valori non sono verificati con il produttore",
        "数値はメーカーに確認されていません",
        "这些数值未与制造商核实",
        "os valores não foram verificados com o fabricante",
    ],
    [
        "the panel meets these rules",
        "el panel cumple estas reglas",
        "der Nutzen erfüllt diese Regeln",
        "le panneau respecte ces règles",
        "il pannello rispetta queste regole",
        "パネルはこの規則を満たしています",
        "拼板符合这些规则",
        "o painel cumpre estas regras",
    ],
];

fn word(index: usize, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    WORDS[index][column]
}

/// The report as text in a language.
pub fn report_text(rules: &FabRules, findings: &[FabFinding], lang: Lang) -> String {
    let mut out = format!("{}: {}", word(0, lang), rules.name);
    if !rules.verified {
        let _ = write!(out, " ({})", word(1, lang));
    }
    out.push('\n');
    if findings.is_empty() {
        let _ = writeln!(out, "{}", word(2, lang));
    }
    for finding in findings {
        let _ = writeln!(out, "{}", finding.line(lang));
    }
    out
}

/// The report as JSON.
pub fn report_json(rules: &FabRules, findings: &[FabFinding]) -> Value {
    let list: Vec<Value> = findings
        .iter()
        .map(|f| {
            json!({
                "code": f.code,
                "severity": f.severity.as_str(),
                "detail": f.detail,
            })
        })
        .collect();
    json!({
        "rules": rules.name,
        "verified": rules.verified,
        "errors": findings.iter().filter(|f| f.severity == FabSeverity::Error).count(),
        "warnings": findings.iter().filter(|f| f.severity == FabSeverity::Warning).count(),
        "findings": list,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Line, Pt};

    fn config() -> FabConfig {
        FabConfig::parse(include_str!("../../../config/panel_fab.cfg")).unwrap()
    }

    fn facts() -> PanelFacts {
        PanelFacts {
            width_nm: 100_000_000,
            height_nm: 80_000_000,
            thickness_nm: 1_600_000,
            v_score: false,
            score_gap_nm: None,
            v_score_clearance_nm: 500_000,
            route_gap_nm: 2_000_000,
            tab_width_nm: 5_000_000,
            stamp_diameter_nm: 500_000,
            stamp_pitch_nm: 900_000,
            stamp_holes: 5,
            rail_nm: 8_000_000,
            fiducials: 3,
            fiducial_diameter_nm: 1_000_000,
            fiducial_mask_nm: 1_700_000,
            tool_holes: 4,
            tool_hole_diameter_nm: 2_000_000,
        }
    }

    fn codes_of(findings: &[FabFinding]) -> Vec<&'static str> {
        findings.iter().map(|f| f.code).collect()
    }

    #[test]
    fn the_file_in_the_config_folder_holds_the_sets_with_the_figures_of_the_notes() {
        let c = config();
        assert_eq!(c.names(), ["common", "jlcpcb", "pcbway"]);
        assert!(!c.smt_assembly);
        assert_eq!(c.board_thickness_nm, 1_600_000);
        for r in &c.rules {
            assert!(!r.verified, "{}", r.name);
        }
        let common = c.find("common").unwrap();
        assert_eq!(common.v_cut_angle_deg, 25);
        assert_eq!(common.v_cut_copper_min_nm, 400_000);
        assert_eq!(common.v_cut_thickness_min_nm, 600_000);
        assert_eq!(common.v_cut_panel_min_nm, 60_000_000);
        assert_eq!(common.route_gap_min_nm, 1_600_000);
        assert_eq!(common.stamp_hole_min_nm, 450_000);
        assert_eq!((common.stamp_holes_min, common.stamp_holes_max), (5, 6));
        assert_eq!(common.stamp_spacing_min_nm, 350_000);
        assert_eq!(common.fiducial_diameter_nm, 1_000_000);
        assert_eq!(common.fiducial_mask_nm, 1_700_000);
        assert_eq!(common.tool_edge_nm, 3_000_000);
        assert_eq!(common.tool_edge_smt_nm, 5_000_000);
        assert_eq!(common.tool_hole_diameter_nm, 2_000_000);
    }

    #[test]
    fn a_panel_that_meets_the_rules_has_no_finding_in_every_set() {
        let c = config();
        for r in &c.rules {
            let mut f = facts();
            f.tab_width_nm = 6_000_000;
            f.stamp_pitch_nm = 1_000_000;
            assert!(check(&f, r, false).is_empty(), "{}", r.name);
        }
    }

    #[test]
    fn the_routed_tab_is_judged_in_the_order_of_the_rules() {
        let c = config();
        let r = c.find("common").unwrap();
        let mut f = facts();
        f.route_gap_nm = 1_000_000;
        f.stamp_diameter_nm = 400_000;
        f.stamp_pitch_nm = 600_000;
        f.stamp_holes = 3;
        f.tab_width_nm = 1_000_000;
        let found = check(&f, r, false);
        assert_eq!(
            codes_of(&found),
            [
                FAB_ROUTE_GAP,
                FAB_STAMP_HOLES,
                FAB_STAMP_HOLES,
                FAB_STAMP_HOLES,
                FAB_TAB_NARROW
            ]
        );
        assert!(found[0].detail.contains("1 mm") && found[0].detail.contains("1.6 mm"));
        // the spacing is the edge to edge distance: pitch minus the diameter
        assert!(found[2].detail.contains("0.2 mm"), "{}", found[2].detail);
        assert_eq!(found[3].severity, FabSeverity::Warning);
    }

    #[test]
    fn a_tab_without_holes_uses_the_other_least_tab() {
        let c = config();
        let r = c.find("pcbway").unwrap();
        let mut f = facts();
        f.stamp_holes = 0;
        f.tab_width_nm = 1_500_000;
        let found = check(&f, r, false);
        assert_eq!(codes_of(&found), [FAB_TAB_NARROW]);
        assert!(found[0].detail.contains("without holes"));
        f.tab_width_nm = 2_000_000;
        assert!(check(&f, r, false).is_empty());
    }

    #[test]
    fn the_vcut_is_judged_by_the_size_the_thickness_the_copper_and_the_distance_between_cuts() {
        let c = config();
        let r = c.find("common").unwrap();
        let mut f = facts();
        f.v_score = true;
        f.width_nm = 50_000_000;
        f.thickness_nm = 400_000;
        f.v_score_clearance_nm = 200_000;
        let found = check(&f, r, false);
        assert_eq!(
            codes_of(&found),
            [FAB_VCUT_LIMITS, FAB_VCUT_LIMITS, FAB_VCUT_LIMITS]
        );
        assert_eq!(found[2].severity, FabSeverity::Warning);
        let j = c.find("jlcpcb").unwrap();
        let mut g = facts();
        g.v_score = true;
        g.score_gap_nm = Some(1_000_000);
        let found = check(&g, j, false);
        assert_eq!(codes_of(&found), [FAB_VCUT_SPACING]);
        g.score_gap_nm = Some(2_000_000);
        assert!(check(&g, j, false).is_empty());
        // a routed gap is not judged on a V-cut panel
        g.route_gap_nm = 0;
        assert!(check(&g, j, false).is_empty());
    }

    #[test]
    fn the_fiducials_the_tooling_edge_and_the_tooling_holes_follow_assembly() {
        let c = config();
        let r = c.find("common").unwrap();
        let mut f = facts();
        f.rail_nm = 4_000_000;
        assert!(check(&f, r, false).is_empty());
        let found = check(&f, r, true);
        assert_eq!(codes_of(&found), [FAB_TOOL_EDGE]);
        assert!(found[0].detail.contains("assembly tooling edge is 5 mm"));
        f.rail_nm = 0;
        assert_eq!(codes_of(&check(&f, r, false)), [FAB_TOOL_EDGE]);
        f.rail_nm = 8_000_000;
        f.fiducials = 2;
        f.fiducial_diameter_nm = 1_500_000;
        f.fiducial_mask_nm = 1_200_000;
        f.tool_hole_diameter_nm = 3_000_000;
        let found = check(&f, r, false);
        assert_eq!(
            codes_of(&found),
            [FAB_FIDUCIAL, FAB_FIDUCIAL, FAB_FIDUCIAL, FAB_TOOL_HOLE]
        );
        f.tool_holes = 1;
        f.tool_hole_diameter_nm = 2_000_000;
        let found = check(&f, c.find("pcbway").unwrap(), true);
        assert!(!codes_of(&found).contains(&FAB_TOOL_HOLE));
        let none = FabRules::empty("none");
        assert!(check(&f, &none, true).is_empty());
    }

    #[test]
    fn the_facts_come_from_the_plan_and_the_settings() {
        let plan = PanelPlan {
            width: 120_000_000,
            height: 90_000_000,
            score: vec![
                Line {
                    from: Pt {
                        x: 10_000_000,
                        y: 0,
                    },
                    to: Pt {
                        x: 10_000_000,
                        y: 90_000_000,
                    },
                },
                Line {
                    from: Pt {
                        x: 60_000_000,
                        y: 0,
                    },
                    to: Pt {
                        x: 60_000_000,
                        y: 90_000_000,
                    },
                },
                Line {
                    from: Pt {
                        x: 0,
                        y: 41_000_000,
                    },
                    to: Pt {
                        x: 120_000_000,
                        y: 41_000_000,
                    },
                },
                Line {
                    from: Pt {
                        x: 0,
                        y: 45_000_000,
                    },
                    to: Pt {
                        x: 120_000_000,
                        y: 45_000_000,
                    },
                },
            ],
            columns: 3,
            rows: 2,
            origins: Vec::new(),
            profile: Vec::new(),
            tabs: Vec::new(),
            holes: Vec::new(),
            fiducials: Vec::new(),
            local_fiducials: Vec::new(),
            mixed: false,
            copies: Vec::new(),
            notes: Vec::new(),
        };
        let settings = PanelSettings {
            joint: Joint::VScore,
            frame: Frame::None,
            ..PanelSettings::default()
        };
        let f = facts_from(&plan, &settings, 1_000_000);
        assert_eq!(
            (f.width_nm, f.height_nm, f.thickness_nm),
            (120_000_000, 90_000_000, 1_000_000)
        );
        assert!(f.v_score);
        assert_eq!(f.score_gap_nm, Some(4_000_000));
        assert_eq!(f.rail_nm, 0);
        assert_eq!(f.fiducials, settings.fiducial_count);
        assert_eq!(f.stamp_pitch_nm, settings.mouse_bite_pitch_nm);
    }

    #[test]
    fn the_report_is_in_the_eight_languages_and_says_what_is_not_verified() {
        let c = config();
        let r = c.find("jlcpcb").unwrap();
        let mut f = facts();
        f.route_gap_nm = 100_000;
        let found = check(&f, r, false);
        let english = report_text(r, &found, Lang::En);
        assert!(english
            .starts_with("Panel rules: jlcpcb (the figures are not verified with the maker)\n"));
        assert!(english.contains("error: PHOENIX-PANEL-024: The routed gap is narrower than the maker allows: the gap is 0.1 mm"));
        let mut seen = Vec::new();
        for lang in Lang::ALL {
            let text = report_text(r, &found, lang);
            assert!(text.contains("PHOENIX-PANEL-024"), "{lang:?}");
            assert!(!seen.contains(&text), "{lang:?} repeats another language");
            seen.push(text);
        }
        let clean = report_text(r, &[], Lang::Pt);
        assert!(clean.contains("o painel cumpre estas regras"), "{clean}");
        let json = report_json(r, &found);
        assert_eq!(json["errors"], 1);
        assert_eq!(json["findings"][0]["code"], FAB_ROUTE_GAP);
        assert_eq!(json["verified"], false);
    }

    #[test]
    fn a_bad_file_is_refused_with_the_line_and_the_key() {
        for (text, needle) in [
            ("tool_edge_nm = 3\n", "line 1: unknown key tool_edge_nm"),
            (
                "[rules a]\nv_cut_angle_dg = 25\n",
                "line 2: unknown key v_cut_angle_dg",
            ),
            ("[rules a]\ntab_min_nm = -1\n", "tab_min_nm is not between"),
            (
                "[rules a]\ntab_min_nm = x\n",
                "the value of tab_min_nm is not a number",
            ),
            ("[rules a]\nverified = yes\n", "verified is true or false"),
            ("[rules a]\n[rules a]\n", "appear twice"),
            ("[rules ]\n", "1 to 32 letters"),
            ("[rules a b]\n", "1 to 32 letters"),
            ("[other a]\n", "[rules NAME]"),
            ("board_thickness_nm = 0\n", "not above 0"),
            ("smt_assembly = maybe\n", "smt_assembly is true or false"),
            (
                "[rules a]\nstamp_hole_min_nm = 5\nstamp_hole_max_nm = 4\n",
                "stamp_hole_max_nm is below",
            ),
            (
                "[rules a]\nstamp_holes_min = 5\nstamp_holes_max = 4\n",
                "stamp_holes_max is below",
            ),
            (
                "[rules a]\ntool_edge_nm = 5\ntool_edge_smt_nm = 4\n",
                "tool_edge_smt_nm is below",
            ),
            (
                "[rules a]\nstamp_holes_min = 5000\n",
                "stamp_holes_min is not between",
            ),
            ("no equals sign\n", "not key = value"),
        ] {
            let e = FabConfig::parse(text).unwrap_err();
            assert_eq!(e.code, SETTINGS_INVALID);
            assert!(e.detail.contains(needle), "{text:?}: {}", e.detail);
        }
        assert_eq!(
            FabConfig::parse("# only a comment\n\n").unwrap(),
            FabConfig::default()
        );
    }

    #[test]
    fn a_range_of_random_panels_never_panics_and_errors_follow_the_rules() {
        let c = config();
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..500 {
            let mut f = facts();
            f.v_score = next() % 2 == 0;
            f.width_nm = (next() % 300_000_000) as i64;
            f.height_nm = (next() % 300_000_000) as i64;
            f.thickness_nm = (next() % 4_000_000) as i64;
            f.route_gap_nm = (next() % 5_000_000) as i64;
            f.tab_width_nm = (next() % 8_000_000) as i64;
            f.stamp_diameter_nm = (next() % 1_500_000) as i64;
            f.stamp_pitch_nm = (next() % 2_000_000) as i64;
            f.stamp_holes = (next() % 9) as u32;
            f.rail_nm = (next() % 12_000_000) as i64;
            f.score_gap_nm = if next() % 3 == 0 {
                None
            } else {
                Some((next() % 6_000_000) as i64)
            };
            let smt = next() % 2 == 0;
            for r in &c.rules {
                let found = check(&f, r, smt);
                // the same facts always give the same findings
                assert_eq!(found, check(&f, r, smt));
                // a V-cut panel gets no finding about routed tabs and the other way round
                if f.v_score {
                    assert!(found.iter().all(|x| ![
                        FAB_ROUTE_GAP,
                        FAB_STAMP_HOLES,
                        FAB_TAB_NARROW
                    ]
                    .contains(&x.code)));
                } else {
                    assert!(found
                        .iter()
                        .all(|x| ![FAB_VCUT_LIMITS, FAB_VCUT_SPACING].contains(&x.code)));
                }
            }
        }
    }
}
