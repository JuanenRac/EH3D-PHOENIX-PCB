// SPDX-License-Identifier: GPL-3.0-or-later
//! The command `pnp`: the pick and place files of a board, one for each face, as CSV and as JSON
//! `phoenix-pnp/1`.
//!
//! It is a dry run unless `--out` names a folder, which must not exist. Exit codes: 0 done, 2 the
//! arguments or the input are unusable.

use std::io::Write;

use eh3d_phoenix_board::Side;
use eh3d_phoenix_export::fabcodes::{FabError, PNP_OPTION_INVALID, PNP_OUTPUT_FAILED};
use eh3d_phoenix_export::pnp::{plan, pnp_csv, pnp_json, PnpOrigin, PnpPlan, PnpSettings, Unit};
use eh3d_phoenix_export::Lang;

use crate::fabpack::{
    board_of, load_parts, load_pnp_settings, not_fitted_set, split_designators, write_folder_new,
};

pub(crate) const USAGE: &str = "\
phoenix pnp [--config FILE] [--maker NAME] [--parts FILE] [--dnp R1,R2,...]
            [--variant NAME [--variant-file FILE]] [--origin lower_left|aux|panel] [--units mm|mil] [--lang CODE] [--out FOLDER] <board>

  pnp           the pick and place files of a board, one for each face, as CSV and as JSON
                (phoenix-pnp/1): designator, value, footprint, X, Y, rotation and face of every
                part. The centre of a part is the middle of the box of its pads (or the mean of
                its pads, or the origin of its footprint, from the settings); the extra rotations
                of the table of config/pnp_rotations.cfg (by footprint pattern, apart for each
                maker) are added; the origin is the lower left corner of the board, an auxiliary
                origin or the origin of a panel. Parts that are not fitted (the file of part
                data, --dnp) are left out and listed, or marked. A dry run unless --out names a
                folder that does not exist. A prototype: never loaded in a machine
  --config      the settings (default: config/pnp_rotations.cfg when it exists)
  --maker       the section of the settings for a machine or maker
  --parts       a CSV of part data; the parts it says are not fitted are left out
  --variant     an assembly variant of an EAGLE file: the parts it does not fit are left out like
                --dnp; the variants are read from the board file when it is an EAGLE .brd or .sch,
                otherwise from --variant-file
  --lang CODE   the messages in en, es, de, fr, it, ja, zh or pt
";

/// The files of the plan: for each face that has parts, the CSV and the JSON, with `prefix` before
/// each name.
pub(crate) fn pnp_files(
    plan: &PnpPlan,
    settings: &PnpSettings,
    prefix: &str,
) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    for (side, word) in [(Side::Top, "top"), (Side::Bottom, "bottom")] {
        let rows = plan.rows(side);
        if rows.is_empty() {
            continue;
        }
        files.push((
            format!("{prefix}pnp_{word}.csv"),
            pnp_csv(rows, settings).into_bytes(),
        ));
        files.push((
            format!("{prefix}pnp_{word}.json"),
            pnp_json(plan, side, settings).into_bytes(),
        ));
    }
    files
}

struct Options {
    lang: Lang,
    config: Option<String>,
    maker: Option<String>,
    parts: Option<String>,
    dnp: Vec<String>,
    variant: Option<String>,
    variant_file: Option<String>,
    origin: Option<PnpOrigin>,
    units: Option<Unit>,
    out: Option<String>,
    files: Vec<String>,
}

fn parse_options(args: &[&str]) -> Result<Options, FabError> {
    let mut o = Options {
        lang: Lang::En,
        config: None,
        maker: None,
        parts: None,
        dnp: Vec::new(),
        variant: None,
        variant_file: None,
        origin: None,
        units: None,
        out: None,
        files: Vec::new(),
    };
    let bad = |what: &str| FabError::new(PNP_OPTION_INVALID, what.to_owned());
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, FabError> {
            iter.next()
                .map(|v| (*v).to_owned())
                .ok_or_else(|| bad(what))
        };
        match *arg {
            "--lang" => {
                let tag = value("--lang needs a language code")?;
                o.lang = Lang::from_tag(&tag).ok_or_else(|| bad("unknown language code"))?;
            }
            "--config" => o.config = Some(value("--config needs a file")?),
            "--maker" => o.maker = Some(value("--maker needs a name")?),
            "--parts" => o.parts = Some(value("--parts needs a file")?),
            "--dnp" => o
                .dnp
                .extend(split_designators(&value("--dnp needs a list")?)),
            "--variant" => o.variant = Some(value("--variant needs a name")?),
            "--variant-file" => o.variant_file = Some(value("--variant-file needs a file")?),
            "--origin" => {
                o.origin = Some(match value("--origin needs a word")?.as_str() {
                    "lower_left" => PnpOrigin::LowerLeft,
                    "aux" => PnpOrigin::Aux,
                    "panel" => PnpOrigin::Panel,
                    _ => return Err(bad("--origin is lower_left, aux or panel")),
                })
            }
            "--units" => {
                o.units = Some(match value("--units needs a word")?.as_str() {
                    "mm" => Unit::Mm,
                    "mil" => Unit::Mil,
                    _ => return Err(bad("--units is mm or mil")),
                })
            }
            "--out" => o.out = Some(value("--out needs a folder")?),
            other if other.starts_with("--") => {
                return Err(FabError::new(
                    PNP_OPTION_INVALID,
                    format!("unknown option {other}"),
                ))
            }
            other => o.files.push(other.to_owned()),
        }
    }
    Ok(o)
}

fn pnp(o: &Options, out: &mut dyn Write) -> Result<i32, FabError> {
    let [board_path] = o.files.as_slice() else {
        return Err(FabError::new(
            PNP_OPTION_INVALID,
            "give exactly one board file",
        ));
    };
    let mut settings = load_pnp_settings(o.config.as_deref(), o.maker.as_deref())?;
    if let Some(origin) = o.origin {
        settings.origin = origin;
    }
    if let Some(unit) = o.units {
        settings.unit = unit;
    }
    settings.validate()?;
    let board = board_of(board_path)?;
    let parts = load_parts(o.parts.as_deref())?;
    let mut dnp = o.dnp.clone();
    let mut variant_note = String::new();
    if let Some(name) = &o.variant {
        let source = o.variant_file.as_deref().unwrap_or(board_path);
        let view = crate::eagle_deep::variant_view(source, name, o.lang)
            .map_err(|message| FabError::new(PNP_OPTION_INVALID, message))?;
        let left_out = view.not_fitted();
        variant_note = format!(
            "variant: {name}, not fitted: {}
",
            left_out.len()
        );
        dnp.extend(left_out);
    } else if o.variant_file.is_some() {
        return Err(FabError::new(
            PNP_OPTION_INVALID,
            "--variant-file needs --variant",
        ));
    }
    let not_fitted = not_fitted_set(&board, &parts, &dnp);
    let placement = plan(&board, &settings, &not_fitted);
    let files = pnp_files(&placement, &settings, "");

    let mut text = format!(
        "board: {}\ntop: {} part(s), bottom: {} part(s), left out: {}\nunits: {}\n",
        board.name,
        placement.top.len(),
        placement.bottom.len(),
        placement.excluded.len(),
        settings.unit.name()
    );
    text.push_str(&variant_note);
    for f in &placement.findings {
        text.push_str(&f.line(o.lang));
        text.push('\n');
    }
    if let Some(dir) = &o.out {
        write_folder_new(dir, &files, PNP_OUTPUT_FAILED)?;
        for (name, _) in &files {
            text.push_str(&format!("written: {dir}/{name}\n"));
        }
    } else {
        text.push_str("dry run: nothing was written, give --out to write the files\n");
    }
    let _ = out.write_all(text.as_bytes());
    Ok(0)
}

pub(crate) fn pnp_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            let _ = err.write_all(USAGE.as_bytes());
            let _ = writeln!(err, "error: {}", error.message(Lang::En));
            return 2;
        }
    };
    match pnp(&options, out) {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(err, "error: {}", error.message(options.lang));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_are_read_and_the_wrong_ones_refused() {
        let o = parse_options(&[
            "--origin", "aux", "--units", "mil", "--maker", "acme", "--dnp", "R1,R2", "--out", "d",
            "b.json",
        ])
        .unwrap();
        assert_eq!(o.origin, Some(PnpOrigin::Aux));
        assert_eq!(o.units, Some(Unit::Mil));
        assert_eq!(o.maker.as_deref(), Some("acme"));
        assert_eq!(o.dnp, vec!["R1", "R2"]);
        assert_eq!(o.files, vec!["b.json".to_owned()]);
        for bad in [
            vec!["--origin", "middle"],
            vec!["--units", "inch"],
            vec!["--config"],
            vec!["--lang", "xx"],
            vec!["--nope"],
        ] {
            let e = parse_options(&bad).err().unwrap();
            assert_eq!(e.code, PNP_OPTION_INVALID, "{bad:?}");
        }
    }
}
