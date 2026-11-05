// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `phoenix rf`: Touchstone files, impedance matching and ladder filters.

use eh3d_phoenix_cli::run;

fn call(list: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = list.iter().map(|s| s.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

const AMPLIFIER: &str = "! test amplifier with invented values\n# MHz S RI R 50\n\
    100 0.10 0.00 3.00 0.50 0.05 0.00 0.30 -0.10\n\
    200 0.20 0.05 2.80 0.40 0.05 0.01 0.35 -0.12\n";

fn write(dir: &tempfile::TempDir, name: &str, text: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn a_touchstone_file_is_summarised_and_evaluated_at_a_frequency() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "amp.s2p", AMPLIFIER);
    let (code, header, err) = call(&["rf", "touchstone", &path]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(header.contains("ports: 2") && header.contains("points: 2"));
    assert!(header.contains("noise points: 0"));
    let (code, text, _) = call(&["rf", "touchstone", &path, "--freq-ghz", "0.15"]);
    assert_eq!(code, 0);
    for want in [
        "S11:",
        "S21:",
        "input return loss:",
        "insertion loss:",
        "group delay:",
        "stability: K",
        "maximum stable gain:",
    ] {
        assert!(text.contains(want), "{want}\n{text}");
    }
}

#[test]
fn a_one_port_file_gives_the_impedance() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "load.s1p", "# GHz S RI R 50\n1 0.3333333333 0\n");
    let (code, text, _) = call(&["rf", "touchstone", &path, "--freq-ghz", "1"]);
    assert_eq!(code, 0);
    assert!(text.contains("VSWR: 2.0000") && text.contains("impedance: 100.0000"));
}

#[test]
fn touchstone_errors_have_their_exit_codes_and_languages() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(&dir, "amp.s2p", AMPLIFIER);
    let bad = write(&dir, "bad.s1p", "# GHz S RI\n1 0.5 oops\n");
    let no_ports = write(&dir, "amp.txt", AMPLIFIER);
    let (code, _, err) = call(&["rf", "touchstone", &bad]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-RF-002") && err.contains("line 2"),
        "{err}"
    );
    let (code, _, err) = call(&["rf", "touchstone", &bad, "--lang", "es"]);
    assert_eq!(code, 2);
    assert!(err.contains("error de sintaxis"), "{err}");
    let (code, _, err) = call(&["rf", "touchstone", &no_ports]);
    assert_eq!(code, 2);
    assert!(err.contains("extension"));
    // a frequency outside the file has no result
    let (code, _, err) = call(&["rf", "touchstone", &good, "--freq-ghz", "5"]);
    assert_eq!(code, 1);
    assert!(err.contains("PHOENIX-RF-010"), "{err}");
    let (code, _, err) = call(&["rf", "touchstone", &good, "--freq-ghz", "fast"]);
    assert_eq!(code, 2);
    assert!(err.contains("not a number"));
    let missing = dir.path().join("absent.s2p");
    let (code, _, _) = call(&["rf", "touchstone", missing.to_str().unwrap()]);
    assert_eq!(code, 2);
}

#[test]
fn the_settings_file_changes_the_limits() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(&dir, "amp.s2p", AMPLIFIER);
    let settings = write(&dir, "rf.cfg", "max_points = 1\n");
    let (code, _, err) = call(&["rf", "touchstone", &good, "--settings", &settings]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-RF-009"), "{err}");
    let filter_limit = write(&dir, "order.cfg", "max_filter_order = 2\n");
    let (code, _, err) = call(&[
        "rf",
        "filter",
        "butterworth",
        "3",
        "lowpass",
        "--fc-ghz",
        "0.1",
        "--settings",
        &filter_limit,
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("PHOENIX-RF-009"), "{err}");
    let broken = write(&dir, "broken.cfg", "nonsense\n");
    let (code, _, err) = call(&["rf", "touchstone", &good, "--settings", &broken]);
    assert_eq!(code, 2);
    assert!(err.contains("settings"));
}

#[test]
fn the_l_network_from_50_to_100_ohm_has_the_book_components() {
    let (code, text, err) = call(&[
        "rf",
        "match",
        "l",
        "--source",
        "50",
        "--load",
        "100",
        "--freq-ghz",
        "0.1",
    ]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(
        text.contains("79.5775 nH") && text.contains("15.9155 pF"),
        "{text}"
    );
    assert!(text.contains("low-pass") && text.contains("high-pass"));
    // the source defaults to 50 ohm
    let (code, again, _) = call(&["rf", "match", "l", "--load", "100", "--freq-ghz", "0.1"]);
    assert_eq!(code, 0);
    assert_eq!(text, again);
    // complex impedances
    let (code, complex, _) = call(&[
        "rf",
        "match",
        "l",
        "--source",
        "50+j10",
        "--load",
        "20-j30",
        "--freq-ghz",
        "0.433",
    ]);
    assert_eq!(code, 0);
    assert!(complex.contains("check: the source sees"));
}

#[test]
fn stubs_and_quarter_wave_sections_give_their_lengths() {
    let (code, text, err) = call(&[
        "rf",
        "match",
        "stub",
        "--load",
        "100",
        "--z0",
        "50",
        "--freq-ghz",
        "1",
        "--stub",
        "shunt",
        "--end",
        "short",
        "--eps-eff",
        "1",
    ]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(
        text.contains("solution 2") && text.contains("54.736 deg"),
        "{text}"
    );
    let (code, series, _) = call(&[
        "rf",
        "match",
        "stub",
        "--load",
        "30+j40",
        "--freq-ghz",
        "1",
        "--stub",
        "series",
        "--end",
        "open",
    ]);
    assert_eq!(code, 0);
    assert!(series.contains("series stub, open"));
    let (code, qw, _) = call(&[
        "rf",
        "match",
        "qw",
        "--load",
        "200",
        "--z0",
        "50",
        "--freq-ghz",
        "1",
    ]);
    assert_eq!(code, 0);
    assert!(qw.contains("transformer: 100.0000 ohm"), "{qw}");
    let (code, complex, _) = call(&[
        "rf",
        "match",
        "qw",
        "--load",
        "30+j40",
        "--freq-ghz",
        "1",
        "--eps-eff",
        "2.5",
    ]);
    assert_eq!(code, 0);
    assert!(complex.contains("then a real impedance"));
}

#[test]
fn matching_problems_are_exit_1_and_bad_input_is_exit_2() {
    let (code, _, err) = call(&["rf", "match", "l", "--load", "-5", "--freq-ghz", "1"]);
    assert_eq!(code, 1);
    assert!(err.contains("PHOENIX-RF-001"), "{err}");
    let (code, _, err) = call(&["rf", "match", "l", "--load", "abc", "--freq-ghz", "1"]);
    assert_eq!(code, 2);
    assert!(err.contains("--load"));
    let (code, _, _) = call(&["rf", "match", "l", "--freq-ghz", "1"]);
    assert_eq!(code, 2);
    let (code, _, _) = call(&["rf", "match", "l", "--load", "100"]);
    assert_eq!(code, 2);
    let (code, _, _) = call(&[
        "rf",
        "match",
        "stub",
        "--load",
        "100",
        "--freq-ghz",
        "1",
        "--end",
        "x",
    ]);
    assert_eq!(code, 2);
}

#[test]
fn a_filter_is_designed_and_swept() {
    let (code, text, err) = call(&[
        "rf",
        "filter",
        "butterworth",
        "3",
        "lowpass",
        "--fc-ghz",
        "0.1",
        "--sweep-ghz",
        "0.01,0.2,3",
    ]);
    assert_eq!((code, err.is_empty()), (0, true), "{err}");
    assert!(text.contains("prototype g: 1.0000 2.0000 1.0000"), "{text}");
    assert!(text.contains("capacitor 31.8310 pF") && text.contains("inductor 159.1549 nH"));
    assert!(text.contains("frequency, insertion loss, return loss"));
    let (code, band, _) = call(&[
        "rf",
        "filter",
        "chebyshev",
        "5",
        "bandpass",
        "--f1-ghz",
        "0.09",
        "--f2-ghz",
        "0.11",
        "--ripple-db",
        "0.1",
        "--z0",
        "75",
        "--first",
        "series",
        "--sweep-ghz",
        "0.05,0.2,5",
        "--log",
    ]);
    assert_eq!(code, 0);
    assert!(band.contains("Chebyshev (0.1 dB ripple) filter of order 5, band-pass"));
    let (code, bessel, _) = call(&[
        "rf", "filter", "bessel", "4", "highpass", "--fc-ghz", "0.05",
    ]);
    assert_eq!(code, 0);
    assert!(bessel.contains("Bessel filter of order 4, high-pass"));
    let (code, stop, _) = call(&[
        "rf",
        "filter",
        "butterworth",
        "3",
        "bandstop",
        "--f1-ghz",
        "0.09",
        "--f2-ghz",
        "0.11",
    ]);
    assert_eq!(code, 0);
    assert!(stop.contains("band-stop"));
}

#[test]
fn filter_errors_are_reported_in_the_language_asked() {
    let (code, _, err) = call(&[
        "rf",
        "filter",
        "chebyshev",
        "3",
        "lowpass",
        "--fc-ghz",
        "0.1",
        "--lang",
        "de",
    ]);
    assert_eq!(code, 1);
    assert!(
        err.contains("PHOENIX-RF-007") && err.contains("Das Filter ist ungültig"),
        "{err}"
    );
    let (code, _, err) = call(&[
        "rf", "filter", "bessel", "9", "lowpass", "--fc-ghz", "0.1", "--lang", "pt",
    ]);
    assert_eq!(code, 1);
    assert!(err.contains("O filtro não é válido"), "{err}");
    let (code, _, err) = call(&["rf", "filter", "butterworth", "3", "lowpass"]);
    assert_eq!(code, 2);
    assert!(err.contains("band edges"));
    let (code, _, _) = call(&[
        "rf",
        "filter",
        "butterworth",
        "x",
        "lowpass",
        "--fc-ghz",
        "0.1",
    ]);
    assert_eq!(code, 2);
    let (code, _, _) = call(&[
        "rf",
        "filter",
        "butterworth",
        "3",
        "lowpass",
        "--fc-ghz",
        "0.1",
        "--sweep-ghz",
        "1,2",
    ]);
    assert_eq!(code, 2);
}

#[test]
fn rf_arguments_that_cannot_be_used_exit_2_with_the_usage() {
    for list in [
        &["rf"][..],
        &["rf", "nonsense"],
        &["rf", "touchstone"],
        &["rf", "match"],
        &["rf", "match", "nonsense", "--load", "1", "--freq-ghz", "1"],
        &["rf", "filter", "butterworth", "3"],
        &["rf", "filter", "wavelet", "3", "lowpass", "--fc-ghz", "1"],
        &[
            "rf",
            "filter",
            "butterworth",
            "3",
            "midpass",
            "--fc-ghz",
            "1",
        ],
        &["rf", "filter", "butterworth", "3", "lowpass", "--fc-ghz"],
        &[
            "rf",
            "filter",
            "butterworth",
            "3",
            "lowpass",
            "--unknown",
            "1",
        ],
        &["rf", "touchstone", "x.s2p", "--lang", "xx"],
    ] {
        let (code, out, err) = call(list);
        assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
        assert!(err.contains("phoenix rf"), "{list:?}: {err}");
    }
}
