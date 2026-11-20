// SPDX-License-Identifier: GPL-3.0-or-later
//! The whole way over a real connection: a small HTTP server on the loopback address of this
//! computer answers the routes of the work queue the way EH3D-SERVER documents them, and the
//! server mode talks to it with the plain HTTP transport of the client of the suite. Nothing
//! leaves the computer.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use eh3d_phoenix_rules::Lang;
use eh3d_phoenix_server::codes::SERVER_FAILED;
use eh3d_phoenix_server::{
    connect, dry_run, prepare_measurements, report_json, report_text, send, ServerSettings,
    SleepPause,
};
use eh3d_serverclient::Json;

const VIEW: &str = "{\"kind\":\"inspect\",\"verdict\":\"fail\",\"errors\":1,\"warnings\":0,\"measurements\":[{\"name\":\"width\",\"nominal\":1.6,\"value\":1.8,\"tolerance\":0.1,\"pass\":false,\"deviation\":0.2,\"unit\":\"mm\"}]}";

/// What the small server saw: the request line and the body of each request, in order.
type Seen = Arc<Mutex<Vec<(String, String)>>>;

fn read_request(stream: &mut TcpStream) -> Option<(String, String, String)> {
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
    let method = first.next()?.to_owned();
    let path = first.next()?.to_owned();
    let token_ok = head.contains("Bearer loopback-token");
    Some((
        format!(
            "{method} {path}{}",
            if token_ok { "" } else { " (no token)" }
        ),
        body,
        head,
    ))
}

fn answer(stream: &mut TcpStream, status: &str, body: &str) {
    let text = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(text.as_bytes());
}

/// Serves `connections` requests and returns the address and what was seen.
fn small_server(connections: usize, refuse: bool) -> (String, Seen, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let seen: Seen = Arc::default();
    let log = seen.clone();
    let handle = thread::spawn(move || {
        for _ in 0..connections {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let Some((line, body, _)) = read_request(&mut stream) else {
                continue;
            };
            log.lock().unwrap().push((line.clone(), body));
            if refuse {
                answer(
                    &mut stream,
                    "403 Forbidden",
                    "{\"error\":\"forbidden\",\"detail\":\"\"}",
                );
                continue;
            }
            match line.as_str() {
                "POST /api/v1/work/jobs" => answer(
                    &mut stream,
                    "201 Created",
                    &format!("{{\"id\":\"w-1\",\"state\":\"queued\",\"view\":{VIEW}}}"),
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

fn settings(address: &str) -> ServerSettings {
    let mut s = ServerSettings {
        allow_network: true,
        poll_ms: 10,
        ..ServerSettings::default()
    };
    s.link.set("server_address", address).unwrap();
    s.link.set("server_token", "loopback-token").unwrap();
    s.link.set("server_machine", "pcb-1").unwrap();
    s.link.set("server_retries", "0").unwrap();
    s.link.set("server_timeout_s", "5").unwrap();
    s
}

#[test]
fn a_measurement_report_is_made_over_a_real_connection_and_read_back() {
    let (address, seen, handle) = small_server(3, false);
    let s = settings(&address);
    let prepared = prepare_measurements("width, 1.6, 0.1, 1.8, mm\n", &s).unwrap();
    let mut client = connect(&s);
    let report = send(
        &mut client,
        &mut SleepPause,
        &s,
        "board",
        prepared,
        Lang::En,
    )
    .unwrap();
    handle.join().unwrap();
    let outcome = report.outcome.as_ref().unwrap();
    assert_eq!(
        (outcome.state.as_str(), outcome.verdict.as_str()),
        ("done", "fail")
    );
    assert_eq!(outcome.problems.len(), 1);
    assert_eq!(outcome.problems[0].item, "width");
    assert!(report.has_errors());
    // the three requests went in order, each with the token, and the body of the first holds the
    // machine and the request
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
    let body = Json::parse(&seen[0].1).unwrap();
    assert_eq!(body.get("machine").and_then(Json::as_str), Some("pcb-1"));
    assert_eq!(
        body.get("request")
            .and_then(|r| r.get("kind"))
            .and_then(Json::as_str),
        Some("inspect")
    );
    // the text and the JSON of the report never hold the token
    let text = report_text(&report, 10, Lang::En);
    let json = report_json(&report, 10);
    for out in [&text, &json] {
        assert!(!out.contains("loopback-token"), "{out}");
    }
    assert!(text.contains("width"));
}

#[test]
fn a_refusal_of_the_server_is_reported_by_its_code_in_the_language_asked() {
    let (address, _, handle) = small_server(1, true);
    let s = settings(&address);
    let prepared = prepare_measurements("width, 1.6, 0.1, 1.8\n", &s).unwrap();
    let mut client = connect(&s);
    let e = send(&mut client, &mut SleepPause, &s, "b", prepared, Lang::Es).unwrap_err();
    handle.join().unwrap();
    assert_eq!(e.code, SERVER_FAILED);
    assert!(!e.detail.is_empty() && !e.detail.contains("loopback-token"));
}

#[test]
fn a_server_that_is_not_listening_is_reported_and_a_dry_run_needs_none() {
    // an address that was open a moment ago and is closed now
    let address = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().to_string()
    };
    let s = settings(&address);
    let prepared = prepare_measurements("width, 1.6, 0.1, 1.8\n", &s).unwrap();
    let report = dry_run(prepared.clone());
    assert!(!report.sent);
    let mut client = connect(&s);
    let e = send(&mut client, &mut SleepPause, &s, "b", prepared, Lang::En).unwrap_err();
    assert_eq!(e.code, SERVER_FAILED);
}
