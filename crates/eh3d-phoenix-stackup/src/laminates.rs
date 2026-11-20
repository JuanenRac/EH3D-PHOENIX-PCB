// SPDX-License-Identifier: GPL-3.0-or-later
//! A small library of laminates: the figures that the published data sheets of some common
//! materials state, written down once so that a stack-up can start from them.
//!
//! **Nothing here is verified.** Every entry has `verified: false`: the figures were typed in from
//! the data sheet of the maker, they were not measured and not compared with a second source, and
//! a data sheet states them for a stated frequency, a stated resin content and a stated test
//! method that a real board does not share. A figure that the sheet does not state clearly is
//! left undefined (`None`), never guessed. Use the library to get a first estimate and replace the
//! figures with the ones of the laminate the fabricator quotes.
//!
//! Frequencies are in GHz, temperatures in degrees Celsius, expansion in ppm per degree Celsius
//! and thicknesses in micrometres (the commercial thicknesses of a core, as the sheet lists them).

use crate::material::MaterialSample;

/// The kind of a laminate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Glass-reinforced epoxy (FR-4 and its variants).
    Epoxy,
    /// Hydrocarbon or PTFE laminates for high frequencies.
    HighFrequency,
    /// Polyimide laminates and films, rigid or flexible.
    Polyimide,
    /// A dielectric on a metal base (insulated metal substrate).
    MetalBase,
    /// A conductor, listed for its conductivity.
    Conductor,
}

/// The figures a data sheet states for one laminate. `None` is a figure the sheet does not state
/// clearly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Laminate {
    /// The identifier used by the library: lowercase, digits and hyphens.
    pub id: &'static str,
    /// The maker.
    pub maker: &'static str,
    /// The name of the product as the sheet writes it.
    pub name: &'static str,
    /// The kind of laminate.
    pub family: Family,
    /// The dielectric constant.
    pub dk: Option<f64>,
    /// The frequency the dielectric constant is stated at, in GHz.
    pub dk_ghz: Option<f64>,
    /// The loss tangent.
    pub df: Option<f64>,
    /// The frequency the loss tangent is stated at, in GHz.
    pub df_ghz: Option<f64>,
    /// The glass transition temperature.
    pub tg_c: Option<f64>,
    /// Whether `tg_c` is a lower bound (the sheet says "over" or "greater than").
    pub tg_is_lower_bound: bool,
    /// The decomposition temperature (5 % weight loss unless the sheet says otherwise).
    pub td_c: Option<f64>,
    /// The expansion in the plane (X), below the glass transition.
    pub cte_x_ppm: Option<f64>,
    /// The expansion in the plane (Y), below the glass transition.
    pub cte_y_ppm: Option<f64>,
    /// The expansion through the thickness (Z), below the glass transition.
    pub cte_z_ppm: Option<f64>,
    /// The expansion through the thickness (Z), above the glass transition.
    pub cte_z_above_tg_ppm: Option<f64>,
    /// The thermal conductivity in watts per metre and kelvin.
    pub thermal_w_per_mk: Option<f64>,
    /// The conductivity in siemens per metre (a conductor).
    pub conductivity_s_per_m: Option<f64>,
    /// The commercial thicknesses of a core, in micrometres, from the thinnest.
    pub core_thicknesses_um: &'static [u32],
    /// Whether the figures were checked against a second source. Always false here.
    pub verified: bool,
}

/// An entry with every figure undefined, to build the others from.
const NONE: Laminate = Laminate {
    id: "",
    maker: "",
    name: "",
    family: Family::Epoxy,
    dk: None,
    dk_ghz: None,
    df: None,
    df_ghz: None,
    tg_c: None,
    tg_is_lower_bound: false,
    td_c: None,
    cte_x_ppm: None,
    cte_y_ppm: None,
    cte_z_ppm: None,
    cte_z_above_tg_ppm: None,
    thermal_w_per_mk: None,
    conductivity_s_per_m: None,
    core_thicknesses_um: &[],
    verified: false,
};

/// The laminates of the library, in the order of their identifiers.
pub const LAMINATES: &[Laminate] = &[
    Laminate {
        id: "arlon-med-85n",
        maker: "Arlon",
        name: "85N",
        family: Family::Polyimide,
        tg_c: Some(250.0),
        tg_is_lower_bound: true,
        td_c: Some(407.0),
        ..NONE
    },
    Laminate {
        id: "copper-foil",
        maker: "",
        name: "Copper foil",
        family: Family::Conductor,
        conductivity_s_per_m: Some(59_590_000.0),
        ..NONE
    },
    Laminate {
        id: "dupont-pyralux-ap",
        maker: "DuPont",
        name: "Pyralux AP",
        family: Family::Polyimide,
        dk: Some(3.4),
        dk_ghz: Some(0.001),
        tg_c: Some(220.0),
        cte_x_ppm: Some(25.0),
        cte_y_ppm: Some(25.0),
        ..NONE
    },
    Laminate {
        id: "isola-370hr",
        maker: "Isola",
        name: "370HR",
        family: Family::Epoxy,
        dk: Some(4.04),
        df: Some(0.0210),
        tg_c: Some(180.0),
        td_c: Some(340.0),
        ..NONE
    },
    Laminate {
        id: "isola-is400",
        maker: "Isola",
        name: "IS400",
        family: Family::Epoxy,
        dk: Some(4.00),
        dk_ghz: Some(0.1),
        df: Some(0.020),
        df_ghz: Some(0.1),
        tg_c: Some(150.0),
        td_c: Some(330.0),
        cte_x_ppm: Some(13.0),
        cte_y_ppm: Some(13.0),
        cte_z_ppm: Some(50.0),
        cte_z_above_tg_ppm: Some(250.0),
        ..NONE
    },
    Laminate {
        id: "iteq-it-158",
        maker: "ITEQ",
        name: "IT-158",
        family: Family::Epoxy,
        tg_c: Some(150.0),
        ..NONE
    },
    Laminate {
        id: "rogers-ro3003",
        maker: "Rogers",
        name: "RO3003",
        family: Family::HighFrequency,
        dk: Some(3.00),
        dk_ghz: Some(10.0),
        df: Some(0.0013),
        df_ghz: Some(10.0),
        cte_x_ppm: Some(17.0),
        cte_y_ppm: Some(16.0),
        cte_z_ppm: Some(25.0),
        thermal_w_per_mk: Some(0.50),
        core_thicknesses_um: &[130, 250, 500, 750, 1520],
        ..NONE
    },
    Laminate {
        id: "rogers-ro3006",
        maker: "Rogers",
        name: "RO3006",
        family: Family::HighFrequency,
        dk: Some(6.15),
        dk_ghz: Some(10.0),
        df: Some(0.0020),
        df_ghz: Some(10.0),
        cte_x_ppm: Some(17.0),
        cte_y_ppm: Some(17.0),
        cte_z_ppm: Some(24.0),
        thermal_w_per_mk: Some(0.79),
        core_thicknesses_um: &[130, 250, 640, 1280],
        ..NONE
    },
    Laminate {
        id: "rogers-ro3010",
        maker: "Rogers",
        name: "RO3010",
        family: Family::HighFrequency,
        dk: Some(10.2),
        dk_ghz: Some(10.0),
        df: Some(0.0022),
        df_ghz: Some(10.0),
        cte_x_ppm: Some(13.0),
        cte_y_ppm: Some(11.0),
        cte_z_ppm: Some(16.0),
        thermal_w_per_mk: Some(0.95),
        core_thicknesses_um: &[130, 250, 640, 1280],
        ..NONE
    },
    Laminate {
        id: "rogers-ro4003c",
        maker: "Rogers",
        name: "RO4003C",
        family: Family::HighFrequency,
        dk: Some(3.38),
        dk_ghz: Some(10.0),
        df: Some(0.0027),
        df_ghz: Some(10.0),
        tg_c: Some(280.0),
        tg_is_lower_bound: true,
        td_c: Some(425.0),
        cte_x_ppm: Some(11.0),
        cte_y_ppm: Some(14.0),
        cte_z_ppm: Some(46.0),
        thermal_w_per_mk: Some(0.71),
        core_thicknesses_um: &[203, 305, 406, 508, 813, 1524],
        ..NONE
    },
    Laminate {
        id: "rogers-ro4350b",
        maker: "Rogers",
        name: "RO4350B",
        family: Family::HighFrequency,
        dk: Some(3.48),
        dk_ghz: Some(10.0),
        df: Some(0.0037),
        df_ghz: Some(10.0),
        tg_c: Some(280.0),
        tg_is_lower_bound: true,
        td_c: Some(390.0),
        cte_x_ppm: Some(10.0),
        cte_y_ppm: Some(12.0),
        cte_z_ppm: Some(32.0),
        thermal_w_per_mk: Some(0.69),
        core_thicknesses_um: &[101, 168, 254, 338, 422, 508, 762, 1524],
        ..NONE
    },
    Laminate {
        id: "shengyi-s1000-2",
        maker: "Shengyi",
        name: "S1000-2",
        family: Family::Epoxy,
        dk: Some(4.80),
        df: Some(0.013),
        tg_c: Some(180.0),
        td_c: Some(345.0),
        cte_z_ppm: Some(45.0),
        cte_z_above_tg_ppm: Some(220.0),
        thermal_w_per_mk: Some(0.62),
        ..NONE
    },
    Laminate {
        id: "taconic-rf-35",
        maker: "Taconic",
        name: "RF-35",
        family: Family::HighFrequency,
        dk: Some(3.50),
        dk_ghz: Some(1.9),
        df: Some(0.0018),
        df_ghz: Some(1.9),
        tg_c: Some(315.0),
        tg_is_lower_bound: true,
        thermal_w_per_mk: Some(0.24),
        ..NONE
    },
    Laminate {
        id: "taconic-tlc",
        maker: "Taconic",
        name: "TLC",
        family: Family::HighFrequency,
        df: Some(0.0030),
        df_ghz: Some(10.0),
        cte_z_ppm: Some(70.0),
        thermal_w_per_mk: Some(0.24),
        ..NONE
    },
    Laminate {
        id: "taconic-tlx",
        maker: "Taconic",
        name: "TLX",
        family: Family::HighFrequency,
        dk: Some(2.50),
        dk_ghz: Some(10.0),
        df: Some(0.0019),
        df_ghz: Some(10.0),
        ..NONE
    },
    Laminate {
        id: "thinflex-w-0505ed",
        maker: "Thinflex",
        name: "W-0505ED",
        family: Family::Polyimide,
        dk: Some(3.3),
        dk_ghz: Some(1.0),
        df: Some(0.01),
        df_ghz: Some(1.0),
        cte_x_ppm: Some(24.0),
        cte_y_ppm: Some(24.0),
        ..NONE
    },
    Laminate {
        id: "ventec-vt-4b3",
        maker: "Ventec",
        name: "VT-4B3",
        family: Family::MetalBase,
        dk: Some(4.8),
        dk_ghz: Some(0.001),
        df: Some(0.016),
        df_ghz: Some(0.001),
        tg_c: Some(130.0),
        td_c: Some(380.0),
        thermal_w_per_mk: Some(3.0),
        ..NONE
    },
];

/// The laminate with an identifier, in any case.
pub fn find(id: &str) -> Option<&'static Laminate> {
    LAMINATES
        .iter()
        .find(|l| l.id.eq_ignore_ascii_case(id.trim()))
}

impl Laminate {
    /// The material sample of the laminate: the dielectric constant and the loss tangent at the
    /// frequencies the sheet states them at. `None` when the sheet states no constant, no loss
    /// tangent or no frequency for one of them, because a sample needs all of them; a figure is
    /// never completed with a guess.
    pub fn sample(&self) -> Option<MaterialSample> {
        let sample = MaterialSample {
            name: format!("{} {}", self.maker, self.name).trim().to_owned(),
            dk_points: vec![(self.dk_ghz?, self.dk?)],
            df_points: vec![(self.df_ghz?, self.df?)],
            thickness_nm: None,
        };
        sample.validate().ok()?;
        Some(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identifiers_are_unique_sorted_and_made_of_lowercase_digits_and_hyphens() {
        let ids: Vec<&str> = LAMINATES.iter().map(|l| l.id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted);
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
        for id in ids {
            assert!(
                !id.is_empty()
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "{id}"
            );
        }
    }

    #[test]
    fn nothing_in_the_library_is_verified() {
        assert!(LAMINATES.len() >= 15);
        assert!(LAMINATES.iter().all(|l| !l.verified));
    }

    #[test]
    fn every_figure_is_in_a_plausible_range() {
        for l in LAMINATES {
            let id = l.id;
            for (name, value, low, high) in [
                ("dk", l.dk, 1.0, 100.0),
                ("df", l.df, 0.0, 0.1),
                ("tg", l.tg_c, 20.0, 400.0),
                ("td", l.td_c, 100.0, 600.0),
                ("cte_x", l.cte_x_ppm, 0.0, 100.0),
                ("cte_y", l.cte_y_ppm, 0.0, 100.0),
                ("cte_z", l.cte_z_ppm, 0.0, 300.0),
                ("cte_z_above", l.cte_z_above_tg_ppm, 0.0, 600.0),
                ("thermal", l.thermal_w_per_mk, 0.0, 500.0),
                ("dk_ghz", l.dk_ghz, 0.0001, 200.0),
                ("df_ghz", l.df_ghz, 0.0001, 200.0),
            ] {
                if let Some(v) = value {
                    assert!((low..=high).contains(&v), "{id} {name} {v}");
                }
            }
            // a frequency is stated only with its figure
            assert_eq!(
                l.dk.is_some() && l.dk_ghz.is_some(),
                l.dk_ghz.is_some(),
                "{id}"
            );
            assert_eq!(
                l.df.is_some() && l.df_ghz.is_some(),
                l.df_ghz.is_some(),
                "{id}"
            );
            assert!(!l.tg_is_lower_bound || l.tg_c.is_some(), "{id}");
            assert!(
                l.core_thicknesses_um.windows(2).all(|w| w[0] < w[1]),
                "{id}"
            );
        }
    }

    #[test]
    fn a_lookup_finds_the_entry_in_any_case() {
        let r = find(" Rogers-RO4350B ").unwrap();
        assert_eq!(
            (r.dk, r.df, r.td_c),
            (Some(3.48), Some(0.0037), Some(390.0))
        );
        assert!(r.tg_is_lower_bound);
        assert_eq!(r.core_thicknesses_um.last(), Some(&1524));
        assert!(find("nothing").is_none());
        assert!(find("").is_none());
        let copper = find("copper-foil").unwrap();
        assert_eq!(copper.family, Family::Conductor);
        assert_eq!(copper.conductivity_s_per_m, Some(59_590_000.0));
    }

    #[test]
    fn a_sample_needs_the_constant_the_loss_and_both_frequencies() {
        let s = find("rogers-ro3003").unwrap().sample().unwrap();
        assert_eq!(s.name, "Rogers RO3003");
        assert_eq!(s.dk_points, [(10.0, 3.0)]);
        assert_eq!(s.df_points, [(10.0, 0.0013)]);
        // no frequency, no constant, or no loss: no sample
        for id in [
            "isola-370hr",
            "taconic-tlc",
            "iteq-it-158",
            "arlon-med-85n",
            "copper-foil",
        ] {
            assert!(find(id).unwrap().sample().is_none(), "{id}");
        }
        let with_sample = LAMINATES.iter().filter(|l| l.sample().is_some()).count();
        assert_eq!(with_sample, 10);
    }

    #[test]
    fn the_high_frequency_laminates_are_lossier_in_the_order_the_sheets_state() {
        let df = |id: &str| find(id).unwrap().df.unwrap();
        assert!(df("rogers-ro3003") < df("rogers-ro3006"));
        assert!(df("rogers-ro3006") < df("rogers-ro3010"));
        assert!(df("rogers-ro4003c") < df("rogers-ro4350b"));
        assert!(df("rogers-ro4350b") < df("isola-is400"));
    }
}
