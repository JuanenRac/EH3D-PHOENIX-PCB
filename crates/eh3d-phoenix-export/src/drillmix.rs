// SPDX-License-Identifier: GPL-3.0-or-later
//! The plated and the not plated holes in one Excellon file.
//!
//! Some makers take a single drill file in which every tool says whether it drills plated or not
//! plated holes (the mixed mode, `TF.FileFunction,MixedPlating`). This module joins the two files
//! that [`crate::excellon`] writes: the tools of the plated file come first and keep their numbers,
//! the tools of the other file follow, numbered after them, and the lines that select a tool in
//! the body are renumbered the same way. A tool keeps its own `TA.AperFunction` attribute, which
//! is what tells a plated tool from a tool of the same diameter that is not plated, and the
//! attribute `TF.FileFunction` becomes `MixedPlating,<first layer>,<last layer>`.
//!
//! The text is treated as text: nothing is parsed into numbers, the order of the holes is not
//! touched and the same two files always give the same bytes. The two files must agree on the
//! units, the zeros and the other header lines; a difference is refused with the line that
//! differs. Not more than 99 tools are numbered.

const FILE_FUNCTION: &str = "TF.FileFunction,";

/// A drill file taken apart: the comments of the header, the unit line, the tools with their
/// attribute lines, and the body from the selection of the first tool to the line before `M30`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Parts {
    /// The comments of the header that come before the first tool, in order.
    comments: Vec<String>,
    /// The line with the unit (`METRIC,TZ`).
    unit: String,
    /// The other lines of the header that are not comments or tools.
    other: Vec<String>,
    /// Each tool: the text of its definition after the number, and the attribute line before it.
    tools: Vec<(String, Option<String>)>,
    /// The body lines.
    body: Vec<String>,
}

fn is_tool_definition(line: &str) -> Option<(u32, &str)> {
    let rest = line.strip_prefix('T')?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits > 2 {
        return None;
    }
    let after = &rest[digits..];
    if !after.starts_with('C') {
        return None;
    }
    let number: u32 = rest[..digits].parse().ok()?;
    Some((number, after))
}

fn is_tool_selection(line: &str) -> Option<u32> {
    let rest = line.strip_prefix('T')?;
    if rest.is_empty() || rest.len() > 2 || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

fn is_attribute_line(line: &str) -> bool {
    line.starts_with("; #@! TA.") || line.starts_with(";#@! TA.")
}

fn is_delete_attribute(line: &str) -> bool {
    line.starts_with("; #@! TD") || line.starts_with(";#@! TD")
}

fn take_apart(text: &str, which: &str) -> Result<Parts, String> {
    let mut lines = text.lines();
    if lines.next() != Some("M48") {
        return Err(format!("the {which} file does not start with M48"));
    }
    let mut parts = Parts {
        comments: Vec::new(),
        unit: String::new(),
        other: Vec::new(),
        tools: Vec::new(),
        body: Vec::new(),
    };
    let mut pending: Option<String> = None;
    let mut in_body = false;
    let mut ended = false;
    for line in lines {
        if ended {
            if line.is_empty() {
                continue;
            }
            return Err(format!("the {which} file has text after M30"));
        }
        if in_body {
            if line == "M30" {
                ended = true;
            } else {
                parts.body.push(line.to_owned());
            }
            continue;
        }
        if line == "M30" {
            return Err(format!("the {which} file has no % line"));
        } else if line == "%" {
            in_body = true;
        } else if is_attribute_line(line) {
            pending = Some(line.to_owned());
        } else if is_delete_attribute(line) {
            // written once after the last tool; the merged file writes its own
        } else if line.starts_with(';') {
            parts.comments.push(line.to_owned());
        } else if line.starts_with("METRIC") || line.starts_with("INCH") {
            if !parts.unit.is_empty() {
                return Err(format!("the {which} file has two unit lines"));
            }
            line.clone_into(&mut parts.unit);
        } else if let Some((_, rest)) = is_tool_definition(line) {
            parts.tools.push((rest.to_owned(), pending.take()));
        } else {
            parts.other.push(line.to_owned());
        }
    }
    if !ended {
        return Err(format!("the {which} file does not end with M30"));
    }
    if !in_body {
        return Err(format!("the {which} file has no % line"));
    }
    if parts.unit.is_empty() {
        return Err(format!("the {which} file has no unit line"));
    }
    Ok(parts)
}

/// The layers of the `FileFunction` comment of a file (`Plated,1,4,PTH,Drill` gives 1 and 4).
fn layers_of(comments: &[String]) -> Option<(String, String)> {
    let line = comments.iter().find(|c| c.contains(FILE_FUNCTION))?;
    let at = line.find(FILE_FUNCTION)? + FILE_FUNCTION.len();
    let values: Vec<&str> = line[at..].split(',').collect();
    let (from, to) = (values.get(1)?, values.get(2)?);
    let digits = |s: &&str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (digits(from) && digits(to)).then(|| ((*from).to_owned(), (*to).to_owned()))
}

/// Joins the file of the plated holes and the file of the holes that are not plated (either may
/// be missing, not both) into one mixed file.
pub fn merge_plated_and_not_plated(
    plated: Option<&str>,
    not_plated: Option<&str>,
) -> Result<String, String> {
    let first = plated
        .map(|t| take_apart(t, "plated"))
        .transpose()?
        .map(|p| ("plated", p));
    let second = not_plated
        .map(|t| take_apart(t, "not plated"))
        .transpose()?
        .map(|p| ("not plated", p));
    let sources: Vec<(&str, Parts)> = first.into_iter().chain(second).collect();
    let Some((_, lead)) = sources.first() else {
        return Err("there is no drill file to merge".to_owned());
    };
    for (which, other) in &sources[1..] {
        if other.unit != lead.unit {
            return Err(format!(
                "the units differ: '{}' and '{}' ({which})",
                lead.unit, other.unit
            ));
        }
        if other.other != lead.other {
            return Err(format!("the header lines of the {which} file differ"));
        }
        let plain = |parts: &Parts| -> Vec<String> {
            parts
                .comments
                .iter()
                .filter(|c| !c.contains("#@!"))
                .cloned()
                .collect()
        };
        if plain(other) != plain(lead) {
            return Err(format!("the comments of the {which} file differ"));
        }
    }
    let total: usize = sources.iter().map(|(_, p)| p.tools.len()).sum();
    if total > 99 {
        return Err(format!("{total} tools do not fit the numbers T01 to T99"));
    }
    let (from, to) = layers_of(&lead.comments).unwrap_or_else(|| ("1".to_owned(), "2".to_owned()));
    let mut out: Vec<String> = vec!["M48".to_owned()];
    for comment in &lead.comments {
        if comment.contains(FILE_FUNCTION) {
            out.push(format!("; #@! {FILE_FUNCTION}MixedPlating,{from},{to}"));
        } else {
            out.push(comment.clone());
        }
    }
    out.push(lead.unit.clone());
    out.extend(lead.other.iter().cloned());
    let mut body: Vec<String> = Vec::new();
    let mut offset = 0u32;
    for (_, parts) in &sources {
        for (index, (rest, attribute)) in parts.tools.iter().enumerate() {
            if let Some(line) = attribute {
                out.push(line.clone());
            }
            out.push(format!("T{:02}{rest}", offset + index as u32 + 1));
        }
        for line in &parts.body {
            match is_tool_selection(line) {
                Some(n) => body.push(format!("T{:02}", n + offset)),
                None => body.push(line.clone()),
            }
        }
        offset += parts.tools.len() as u32;
    }
    if sources
        .iter()
        .any(|(_, p)| p.tools.iter().any(|t| t.1.is_some()))
    {
        out.push("; #@! TD.AperFunction".to_owned());
    }
    out.push("%".to_owned());
    out.extend(body);
    out.push("M30".to_owned());
    out.push(String::new());
    Ok(out.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLATED: &str = "M48\n; #@! TF.GenerationSoftware,EH3D,PHOENIX,0.1\n; #@! TF.FileFunction,Plated,1,4,PTH,Drill\nMETRIC,TZ\n; #@! TA.AperFunction,Plated,PTH,ViaDrill\nT01C0.3\n; #@! TA.AperFunction,Plated,PTH,ComponentDrill\nT02C0.8\n; #@! TD.AperFunction\n%\nT01\nX1.0Y1.0\nX2.0Y1.0\nT02\nX5.0Y5.0\nM30\n";
    const NOT_PLATED: &str = "M48\n; #@! TF.GenerationSoftware,EH3D,PHOENIX,0.1\n; #@! TF.FileFunction,NonPlated,1,4,NPTH,Drill\nMETRIC,TZ\n; #@! TA.AperFunction,NonPlated,NPTH,MechanicalDrill\nT01C0.8\n; #@! TA.AperFunction,NonPlated,NPTH,MechanicalDrill\nT02C3.2\n; #@! TD.AperFunction\n%\nT01\nX9.0Y9.0\nT02\nX20.0Y20.0\nM30\n";

    #[test]
    fn two_files_become_one_with_the_tools_of_the_second_after_the_first() {
        let merged = merge_plated_and_not_plated(Some(PLATED), Some(NOT_PLATED)).unwrap();
        assert_eq!(
            merged,
            "M48\n; #@! TF.GenerationSoftware,EH3D,PHOENIX,0.1\n; #@! TF.FileFunction,MixedPlating,1,4\nMETRIC,TZ\n\
             ; #@! TA.AperFunction,Plated,PTH,ViaDrill\nT01C0.3\n\
             ; #@! TA.AperFunction,Plated,PTH,ComponentDrill\nT02C0.8\n\
             ; #@! TA.AperFunction,NonPlated,NPTH,MechanicalDrill\nT03C0.8\n\
             ; #@! TA.AperFunction,NonPlated,NPTH,MechanicalDrill\nT04C3.2\n\
             ; #@! TD.AperFunction\n%\n\
             T01\nX1.0Y1.0\nX2.0Y1.0\nT02\nX5.0Y5.0\nT03\nX9.0Y9.0\nT04\nX20.0Y20.0\nM30\n"
        );
        // a plated tool and a tool that is not plated keep their own attribute even at one diameter
        assert!(merged.contains("Plated,PTH,ComponentDrill\nT02C0.8"));
        assert!(merged.contains("NonPlated,NPTH,MechanicalDrill\nT03C0.8"));
    }

    #[test]
    fn one_file_alone_is_made_mixed_too_and_none_is_refused() {
        let only = merge_plated_and_not_plated(None, Some(NOT_PLATED)).unwrap();
        assert!(only.contains("TF.FileFunction,MixedPlating,1,4\n"));
        assert!(only.contains("T01C0.8") && only.contains("T02C3.2"));
        assert!(only.ends_with("T02\nX20.0Y20.0\nM30\n"));
        let e = merge_plated_and_not_plated(None, None).unwrap_err();
        assert!(e.contains("no drill file"), "{e}");
    }

    #[test]
    fn the_same_files_always_give_the_same_bytes_and_the_merge_is_not_repeated() {
        let a = merge_plated_and_not_plated(Some(PLATED), Some(NOT_PLATED)).unwrap();
        let b = merge_plated_and_not_plated(Some(PLATED), Some(NOT_PLATED)).unwrap();
        assert_eq!(a, b);
        // a merged file taken apart again keeps its four tools (the merge is a text, not a state)
        let parts = take_apart(&a, "merged").unwrap();
        assert_eq!(parts.tools.len(), 4);
        assert_eq!(parts.body.len(), 9);
    }

    #[test]
    fn files_without_attributes_keep_their_lines_and_get_no_attribute() {
        let plated = "M48\n;made by a test\nMETRIC,TZ\nT01C0.3\n%\nT01\nX1Y1\nM30\n";
        let other = "M48\n;made by a test\nMETRIC,TZ\nT01C0.9F200S65\n%\nT01\nX2Y2\nG85X3Y2\nM30\n";
        let merged = merge_plated_and_not_plated(Some(plated), Some(other)).unwrap();
        assert_eq!(
            merged,
            "M48\n;made by a test\nMETRIC,TZ\nT01C0.3\nT02C0.9F200S65\n%\nT01\nX1Y1\nT02\nX2Y2\nG85X3Y2\nM30\n"
        );
        assert!(!merged.contains("#@!"));
    }

    #[test]
    fn files_that_do_not_agree_are_refused_with_what_differs() {
        let inch = NOT_PLATED.replace("METRIC,TZ", "INCH,TZ");
        let e = merge_plated_and_not_plated(Some(PLATED), Some(&inch)).unwrap_err();
        assert!(e.contains("units differ"), "{e}");
        let style = NOT_PLATED.replace("M48\n", "M48\n;FILE_FORMAT=3:3\n");
        let e = merge_plated_and_not_plated(Some(PLATED), Some(&style)).unwrap_err();
        assert!(e.contains("comments of the not plated file differ"), "{e}");
        let extra = NOT_PLATED.replace("METRIC,TZ\n", "METRIC,TZ\nFMAT,2\n");
        let e = merge_plated_and_not_plated(Some(PLATED), Some(&extra)).unwrap_err();
        assert!(e.contains("header lines"), "{e}");
    }

    #[test]
    fn damaged_files_are_refused_and_never_panic() {
        for (text, needle) in [
            ("", "does not start with M48"),
            (
                "M48\nMETRIC,TZ\nT01C1\n%\nT01\nX1Y1\n",
                "does not end with M30",
            ),
            ("M48\nMETRIC,TZ\nT01C1\nM30\n", "no % line"),
            ("M48\nT01C1\n%\nT01\nX1Y1\nM30\n", "no unit line"),
            ("M48\nMETRIC,TZ\nINCH,TZ\n%\nM30\n", "two unit lines"),
            ("M48\nMETRIC,TZ\n%\nM30\nT01\n", "text after M30"),
        ] {
            let e = merge_plated_and_not_plated(Some(text), None).unwrap_err();
            assert!(e.contains(needle), "{text:?}: {e}");
        }
        let mut state = 0x7a11_ed00u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as usize
        };
        for _ in 0..2000 {
            let mut bytes = PLATED.as_bytes().to_vec();
            for _ in 0..(1 + next() % 5) {
                let at = next() % bytes.len();
                match next() % 3 {
                    0 => bytes[at] = b"T%M30C0123;\n,X"[next() % 14],
                    1 => {
                        bytes.remove(at);
                    }
                    _ => bytes.insert(at, b"T%\nM"[next() % 4]),
                }
            }
            if let Ok(text) = String::from_utf8(bytes) {
                let _ = merge_plated_and_not_plated(Some(&text), Some(NOT_PLATED));
            }
        }
    }

    #[test]
    fn more_than_ninety_nine_tools_are_refused() {
        let tools: String = (1..=60).map(|n| format!("T{n:02}C{n}\n")).collect();
        let big = format!("M48\nMETRIC,TZ\n{tools}%\nT01\nX1Y1\nM30\n");
        let e = merge_plated_and_not_plated(Some(&big), Some(&big)).unwrap_err();
        assert!(e.contains("120 tools"), "{e}");
        assert!(merge_plated_and_not_plated(Some(&big), None).is_ok());
    }

    #[test]
    fn the_layers_come_from_the_file_function_and_default_to_the_outer_pair() {
        let comments = vec!["; #@! TF.FileFunction,Plated,1,6,PTH,Mixed".to_owned()];
        assert_eq!(layers_of(&comments), Some(("1".to_owned(), "6".to_owned())));
        assert_eq!(
            layers_of(&["; #@! TF.FileFunction,Plated".to_owned()]),
            None
        );
        assert_eq!(
            layers_of(&["; #@! TF.FileFunction,Plated,a,b".to_owned()]),
            None
        );
        let no_function = "M48\nMETRIC,TZ\nT01C1\n%\nT01\nX1Y1\nM30\n";
        let merged = merge_plated_and_not_plated(Some(no_function), None).unwrap();
        assert!(!merged.contains("MixedPlating"));
    }
}
