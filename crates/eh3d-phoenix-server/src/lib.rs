// SPDX-License-Identifier: GPL-3.0-or-later
//! The server mode: a board, or a file of measurements, is sent to EH3D-SERVER as a job of its
//! plugin `pcb` (`rules_check`, `drill_report`, `bom_report` or `inspect`) and the report of the
//! station is read back. The program works alone as it did before; this is another way to get a
//! report, through the server that the machines and the other programs of the suite share.
//!
//! * [`settings`]: the keys of the link (`server_address`, `server_token`, `server_machine`,
//!   `server_timeout_s`, `server_retries`, `server_mdns`), whether anything may be sent and every
//!   limit, read from `config/server.cfg`;
//! * [`request`]: a board reduced to plain facts and written as the JSON text of the request;
//! * [`session`]: the job on the server, over the client of the suite (`eh3d-serverclient`);
//! * [`report`]: the text, with its words in the eight languages, and the JSON of a report;
//! * [`codes`]: the codes `PHOENIX-SERVER-001` to `PHOENIX-SERVER-014` with their titles in the
//!   eight languages.
//!
//! **State: prototype.** It has been tried only against a server in memory that answers the way
//! the server is documented to answer; it has never talked to a running EH3D-SERVER. Nothing is
//! sent unless the settings allow it and the command asks for it.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod codes;
pub mod report;
pub mod request;
pub mod session;
pub mod settings;

use eh3d_phoenix_board::Board;

pub use codes::{title, Note, ServerError, Severity};
pub use report::{report_json, report_text, ServerReport, FORMAT};
pub use request::{facts_from_board, parse_measurements, Facts, Kind, Measurement};
pub use session::{check_ready, connect, run_job, Outcome, Pause, SleepPause, APP};
pub use settings::ServerSettings;

/// A request that is ready to be shown or sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    /// The kind of report asked of the station.
    pub kind: Kind,
    /// The JSON text of the request.
    pub request: String,
    /// How many items (tracks, holes, parts...) it holds.
    pub items: usize,
    /// What was left out or approximated.
    pub notes: Vec<Note>,
}

/// Makes the request of a kind from a board.
pub fn prepare_board(
    kind: Kind,
    board: &Board,
    settings: &ServerSettings,
) -> Result<Prepared, ServerError> {
    let facts = facts_from_board(board, settings)?;
    let (request, items) = match kind {
        Kind::Rules => (
            request::rules_request(&facts, settings)?,
            facts.tracks.len() + facts.clearances.len() + facts.drills.len(),
        ),
        Kind::Drill => (
            request::drill_request(&facts, settings)?,
            facts.drills.len(),
        ),
        Kind::Bom => (request::bom_request(&facts, settings)?, facts.parts.len()),
        Kind::Inspect => {
            return Err(ServerError::new(
                codes::OPTION_INVALID,
                "the inspection takes a file of measurements, not a board",
            ))
        }
    };
    Ok(Prepared {
        kind,
        request,
        items,
        notes: facts.notes,
    })
}

/// Makes the request `inspect` from the text of a file of measurements.
pub fn prepare_measurements(
    text: &str,
    settings: &ServerSettings,
) -> Result<Prepared, ServerError> {
    let rows = parse_measurements(text)?;
    let request = request::inspect_request(&rows, settings)?;
    Ok(Prepared {
        kind: Kind::Inspect,
        request,
        items: rows.len(),
        notes: Vec::new(),
    })
}

/// A dry run: the request is shown and nothing is sent.
pub fn dry_run(prepared: Prepared) -> ServerReport {
    let mut notes = prepared.notes;
    notes.push(Note::new(codes::DRY_RUN, Severity::Info, ""));
    ServerReport {
        kind: prepared.kind,
        sent: false,
        request: prepared.request,
        items: prepared.items,
        notes,
        outcome: None,
    }
}

/// Sends the request over a client already made and reads the report of the station. The
/// settings must allow it ([`check_ready`]).
pub fn send(
    client: &mut eh3d_serverclient::Client,
    pause: &mut dyn Pause,
    settings: &ServerSettings,
    name: &str,
    prepared: Prepared,
    lang: eh3d_phoenix_rules::Lang,
) -> Result<ServerReport, ServerError> {
    let mut notes = prepared.notes;
    notes.extend(check_ready(settings)?);
    let outcome = run_job(client, pause, settings, name, &prepared.request, lang)?;
    Ok(ServerReport {
        kind: prepared.kind,
        sent: true,
        request: prepared.request,
        items: prepared.items,
        notes,
        outcome: Some(outcome),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_rules::Lang;
    use eh3d_serverclient as sc;
    use eh3d_serverclient::sim::SimServer;
    use eh3d_serverclient::Json;
    use eh3d_serverclient::{Reply, Request, Transport, TransportError};

    /// A station that makes the report of a rules check from the request itself, as the plugin
    /// does for the widths of the tracks, so the whole way (board, request, job, report) is run.
    struct Station;

    impl Transport for Station {
        fn send(&self, req: &Request) -> Result<Reply, TransportError> {
            let ok = |body: &str| {
                Ok(Reply {
                    status: 200,
                    body: body.to_owned(),
                })
            };
            match (req.method.as_str(), req.path.as_str()) {
                ("POST", "/api/v1/work/jobs") => {
                    let body = Json::parse(req.body.as_deref().unwrap_or("")).unwrap();
                    let request = body.get("request").unwrap();
                    let narrow = request
                        .get("board")
                        .and_then(|b| b.get("tracks"))
                        .and_then(Json::as_array)
                        .map_or(0, |tracks| {
                            tracks
                                .iter()
                                .filter(|t| matches!(t.get("width_mm"), Some(Json::Num(w)) if *w < 0.15))
                                .count()
                        });
                    let verdict = if narrow == 0 { "pass" } else { "fail" };
                    ok(&format!(
                        "{{\"id\":\"w-9\",\"state\":\"queued\",\"view\":{{\"verdict\":\"{verdict}\",\"errors\":{narrow},\"warnings\":0,\"violations\":[]}}}}"
                    ))
                }
                ("POST", "/api/v1/work/jobs/w-9/start") => ok("{\"state\":\"running\"}"),
                ("GET", "/api/v1/work/jobs/w-9") => ok("{\"id\":\"w-9\",\"state\":\"done\",\"view\":{\"verdict\":\"fail\",\"errors\":1,\"warnings\":0,\"violations\":[{\"rule\":\"track_width\",\"severity\":\"error\",\"item\":\"X\",\"message\":\"narrow\"}]}}"),
                _ => Ok(Reply {
                    status: 404,
                    body: "{\"error\":\"not_found\",\"detail\":\"\"}".to_owned(),
                }),
            }
        }
    }

    fn settings() -> ServerSettings {
        let mut s = ServerSettings {
            allow_network: true,
            ..ServerSettings::default()
        };
        s.link.set("server_address", "127.0.0.1:18200").unwrap();
        s.link.set("server_token", "t").unwrap();
        s.link.set("server_machine", "pcb-1").unwrap();
        s
    }

    fn prepared() -> Prepared {
        prepare_measurements("width, 1.6, 0.1, 1.8, mm\n", &settings()).unwrap()
    }

    #[test]
    fn measurements_make_the_request_inspect_and_a_board_is_refused_for_it() {
        let p = prepared();
        assert_eq!((p.kind, p.items), (Kind::Inspect, 1));
        let j = Json::parse(&p.request).unwrap();
        assert_eq!(j.get("kind").and_then(Json::as_str), Some("inspect"));
        assert_eq!(
            prepare_measurements("", &settings()).unwrap_err().code,
            codes::NOTHING_TO_SEND
        );
        assert_eq!(
            prepare_measurements("bad line\n", &settings())
                .unwrap_err()
                .code,
            codes::INPUT_UNREADABLE
        );
    }

    #[test]
    fn a_dry_run_has_no_outcome_and_the_note_that_nothing_was_sent() {
        let report = dry_run(prepared());
        assert!(!report.sent && report.outcome.is_none());
        assert_eq!(report.notes.last().map(|n| n.code), Some(codes::DRY_RUN));
        assert!(!report.has_errors());
        let text = report_text(&report, 10, Lang::Es);
        assert!(text.contains("simulacro"));
        assert!(text.contains("PHOENIX-SERVER-012"));
    }

    #[test]
    fn the_whole_way_from_the_request_to_the_report_runs_over_the_client_of_the_suite() {
        let s = settings();
        let mut client = sc::Client::new(s.link.clone(), Box::new(Station))
            .with_sleeper(Box::new(SimServer::new().sleeper()));
        let report = send(
            &mut client,
            &mut SleepPause,
            &s,
            "demo",
            prepared(),
            Lang::En,
        )
        .unwrap();
        assert!(report.sent);
        let outcome = report.outcome.as_ref().unwrap();
        assert_eq!((outcome.state.as_str(), outcome.errors), ("done", 1));
        assert!(report.has_errors());
        let text = report_text(&report, 10, Lang::Pt);
        assert!(text.contains("reprovado"), "{text}");
        assert!(text.contains("track_width"));
    }

    #[test]
    fn nothing_is_sent_when_the_settings_do_not_allow_it() {
        let mut s = settings();
        s.allow_network = false;
        let mut client = sc::Client::new(s.link.clone(), Box::new(Station));
        let e = send(
            &mut client,
            &mut SleepPause,
            &s,
            "demo",
            prepared(),
            Lang::En,
        )
        .unwrap_err();
        assert_eq!(e.code, codes::NETWORK_NOT_ALLOWED);
    }
}
