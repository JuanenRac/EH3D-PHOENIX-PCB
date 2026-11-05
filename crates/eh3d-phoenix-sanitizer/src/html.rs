// SPDX-License-Identifier: GPL-3.0-or-later
//! The HTML report: one page with no external resource (the style and the small script that sorts
//! the tables are inline, and a content policy forbids everything else). Every name that comes
//! from a library is escaped, because a library file is hostile until proven otherwise.

use eh3d_phoenix_rules::Lang;

use crate::group::Group;
use crate::msg::{text, Msg};
use crate::report::Report;

const STYLE: &str = "\
:root{color-scheme:light dark;--bg:#fbfaf8;--fg:#1d1b19;--mut:#6b655e;--line:#d8d2c9;--head:#eee9e1;--acc:#a85a00}\
@media (prefers-color-scheme:dark){:root{--bg:#161412;--fg:#ece7df;--mut:#a39b90;--line:#3a352f;--head:#26221e;--acc:#e0a050}}\
body{margin:0;padding:24px 16px;background:var(--bg);color:var(--fg);font:15px/1.5 system-ui,sans-serif}\
main{max-width:1200px;margin:0 auto}\
h1{font-size:1.6em;margin:0 0 4px}h2{font-size:1.2em;margin:32px 0 8px}\
p.note{color:var(--mut);margin:0 0 16px}\
table{border-collapse:collapse;width:100%;margin:8px 0;font-size:14px}\
th,td{border:1px solid var(--line);padding:4px 8px;text-align:left;vertical-align:top;word-break:break-word}\
th{background:var(--head)}table.sortable th{cursor:pointer}\
td.num{text-align:right}tr.keep td{font-weight:600}.mono{font-family:ui-monospace,monospace;font-size:12px}\
.flag{color:var(--acc)}\
@media (max-width:640px){table{font-size:12px}th,td{padding:3px 4px}}";

const SCRIPT: &str = "\
document.querySelectorAll('table.sortable').forEach(function(t){\
Array.prototype.forEach.call(t.tHead.rows[0].cells,function(c,i){\
c.addEventListener('click',function(){\
var d=c.getAttribute('data-d')==='1'?-1:1;c.setAttribute('data-d',d===1?'1':'0');\
var rows=Array.prototype.slice.call(t.tBodies[0].rows);\
rows.sort(function(a,b){var x=a.cells[i].textContent.trim(),y=b.cells[i].textContent.trim(),\
nx=parseFloat(x),ny=parseFloat(y);\
if(!isNaN(nx)&&!isNaN(ny)&&String(nx)===x&&String(ny)===y){return (nx-ny)*d;}\
return x.localeCompare(y)*d;});\
rows.forEach(function(r){t.tBodies[0].appendChild(r);});\
});});});";

/// Escapes text for HTML content and attributes.
pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

fn lang_tag(lang: Lang) -> &'static str {
    if lang == Lang::Pt {
        "pt-PT"
    } else {
        lang.tag()
    }
}

fn cell(out: &mut String, value: &str) {
    out.push_str("<td>");
    out.push_str(&escape(value));
    out.push_str("</td>");
}

fn num_cell(out: &mut String, value: impl std::fmt::Display) {
    out.push_str(&format!("<td class=\"num\">{value}</td>"));
}

fn head(out: &mut String, labels: &[&str]) {
    out.push_str("<table class=\"sortable\"><thead><tr>");
    for l in labels {
        out.push_str(&format!("<th>{}</th>", escape(l)));
    }
    out.push_str("</tr></thead><tbody>");
}

fn yes_no(lang: Lang, value: bool) -> &'static str {
    text(if value { Msg::Yes } else { Msg::No }, lang)
}

fn kind_label(kind: &str, lang: Lang) -> &'static str {
    match kind {
        "package" => text(Msg::KindPackage, lang),
        "symbol" => text(Msg::KindSymbol, lang),
        _ => text(Msg::KindDeviceSet, lang),
    }
}

fn review_of(g: &Group) -> String {
    let mut parts = Vec::new();
    if g.weak {
        parts.push("below-threshold");
    }
    if !g.names_agree {
        parts.push("names-differ");
    }
    if g.members.iter().any(|m| m.turned) {
        parts.push("turned");
    }
    parts.join(" ")
}

fn groups_table(out: &mut String, report: &Report, kind: &str, lang: Lang) {
    let groups: Vec<&Group> = report.groups.iter().filter(|g| g.kind == kind).collect();
    if groups.is_empty() {
        return;
    }
    head(
        out,
        &[
            text(Msg::GroupId, lang),
            text(Msg::Score, lang),
            text(Msg::Exact, lang),
            text(Msg::NamesAgree, lang),
            text(Msg::Review, lang),
            text(Msg::Library, lang),
            text(Msg::Name, lang),
            text(Msg::Hash, lang),
            text(Msg::Turned, lang),
            text(Msg::Uses, lang),
            text(Msg::Keep, lang),
        ],
    );
    for g in groups {
        let review = review_of(g);
        for m in &g.members {
            out.push_str(if m.keep {
                "<tr class=\"keep\">"
            } else {
                "<tr>"
            });
            cell(out, &g.id);
            num_cell(out, m.score);
            cell(out, yes_no(lang, g.exact));
            cell(out, yes_no(lang, g.names_agree));
            out.push_str(&format!("<td class=\"flag\">{}</td>", escape(&review)));
            cell(out, &m.library);
            cell(out, &m.name);
            let short: String = m.hash.chars().take(16).collect();
            out.push_str(&format!(
                "<td class=\"mono\" title=\"{}\">{}</td>",
                escape(&m.hash),
                escape(&short)
            ));
            cell(out, yes_no(lang, m.turned));
            num_cell(out, m.uses);
            cell(out, if m.keep { yes_no(lang, true) } else { "" });
            out.push_str("</tr>");
        }
    }
    out.push_str("</tbody></table>");
}

/// The report as one self-contained HTML page in a language.
pub fn html_report(report: &Report, lang: Lang) -> String {
    let t = &report.totals;
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n");
    out.push_str(&format!(
        "<html lang=\"{}\"><head><meta charset=\"utf-8\">",
        lang_tag(lang)
    ));
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    out.push_str("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'\">");
    out.push_str(&format!(
        "<title>{}</title>",
        escape(text(Msg::Title, lang))
    ));
    out.push_str(&format!("<style>{STYLE}</style></head><body><main>"));
    out.push_str(&format!("<h1>{}</h1>", escape(text(Msg::Title, lang))));
    out.push_str(&format!(
        "<p class=\"note\">{}</p>",
        escape(text(Msg::Intro, lang))
    ));

    out.push_str("<table><tbody>");
    let row = |out: &mut String, label: &str, value: String| {
        out.push_str(&format!(
            "<tr><th>{}</th><td>{}</td></tr>",
            escape(label),
            escape(&value)
        ));
    };
    row(
        &mut out,
        text(Msg::FilesRead, lang),
        format!("{} / {}", t.read, t.files),
    );
    row(
        &mut out,
        text(Msg::Footprints, lang),
        t.packages.to_string(),
    );
    row(&mut out, text(Msg::Symbols, lang), t.symbols.to_string());
    row(
        &mut out,
        text(Msg::DeviceSets, lang),
        t.devicesets.to_string(),
    );
    row(
        &mut out,
        text(Msg::Groups, lang),
        format!("{} / {} / {}", t.groups[0], t.groups[1], t.groups[2]),
    );
    row(
        &mut out,
        text(Msg::Redundant, lang),
        format!(
            "{} / {} / {}",
            t.redundant[0], t.redundant[1], t.redundant[2]
        ),
    );
    row(
        &mut out,
        text(Msg::Threshold, lang),
        report.settings.threshold.to_string(),
    );
    out.push_str("</tbody></table>");

    out.push_str(&format!("<h2>{}</h2>", escape(text(Msg::Libraries, lang))));
    head(
        &mut out,
        &[
            text(Msg::Path, lang),
            text(Msg::Version, lang),
            text(Msg::Status, lang),
            text(Msg::Footprints, lang),
            text(Msg::Symbols, lang),
            text(Msg::DeviceSets, lang),
        ],
    );
    for l in &report.libraries {
        out.push_str("<tr>");
        cell(&mut out, &l.path);
        cell(&mut out, &l.version);
        let status = if l.status == "read" {
            text(Msg::Read, lang)
        } else {
            text(Msg::Skipped, lang)
        };
        cell(&mut out, status);
        num_cell(&mut out, l.packages);
        num_cell(&mut out, l.symbols);
        num_cell(&mut out, l.devicesets);
        out.push_str("</tr>");
    }
    out.push_str("</tbody></table>");

    if !report.identical_files.is_empty() {
        out.push_str(&format!(
            "<h2>{}</h2><ul>",
            escape(text(Msg::IdenticalFiles, lang))
        ));
        for set in &report.identical_files {
            out.push_str(&format!("<li>{}</li>", escape(&set.join(", "))));
        }
        out.push_str("</ul>");
    }

    out.push_str(&format!("<h2>{}</h2>", escape(text(Msg::Groups, lang))));
    if report.groups.is_empty() {
        out.push_str(&format!("<p>{}</p>", escape(text(Msg::NoGroups, lang))));
    }
    for kind in crate::group::KINDS {
        if report.groups.iter().any(|g| g.kind == kind) {
            out.push_str(&format!("<h3>{}</h3>", escape(kind_label(kind, lang))));
            groups_table(&mut out, report, kind, lang);
        }
    }

    if !report.notes.is_empty() {
        out.push_str(&format!("<h2>{}</h2>", escape(text(Msg::Notes, lang))));
        head(
            &mut out,
            &[
                text(Msg::Code, lang),
                text(Msg::Library, lang),
                text(Msg::Detail, lang),
            ],
        );
        for n in &report.notes {
            out.push_str("<tr>");
            cell(&mut out, &n.code);
            cell(&mut out, &n.library);
            cell(&mut out, &n.detail);
            out.push_str("</tr>");
        }
        out.push_str("</tbody></table>");
    }
    out.push_str(&format!("</main><script>{SCRIPT}</script></body></html>\n"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::analyse_libraries;
    use crate::scan::{load_bytes, LoadedLibrary};
    use crate::settings::SanitizerSettings;
    use crate::testdata::{library_xml, package_xml};

    fn lib(path: &str, text: &str, s: &SanitizerSettings) -> LoadedLibrary {
        let mut l = LoadedLibrary {
            path: path.into(),
            bytes: 0,
            sha256: "ab".into(),
            version: String::new(),
            data: None,
            notes: Vec::new(),
        };
        load_bytes(&mut l, text.as_bytes(), s);
        l
    }

    fn sample_report(name: &str) -> Report {
        let s = SanitizerSettings::default();
        let two = [(-0.9, 0.0), (0.9, 0.0)];
        let libs = [
            lib(
                "a.lbr",
                &library_xml(&[package_xml(name, &two, 1.2, 1.4)], &[], &[]),
                &s,
            ),
            lib(
                "b.lbr",
                &library_xml(&[package_xml("B", &two, 1.2, 1.4)], &[], &[]),
                &s,
            ),
        ];
        analyse_libraries(&libs, Vec::new(), 0, &s)
    }

    #[test]
    fn the_page_is_self_contained_and_has_the_data() {
        let html = html_report(&sample_report("R0805"), Lang::En);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<title>Library sanitizer report</title>"));
        assert!(html.contains("R0805") && html.contains("P001"));
        assert!(html.contains("class=\"sortable\""));
        assert!(html.contains("<script>"));
        // no external resource of any kind
        for needle in ["http:", "https:", "src=", "href=", "@import", "url("] {
            assert!(!html.contains(needle), "{needle}");
        }
        assert!(html.contains("default-src 'none'"));
        assert!(html.ends_with("</html>\n"));
    }

    #[test]
    fn names_from_a_library_are_escaped() {
        let html = html_report(
            &sample_report("A&lt;script&gt;alert(1)&lt;/script&gt;"),
            Lang::En,
        );
        // the name read from the file is A<script>alert(1)</script>
        assert!(html.contains("A&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert(1)"));
        assert_eq!(escape("a\"b'c<&>"), "a&quot;b&#39;c&lt;&amp;&gt;");
    }

    #[test]
    fn the_page_is_deterministic_and_follows_the_language() {
        let report = sample_report("R");
        assert_eq!(
            html_report(&report, Lang::Es),
            html_report(&report, Lang::Es)
        );
        let es = html_report(&report, Lang::Es);
        assert!(es.contains("lang=\"es\"") && es.contains("Informe del saneador de bibliotecas"));
        let pt = html_report(&report, Lang::Pt);
        assert!(pt.contains("lang=\"pt-PT\"") && pt.contains("Ficheiros lidos"));
        for lang in Lang::ALL {
            let page = html_report(&report, lang);
            // the page carries the title escaped (the apostrophe of French becomes an entity)
            assert!(page.contains(&escape(text(Msg::Title, lang))), "{lang:?}");
        }
    }

    #[test]
    fn an_empty_report_says_there_are_no_duplicates() {
        let s = SanitizerSettings::default();
        let report = analyse_libraries(&[], Vec::new(), 0, &s);
        let html = html_report(&report, Lang::En);
        assert!(html.contains("No duplicates found."));
        assert!(!html.contains("<h3>"));
    }
}
