// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `server` on a board written by the test itself: without `--send` it only shows the
//! request, with `--send` it needs the switch of the settings, and with both it talks to a small
//! HTTP server on the loopback address of this computer that answers the way EH3D-SERVER
//! documents. Nothing leaves the computer and nothing is written.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadShape, Segment, Side, Track,
    Via,
};
use eh3d_phoenix_cli::run;
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

const MM: i64 = 1_000_000;

fn command(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn id(n: u32) -> Id {
    ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, u64::from(n)).unwrap()
}

fn pt(x: i64, y: i64) -> Point {
    Point::from_nm(x, y).unwrap()
}

/// A 20 x 10 mm board with one layer, two nets, two tracks 0.5 mm apart, a via and a resistor.
fn board() -> Board {
    let mut b = Board {
        name: "demo".into(),
        outline: vec![
            pt(0, 0),
            pt(20 * MM, 0),
            pt(20 * MM, 10 * MM),
            pt(0, 10 * MM),
        ],
        ..Board::default()
    };
    b.layers.insert(
        id(1),
        Layer {
            name: "Top".into(),
            kind: LayerKind::Copper,
            side: None,
        },
    );
    b.nets.insert(id(2), Net { name: "GND".into() });
    b.nets.insert(id(3), Net { name: "SIG".into() });
    for (n, net, y) in [(30, 2, 3 * MM), (31, 3, 3 * MM + 500_000)] {
        b.tracks.insert(
            id(n),
            Track {
                net: id(net),
                layer: id(1),
                width: Nm::new(150_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(2 * MM, y),
                    end: pt(12 * MM, y),
                }],
            },
        );
    }
    b.vias.insert(
        id(40),
        Via {
            net: id(2),
            at: pt(15 * MM, 5 * MM),
            drill: Nm::new(300_000).unwrap(),
            diameter: Nm::new(600_000).unwrap(),
            from_layer: id(1),
            to_layer: id(1),
        },
    );
    let mut resistor = Footprint {
        name: "R_0603".into(),
        pads: Default::default(),
    };
    resistor.pads.insert(
        id(11),
        Pad {
            number: "1".into(),
            shape: PadShape::Circle {
                diameter: Nm::new(800_000).unwrap(),
            },
            at: pt(0, 0),
            rotation: Udeg::ZERO,
            drill: None,
            layers: vec![id(1)],
            roundness: 0,
            plated: true,
        },
    );
    b.footprints.insert(id(10), resistor);
    b.components.insert(
        id(20),
        Component {
            refdes: "R1".into(),
            value: "10k".into(),
            symbol: None,
            footprint: id(10),
            at: pt(10 * MM, 7 * MM),
            rotation: Udeg::ZERO,
            side: Side::Top,
        },
    );
    b
}

fn write_text(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

fn write_board(dir: &Path) -> String {
    let path = dir.join("board.json");
    fs::write(&path, board().to_bytes().unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

fn names(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

const VIEW: &str = "{\"kind\":\"rules_check\",\"verdict\":\"fail\",\"errors\":1,\"warnings\":0,\"violations\":[{\"rule\":\"track_width\",\"severity\":\"error\",\"item\":\"GND Top\",\"message\":\"GND Top is 0.15 mm wide\"}]}";

type Seen = Arc<Mutex<Vec<(String, String)>>>;

fn read_request(stream: &mut TcpStream) -> Option<(String, String)> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 1024];
    let head_end = loop {
        if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&data[..head_end]).into_owned();
    let length = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0);
    while data.len() < head_end + length {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..n]);
    }
    let body = String::from_utf8_lossy(&data[head_end..]).into_owned();
    let mut first = head.lines().next()?.split(' ');
    let line = format!("{} {}", first.next()?, first.next()?);
    Some((line, body))
}

fn answer(stream: &mut TcpStream, status: &str, body: &str) {
    let text = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(text.as_bytes());
}

/// Serves `connections` requests (make, start, ask, or health) and returns the address and what was seen.
fn small_server(connections: usize) -> (String, Seen, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let seen: Seen = Arc::default();
    let log = seen.clone();
    let handle = thread::spawn(move || {
        for _ in 0..connections {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let Some((line, body)) = read_request(&mut stream) else {
                continue;
            };
            log.lock().unwrap().push((line.clone(), body));
            match line.as_str() {
                "GET /api/v1/health" => answer(
                    &mut stream,
                    "200 OK",
                    "{\"name\":\"eh3d-server\",\"version\":\"0.9.1\",\"api_revision\":1,\"status\":\"ok\"}",
                ),
                "POST /api/v1/work/jobs" => answer(
                    &mut stream,
                    "201 Created",
                    "{\"id\":\"w-1\",\"state\":\"queued\"}",
                ),
                "POST /api/v1/work/jobs/w-1/start" => answer(
                    &mut stream,
                    "200 OK",
                    "{\"id\":\"w-1\",\"state\":\"running\"}",
                ),
                "GET /api/v1/work/jobs/w-1" => answer(
                    &mut stream,
                    "200 OK",
                    &format!(
                        "{{\"id\":\"w-1\",\"state\":\"done\",\"message\":\"\",\"view\":{VIEW}}}"
                    ),
                ),
                _ => answer(
                    &mut stream,
                    "404 Not Found",
                    "{\"error\":\"not_found\",\"detail\":\"\"}",
                ),
            }
        }
    });
    (address, seen, handle)
}

fn settings_text(address: &str, allow: bool) -> String {
    format!(
        "server_address = {address}\nserver_token = cli-secret-token\nserver_machine = pcb-1\nserver_retries = 0\nserver_timeout_s = 5\nallow_network = {allow}\npoll_ms = 10\n"
    )
}

#[test]
fn without_send_the_command_shows_the_request_and_sends_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let (code, out, err) = command(&["server", &path]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("Server report: rules_check (dry run, nothing was sent)\n"),
        "{out}"
    );
    // two tracks, one gap, one hole
    assert!(out.contains("Items: 4"), "{out}");
    assert!(out.contains("\"kind\":\"rules_check\""), "{out}");
    assert!(out.contains("info: PHOENIX-SERVER-012"), "{out}");
    assert_eq!(names(dir.path()), vec!["board.json"]);
    // the same board gives the same text
    assert_eq!(command(&["server", &path]).1, out);
}

#[test]
fn the_json_of_a_dry_run_holds_the_request_with_the_gap_between_the_tracks() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let (code, out, _) = command(&["server", "--json", &path]);
    assert_eq!(code, 0);
    let j: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(j["format"], "phoenix-server/1");
    assert_eq!(j["sent"], false);
    let board = &j["request"]["board"];
    assert_eq!(board["outline"]["width_mm"], 20.0);
    assert_eq!(board["tracks"].as_array().unwrap().len(), 2);
    // 0.5 mm between the centres less the two half widths of 0.075 mm
    assert_eq!(board["clearances"][0]["gap_mm"], 0.35);
    assert_eq!(board["drills"][0]["dia_mm"], 0.3);
    assert!(j.get("job").is_none());
}

#[test]
fn the_other_kinds_make_their_own_requests() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let (code, out, _) = command(&["server", "--kind", "drill", "--json", &path]);
    assert_eq!(code, 0);
    let j: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(j["request"]["kind"], "drill_report");
    assert_eq!(j["items"], 1);
    let (code, out, _) = command(&["server", "--kind", "bom", "--json", &path]);
    assert_eq!(code, 0);
    let j: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(j["request"]["components"][0]["footprint"], "R_0603");
    let rows = write_text(
        dir.path(),
        "m.csv",
        "# name, nominal, tolerance, value\nwidth, 1.6, 0.1, 1.62, mm\n",
    );
    let (code, out, _) = command(&["server", "--kind", "inspect", "--json", &rows]);
    assert_eq!(code, 0);
    let j: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(j["request"]["measurements"][0]["unit"], "mm");
    // a board is not a file of measurements
    let (code, _, err) = command(&["server", "--kind", "inspect", &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SERVER-003"), "{err}");
}

#[test]
fn the_rules_of_the_settings_go_over_the_rules_of_the_station() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let cfg = write_text(dir.path(), "server.cfg", "rule_min_track_nm = 200000\n");
    let (code, out, _) = command(&["server", "--config", &cfg, "--json", &path]);
    assert_eq!(code, 0);
    let j: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(j["request"]["rules"]["min_track_mm"], 0.2);
}

#[test]
fn sending_needs_the_switch_the_address_the_machine_and_the_token() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    // the switch is off: nothing is tried, even with an address
    let off = write_text(dir.path(), "off.cfg", &settings_text("127.0.0.1:9", false));
    let (code, _, err) = command(&["server", "--send", "--config", &off, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SERVER-005"), "{err}");
    assert!(!err.contains("cli-secret-token"), "{err}");
    // the switch is on and there is no address
    let none = write_text(dir.path(), "none.cfg", "allow_network = true\n");
    let (code, _, err) = command(&["server", "--send", "--config", &none, &path]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SERVER-006"), "{err}");
    assert!(err.contains("server_address"), "{err}");
}

#[test]
fn a_report_is_made_over_a_real_connection_and_the_exit_code_follows_the_verdict() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let (address, seen, handle) = small_server(3);
    let cfg = write_text(dir.path(), "on.cfg", &settings_text(&address, true));
    let (code, out, err) = command(&["server", "--send", "--config", &cfg, "--lang", "pt", &path]);
    handle.join().unwrap();
    assert_eq!(code, 1, "{err}\n{out}");
    assert!(
        out.contains("Relatório do servidor: rules_check (enviado para o servidor)"),
        "{out}"
    );
    assert!(out.contains("reprovado"), "{out}");
    assert!(out.contains("track_width [GND Top]"), "{out}");
    assert!(!out.contains("cli-secret-token") && !err.contains("cli-secret-token"));
    let seen = seen.lock().unwrap();
    let lines: Vec<&str> = seen.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(
        lines,
        [
            "POST /api/v1/work/jobs",
            "POST /api/v1/work/jobs/w-1/start",
            "GET /api/v1/work/jobs/w-1",
        ]
    );
    // the job is named after the board file and carries the request of the dry run
    let body: serde_json::Value = serde_json::from_str(&seen[0].1).unwrap();
    assert_eq!(body["machine"], "pcb-1");
    assert_eq!(body["name"], "board");
    assert_eq!(body["request"]["kind"], "rules_check");
    assert_eq!(names(dir.path()), vec!["board.json", "on.cfg"]);
}

#[test]
fn health_asks_the_server_whether_it_is_there_and_sends_no_token() {
    let dir = tempfile::tempdir().unwrap();
    let (address, seen, handle) = small_server(2);
    let cfg = write_text(dir.path(), "on.cfg", &settings_text(&address, true));
    let (code, out, err) = command(&["server", "--health", "--config", &cfg, "--lang", "es"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "Estado del servidor: eh3d-server 0.9.1\nEstado: ok\n  api_revision: 1\n"
    );
    let (code, out, _) = command(&["server", "--health", "--json", "--config", &cfg]);
    handle.join().unwrap();
    assert_eq!(code, 0);
    let j: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(j["health"]["status"], "ok");
    assert_eq!(j["health"]["api_revision"], 1);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(seen.iter().all(|(line, _)| line == "GET /api/v1/health"));
}

#[test]
fn a_job_that_is_on_the_server_is_read_by_its_id() {
    let dir = tempfile::tempdir().unwrap();
    let (address, seen, handle) = small_server(1);
    let cfg = write_text(dir.path(), "on.cfg", &settings_text(&address, true));
    let (code, out, err) = command(&["server", "--job", "w-1", "--config", &cfg]);
    handle.join().unwrap();
    // the verdict of the report is a fail
    assert_eq!(code, 1, "{err}\n{out}");
    assert!(
        out.contains("Server report: rules_check (sent to the server)"),
        "{out}"
    );
    assert!(out.contains("Job: w-1 (State: done)"), "{out}");
    assert!(out.contains("track_width [GND Top]"), "{out}");
    assert_eq!(seen.lock().unwrap()[0].0, "GET /api/v1/work/jobs/w-1");
}

#[test]
fn health_and_job_need_the_switch_and_take_no_file_and_a_bad_id_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let off = write_text(dir.path(), "off.cfg", &settings_text("127.0.0.1:9", false));
    for args in [
        vec!["server", "--health", "--config", off.as_str()],
        vec!["server", "--job", "w-1", "--config", off.as_str()],
    ] {
        let (code, _, err) = command(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.contains("PHOENIX-SERVER-005"), "{args:?}: {err}");
    }
    let on = write_text(dir.path(), "on.cfg", &settings_text("127.0.0.1:9", true));
    let (code, _, err) = command(&["server", "--job", "../etc", "--config", &on]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SERVER-002"), "{err}");
    for args in [
        vec!["server", "--health", "--job", "w-1"],
        vec!["server", "--health", path.as_str()],
        vec!["server", "--job", "w-1", "--send"],
        vec!["server", "--job"],
    ] {
        let (code, _, err) = command(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.contains("PHOENIX-SERVER-002"), "{args:?}: {err}");
    }
}

#[test]
fn a_bad_option_a_bad_setting_or_a_missing_file_is_refused_with_its_code() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let (code, _, err) = command(&["server"]);
    assert_eq!(code, 2);
    assert!(err.starts_with("phoenix server "), "{err}");
    for args in [
        vec!["server", "--bogus", path.as_str()],
        vec!["server", "--kind", "bogus", path.as_str()],
        vec!["server", "--kind"],
        vec!["server", "--lang", "xx", path.as_str()],
        vec!["server", path.as_str(), path.as_str()],
    ] {
        let (code, _, err) = command(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.contains("PHOENIX-SERVER-002"), "{args:?}: {err}");
    }
    let cfg = write_text(dir.path(), "bad.cfg", "poll_ms = 1\n");
    let (code, _, err) = command(&["server", "--config", &cfg, &path]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-SERVER-001") && err.contains("poll_ms"),
        "{err}"
    );
    let missing = dir.path().join("missing.json");
    let (code, _, err) = command(&["server", missing.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SERVER-003"), "{err}");
    assert_eq!(names(dir.path()), vec!["bad.cfg", "board.json"]);
}

#[test]
fn saving_and_loading_a_project_need_the_switch_and_say_so_in_every_language() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let off = write_text(dir.path(), "off.cfg", &settings_text("127.0.0.1:9", false));
    let out_file = dir.path().join("back.json");
    let out_file = out_file.to_str().unwrap();
    let mut seen = Vec::new();
    for tag in ["en", "es", "de", "fr", "it", "ja", "zh", "pt"] {
        let (code, out, err) = command(&[
            "server",
            "--save-project",
            "p-1",
            "--config",
            &off,
            "--lang",
            tag,
            &path,
        ]);
        assert_eq!((code, out.as_str()), (2, ""), "{tag}");
        assert!(err.contains("PHOENIX-SERVER-005"), "{tag}: {err}");
        assert!(!err.contains("cli-secret-token"), "{err}");
        assert!(!seen.contains(&err), "{tag} repeats a text");
        seen.push(err);
        let (code, _, err) = command(&[
            "server",
            "--load-project",
            "p-1",
            "--name",
            "board.json",
            "--out",
            out_file,
            "--config",
            &off,
            "--lang",
            tag,
        ]);
        assert_eq!(code, 2, "{tag}");
        assert!(err.contains("PHOENIX-SERVER-005"), "{tag}: {err}");
    }
    // nothing was written
    assert_eq!(names(dir.path()), vec!["board.json", "off.cfg"]);
}

#[test]
fn the_options_of_the_store_are_checked_before_any_connection() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_board(dir.path());
    let on = write_text(dir.path(), "on.cfg", &settings_text("127.0.0.1:9", true));
    let existing = path.as_str();
    for args in [
        // both at once, or neither
        vec![
            "server",
            "--save-project",
            "p",
            "--load-project",
            "q",
            &path,
        ],
        // a save takes one file and none of the options of a load
        vec!["server", "--save-project", "p"],
        vec!["server", "--save-project", "p", "--out", "x.json", &path],
        vec!["server", "--save-project", "p", "--version", "2", &path],
        // a load needs the name and the file to write, takes no file and no tag
        vec!["server", "--load-project", "p", "--out", "x.json"],
        vec!["server", "--load-project", "p", "--name", "a.json"],
        vec![
            "server",
            "--load-project",
            "p",
            "--name",
            "a.json",
            "--out",
            "x.json",
            "--tag",
            "t",
        ],
        // the options of the store belong to the store
        vec!["server", "--tag", "t", &path],
        vec!["server", "--health", "--name", "a.json"],
        vec!["server", "--save-project", "p", "--send", &path],
        // a version is a whole number from 1
        vec!["server", "--load-project", "p", "--version", "x"],
        vec!["server", "--load-project", "p", "--version"],
    ] {
        let mut full = args.clone();
        full.extend(["--config", on.as_str()]);
        let (code, out, err) = command(&full);
        assert_eq!((code, out.as_str()), (2, ""), "{args:?}");
        assert!(err.contains("PHOENIX-SERVER-002"), "{args:?}: {err}");
    }
    // names that cannot be a project or a file of the store never reach the server
    for args in [
        vec!["server", "--save-project", "a b", &path],
        vec!["server", "--save-project", "p", "--name", "../x", &path],
        vec!["server", "--save-project", "p", "--name", "a b.json", &path],
    ] {
        let mut full = args.clone();
        full.extend(["--config", on.as_str()]);
        let (code, _, err) = command(&full);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.contains("PHOENIX-SERVER-017"), "{args:?}: {err}");
    }
    // a file to write that exists is never replaced
    let (code, _, err) = command(&[
        "server",
        "--load-project",
        "p",
        "--name",
        "a.json",
        "--out",
        existing,
        "--config",
        &on,
    ]);
    assert_eq!(code, 2);
    assert!(err.contains("PHOENIX-SERVER-018"), "{err}");
    assert_eq!(names(dir.path()), vec!["board.json", "on.cfg"]);
}
