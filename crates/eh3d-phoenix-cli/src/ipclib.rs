// SPDX-License-Identifier: GPL-3.0-or-later
//! The commands `ipc` and `libindex`: the own library of footprints by the method of IPC-7351B,
//! and the bulk import of EAGLE and KiCad libraries into an index. Both are a dry run unless
//! `--out` names a new file, and neither writes over an input file or over a file that exists.
//!
//! Exit codes of `ipc`: 0 done and nothing found, 1 the check finds something or the comparison
//! finds a difference, 2 the arguments, the input or the output are unusable. `libindex` exits
//! with 0 when it made the index, whatever its report says, and 2 when it could not.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use eh3d_phoenix_ipclib::check::check_footprint;
use eh3d_phoenix_ipclib::codes::{
    IpcError, DENSITY_INVALID, FILE_UNUSABLE, IDX_FILE_UNREADABLE, IDX_OPTION_INVALID,
    IDX_WRITE_FAILED, OPTION_INVALID,
};
use eh3d_phoenix_ipclib::{
    check_library, compare_pads, generate, index_folder, library, load_footprint, save_kicad_mod,
    save_lbr, write_index, BulkSettings, Density, FamilySet, IpcFootprint, IpcSettings,
};
use eh3d_phoenix_libedit::write::fmt_mm;
use eh3d_phoenix_rules::Lang;
use serde_json::{json, Value};

pub(crate) const IPC_USAGE: &str = "\
phoenix ipc list [--dir FOLDER] [--json] [--lang xx]
phoenix ipc show <family> [<variant>] [--dir FOLDER] [--json]
phoenix ipc generate <family> <variant> [--param NAME=VALUE ...] [--density A|B|C] [--dir FOLDER]
                     [--config FILE] [--out NEW.lbr|NEW.kicad_mod] [--json] [--lang xx]
phoenix ipc check [--family NAME] [--dir FOLDER] [--config FILE] [--json] [--lang xx]
phoenix ipc compare <library.lbr|file.kicad_mod> <family> <variant> [--name FOOTPRINT]
                    [--param NAME=VALUE ...] [--density A|B|C] [--dir FOLDER] [--config FILE]
                    [--json] [--lang xx]

  ipc        the own library of footprints by the method of IPC-7351B: families (chip 0201 to
             2512, MELF, SOD, SOT, SOIC, SSOP, TSSOP, QFN, DFN, QFP, BGA, DIP, headers and
             connectors of a fixed pitch) with their dimensions in data files, three density
             levels and the canonical names (RESC1608X55N, SOIC127P600X175-8N). The dimensions
             are a prototype, not checked against a datasheet
  list       the families and their variants
  show       the dimensions of a family or of one variant
  generate   make a footprint; a dry run unless --out names a new .lbr or .kicad_mod file, and a
             file that exists is never written over
  check      check the library (names, pads that overlap, pitch, origin, mask and paste,
             silkscreen, courtyard by density; codes PHOENIX-IPC-001 to 010)
  compare    compare a footprint of your library with the generated one, pad by pad; a report,
             nothing is replaced (exit 1 when pads differ)
  --dir      the folder of family files (config/ipc_families holds the ones that come with the
             program); --config the settings (config/ipc.cfg shows every key)
  --json     print the result as JSON. --lang xx writes the report in en, es, de, fr, it, ja, zh
             or pt
";

pub(crate) const LIBINDEX_USAGE: &str = "\
phoenix libindex <folder> [--json] [--out INDEX.json] [--config FILE] [--lang xx]

  libindex   walk a folder for EAGLE libraries (.lbr) and KiCad files (.kicad_mod, .kicad_sym),
             judge every footprint and symbol (pads with the same name, without copper, a hole
             larger than the pad, an origin far from the pads, empty names, repeated names and
             repeated geometries between libraries) and make a deterministic index,
             phoenix-libindex/1, with the hash of the content of each item. The files are only
             read and never copied; links are not followed; the limits are those of
             config/import_bulk.cfg. A dry run: the index is written only to a new file named
             with --out, and a file that exists is never written over
  --json     print the index as JSON instead of the quality report
";

/// The largest settings file that is read, in bytes.
const MAX_SETTINGS_BYTES: u64 = 1024 * 1024;

const VALUE_FLAGS: [&str; 8] = [
    "--dir",
    "--config",
    "--out",
    "--density",
    "--family",
    "--name",
    "--lang",
    "--param",
];

#[derive(Default)]
struct Opts {
    dir: Option<String>,
    config: Option<String>,
    out: Option<String>,
    density: Option<String>,
    family: Option<String>,
    name: Option<String>,
    params: BTreeMap<String, String>,
    json: bool,
    lang: Option<Lang>,
    positional: Vec<String>,
    used: Vec<&'static str>,
}

fn parse(args: &[&str], code: &'static str) -> Result<Opts, IpcError> {
    let bad = |what: String| IpcError::new(code, what);
    let mut o = Opts::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let arg = *arg;
        if arg == "--json" {
            o.json = true;
            o.used.push("--json");
            continue;
        }
        if let Some(flag) = VALUE_FLAGS.iter().find(|f| **f == arg) {
            let Some(value) = iter.next() else {
                return Err(bad(format!("{arg} needs a value")));
            };
            let value = (*value).to_owned();
            o.used.push(*flag);
            match arg {
                "--dir" => o.dir = Some(value),
                "--config" => o.config = Some(value),
                "--out" => o.out = Some(value),
                "--density" => o.density = Some(value),
                "--family" => o.family = Some(value),
                "--name" => o.name = Some(value),
                "--lang" => {
                    let lang = Lang::from_tag(&value)
                        .ok_or_else(|| bad("--lang is not a language of the suite".to_owned()))?;
                    o.lang = Some(lang);
                }
                _ => {
                    let (name, text) = value
                        .split_once('=')
                        .ok_or_else(|| bad("--param needs NAME=VALUE".to_owned()))?;
                    if o.params
                        .insert(name.trim().to_owned(), text.trim().to_owned())
                        .is_some()
                    {
                        return Err(bad(format!("the parameter {} is given twice", name.trim())));
                    }
                }
            }
            continue;
        }
        if arg.starts_with("--") {
            return Err(bad(format!("unknown option {arg}")));
        }
        o.positional.push(arg.to_owned());
    }
    Ok(o)
}

impl Opts {
    /// Refuses the options that the command does not take.
    fn only(&self, allowed: &[&str], code: &'static str) -> Result<(), IpcError> {
        for flag in &self.used {
            if !allowed.contains(flag) {
                return Err(IpcError::new(
                    code,
                    format!("{flag} does not apply to this command"),
                ));
            }
        }
        Ok(())
    }

    fn lang(&self) -> Lang {
        self.lang.unwrap_or(Lang::En)
    }
}

fn read_small(path: &str) -> Result<String, IpcError> {
    let unusable = |e: String| IpcError::new(FILE_UNUSABLE, format!("{path}: {e}"));
    let length = std::fs::metadata(path)
        .map_err(|e| unusable(e.to_string()))?
        .len();
    if length > MAX_SETTINGS_BYTES {
        return Err(unusable(format!("larger than {MAX_SETTINGS_BYTES} bytes")));
    }
    std::fs::read_to_string(path).map_err(|e| unusable(e.to_string()))
}

fn settings_of(o: &Opts) -> Result<IpcSettings, IpcError> {
    match &o.config {
        Some(path) => IpcSettings::parse(&read_small(path)?),
        None => Ok(IpcSettings::builtin()),
    }
}

fn families_of(o: &Opts, settings: &IpcSettings) -> Result<FamilySet, IpcError> {
    match &o.dir {
        Some(dir) => FamilySet::from_dir(Path::new(dir), settings),
        None => FamilySet::builtin(),
    }
}

fn density_of(o: &Opts, settings: &IpcSettings) -> Result<Density, IpcError> {
    match &o.density {
        Some(text) => {
            Density::parse(text).ok_or_else(|| IpcError::new(DENSITY_INVALID, text.as_str()))
        }
        None => Ok(settings.default_density),
    }
}

fn print_json(out: &mut dyn Write, value: &Value) {
    let _ = writeln!(
        out,
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

fn list(o: &Opts, out: &mut dyn Write) -> Result<i32, IpcError> {
    o.only(&["--dir", "--config", "--json", "--lang"], OPTION_INVALID)?;
    if !o.positional.is_empty() {
        return Err(IpcError::new(OPTION_INVALID, "list takes no arguments"));
    }
    let settings = settings_of(o)?;
    let set = families_of(o, &settings)?;
    if o.json {
        let rows: Vec<Value> = set
            .families
            .iter()
            .map(|f| {
                json!({
                    "family": f.family,
                    "title": f.title,
                    "standard": f.standard,
                    "verified": f.verified,
                    "status": f.status,
                    "layout": f.layout,
                    "variants": f.variant_names(),
                })
            })
            .collect();
        print_json(
            out,
            &json!({"format": "phoenix-ipc-list/1", "families": rows}),
        );
    } else {
        for f in &set.families {
            let _ = writeln!(out, "{}: {} ({})", f.family, f.title, f.standard);
            let _ = writeln!(
                out,
                "  {}; variants: {}",
                f.status,
                f.variant_names().join(", ")
            );
        }
    }
    Ok(0)
}

fn show(o: &Opts, out: &mut dyn Write) -> Result<i32, IpcError> {
    o.only(&["--dir", "--config", "--json"], OPTION_INVALID)?;
    let (name, variant) = match o.positional.as_slice() {
        [family] => (family.as_str(), None),
        [family, variant] => (family.as_str(), Some(variant.as_str())),
        _ => {
            return Err(IpcError::new(
                OPTION_INVALID,
                "show needs a family and, optionally, a variant",
            ))
        }
    };
    let settings = settings_of(o)?;
    let set = families_of(o, &settings)?;
    let family = set.get(name)?;
    let goals_of = |v: &eh3d_phoenix_ipclib::family::Variant| -> Vec<Value> {
        Density::ALL
            .iter()
            .map(|d| {
                let g = family.goals(v, *d);
                json!({
                    "density": d.key(),
                    "letter": d.letter().to_string(),
                    "toe": g.toe,
                    "heel": g.heel,
                    "side": g.side,
                    "courtyard": g.courtyard,
                    "pad_ratio": g.pad_ratio,
                    "drill_extra": g.drill_extra,
                    "annular": g.annular,
                })
            })
            .collect()
    };
    let value = match variant {
        None => json!({
            "family": family.family,
            "title": family.title,
            "standard": family.standard,
            "verified": family.verified,
            "status": family.status,
            "layout": family.layout,
            "naming": family.naming,
            "prefixes": family.prefixes,
            "tolerances": {"fab": family.tolerances.fab, "place": family.tolerances.place},
            "variants": family.variant_names(),
        }),
        Some(v) => {
            let var = family.variant(v)?;
            json!({
                "family": family.family,
                "variant": v,
                "verified": family.verified,
                "status": family.status,
                "code": var.code,
                "values": var.values,
                "slots_a": var.slots_a,
                "slots_b": var.slots_b,
                "pads": var.pads.len(),
                "goals": goals_of(var),
            })
        }
    };
    print_json(out, &value);
    Ok(0)
}

/// The pads of a footprint as lines of text, in millimetres.
fn pad_lines(fp: &IpcFootprint) -> String {
    let mut text = String::new();
    for p in &fp.pads {
        text.push_str(&format!(
            "  pad {}: x {} y {} size {} x {}",
            p.name,
            fmt_mm(p.x),
            fmt_mm(p.y),
            fmt_mm(p.w),
            fmt_mm(p.h)
        ));
        if p.is_smd() {
            text.push_str(&format!(", paste {} %\n", p.paste));
        } else {
            text.push_str(&format!(", hole {}\n", fmt_mm(p.drill)));
        }
    }
    text
}

fn generate_command(o: &Opts, out: &mut dyn Write) -> Result<i32, IpcError> {
    o.only(
        &[
            "--dir",
            "--config",
            "--out",
            "--density",
            "--param",
            "--json",
            "--lang",
        ],
        OPTION_INVALID,
    )?;
    let [family, variant] = o.positional.as_slice() else {
        return Err(IpcError::new(
            OPTION_INVALID,
            "generate needs a family and a variant",
        ));
    };
    let settings = settings_of(o)?;
    let set = families_of(o, &settings)?;
    let level = density_of(o, &settings)?;
    let fp = generate(&set, family, variant, level, &o.params)?;
    let found = check_footprint(&fp, &set, &settings);
    let written = match &o.out {
        None => None,
        Some(target) => {
            let path = Path::new(target);
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            match ext.as_str() {
                "lbr" => {
                    save_lbr(std::slice::from_ref(&fp), path)?;
                }
                "kicad_mod" => {
                    save_kicad_mod(&fp, path)?;
                }
                _ => {
                    return Err(IpcError::new(
                        OPTION_INVALID,
                        "--out must end in .lbr or .kicad_mod",
                    ))
                }
            }
            Some(target.clone())
        }
    };
    let lang = o.lang();
    if o.json {
        let list: Vec<Value> = found
            .iter()
            .map(|f| json!({"code": f.code, "detail": f.detail}))
            .collect();
        let mut value = fp.to_json();
        value["check"] = json!(list);
        value["written"] = json!(written);
        print_json(out, &value);
    } else {
        let _ = writeln!(
            out,
            "{} (density {}, {})",
            fp.name,
            fp.density.key(),
            fp.notes[0]
        );
        for note in fp.notes.iter().skip(1) {
            let _ = writeln!(out, "  {note}");
        }
        let _ = out.write_all(pad_lines(&fp).as_bytes());
        let _ = writeln!(
            out,
            "courtyard {} x {} mm",
            fmt_mm(fp.court.2 - fp.court.0),
            fmt_mm(fp.court.3 - fp.court.1)
        );
        for f in &found {
            let _ = writeln!(
                out,
                "  {}: {}: {}",
                f.code,
                eh3d_phoenix_ipclib::codes::title(f.code, lang),
                f.detail
            );
        }
        match &written {
            Some(path) => {
                let _ = writeln!(out, "written: {path}");
            }
            None => {
                let _ = writeln!(
                    out,
                    "dry run: nothing was written (name a new file with --out)"
                );
            }
        }
    }
    Ok(i32::from(!found.is_empty()))
}

fn check_command(o: &Opts, out: &mut dyn Write) -> Result<i32, IpcError> {
    o.only(
        &["--dir", "--config", "--family", "--json", "--lang"],
        OPTION_INVALID,
    )?;
    if !o.positional.is_empty() {
        return Err(IpcError::new(OPTION_INVALID, "check takes no arguments"));
    }
    let settings = settings_of(o)?;
    let set = families_of(o, &settings)?;
    let mut footprints = library(&set)?;
    if let Some(name) = &o.family {
        set.get(name)?;
        footprints.retain(|f| &f.family == name);
    }
    let report = check_library(&footprints, &set, &settings);
    if o.json {
        print_json(out, &report.to_json());
    } else {
        let _ = out.write_all(report.to_text(o.lang()).as_bytes());
    }
    Ok(i32::from(!report.is_clean()))
}

fn compare_command(o: &Opts, out: &mut dyn Write) -> Result<i32, IpcError> {
    o.only(
        &[
            "--dir",
            "--config",
            "--name",
            "--density",
            "--param",
            "--json",
            "--lang",
        ],
        OPTION_INVALID,
    )?;
    let [file, family, variant] = o.positional.as_slice() else {
        return Err(IpcError::new(
            OPTION_INVALID,
            "compare needs a file, a family and a variant",
        ));
    };
    let settings = settings_of(o)?;
    let set = families_of(o, &settings)?;
    let level = density_of(o, &settings)?;
    let fp = generate(&set, family, variant, level, &o.params)?;
    let (name, owner) =
        load_footprint(Path::new(file), o.name.as_deref(), settings.max_file_bytes)?;
    let report = compare_pads(
        &name,
        &owner,
        &fp.name,
        &eh3d_phoenix_ipclib::pads::from_ipc(&fp),
        settings.compare_tol_nm,
    );
    if o.json {
        print_json(out, &report.to_json());
    } else {
        let _ = out.write_all(report.to_text(o.lang()).as_bytes());
    }
    Ok(i32::from(!report.is_same()))
}

fn ipc_inner(
    args: &[&str],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, (IpcError, Lang)> {
    let Some((sub, rest)) = args.split_first() else {
        let _ = err.write_all(IPC_USAGE.as_bytes());
        return Ok(2);
    };
    let o = parse(rest, OPTION_INVALID).map_err(|e| (e, Lang::En))?;
    let lang = o.lang();
    let result = match *sub {
        "list" => list(&o, out),
        "show" => show(&o, out),
        "generate" => generate_command(&o, out),
        "check" => check_command(&o, out),
        "compare" => compare_command(&o, out),
        other => {
            let _ = err.write_all(IPC_USAGE.as_bytes());
            Err(IpcError::new(
                OPTION_INVALID,
                format!("unknown subcommand {other}"),
            ))
        }
    };
    result.map_err(|e| (e, lang))
}

/// Runs the command `ipc` with the arguments that follow its name. Returns the exit code.
pub(crate) fn ipc_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match ipc_inner(args, out, err) {
        Ok(code) => code,
        Err((e, lang)) => {
            let _ = writeln!(err, "error: {}", e.message(lang));
            2
        }
    }
}

fn libindex_inner(
    args: &[&str],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, (IpcError, Lang)> {
    let o = parse(args, IDX_OPTION_INVALID).map_err(|e| (e, Lang::En))?;
    let lang = o.lang();
    let fail = |e: IpcError| (e, lang);
    o.only(
        &["--config", "--out", "--json", "--lang"],
        IDX_OPTION_INVALID,
    )
    .map_err(fail)?;
    let [folder] = o.positional.as_slice() else {
        let _ = err.write_all(LIBINDEX_USAGE.as_bytes());
        return Ok(2);
    };
    let settings = match &o.config {
        Some(path) => {
            let text = read_small(path)
                .map_err(|e| IpcError::new(IDX_FILE_UNREADABLE, e.detail))
                .map_err(fail)?;
            BulkSettings::parse(&text).map_err(fail)?
        }
        None => BulkSettings::builtin(),
    };
    if let Some(target) = &o.out {
        if !target.to_ascii_lowercase().ends_with(".json") {
            return Err(fail(IpcError::new(
                IDX_OPTION_INVALID,
                "--out must end in .json",
            )));
        }
        if Path::new(target).exists() {
            return Err(fail(IpcError::new(
                IDX_WRITE_FAILED,
                format!("{target} exists and is never written over"),
            )));
        }
    }
    let index = index_folder(Path::new(folder), &settings).map_err(fail)?;
    let written = match &o.out {
        Some(target) => {
            write_index(&index, Path::new(target)).map_err(fail)?;
            Some(target.clone())
        }
        None => None,
    };
    if o.json {
        print_json(out, &index.to_json());
    } else {
        let _ = out.write_all(index.to_text(lang).as_bytes());
        match &written {
            Some(path) => {
                let _ = writeln!(out, "index written: {path}");
            }
            None => {
                let _ = writeln!(
                    out,
                    "dry run: no index was written (name a new file with --out)"
                );
            }
        }
    }
    Ok(0)
}

/// Runs the command `libindex` with the arguments that follow its name. Returns the exit code.
pub(crate) fn libindex_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match libindex_inner(args, out, err) {
        Ok(code) => code,
        Err((e, lang)) => {
            let _ = writeln!(err, "error: {}", e.message(lang));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(
        command: fn(&[&str], &mut dyn Write, &mut dyn Write) -> i32,
        args: &[&str],
    ) -> (i32, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = command(args, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn list_and_show_name_the_families_and_their_status() {
        let (code, out, _) = run(ipc_command, &["list"]);
        assert_eq!(code, 0);
        assert!(out.contains("chip: Chip resistors"));
        assert!(out.contains("not verified against a datasheet"));
        let (code, out, _) = run(ipc_command, &["list", "--json"]);
        assert_eq!(code, 0);
        let value: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["families"].as_array().unwrap().len(), 15);
        let (code, out, _) = run(ipc_command, &["show", "soic", "narrow"]);
        assert_eq!(code, 0);
        assert!(out.contains("\"span\": 6.0"), "{out}");
        let (code, _, err) = run(ipc_command, &["show", "chop"]);
        assert_eq!(code, 2);
        assert!(err.contains("PHOENIX-IPC-020"), "{err}");
    }

    #[test]
    fn generate_is_a_dry_run_and_writes_only_a_new_file() {
        let (code, out, _) = run(
            ipc_command,
            &[
                "generate",
                "soic",
                "narrow",
                "--param",
                "pins=8",
                "--density",
                "A",
            ],
        );
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("SOIC127P600X175-8M"), "{out}");
        assert!(out.contains("dry run"), "{out}");
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.kicad_mod");
        let path = target.to_str().unwrap();
        let args = ["generate", "chip", "0603", "--out", path];
        let (code, out, _) = run(ipc_command, &args);
        assert_eq!(code, 0, "{out}");
        assert!(std::fs::read_to_string(&target)
            .unwrap()
            .contains("RESC1608X55N"));
        let (code, _, err) = run(ipc_command, &args);
        assert_eq!(code, 2);
        assert!(err.contains("PHOENIX-IPC-027"), "{err}");
        let lbr = dir.path().join("a.lbr");
        let args = [
            "generate",
            "chip",
            "0805",
            "--out",
            lbr.to_str().unwrap(),
            "--json",
        ];
        let (code, out, _) = run(ipc_command, &args);
        assert_eq!(code, 0, "{out}");
        let value: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["name"], "RESC2012X60N");
        assert!(lbr.exists());
        let bad = dir.path().join("a.txt");
        let args = ["generate", "chip", "0805", "--out", bad.to_str().unwrap()];
        assert_eq!(run(ipc_command, &args).0, 2);
        assert!(!bad.exists());
    }

    #[test]
    fn check_passes_for_the_library_and_options_are_refused_where_they_do_not_apply() {
        let (code, out, _) = run(ipc_command, &["check"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("0 findings"));
        let (code, out, _) = run(ipc_command, &["check", "--family", "qfn", "--lang", "pt"]);
        assert_eq!(code, 0);
        assert!(out.contains("footprints checked"));
        for args in [
            &["check", "--out", "x"][..],
            &["list", "extra"][..],
            &["generate", "chip"][..],
            &["generate", "chip", "0603", "--density", "D"][..],
            &["generate", "chip", "0603", "--colour", "red"][..],
            &["nothing"][..],
        ] {
            assert_eq!(run(ipc_command, args).0, 2, "{args:?}");
        }
        assert_eq!(run(ipc_command, &[]).0, 2);
    }

    #[test]
    fn compare_reports_the_differences_with_a_footprint_of_the_owner() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("owner.kicad_mod");
        let path = target.to_str().unwrap();
        let (code, _, _) = run(ipc_command, &["generate", "chip", "0603", "--out", path]);
        assert_eq!(code, 0);
        let (code, out, _) = run(ipc_command, &["compare", path, "chip", "0603"]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("2 of 2 pads agree"));
        let (code, out, _) = run(ipc_command, &["compare", path, "chip", "0805", "--json"]);
        assert_eq!(code, 1, "{out}");
        let value: Value = serde_json::from_str(&out).unwrap();
        assert!(!value["differences"].as_array().unwrap().is_empty());
    }

    #[test]
    fn libindex_prints_the_report_and_writes_the_index_once() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path().join("a.lbr");
        let (code, _, _) = run(
            ipc_command,
            &["generate", "chip", "0603", "--out", lib.to_str().unwrap()],
        );
        assert_eq!(code, 0);
        let folder = dir.path().to_str().unwrap();
        let (code, out, _) = run(libindex_command, &[folder]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("1 files, 1 footprints, 0 symbols"), "{out}");
        assert!(out.contains("dry run"));
        let (code, out, _) = run(libindex_command, &[folder, "--json"]);
        assert_eq!(code, 0);
        let value: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(value["format"], "phoenix-libindex/1");
        let elsewhere = tempfile::tempdir().unwrap();
        let index = elsewhere.path().join("index.json");
        let args = [folder, "--out", index.to_str().unwrap()];
        let (code, out, _) = run(libindex_command, &args);
        assert_eq!(code, 0, "{out}");
        let (code, _, err) = run(libindex_command, &args);
        assert_eq!(code, 2);
        assert!(err.contains("PHOENIX-LIBIDX-027"), "{err}");
        assert_eq!(run(libindex_command, &[]).0, 2);
        assert_eq!(
            run(libindex_command, &["/nonexistent-folder-for-the-test"]).0,
            2
        );
    }
}
