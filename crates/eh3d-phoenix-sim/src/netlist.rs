// SPDX-License-Identifier: GPL-3.0-or-later
//! The netlist in memory: elements with their nodes, diode models, subcircuit definitions and
//! parameters. Node names are kept in lower case; the names of the elements keep their case.

use std::collections::BTreeMap;
use std::f64::consts::PI;

use eh3d_phoenix_rf::Complex;

/// The time dependence of an independent source.
#[derive(Debug, Clone, PartialEq)]
pub enum Wave {
    /// Constant: the value is the DC value.
    None,
    /// `SIN(vo va freq td theta phase)`.
    Sin {
        /// The offset.
        vo: f64,
        /// The amplitude.
        va: f64,
        /// The frequency in Hz.
        freq: f64,
        /// The delay in seconds.
        td: f64,
        /// The damping factor in 1/s.
        theta: f64,
        /// The phase in degrees.
        phase_deg: f64,
    },
    /// `PULSE(v1 v2 td tr tf pw per)`.
    Pulse {
        /// The initial value.
        v1: f64,
        /// The pulsed value.
        v2: f64,
        /// The delay.
        td: f64,
        /// The rise time.
        tr: f64,
        /// The fall time.
        tf: f64,
        /// The width of the pulse.
        pw: f64,
        /// The period; zero for a single pulse.
        per: f64,
    },
    /// `PWL(t1 v1 t2 v2 ...)`.
    Pwl(Vec<(f64, f64)>),
}

impl Wave {
    /// The value at a time in seconds.
    pub fn at(&self, t: f64) -> f64 {
        match self {
            Wave::None => 0.0,
            Wave::Sin {
                vo,
                va,
                freq,
                td,
                theta,
                phase_deg,
            } => {
                if t < *td {
                    *vo
                } else {
                    let dt = t - td;
                    vo + va
                        * (-dt * theta).exp()
                        * (2.0 * PI * freq * dt + phase_deg.to_radians()).sin()
                }
            }
            Wave::Pulse {
                v1,
                v2,
                td,
                tr,
                tf,
                pw,
                per,
            } => {
                let mut tt = t - td;
                if tt < 0.0 {
                    return *v1;
                }
                if *per > 0.0 {
                    tt %= per;
                }
                if tt < *tr {
                    v1 + (v2 - v1) * tt / tr
                } else if tt < tr + pw {
                    *v2
                } else if tt < tr + pw + tf {
                    v2 + (v1 - v2) * (tt - tr - pw) / tf
                } else {
                    *v1
                }
            }
            Wave::Pwl(points) => {
                let Some(first) = points.first() else {
                    return 0.0;
                };
                if t <= first.0 {
                    return first.1;
                }
                for pair in points.windows(2) {
                    let (a, b) = (pair[0], pair[1]);
                    if t <= b.0 {
                        if b.0 > a.0 {
                            return a.1 + (b.1 - a.1) * (t - a.0) / (b.0 - a.0);
                        }
                        return b.1;
                    }
                }
                points.last().map_or(0.0, |p| p.1)
            }
        }
    }

    /// The value the waveform starts from, at time zero before any delay.
    pub fn start_value(&self) -> f64 {
        match self {
            Wave::None => 0.0,
            Wave::Sin { vo, .. } => *vo,
            Wave::Pulse { v1, .. } => *v1,
            Wave::Pwl(points) => points.first().map_or(0.0, |p| p.1),
        }
    }
}

/// An independent source (voltage or current).
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    /// The DC value, when the line gives one.
    pub dc: Option<f64>,
    /// The AC magnitude and phase in degrees, when the line gives them.
    pub ac: Option<(f64, f64)>,
    /// The waveform of the transient analysis.
    pub wave: Wave,
}

impl Source {
    /// A source with only a DC value.
    pub fn dc(value: f64) -> Source {
        Source {
            dc: Some(value),
            ac: None,
            wave: Wave::None,
        }
    }

    /// The value of the operating point: the DC value, or else where the waveform starts.
    pub fn dc_value(&self) -> f64 {
        match self.dc {
            Some(v) => v,
            None => self.wave.start_value(),
        }
    }

    /// The value at a time of the transient analysis: the waveform when there is one, else the
    /// DC value.
    pub fn value_at(&self, t: f64) -> f64 {
        if self.wave == Wave::None {
            self.dc.unwrap_or(0.0)
        } else {
            self.wave.at(t)
        }
    }

    /// The phasor of the small-signal analysis; zero when the source has no AC part.
    pub fn ac_phasor(&self) -> Complex {
        match self.ac {
            Some((magnitude, phase_deg)) => Complex::from_polar(magnitude, phase_deg.to_radians()),
            None => Complex::ZERO,
        }
    }
}

/// What an element is.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// `R`: resistance in ohm.
    Resistor(f64),
    /// `C`: capacitance in farad, with an optional initial voltage.
    Capacitor {
        /// The capacitance.
        value: f64,
        /// The initial voltage.
        ic: Option<f64>,
    },
    /// `L`: inductance in henry, with an optional initial current.
    Inductor {
        /// The inductance.
        value: f64,
        /// The initial current.
        ic: Option<f64>,
    },
    /// `V`: independent voltage source.
    Voltage(Source),
    /// `I`: independent current source (the current flows from the first node to the second
    /// through the source).
    Current(Source),
    /// `E`: voltage-controlled voltage source (four nodes).
    Vcvs(f64),
    /// `G`: voltage-controlled current source (four nodes).
    Vccs(f64),
    /// `F`: current-controlled current source.
    Cccs {
        /// The voltage source whose current controls it.
        control: String,
        /// The gain.
        gain: f64,
    },
    /// `H`: current-controlled voltage source.
    Ccvs {
        /// The voltage source whose current controls it.
        control: String,
        /// The transresistance in ohm.
        rm: f64,
    },
    /// `K`: coupling of two inductors (no nodes).
    Mutual {
        /// The first inductor.
        l1: String,
        /// The second inductor.
        l2: String,
        /// The coupling coefficient, above 0 and below 1.
        k: f64,
    },
    /// `D`: diode, anode first.
    Diode {
        /// The model, in lower case.
        model: String,
    },
    /// `X`: an instance of a subcircuit.
    Instance {
        /// The subcircuit, in lower case.
        subckt: String,
    },
}

/// One element of the netlist.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// The name (`R1`).
    pub name: String,
    /// The nodes, in the order of the line.
    pub nodes: Vec<String>,
    /// What it is.
    pub kind: Kind,
}

/// A diode model (`.model name D(IS=.. N=..)`): the Shockley equation.
#[derive(Debug, Clone, PartialEq)]
pub struct DiodeModel {
    /// The saturation current in ampere.
    pub is: f64,
    /// The emission coefficient.
    pub n: f64,
}

/// A subcircuit definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Subckt {
    /// The ports, in lower case.
    pub ports: Vec<String>,
    /// The elements of its body.
    pub elements: Vec<Element>,
}

/// A line of the netlist that was read and left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ignored {
    /// The line.
    pub line: usize,
    /// What was left out.
    pub text: String,
}

/// A whole netlist.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Netlist {
    /// The title (the first line of a SPICE file).
    pub title: String,
    /// The parameters, by name in lower case.
    pub params: BTreeMap<String, f64>,
    /// The diode models, by name in lower case.
    pub models: BTreeMap<String, DiodeModel>,
    /// The subcircuits, by name in lower case.
    pub subckts: BTreeMap<String, Subckt>,
    /// The elements of the top level.
    pub elements: Vec<Element>,
    /// The directives that were read and left out (analyses are asked for by the program).
    pub ignored: Vec<Ignored>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sine_starts_after_its_delay_and_decays() {
        let w = Wave::Sin {
            vo: 1.0,
            va: 2.0,
            freq: 1000.0,
            td: 1e-3,
            theta: 0.0,
            phase_deg: 0.0,
        };
        assert_eq!(w.at(0.0), 1.0);
        assert!((w.at(1e-3 + 0.25e-3) - 3.0).abs() < 1e-12);
        let d = Wave::Sin {
            vo: 0.0,
            va: 1.0,
            freq: 1000.0,
            td: 0.0,
            theta: 1000.0,
            phase_deg: 0.0,
        };
        assert!((d.at(0.25e-3) - (-0.25f64).exp()).abs() < 1e-12);
    }

    #[test]
    fn a_pulse_rises_stays_falls_and_repeats() {
        let w = Wave::Pulse {
            v1: 0.0,
            v2: 5.0,
            td: 1.0,
            tr: 1.0,
            tf: 1.0,
            pw: 2.0,
            per: 10.0,
        };
        assert_eq!(w.at(0.5), 0.0);
        assert_eq!(w.at(1.5), 2.5);
        assert_eq!(w.at(3.0), 5.0);
        assert_eq!(w.at(4.5), 2.5);
        assert_eq!(w.at(7.0), 0.0);
        assert_eq!(w.at(13.0), 5.0);
        let step = Wave::Pulse {
            v1: 0.0,
            v2: 1.0,
            td: 0.0,
            tr: 0.0,
            tf: 0.0,
            pw: 1.0,
            per: 0.0,
        };
        assert_eq!(step.at(0.0), 1.0);
        assert_eq!(step.at(2.0), 0.0);
    }

    #[test]
    fn a_piecewise_linear_source_interpolates_and_holds_its_ends() {
        let w = Wave::Pwl(vec![(0.0, 1.0), (1.0, 3.0), (1.0, 7.0), (2.0, 7.0)]);
        assert_eq!(w.at(-1.0), 1.0);
        assert_eq!(w.at(0.5), 2.0);
        assert_eq!(w.at(1.5), 7.0);
        assert_eq!(w.at(9.0), 7.0);
    }

    #[test]
    fn the_operating_point_uses_the_dc_value_or_the_start_of_the_waveform() {
        let mut s = Source::dc(2.0);
        assert_eq!(s.dc_value(), 2.0);
        assert_eq!(s.value_at(5.0), 2.0);
        s.dc = None;
        s.wave = Wave::Pulse {
            v1: 0.5,
            v2: 1.0,
            td: 0.0,
            tr: 0.0,
            tf: 0.0,
            pw: 1.0,
            per: 0.0,
        };
        assert_eq!(s.dc_value(), 0.5);
        assert_eq!(s.value_at(0.5), 1.0);
        assert_eq!(s.ac_phasor(), Complex::ZERO);
        s.ac = Some((2.0, 90.0));
        assert!(s.ac_phasor().re.abs() < 1e-12);
        assert!((s.ac_phasor().im - 2.0).abs() < 1e-12);
    }
}
