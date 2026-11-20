// SPDX-License-Identifier: GPL-3.0-or-later
//! The subcommand `ipc padstack`: the names of padstacks judged by the naming rule, and made from
//! lengths in millimetres.

use eh3d_phoenix_cli::run;

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

#[test]
fn good_names_are_shown_with_their_canonical_form_and_exit_zero() {
    let (code, out, err) = call(&[
        "ipc",
        "padstack",
        "c150h90z140x170m165mx185a200y300",
        "v30h15l1-3",
        "c130_95",
    ]);
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert_eq!(
        out,
        "c150h90z140x170m165mx185a200y300 -> c150h90z140x170m165mx185a200y300\n\
         v30h15l1-3 -> v30h15l1-3\n\
         c130_95 -> c130_95\n"
    );
}

#[test]
fn a_name_in_another_order_gets_a_warning_with_the_canonical_name() {
    let (code, out, _) = call(&["ipc", "padstack", "c150m165h90z140"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("c150m165h90z140\n"), "{out}");
    assert!(
        out.contains("warning: PHOENIX-IPC-051: The name of the padstack is not the canonical one: the canonical name is c150h90z140m165"),
        "{out}"
    );
}

#[test]
fn a_name_that_cannot_be_is_an_error_in_every_language() {
    for (tag, title) in [
        ("en", "The name of the padstack is not valid"),
        ("es", "El nombre del padstack no es válido"),
        ("de", "Der Name des Padstacks ist ungültig"),
        ("fr", "Le nom du padstack n'est pas valide"),
        ("it", "Il nome del padstack non è valido"),
        ("ja", "パッドスタック名が正しくありません"),
        ("zh", "焊盘堆栈名称无效"),
        ("pt", "O nome do padstack não é válido"),
    ] {
        let (code, out, _) = call(&["ipc", "padstack", "c150h150", "--lang", tag]);
        assert_eq!(code, 1, "{tag}");
        assert!(
            out.contains(&format!(
                "error: PHOENIX-IPC-050: {title}: the hole is not smaller than the land"
            )),
            "{tag}: {out}"
        );
    }
    let (code, out, _) = call(&["ipc", "padstack", "c150q9", "c1.5"]);
    assert_eq!(code, 1);
    assert!(out.contains("at 4: a modifier is expected"), "{out}");
    assert!(
        out.contains("the character '.' cannot be in a name"),
        "{out}"
    );
}

#[test]
fn a_padstack_is_made_from_millimetres_and_the_json_has_the_fields() {
    let (code, out, err) = call(&[
        "ipc",
        "padstack",
        "--param",
        "shape=c",
        "--param",
        "size=1.5",
        "--param",
        "hole=0.9",
        "--param",
        "mask=1.65",
    ]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, "c150h90m165 -> c150h90m165\n");
    let (code, out, _) = call(&["ipc", "padstack", "c150h90m165", "--json"]);
    assert_eq!(code, 0);
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    let first = &value["padstacks"][0];
    assert_eq!(first["name"], "c150h90m165");
    assert_eq!(first["canonical"], "c150h90m165");
    assert_eq!(first["padstack"]["a_nm"], 1_500_000);
    assert_eq!(first["padstack"]["hole_nm"], 900_000);
    assert_eq!(first["padstack"]["mask_nm"], 1_650_000);
    assert_eq!(first["padstack"]["plated"], true);
    assert_eq!(first["findings"].as_array().unwrap().len(), 0);
}

#[test]
fn the_wrong_calls_are_refused_with_a_code() {
    let (code, _, err) = call(&["ipc", "padstack"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-IPC-025") && err.contains("needs names"),
        "{err}"
    );
    let (code, _, err) = call(&["ipc", "padstack", "c150", "--param", "shape=c"]);
    assert_eq!(code, 2);
    assert!(err.contains("not both"), "{err}");
    let (code, _, err) = call(&[
        "ipc", "padstack", "--param", "shape=c", "--param", "size=abc",
    ]);
    assert_eq!(code, 2);
    assert!(
        err.contains("PHOENIX-IPC-050") && err.contains("size: 'abc'"),
        "{err}"
    );
    let (code, _, err) = call(&["ipc", "padstack", "c150", "--dir", "x"]);
    assert_eq!(code, 2);
    assert!(err.contains("--dir does not apply"), "{err}");
}
