// SPDX-License-Identifier: GPL-3.0-or-later
//! A library of typical stack-ups by number of layers, with the thickness of every layer, and the
//! impedance of their lines by the formulas of this crate.
//!
//! The data is in `config/stackup_library.json` (format `phoenix-stack-library/1`): the
//! suggested stack-ups of 6, 8 and 10 layers of a public guideline of a board maker (each as a
//! sequence of `mask`, `cu` (copper), `pp` (prepreg) and `core` layers in micrometres), the
//! common prepregs and core thicknesses, the copper that is left after plating, and the
//! dielectric constants of the public impedance calculator of a maker. **Nothing in it is
//! verified**: the file says `verified: false`, a prepreg is given by its thickness and not by its
//! type because the factory chooses the type, and the loss tangent is not stated by the sources, so
//! it is written as 0. The library gives a first estimate; the stack-up the factory builds is the
//! one it confirms.
//!
//! [`StackLibrary::job`] turns a stack-up of the library into a job for `phoenix stackup`: a
//! microstrip on each outer copper layer and a stripline on each inner one, either with a given
//! width or for a target impedance.

use serde::Deserialize;

use crate::error::StackupError;
use crate::job::StackupJob;
use crate::line::{LineKind, LineSpec};
use crate::material::MaterialSample;
use crate::stackup::{CopperThickness, Layer, Stackup};

/// The data file that comes with the program.
pub const LIBRARY_TEXT: &str = include_str!("../../../config/stackup_library.json");

/// The format the file must declare.
pub const LIBRARY_FORMAT: &str = "phoenix-stack-library/1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMaterial {
    name: String,
    dk: f64,
    df: f64,
    ghz: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStack {
    id: String,
    layers: usize,
    nominal_um: i64,
    source: String,
    sequence: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLibrary {
    format: String,
    verified: bool,
    source: String,
    notes: Vec<String>,
    materials: Vec<RawMaterial>,
    prepregs: Vec<Prepreg>,
    core_thickness_um: Vec<i64>,
    outer_copper: Vec<OuterCopper>,
    stackups: Vec<RawStack>,
}

/// A prepreg that a factory commonly uses.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prepreg {
    /// The style of the glass cloth (`106`, `1080`, `2116`, `7628`).
    #[serde(rename = "type")]
    pub kind: String,
    /// The thickness after pressing, in micrometres.
    pub thickness_um: i64,
    /// The resin content in percent.
    pub resin_percent: u32,
}

/// The copper of an outer layer after plating, by the foil it starts from.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OuterCopper {
    /// The base foil in micrometres.
    pub base_um: i64,
    /// The least thickness after plating for class 2, in tenths of a micrometre.
    pub class2_tenth_um: i64,
    /// The least thickness after plating for class 3, in tenths of a micrometre.
    pub class3_tenth_um: i64,
}

/// A stack-up of the library.
#[derive(Debug, Clone, PartialEq)]
pub struct LibraryStack {
    /// The identifier, like `6l-1.6`.
    pub id: String,
    /// The number of copper layers.
    pub layers: usize,
    /// The thickness the stack-up is sold for, in micrometres.
    pub nominal_um: i64,
    /// The thickness of the layers added up, in nanometres.
    pub total_nm: i64,
    /// Where the stack-up comes from.
    pub source: String,
    /// The layers: `L1` to `Ln` of copper, `PP1`.. and `CORE1`.. between them, and the solder mask
    /// on both outer layers.
    pub stackup: Stackup,
}

/// How a line of the library is given.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Request {
    /// A track of this width in nanometres.
    Width(i64),
    /// The width is found for this impedance in ohm.
    Target(f64),
}

/// The library.
#[derive(Debug, Clone, PartialEq)]
pub struct StackLibrary {
    /// True when the figures were compared with the maker (the file says false).
    pub verified: bool,
    /// Where the figures come from.
    pub source: String,
    /// What the file says the figures leave out.
    pub notes: Vec<String>,
    /// The materials the layers refer to.
    pub materials: Vec<MaterialSample>,
    /// The common prepregs.
    pub prepregs: Vec<Prepreg>,
    /// The thicknesses of the cores in stock, in micrometres.
    pub core_thickness_um: Vec<i64>,
    /// The copper of an outer layer after plating.
    pub outer_copper: Vec<OuterCopper>,
    /// The stack-ups, in the order of the file.
    pub stacks: Vec<LibraryStack>,
}

fn bad(reason: impl Into<String>) -> StackupError {
    StackupError::InvalidJob {
        reason: format!("stack-up library: {}", reason.into()),
    }
}

fn layers_of(raw: &RawStack) -> Result<(Vec<Layer>, usize, i64), StackupError> {
    let mut layers = Vec::new();
    let (mut copper, mut prepreg, mut core, mut masks) = (0usize, 0usize, 0usize, 0usize);
    let mut total_nm: i64 = 0;
    for token in &raw.sequence {
        let (kind, value) = token
            .split_once(':')
            .ok_or_else(|| bad(format!("{}: '{token}' is not kind:micrometres", raw.id)))?;
        let um: i64 = value
            .parse()
            .ok()
            .filter(|v| (1..=100_000).contains(v))
            .ok_or_else(|| {
                bad(format!(
                    "{}: '{token}' has no thickness from 1 to 100000",
                    raw.id
                ))
            })?;
        let nm = um * 1000;
        total_nm += nm;
        match kind {
            "cu" => {
                copper += 1;
                layers.push(Layer::Copper {
                    name: format!("L{copper}"),
                    thickness: CopperThickness::Um(um as f64),
                    roughness_rms_nm: 0,
                });
            }
            "pp" => {
                prepreg += 1;
                layers.push(Layer::Prepreg {
                    name: format!("PP{prepreg}"),
                    thickness_nm: nm,
                    material: "fr4-library".to_owned(),
                });
            }
            "core" => {
                core += 1;
                layers.push(Layer::Core {
                    name: format!("CORE{core}"),
                    thickness_nm: nm,
                    material: "fr4-library".to_owned(),
                });
            }
            "mask" => {
                masks += 1;
                layers.push(Layer::SolderMask {
                    name: if copper == 0 {
                        "MASK-TOP"
                    } else {
                        "MASK-BOTTOM"
                    }
                    .to_owned(),
                    thickness_nm: nm,
                    material: "mask-library".to_owned(),
                });
            }
            other => {
                return Err(bad(format!(
                    "{}: the kind '{other}' is not cu pp core mask",
                    raw.id
                )))
            }
        }
    }
    if masks != 2 {
        return Err(bad(format!(
            "{}: a stack-up has a mask on each outer layer",
            raw.id
        )));
    }
    Ok((layers, copper, total_nm))
}

impl StackLibrary {
    /// Reads the text of a library and checks every stack-up with the checks of the stack-up
    /// model.
    pub fn parse(text: &str) -> Result<StackLibrary, StackupError> {
        let raw: RawLibrary = serde_json::from_str(text).map_err(|e| bad(e.to_string()))?;
        if raw.format != LIBRARY_FORMAT {
            return Err(bad(format!(
                "the format is {}, not {LIBRARY_FORMAT}",
                raw.format
            )));
        }
        let materials: Vec<MaterialSample> = raw
            .materials
            .iter()
            .map(|m| MaterialSample {
                name: m.name.clone(),
                dk_points: vec![(m.ghz, m.dk)],
                df_points: vec![(m.ghz, m.df)],
                thickness_nm: None,
            })
            .collect();
        let mut stacks: Vec<LibraryStack> = Vec::new();
        for r in &raw.stackups {
            if stacks.iter().any(|s| s.id == r.id) {
                return Err(bad(format!("the id {} appears twice", r.id)));
            }
            let (layers, copper, total_nm) = layers_of(r)?;
            if copper != r.layers {
                return Err(bad(format!(
                    "{}: {copper} copper layers in the sequence and {} declared",
                    r.id, r.layers
                )));
            }
            if (total_nm / 1000 - r.nominal_um).abs() > r.nominal_um / 50 {
                return Err(bad(format!(
                    "{}: the layers add up to {} um and the stack-up is sold for {} um",
                    r.id,
                    total_nm / 1000,
                    r.nominal_um
                )));
            }
            let stackup = Stackup {
                name: r.id.clone(),
                layers,
            };
            stackup.validate_with(&materials)?;
            stacks.push(LibraryStack {
                id: r.id.clone(),
                layers: r.layers,
                nominal_um: r.nominal_um,
                total_nm,
                source: r.source.clone(),
                stackup,
            });
        }
        for p in &raw.prepregs {
            if p.thickness_um <= 0 || !(1..=100).contains(&p.resin_percent) {
                return Err(bad(format!(
                    "the prepreg {} has a figure out of range",
                    p.kind
                )));
            }
        }
        if raw.core_thickness_um.windows(2).any(|w| w[0] >= w[1])
            || raw.core_thickness_um.is_empty()
        {
            return Err(bad(
                "the core thicknesses are not listed from the thinnest, each once",
            ));
        }
        for c in &raw.outer_copper {
            if c.base_um <= 0 || c.class2_tenth_um <= 0 || c.class3_tenth_um < c.class2_tenth_um {
                return Err(bad(format!(
                    "the copper of the {} um foil is out of range",
                    c.base_um
                )));
            }
        }
        Ok(StackLibrary {
            verified: raw.verified,
            source: raw.source,
            notes: raw.notes,
            materials,
            prepregs: raw.prepregs,
            core_thickness_um: raw.core_thickness_um,
            outer_copper: raw.outer_copper,
            stacks,
        })
    }

    /// The library that comes with the program.
    pub fn builtin() -> Result<StackLibrary, StackupError> {
        StackLibrary::parse(LIBRARY_TEXT)
    }

    /// A stack-up by its identifier, in any case.
    pub fn find(&self, id: &str) -> Option<&LibraryStack> {
        self.stacks
            .iter()
            .find(|s| s.id.eq_ignore_ascii_case(id.trim()))
    }

    /// The stack-ups with a number of copper layers.
    pub fn with_layers(&self, layers: usize) -> Vec<&LibraryStack> {
        self.stacks.iter().filter(|s| s.layers == layers).collect()
    }

    /// The least copper of an outer layer after plating, in tenths of a micrometre, for a base
    /// foil in micrometres and a class (2 or 3).
    pub fn plated_copper_tenth_um(&self, base_um: i64, class: u8) -> Option<i64> {
        let row = self.outer_copper.iter().find(|c| c.base_um == base_um)?;
        match class {
            2 => Some(row.class2_tenth_um),
            3 => Some(row.class3_tenth_um),
            _ => None,
        }
    }

    /// The job for `phoenix stackup`: a microstrip on each outer copper layer and a stripline on
    /// each inner one (`L1` to `Ln`), with the request given.
    pub fn job(&self, id: &str, request: Request) -> Result<StackupJob, StackupError> {
        let stack = self
            .find(id)
            .ok_or_else(|| bad(format!("no stack-up is called {id}")))?;
        let lines = (1..=stack.layers)
            .map(|k| LineSpec {
                name: format!("L{k}"),
                kind: if k == 1 || k == stack.layers {
                    LineKind::Microstrip
                } else {
                    LineKind::Stripline
                },
                layer: format!("L{k}"),
                width_nm: match request {
                    Request::Width(w) => Some(w),
                    Request::Target(_) => None,
                },
                target_ohm: match request {
                    Request::Width(_) => None,
                    Request::Target(z) => Some(z),
                },
                gap_nm: None,
            })
            .collect();
        Ok(StackupJob {
            stackup: stack.stackup.clone(),
            materials: self.materials.clone(),
            lines,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::StackupSettings;

    fn library() -> StackLibrary {
        StackLibrary::builtin().unwrap()
    }

    #[test]
    fn the_library_that_comes_with_the_program_reads_and_says_it_is_not_verified() {
        let lib = library();
        assert!(!lib.verified);
        assert_eq!(lib.stacks.len(), 8);
        assert!(lib
            .notes
            .iter()
            .any(|n| n.contains("8 layer stackup of 2.00 mm is left out")));
        assert_eq!(lib.materials.len(), 2);
        assert_eq!(lib.prepregs.len(), 4);
        assert_eq!(lib.core_thickness_um.len(), 15);
    }

    #[test]
    fn every_stackup_has_its_layers_in_order_and_the_total_of_the_guideline() {
        let lib = library();
        for (id, layers, total_um) in [
            ("6l-1.6", 6, 1610),
            ("6l-2.0", 6, 2010),
            ("6l-2.4", 6, 2410),
            ("8l-1.6", 8, 1620),
            ("8l-2.4", 8, 2410),
            ("10l-1.6", 10, 1620),
            ("10l-2.0", 10, 2015),
            ("10l-2.4", 10, 2410),
        ] {
            let s = lib.find(id).unwrap();
            assert_eq!(s.layers, layers, "{id}");
            assert_eq!(s.total_nm, total_um * 1000, "{id}");
            assert_eq!(
                s.stackup.total_thickness_nm().unwrap(),
                total_um * 1000,
                "{id}"
            );
            let coppers: Vec<&str> = s
                .stackup
                .layers
                .iter()
                .filter(|l| l.is_copper())
                .map(Layer::name)
                .collect();
            let want: Vec<String> = (1..=layers).map(|k| format!("L{k}")).collect();
            assert_eq!(
                coppers,
                want.iter().map(String::as_str).collect::<Vec<_>>(),
                "{id}"
            );
            // the solder mask is on both outer layers and between every two coppers is a dielectric
            assert!(s.stackup.layers.first().is_some_and(Layer::is_mask), "{id}");
            assert!(s.stackup.layers.last().is_some_and(Layer::is_mask), "{id}");
            assert!(s.stackup.validate_with(&lib.materials).is_ok(), "{id}");
        }
        assert_eq!(lib.with_layers(6).len(), 3);
        assert_eq!(lib.with_layers(8).len(), 2);
        assert_eq!(lib.with_layers(10).len(), 3);
        assert!(lib.with_layers(4).is_empty());
        assert!(lib.find("6L-1.6").is_some());
        assert!(lib.find("4l-1.6").is_none());
    }

    #[test]
    fn the_symmetric_stackups_give_the_same_impedance_on_both_outer_layers() {
        let lib = library();
        let settings = StackupSettings::default();
        for s in &lib.stacks {
            let job = lib.job(&s.id, Request::Width(150_000)).unwrap();
            let out = job.run(1.0, &settings);
            let first = out.first().unwrap().result.as_ref().unwrap().impedance_ohm;
            let last = out.last().unwrap().result.as_ref().unwrap().impedance_ohm;
            assert!((first - last).abs() < 1e-6, "{}: {first} {last}", s.id);
        }
    }

    #[test]
    fn a_width_for_fifty_ohm_is_found_on_every_line_of_every_stackup() {
        let lib = library();
        let settings = StackupSettings::default();
        for s in &lib.stacks {
            let job = lib.job(&s.id, Request::Target(50.0)).unwrap();
            assert_eq!(job.lines.len(), s.layers);
            for outcome in job.run(1.0, &settings) {
                let found = outcome
                    .result
                    .unwrap_or_else(|e| panic!("{} {}: {e}", s.id, outcome.name));
                assert!(outcome.synthesized);
                assert!(
                    (found.impedance_ohm - 50.0).abs() < 0.05,
                    "{} {}",
                    s.id,
                    outcome.name
                );
                assert!(
                    (50_000..=1_000_000).contains(&found.width_nm),
                    "{} {}: {}",
                    s.id,
                    outcome.name,
                    found.width_nm
                );
            }
        }
    }

    #[test]
    fn a_microstrip_over_the_thin_prepreg_is_narrower_than_over_the_thick_one() {
        let lib = library();
        let settings = StackupSettings::default();
        let width = |id: &str| -> i64 {
            let job = lib.job(id, Request::Target(50.0)).unwrap();
            job.run(1.0, &settings)[0].result.as_ref().unwrap().width_nm
        };
        // L1 sits over a prepreg of 0.15 mm (6l-2.0) or of 0.20 mm (6l-1.6 and 6l-2.4)
        assert!(width("6l-2.0") < width("6l-1.6"));
        assert_eq!(width("6l-1.6"), width("6l-2.4"));
        // and the impedance of a track of a fixed width rises with the dielectric under it
        let at = |id: &str| -> f64 {
            let job = lib.job(id, Request::Width(200_000)).unwrap();
            job.run(1.0, &settings)[0]
                .result
                .as_ref()
                .unwrap()
                .impedance_ohm
        };
        assert!(at("6l-2.0") < at("6l-1.6"));
    }

    #[test]
    fn the_numbers_of_a_stackup_are_in_the_range_of_an_fr4_board() {
        let lib = library();
        let settings = StackupSettings::default();
        let job = lib.job("6l-1.6", Request::Width(150_000)).unwrap();
        let out = job.run(1.0, &settings);
        let microstrip = out[0].result.as_ref().unwrap();
        // 0.15 mm over 0.2 mm of FR-4 with a dielectric constant of 4.5
        assert!(
            (60.0..=75.0).contains(&microstrip.impedance_ohm),
            "{}",
            microstrip.impedance_ohm
        );
        assert!(
            (3.0..=3.6).contains(&microstrip.eps_eff),
            "{}",
            microstrip.eps_eff
        );
        let stripline = out[1].result.as_ref().unwrap();
        assert!(
            (35.0..=60.0).contains(&stripline.impedance_ohm),
            "{}",
            stripline.impedance_ohm
        );
        assert!(
            (stripline.eps_eff - 4.5).abs() < 0.01,
            "{}",
            stripline.eps_eff
        );
        // no loss tangent is stated, so no dielectric loss follows
        assert_eq!(microstrip.dielectric_loss_db_per_cm, 0.0);
    }

    #[test]
    fn the_copper_after_plating_and_the_wrong_requests_are_answered() {
        let lib = library();
        assert_eq!(lib.plated_copper_tenth_um(35, 2), Some(479));
        assert_eq!(lib.plated_copper_tenth_um(35, 3), Some(529));
        assert_eq!(lib.plated_copper_tenth_um(18, 2), Some(334));
        assert_eq!(lib.plated_copper_tenth_um(140, 3), Some(1445));
        assert_eq!(lib.plated_copper_tenth_um(36, 2), None);
        assert_eq!(lib.plated_copper_tenth_um(35, 1), None);
        let e = lib.job("nobody", Request::Width(1)).unwrap_err();
        assert!(
            e.to_string().contains("no stack-up is called nobody"),
            "{e}"
        );
    }

    fn edited(from: &str, to: &str) -> Result<StackLibrary, StackupError> {
        assert!(LIBRARY_TEXT.contains(from), "{from}");
        StackLibrary::parse(&LIBRARY_TEXT.replacen(from, to, 1))
    }

    #[test]
    fn a_damaged_library_is_refused_with_what_is_wrong() {
        for (from, to, needle) in [
            (
                "phoenix-stack-library/1",
                "phoenix-stack-library/9",
                "the format is",
            ),
            (
                "\"layers\": 6",
                "\"layers\": 7",
                "copper layers in the sequence",
            ),
            ("\"nominal_um\": 1600", "\"nominal_um\": 1700", "add up to"),
            ("\"cu:35\"", "\"cu:abc\"", "has no thickness"),
            ("\"core:360\"", "\"glass:360\"", "is not cu pp core mask"),
            ("\"core:360\"", "\"core360\"", "is not kind:micrometres"),
            ("\"mask:20\"", "\"cu:20\"", "a mask on each outer layer"),
            ("\"id\": \"6l-2.0\"", "\"id\": \"6l-1.6\"", "appears twice"),
            (
                "\"resin_percent\": 65",
                "\"resin_percent\": 0",
                "out of range",
            ),
            (
                "\"verified\": false",
                "\"verifyed\": false",
                "unknown field",
            ),
            ("[100, 150, 200", "[150, 100, 200", "from the thinnest"),
            (
                "\"class3_tenth_um\": 343",
                "\"class3_tenth_um\": 100",
                "out of range",
            ),
        ] {
            let e = edited(from, to).unwrap_err();
            assert!(e.to_string().contains(needle), "{from} -> {to}: {e}");
        }
        assert!(StackLibrary::parse("{").is_err());
    }
}
