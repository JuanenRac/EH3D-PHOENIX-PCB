// SPDX-License-Identifier: GPL-3.0-or-later
//! The reader of SPICE-style netlists.
//!
//! The first line is the title. Comments start with `*` (a whole line) or `;` (the rest of a
//! line); a line that starts with `+` continues the one before. Supported: `R C L V I E G F H K
//! D Q M U X`, `.param`, `.model` (diodes, bipolar and MOS transistors, op-amps and comparators;
//! a model inside a `.subckt` belongs to it), `.subckt` / `.ends` and `.end`. Other directives (`.op`,
//! `.ac`, `.tran`, `.print` ...) are noted in [`Netlist::ignored`]: the program asks for the
//! analyses. Every error names its line.

use std::collections::{BTreeMap, BTreeSet};

use crate::codes::{
    SimError, DUPLICATE_NAME, LIMIT_EXCEEDED, MODEL_MISMATCH, NETLIST_SYNTAX, SUBCKT_INVALID,
    UNDEFINED, UNSUPPORTED, VALUE_INVALID,
};
use crate::expr::{eval, ExprError};
use crate::netlist::{
    BjtModel, ComparatorModel, DiodeModel, Element, Ignored, Kind, Model, MosModel, Netlist,
    OpampModel, Source, Subckt, Wave,
};
use crate::settings::SimSettings;
use crate::units::parse_number;

struct Env<'a> {
    params: &'a BTreeMap<String, f64>,
    max_depth: usize,
}

/// Splits a logical line into tokens. Spaces, parentheses and commas separate; `=` is a token of
/// its own; a group between braces is one token.
fn tokenize(line: &str, number: usize) -> Result<Vec<String>, SimError> {
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() || c == '(' || c == ')' || c == ',' {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            i += 1;
        } else if c == '=' {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            out.push("=".to_string());
            i += 1;
        } else if c == '{' {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            let mut depth = 0usize;
            let mut j = i;
            let mut closed = false;
            while j < chars.len() {
                if chars[j] == '{' {
                    depth += 1;
                } else if chars[j] == '}' {
                    depth -= 1;
                    if depth == 0 {
                        closed = true;
                        break;
                    }
                }
                j += 1;
            }
            if !closed {
                return Err(SimError::at(
                    NETLIST_SYNTAX,
                    number,
                    "a brace is not closed".to_string(),
                ));
            }
            out.push(chars[i..=j].iter().collect());
            i = j + 1;
        } else {
            current.push(c);
            i += 1;
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    Ok(out)
}

/// The value of a token: a number with a suffix, or an expression between braces.
fn value(token: &str, number: usize, env: &Env) -> Result<f64, SimError> {
    if let Some(inner) = token.strip_prefix('{') {
        let inner = inner.strip_suffix('}').unwrap_or(inner);
        return match eval(inner, env.params, env.max_depth) {
            Ok(v) => Ok(v),
            Err(ExprError::Syntax(why)) => Err(SimError::at(
                VALUE_INVALID,
                number,
                format!("{{{inner}}}: {why}"),
            )),
            Err(ExprError::Undefined(name)) => Err(SimError::at(
                UNDEFINED,
                number,
                format!("parameter {name} is not defined"),
            )),
        };
    }
    parse_number(token)
        .ok_or_else(|| SimError::at(VALUE_INVALID, number, format!("'{token}' is not a number")))
}

fn node(token: &str, number: usize) -> Result<String, SimError> {
    if token.is_empty() || token.starts_with('{') || token == "=" {
        return Err(SimError::at(
            NETLIST_SYNTAX,
            number,
            format!("'{token}' is not a node name"),
        ));
    }
    Ok(token.to_lowercase())
}

fn is_keyword(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "dc" | "ac" | "sin" | "pulse" | "pwl"
    )
}

fn parse_source(
    name: &str,
    tokens: &[String],
    number: usize,
    env: &Env,
) -> Result<Source, SimError> {
    let syntax = |why: &str| SimError::at(NETLIST_SYNTAX, number, format!("{name}: {why}"));
    let invalid = |why: &str| SimError::at(VALUE_INVALID, number, format!("{name}: {why}"));
    let mut source = Source {
        dc: None,
        ac: None,
        wave: Wave::None,
    };
    let mut i = 0;
    while i < tokens.len() {
        let key = tokens[i].to_ascii_lowercase();
        match key.as_str() {
            "dc" => {
                let token = tokens
                    .get(i + 1)
                    .ok_or_else(|| syntax("DC needs a value"))?;
                source.dc = Some(value(token, number, env)?);
                i += 2;
            }
            "ac" => {
                let token = tokens
                    .get(i + 1)
                    .ok_or_else(|| syntax("AC needs a value"))?;
                let magnitude = value(token, number, env)?;
                i += 2;
                let mut phase = 0.0;
                if let Some(next) = tokens.get(i) {
                    if !is_keyword(next) {
                        phase = value(next, number, env)?;
                        i += 1;
                    }
                }
                source.ac = Some((magnitude, phase));
            }
            "sin" | "pulse" | "pwl" => {
                i += 1;
                let mut args: Vec<f64> = Vec::new();
                while i < tokens.len() && !is_keyword(&tokens[i]) {
                    args.push(value(&tokens[i], number, env)?);
                    i += 1;
                }
                source.wave = match key.as_str() {
                    "sin" => {
                        if !(3..=6).contains(&args.len()) {
                            return Err(syntax(
                                "SIN needs 3 to 6 values: vo va freq td theta phase",
                            ));
                        }
                        let get = |k: usize| args.get(k).copied().unwrap_or(0.0);
                        if args[2] <= 0.0 {
                            return Err(invalid("the frequency of SIN must be above zero"));
                        }
                        Wave::Sin {
                            vo: args[0],
                            va: args[1],
                            freq: args[2],
                            td: get(3),
                            theta: get(4),
                            phase_deg: get(5),
                        }
                    }
                    "pulse" => {
                        if args.len() != 7 {
                            return Err(syntax("PULSE needs 7 values: v1 v2 td tr tf pw per"));
                        }
                        let (td, tr, tf, pw, per) = (args[2], args[3], args[4], args[5], args[6]);
                        if td < 0.0 || tr < 0.0 || tf < 0.0 || pw < 0.0 || per < 0.0 {
                            return Err(invalid("the times of PULSE cannot be negative"));
                        }
                        if per > 0.0 && tr + pw + tf > per {
                            return Err(invalid("the period of PULSE is shorter than its pulse"));
                        }
                        Wave::Pulse {
                            v1: args[0],
                            v2: args[1],
                            td,
                            tr,
                            tf,
                            pw,
                            per,
                        }
                    }
                    _ => {
                        if args.len() < 2 || !args.len().is_multiple_of(2) {
                            return Err(syntax("PWL needs pairs of time and value"));
                        }
                        let points: Vec<(f64, f64)> =
                            args.chunks(2).map(|p| (p[0], p[1])).collect();
                        if points.windows(2).any(|w| w[1].0 < w[0].0) {
                            return Err(invalid("the times of PWL must not decrease"));
                        }
                        Wave::Pwl(points)
                    }
                };
            }
            _ => {
                if i == 0 {
                    source.dc = Some(value(&tokens[0], number, env)?);
                    i += 1;
                } else {
                    return Err(syntax(&format!("'{}' is not understood", tokens[i])));
                }
            }
        }
    }
    if source.dc.is_none() && source.ac.is_none() && source.wave == Wave::None {
        return Err(syntax("a source needs a value"));
    }
    Ok(source)
}

fn parse_ic(
    name: &str,
    rest: &[String],
    number: usize,
    env: &Env,
) -> Result<Option<f64>, SimError> {
    if rest.is_empty() {
        return Ok(None);
    }
    if rest.len() == 3 && rest[0].eq_ignore_ascii_case("ic") && rest[1] == "=" {
        return Ok(Some(value(&rest[2], number, env)?));
    }
    Err(SimError::at(
        NETLIST_SYNTAX,
        number,
        format!("{name}: only ic = value may follow the value"),
    ))
}

/// Reads one element line (already split into tokens).
fn parse_element(tokens: &[String], number: usize, env: &Env) -> Result<Element, SimError> {
    let name = tokens[0].clone();
    let letter = name.chars().next().map_or(' ', |c| c.to_ascii_lowercase());
    let syntax = |why: &str| SimError::at(NETLIST_SYNTAX, number, format!("{name}: {why}"));
    let invalid = |why: &str| SimError::at(VALUE_INVALID, number, format!("{name}: {why}"));
    let make = |nodes: Vec<String>, kind: Kind| Element {
        name: name.clone(),
        nodes,
        kind,
    };
    match letter {
        'r' | 'c' | 'l' => {
            if tokens.len() < 4 {
                return Err(syntax("needs two nodes and a value"));
            }
            let nodes = vec![node(&tokens[1], number)?, node(&tokens[2], number)?];
            let v = value(&tokens[3], number, env)?;
            let ic = parse_ic(&name, &tokens[4..], number, env)?;
            match letter {
                'r' => {
                    if tokens.len() > 4 {
                        return Err(syntax("a resistor takes no more than its value"));
                    }
                    if v <= 0.0 {
                        return Err(invalid("the resistance must be above zero"));
                    }
                    Ok(make(nodes, Kind::Resistor(v)))
                }
                'c' => {
                    if v < 0.0 {
                        return Err(invalid("the capacitance cannot be negative"));
                    }
                    Ok(make(nodes, Kind::Capacitor { value: v, ic }))
                }
                _ => {
                    if v <= 0.0 {
                        return Err(invalid("the inductance must be above zero"));
                    }
                    Ok(make(nodes, Kind::Inductor { value: v, ic }))
                }
            }
        }
        'v' | 'i' => {
            if tokens.len() < 4 {
                return Err(syntax("needs two nodes and a value"));
            }
            let nodes = vec![node(&tokens[1], number)?, node(&tokens[2], number)?];
            let source = parse_source(&name, &tokens[3..], number, env)?;
            if letter == 'v' {
                Ok(make(nodes, Kind::Voltage(source)))
            } else {
                Ok(make(nodes, Kind::Current(source)))
            }
        }
        'e' | 'g' => {
            if tokens.len() != 6 {
                return Err(syntax("needs four nodes and a gain"));
            }
            let mut nodes = Vec::new();
            for token in &tokens[1..5] {
                nodes.push(node(token, number)?);
            }
            let gain = value(&tokens[5], number, env)?;
            if letter == 'e' {
                Ok(make(nodes, Kind::Vcvs(gain)))
            } else {
                Ok(make(nodes, Kind::Vccs(gain)))
            }
        }
        'f' | 'h' => {
            if tokens.len() != 5 {
                return Err(syntax("needs two nodes, a voltage source and a gain"));
            }
            let nodes = vec![node(&tokens[1], number)?, node(&tokens[2], number)?];
            let control = tokens[3].clone();
            let gain = value(&tokens[4], number, env)?;
            if letter == 'f' {
                Ok(make(nodes, Kind::Cccs { control, gain }))
            } else {
                Ok(make(nodes, Kind::Ccvs { control, rm: gain }))
            }
        }
        'k' => {
            if tokens.len() != 4 {
                return Err(syntax("needs two inductors and a coefficient"));
            }
            let k = value(&tokens[3], number, env)?;
            if k <= 0.0 || k >= 1.0 {
                return Err(invalid(
                    "the coupling coefficient must be above 0 and below 1",
                ));
            }
            Ok(make(
                Vec::new(),
                Kind::Mutual {
                    l1: tokens[1].clone(),
                    l2: tokens[2].clone(),
                    k,
                },
            ))
        }
        'd' => {
            if tokens.len() != 4 {
                return Err(syntax("needs an anode, a cathode and a model"));
            }
            let nodes = vec![node(&tokens[1], number)?, node(&tokens[2], number)?];
            Ok(make(
                nodes,
                Kind::Diode {
                    model: tokens[3].to_lowercase(),
                },
            ))
        }
        'q' => {
            if tokens.len() == 6 {
                return Err(SimError::at(
                    UNSUPPORTED,
                    number,
                    format!("{name}: a fourth node (the substrate) is not supported"),
                ));
            }
            if tokens.len() != 5 {
                return Err(syntax("needs a collector, a base, an emitter and a model"));
            }
            let mut nodes = Vec::new();
            for token in &tokens[1..4] {
                nodes.push(node(token, number)?);
            }
            Ok(make(
                nodes,
                Kind::Bjt {
                    model: tokens[4].to_lowercase(),
                },
            ))
        }
        'm' => {
            if tokens.len() < 6 {
                return Err(syntax(
                    "needs a drain, a gate, a source, a bulk and a model",
                ));
            }
            let mut nodes = Vec::new();
            for token in &tokens[1..5] {
                nodes.push(node(token, number)?);
            }
            let rest = &tokens[6..];
            if !rest.len().is_multiple_of(3) {
                return Err(syntax("W and L are written name = value"));
            }
            let (mut w, mut l) = (None, None);
            for triple in rest.chunks(3) {
                if triple[1] != "=" {
                    return Err(syntax("W and L are written name = value"));
                }
                let v = value(&triple[2], number, env)?;
                match triple[0].to_ascii_lowercase().as_str() {
                    "w" => w = Some(v),
                    "l" => l = Some(v),
                    other => return Err(syntax(&format!("'{other}' is not understood"))),
                }
            }
            if w.is_some_and(|v| !positive(v)) || l.is_some_and(|v| !positive(v)) {
                return Err(invalid("W and L must be above zero"));
            }
            Ok(make(
                nodes,
                Kind::Mosfet {
                    model: tokens[5].to_lowercase(),
                    w,
                    l,
                },
            ))
        }
        'u' => {
            if tokens.len() != 5 {
                return Err(syntax(
                    "needs a non-inverting input, an inverting input, an output and a model",
                ));
            }
            let mut nodes = Vec::new();
            for token in &tokens[1..4] {
                nodes.push(node(token, number)?);
            }
            Ok(make(
                nodes,
                Kind::Amp {
                    model: tokens[4].to_lowercase(),
                },
            ))
        }
        'x' => {
            if tokens.len() < 3 {
                return Err(syntax("needs nodes and the name of a subcircuit"));
            }
            let last = tokens.len() - 1;
            let mut nodes = Vec::new();
            for token in &tokens[1..last] {
                nodes.push(node(token, number)?);
            }
            Ok(make(
                nodes,
                Kind::Instance {
                    subckt: tokens[last].to_lowercase(),
                },
            ))
        }
        other => Err(SimError::at(
            UNSUPPORTED,
            number,
            format!("{name}: elements of type '{other}' are not supported"),
        )),
    }
}

fn positive(v: f64) -> bool {
    v.is_finite() && v > 0.0
}

fn not_negative(v: f64) -> bool {
    v.is_finite() && v >= 0.0
}

/// Reads the `name = value` pairs of a `.model` line, the names in lower case.
fn model_pairs(rest: &[String], number: usize, env: &Env) -> Result<Vec<(String, f64)>, SimError> {
    let shape = || {
        SimError::at(
            NETLIST_SYNTAX,
            number,
            "model parameters are written name = value".to_string(),
        )
    };
    if !rest.len().is_multiple_of(3) {
        return Err(shape());
    }
    let mut out = Vec::new();
    for triple in rest.chunks(3) {
        if triple[1] != "=" {
            return Err(shape());
        }
        out.push((
            triple[0].to_ascii_lowercase(),
            value(&triple[2], number, env)?,
        ));
    }
    Ok(out)
}

/// Builds a model of the type named on a `.model` line. A parameter that is not known is noted in
/// `ignored`, as SPICE programs do.
fn build_model(
    kind: &str,
    pairs: &[(String, f64)],
    settings: &SimSettings,
    number: usize,
    ignored: &mut Vec<Ignored>,
) -> Result<Model, SimError> {
    let bad = |why: &str| SimError::at(VALUE_INVALID, number, why.to_string());
    let mut note = |key: &str| {
        ignored.push(Ignored {
            line: number,
            text: format!("model parameter {key}"),
        })
    };
    match kind {
        "d" => {
            let mut m = DiodeModel::new(settings.diode_is, settings.diode_n);
            m.ibv = settings.diode_ibv;
            for (key, v) in pairs {
                let v = *v;
                match key.as_str() {
                    "is" => m.is = v,
                    "n" => m.n = v,
                    "rs" => m.rs = v,
                    "cjo" | "cj0" => m.cjo = v,
                    "vj" => m.vj = v,
                    "m" => m.m = v,
                    "fc" => m.fc = v,
                    "tt" => m.tt = v,
                    "bv" => m.bv = Some(v),
                    "ibv" => m.ibv = v,
                    "nbv" => m.nbv = v,
                    other => note(other),
                }
            }
            if !positive(m.is) || !positive(m.n) {
                return Err(bad("IS and N of a diode must be above zero"));
            }
            if !not_negative(m.rs) || !not_negative(m.cjo) || !not_negative(m.tt) {
                return Err(bad("RS, CJO and TT of a diode cannot be negative"));
            }
            if !positive(m.vj) || !not_negative(m.m) || !(0.0..1.0).contains(&m.fc) {
                return Err(bad(
                    "VJ of a diode must be above zero, M cannot be negative and FC must be in [0, 1)",
                ));
            }
            if m.bv.is_some_and(|b| !positive(b)) || !positive(m.ibv) || !positive(m.nbv) {
                return Err(bad("BV, IBV and NBV of a diode must be above zero"));
            }
            Ok(Model::Diode(m))
        }
        "npn" | "pnp" => {
            let mut m = BjtModel::new(
                kind == "pnp",
                settings.bjt_is,
                settings.bjt_bf,
                settings.bjt_br,
            );
            for (key, v) in pairs {
                let v = *v;
                match key.as_str() {
                    "is" => m.is = v,
                    "bf" => m.bf = v,
                    "br" => m.br = v,
                    "nf" => m.nf = v,
                    "nr" => m.nr = v,
                    "vaf" | "va" => m.vaf = Some(v),
                    "rb" => m.rb = v,
                    "rc" => m.rc = v,
                    "re" => m.re = v,
                    "cje" => m.cje = v,
                    "vje" => m.vje = v,
                    "mje" => m.mje = v,
                    "cjc" => m.cjc = v,
                    "vjc" => m.vjc = v,
                    "mjc" => m.mjc = v,
                    "fc" => m.fc = v,
                    "tf" => m.tf = v,
                    "tr" => m.tr = v,
                    other => note(other),
                }
            }
            let strict = [m.is, m.bf, m.br, m.nf, m.nr, m.vje, m.vjc];
            if strict.iter().any(|v| !positive(*v)) || m.vaf.is_some_and(|v| !positive(v)) {
                return Err(bad(
                    "IS, BF, BR, NF, NR, VJE, VJC and VAF of a transistor must be above zero",
                ));
            }
            let loose = [m.rb, m.rc, m.re, m.cje, m.cjc, m.mje, m.mjc, m.tf, m.tr];
            if loose.iter().any(|v| !not_negative(*v)) || !(0.0..1.0).contains(&m.fc) {
                return Err(bad(
                    "RB, RC, RE, CJE, CJC, MJE, MJC, TF and TR of a transistor cannot be negative and FC must be in [0, 1)",
                ));
            }
            Ok(Model::Bjt(m))
        }
        "nmos" | "pmos" => {
            let mut m = MosModel {
                pmos: kind == "pmos",
                vto: settings.mos_vto,
                kp: settings.mos_kp,
                lambda: settings.mos_lambda,
                w: settings.mos_w,
                l: settings.mos_l,
                cgs: 0.0,
                cgd: 0.0,
            };
            for (key, v) in pairs {
                let v = *v;
                match key.as_str() {
                    "vto" | "vt0" => m.vto = v,
                    "kp" => m.kp = v,
                    "lambda" => m.lambda = v,
                    "w" => m.w = v,
                    "l" => m.l = v,
                    "cgs" => m.cgs = v,
                    "cgd" => m.cgd = v,
                    other => note(other),
                }
            }
            if !m.vto.is_finite() || !positive(m.kp) || !positive(m.w) || !positive(m.l) {
                return Err(bad(
                    "VTO of a MOSFET must be a number and KP, W and L must be above zero",
                ));
            }
            if !not_negative(m.lambda) || !not_negative(m.cgs) || !not_negative(m.cgd) {
                return Err(bad("LAMBDA, CGS and CGD of a MOSFET cannot be negative"));
            }
            Ok(Model::Mos(m))
        }
        "opamp" => {
            let mut m = OpampModel {
                gain: settings.opamp_gain,
                gbw: settings.opamp_gbw,
                vpos: None,
                vneg: None,
                rout: settings.opamp_rout,
            };
            for (key, v) in pairs {
                let v = *v;
                match key.as_str() {
                    "gain" | "aol" => m.gain = v,
                    "gbw" => m.gbw = v,
                    "vpos" => m.vpos = Some(v),
                    "vneg" => m.vneg = Some(v),
                    "rout" => m.rout = v,
                    other => note(other),
                }
            }
            if !positive(m.gain) || !not_negative(m.gbw) || !positive(m.rout) {
                return Err(bad(
                    "GAIN and ROUT of an op-amp must be above zero and GBW cannot be negative",
                ));
            }
            match (m.vpos, m.vneg) {
                (None, None) => {}
                (Some(p), Some(n)) if p.is_finite() && n.is_finite() && p > n => {}
                _ => {
                    return Err(bad(
                        "VPOS and VNEG of an op-amp are given together and VPOS must be above VNEG",
                    ))
                }
            }
            Ok(Model::Opamp(m))
        }
        "comp" => {
            let mut m = ComparatorModel {
                vhigh: settings.comparator_high,
                vlow: settings.comparator_low,
                gain: settings.comparator_gain,
                rout: settings.opamp_rout,
            };
            for (key, v) in pairs {
                let v = *v;
                match key.as_str() {
                    "vhigh" => m.vhigh = v,
                    "vlow" => m.vlow = v,
                    "gain" => m.gain = v,
                    "rout" => m.rout = v,
                    other => note(other),
                }
            }
            if !m.vhigh.is_finite()
                || !m.vlow.is_finite()
                || m.vhigh <= m.vlow
                || !positive(m.gain)
                || !positive(m.rout)
            {
                return Err(bad(
                    "VHIGH of a comparator must be above VLOW and GAIN and ROUT must be above zero",
                ));
            }
            Ok(Model::Comparator(m))
        }
        other => Err(SimError::at(
            UNSUPPORTED,
            number,
            format!(
                "model type '{other}' is not supported (D, NPN, PNP, NMOS, PMOS, OPAMP and COMP are)"
            ),
        )),
    }
}

/// Reads one element line from text (used by the export of a schematic). The parameters are the
/// ones already defined.
pub(crate) fn parse_element_line(
    text: &str,
    number: usize,
    params: &BTreeMap<String, f64>,
    settings: &SimSettings,
) -> Result<Element, SimError> {
    let tokens = tokenize(text, number)?;
    if tokens.is_empty() {
        return Err(SimError::at(
            NETLIST_SYNTAX,
            number,
            "the line is empty".to_string(),
        ));
    }
    let env = Env {
        params,
        max_depth: settings.max_expr_depth,
    };
    parse_element(&tokens, number, &env)
}

/// An element with the line it was read from.
type Placed = (Element, usize);

/// The error for a model that is not there or that is of another kind.
fn check_model(
    element: &Element,
    model: &str,
    found: Option<&Model>,
    letter: char,
    number: usize,
) -> Result<(), SimError> {
    let Some(found) = found else {
        return Err(SimError::at(
            UNDEFINED,
            number,
            format!("{}: the model {model} is not defined", element.name),
        ));
    };
    let fits = matches!(
        (letter, found),
        ('d', Model::Diode(_))
            | ('q', Model::Bjt(_))
            | ('m', Model::Mos(_))
            | ('u', Model::Opamp(_))
            | ('u', Model::Comparator(_))
    );
    if fits {
        Ok(())
    } else {
        Err(SimError::at(
            MODEL_MISMATCH,
            number,
            format!(
                "{}: the model {model} is a {} and does not fit this element",
                element.name,
                found.type_name()
            ),
        ))
    }
}

/// Checks the references of the elements of one scope (the top level or one subcircuit). The
/// models of a subcircuit hide the ones of the netlist.
fn check_scope(
    elements: &[Placed],
    models: &BTreeMap<String, Model>,
    local: Option<&BTreeMap<String, Model>>,
    subckts: &BTreeMap<String, Subckt>,
) -> Result<(), SimError> {
    let mut kinds: BTreeMap<String, &Kind> = BTreeMap::new();
    for (element, _) in elements {
        kinds.insert(element.name.to_lowercase(), &element.kind);
    }
    let find = |name: &str| local.and_then(|l| l.get(name)).or_else(|| models.get(name));
    for (element, number) in elements {
        let number = *number;
        match &element.kind {
            Kind::Diode { model } => check_model(element, model, find(model), 'd', number)?,
            Kind::Bjt { model } => check_model(element, model, find(model), 'q', number)?,
            Kind::Mosfet { model, .. } => check_model(element, model, find(model), 'm', number)?,
            Kind::Amp { model } => check_model(element, model, find(model), 'u', number)?,
            Kind::Instance { subckt } => {
                let Some(sub) = subckts.get(subckt) else {
                    return Err(SimError::at(
                        UNDEFINED,
                        number,
                        format!("{}: the subcircuit {subckt} is not defined", element.name),
                    ));
                };
                if sub.ports.len() != element.nodes.len() {
                    return Err(SimError::at(
                        SUBCKT_INVALID,
                        number,
                        format!(
                            "{}: {subckt} has {} ports and the instance gives {}",
                            element.name,
                            sub.ports.len(),
                            element.nodes.len()
                        ),
                    ));
                }
            }
            Kind::Mutual { l1, l2, .. } => {
                if l1.eq_ignore_ascii_case(l2) {
                    return Err(SimError::at(
                        SUBCKT_INVALID,
                        number,
                        format!(
                            "{}: an inductor cannot be coupled with itself",
                            element.name
                        ),
                    ));
                }
                for l in [l1, l2] {
                    if !matches!(kinds.get(&l.to_lowercase()), Some(Kind::Inductor { .. })) {
                        return Err(SimError::at(
                            UNDEFINED,
                            number,
                            format!("{}: {l} is not an inductor of this circuit", element.name),
                        ));
                    }
                }
            }
            Kind::Cccs { control, .. } | Kind::Ccvs { control, .. } => {
                if !matches!(kinds.get(&control.to_lowercase()), Some(Kind::Voltage(_))) {
                    return Err(SimError::at(
                        UNDEFINED,
                        number,
                        format!(
                            "{}: {control} is not a voltage source of this circuit",
                            element.name
                        ),
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// A subcircuit being read.
struct Building {
    name: String,
    ports: Vec<String>,
    elements: Vec<Placed>,
    names: BTreeSet<String>,
    models: BTreeMap<String, Model>,
    line: usize,
}

/// Reads a netlist.
pub fn parse(text: &str, settings: &SimSettings) -> Result<Netlist, SimError> {
    if text.len() as u64 > settings.max_file_bytes {
        return Err(SimError::new(
            LIMIT_EXCEEDED,
            format!(
                "the netlist is larger than max_file_bytes ({})",
                settings.max_file_bytes
            ),
        ));
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut title = String::new();
    let mut logical: Vec<(usize, String)> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        if number > settings.max_lines {
            return Err(SimError::at(
                LIMIT_EXCEEDED,
                number,
                format!("more than max_lines ({}) lines", settings.max_lines),
            ));
        }
        if index == 0 {
            title = raw.trim().to_string();
            continue;
        }
        let body = raw.split(';').next().unwrap_or("").trim();
        if body.is_empty() || body.starts_with('*') {
            continue;
        }
        if let Some(rest) = body.strip_prefix('+') {
            match logical.last_mut() {
                Some(last) => {
                    last.1.push(' ');
                    last.1.push_str(rest.trim());
                }
                None => {
                    return Err(SimError::at(
                        NETLIST_SYNTAX,
                        number,
                        "a continuation line has nothing to continue".to_string(),
                    ))
                }
            }
        } else {
            logical.push((number, body.to_string()));
        }
    }

    let mut params: BTreeMap<String, f64> = BTreeMap::new();
    let mut models: BTreeMap<String, Model> = BTreeMap::new();
    let mut subckts: BTreeMap<String, Subckt> = BTreeMap::new();
    let mut sub_lines: BTreeMap<String, Vec<Placed>> = BTreeMap::new();
    let mut top: Vec<Placed> = Vec::new();
    let mut top_names: BTreeSet<String> = BTreeSet::new();
    let mut building: Option<Building> = None;
    let mut ignored: Vec<Ignored> = Vec::new();
    let mut element_count = 0usize;

    for (number, line) in &logical {
        let number = *number;
        let tokens = tokenize(line, number)?;
        if tokens.is_empty() {
            continue;
        }
        let first = tokens[0].to_ascii_lowercase();
        if first.starts_with('.') {
            match first.as_str() {
                ".end" => break,
                ".param" => {
                    let rest = &tokens[1..];
                    if rest.is_empty() || !rest.len().is_multiple_of(3) {
                        return Err(SimError::at(
                            NETLIST_SYNTAX,
                            number,
                            ".param needs name = value".to_string(),
                        ));
                    }
                    for triple in rest.chunks(3) {
                        if triple[1] != "=" || !triple[0].starts_with(|c: char| c.is_alphabetic()) {
                            return Err(SimError::at(
                                NETLIST_SYNTAX,
                                number,
                                ".param needs name = value".to_string(),
                            ));
                        }
                        let key = triple[0].to_lowercase();
                        if params.contains_key(&key) {
                            return Err(SimError::at(
                                DUPLICATE_NAME,
                                number,
                                format!("parameter {key} is defined twice"),
                            ));
                        }
                        let env = Env {
                            params: &params,
                            max_depth: settings.max_expr_depth,
                        };
                        let v = value(&triple[2], number, &env)?;
                        params.insert(key, v);
                    }
                }
                ".model" => {
                    if tokens.len() < 3 {
                        return Err(SimError::at(
                            NETLIST_SYNTAX,
                            number,
                            ".model needs a name and a type".to_string(),
                        ));
                    }
                    let key = tokens[1].to_lowercase();
                    let taken = match building.as_ref() {
                        Some(b) => b.models.contains_key(&key),
                        None => models.contains_key(&key),
                    };
                    if taken {
                        return Err(SimError::at(
                            DUPLICATE_NAME,
                            number,
                            format!("model {key} is defined twice"),
                        ));
                    }
                    let env = Env {
                        params: &params,
                        max_depth: settings.max_expr_depth,
                    };
                    let pairs = model_pairs(&tokens[3..], number, &env)?;
                    let model = build_model(
                        &tokens[2].to_ascii_lowercase(),
                        &pairs,
                        settings,
                        number,
                        &mut ignored,
                    )?;
                    match building.as_mut() {
                        Some(b) => b.models.insert(key, model),
                        None => models.insert(key, model),
                    };
                }
                ".subckt" => {
                    if building.is_some() {
                        return Err(SimError::at(
                            SUBCKT_INVALID,
                            number,
                            "a subcircuit cannot be defined inside another".to_string(),
                        ));
                    }
                    if tokens.len() < 3 {
                        return Err(SimError::at(
                            SUBCKT_INVALID,
                            number,
                            ".subckt needs a name and at least one port".to_string(),
                        ));
                    }
                    if tokens[2..]
                        .iter()
                        .any(|t| t.to_ascii_lowercase().starts_with("params"))
                    {
                        return Err(SimError::at(
                            UNSUPPORTED,
                            number,
                            "parameters of a subcircuit are not supported".to_string(),
                        ));
                    }
                    let name = tokens[1].to_lowercase();
                    if subckts.contains_key(&name) {
                        return Err(SimError::at(
                            DUPLICATE_NAME,
                            number,
                            format!("subcircuit {name} is defined twice"),
                        ));
                    }
                    let mut ports: Vec<String> = Vec::new();
                    for token in &tokens[2..] {
                        let port = node(token, number)?;
                        if settings.is_netlist_ground(&port) || ports.contains(&port) {
                            return Err(SimError::at(
                                SUBCKT_INVALID,
                                number,
                                format!("port {port} is ground or is listed twice"),
                            ));
                        }
                        ports.push(port);
                    }
                    building = Some(Building {
                        name,
                        ports,
                        elements: Vec::new(),
                        names: BTreeSet::new(),
                        models: BTreeMap::new(),
                        line: number,
                    });
                }
                ".ends" => {
                    let Some(done) = building.take() else {
                        return Err(SimError::at(
                            SUBCKT_INVALID,
                            number,
                            ".ends without .subckt".to_string(),
                        ));
                    };
                    if let Some(named) = tokens.get(1) {
                        if named.to_lowercase() != done.name {
                            return Err(SimError::at(
                                SUBCKT_INVALID,
                                number,
                                format!(".ends {named} closes {}", done.name),
                            ));
                        }
                    }
                    subckts.insert(
                        done.name.clone(),
                        Subckt {
                            ports: done.ports,
                            models: done.models,
                            elements: done.elements.iter().map(|(e, _)| e.clone()).collect(),
                        },
                    );
                    sub_lines.insert(done.name, done.elements);
                }
                other => ignored.push(Ignored {
                    line: number,
                    text: other.to_string(),
                }),
            }
            continue;
        }

        let env = Env {
            params: &params,
            max_depth: settings.max_expr_depth,
        };
        let element = parse_element(&tokens, number, &env)?;
        element_count += 1;
        if element_count > settings.max_elements {
            return Err(SimError::at(
                LIMIT_EXCEEDED,
                number,
                format!(
                    "more than max_elements ({}) elements",
                    settings.max_elements
                ),
            ));
        }
        let key = element.name.to_lowercase();
        let (names, list) = match building.as_mut() {
            Some(b) => (&mut b.names, &mut b.elements),
            None => (&mut top_names, &mut top),
        };
        if !names.insert(key) {
            return Err(SimError::at(
                DUPLICATE_NAME,
                number,
                format!("the element {} is defined twice", element.name),
            ));
        }
        list.push((element, number));
    }

    if let Some(open) = building {
        return Err(SimError::at(
            SUBCKT_INVALID,
            open.line,
            format!("the subcircuit {} is not closed with .ends", open.name),
        ));
    }
    check_scope(&top, &models, None, &subckts)?;
    for (name, placed) in &sub_lines {
        let local = subckts.get(name).map(|sub| &sub.models);
        check_scope(placed, &models, local, &subckts)?;
    }
    Ok(Netlist {
        title,
        params,
        models,
        subckts,
        elements: top.into_iter().map(|(e, _)| e).collect(),
        ignored,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<Netlist, SimError> {
        parse(text, &SimSettings::default())
    }

    #[test]
    fn a_small_netlist_is_read_with_units_comments_and_continuations() {
        let nl = read(
            "divider\n* a comment\nV1 in 0 DC 10 ; inline\nR1 in Out 1k\n+ \nR2 out 0\n+ 3k\n.op\n.end\nR9 x 0 1\n",
        )
        .unwrap();
        assert_eq!(nl.title, "divider");
        assert_eq!(nl.elements.len(), 3);
        assert_eq!(
            nl.elements[1].nodes,
            vec!["in".to_string(), "out".to_string()]
        );
        assert_eq!(nl.elements[1].kind, Kind::Resistor(1000.0));
        assert_eq!(nl.elements[2].kind, Kind::Resistor(3000.0));
        assert_eq!(nl.ignored.len(), 1);
        assert_eq!(nl.ignored[0].text, ".op");
        match &nl.elements[0].kind {
            Kind::Voltage(s) => assert_eq!(s.dc, Some(10.0)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parameters_and_expressions_are_resolved() {
        let nl = read("t\n.param rr=2k cc={1/(2*pi*rr*1k)}\nR1 a 0 {rr*2}\nC1 a 0 {cc} ic={0.5}\n")
            .unwrap();
        assert_eq!(nl.params["rr"], 2000.0);
        assert_eq!(nl.elements[0].kind, Kind::Resistor(4000.0));
        match &nl.elements[1].kind {
            Kind::Capacitor { value, ic } => {
                assert!(
                    (value - 1.0 / (2.0 * std::f64::consts::PI * 2000.0 * 1000.0)).abs() < 1e-20
                );
                assert_eq!(*ic, Some(0.5));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn sources_take_dc_ac_and_waveforms() {
        let nl = read(
            "t\nV1 a 0 5\nV2 b 0 DC 1 AC 2 30\nV3 c 0 SIN(0 1 1k)\nV4 d 0 PULSE(0 5 1u 1n 1n 5u 10u)\nV5 e 0 PWL(0 0 1m 1)\nI1 f 0 AC 1\n",
        )
        .unwrap();
        let source = |i: usize| match &nl.elements[i].kind {
            Kind::Voltage(s) | Kind::Current(s) => s.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(source(0).dc, Some(5.0));
        assert_eq!(source(1).ac, Some((2.0, 30.0)));
        assert!(matches!(source(2).wave, Wave::Sin { freq, .. } if freq == 1000.0));
        assert!(matches!(source(3).wave, Wave::Pulse { per, .. } if (per - 1e-5).abs() < 1e-18));
        assert_eq!(source(4).wave, Wave::Pwl(vec![(0.0, 0.0), (1e-3, 1.0)]));
        assert_eq!(source(5).ac, Some((1.0, 0.0)));
    }

    #[test]
    fn models_subcircuits_and_dependent_sources_are_read() {
        let nl = read(
            "t\n.model dm D(IS=2e-14 N=1.5 RS=1 FOO=3)\n.subckt half a b\nR1 a m 1k\nR2 m b 1k\n.ends half\nX1 in out half\nD1 in out DM\nE1 o 0 in 0 2\nG1 o 0 in 0 1m\nV1 in 0 1\nF1 o 0 V1 3\nH1 p 0 v1 100\nL1 a 0 1m\nL2 b 0 4m\nK1 L1 l2 0.99\n",
        )
        .unwrap();
        match &nl.models["dm"] {
            Model::Diode(m) => {
                assert_eq!((m.is, m.n, m.rs), (2e-14, 1.5, 1.0));
                assert_eq!(m.bv, None);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            nl.subckts["half"].ports,
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(nl.subckts["half"].elements.len(), 2);
        assert_eq!(nl.elements.len(), 10);
        assert!(nl.ignored.iter().any(|i| i.text.contains("foo")));
        assert!(!nl.ignored.iter().any(|i| i.text.contains("rs")));
    }

    fn fails(text: &str, code: &str, line: usize) {
        let e = read(text).unwrap_err();
        assert_eq!(e.code, code, "{text}: {e}");
        assert_eq!(e.line, Some(line), "{text}: {e}");
    }

    #[test]
    fn errors_have_a_code_and_the_line() {
        fails("t\nR1 a 0\n", NETLIST_SYNTAX, 2);
        fails("t\n\nR1 a 0 x\n", VALUE_INVALID, 3);
        fails("t\nR1 a 0 0\n", VALUE_INVALID, 2);
        fails("t\nZ1 a b c\n", UNSUPPORTED, 2);
        fails("t\nQ1 a b c npn\n", UNDEFINED, 2);
        fails("t\nQ1 a b c d npn\n", UNSUPPORTED, 2);
        fails("t\nQ1 a b npn\n", NETLIST_SYNTAX, 2);
        fails("t\nM1 a b c npn\n", NETLIST_SYNTAX, 2);
        fails("t\nU1 a b out\n", NETLIST_SYNTAX, 2);
        fails("t\n.model m1 D\nQ1 a b c m1\n", MODEL_MISMATCH, 3);
        fails("t\n.model m1 NPN\nD1 a b m1\n", MODEL_MISMATCH, 3);
        fails("t\n.model m1 NMOS\nU1 a b c m1\n", MODEL_MISMATCH, 3);
        fails("t\nR1 a 0 1\nr1 b 0 1\n", DUPLICATE_NAME, 3);
        fails("t\nD1 a 0 nomodel\n", UNDEFINED, 2);
        fails("t\nR1 a 0 {zz}\n", UNDEFINED, 2);
        fails("t\nR1 a 0 {1+}\n", VALUE_INVALID, 2);
        fails("t\nR1 a 0 {1\n", NETLIST_SYNTAX, 2);
        fails("t\n.subckt s a\nR1 a 0 1\n", SUBCKT_INVALID, 2);
        fails("t\n.ends\n", SUBCKT_INVALID, 2);
        fails("t\nX1 a b nothing\n", UNDEFINED, 2);
        fails(
            "t\n.subckt s a b\nR1 a b 1\n.ends\nX1 a s\n",
            SUBCKT_INVALID,
            5,
        );
        fails("t\nL1 a 0 1\nK1 L1 L2 0.5\n", UNDEFINED, 3);
        fails("t\nL1 a 0 1\nL2 b 0 1\nK1 L1 L2 1\n", VALUE_INVALID, 4);
        fails("t\nF1 a 0 V9 2\n", UNDEFINED, 2);
        fails("t\nV1 a 0 SIN(0 1)\n", NETLIST_SYNTAX, 2);
        fails("t\nV1 a 0 PWL(0 0 1)\n", NETLIST_SYNTAX, 2);
        fails("t\n.model m TRIAC\n", UNSUPPORTED, 2);
        fails("t\n.model m NPN(BF=0)\n", VALUE_INVALID, 2);
        fails("t\n.model m D(CJO=-1p)\n", VALUE_INVALID, 2);
        fails("t\n.model m D(FC=1)\n", VALUE_INVALID, 2);
        fails("t\n.model m NMOS(KP=0)\n", VALUE_INVALID, 2);
        fails("t\n.model m OPAMP(VPOS=5)\n", VALUE_INVALID, 2);
        fails("t\n.model m COMP(VHIGH=0 VLOW=1)\n", VALUE_INVALID, 2);
        fails("t\n.model m NPN(IS)\n", NETLIST_SYNTAX, 2);
        fails("t\n.model m D\n.model M D\n", DUPLICATE_NAME, 3);
        fails("t\nM1 a b c d nm W=0\n.model nm NMOS\n", VALUE_INVALID, 2);
        fails("t\n+ R1 a 0 1\n", NETLIST_SYNTAX, 2);
        fails("t\n.param a=1 a=2\n", DUPLICATE_NAME, 2);
        fails("t\nC1 a 0 1u foo\n", NETLIST_SYNTAX, 2);
    }

    #[test]
    fn transistors_amplifiers_and_their_models_are_read() {
        let nl = read(
            "t\n.model q1m NPN(IS=2e-15 BF=150 VAF=80 RB=10 CJE=1p TF=0.4n)\n.model q2m PNP\n.model nm NMOS(VTO=0.7 KP=1e-4 LAMBDA=0.05 CGS=2f)\n.model pm PMOS(VTO=-0.7)\n.model oa OPAMP(GAIN=2e5 GBW=2meg VPOS=12 VNEG=-12)\n.model cm COMP(VHIGH=3.3)\nQ1 c b e q1m\nQ2 c2 b e q2m\nM1 d g s s nm W=10u L=1u\nM2 d2 g s s pm\nU1 p n o oa\nU2 p n o2 cm\n",
        )
        .unwrap();
        match &nl.models["q1m"] {
            Model::Bjt(m) => {
                assert!(!m.pnp);
                assert_eq!((m.is, m.bf, m.br), (2e-15, 150.0, 1.0));
                assert_eq!(m.vaf, Some(80.0));
                assert_eq!((m.rb, m.cje), (10.0, 1e-12));
                assert!((m.tf - 0.4e-9).abs() < 1e-21);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(nl.models["q2m"].type_name(), "PNP");
        match &nl.models["nm"] {
            Model::Mos(m) => assert_eq!((m.vto, m.kp, m.lambda, m.cgs), (0.7, 1e-4, 0.05, 2e-15)),
            other => panic!("{other:?}"),
        }
        match &nl.models["oa"] {
            Model::Opamp(m) => {
                assert_eq!((m.gain, m.gbw), (2e5, 2e6));
                assert_eq!((m.vpos, m.vneg), (Some(12.0), Some(-12.0)));
            }
            other => panic!("{other:?}"),
        }
        match &nl.models["cm"] {
            Model::Comparator(m) => assert_eq!((m.vhigh, m.vlow), (3.3, 0.0)),
            other => panic!("{other:?}"),
        }
        assert_eq!(nl.elements.len(), 6);
        assert!(matches!(
            &nl.elements[2].kind,
            Kind::Mosfet { model, w: Some(w), l: Some(l) }
                if model == "nm" && (*w - 1e-5).abs() < 1e-18 && (*l - 1e-6).abs() < 1e-18
        ));
        assert!(matches!(
            &nl.elements[3].kind,
            Kind::Mosfet {
                w: None,
                l: None,
                ..
            }
        ));
        assert_eq!(nl.elements[2].nodes.len(), 4);
        assert_eq!(nl.elements[0].nodes, vec!["c", "b", "e"]);
    }

    #[test]
    fn a_model_inside_a_subcircuit_belongs_to_it_and_hides_the_global_one() {
        let nl = read(
            "t\n.model dm D(IS=1e-14)\n.subckt s a b\n.model dm D(IS=5e-15)\nD1 a b dm\n.ends\nD2 x y dm\nX1 x y s\n",
        )
        .unwrap();
        match (&nl.models["dm"], &nl.subckts["s"].models["dm"]) {
            (Model::Diode(g), Model::Diode(l)) => assert_eq!((g.is, l.is), (1e-14, 5e-15)),
            other => panic!("{other:?}"),
        }
        // the local model is not visible outside
        let e =
            read("t\n.subckt s a b\n.model only D\nD1 a b only\n.ends\nD2 x y only\n").unwrap_err();
        assert_eq!(e.code, UNDEFINED);
        assert_eq!(e.line, Some(6));
        // a model defined twice inside the same subcircuit
        let e = read("t\n.subckt s a b\n.model m D\n.model m D\nR1 a b 1\n.ends\n").unwrap_err();
        assert_eq!(e.code, DUPLICATE_NAME);
    }

    #[test]
    fn limits_of_the_settings_are_enforced() {
        let s =
            SimSettings::parse("max_elements = 2\nmax_lines = 4\nmax_file_bytes = 40\n").unwrap();
        assert_eq!(
            parse("t\nR1 a 0 1\nR2 a 0 1\nR3 a 0 1\n", &s)
                .unwrap_err()
                .code,
            LIMIT_EXCEEDED
        );
        let long = "t\n* x\n* x\n* x\n* x\nR1 a 0 1\n";
        assert_eq!(parse(long, &s).unwrap_err().code, LIMIT_EXCEEDED);
        let big = format!("t\n* {}\n", "x".repeat(100));
        assert_eq!(parse(&big, &s).unwrap_err().code, LIMIT_EXCEEDED);
    }

    #[test]
    fn garbage_is_an_error_and_never_a_panic() {
        let good = "t\nV1 a 0 SIN(0 1 1k)\n.model m D(IS=1e-14)\nD1 a b m\nR1 b 0 {2*3}\n.subckt s p q\nR1 p q 1\n.ends\n";
        let chars: Vec<char> = good.chars().collect();
        for cut in 0..chars.len() {
            let piece: String = chars[..cut].iter().collect();
            let _ = read(&piece);
        }
        for junk in [
            "{{{{", "}}}}", "((((", "=====", "\u{0}", ".param", ".subckt", "X", "+",
        ] {
            let _ = read(&format!("t\n{junk}\n"));
        }
    }
}
