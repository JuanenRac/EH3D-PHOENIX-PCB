//! The capability profiles of the makers that come with the program and are made from design rules
//! (`*-dru`) and from public figures (`ncab`): the numbers of each file, the lines that stay off
//! and the shape of the files.

use eh3d_phoenix_rules::{builtin_names, builtin_profile, DfmProfile, BUILTIN};

/// The figures of a profile of design rules, in nanometres.
struct Figures {
    name: &'static str,
    layers: u32,
    track: i64,
    clearance: i64,
    drill: i64,
    ring: i64,
    edge: i64,
    hole_gap: i64,
    pad_ring: i64,
}

fn figures(name: &'static str, layers: u32, numbers: [i64; 6]) -> Figures {
    Figures {
        name,
        layers,
        track: numbers[0],
        clearance: numbers[1],
        drill: numbers[2],
        ring: numbers[3],
        edge: numbers[4],
        hole_gap: numbers[5],
        pad_ring: 0,
    }
}

fn with_pad_ring(f: Figures, pad_ring: i64) -> Figures {
    Figures { pad_ring, ..f }
}

fn all_figures() -> Vec<Figures> {
    vec![
        with_pad_ring(
            figures(
                "pcbway-dru",
                2,
                [152_400, 205_000, 300_000, 203_200, 1_016_000, 203_200],
            ),
            254_000,
        ),
        with_pad_ring(
            figures(
                "oshpark-2l-dru",
                2,
                [152_400, 152_400, 254_000, 127_000, 381_000, 152_400],
            ),
            127_000,
        ),
        with_pad_ring(
            figures(
                "oshpark-4l-dru",
                4,
                [127_000, 127_000, 254_000, 101_600, 381_000, 152_400],
            ),
            101_600,
        ),
        with_pad_ring(
            figures(
                "eurocircuits-1l-dru",
                1,
                [150_000, 150_000, 350_000, 300_000, 250_000, 350_000],
            ),
            300_000,
        ),
        with_pad_ring(
            figures(
                "eurocircuits-2l-dru",
                2,
                [150_000, 150_000, 350_000, 125_000, 250_000, 350_000],
            ),
            175_000,
        ),
        with_pad_ring(
            figures(
                "eurocircuits-4l-dru",
                4,
                [150_000, 150_000, 350_000, 125_000, 250_000, 350_000],
            ),
            175_000,
        ),
        with_pad_ring(
            figures(
                "eurocircuits-6l-dru",
                6,
                [150_000, 150_000, 350_000, 125_000, 250_000, 350_000],
            ),
            175_000,
        ),
        with_pad_ring(
            figures(
                "pcbpool-dru",
                2,
                [152_400, 152_400, 304_800, 152_400, 508_000, 203_200],
            ),
            203_200,
        ),
        with_pad_ring(
            figures(
                "allpcb-1l-dru",
                1,
                [152_400, 152_400, 250_000, 152_400, 304_800, 254_000],
            ),
            152_400,
        ),
        with_pad_ring(
            figures(
                "allpcb-2l-dru",
                2,
                [152_400, 152_400, 250_000, 152_400, 304_800, 254_000],
            ),
            152_400,
        ),
        with_pad_ring(
            figures(
                "allpcb-4l-dru",
                4,
                [152_400, 152_400, 250_000, 152_400, 304_800, 254_000],
            ),
            152_400,
        ),
        with_pad_ring(
            figures(
                "allpcb-6l-dru",
                6,
                [152_400, 152_400, 250_000, 152_400, 304_800, 254_000],
            ),
            152_400,
        ),
        with_pad_ring(
            figures(
                "generic-8mil-dru",
                0,
                [254_000, 203_200, 609_600, 203_200, 1_016_000, 254_000],
            ),
            254_000,
        ),
        with_pad_ring(
            figures(
                "generic-10mil-dru",
                0,
                [254_000, 254_000, 609_600, 203_200, 1_016_000, 254_000],
            ),
            254_000,
        ),
    ]
}

fn profile(name: &str) -> DfmProfile {
    DfmProfile::parse(builtin_profile(name).unwrap_or_else(|| panic!("no profile {name}")))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn the_profiles_that_come_with_the_program_are_listed_in_order() {
    assert_eq!(
        builtin_names(),
        "example, jlcpcb, pcbway, ncab, pcbway-dru, oshpark-2l-dru, oshpark-4l-dru, \
         eurocircuits-1l-dru, eurocircuits-2l-dru, eurocircuits-4l-dru, eurocircuits-6l-dru, \
         pcbpool-dru, allpcb-1l-dru, allpcb-2l-dru, allpcb-4l-dru, allpcb-6l-dru, \
         generic-8mil-dru, generic-10mil-dru"
    );
    assert_eq!(BUILTIN.len(), 18);
}

#[test]
fn each_profile_of_design_rules_holds_the_numbers_of_its_rules() {
    for f in all_figures() {
        let p = profile(f.name);
        let r = &p.rules;
        assert_eq!(r.min_track_width.get(), f.track, "{} track", f.name);
        assert_eq!(r.min_clearance.get(), f.clearance, "{} clearance", f.name);
        assert_eq!(r.min_drill.get(), f.drill, "{} drill", f.name);
        assert_eq!(r.min_annular_ring.get(), f.ring, "{} ring", f.name);
        assert_eq!(r.min_edge_clearance.get(), f.edge, "{} edge", f.name);
        assert_eq!(r.min_drill_spacing.get(), f.hole_gap, "{} hole gap", f.name);
        assert_eq!(p.min_pad_ring_nm, f.pad_ring, "{} pad ring", f.name);
        assert_eq!(p.max_copper_layers, f.layers, "{} layers", f.name);
    }
}

#[test]
fn a_profile_of_design_rules_leaves_off_what_the_rules_do_not_say() {
    for f in all_figures() {
        let p = profile(f.name);
        assert!(!p.verified, "{}", f.name);
        assert_eq!(
            p.source, "design rules of the manufacturer as shipped with EAGLE",
            "{}",
            f.name
        );
        assert!(!p.name.is_empty(), "{}", f.name);
        // the lines that a design rule file has no number for are off
        assert_eq!(p.rules.min_via_diameter.get(), 0, "{}", f.name);
        assert_eq!(p.rules.min_mask_bridge.get(), 0, "{}", f.name);
        assert_eq!(p.rules.connect_tolerance.get(), 0, "{}", f.name);
        assert_eq!(
            (
                p.max_board_long_nm,
                p.max_board_short_nm,
                p.max_holes,
                p.board_thickness_nm
            ),
            (0, 0, 0, 0),
            "{}",
            f.name
        );
        assert_eq!(
            (
                p.min_smd_pad_nm,
                p.min_silk_stroke_nm,
                p.min_cutout_nm,
                p.min_unplated_drill_nm
            ),
            (0, 0, 0, 0),
            "{}",
            f.name
        );
    }
}

#[test]
fn the_number_of_layers_in_the_name_is_the_number_in_the_file() {
    for (name, _) in BUILTIN.iter().filter(|(n, _)| n.ends_with("-dru")) {
        let layers = name
            .split('-')
            .find_map(|part| part.strip_suffix('l').and_then(|n| n.parse::<u32>().ok()));
        if let Some(layers) = layers {
            assert_eq!(profile(name).max_copper_layers, layers, "{name}");
        }
    }
}

#[test]
fn the_files_carry_no_private_text_and_no_date() {
    for (name, text) in BUILTIN {
        for word in ["C:\\", "/Users", "TRABAJO", "SONNET", "Claude"] {
            assert!(!text.contains(word), "{name}: {word}");
        }
        // no year in a comment: the figures are the only numbers of a line
        for line in text.lines().filter(|l| l.starts_with('#')) {
            let digits: Vec<&str> = line.split(|c: char| !c.is_ascii_digit()).collect();
            assert!(
                !digits.iter().any(|d| d.len() == 4 && d.starts_with("20")),
                "{name}: {line}"
            );
        }
    }
}

/// A small generator with a fixed seed (xorshift).
struct Seeded(u64);

impl Seeded {
    fn step(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn the_order_of_the_lines_of_a_profile_does_not_change_what_it_says() {
    let mut rng = Seeded(0x00d1_f0a5_0000_0017);
    for (name, text) in BUILTIN {
        let wanted = DfmProfile::parse(text).unwrap();
        let mut lines: Vec<&str> = text.lines().collect();
        for _ in 0..8 {
            // Fisher-Yates with the seeded generator
            for i in (1..lines.len()).rev() {
                let j = (rng.step() % (i as u64 + 1)) as usize;
                lines.swap(i, j);
            }
            let shuffled = lines.join("\n");
            assert_eq!(DfmProfile::parse(&shuffled).unwrap(), wanted, "{name}");
        }
    }
}

#[test]
fn the_profile_of_jlcpcb_holds_the_further_public_figures() {
    let p = profile("jlcpcb");
    assert!(!p.verified);
    assert_eq!(p.rules.min_track_width.get(), 100_000);
    assert_eq!(p.min_unplated_drill_nm, 500_000);
    assert_eq!(p.min_cutout_nm, 1_000_000);
    assert_eq!(p.min_smd_pad_nm, 250_000);
    assert_eq!(p.min_silk_stroke_nm, 150_000);
    assert_eq!(p.min_silk_text_height_nm, 1_000_000);
    // what stayed as it was
    assert_eq!((p.min_pad_ring_nm, p.min_hole_edge_nm), (150_000, 0));
}

#[test]
fn the_profile_of_ncab_holds_only_what_its_sheet_says() {
    let p = profile("ncab");
    assert!(!p.verified);
    assert_eq!(p.rules.min_track_width.get(), 50_000);
    assert_eq!(p.rules.min_clearance.get(), 50_000);
    assert_eq!(p.rules.min_drill.get(), 100_000);
    assert_eq!(p.max_copper_layers, 0);
    assert_eq!(p.rules.min_mask_bridge.get(), 0);
    assert_eq!(p.max_board_long_nm, 0);
}
