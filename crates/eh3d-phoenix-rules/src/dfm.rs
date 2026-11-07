// SPDX-License-Identifier: GPL-3.0-or-later
//! Checks of manufacturability against a capability profile of a board maker, read from a plain
//! text file (`config/dfm/example.cfg` shows the form).
//!
//! A profile holds two kinds of lines. The first kind are the limits of the rule engine (the
//! narrowest track, the clearance, the smallest drill, the annular ring, the distance to the
//! edge, the solder mask bridge, the drill spacing), which become the [`RuleValues`] of the usual
//! check. The second kind are what a maker can build at all: the most copper layers, the smallest
//! and the largest board (the short and the long side), the largest drill and the most holes.
//! A limit of zero turns that line off.
//!
//! The program carries an example of the form with invented round numbers and the public figures
//! of two makers (see [`BUILTIN`]), all marked `verified = false` because nothing compares them
//! with the maker; every check against a profile that is not verified says so with
//! `PHOENIX-DFM-006`. Put the figures of the maker you use in a copy of the file, with where you
//! read them in `source`, and mark it verified yourself.

use eh3d_phoenix_board::Board;
use eh3d_phoenix_model::Nm;

use crate::codes::{Lang, RULEFILE_VALUE};
use crate::report::{Finding, Report, Severity};
use crate::rulefile::RuleValues;

/// More copper layers than the profile offers.
pub const LAYERS: &str = "PHOENIX-DFM-001";
/// A board smaller than the profile accepts.
pub const BOARD_SMALL: &str = "PHOENIX-DFM-002";
/// A board larger than the profile accepts.
pub const BOARD_LARGE: &str = "PHOENIX-DFM-003";
/// A drill larger than the profile accepts.
pub const DRILL_LARGE: &str = "PHOENIX-DFM-004";
/// More holes than the profile accepts.
pub const TOO_MANY_HOLES: &str = "PHOENIX-DFM-005";
/// The profile is not verified against the maker.
pub const UNVERIFIED: &str = "PHOENIX-DFM-006";

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (LAYERS, "More copper layers than the profile offers"),
    (BOARD_SMALL, "Board smaller than the profile accepts"),
    (BOARD_LARGE, "Board larger than the profile accepts"),
    (DRILL_LARGE, "Drill larger than the profile accepts"),
    (TOO_MANY_HOLES, "More holes than the profile accepts"),
    (
        UNVERIFIED,
        "The capability profile is not verified against the maker",
    ),
];

const TITLES: &[(&str, [&str; 8])] = &[
    (
        LAYERS,
        [
            "More copper layers than the profile offers",
            "Más capas de cobre de las que ofrece el perfil",
            "Mehr Kupferlagen, als das Profil bietet",
            "Plus de couches de cuivre que le profil n'en offre",
            "Più strati di rame di quelli offerti dal profilo",
            "プロファイルが対応する銅層数を超えています",
            "铜层数超过配置文件所支持的数量",
            "Mais camadas de cobre do que o perfil oferece",
        ],
    ),
    (
        BOARD_SMALL,
        [
            "Board smaller than the profile accepts",
            "Placa más pequeña de lo que acepta el perfil",
            "Platine kleiner, als das Profil zulässt",
            "Carte plus petite que ce que le profil accepte",
            "Scheda più piccola di quanto accetti il profilo",
            "基板がプロファイルの許容より小さい",
            "电路板小于配置文件允许的尺寸",
            "Placa mais pequena do que o perfil aceita",
        ],
    ),
    (
        BOARD_LARGE,
        [
            "Board larger than the profile accepts",
            "Placa más grande de lo que acepta el perfil",
            "Platine größer, als das Profil zulässt",
            "Carte plus grande que ce que le profil accepte",
            "Scheda più grande di quanto accetti il profilo",
            "基板がプロファイルの許容より大きい",
            "电路板大于配置文件允许的尺寸",
            "Placa maior do que o perfil aceita",
        ],
    ),
    (
        DRILL_LARGE,
        [
            "Drill larger than the profile accepts",
            "Taladro mayor de lo que acepta el perfil",
            "Bohrung größer, als das Profil zulässt",
            "Perçage plus grand que ce que le profil accepte",
            "Foro più grande di quanto accetti il profilo",
            "ドリル径がプロファイルの許容より大きい",
            "钻孔大于配置文件允许的尺寸",
            "Furo maior do que o perfil aceita",
        ],
    ),
    (
        TOO_MANY_HOLES,
        [
            "More holes than the profile accepts",
            "Más agujeros de los que acepta el perfil",
            "Mehr Bohrungen, als das Profil zulässt",
            "Plus de perçages que le profil n'en accepte",
            "Più fori di quelli accettati dal profilo",
            "穴の数がプロファイルの許容を超えています",
            "孔数超过配置文件允许的数量",
            "Mais furos do que o perfil aceita",
        ],
    ),
    (
        UNVERIFIED,
        [
            "The capability profile is not verified against the maker",
            "El perfil de capacidades no está verificado con el fabricante",
            "Das Fähigkeitsprofil ist nicht beim Hersteller geprüft",
            "Le profil de capacités n'est pas vérifié auprès du fabricant",
            "Il profilo delle capacità non è verificato con il produttore",
            "能力プロファイルはメーカーに確認されていません",
            "能力配置文件尚未与制造商核实",
            "O perfil de capacidades não foi verificado junto do fabricante",
        ],
    ),
];

/// The title of a code of the capability checks in a language; `None` for any other code.
pub fn title(code: &str, lang: Lang) -> Option<&'static str> {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, texts)| texts[lang.column()])
}

/// A profile that cannot be read: the line of the file and what is wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DfmError {
    /// The line of the file, counted from 1.
    pub line: usize,
    /// What is wrong, in English.
    pub detail: String,
}

impl DfmError {
    /// The stable code of the error.
    pub fn code(&self) -> &'static str {
        RULEFILE_VALUE
    }
}

impl std::fmt::Display for DfmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.detail)
    }
}

impl std::error::Error for DfmError {}

/// What a board maker can build, and the limits of the usual rule check that follow from it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DfmProfile {
    /// The name of the profile, for the report.
    pub name: String,
    /// Where the figures were read, in the words of the person who wrote the profile.
    pub source: String,
    /// Whether the figures were checked against the maker. False unless the file says so.
    pub verified: bool,
    /// The limits of the rule engine.
    pub rules: RuleValues,
    /// The most copper layers; 0 turns the line off.
    pub max_copper_layers: u32,
    /// The least short side of the board, in nanometres.
    pub min_board_short_nm: i64,
    /// The least long side of the board.
    pub min_board_long_nm: i64,
    /// The greatest short side of the board.
    pub max_board_short_nm: i64,
    /// The greatest long side of the board.
    pub max_board_long_nm: i64,
    /// The largest drill.
    pub max_drill_nm: i64,
    /// The most holes (vias and drilled pads).
    pub max_holes: u64,
}

fn length(value: &str) -> Option<Nm> {
    value
        .parse::<i64>()
        .ok()
        .filter(|v| (0..=1_000_000_000_000).contains(v))
        .and_then(|v| Nm::new(v).ok())
}

fn flag(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

impl DfmProfile {
    /// Parses a profile: `key = value` lines, `#` starts a comment, a key written twice, a key
    /// that is not known or a value out of range is refused with its line.
    pub fn parse(text: &str) -> Result<DfmProfile, DfmError> {
        let mut p = DfmProfile::default();
        let mut seen = std::collections::BTreeSet::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let fail = |detail: String| DfmError { line, detail };
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| fail("not key = value".to_owned()))?;
            let (key, value) = (key.trim(), value.trim());
            if !seen.insert(key.to_owned()) {
                return Err(fail(format!("{key} is written twice")));
            }
            let bad = || fail(format!("the value of {key} is not valid"));
            let len = || length(value).ok_or_else(bad);
            let big = || {
                value
                    .parse::<i64>()
                    .ok()
                    .filter(|v| *v >= 0)
                    .ok_or_else(bad)
            };
            match key {
                "name" => p.name = value.to_owned(),
                "source" => p.source = value.to_owned(),
                "verified" => p.verified = flag(value).ok_or_else(bad)?,
                "min_track_width_nm" => p.rules.min_track_width = len()?,
                "min_clearance_nm" => p.rules.min_clearance = len()?,
                "min_drill_nm" => p.rules.min_drill = len()?,
                "min_via_diameter_nm" => p.rules.min_via_diameter = len()?,
                "min_annular_ring_nm" => p.rules.min_annular_ring = len()?,
                "min_edge_clearance_nm" => p.rules.min_edge_clearance = len()?,
                "min_mask_bridge_nm" => p.rules.min_mask_bridge = len()?,
                "min_drill_spacing_nm" => p.rules.min_drill_spacing = len()?,
                "connect_tolerance_nm" => p.rules.connect_tolerance = len()?,
                "silkscreen_over_pads" => {
                    p.rules.silkscreen_over_pads = flag(value).ok_or_else(bad)?
                }
                "silkscreen_over_mask_openings" => {
                    p.rules.silkscreen_over_mask_openings = flag(value).ok_or_else(bad)?
                }
                "max_copper_layers" => {
                    p.max_copper_layers = value
                        .parse::<u32>()
                        .ok()
                        .filter(|v| *v <= 256)
                        .ok_or_else(bad)?
                }
                "min_board_short_nm" => p.min_board_short_nm = big()?,
                "min_board_long_nm" => p.min_board_long_nm = big()?,
                "max_board_short_nm" => p.max_board_short_nm = big()?,
                "max_board_long_nm" => p.max_board_long_nm = big()?,
                "max_drill_nm" => p.max_drill_nm = big()?,
                "max_holes" => p.max_holes = big()?.unsigned_abs(),
                _ => return Err(fail(format!("unknown key {key}"))),
            }
        }
        Ok(p)
    }
}

/// The profiles that come with the program: their names and the text of their files in `config/`.
/// The figures are in those files and nowhere else; the program only reads them.
pub const BUILTIN: &[(&str, &str)] = &[
    ("example", include_str!("../../../config/dfm/example.cfg")),
    ("jlcpcb", include_str!("../../../config/dfm_jlcpcb.cfg")),
    ("pcbway", include_str!("../../../config/dfm_pcbway.cfg")),
];

/// The text of a profile that comes with the program, by name (without regard to case).
pub fn builtin_profile(name: &str) -> Option<&'static str> {
    BUILTIN
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, text)| *text)
}

/// The names of the profiles that come with the program, joined for a message.
pub fn builtin_names() -> String {
    BUILTIN
        .iter()
        .map(|(n, _)| *n)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The box of the outline as its short and its long side, in nanometres.
fn sides(board: &Board) -> Option<(i64, i64)> {
    let xs = board.outline.iter().map(|p| p.x_nm.get());
    let ys = board.outline.iter().map(|p| p.y_nm.get());
    let width = xs.clone().max()? - xs.min()?;
    let height = ys.clone().max()? - ys.min()?;
    Some((width.min(height), width.max(height)))
}

/// The number of holes and the largest drill, in nanometres: the vias and the drilled pads of the
/// placed components.
fn holes(board: &Board) -> (u64, i64) {
    let mut count = 0u64;
    let mut largest = 0i64;
    for via in board.vias.values() {
        count += 1;
        largest = largest.max(via.drill.get());
    }
    for component in board.components.values() {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        for drill in footprint.pads.values().filter_map(|pad| pad.drill) {
            count += 1;
            largest = largest.max(drill.get());
        }
    }
    (count, largest)
}

/// The findings of the lines of a profile that the rule engine does not judge: layers, size of the
/// board, the largest drill and the number of holes, and the note that the profile is not
/// verified. The limits of the rule engine are checked with `check_board` and the values in
/// `profile.rules`.
pub fn dfm_findings(board: &Board, profile: &DfmProfile) -> Report {
    let mut report = Report::new("check");
    let name = if profile.name.is_empty() {
        "(unnamed)"
    } else {
        profile.name.as_str()
    };
    let mut add = |finding: Finding| report.confirmed.push(finding);
    let layers = board.copper_stack.len() as u32;
    if profile.max_copper_layers > 0 && layers > profile.max_copper_layers {
        add(Finding::new(
            40,
            "dfm_layers",
            LAYERS,
            Severity::Error,
            "board".to_owned(),
        )
        .why(&format!(
            "{layers} copper layers, the profile {name} offers {}",
            profile.max_copper_layers
        )));
    }
    if let Some((short, long)) = sides(board) {
        for (side, value, least, most) in [
            (
                "short side",
                short,
                profile.min_board_short_nm,
                profile.max_board_short_nm,
            ),
            (
                "long side",
                long,
                profile.min_board_long_nm,
                profile.max_board_long_nm,
            ),
        ] {
            if least > 0 && value < least {
                add(Finding::new(
                    41,
                    "dfm_board_small",
                    BOARD_SMALL,
                    Severity::Error,
                    "board".to_owned(),
                )
                .values(value, least)
                .why(side));
            }
            if most > 0 && value > most {
                add(Finding::new(
                    42,
                    "dfm_board_large",
                    BOARD_LARGE,
                    Severity::Error,
                    "board".to_owned(),
                )
                .values(value, most)
                .why(side));
            }
        }
    }
    let (count, largest) = holes(board);
    if profile.max_drill_nm > 0 && largest > profile.max_drill_nm {
        add(Finding::new(
            43,
            "dfm_drill_large",
            DRILL_LARGE,
            Severity::Error,
            "holes".to_owned(),
        )
        .values(largest, profile.max_drill_nm)
        .why("the largest drill of the board"));
    }
    if profile.max_holes > 0 && count > profile.max_holes {
        add(Finding::new(
            44,
            "dfm_holes",
            TOO_MANY_HOLES,
            Severity::Error,
            "holes".to_owned(),
        )
        .why(&format!(
            "{count} holes, the profile {name} accepts {}",
            profile.max_holes
        )));
    }
    if !profile.verified {
        let mut finding = Finding::new(
            45,
            "dfm_unverified",
            UNVERIFIED,
            Severity::Info,
            name.to_owned(),
        );
        finding = finding.why(if profile.source.is_empty() {
            "no source is given"
        } else {
            profile.source.as_str()
        });
        report.confirmed.push(finding);
    }
    report.finish();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_board::{
        Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadShape, Side, Via,
    };
    use eh3d_phoenix_model::{ObjectId, Point, Udeg};

    fn id(n: u32) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x, y).unwrap()
    }

    /// A board of 40 mm by 25 mm with two copper layers, two vias with drills of 0.3 mm and
    /// 0.8 mm and one part with two drilled pads of 1.0 mm.
    fn board() -> Board {
        let mut b = Board::default();
        for (n, name) in [(1, "Top"), (2, "Bottom")] {
            b.layers.insert(
                id(n),
                Layer {
                    name: name.into(),
                    kind: LayerKind::Copper,
                    side: None,
                },
            );
        }
        b.copper_stack = vec![id(1), id(2)];
        b.outline = vec![
            pt(0, 0),
            pt(40_000_000, 0),
            pt(40_000_000, 25_000_000),
            pt(0, 25_000_000),
        ];
        b.nets.insert(id(10), Net { name: "A".into() });
        for (n, x, drill) in [(30, 5_000_000, 300_000), (31, 9_000_000, 800_000)] {
            b.vias.insert(
                id(n),
                Via {
                    net: id(10),
                    at: pt(x, 5_000_000),
                    drill: Nm::new(drill).unwrap(),
                    diameter: Nm::new(drill + 300_000).unwrap(),
                    from_layer: id(1),
                    to_layer: id(2),
                },
            );
        }
        let mut footprint = Footprint {
            name: "HDR2".into(),
            pads: Default::default(),
        };
        for (n, x) in [(41, 0), (42, 2_540_000)] {
            footprint.pads.insert(
                id(n),
                Pad {
                    number: "1".into(),
                    shape: PadShape::Circle {
                        diameter: Nm::new(1_800_000).unwrap(),
                    },
                    at: pt(x, 0),
                    rotation: Udeg::new(0).unwrap(),
                    drill: Some(Nm::new(1_000_000).unwrap()),
                    layers: vec![id(1), id(2)],
                    roundness: 0,
                    plated: true,
                },
            );
        }
        b.footprints.insert(id(40), footprint);
        b.components.insert(
            id(50),
            Component {
                refdes: "J1".into(),
                value: "HDR".into(),
                symbol: None,
                footprint: id(40),
                at: pt(20_000_000, 12_000_000),
                rotation: Udeg::new(0).unwrap(),
                side: Side::Top,
            },
        );
        b
    }

    fn codes(report: &Report) -> Vec<&'static str> {
        report.confirmed.iter().map(|f| f.code).collect()
    }

    #[test]
    fn a_profile_that_is_off_asks_nothing_but_says_it_is_not_verified() {
        let report = dfm_findings(&board(), &DfmProfile::default());
        assert_eq!(codes(&report), [UNVERIFIED]);
        assert!(report.passes());
        let verified = DfmProfile {
            verified: true,
            ..DfmProfile::default()
        };
        assert!(dfm_findings(&board(), &verified).confirmed.is_empty());
    }

    #[test]
    fn the_layers_the_size_the_largest_drill_and_the_holes_are_judged_by_hand_figures() {
        let profile = DfmProfile::parse(
            "verified = true\nmax_copper_layers = 1\nmin_board_short_nm = 30000000\n\
             max_board_long_nm = 35000000\nmax_drill_nm = 900000\nmax_holes = 3\n",
        )
        .unwrap();
        let report = dfm_findings(&board(), &profile);
        // 2 layers > 1; short side 25 mm < 30 mm; long side 40 mm > 35 mm; the largest drill is
        // 1.0 mm (the pads) > 0.9 mm; 2 vias + 2 pads = 4 holes > 3
        assert_eq!(
            codes(&report),
            [
                LAYERS,
                BOARD_SMALL,
                BOARD_LARGE,
                DRILL_LARGE,
                TOO_MANY_HOLES
            ]
        );
        assert!(!report.passes());
        let small = &report.confirmed[1];
        assert_eq!(
            (small.measured_nm, small.limit_nm),
            (Some(25_000_000), Some(30_000_000))
        );
        let large = &report.confirmed[2];
        assert_eq!(
            (large.measured_nm, large.limit_nm),
            (Some(40_000_000), Some(35_000_000))
        );
        let drill = &report.confirmed[3];
        assert_eq!(
            (drill.measured_nm, drill.limit_nm),
            (Some(1_000_000), Some(900_000))
        );
        assert!(report.confirmed[4].detail.starts_with("4 holes"));
    }

    #[test]
    fn a_board_that_fits_the_profile_gives_nothing() {
        let profile = DfmProfile::parse(
            "verified = true\nmax_copper_layers = 2\nmin_board_short_nm = 25000000\n\
             max_board_short_nm = 25000000\nmax_board_long_nm = 40000000\n\
             max_drill_nm = 1000000\nmax_holes = 4\n",
        )
        .unwrap();
        assert!(dfm_findings(&board(), &profile).confirmed.is_empty());
    }

    #[test]
    fn the_limits_of_the_rule_engine_come_from_the_same_file() {
        let profile = DfmProfile::parse("verified = true\nmin_drill_nm = 400000\n").unwrap();
        assert_eq!(profile.rules.min_drill.get(), 400_000);
        // the via with the drill of 0.3 mm is below it: one finding of the usual check
        let report = crate::check_board(&board(), &profile.rules);
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        assert_eq!(report.confirmed[0].code, crate::codes::DRILL_SMALL);
    }

    #[test]
    fn what_is_wrong_in_a_profile_is_refused_with_its_line() {
        for (text, line, needle) in [
            ("oops\n", 1, "not key = value"),
            ("\nmax_holes = -1\n", 2, "max_holes"),
            ("verified = yes\n", 1, "verified"),
            ("colour = red\n", 1, "unknown key"),
            ("min_drill_nm = 1\nmin_drill_nm = 2\n", 2, "written twice"),
            ("max_copper_layers = 300\n", 1, "max_copper_layers"),
            ("min_clearance_nm = -5\n", 1, "min_clearance_nm"),
        ] {
            let e = DfmProfile::parse(text).unwrap_err();
            assert_eq!(e.line, line, "{text}");
            assert!(e.detail.contains(needle), "{text}: {}", e.detail);
            assert_eq!(e.code(), RULEFILE_VALUE);
        }
        let shipped = include_str!("../../../config/dfm/example.cfg");
        let profile = DfmProfile::parse(shipped).unwrap();
        assert!(!profile.verified);
        assert!(profile.max_copper_layers > 0);
    }

    #[test]
    fn every_code_has_a_title_in_all_eight_languages() {
        assert_eq!(REGISTRY.len(), TITLES.len());
        for (code, english) in REGISTRY {
            for lang in Lang::ALL {
                assert!(!title(code, lang).unwrap_or_default().is_empty(), "{code}");
            }
            assert_eq!(title(code, Lang::En), Some(*english));
            assert_ne!(title(code, Lang::Es), Some(*english), "{code}");
            assert_ne!(title(code, Lang::Pt), Some(*english), "{code}");
        }
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, english) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(english), "{code}: {row}");
        }
    }

    fn limits(p: &DfmProfile) -> [i64; 8] {
        let r = &p.rules;
        [
            r.min_track_width.get(),
            r.min_clearance.get(),
            r.min_drill.get(),
            r.min_via_diameter.get(),
            r.min_annular_ring.get(),
            r.min_edge_clearance.get(),
            r.min_mask_bridge.get(),
            r.min_drill_spacing.get(),
        ]
    }

    #[test]
    fn the_profiles_that_come_with_the_program_hold_the_figures_of_their_files() {
        assert_eq!(builtin_profile("JLCPCB"), Some(BUILTIN[1].1));
        assert_eq!(builtin_profile("nobody"), None);
        assert_eq!(builtin_names(), "example, jlcpcb, pcbway");
        let jlc = DfmProfile::parse(builtin_profile("jlcpcb").unwrap()).unwrap();
        assert_eq!(jlc.name, "JLCPCB standard rigid FR-4");
        assert!(jlc.source.starts_with("https://jlcpcb.com/"));
        assert!(!jlc.verified);
        assert_eq!(
            limits(&jlc),
            [100_000, 100_000, 150_000, 250_000, 50_000, 200_000, 100_000, 200_000]
        );
        assert_eq!(jlc.max_copper_layers, 32);
        assert_eq!(
            (
                jlc.min_board_short_nm,
                jlc.max_board_short_nm,
                jlc.max_board_long_nm
            ),
            (3_000_000, 600_000_000, 670_000_000)
        );
        assert_eq!((jlc.max_drill_nm, jlc.max_holes), (6_300_000, 0));
        let pcbway = DfmProfile::parse(builtin_profile("pcbway").unwrap()).unwrap();
        assert_eq!(pcbway.name, "PCBWay standard rigid FR-4");
        assert!(pcbway.source.starts_with("https://www.pcbway.com/"));
        assert!(!pcbway.verified);
        assert_eq!(
            limits(&pcbway),
            [100_000, 100_000, 150_000, 0, 150_000, 250_000, 101_600, 279_400]
        );
        assert_eq!(pcbway.max_copper_layers, 14);
        assert_eq!(
            (pcbway.max_board_short_nm, pcbway.max_board_long_nm),
            (560_000_000, 1_150_000_000)
        );
        assert_eq!(pcbway.max_drill_nm, 6_000_000);
    }

    #[test]
    fn a_board_is_judged_against_each_maker_with_the_figures_of_its_file() {
        // the board of these tests has two copper layers and is 25 x 40 mm: it fits both makers
        for name in ["jlcpcb", "pcbway"] {
            let profile = DfmProfile::parse(builtin_profile(name).unwrap()).unwrap();
            assert_eq!(
                codes(&dfm_findings(&board(), &profile)),
                [UNVERIFIED],
                "{name}"
            );
        }
        // a maker that builds fewer layers than the board has is refused
        let mut small = DfmProfile::parse(builtin_profile("pcbway").unwrap()).unwrap();
        small.max_copper_layers = 1;
        assert_eq!(codes(&dfm_findings(&board(), &small)), [LAYERS, UNVERIFIED]);
    }

    #[test]
    fn the_profiles_keep_their_bytes_and_say_where_the_figures_came_from() {
        for (name, text) in BUILTIN {
            // the files of the makers are not converted by git; the example is
            assert!(
                *name == "example" || !text.contains('\r'),
                "{name} has carriage returns"
            );
            let profile = DfmProfile::parse(text).unwrap();
            assert!(
                !profile.name.is_empty() && !profile.source.is_empty(),
                "{name}"
            );
            // every length is a whole number of micrometres
            for nm in limits(&profile) {
                assert_eq!(nm % 100, 0, "{name}");
            }
        }
    }
}
