// SPDX-License-Identifier: GPL-3.0-or-later
//! The netlist in memory: elements with their nodes, diode models, subcircuit definitions and
//! parameters. Node names are kept in lower case; the names of the elements keep their case.

use std::collections::BTreeMap;
use std::f64::consts::PI;

use eh3d_phoenix_rf::Complex;

use crate::ac::SweepKind;

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

    /// The times in `(0, stop]` where the waveform has a corner or a jump, in no particular
    /// order and possibly repeated. `None` when there would be more than `limit` of them (a
    /// pulse with a very short period).
    pub fn breakpoints(&self, stop: f64, limit: usize) -> Option<Vec<f64>> {
        let mut out: Vec<f64> = Vec::new();
        let mut push = |t: f64| {
            if t > 0.0 && t <= stop {
                out.push(t);
            }
        };
        match self {
            Wave::None => {}
            Wave::Sin { td, .. } => push(*td),
            Wave::Pulse {
                td,
                tr,
                tf,
                pw,
                per,
                ..
            } => {
                let periods = if *per > 0.0 && stop >= *td {
                    ((stop - td) / per).floor() + 1.0
                } else {
                    1.0
                };
                if periods * 4.0 > limit as f64 {
                    return None;
                }
                for k in 0..(periods as usize) {
                    let base = td + k as f64 * per;
                    push(base);
                    push(base + tr);
                    push(base + tr + pw);
                    push(base + tr + pw + tf);
                }
            }
            Wave::Pwl(points) => {
                if points.len() > limit {
                    return None;
                }
                for (t, _) in points {
                    push(*t);
                }
            }
        }
        Some(out)
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
    /// `Q`: bipolar transistor; the nodes are collector, base and emitter.
    Bjt {
        /// The model, in lower case.
        model: String,
    },
    /// `M`: MOSFET; the nodes are drain, gate, source and bulk (the level 1 model does not use
    /// the bulk).
    Mosfet {
        /// The model, in lower case.
        model: String,
        /// The channel width in metres, when the line gives it.
        w: Option<f64>,
        /// The channel length in metres, when the line gives it.
        l: Option<f64>,
    },
    /// `U`: operational amplifier or comparator; the nodes are the non-inverting input, the
    /// inverting input and the output.
    Amp {
        /// The model, in lower case.
        model: String,
    },
    /// `S`: voltage-controlled switch; the nodes are the two switched nodes and then the
    /// positive and the negative control node.
    Switch {
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

/// A diode model (`.model name D(...)`): the Shockley equation with a series resistance, a
/// junction capacitance, a transit time and a reverse breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct DiodeModel {
    /// The saturation current in ampere.
    pub is: f64,
    /// The emission coefficient.
    pub n: f64,
    /// The series resistance in ohm (zero for none).
    pub rs: f64,
    /// The junction capacitance at zero bias in farad.
    pub cjo: f64,
    /// The junction potential in volt.
    pub vj: f64,
    /// The grading coefficient of the junction.
    pub m: f64,
    /// The coefficient of the forward-bias depletion capacitance, below 1.
    pub fc: f64,
    /// The transit time in second.
    pub tt: f64,
    /// The reverse breakdown voltage (a positive number), when the model has one.
    pub bv: Option<f64>,
    /// The current at the breakdown voltage, in ampere.
    pub ibv: f64,
    /// The emission coefficient of the breakdown.
    pub nbv: f64,
}

impl DiodeModel {
    /// A diode model with a saturation current and an emission coefficient and the defaults of
    /// SPICE for everything else.
    pub fn new(is: f64, n: f64) -> DiodeModel {
        DiodeModel {
            is,
            n,
            rs: 0.0,
            cjo: 0.0,
            vj: 1.0,
            m: 0.5,
            fc: 0.5,
            tt: 0.0,
            bv: None,
            ibv: 1e-3,
            nbv: 1.0,
        }
    }
}

/// A bipolar transistor model (`.model name NPN(...)` or `PNP(...)`): the transport form of
/// Ebers-Moll with the forward Early effect, parasitic resistances, depletion capacitances and
/// transit times.
#[derive(Debug, Clone, PartialEq)]
pub struct BjtModel {
    /// True for a PNP transistor.
    pub pnp: bool,
    /// The transport saturation current in ampere.
    pub is: f64,
    /// The ideal maximum forward current gain.
    pub bf: f64,
    /// The ideal maximum reverse current gain.
    pub br: f64,
    /// The forward emission coefficient.
    pub nf: f64,
    /// The reverse emission coefficient.
    pub nr: f64,
    /// The forward Early voltage, when the model has one.
    pub vaf: Option<f64>,
    /// The base resistance in ohm.
    pub rb: f64,
    /// The collector resistance in ohm.
    pub rc: f64,
    /// The emitter resistance in ohm.
    pub re: f64,
    /// The base-emitter capacitance at zero bias in farad.
    pub cje: f64,
    /// The base-emitter junction potential in volt.
    pub vje: f64,
    /// The base-emitter grading coefficient.
    pub mje: f64,
    /// The base-collector capacitance at zero bias in farad.
    pub cjc: f64,
    /// The base-collector junction potential in volt.
    pub vjc: f64,
    /// The base-collector grading coefficient.
    pub mjc: f64,
    /// The coefficient of the forward-bias depletion capacitances, below 1.
    pub fc: f64,
    /// The forward transit time in second.
    pub tf: f64,
    /// The reverse transit time in second.
    pub tr: f64,
}

impl BjtModel {
    /// A transistor model with the three main parameters and the defaults of SPICE for
    /// everything else.
    pub fn new(pnp: bool, is: f64, bf: f64, br: f64) -> BjtModel {
        BjtModel {
            pnp,
            is,
            bf,
            br,
            nf: 1.0,
            nr: 1.0,
            vaf: None,
            rb: 0.0,
            rc: 0.0,
            re: 0.0,
            cje: 0.0,
            vje: 0.75,
            mje: 0.33,
            cjc: 0.0,
            vjc: 0.75,
            mjc: 0.33,
            fc: 0.5,
            tf: 0.0,
            tr: 0.0,
        }
    }
}

/// A MOSFET model of level 1 (`.model name NMOS(...)` or `PMOS(...)`).
#[derive(Debug, Clone, PartialEq)]
pub struct MosModel {
    /// True for a P channel.
    pub pmos: bool,
    /// The threshold voltage (negative for an enhancement P channel).
    pub vto: f64,
    /// The transconductance parameter in A/V^2.
    pub kp: f64,
    /// The channel-length modulation in 1/V.
    pub lambda: f64,
    /// The width in metres, used when the instance does not give one.
    pub w: f64,
    /// The length in metres, used when the instance does not give one.
    pub l: f64,
    /// The gate-source capacitance in farad (constant).
    pub cgs: f64,
    /// The gate-drain capacitance in farad (constant).
    pub cgd: f64,
}

/// An operational-amplifier macromodel (`.model name OPAMP(...)`): a transconductance into a
/// pole that sets the gain-bandwidth product, a smooth limit at the supply rails and an output
/// resistance.
#[derive(Debug, Clone, PartialEq)]
pub struct OpampModel {
    /// The open-loop DC gain.
    pub gain: f64,
    /// The gain-bandwidth product in hertz (zero for no pole).
    pub gbw: f64,
    /// The positive limit of the output in volt, when the model has one.
    pub vpos: Option<f64>,
    /// The negative limit of the output in volt, when the model has one.
    pub vneg: Option<f64>,
    /// The output resistance in ohm.
    pub rout: f64,
}

/// A simple comparator (`.model name COMP(...)`): the output goes smoothly from the low level to
/// the high level as the input difference crosses zero.
#[derive(Debug, Clone, PartialEq)]
pub struct ComparatorModel {
    /// The output level for a positive difference, in volt.
    pub vhigh: f64,
    /// The output level for a negative difference, in volt.
    pub vlow: f64,
    /// The slope of the transition at zero difference, in V/V.
    pub gain: f64,
    /// The output resistance in ohm.
    pub rout: f64,
}

/// A voltage-controlled switch (`.model name SW(...)`): a resistance that goes from `roff` to
/// `ron` as the control voltage crosses `vt`. The transition is smooth (the logarithm of the
/// conductance follows a hyperbolic tangent over `vt - vh` to `vt + vh`), so the Newton-Raphson
/// iteration has a derivative to follow; there is no hysteresis.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchModel {
    /// The control voltage at which the switch is half on (on a logarithmic scale), in volt.
    pub vt: f64,
    /// The half width of the transition in volt.
    pub vh: f64,
    /// The resistance when on, in ohm.
    pub ron: f64,
    /// The resistance when off, in ohm.
    pub roff: f64,
}

/// A model of any kind.
#[derive(Debug, Clone, PartialEq)]
pub enum Model {
    /// A diode.
    Diode(DiodeModel),
    /// A bipolar transistor.
    Bjt(BjtModel),
    /// A MOSFET.
    Mos(MosModel),
    /// An operational amplifier.
    Opamp(OpampModel),
    /// A comparator.
    Comparator(ComparatorModel),
    /// A voltage-controlled switch.
    Switch(SwitchModel),
}

impl Model {
    /// The type as it is written in a `.model` line.
    pub fn type_name(&self) -> &'static str {
        match self {
            Model::Diode(_) => "D",
            Model::Bjt(m) if m.pnp => "PNP",
            Model::Bjt(_) => "NPN",
            Model::Mos(m) if m.pmos => "PMOS",
            Model::Mos(_) => "NMOS",
            Model::Opamp(_) => "OPAMP",
            Model::Comparator(_) => "COMP",
            Model::Switch(_) => "SW",
        }
    }
}

/// A subcircuit definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Subckt {
    /// The ports, in lower case.
    pub ports: Vec<String>,
    /// The models defined inside it; they hide the models of the netlist with the same name.
    pub models: BTreeMap<String, Model>,
    /// The elements of its body.
    pub elements: Vec<Element>,
}

/// An analysis asked for by a directive of the netlist (`.op`, `.dc`, `.tran`, `.ac`).
#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    /// `.op`: the DC operating point.
    Op,
    /// `.dc source start stop step`: the operating point while one independent source is swept.
    Dc {
        /// The name of the independent source, in lower case.
        source: String,
        /// The first value.
        start: f64,
        /// The last value.
        stop: f64,
        /// The increment (negative to sweep downwards).
        step: f64,
    },
    /// `.tran tstep tstop [uic]`.
    Tran {
        /// The step in seconds.
        tstep: f64,
        /// The end time in seconds.
        tstop: f64,
        /// Start from the initial conditions.
        uic: bool,
    },
    /// `.ac lin|dec|oct points start stop`.
    Ac {
        /// How the frequencies are spaced.
        kind: SweepKind,
        /// The number of points (in all, per decade or per octave).
        points: usize,
        /// The first frequency in hertz.
        start_hz: f64,
        /// The last frequency in hertz.
        stop_hz: f64,
    },
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
    /// The models, by name in lower case.
    pub models: BTreeMap<String, Model>,
    /// The subcircuits, by name in lower case.
    pub subckts: BTreeMap<String, Subckt>,
    /// The elements of the top level.
    pub elements: Vec<Element>,
    /// The analyses the netlist asks for, in the order of its lines.
    pub analyses: Vec<Directive>,
    /// The directives that were read and left out (`.print`, `.options` ...).
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
    fn the_corners_of_a_pulse_and_of_a_piecewise_linear_source_are_breakpoints() {
        let pulse = Wave::Pulse {
            v1: 0.0,
            v2: 1.0,
            td: 1.0,
            tr: 0.5,
            tf: 0.5,
            pw: 2.0,
            per: 10.0,
        };
        let mut t = pulse.breakpoints(12.0, 100).unwrap();
        t.sort_by(f64::total_cmp);
        assert_eq!(t, vec![1.0, 1.5, 3.5, 4.0, 11.0, 11.5]);
        let single = Wave::Pulse {
            v1: 0.0,
            v2: 1.0,
            td: 0.0,
            tr: 0.0,
            tf: 0.0,
            pw: 1.0,
            per: 0.0,
        };
        let mut t = single.breakpoints(5.0, 100).unwrap();
        t.sort_by(f64::total_cmp);
        assert_eq!(t, vec![1.0, 1.0]);
        let pwl = Wave::Pwl(vec![(0.0, 0.0), (1.0, 2.0), (3.0, 2.0), (9.0, 0.0)]);
        assert_eq!(pwl.breakpoints(5.0, 100).unwrap(), vec![1.0, 3.0]);
        assert!(Wave::None.breakpoints(5.0, 100).unwrap().is_empty());
        // a period that is too short for the limit
        let fast = Wave::Pulse {
            v1: 0.0,
            v2: 1.0,
            td: 0.0,
            tr: 0.0,
            tf: 0.0,
            pw: 1e-9,
            per: 2e-9,
        };
        assert!(fast.breakpoints(1.0, 1000).is_none());
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
