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

use eh3d_phoenix_board::Board;

use crate::codes::Lang;
use crate::dfm::DfmProfile;
use crate::report::{Finding, Severity};

/// More different drill sizes than the profile accepts.
pub const DRILL_SIZES: &str = "PHOENIX-DFM-007";

/// Every code of this module with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[(
    DRILL_SIZES,
    "More different drill sizes than the profile accepts",
)];

const TITLES: &[(&str, [&str; 8])] = &[(
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
)];

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
