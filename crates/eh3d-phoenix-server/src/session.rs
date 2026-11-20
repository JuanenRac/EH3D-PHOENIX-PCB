// SPDX-License-Identifier: GPL-3.0-or-later
//! The conversation with the server: the job is made on the machine of the plugin `pcb`
//! (`POST /api/v1/work/jobs`), started (`POST .../start`) and asked about (`GET .../{id}`) until
//! it ends; the report of the station is in the `view` of the job. The link itself (the token,
//! the retries with growing pauses, the time limits, the state of the connection and the refusals
//! of the server by their code in the language of the window) is the client of the suite,
//! `eh3d-serverclient`; this file adds only the job and its words.

use std::thread;
use std::time::Duration;

use eh3d_phoenix_rules::Lang;
use eh3d_serverclient as sc;
use eh3d_serverclient::json::quote;
use eh3d_serverclient::Json;

use crate::codes::{
    Note, ServerError, Severity, BAD_ANSWER, JOB_ENDED_BAD, JOB_TIMEOUT, NETWORK_NOT_ALLOWED,
    NOT_CONFIGURED, OPTION_INVALID, PLAIN_TOKEN, SERVER_FAILED,
};
use crate::settings::ServerSettings;

/// The name this program gives the server for its jobs.
pub const APP: &str = "phoenix-pcb";

/// How long to wait between two questions about the job.
pub trait Pause {
    /// Waits `ms` milliseconds.
    fn pause(&mut self, ms: u64);
}

/// The pause of a real run: the thread sleeps.
#[derive(Debug, Default, Clone, Copy)]
pub struct SleepPause;

impl Pause for SleepPause {
    fn pause(&mut self, ms: u64) {
        thread::sleep(Duration::from_millis(ms));
    }
}

/// One problem of the report of the station.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// `error` or `warning`.
    pub severity: String,
    /// The rule of the station (`track_width`, `drill_small`, ...).
    pub rule: String,
    /// The track, the hole, the part or the measurement.
    pub item: String,
    /// The sentence of the station.
    pub message: String,
}

/// What the server said of the job when it ended.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outcome {
    /// The id of the job on the server.
    pub job_id: String,
    /// `queued`, `running`, `paused`, `done`, `failed` or `cancelled`.
    pub state: String,
    /// The message of the job; may be empty.
    pub message: String,
    /// `pass` or `fail`; empty when the report has none.
    pub verdict: String,
    /// The errors of the report.
    pub errors: u64,
    /// The warnings of the report.
    pub warnings: u64,
    /// The problems the report lists.
    pub problems: Vec<Problem>,
    /// The figures of the report (holes, travel, parts...), as `(name, text)`.
    pub figures: Vec<(String, String)>,
    /// How many times the job was asked about.
    pub polls: u32,
    /// The kind of report the station made (`rules_check`, `drill_report`...); may be empty.
    pub kind: String,
}

impl Outcome {
    /// True when the job ended and its verdict is `pass`.
    pub fn passes(&self) -> bool {
        self.verdict == "pass" && self.errors == 0
    }
}

/// The language of the client that matches one of the program.
pub fn client_lang(lang: Lang) -> sc::Lang {
    sc::Lang::from_tag(lang.tag()).unwrap_or(sc::Lang::En)
}

/// Whether the settings allow to send, and the notes that go with it. Sending needs
/// `allow_network`, an address, a machine and a token; a token for an address that is not local
/// travels as plain text, which is said.
pub fn check_ready(settings: &ServerSettings) -> Result<Vec<Note>, ServerError> {
    if !settings.allow_network {
        return Err(ServerError::new(
            NETWORK_NOT_ALLOWED,
            "set allow_network = true in the settings and give --send",
        ));
    }
    let link = &settings.link;
    if !link.is_configured() {
        return Err(ServerError::new(NOT_CONFIGURED, "server_address is empty"));
    }
    if link.server_machine.is_empty() {
        return Err(ServerError::new(NOT_CONFIGURED, "server_machine is empty"));
    }
    if link.server_token.is_empty() {
        return Err(ServerError::new(
            NOT_CONFIGURED,
            "server_token is empty: make a token for this program on the server",
        ));
    }
    let mut notes = Vec::new();
    if sc::plain_token_warning(&link.server_address) {
        notes.push(Note::new(
            PLAIN_TOKEN,
            Severity::Warning,
            "use a tunnel (SSH, VPN) or a network you trust",
        ));
    }
    Ok(notes)
}

fn failed(error: &sc::ClientError, lang: Lang) -> ServerError {
    ServerError::new(SERVER_FAILED, error.text(client_lang(lang)))
}

fn text_of(value: Option<&Json>) -> String {
    value.and_then(Json::as_str).unwrap_or("").to_owned()
}

fn number_text(value: &Json) -> Option<String> {
    match value {
        Json::Num(n) if n.is_finite() => Some(format!("{n}")),
        _ => None,
    }
}

/// Reads the report of the station out of the `view` of a job.
fn read_view(view: &Json, outcome: &mut Outcome) {
    outcome.kind = text_of(view.get("kind"));
    outcome.verdict = text_of(view.get("verdict"));
    outcome.errors = view.get("errors").and_then(Json::as_u64).unwrap_or(0);
    outcome.warnings = view.get("warnings").and_then(Json::as_u64).unwrap_or(0);
    for key in ["violations", "issues"] {
        for row in view.get(key).and_then(Json::as_array).unwrap_or(&[]) {
            outcome.problems.push(Problem {
                severity: text_of(row.get("severity")),
                rule: text_of(row.get("rule")),
                item: text_of(row.get("item")),
                message: text_of(row.get("message")),
            });
        }
    }
    for row in view
        .get("measurements")
        .and_then(Json::as_array)
        .unwrap_or(&[])
    {
        if row.get("pass").and_then(Json::as_bool) == Some(false) {
            let part = |key: &str| row.get(key).and_then(number_text).unwrap_or_default();
            outcome.problems.push(Problem {
                severity: "error".to_owned(),
                rule: "measurement".to_owned(),
                item: text_of(row.get("name")),
                message: format!(
                    "value {} against {} with a tolerance of {}",
                    part("value"),
                    part("nominal"),
                    part("tolerance")
                ),
            });
        }
    }
    for key in ["hits", "travel_mm", "estimate_s", "lines", "parts"] {
        if let Some(text) = view.get(key).and_then(number_text) {
            outcome.figures.push((key.to_owned(), text));
        }
    }
    if let Some(tools) = view.get("tools").and_then(Json::as_array) {
        outcome
            .figures
            .push(("tools".to_owned(), tools.len().to_string()));
    }
}

fn job_of(reply: &sc::Reply) -> Result<Json, ServerError> {
    let bad = |why: &str| ServerError::new(BAD_ANSWER, why.to_owned());
    let json = reply.json().ok_or_else(|| bad("the answer is not JSON"))?;
    let job = match json.get("job") {
        Some(inner) => inner.clone(),
        None => json,
    };
    if job.get("id").is_none() && job.get("state").is_none() {
        return Err(bad("the answer holds no job"));
    }
    Ok(job)
}

fn finished(state: &str) -> bool {
    matches!(state, "done" | "failed" | "cancelled")
}

/// Reads what the server says of a job: its state, its message and the report in its `view`. A job
/// that ended without a report is an error when `must_end` is true.
fn outcome_of(job: &Json, id: &str, polls: u32, must_end: bool) -> Result<Outcome, ServerError> {
    let mut outcome = Outcome {
        job_id: id.to_owned(),
        state: text_of(job.get("state")),
        message: text_of(job.get("message")),
        polls,
        ..Outcome::default()
    };
    match job.get("view") {
        Some(view) if matches!(view, Json::Obj(_)) => read_view(view, &mut outcome),
        _ if outcome.state != "done" && must_end => {
            return Err(ServerError::new(
                JOB_ENDED_BAD,
                format!("{}: {}", outcome.state, outcome.message),
            ))
        }
        _ => {}
    }
    Ok(outcome)
}

/// Whether a text can be the id of a job: letters, digits, `_` and `-`, up to 64.
pub fn job_id_is_safe(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Reads a job that is already on the server, once, and gives what it says: the state, and the
/// report when there is one. A job that has not ended is not an error: its state says so.
pub fn read_job(client: &mut sc::Client, id: &str, lang: Lang) -> Result<Outcome, ServerError> {
    if !job_id_is_safe(id) {
        return Err(ServerError::new(
            OPTION_INVALID,
            "the id of a job is letters, digits, _ and - up to 64 characters",
        ));
    }
    let reply = client
        .get(&format!("/api/v1/work/jobs/{}", sc::path_segment(id)))
        .map_err(|e| failed(&e, lang))?;
    let job = job_of(&reply)?;
    outcome_of(&job, id, 1, false)
}

/// What `GET /api/v1/health` says of the server. It needs no login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// The name the server gives itself.
    pub name: String,
    /// Its version.
    pub version: String,
    /// `ok` when it is well.
    pub status: String,
    /// The revision of its API.
    pub api_revision: u64,
}

/// Whether the settings allow to reach the server at all: the switch and an address. Nothing else
/// is needed to ask whether it is there.
pub fn check_reachable(settings: &ServerSettings) -> Result<(), ServerError> {
    if !settings.allow_network {
        return Err(ServerError::new(
            NETWORK_NOT_ALLOWED,
            "set allow_network = true in the settings",
        ));
    }
    if !settings.link.is_configured() {
        return Err(ServerError::new(NOT_CONFIGURED, "server_address is empty"));
    }
    Ok(())
}

/// Asks the server whether it is there and well (`GET /api/v1/health`, with its retries and time
/// limit). No token is sent.
pub fn probe(client: &mut sc::Client, lang: Lang) -> Result<Probe, ServerError> {
    let h = client.health().map_err(|e| failed(&e, lang))?;
    Ok(Probe {
        name: h.name,
        version: h.version,
        status: h.status,
        api_revision: h.api_revision,
    })
}

/// Makes the job on the server, starts it (when the settings say so), asks about it until it
/// ends and reads its report. A job that does not end within `max_polls` questions is stopped on
/// the server and reported as timed out.
pub fn run_job(
    client: &mut sc::Client,
    pause: &mut dyn Pause,
    settings: &ServerSettings,
    name: &str,
    request: &str,
    lang: Lang,
) -> Result<Outcome, ServerError> {
    let body = format!(
        "{{\"machine\":{},\"request\":{request},\"name\":{},\"app\":{}}}",
        quote(&settings.link.server_machine),
        quote(name),
        quote(APP)
    );
    let made = client
        .post("/api/v1/work/jobs", Some(&body))
        .map_err(|e| failed(&e, lang))?;
    let created = job_of(&made)?;
    let id = created
        .get("id")
        .and_then(Json::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| ServerError::new(BAD_ANSWER, "the job has no id"))?
        .to_owned();
    let path = format!("/api/v1/work/jobs/{}", sc::path_segment(&id));
    if settings.start_job {
        client
            .post(&format!("{path}/start"), None)
            .map_err(|e| failed(&e, lang))?;
    }
    let mut polls = 0u32;
    loop {
        polls += 1;
        let reply = client.get(&path).map_err(|e| failed(&e, lang))?;
        let job = job_of(&reply)?;
        let state = text_of(job.get("state"));
        if finished(&state) || !settings.start_job {
            return outcome_of(&job, &id, polls, settings.start_job);
        }
        if polls >= settings.max_polls {
            // the job is stopped so that it does not stay on the machine; if this fails too the
            // job is still reported as timed out
            let _ = client.post(&format!("{path}/cancel"), None);
            return Err(ServerError::new(
                JOB_TIMEOUT,
                format!("job {id} was asked about {polls} times and is {state}"),
            ));
        }
        pause.pause(settings.poll_ms);
    }
}

/// A client that talks plain HTTP to the address of the settings.
pub fn connect(settings: &ServerSettings) -> sc::Client {
    sc::Client::connect(settings.link.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_serverclient::sim::SimServer;
    use eh3d_serverclient::{Reply, Request, Transport, TransportError};
    use std::sync::{Arc, Mutex};

    /// The routes of the work queue of the server, in memory, over the double of the client.
    #[derive(Default)]
    struct Script {
        bodies: Vec<String>,
        seen: Vec<String>,
        started: bool,
        cancelled: bool,
        polls: usize,
        running_polls: usize,
        final_state: &'static str,
        message: &'static str,
        view: String,
        down: bool,
        garbage: bool,
        no_id: bool,
    }

    #[derive(Clone)]
    struct Double(Arc<Mutex<Script>>);

    impl Double {
        fn new(view: &str) -> Double {
            Double(Arc::new(Mutex::new(Script {
                final_state: "done",
                view: view.to_owned(),
                ..Script::default()
            })))
        }

        fn with(&self, f: impl FnOnce(&mut Script)) {
            f(&mut self.0.lock().unwrap());
        }

        fn seen(&self) -> Vec<String> {
            self.0.lock().unwrap().seen.clone()
        }
    }

    fn reply(status: u16, body: &str) -> Result<Reply, TransportError> {
        Ok(Reply {
            status,
            body: body.to_owned(),
        })
    }

    impl Transport for Double {
        fn send(&self, req: &Request) -> Result<Reply, TransportError> {
            let mut st = self.0.lock().unwrap();
            st.seen.push(format!("{} {}", req.method, req.path));
            if st.down {
                return Err(TransportError::Connect("down".to_owned()));
            }
            if req.method == "GET" && req.path == "/api/v1/health" {
                return reply(
                    200,
                    "{\"name\":\"eh3d-server\",\"version\":\"0.9.1\",\"api_revision\":1,\"status\":\"ok\"}",
                );
            }
            if req.bearer() != Some("program-token") {
                return reply(401, "{\"error\":\"unauthorized\",\"detail\":\"\"}");
            }
            if st.garbage {
                return reply(200, "not json");
            }
            let path = req.path.as_str();
            match (req.method.as_str(), path) {
                ("POST", "/api/v1/work/jobs") => {
                    st.bodies.push(req.body.clone().unwrap_or_default());
                    if st.no_id {
                        return reply(201, "{\"ok\":true}");
                    }
                    reply(201, "{\"id\":\"w-1\",\"state\":\"queued\",\"view\":null}")
                }
                ("POST", "/api/v1/work/jobs/w-1/start") => {
                    st.started = true;
                    reply(200, "{\"id\":\"w-1\",\"state\":\"running\"}")
                }
                ("POST", "/api/v1/work/jobs/w-1/cancel") => {
                    st.cancelled = true;
                    reply(200, "{}")
                }
                ("GET", "/api/v1/work/jobs/w-1") => {
                    st.polls += 1;
                    let state = if !st.started {
                        "queued"
                    } else if st.polls <= st.running_polls {
                        "running"
                    } else {
                        st.final_state
                    };
                    // the view of a job is there from the moment it is made, and the double
                    // shows it again when the job has ended
                    let view = if state == "running" || st.view.is_empty() {
                        "null".to_owned()
                    } else {
                        st.view.clone()
                    };
                    reply(
                        200,
                        &format!(
                            "{{\"job\":{{\"id\":\"w-1\",\"state\":\"{state}\",\"message\":\"{}\",\"view\":{view}}}}}",
                            st.message
                        ),
                    )
                }
                _ => reply(404, "{\"error\":\"not_found\",\"detail\":\"\"}"),
            }
        }
    }

    const FAIL_VIEW: &str = "{\"kind\":\"rules_check\",\"verdict\":\"fail\",\"errors\":2,\"warnings\":1,\"violations\":[{\"rule\":\"track_width\",\"severity\":\"error\",\"item\":\"GND top\",\"message\":\"GND top is 0.1 mm wide\"},{\"rule\":\"drill_large\",\"severity\":\"warning\",\"item\":\"J1.1\",\"message\":\"J1.1 is 7 mm\"}]}";
    const PASS_VIEW: &str = "{\"kind\":\"drill_report\",\"hits\":12,\"tools\":[{\"dia_mm\":0.3},{\"dia_mm\":1}],\"travel_mm\":84.5,\"estimate_s\":6.5,\"verdict\":\"pass\",\"errors\":0,\"warnings\":0}";

    fn settings() -> ServerSettings {
        let mut s = ServerSettings {
            allow_network: true,
            ..ServerSettings::default()
        };
        s.link.set("server_address", "127.0.0.1:18200").unwrap();
        s.link.set("server_token", "program-token").unwrap();
        s.link.set("server_machine", "pcb-1").unwrap();
        s
    }

    fn client(double: &Double, s: &ServerSettings) -> (sc::Client, SimServer) {
        let sim = SimServer::new();
        let c = sc::Client::new(s.link.clone(), Box::new(double.clone()))
            .with_sleeper(Box::new(sim.sleeper()));
        (c, sim)
    }

    #[derive(Default)]
    struct Counting(Vec<u64>);

    impl Pause for Counting {
        fn pause(&mut self, ms: u64) {
            self.0.push(ms);
        }
    }

    const REQUEST: &str = "{\"kind\":\"rules_check\",\"board\":{}}";

    #[test]
    fn a_job_is_made_started_followed_and_its_report_is_read() {
        let double = Double::new(FAIL_VIEW);
        double.with(|s| s.running_polls = 2);
        let s = settings();
        let (mut c, _) = client(&double, &s);
        let mut pause = Counting::default();
        let o = run_job(&mut c, &mut pause, &s, "demo", REQUEST, Lang::En).unwrap();
        assert_eq!((o.state.as_str(), o.verdict.as_str()), ("done", "fail"));
        assert_eq!((o.errors, o.warnings, o.polls), (2, 1, 3));
        assert!(!o.passes());
        assert_eq!(o.problems.len(), 2);
        assert_eq!(o.problems[0].rule, "track_width");
        assert_eq!(o.problems[1].severity, "warning");
        // two questions that found it running, each followed by a pause of the settings
        assert_eq!(pause.0, vec![300, 300]);
        assert_eq!(
            double.seen(),
            vec![
                "POST /api/v1/work/jobs",
                "POST /api/v1/work/jobs/w-1/start",
                "GET /api/v1/work/jobs/w-1",
                "GET /api/v1/work/jobs/w-1",
                "GET /api/v1/work/jobs/w-1",
            ]
        );
    }

    #[test]
    fn the_job_carries_the_machine_the_name_the_application_and_the_request_and_never_the_token() {
        let double = Double::new(PASS_VIEW);
        let s = settings();
        let (mut c, _) = client(&double, &s);
        run_job(
            &mut c,
            &mut Counting::default(),
            &s,
            "board a",
            REQUEST,
            Lang::En,
        )
        .unwrap();
        let body = double.0.lock().unwrap().bodies[0].clone();
        let j = Json::parse(&body).unwrap();
        assert_eq!(j.get("machine").and_then(Json::as_str), Some("pcb-1"));
        assert_eq!(j.get("name").and_then(Json::as_str), Some("board a"));
        assert_eq!(j.get("app").and_then(Json::as_str), Some(APP));
        assert_eq!(
            j.get("request").unwrap().get("kind").and_then(Json::as_str),
            Some("rules_check")
        );
        assert!(!body.contains("program-token"));
    }

    #[test]
    fn a_report_without_violations_passes_and_its_figures_are_kept() {
        let double = Double::new(PASS_VIEW);
        let s = settings();
        let (mut c, _) = client(&double, &s);
        let o = run_job(&mut c, &mut Counting::default(), &s, "d", REQUEST, Lang::En).unwrap();
        assert!(o.passes());
        assert_eq!(o.polls, 1);
        assert!(o.figures.contains(&("hits".to_owned(), "12".to_owned())));
        assert!(o
            .figures
            .contains(&("travel_mm".to_owned(), "84.5".to_owned())));
        assert!(o.figures.contains(&("tools".to_owned(), "2".to_owned())));
    }

    #[test]
    fn failed_measurements_are_listed_as_problems() {
        let view = "{\"verdict\":\"fail\",\"errors\":1,\"warnings\":0,\"measurements\":[{\"name\":\"width\",\"nominal\":1.6,\"value\":1.8,\"tolerance\":0.1,\"pass\":false},{\"name\":\"bow\",\"nominal\":0,\"value\":0.1,\"tolerance\":0.5,\"pass\":true}]}";
        let double = Double::new(view);
        let s = settings();
        let (mut c, _) = client(&double, &s);
        let o = run_job(&mut c, &mut Counting::default(), &s, "d", REQUEST, Lang::En).unwrap();
        assert_eq!(o.problems.len(), 1);
        assert_eq!(o.problems[0].item, "width");
        assert_eq!(
            o.problems[0].message,
            "value 1.8 against 1.6 with a tolerance of 0.1"
        );
    }

    #[test]
    fn a_job_that_is_not_started_is_read_once_with_the_report_it_has() {
        let double = Double::new(PASS_VIEW);
        let mut s = settings();
        s.start_job = false;
        let (mut c, _) = client(&double, &s);
        let o = run_job(&mut c, &mut Counting::default(), &s, "d", REQUEST, Lang::En).unwrap();
        assert_eq!(o.state, "queued");
        assert_eq!(o.verdict, "pass");
        assert!(!double.seen().iter().any(|r| r.ends_with("/start")));
        assert!(!double.0.lock().unwrap().started);
    }

    #[test]
    fn a_job_that_does_not_end_is_stopped_on_the_server_and_reported_as_timed_out() {
        let double = Double::new(PASS_VIEW);
        double.with(|s| s.running_polls = usize::MAX);
        let mut s = settings();
        s.max_polls = 3;
        let (mut c, _) = client(&double, &s);
        let mut pause = Counting::default();
        let e = run_job(&mut c, &mut pause, &s, "d", REQUEST, Lang::En).unwrap_err();
        assert_eq!(e.code, JOB_TIMEOUT);
        assert!(
            e.detail.contains("w-1") && e.detail.contains('3'),
            "{}",
            e.detail
        );
        assert!(double.0.lock().unwrap().cancelled);
        assert_eq!(pause.0.len(), 2);
    }

    #[test]
    fn a_job_that_failed_with_a_report_is_an_outcome_and_without_one_is_an_error() {
        let double = Double::new(FAIL_VIEW);
        double.with(|s| {
            s.final_state = "failed";
            s.message = "the report has errors";
        });
        let s = settings();
        let (mut c, _) = client(&double, &s);
        let o = run_job(&mut c, &mut Counting::default(), &s, "d", REQUEST, Lang::En).unwrap();
        assert_eq!(o.state, "failed");
        assert_eq!(o.message, "the report has errors");
        assert_eq!(o.verdict, "fail");

        let lost = Double::new("");
        lost.with(|s| {
            s.final_state = "cancelled";
            s.message = "stopped";
        });
        let (mut c, _) = client(&lost, &s);
        let e = run_job(&mut c, &mut Counting::default(), &s, "d", REQUEST, Lang::En).unwrap_err();
        assert_eq!(e.code, JOB_ENDED_BAD);
        assert!(e.detail.contains("cancelled"), "{}", e.detail);
    }

    #[test]
    fn a_server_that_is_not_there_is_tried_again_and_then_reported_in_the_language_asked() {
        let double = Double::new(PASS_VIEW);
        double.with(|s| s.down = true);
        let s = settings();
        let (mut c, sim) = client(&double, &s);
        let e = run_job(&mut c, &mut Counting::default(), &s, "d", REQUEST, Lang::Es).unwrap_err();
        assert_eq!(e.code, SERVER_FAILED);
        assert!(
            e.detail.starts_with("No se puede llegar al servidor"),
            "{}",
            e.detail
        );
        // the request that makes the job is sent again with growing pauses, and nothing more
        assert_eq!(sim.pauses().len(), 2);
        assert_eq!(double.seen().len(), 3);
    }

    #[test]
    fn a_token_the_server_refuses_is_reported_and_is_not_in_the_text() {
        let double = Double::new(PASS_VIEW);
        let mut s = settings();
        s.link.set("server_token", "wrong-token-value").unwrap();
        let (mut c, _) = client(&double, &s);
        let e = run_job(&mut c, &mut Counting::default(), &s, "d", REQUEST, Lang::En).unwrap_err();
        assert_eq!(e.code, SERVER_FAILED);
        assert!(!e.detail.is_empty());
        assert!(!e.detail.contains("wrong-token-value"));
    }

    #[test]
    fn an_answer_that_is_not_a_job_is_a_bad_answer() {
        let garbage = Double::new(PASS_VIEW);
        garbage.with(|s| s.garbage = true);
        let s = settings();
        let (mut c, _) = client(&garbage, &s);
        let e = run_job(&mut c, &mut Counting::default(), &s, "d", REQUEST, Lang::En).unwrap_err();
        assert_eq!(e.code, BAD_ANSWER);

        let no_id = Double::new(PASS_VIEW);
        no_id.with(|s| s.no_id = true);
        let (mut c, _) = client(&no_id, &s);
        let e = run_job(&mut c, &mut Counting::default(), &s, "d", REQUEST, Lang::En).unwrap_err();
        assert_eq!(e.code, BAD_ANSWER);
    }

    #[test]
    fn sending_needs_the_switch_the_address_the_machine_and_the_token() {
        let mut s = settings();
        assert!(check_ready(&s).unwrap().is_empty());
        s.allow_network = false;
        assert_eq!(check_ready(&s).unwrap_err().code, NETWORK_NOT_ALLOWED);
        for (key, why) in [
            ("server_address", "server_address"),
            ("server_machine", "server_machine"),
            ("server_token", "server_token"),
        ] {
            let mut t = settings();
            t.link.set(key, "").unwrap();
            let e = check_ready(&t).unwrap_err();
            assert_eq!(e.code, NOT_CONFIGURED);
            assert!(e.detail.contains(why), "{}", e.detail);
        }
    }

    #[test]
    fn a_token_for_an_address_that_is_not_local_is_warned_about() {
        let mut s = settings();
        s.link.set("server_address", "192.168.0.180:18200").unwrap();
        let notes = check_ready(&s).unwrap();
        assert_eq!(
            notes.len(),
            usize::from(sc::plain_token_warning("192.168.0.180:18200"))
        );
        for note in &notes {
            assert_eq!(note.code, PLAIN_TOKEN);
        }
        s.link.set("server_address", "localhost:18200").unwrap();
        assert!(check_ready(&s).unwrap().is_empty());
    }

    #[test]
    fn a_job_that_is_on_the_server_is_read_once_whatever_its_state() {
        let double = Double::new(PASS_VIEW);
        let s = settings();
        let (mut c, _) = client(&double, &s);
        // not started: the double says `queued` and shows the report it made when it was prepared
        let o = read_job(&mut c, "w-1", Lang::En).unwrap();
        assert_eq!((o.state.as_str(), o.verdict.as_str()), ("queued", "pass"));
        assert_eq!((o.kind.as_str(), o.polls), ("drill_report", 1));
        double.with(|s| s.started = true);
        let o = read_job(&mut c, "w-1", Lang::En).unwrap();
        assert_eq!(o.state, "done");
        assert_eq!(
            double.seen(),
            vec!["GET /api/v1/work/jobs/w-1", "GET /api/v1/work/jobs/w-1"]
        );
        // a job that is still running has no report and is not an error
        let running = Double::new(PASS_VIEW);
        running.with(|s| {
            s.started = true;
            s.running_polls = usize::MAX;
        });
        let (mut c, _) = client(&running, &s);
        let o = read_job(&mut c, "w-1", Lang::En).unwrap();
        assert_eq!((o.state.as_str(), o.verdict.as_str()), ("running", ""));
        assert!(!o.passes());
    }

    #[test]
    fn the_id_of_a_job_that_could_change_the_path_is_refused_before_anything_is_sent() {
        let double = Double::new(PASS_VIEW);
        let s = settings();
        let (mut c, _) = client(&double, &s);
        for bad in ["", "a/b", "../x", "w 1", "w-1?x=1", "é", &"a".repeat(65)] {
            let e = read_job(&mut c, bad, Lang::En).unwrap_err();
            assert_eq!(e.code, OPTION_INVALID, "{bad:?}");
        }
        assert!(double.seen().is_empty());
        assert!(job_id_is_safe("w-1_A") && !job_id_is_safe("w.1"));
    }

    #[test]
    fn the_server_is_asked_whether_it_is_there_without_a_token() {
        let double = Double::new(PASS_VIEW);
        let mut s = settings();
        s.link.set("server_token", "").unwrap();
        let (mut c, _) = client(&double, &s);
        let p = probe(&mut c, Lang::En).unwrap();
        assert_eq!(
            (p.name.as_str(), p.version.as_str(), p.status.as_str()),
            ("eh3d-server", "0.9.1", "ok")
        );
        assert_eq!(p.api_revision, 1);
        let down = Double::new(PASS_VIEW);
        down.with(|s| s.down = true);
        let (mut c, _) = client(&down, &s);
        let e = probe(&mut c, Lang::Pt).unwrap_err();
        assert_eq!(e.code, SERVER_FAILED);
        assert!(
            e.detail.starts_with("Não é possível chegar ao servidor"),
            "{}",
            e.detail
        );
    }

    #[test]
    fn reaching_the_server_needs_the_switch_and_an_address_and_no_token() {
        let mut s = settings();
        s.link.set("server_token", "").unwrap();
        s.link.set("server_machine", "").unwrap();
        assert!(check_reachable(&s).is_ok());
        s.allow_network = false;
        assert_eq!(check_reachable(&s).unwrap_err().code, NETWORK_NOT_ALLOWED);
        let mut t = settings();
        t.link.set("server_address", "").unwrap();
        assert_eq!(check_reachable(&t).unwrap_err().code, NOT_CONFIGURED);
    }

    #[test]
    fn the_language_of_the_client_follows_the_language_of_the_program() {
        for lang in Lang::ALL {
            assert_eq!(client_lang(lang), sc::Lang::from_tag(lang.tag()).unwrap());
        }
        assert_eq!(client_lang(Lang::Pt), sc::Lang::Pt);
    }
}
