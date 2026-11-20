// SPDX-License-Identifier: GPL-3.0-or-later
//! The eight languages of the command line: every command of the table is described in each of
//! them, none of the descriptions is a copy of the English one, the ones of Japanese and Chinese
//! are in their scripts, the Portuguese is the Portuguese of Portugal, and `phoenix help` gives
//! the page of every command, and the listing, in each language. The test goes through the table
//! of commands, so a command that is added is checked without a line being added here.

use eh3d_phoenix_cli::commands::{Command, COMMANDS};
use eh3d_phoenix_cli::run;

const TAGS: [&str; 8] = ["en", "es", "de", "fr", "it", "ja", "zh", "pt"];

fn call(list: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn summaries() -> impl Iterator<Item = (&'static Command, usize, &'static str)> {
    COMMANDS
        .iter()
        .flat_map(|c| c.summary.iter().enumerate().map(move |(i, s)| (c, i, *s)))
}

#[test]
fn every_command_has_a_description_in_each_language_and_none_is_a_copy_of_the_english_one() {
    for c in COMMANDS {
        assert_eq!(c.summary.len(), TAGS.len(), "{}", c.name);
        assert!(c.summary[0].is_ascii(), "{}: the English one", c.name);
    }
    for (c, column, text) in summaries() {
        assert!(text.len() > 8, "{} in {}", c.name, TAGS[column]);
        assert_eq!(text, text.trim(), "{} in {}", c.name, TAGS[column]);
        if column > 0 {
            assert_ne!(text, c.summary[0], "{} in {}", c.name, TAGS[column]);
        }
    }
}

#[test]
fn japanese_and_chinese_are_written_in_their_scripts_and_portuguese_is_that_of_portugal() {
    for (c, column, text) in summaries() {
        match TAGS[column] {
            "ja" | "zh" => assert!(!text.is_ascii(), "{} in {}", c.name, TAGS[column]),
            "pt" => {
                for brazilian in ["arquivo", "tela", "usuário", "salvar", "Salvar", "planilha"] {
                    assert!(!text.contains(brazilian), "{}: {brazilian}", c.name);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn the_page_of_every_command_is_written_in_each_language_with_the_same_usage() {
    for c in COMMANDS {
        let (_, english, _) = call(&["help", "--lang", "en", c.name]);
        let usage: Vec<&str> = english
            .lines()
            .skip(1)
            .filter(|l| l.starts_with("phoenix "))
            .collect();
        assert!(!usage.is_empty(), "{}: no line of usage\n{english}", c.name);
        for (column, tag) in TAGS.iter().copied().enumerate() {
            let (code, page, err) = call(&["help", "--lang", tag, c.name]);
            assert_eq!((code, err.as_str()), (0, ""), "{} in {tag}", c.name);
            let head = format!("phoenix {}: {}", c.name, c.summary[column]);
            assert!(page.starts_with(&head), "{} in {tag}: {page}", c.name);
            // the lines of usage are the same in every language: they are the syntax
            for line in &usage {
                assert!(page.contains(line), "{} in {tag}: {line}", c.name);
            }
        }
    }
}

#[test]
fn the_listing_and_the_topics_differ_from_language_to_language_and_name_every_command() {
    let mut listings: Vec<String> = Vec::new();
    for (column, tag) in TAGS.iter().copied().enumerate() {
        let (code, text, err) = call(&["help", "--lang", tag]);
        assert_eq!((code, err.as_str()), (0, ""), "{tag}");
        for c in COMMANDS {
            assert!(text.contains(c.name), "{tag}: {}", c.name);
            assert!(text.contains(c.summary[column]), "{tag}: {}", c.name);
        }
        assert!(!listings.contains(&text), "{tag} repeats a listing");
        listings.push(text);
    }
    for topic in ["board", "manufacturing", "signal", "libraries", "errors"] {
        let mut pages: Vec<String> = Vec::new();
        for tag in TAGS {
            let (code, text, err) = call(&["help", "--lang", tag, topic]);
            assert_eq!((code, err.as_str()), (0, ""), "{topic} in {tag}");
            assert!(!pages.contains(&text), "{topic} in {tag} repeats a page");
            pages.push(text);
        }
    }
}
