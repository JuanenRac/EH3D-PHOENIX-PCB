// SPDX-License-Identifier: GPL-3.0-or-later
//! Circuits of the textbooks with transistors, diodes with parasitics and op-amps, solved with
//! the public interface of the simulator. The expected values come from the formulas of the
//! books or from a fixed-point iteration written here, not from the code under test.

use eh3d_phoenix_sim::codes::{FLOATING_NODE, MODEL_MISMATCH, SINGULAR_MATRIX, UNDEFINED};
use eh3d_phoenix_sim::output::{ac_csv, op_csv, select, tran_csv};
use eh3d_phoenix_sim::{
    ac_at, operating_point, parse, to_spice, transient, Circuit, Method, OpResult, SimSettings,
    TranRequest,
};

fn circuit(text: &str) -> Circuit {
    let s = SimSettings::default();
    Circuit::compile(&parse(text, &s).unwrap(), &s).unwrap()
}

fn op(text: &str) -> OpResult {
    operating_point(&circuit(text), &SimSettings::default()).unwrap()
}

fn v(r: &OpResult, label: &str) -> f64 {
    r.get(label).unwrap()
}

fn tran(text: &str, tstep: f64, tstop: f64, method: Method, adaptive: bool) -> Vec<(f64, f64)> {
    let req = TranRequest {
        tstep,
        tstop,
        method,
        adaptive,
        uic: false,
    };
    let r = transient(&circuit(text), &SimSettings::default(), &req).unwrap();
    let out = r
        .series("v(out)")
        .unwrap_or_else(|| r.series("v(c)").unwrap());
    r.times.iter().copied().zip(out).collect()
}

/// The operating point of a transistor with a base divider and an emitter resistor, by
/// iterating `Ib = (Vth - Vbe) / (Rth + (BF + 1) RE)` and `Vbe = Vt ln(Ic / IS + 1)`.
fn bias(vcc: f64, r1: f64, r2: f64, re: f64, bf: f64, is: f64) -> (f64, f64, f64) {
    let vt = SimSettings::default().thermal_voltage();
    let vth = vcc * r2 / (r1 + r2);
    let rth = r1 * r2 / (r1 + r2);
    let mut vbe: f64 = 0.65;
    let mut ib: f64 = 0.0;
    for _ in 0..300 {
        ib = (vth - vbe) / (rth + (bf + 1.0) * re);
        vbe = vt * (bf * ib / is + 1.0).ln();
    }
    (bf * ib, ib, vbe)
}

const COMMON_EMITTER: &str = "ce\n.model qn NPN(IS=1e-14 BF=100)\nV1 vcc 0 12\nV2 in 0 DC 0 AC 1\nR1 vcc b 47k\nR2 b 0 10k\nRC vcc c 2.2k\nRE e 0 470\nCE e 0 10m\nC1 in b 10m\nQ1 c b e qn\n";

#[test]
fn the_common_emitter_rests_where_newton_and_the_book_say() {
    let r = op(COMMON_EMITTER);
    let (ic, ib, vbe) = bias(12.0, 47e3, 10e3, 470.0, 100.0, 1e-14);
    let ve = (ic + ib) * 470.0;
    assert!(
        (v(&r, "v(c)") - (12.0 - ic * 2200.0)).abs() < 2e-3,
        "{}",
        v(&r, "v(c)")
    );
    assert!((v(&r, "v(e)") - ve).abs() < 2e-3, "{}", v(&r, "v(e)"));
    assert!(
        (v(&r, "v(b)") - (ve + vbe)).abs() < 2e-3,
        "{}",
        v(&r, "v(b)")
    );
    // a collector current of about 2.5 mA, with the transistor in the active region
    assert!(ic > 2.0e-3 && ic < 3.0e-3, "{ic}");
    assert!(v(&r, "v(c)") - v(&r, "v(e)") > 1.0);
    assert_eq!(r.strategy, "direct");
    assert!(r.iterations > 1);
}

#[test]
fn the_common_emitter_amplifies_by_minus_gm_times_rc() {
    let c = circuit(COMMON_EMITTER);
    let s = SimSettings::default();
    let (ic, _, _) = bias(12.0, 47e3, 10e3, 470.0, 100.0, 1e-14);
    let gm = ic / s.thermal_voltage();
    let r = ac_at(&c, &s, &[1000.0]).unwrap();
    let h = r.series("v(c)").unwrap()[0];
    let want = gm * 2200.0;
    assert!(want > 150.0 && want < 300.0, "{want}");
    assert!((h.abs() - want).abs() < 0.01 * want, "{} {want}", h.abs());
    // the output is turned by half a turn
    assert!((h.arg_deg().abs() - 180.0).abs() < 2.0, "{}", h.arg_deg());
    // the emitter follows the base only a little: it is bypassed
    let e = r.series("v(e)").unwrap()[0];
    assert!(e.abs() < 0.05, "{}", e.abs());
}

#[test]
fn currents_are_conserved_at_the_nodes_of_the_common_emitter() {
    let r = op(COMMON_EMITTER);
    let (vcc, vb, vc, ve) = (v(&r, "v(vcc)"), v(&r, "v(b)"), v(&r, "v(c)"), v(&r, "v(e)"));
    let i_r1 = (vcc - vb) / 47e3;
    let i_r2 = vb / 10e3;
    let i_rc = (vcc - vc) / 2200.0;
    let i_re = ve / 470.0;
    let ib = i_r1 - i_r2;
    // the emitter takes what the collector and the base give
    assert!((i_rc + ib - i_re).abs() < 1e-5, "{} {}", i_rc + ib, i_re);
    // the supply gives what the two resistors take
    assert!((v(&r, "i(v1)") + i_r1 + i_rc).abs() < 1e-9);
    // a base current of a hundredth of the collector current
    assert!((i_rc / ib - 100.0).abs() < 2.0, "{}", i_rc / ib);
}

#[test]
fn a_pnp_circuit_is_the_mirror_image_of_the_npn_one() {
    let npn = op("a\n.model qn NPN(IS=1e-14 BF=100)\nV1 vcc 0 12\nR1 vcc b 47k\nR2 b 0 10k\nRC vcc c 2.2k\nRE e 0 470\nQ1 c b e qn\n");
    let pnp = op("b\n.model qp PNP(IS=1e-14 BF=100)\nV1 vcc 0 12\nR1 b 0 47k\nR2 vcc b 10k\nRC c 0 2.2k\nRE vcc e 470\nQ1 c b e qp\n");
    for node in ["v(b)", "v(c)", "v(e)"] {
        let sum = v(&npn, node) + v(&pnp, node);
        assert!((sum - 12.0).abs() < 3e-3, "{node}: {sum}");
    }
}

#[test]
fn the_early_voltage_makes_the_collector_current_grow_with_the_collector_voltage() {
    let text = |vc: &str| {
        format!("e\n.model qe NPN(IS=1e-14 BF=100 VAF=50)\nV1 b 0 0.65\nV2 c 0 {vc}\nQ1 c b 0 qe\n")
    };
    let low = -v(&op(&text("1")), "i(v2)");
    let high = -v(&op(&text("5")), "i(v2)");
    let want = (1.0 + 4.35 / 50.0) / (1.0 + 0.35 / 50.0);
    assert!((high / low - want).abs() < 1e-5, "{} {want}", high / low);
}

#[test]
fn the_current_mirror_copies_the_reference_current_less_the_base_currents() {
    let r = op("m\n.model qn NPN(IS=1e-14 BF=100)\nV1 vcc 0 10\nR1 vcc b1 9.3k\nQ1 b1 b1 0 qn\nQ2 out b1 0 qn\nV2 out 0 5\n");
    let iref = (v(&r, "v(vcc)") - v(&r, "v(b1)")) / 9300.0;
    let iout = -v(&r, "i(v2)");
    assert!(iref > 0.9e-3 && iref < 1.1e-3, "{iref}");
    // Iout = Iref / (1 + 2 / BF)
    let want = 100.0 / 102.0;
    assert!((iout / iref - want).abs() < 1e-3, "{}", iout / iref);
    assert!((iout / iref - 1.0).abs() < 0.025);
}

#[test]
fn a_model_of_a_subcircuit_hides_the_global_one_of_the_same_name() {
    let text = "m\n.model qm NPN(IS=1e-14 BF=100)\n.subckt mir iref iout\n.model qm NPN(IS=1e-14 BF=10)\nQ1 iref iref 0 qm\nQ2 iout iref 0 qm\n.ends\n.subckt outer a b\nX1 a b mir\n.ends\nV1 vcc 0 10\nR1 vcc a1 9.3k\nQ1 a1 a1 0 qm\nQ2 o1 a1 0 qm\nV2 o1 0 5\nR2 vcc a2 9.3k\nX2 a2 o2 outer\nV3 o2 0 5\n";
    let r = op(text);
    let global = -v(&r, "i(v2)") / ((v(&r, "v(vcc)") - v(&r, "v(a1)")) / 9300.0);
    let local = -v(&r, "i(v3)") / ((v(&r, "v(vcc)") - v(&r, "v(a2)")) / 9300.0);
    assert!((global - 100.0 / 102.0).abs() < 1e-3, "{global}");
    assert!((local - 10.0 / 12.0).abs() < 1e-3, "{local}");
    // the transistors of the nested instance carry the names of both instances
    assert!(circuit(text).bjts.iter().any(|q| q.name == "x2.x1.Q1"));
}

#[test]
fn the_cmos_inverter_switches_where_the_square_law_says() {
    let model =
        ".model nm NMOS(VTO=1 KP=2e-5 LAMBDA=0.02)\n.model pm PMOS(VTO=-1 KP=2e-5 LAMBDA=0.02)\n";
    let out_at = |vin: f64| {
        let text = format!(
            "inv\n{model}V1 vdd 0 5\nV2 in 0 DC {vin}\nM1 out in 0 0 nm W=10u L=1u\nM2 out in vdd vdd pm W=40u L=1u\n"
        );
        v(&op(&text), "v(out)")
    };
    assert!((out_at(0.0) - 5.0).abs() < 1e-3);
    assert!((out_at(0.5) - 5.0).abs() < 1e-3);
    assert!(out_at(4.5).abs() < 1e-3);
    assert!(out_at(5.0).abs() < 1e-3);
    // with a P channel four times as wide, the threshold is where
    // sqrt(bn) (vin - 1) = sqrt(bp) (5 - vin - 1): vin = 3 V
    let (mut lo, mut hi): (f64, f64) = (1.0, 4.0);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if out_at(mid) > 2.5 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let threshold = (lo + hi) / 2.0;
    assert!((threshold - 3.0).abs() < 0.01, "{threshold}");
    // the output falls as the input rises
    assert!(out_at(2.5) > out_at(3.5));
}

#[test]
fn the_non_inverting_amplifier_has_the_gain_one_plus_r2_over_r1() {
    let text = "ni\n.model oa OPAMP(GAIN=1e5 GBW=1e6 VPOS=15 VNEG=-15)\nV1 in 0 1\nU1 in n1 out oa\nR1 n1 0 1k\nR2 out n1 9k\n";
    let r = op(text);
    let out = v(&r, "v(out)");
    assert!((out - 10.0).abs() < 5e-3, "{out}");
    // the inverting input follows the non-inverting one
    assert!((v(&r, "v(n1)") - 1.0).abs() < 1e-3);
    // a gain of 3 with 2k and 1k
    let r = op("ni\n.model oa OPAMP(GAIN=1e5 GBW=1e6 VPOS=15 VNEG=-15)\nV1 in 0 1.5\nU1 in n1 out oa\nR1 n1 0 1k\nR2 out n1 2k\n");
    assert!((v(&r, "v(out)") - 4.5).abs() < 5e-3, "{}", v(&r, "v(out)"));
}

#[test]
fn the_open_loop_op_amp_goes_to_its_rails_and_the_comparator_to_its_levels() {
    let amp = |vin: &str| {
        let text = format!("ol\n.model oa OPAMP(GAIN=1e5 GBW=1e6 VPOS=15 VNEG=-15)\nV1 in 0 {vin}\nU1 in 0 out oa\n");
        v(&op(&text), "v(out)")
    };
    assert!((amp("1m") - 15.0).abs() < 1e-3);
    assert!((amp("-1m") + 15.0).abs() < 1e-3);
    assert!(amp("0").abs() < 1e-9);
    let comp = |vp: &str| {
        let text = format!(
            "cp\n.model cm COMP(VHIGH=5 VLOW=0 GAIN=1e4)\nV1 p 0 {vp}\nV2 n 0 2.5\nU1 p n out cm\n"
        );
        v(&op(&text), "v(out)")
    };
    assert!((comp("2.6") - 5.0).abs() < 1e-6);
    assert!(comp("2.4").abs() < 1e-6);
    assert!((comp("2.5") - 2.5).abs() < 1e-6);
}

#[test]
fn the_half_wave_rectifier_with_a_capacitor_has_the_ripple_of_the_book() {
    let text = "rect\n.model dm D(IS=1e-14 N=1)\nV1 in 0 SIN(0 10 50)\nD1 in out dm\nC1 out 0 100u\nRL out 0 1k\n";
    let samples = tran(text, 10e-6, 0.1, Method::BackwardEuler, false);
    let window: Vec<f64> = samples
        .iter()
        .filter(|(t, _)| (0.06..=0.08).contains(t))
        .map(|(_, y)| *y)
        .collect();
    let peak = window.iter().copied().fold(f64::MIN, f64::max);
    let low = window.iter().copied().fold(f64::MAX, f64::min);
    let ripple = peak - low;
    // the peak is the source peak less a diode drop of 0.6 to 0.75 V
    assert!(peak > 9.1 && peak < 9.5, "{peak}");
    // a linear discharge over a period would give Vp / (f R C) = 1.9 V, the exponential less
    assert!(ripple > 1.2 && ripple < 1.9, "{ripple}");
    assert!(samples.iter().all(|(_, y)| *y > -1e-6));
}

#[test]
fn the_zener_diode_holds_its_voltage_while_the_supply_moves() {
    let out_at = |vin: f64| {
        let text = format!(
            "z\n.model dz D(IS=1e-14 BV=5.1 IBV=1e-3)\nV1 in 0 {vin}\nR1 in out 470\nD1 0 out dz\nRL out 0 2.2k\n"
        );
        v(&op(&text), "v(out)")
    };
    // the fixed point of (Vin - V) / 470 = V / 2200 + IBV exp((V - BV) / Vt)
    let vt = SimSettings::default().thermal_voltage();
    let expected = |vin: f64| {
        let mut vz: f64 = 5.1;
        for _ in 0..200 {
            let iz = (vin - vz) / 470.0 - vz / 2200.0;
            vz = 5.1 + vt * (iz / 1e-3).ln();
        }
        vz
    };
    let (a, b) = (out_at(12.0), out_at(14.0));
    assert!((a - expected(12.0)).abs() < 2e-3, "{a} {}", expected(12.0));
    assert!((b - expected(14.0)).abs() < 2e-3, "{b} {}", expected(14.0));
    assert!(a > 5.1 && a < 5.3, "{a}");
    // line regulation: two volts more at the input move the output a few millivolts
    assert!(b - a > 0.002 && b - a < 0.010, "{}", b - a);
    // below the breakdown the diode does not conduct and the divider rules
    let low = out_at(3.0);
    assert!((low - 3.0 * 2200.0 / 2670.0).abs() < 1e-3, "{low}");
}

#[test]
fn the_series_resistance_of_the_diode_adds_its_drop() {
    let r = op("rs\n.model dr D(IS=1e-14 N=1 RS=10)\nV1 in 0 5\nR1 in d 1k\nD1 d 0 dr\n");
    let vt = SimSettings::default().thermal_voltage();
    let mut total: f64 = 0.7;
    let mut i: f64 = 0.0;
    for _ in 0..200 {
        i = (5.0 - total) / 1000.0;
        total = vt * (i / 1e-14 + 1.0).ln() + 10.0 * i;
    }
    assert!(
        (v(&r, "v(d)") - total).abs() < 1e-3,
        "{} {total}",
        v(&r, "v(d)")
    );
    // the junction is a drop of 10 ohm below the pin
    let inner = v(&r, "v(d1.a)");
    assert!((v(&r, "v(d)") - inner - 10.0 * i).abs() < 1e-3);
    assert!(inner < v(&r, "v(d)"));
}

#[test]
fn the_transistor_switch_saturates_and_comes_back_in_a_transient() {
    let text = "sw\n.model qs NPN(IS=1e-14 BF=100 CJE=2p CJC=1p TF=0.4n)\nV1 vcc 0 5\nV2 in 0 PULSE(0 5 10u 10n 10n 50u 0)\nRB in b 10k\nRC vcc c 1k\nQ1 c b 0 qs\n";
    let samples = tran(text, 1e-6, 100e-6, Method::Trapezoidal, true);
    let at = |time: f64| {
        samples
            .iter()
            .find(|(t, _)| *t >= time)
            .map(|(_, y)| *y)
            .unwrap()
    };
    // off before the pulse, saturated (a few tenths of a volt at most) during it, off again
    assert!(at(5e-6) > 4.99, "{}", at(5e-6));
    let on = at(50e-6);
    assert!(on > 0.0 && on < 0.3, "{on}");
    assert!(at(100e-6) > 4.9, "{}", at(100e-6));
    // the output falls while the input rises
    assert!(at(9e-6) > at(30e-6));
}

#[test]
fn a_node_held_only_by_the_gate_of_a_mosfet_or_the_input_of_an_op_amp_has_no_path_to_ground() {
    let c = circuit("g\n.model nm NMOS\nV1 vdd 0 5\nM1 vdd g 0 0 nm\n");
    let e = operating_point(&c, &SimSettings::default()).unwrap_err();
    assert_eq!(e.code, FLOATING_NODE);
    assert!(e.detail.contains("ground: g"), "{}", e.detail);
    let c = circuit("o\n.model oa OPAMP\nR1 out 0 1k\nU1 a b out oa\n");
    let e = operating_point(&c, &SimSettings::default()).unwrap_err();
    assert_eq!(e.code, FLOATING_NODE);
    assert!(e.detail.contains("a, b"), "{}", e.detail);
}

#[test]
fn the_errors_of_models_and_circuits_have_their_codes() {
    let s = SimSettings::default();
    let e = parse("t\nV1 c 0 5\nQ1 c b 0 nothing\n", &s).unwrap_err();
    assert_eq!((e.code, e.line), (UNDEFINED, Some(3)));
    let e = parse("t\n.model dm D\nQ1 c b 0 dm\n", &s).unwrap_err();
    assert_eq!((e.code, e.line), (MODEL_MISMATCH, Some(3)));
    let e = parse("t\n.model qn NPN\nM1 d g s b qn\n", &s).unwrap_err();
    assert_eq!(e.code, MODEL_MISMATCH);
    let c = circuit("t\nV1 a 0 1\nV2 a 0 2\n");
    assert_eq!(operating_point(&c, &s).unwrap_err().code, SINGULAR_MATRIX);
}

#[test]
fn two_runs_write_the_same_bytes() {
    let c = circuit(COMMON_EMITTER);
    let s = SimSettings::default();
    let run = || {
        let o = operating_point(&c, &s).unwrap();
        let a = ac_at(&c, &s, &[100.0, 1000.0, 10000.0]).unwrap();
        let t = transient(
            &c,
            &s,
            &TranRequest {
                tstep: 1e-4,
                tstop: 1e-2,
                method: Method::Trapezoidal,
                adaptive: true,
                uic: false,
            },
        )
        .unwrap();
        (
            op_csv(&o, &select(&o.labels, &[]).unwrap()),
            ac_csv(&a, &select(&a.labels, &[]).unwrap()),
            tran_csv(&t, &select(&t.labels, &[]).unwrap()),
        )
    };
    assert_eq!(run(), run());
}

#[test]
fn the_netlist_with_transistors_survives_writing_and_reading() {
    let s = SimSettings::default();
    let first = parse(COMMON_EMITTER, &s).unwrap();
    let text = to_spice(&first);
    let second = parse(&text, &s).unwrap();
    assert_eq!(first, second, "{text}");
    assert_eq!(to_spice(&second), text);
}
