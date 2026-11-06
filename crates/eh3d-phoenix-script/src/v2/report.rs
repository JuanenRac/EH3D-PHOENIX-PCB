// SPDX-License-Identifier: GPL-3.0-or-later
//! The report of a run of a script of the second version, as text and as JSON. It holds what the
//! run counted and wrote and nothing that depends on the computer or on the time, so the same
//! script on the same document always gives the same report, byte for byte.

use eh3d_phoenix_rules::Lang;
use serde_json::json;

use crate::v2::interp::{Outcome, Plan};

impl Outcome {
    /// How many steps of the history the run is: one when it changed the document, none when it
    /// did not.
    pub fn history_steps(&self) -> usize {
        match self.plan {
            Plan::None => 0,
            _ => 1,
        }
    }

    /// The report as text; the diagnostics carry their titles in the language.
    pub fn to_text(&self, lang: Lang) -> String {
        let mut out = String::new();
        out.push_str(if self.ok {
            "result: carried out\n"
        } else {
            "result: stopped, nothing changed\n"
        });
        out.push_str(&format!(
            "steps: {}; operations: {}; history steps: {}\n",
            self.steps,
            self.operations,
            self.history_steps()
        ));
        for line in &self.output {
            out.push_str(line);
            out.push('\n');
        }
        for d in &self.diagnostics {
            out.push_str(&format!("error: {}\n", d.message(lang)));
        }
        for w in &self.writes {
            out.push_str(&format!("file: {} ({} bytes)\n", w.relative, w.bytes.len()));
        }
        out
    }

    /// The report as JSON (`phoenix-script-run/2`).
    pub fn to_json(&self) -> String {
        let diagnostics: Vec<serde_json::Value> = self
            .diagnostics
            .iter()
            .map(|d| {
                json!({
                    "code": d.code,
                    "line": d.line,
                    "column": d.col,
                    "detail": d.detail,
                })
            })
            .collect();
        let files: Vec<serde_json::Value> = self
            .writes
            .iter()
            .map(|w| json!({ "path": w.relative, "bytes": w.bytes.len() }))
            .collect();
        let value = json!({
            "schema": "phoenix-script-run/2",
            "api": crate::v2::syntax::API,
            "ok": self.ok,
            "steps": self.steps,
            "operations": self.operations,
            "history_steps": self.history_steps(),
            "replaced": matches!(self.plan, Plan::Replaced),
            "output": self.output,
            "diagnostics": diagnostics,
            "files": files,
        });
        serde_json::to_string_pretty(&value).unwrap_or_default() + "\n"
    }
}
