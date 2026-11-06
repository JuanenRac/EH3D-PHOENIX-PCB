// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `help` and the option `--help`: the listing of the commands and of the topics, the
//! help of one topic and the help of one command, in the eight languages. The commands, their
//! summaries and their documents come from the table in `commands.rs`, the same table that `run`
//! dispatches from; the lines of usage come from the text of `phoenix --help`.
//!
//! Exit codes: 0 done, 2 the topic, the command or the language is not known.

use std::io::Write;

use eh3d_phoenix_stackup::Language;

use crate::commands::{command, Command, COMMANDS, HELP_SUMMARY};
use crate::help_topics::{label, Label, TOPICS};

/// The position of a language in the eight-text arrays.
fn index(lang: Language) -> usize {
    match lang {
        Language::En => 0,
        Language::Es => 1,
        Language::De => 2,
        Language::Fr => 3,
        Language::It => 4,
        Language::Ja => 5,
        Language::Zh => 6,
        Language::Pt => 7,
    }
}

/// A language tag as `--lang` takes it; `pt-PT` and `zh-CN` count as `pt` and `zh`.
fn language_of(tag: &str) -> Option<Language> {
    let lower = tag.to_ascii_lowercase();
    let head = lower.split(['-', '_']).next().unwrap_or("");
    Language::from_tag(head)
}

/// The names of the commands that the lines of usage name: the word after `phoenix` (and after the
/// `[--config FILE]` of the first line), in the order of the text.
#[cfg(test)]
pub(crate) fn names_in_usage(usage: &str) -> Vec<&str> {
    let mut names: Vec<&str> = Vec::new();
    for line in usage.lines() {
        let Some(rest) = line.strip_prefix("phoenix ") else {
            continue;
        };
        let rest = rest.strip_prefix("[--config FILE] ").unwrap_or(rest);
        let name = rest.split(' ').next().unwrap_or("");
        if !name.is_empty() && !name.starts_with('-') && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// The paragraphs of the usage text that describe one command: its lines of usage and the
/// description under them. A group of commands that share a description comes out whole.
pub(crate) fn usage_of(usage: &str, name: &str) -> Option<String> {
    let lines: Vec<&str> = usage.lines().collect();
    let names_it = |line: &str| {
        line.strip_prefix("phoenix ")
            .map(|rest| rest.strip_prefix("[--config FILE] ").unwrap_or(rest))
            .is_some_and(|rest| rest.split(' ').next() == Some(name))
    };
    let at = lines.iter().position(|line| names_it(line))?;
    let mut start = at;
    while start > 0 && !lines[start - 1].trim().is_empty() {
        start -= 1;
    }
    // 0: lines of usage, 1: after the blank line, 2: in the description
    let mut phase = 0;
    let mut end = start;
    for (i, line) in lines.iter().enumerate().skip(start) {
        let blank = line.trim().is_empty();
        match phase {
            0 if blank => phase = 1,
            1 if !blank => phase = 2,
            2 if line.starts_with("phoenix ") => break,
            _ => {}
        }
        if !blank {
            end = i + 1;
        }
    }
    let mut text = lines[start..end].join("\n");
    text.push('\n');
    Some(text)
}

struct Options<'a> {
    lang: Language,
    words: Vec<&'a str>,
}

fn parse<'a>(args: &[&'a str]) -> Result<Options<'a>, String> {
    let mut options = Options {
        lang: Language::En,
        words: Vec::new(),
    };
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        match arg {
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                options.lang = language_of(tag).ok_or("unknown language code")?;
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => options.words.push(other),
        }
    }
    Ok(options)
}

fn text(which: Label, lang: Language) -> &'static str {
    label(which)[index(lang)]
}

fn command_line(out: &mut dyn Write, c: &Command, lang: Language, width: usize) {
    let _ = writeln!(out, "  {:<width$}  {}", c.name, c.summary[index(lang)]);
}

fn overview(lang: Language, full: bool, out: &mut dyn Write) {
    let _ = writeln!(out, "{}", text(Label::Title, lang));
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", text(Label::Topics, lang));
    let keys: Vec<&str> = TOPICS.iter().map(|t| t.key).collect();
    let _ = writeln!(out, "  {}", keys.join(", "));
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", text(Label::Commands, lang));
    let width = COMMANDS
        .iter()
        .map(|c| c.name.len())
        .max()
        .unwrap_or(0)
        .max("help".len());
    for c in COMMANDS {
        command_line(out, c, lang, width);
    }
    let _ = writeln!(out, "  {:<width$}  {}", "help", HELP_SUMMARY[index(lang)]);
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", text(Label::Tutorials, lang));
    let _ = writeln!(out, "{}", text(Label::Lang, lang));
    if full {
        let _ = writeln!(out);
        let _ = writeln!(out, "{}:", text(Label::Usage, lang));
        let _ = writeln!(out);
        let _ = out.write_all(crate::USAGE.as_bytes());
    }
}

fn topic_page(key: &str, lang: Language, out: &mut dyn Write) -> bool {
    let Some(topic) = TOPICS.iter().find(|t| t.key == key) else {
        return false;
    };
    let _ = writeln!(out, "{}: {}", text(Label::Topic, lang), topic.key);
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", topic.text[index(lang)]);
    let members: Vec<&Command> = COMMANDS.iter().filter(|c| c.topic == topic.topic).collect();
    if !members.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "{}:", text(Label::CommandsOf, lang));
        let width = members.iter().map(|c| c.name.len()).max().unwrap_or(0);
        for c in &members {
            command_line(out, c, lang, width);
        }
    }
    let mut docs: Vec<&str> = Vec::new();
    for doc in members.iter().map(|c| c.doc).chain(topic.extra_doc) {
        if !docs.contains(&doc) {
            docs.push(doc);
        }
    }
    if !docs.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "{}: {}", text(Label::Doc, lang), docs.join(", "));
    }
    true
}

fn command_page(c: &Command, lang: Language, out: &mut dyn Write) {
    let _ = writeln!(out, "phoenix {}: {}", c.name, c.summary[index(lang)]);
    let _ = writeln!(out);
    let _ = writeln!(out, "{}: {}", text(Label::Doc, lang), c.doc);
    if let Some(usage) = usage_of(crate::USAGE, c.name) {
        let _ = writeln!(out);
        let _ = writeln!(out, "{}:", text(Label::Usage, lang));
        let _ = writeln!(out);
        let _ = out.write_all(usage.as_bytes());
    }
}

/// Runs `phoenix help` (`full` is false) or `phoenix --help` (`full` is true, which adds the lines
/// of usage with every option) with the arguments that follow the word. Returns the exit code.
pub(crate) fn help_command(
    args: &[&str],
    full: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let options = match parse(args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            let _ = writeln!(err, "{}", text(Label::Try, Language::En));
            return 2;
        }
    };
    let lang = options.lang;
    match options.words.as_slice() {
        [] => overview(lang, full, out),
        [word] => {
            if let Some(c) = command(word) {
                command_page(c, lang, out);
            } else if !topic_page(word, lang, out) {
                let _ = writeln!(err, "error: {}: {word}", text(Label::Unknown, lang));
                let _ = writeln!(err, "{}", text(Label::Try, lang));
                return 2;
            }
        }
        _ => {
            let _ = writeln!(err, "error: {}", text(Label::Unknown, lang));
            let _ = writeln!(err, "{}", text(Label::Try, lang));
            return 2;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn run_help(args: &[&str]) -> (i32, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = help_command(args, false, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn the_table_and_the_lines_of_usage_name_the_same_commands() {
        // `help` is answered by `run` and is not a row of the table
        let mut in_usage = names_in_usage(crate::USAGE);
        in_usage.retain(|name| *name != "help");
        let mut in_table: Vec<&str> = COMMANDS.iter().map(|c| c.name).collect();
        in_usage.sort_unstable();
        in_table.sort_unstable();
        assert_eq!(in_usage, in_table);
    }

    #[test]
    fn only_the_words_that_run_handles_have_no_function() {
        let mut without: Vec<&str> = COMMANDS
            .iter()
            .filter(|c| c.run.is_none())
            .map(|c| c.name)
            .collect();
        without.sort_unstable();
        assert_eq!(without, vec!["eagle-rate", "inventory"]);
    }

    #[test]
    fn every_row_has_a_summary_in_each_language_and_a_document_that_exists() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for c in COMMANDS {
            for (i, summary) in c.summary.iter().enumerate() {
                assert!(summary.len() > 8, "{} language {i}", c.name);
            }
            assert!(root.join(c.doc).is_file(), "{}: {}", c.name, c.doc);
        }
        for topic in TOPICS {
            for (i, paragraph) in topic.text.iter().enumerate() {
                assert!(paragraph.len() > 20, "{} language {i}", topic.key);
            }
            if let Some(doc) = topic.extra_doc {
                assert!(root.join(doc).is_file(), "{doc}");
            }
        }
    }

    #[test]
    fn the_listing_has_every_command_in_the_language_asked_for() {
        let (code, text, err) = run_help(&["--lang", "es"]);
        assert_eq!((code, err.as_str()), (0, ""));
        for c in COMMANDS {
            assert!(text.contains(c.summary[1]), "{}", c.name);
        }
        assert!(text.contains("Temas de ayuda"));
        let (_, pt, _) = run_help(&["--lang", "pt-PT"]);
        assert!(pt.contains("Comandos (phoenix help <comando>)"));
    }

    #[test]
    fn a_topic_lists_its_commands_and_a_command_shows_its_usage() {
        let (code, text, _) = run_help(&["simulation"]);
        assert_eq!(code, 0);
        assert!(
            text.contains("  sim ") && text.contains("  netlist "),
            "{text}"
        );
        assert!(text.contains("docs/SIM.md, docs/NETLIST.md"), "{text}");
        let (code, text, _) = run_help(&["--lang", "de", "check"]);
        assert_eq!(code, 0);
        assert!(text.contains("phoenix check [--rules FILE"), "{text}");
        assert!(text.contains("Entwurfsregeln einer Platine"), "{text}");
        assert!(!text.contains("phoenix erc"), "{text}");
    }

    #[test]
    fn what_is_not_known_exits_2() {
        for args in [
            vec!["nothing"],
            vec!["--lang", "xx"],
            vec!["--colour"],
            vec!["check", "sim"],
        ] {
            let (code, out, err) = run_help(&args);
            assert_eq!((code, out.is_empty()), (2, true), "{args:?}");
            assert!(err.contains("phoenix help"), "{args:?}: {err}");
        }
    }

    #[test]
    fn the_group_of_a_command_that_shares_its_description_comes_out_whole() {
        let sim = usage_of(crate::USAGE, "sim").unwrap();
        assert!(sim.contains("phoenix sim --check-filter"));
        assert!(sim.contains("circuit with the solver"));
        assert!(!sim.contains("phoenix netlist"));
    }
}
