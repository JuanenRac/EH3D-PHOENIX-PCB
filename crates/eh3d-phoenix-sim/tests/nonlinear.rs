// SPDX-License-Identifier: GPL-3.0-or-later
//! Non-linear circuits solved with the public interface of the simulator: a diode with a series
//! resistor against the Lambert W function, an LED, a Zener regulator, a half-wave rectifier, a
//! MOSFET inverter against its theoretical transfer table, an inverting amplifier, a
//! voltage-controlled switch, the sweep of a source and the analyses a netlist asks for. The
//! expected values come from the formulas written in each test, evaluated here, not from the code
//! under test. The tolerances allow for the stop rule of Newton-Raphson (a change below `reltol`
//! times the value plus `vntol`, twice in a row), which leaves an error of about 1e-5 V on a
//! junction voltage.

use eh3d_phoenix_sim::output::{dc_csv, select};
use eh3d_phoenix_sim::{
    ac_at, dc_sweep, operating_point, parse, transient, Circuit, DcRequest, Directive, Method,
    SimSettings, SweepKind, TranRequest,
};

fn settings() -> SimSettings {
    SimSettings::default()
}

fn circuit(text: &str) -> Circuit {
    let s = settings();
    Circuit::compile(&parse(text, &s).unwrap(), &s).unwrap()
}

fn op(text: &str, label: &str) -> f64 {
    operating_point(&circuit(text), &settings())
        .unwrap()
        .get(label)
        .unwrap()
}

/// The principal branch of the Lambert W function for `x = exp(l)`. Above `l = 1` it is found by
/// Newton iteration on `ln w + w = l` (the logarithm avoids the overflow of `x` itself), below
/// it by Newton iteration on `w exp(w) = x` from the right of the root.
fn lambert_w_of_exp(l: f64) -> f64 {
    if l > 1.0 {
        let mut w = l - l.ln();
        for _ in 0..100 {
            let f = w.ln() + w - l;
            w -= f / (1.0 / w + 1.0);
        }
        w
    } else {
        let x = l.exp();
        let mut w = x;
        for _ in 0..100 {
            let e = w.exp();
            w -= (w * e - x) / (e * (w + 1.0));
        }
        w
    }
}

/// The voltage across a diode of saturation current `is` and emission coefficient `n` in series
/// with `r` on `vs`: `I = (n Vt / R) W((Is R / (n Vt)) exp((Vs + Is R) / (n Vt))) - Is`, and the
/// diode voltage is `Vs - I R`. Returns the voltage and the current.
fn diode_in_series(vs: f64, r: f64, is: f64, n: f64) -> (f64, f64) {
    let nvt = n * settings().thermal_voltage();
    let l = (is * r / nvt).ln() + (vs + is * r) / nvt;
    let current = nvt / r * lambert_w_of_exp(l) - is;
    (vs - current * r, current)
}

#[test]
fn the_lambert_w_of_the_test_solves_its_own_equation() {
    for l in [-25.0, -5.0, 0.0, 0.5, 2.0, 10.0, 50.0, 190.0] {
        let w = lambert_w_of_exp(l);
        // w exp(w) = exp(l), in the logarithm
        assert!((w.ln() + w - l).abs() < 1e-9, "{l}");
    }
    // W(e) = 1 is the classic value, and W(0) = 0 in the limit
    assert!((lambert_w_of_exp(1.0) - 1.0).abs() < 1e-12);
    assert!(lambert_w_of_exp(-700.0) < 1e-300);
}

#[test]
fn a_diode_with_a_series_resistor_follows_the_lambert_w_solution() {
    for (vs, r, is, n) in [
        (5.0, 1000.0, 1e-14, 1.0),
        (5.0, 1000.0, 2.52e-9, 1.752),
        (3.3, 330.0, 1e-14, 1.0),
        (12.0, 10.0, 1e-12, 1.2),
        (1.0, 100.0, 1e-14, 1.0),
    ] {
        let text =
            format!("t\n.model dm D(IS={is:e} N={n})\nV1 in 0 DC {vs}\nR1 in d {r}\nD1 d 0 dm\n");
        let result = operating_point(&circuit(&text), &settings()).unwrap();
        let (want, current) = diode_in_series(vs, r, is, n);
        let got = result.get("v(d)").unwrap();
        assert!((got - want).abs() < 1e-4, "{vs} {r}: {got} {want}");
        // the current of the source is the current of the diode, with the sign of SPICE
        let supplied = -result.get("i(v1)").unwrap();
        assert!((supplied - current).abs() < 1e-4 * current + 1e-9);
        assert!(result.iterations > 1);
        assert!(
            result.residual < 1e-2 * current + 1e-9,
            "{}",
            result.residual
        );
    }
}

#[test]
fn a_hard_diode_circuit_still_converges_by_one_of_the_aids() {
    // 100 V across 1 ohm into a diode: the first Newton steps are far from the answer
    let text = "t\n.model dm D(IS=1e-14 N=1)\nV1 in 0 DC 100\nR1 in d 1\nD1 d 0 dm\n";
    let result = operating_point(&circuit(text), &settings()).unwrap();
    let (want, _) = diode_in_series(100.0, 1.0, 1e-14, 1.0);
    assert!((result.get("v(d)").unwrap() - want).abs() < 2e-4);
    assert!(["direct", "gmin-stepping", "source-stepping"].contains(&result.strategy));
}

#[test]
fn an_led_takes_the_defaults_of_the_settings_unless_the_line_gives_its_own() {
    let s = settings();
    let default = op(
        "t\n.model l1 LED\nV1 in 0 5\nR1 in d 330\nD1 d 0 l1\n",
        "v(d)",
    );
    let (want, _) = diode_in_series(5.0, 330.0, s.led_is, s.led_n);
    assert!((default - want).abs() < 1e-4, "{default} {want}");
    // a red LED drops about 1.8 V at 10 mA
    assert!(default > 1.7 && default < 1.9, "{default}");
    let own = op(
        "t\n.model l2 LED(IS=1e-20 N=2)\nV1 in 0 5\nR1 in d 330\nD1 d 0 l2\n",
        "v(d)",
    );
    let (want_own, _) = diode_in_series(5.0, 330.0, 1e-20, 2.0);
    assert!((own - want_own).abs() < 1e-4, "{own} {want_own}");
    assert!(own > default + 0.1);
}

#[test]
fn a_zener_diode_regulates_at_its_breakdown_voltage() {
    // the resistor current falls and the breakdown current rises with the Zener voltage
    // (Vz = -v of the diode): the balance is found by bisection
    let vt = settings().thermal_voltage();
    let (vs, r, bv, ibv) = (12.0, 470.0, 5.1, 1e-3);
    let (mut low, mut high) = (0.0f64, vs);
    for _ in 0..200 {
        let vz = 0.5 * (low + high);
        let balance = (vs - vz) / r - ibv * ((vz - bv) / vt).exp();
        if balance > 0.0 {
            low = vz;
        } else {
            high = vz;
        }
    }
    let want = 0.5 * (low + high);
    let got = op(
        "t\n.model zm ZENER(BV=5.1 IBV=1m)\nV1 in 0 12\nR1 in k 470\nD1 0 k zm\n",
        "v(k)",
    );
    assert!((got - want).abs() < 1e-4, "{got} {want}");
    assert!(got > 5.1 && got < 5.3, "{got}");
    // forward, the same part is an ordinary diode
    let forward = op(
        "t\n.model zm ZENER(BV=5.1 IBV=1m)\nV1 in 0 12\nR1 in k 470\nD1 k 0 zm\n",
        "v(k)",
    );
    assert!(forward > 0.5 && forward < 0.8, "{forward}");
}

/// The load voltage of a half-wave rectifier with the diode in series with `rl`, at the source
/// voltage `vs`: zero while the source is negative.
fn rectifier(vs: f64, rl: f64, is: f64) -> f64 {
    if vs <= 0.0 {
        return 0.0;
    }
    let (_, current) = diode_in_series(vs, rl, is, 1.0);
    current * rl
}

#[test]
fn a_half_wave_rectifier_follows_the_static_diode_curve_at_every_step() {
    let text = "t\n.model dm D(IS=1e-14 N=1)\nV1 in 0 SIN(0 5 1k)\nD1 in out dm\nRL out 0 2.2k\n";
    let c = circuit(text);
    let request = TranRequest {
        tstep: 1e-5,
        tstop: 2e-3,
        method: Method::Trapezoidal,
        adaptive: false,
        uic: false,
    };
    let result = transient(&c, &settings(), &request).unwrap();
    let source = result.series("v(in)").unwrap();
    let out = result.series("v(out)").unwrap();
    assert!(result.times.len() > 150);
    let mut peak = 0.0f64;
    for (vs, got) in source.iter().zip(out.iter()) {
        let want = rectifier(*vs, 2200.0, 1e-14);
        assert!((got - want).abs() < 2e-4, "{vs}: {got} {want}");
        peak = peak.max(*got);
    }
    // the peak is the source peak less one diode drop (0.67 V on 2.2 kohm)
    let (drop, _) = diode_in_series(5.0, 2200.0, 1e-14, 1.0);
    assert!((peak - (5.0 - drop)).abs() < 2e-3, "{peak} {drop}");
    // while the source is negative nothing flows
    assert!(out
        .iter()
        .zip(source.iter())
        .all(|(o, s)| *s > 0.0 || o.abs() < 1e-6));
}

/// The output of a resistive NMOS inverter (`beta` in A/V^2, load `rd`, supply `vdd`, threshold
/// `vt`, no channel-length modulation) at an input voltage: cut-off, saturation or triode.
fn inverter(vin: f64, vdd: f64, rd: f64, beta: f64, vt: f64) -> f64 {
    let vov = vin - vt;
    if vov <= 0.0 {
        return vdd;
    }
    let saturated = vdd - rd * beta / 2.0 * vov * vov;
    if saturated >= vov {
        return saturated;
    }
    // triode: Vds = Vdd - Rd beta (Vov Vds - Vds^2 / 2), a quadratic of which the smaller root
    // is the one below Vov
    let a = rd * beta / 2.0;
    let b = -(1.0 + rd * beta * vov);
    (-b - (b * b - 4.0 * a * vdd).sqrt()) / (2.0 * a)
}

const NMOS_INVERTER: &str = "nmos inverter\n.model nm NMOS(VTO=1 KP=2e-5 W=1e-3 L=1e-5)\nVDD vdd 0 DC 5\nVIN in 0 DC 0\nRD vdd out 1k\nM1 out in 0 0 nm\n";

#[test]
fn the_nmos_inverter_follows_its_theoretical_transfer_table() {
    let c = circuit(NMOS_INVERTER);
    let request = DcRequest {
        source: "vin".to_string(),
        start: 0.0,
        stop: 5.0,
        step: 0.25,
    };
    let result = dc_sweep(&c, &settings(), &request).unwrap();
    let out = result.series("v(out)").unwrap();
    assert_eq!(out.len(), 21);
    // beta = KP W / L = 2e-5 x 100
    for (vin, got) in result.sweep.iter().zip(out.iter()) {
        let want = inverter(*vin, 5.0, 1000.0, 2e-3, 1.0);
        assert!((got - want).abs() < 1e-4, "{vin}: {got} {want}");
    }
    // the table has the three regions: cut-off up to the threshold, then a falling curve
    assert!((out[0] - 5.0).abs() < 1e-6 && (out[4] - 5.0).abs() < 1e-6);
    assert!(out.windows(2).all(|w| w[1] <= w[0] + 1e-9));
    assert!((out[8] - 4.0).abs() < 1e-4);
    assert!(out[20] < 0.6);
    // the slope of the saturated part is -beta Rd Vov: -2.25 per volt between 2 V and 2.25 V
    let slope = (out[9] - out[8]) / 0.25;
    assert!((slope + 2.25).abs() < 1e-3, "{slope}");
}

#[test]
fn the_cmos_inverter_swings_rail_to_rail_in_a_sweep() {
    let text = "cmos\n.model nm NMOS(VTO=1 KP=2e-5 W=1e-4 L=1e-5)\n.model pm PMOS(VTO=-1 KP=1e-5 W=4e-4 L=1e-5)\nVDD vdd 0 5\nVIN in 0 0\nMP out in vdd vdd pm\nMN out in 0 0 nm\nRL out 0 100meg\n";
    let c = circuit(text);
    let request = DcRequest {
        source: "vin".to_string(),
        start: 0.0,
        stop: 5.0,
        step: 0.25,
    };
    let result = dc_sweep(&c, &settings(), &request).unwrap();
    let out = result.series("v(out)").unwrap();
    assert!(out[0] > 4.99 && out[20] < 0.01, "{} {}", out[0], out[20]);
    assert!(out.windows(2).all(|w| w[1] <= w[0] + 1e-6));
    // beta of the N channel 2e-4 and of the P channel 4e-4 A/V^2: both saturated at the input
    // where (beta_n / 2) (V - 1)^2 = (beta_p / 2) (4 - V)^2, which is V = 2.757 V; a quarter
    // volt below it the output is high and a quarter volt above it, low
    assert!(out[8] > 4.0, "{}", out[8]);
    assert!(out[12] < 1.0, "{}", out[12]);
}

#[test]
fn an_inverting_amplifier_has_the_gain_minus_r2_over_r1() {
    let ideal = "inverting\n.model oa OPAMP(GAIN=1e6 GBW=0 ROUT=1)\nVIN in 0 DC 1\nR1 in inn 1k\nR2 inn out 2k\nU1 0 inn out oa\n";
    let out = op(ideal, "v(out)");
    // the ideal gain, less the effect of the finite gain: 1 + (1 + R2/R1)/A
    assert!((out + 2.0).abs() < 2e-5, "{out}");
    let want = -2.0 / (1.0 + 3.0 / 1e6);
    assert!((out - want).abs() < 1e-7, "{out} {want}");
    // the input resistance is R1: the source gives 1 V / 1 kohm
    let r = operating_point(&circuit(ideal), &settings()).unwrap();
    assert!((-r.get("i(vin)").unwrap() - 1e-3).abs() < 1e-8);
    // a sweep through zero with rails: the output keeps the gain until the rail is near
    let railed = "inverting\n.model oa OPAMP(GAIN=1e5 GBW=0 VPOS=15 VNEG=-15 ROUT=1)\nVIN in 0 DC 0\nR1 in inn 1k\nR2 inn out 2k\nU1 0 inn out oa\n";
    let result = dc_sweep(
        &circuit(railed),
        &settings(),
        &DcRequest {
            source: "vin".to_string(),
            start: -2.0,
            stop: 2.0,
            step: 0.5,
        },
    )
    .unwrap();
    let out = result.series("v(out)").unwrap();
    for (vin, got) in result.sweep.iter().zip(out.iter()) {
        assert!((got + 2.0 * vin).abs() < 1e-3, "{vin}: {got}");
    }
}

#[test]
fn the_gain_of_the_inverting_amplifier_falls_at_the_gain_bandwidth_product() {
    // closed loop: -(R2/R1) / (1 + (1 + R2/R1) / A(jw)) with A(jw) = A0 / (1 + j f / fp) and
    // fp = GBW / A0; at f = 100 kHz with GBW = 1 MHz the loop gain is about 10 / 3
    let text = "inv\n.model oa OPAMP(GAIN=1e5 GBW=1meg ROUT=1)\nVIN in 0 DC 0 AC 1\nR1 in inn 1k\nR2 inn out 2k\nU1 0 inn out oa\n";
    let f = 1e5;
    let result = ac_at(&circuit(text), &settings(), &[f]).unwrap();
    let got = result.series("v(out)").unwrap()[0].abs();
    let fp = 1e6 / 1e5;
    let a = eh3d_phoenix_rf::Complex::new(1e5, 0.0) / eh3d_phoenix_rf::Complex::new(1.0, f / fp);
    let denominator =
        eh3d_phoenix_rf::Complex::new(1.0, 0.0) + eh3d_phoenix_rf::Complex::new(3.0, 0.0) / a;
    let want = (eh3d_phoenix_rf::Complex::new(2.0, 0.0) / denominator).abs();
    assert!((got - want).abs() < 1e-3 * want, "{got} {want}");
    assert!(got < 2.0);
}

const SWITCH: &str = "switch\n.model sm SW(VT=2.5 VH=0.1 RON=100 ROFF=1e9)\nV1 in 0 DC 5\nVC c 0 DC 5\nS1 in out c 0 sm\nRL out 0 900\n";

#[test]
fn a_switch_is_a_resistor_that_the_control_voltage_chooses() {
    let on = op(SWITCH, "v(out)");
    assert!((on - 4.5).abs() < 1e-6, "{on}");
    let off = op(&SWITCH.replace("VC c 0 DC 5", "VC c 0 DC 0"), "v(out)");
    let want = 5.0 * 900.0 / (900.0 + 1e9);
    assert!((off - want).abs() < 1e-9, "{off} {want}");
    // at the threshold the conductance is the geometric mean of the two
    let middle = op(&SWITCH.replace("VC c 0 DC 5", "VC c 0 DC 2.5"), "v(out)");
    let r_mid = (100.0f64 * 1e9).sqrt();
    let want_mid = 5.0 * 900.0 / (900.0 + r_mid);
    assert!(
        (middle - want_mid).abs() < 1e-3 * want_mid,
        "{middle} {want_mid}"
    );
}

#[test]
fn a_swept_control_takes_the_switch_through_its_transition() {
    let c = circuit(SWITCH);
    let result = dc_sweep(
        &c,
        &settings(),
        &DcRequest {
            source: "vc".to_string(),
            start: 0.0,
            stop: 5.0,
            step: 0.25,
        },
    )
    .unwrap();
    let out = result.series("v(out)").unwrap();
    assert!(out[0] < 1e-5 && (out[20] - 4.5).abs() < 1e-6);
    assert!(out.windows(2).all(|w| w[1] >= w[0] - 1e-12));
    // below the transition it is off and above it is on
    assert!(out[8] < 1e-4, "{}", out[8]);
    assert!(out[12] > 4.4, "{}", out[12]);
}

#[test]
fn a_switch_closed_by_a_pulse_charges_a_capacitor() {
    let text = "sw\n.model sm SW(VT=2.5 VH=0.1 RON=1k ROFF=1e9)\nV1 in 0 DC 5\nVC c 0 PULSE(0 5 1m 1u 1u 1 0)\nS1 in out c 0 sm\nC1 out 0 1u\nRL out 0 1meg\n";
    let c = circuit(text);
    let result = transient(
        &c,
        &settings(),
        &TranRequest {
            tstep: 1e-5,
            tstop: 3e-3,
            method: Method::Trapezoidal,
            adaptive: false,
            uic: false,
        },
    )
    .unwrap();
    let out = result.series("v(out)").unwrap();
    let at = |t: f64| {
        let k = result
            .times
            .iter()
            .position(|x| (x - t).abs() < 0.5e-5)
            .unwrap();
        out[k]
    };
    // before the pulse the switch is off and the divider of ROFF and the 1 Mohm load holds the
    // output at 5 mV
    assert!(at(0.5e-3) < 0.01, "{}", at(0.5e-3));
    // 1 ms after the switch closes the RC (1 kohm, 1 uF) is at one time constant: 1 - 1/e
    // (the control crosses its threshold half way up the 1 us edge; the load changes the end
    // value by 0.1 %)
    let want = 5.0 * (1.0 - (-1.0f64).exp());
    assert!((at(2.0e-3) - want).abs() < 0.1, "{} {want}", at(2.0e-3));
    assert!(at(3.0e-3) > at(2.0e-3));
}

#[test]
fn a_switch_has_a_small_signal_resistance_in_the_ac_analysis() {
    // an RC low-pass whose resistor is the closed switch: 1 kohm and 159.155 nF, corner 1 kHz
    let text = "swac\n.model sm SW(VT=2.5 VH=0.1 RON=1k ROFF=1e9)\nV1 in 0 DC 0 AC 1\nVC c 0 DC 5\nS1 in out c 0 sm\nC1 out 0 159.15494309189535n\n";
    let result = ac_at(&circuit(text), &settings(), &[1000.0]).unwrap();
    let h = result.series("v(out)").unwrap()[0];
    assert!(
        (h.abs() - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-6,
        "{}",
        h.abs()
    );
    assert!((h.arg_deg() + 45.0).abs() < 1e-4, "{}", h.arg_deg());
}

#[test]
fn the_analyses_of_a_netlist_are_read_and_can_be_run() {
    let s = settings();
    let text = "divider\nV1 in 0 DC 1\nR1 in out 1k\nR2 out 0 3k\n.op\n.dc v1 0 4 1\n.ac dec 5 10 1k\n.tran 1u 10u uic\n";
    let netlist = parse(text, &s).unwrap();
    assert_eq!(netlist.analyses.len(), 4);
    let c = Circuit::compile(&netlist, &s).unwrap();
    for directive in &netlist.analyses {
        match directive {
            Directive::Op => {
                assert!(
                    (operating_point(&c, &s).unwrap().get("v(out)").unwrap() - 0.75).abs() < 1e-9
                );
            }
            Directive::Dc {
                source,
                start,
                stop,
                step,
            } => {
                let r = dc_sweep(
                    &c,
                    &s,
                    &DcRequest {
                        source: source.clone(),
                        start: *start,
                        stop: *stop,
                        step: *step,
                    },
                )
                .unwrap();
                assert_eq!(r.sweep, vec![0.0, 1.0, 2.0, 3.0, 4.0]);
            }
            Directive::Ac { kind, .. } => assert_eq!(*kind, SweepKind::Decade),
            Directive::Tran { uic, .. } => assert!(*uic),
        }
    }
}

#[test]
fn a_dc_sweep_writes_the_same_bytes_every_time() {
    let c = circuit(NMOS_INVERTER);
    let request = DcRequest {
        source: "vin".to_string(),
        start: 0.0,
        stop: 5.0,
        step: 0.5,
    };
    let run = || {
        let r = dc_sweep(&c, &settings(), &request).unwrap();
        dc_csv(&r, &select(&r.labels, &["v(out)".to_string()]).unwrap())
    };
    let first = run();
    assert_eq!(first, run());
    assert!(first.starts_with("sweep(vin),v(out)\n0e0,"), "{first}");
    assert_eq!(first.lines().count(), 12);
    let second_row = first.lines().nth(1).unwrap();
    let value: f64 = second_row.split(',').nth(1).unwrap().parse().unwrap();
    assert!((value - 5.0).abs() < 1e-6, "{second_row}");
}
