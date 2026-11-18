// SPDX-License-Identifier: GPL-3.0-or-later
//! Further lines of a capability profile of a board maker, beyond the ones of [`crate::dfm`]:
//! the number of different drill sizes, the aspect ratio of the holes, vias placed in pads, the
//! size of the silkscreen, an odd number of copper layers, the proportions of the board, blind
//! and buried vias and the smallest surface-mount pad.
//!
//! Every line is off when it is missing, 0 or `false`, so a profile written before these lines
//! existed gives the same findings as it did. The findings are errors and take part in the exit
//! code of `phoenix check --dfm`; the figures that decide them are in the profile file and
//! nowhere else.

use std::collections::BTreeSet;

use eh3d_phoenix_board::{Board, Id, Via};

use crate::codes::Lang;
use crate::dfm::DfmProfile;
use crate::geometry::pad_min_side;
use crate::items::placed_pads;
use crate::report::{Finding, Severity};

/// More different drill sizes than the profile accepts.
pub const DRILL_SIZES: &str = "PHOENIX-DFM-007";
/// A hole whose depth is too large for its drill.
pub const HOLE_ASPECT: &str = "PHOENIX-DFM-008";
/// A via placed inside a pad.
pub const VIA_IN_PAD: &str = "PHOENIX-DFM-009";

/// Every code of this module with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (
        DRILL_SIZES,
        "More different drill sizes than the profile accepts",
    ),
    (
        HOLE_ASPECT,
        "Hole aspect ratio above the limit of the profile",
    ),
    (
        VIA_IN_PAD,
        "Via inside a pad, which the profile does not allow",
    ),
];

const TITLES: &[(&str, [&str; 8])] = &[
    (
        DRILL_SIZES,
        [
            "More different drill sizes than the profile accepts",
            "Más tamaños de taladro distintos de los que acepta el perfil",
            "Mehr verschiedene Bohrdurchmesser, als das Profil zulässt",
            "Plus de diamètres de perçage différents que le profil n'en accepte",
            "Più diametri di foratura diversi di quanti ne accetti il profilo",
            "異なるドリル径の種類がプロファイルの許容を超えています",
            "不同钻孔尺寸的数量超过配置文件允许的数量",
            "Mais tamanhos de broca diferentes do que o perfil aceita",
        ],
    ),
    (
        HOLE_ASPECT,
        [
            "Hole aspect ratio above the limit of the profile",
            "Relación de aspecto de un agujero por encima del límite del perfil",
            "Aspektverhältnis einer Bohrung über dem Grenzwert des Profils",
            "Rapport d'aspect d'un perçage supérieur à la limite du profil",
            "Rapporto di forma di un foro superiore al limite del profilo",
            "穴のアスペクト比がプロファイルの上限を超えています",
            "孔的纵横比超过配置文件的限值",
            "Relação de aspeto de um furo acima do limite do perfil",
        ],
    ),
    (
        VIA_IN_PAD,
        [
            "Via inside a pad, which the profile does not allow",
            "Vía dentro de un pad, que el perfil no permite",
            "Durchkontaktierung in einem Pad, die das Profil nicht zulässt",
            "Via dans une pastille, que le profil n'autorise pas",
            "Via dentro un pad, che il profilo non consente",
            "プロファイルが許可していないパッド内のビア",
            "配置文件不允许的焊盘内过孔",
            "Via dentro de um pad, que o perfil não permite",
        ],
    ),
];

/// The title of a code of this module in a language; `None` for any other code.
pub fn title(code: &str, lang: Lang) -> Option<&'static str> {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, texts)| texts[lang.column()])
}

/// The drill of every hole of the board: the vias and the drilled pads of the placed components.
fn drills(board: &Board) -> Vec<i64> {
    let mut out: Vec<i64> = board.vias.values().map(|via| via.drill.get()).collect();
    for component in board.components.values() {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        out.extend(
            footprint
                .pads
                .values()
                .filter_map(|pad| pad.drill)
                .map(|drill| drill.get()),
        );
    }
    out
}

/// The findings of the lines of this module; `name` is the name of the profile for the words.
pub(crate) fn findings(board: &Board, profile: &DfmProfile, name: &str, out: &mut Vec<Finding>) {
    drill_sizes(board, profile, name, out);
    via_in_pad(board, profile, out);
    hole_aspect(board, profile, out);
}

fn drill_sizes(board: &Board, profile: &DfmProfile, name: &str, out: &mut Vec<Finding>) {
    if profile.max_drill_sizes == 0 {
        return;
    }
    let sizes: BTreeSet<i64> = drills(board).into_iter().collect();
    if sizes.len() as u64 > u64::from(profile.max_drill_sizes) {
        out.push(
            Finding::new(
                46,
                "dfm_drill_sizes",
                DRILL_SIZES,
                Severity::Error,
                "holes".to_owned(),
            )
            .why(&format!(
                "{} different drill sizes, the profile {name} accepts {}",
                sizes.len(),
                profile.max_drill_sizes
            )),
        );
    }
}

/// The smallest drill that the board thickness of the profile allows at the aspect ratio limit,
/// rounded up to a whole nanometre: `thickness / drill <= max_x10 / 10`.
fn least_drill_nm(thickness_nm: i64, max_aspect_x10: u32) -> i64 {
    let thickness = u128::try_from(thickness_nm).unwrap_or(0) * 10;
    let max = u128::from(max_aspect_x10).max(1);
    i64::try_from(thickness.div_ceil(max)).unwrap_or(i64::MAX)
}

fn hole_aspect(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    if profile.board_thickness_nm <= 0 || profile.max_hole_aspect_x10 == 0 {
        return;
    }
    let Some(smallest) = drills(board).into_iter().filter(|d| *d > 0).min() else {
        return;
    };
    let needed = least_drill_nm(profile.board_thickness_nm, profile.max_hole_aspect_x10);
    if smallest < needed {
        let tenths = i128::from(profile.board_thickness_nm) * 10 / i128::from(smallest);
        out.push(
            Finding::new(
                47,
                "dfm_hole_aspect",
                HOLE_ASPECT,
                Severity::Error,
                "holes".to_owned(),
            )
            .values(smallest, needed)
            .why(&format!(
                "aspect ratio {}.{}:1 for the smallest drill, the limit is {}.{}:1",
                tenths / 10,
                tenths % 10,
                profile.max_hole_aspect_x10 / 10,
                profile.max_hole_aspect_x10 % 10
            )),
        );
    }
}

/// The copper layers a via joins: those of the stack from its first layer to its last, or just
/// the two it names when they are not both in the stack.
fn via_layers(board: &Board, via: &Via) -> Vec<Id> {
    let stack = &board.copper_stack;
    let first = stack.iter().position(|l| *l == via.from_layer);
    let last = stack.iter().position(|l| *l == via.to_layer);
    match (first, last) {
        (Some(a), Some(b)) => stack[a.min(b)..=a.max(b)].to_vec(),
        _ => vec![via.from_layer, via.to_layer],
    }
}

/// A via whose centre is inside the largest circle that fits in a pad of a part, on a layer both
/// reach. A via at the corner of a rectangular pad is not found: the test never says yes wrongly.
fn via_in_pad(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    if !profile.forbid_via_in_pad {
        return;
    }
    let pads = placed_pads(board);
    for via in board.vias.values() {
        let layers = via_layers(board, via);
        for pad in &pads {
            let side = pad_min_side(pad.shape);
            if 4 * via.at.distance_squared(pad.at) > side * side {
                continue;
            }
            if !pad.layers.iter().any(|l| layers.contains(l)) {
                continue;
            }
            out.push(
                Finding::new(
                    48,
                    "dfm_via_in_pad",
                    VIA_IN_PAD,
                    Severity::Error,
                    pad.label.clone(),
                )
                .at(via.at.x_nm.get(), via.at.y_nm.get())
                .why("a via is placed inside this pad"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dfm::dfm_findings;

    fn codes(board: &Board, text: &str) -> Vec<&'static str> {
        let profile = DfmProfile::parse(&format!("verified = true\n{text}")).unwrap();
        dfm_findings(board, &profile)
            .confirmed
            .iter()
            .map(|f| f.code)
            .collect()
    }

    #[test]
    fn the_different_drill_sizes_are_counted_once_each() {
        // the vias drill 0.3 mm and 0.8 mm and the two pads 1.0 mm: three sizes
        let board = crate::dfm::tests::board();
        assert_eq!(drills(&board).len(), 4);
        assert_eq!(codes(&board, "max_drill_sizes = 3\n"), Vec::<&str>::new());
        assert_eq!(codes(&board, "max_drill_sizes = 2\n"), [DRILL_SIZES]);
        assert_eq!(codes(&board, "max_drill_sizes = 0\n"), Vec::<&str>::new());
        let profile = DfmProfile::parse("verified = true\nmax_drill_sizes = 1\n").unwrap();
        let report = dfm_findings(&board, &profile);
        assert!(report.confirmed[0]
            .detail
            .starts_with("3 different drill sizes"));
        assert!(!report.passes());
    }

    #[test]
    fn a_board_without_holes_has_no_drill_sizes() {
        assert_eq!(
            codes(&Board::default(), "max_drill_sizes = 1\n"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn the_value_of_the_line_must_be_a_small_whole_number() {
        for text in [
            "max_drill_sizes = -1\n",
            "max_drill_sizes = many\n",
            "max_drill_sizes = 10001\n",
        ] {
            assert!(DfmProfile::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn a_hole_deeper_than_the_aspect_limit_is_reported_with_the_drill_that_would_pass() {
        // the smallest drill is 0.3 mm and the board is 1.6 mm thick: 5.3 to 1
        let board = crate::dfm::tests::board();
        let text = "board_thickness_nm = 1600000\nmax_hole_aspect_x10 = 50\n";
        assert_eq!(codes(&board, text), [HOLE_ASPECT]);
        let profile = DfmProfile::parse(&format!("verified = true\n{text}")).unwrap();
        let finding = dfm_findings(&board, &profile).confirmed.remove(0);
        // 16 mm / 5 = 0.32 mm is the least drill that passes
        assert_eq!(
            (finding.measured_nm, finding.limit_nm),
            (Some(300_000), Some(320_000))
        );
        assert!(finding.detail.starts_with("aspect ratio 5.3:1"));
        assert!(finding.detail.ends_with("is 5.0:1"));
        // 6 to 1 accepts it, and the line is off when either figure is 0
        let wide = "board_thickness_nm = 1600000\nmax_hole_aspect_x10 = 60\n";
        assert_eq!(codes(&board, wide), Vec::<&str>::new());
        for off in [
            "board_thickness_nm = 0\nmax_hole_aspect_x10 = 10\n",
            "board_thickness_nm = 1600000\nmax_hole_aspect_x10 = 0\n",
            "board_thickness_nm = 1600000\n",
        ] {
            assert_eq!(codes(&board, off), Vec::<&str>::new(), "{off}");
        }
    }

    #[test]
    fn the_aspect_limit_is_exact_at_the_edge_and_never_overflows() {
        // exactly 0.32 mm at 5.0 to 1 passes, one nanometre less does not
        assert_eq!(least_drill_nm(1_600_000, 50), 320_000);
        assert_eq!(least_drill_nm(1_600_001, 50), 320_001);
        assert_eq!(least_drill_nm(1_000_000_000_000, 1), 10_000_000_000_000);
        assert_eq!(least_drill_nm(1, 0), 10);
        assert!(DfmProfile::parse("board_thickness_nm = -1\n").is_err());
        assert!(DfmProfile::parse("max_hole_aspect_x10 = 1001\n").is_err());
        assert!(DfmProfile::parse("max_hole_aspect_x10 = 1000\n").is_ok());
    }

    fn with_via(at: (i64, i64), layer: u32) -> Board {
        use crate::dfm::tests::{id, pt};
        let mut board = crate::dfm::tests::board();
        let mut via = board.vias.values().next().unwrap().clone();
        via.at = pt(at.0, at.1);
        via.from_layer = id(layer);
        via.to_layer = id(layer);
        board.vias.insert(id(60), via);
        board
    }

    #[test]
    fn a_via_inside_a_pad_is_reported_where_it_is() {
        // the first pad is a circle of 1.8 mm at (20 mm, 12 mm) on the layers 1 and 2
        let inside = with_via((20_400_000, 12_000_000), 2);
        let profile = DfmProfile::parse("verified = true\nforbid_via_in_pad = true\n").unwrap();
        let report = dfm_findings(&inside, &profile);
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        let finding = &report.confirmed[0];
        assert_eq!(finding.code, VIA_IN_PAD);
        assert_eq!(finding.location, "J1 pad 1");
        assert_eq!(finding.at_nm, Some([20_400_000, 12_000_000]));
        assert!(!report.passes());
        // the line is off unless it is switched on
        assert_eq!(
            codes(&inside, "forbid_via_in_pad = false\n"),
            Vec::<&str>::new()
        );
        // the vias of the plain board are far from the pads
        let plain = crate::dfm::tests::board();
        assert_eq!(
            codes(&plain, "forbid_via_in_pad = true\n"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn a_via_just_outside_the_pad_or_on_another_layer_is_not_in_it() {
        // the radius of the pad is 0.9 mm: 0.9 mm away is inside (the edge counts), 0.91 is not
        assert_eq!(
            codes(
                &with_via((20_900_000, 12_000_000), 1),
                "forbid_via_in_pad = true\n"
            ),
            [VIA_IN_PAD]
        );
        assert_eq!(
            codes(
                &with_via((20_910_000, 12_000_000), 1),
                "forbid_via_in_pad = true\n"
            ),
            Vec::<&str>::new()
        );
        // a via on a layer that the pad does not reach
        assert_eq!(
            codes(
                &with_via((20_000_000, 12_000_000), 3),
                "forbid_via_in_pad = true\n"
            ),
            Vec::<&str>::new()
        );
        assert!(DfmProfile::parse("forbid_via_in_pad = maybe\n").is_err());
    }

    #[test]
    fn every_code_has_a_title_in_all_eight_languages_and_a_row_in_the_table() {
        assert_eq!(REGISTRY.len(), TITLES.len());
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, english) in REGISTRY {
            for lang in Lang::ALL {
                assert!(!title(code, lang).unwrap_or_default().is_empty(), "{code}");
            }
            assert_eq!(title(code, Lang::En), Some(*english));
            assert_ne!(title(code, Lang::Es), Some(*english), "{code}");
            assert_ne!(title(code, Lang::Pt), Some(*english), "{code}");
            assert_eq!(crate::codes::title(code, Lang::De), title(code, Lang::De));
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(english), "{code}: {row}");
        }
    }
}
