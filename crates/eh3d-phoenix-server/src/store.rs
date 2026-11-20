// SPDX-License-Identifier: GPL-3.0-or-later
//! The project store of the server: a file of a board is kept in the store of EH3D-SERVER under
//! the application `phoenix-pcb`, each write is a new version, and a write over a file that
//! exists names the version it was made from (`If-Match`), so two people never overwrite each
//! other without knowing. A write that names no version over a file that exists is refused with
//! `PHOENIX-SERVER-015` (the server's `428`), and one that names a version that is no longer the
//! current one with `PHOENIX-SERVER-016` (the `412`): read the file again and decide.
//!
//! The link is the client of the suite (`eh3d-serverclient`); this file adds the checks of the
//! names, the limits of the settings, the refusals with their codes and the words in the eight
//! languages. Nothing is written to the server unless the settings allow it.

use eh3d_phoenix_rules::Lang;
use eh3d_serverclient as sc;
use eh3d_serverclient::json::quote;
use eh3d_serverclient::StoreError;

use crate::codes::{
    ServerError, LIMIT_EXCEEDED, NETWORK_NOT_ALLOWED, NOTHING_TO_SEND, NOT_CONFIGURED,
    OPTION_INVALID, PROJECT_CHANGED, PROJECT_NAME_INVALID, PROJECT_NEEDS_TAG, SERVER_FAILED,
};
use crate::session::{client_lang, APP};
use crate::settings::ServerSettings;

/// The format name of the JSON that describes what the store did.
pub const STORE_FORMAT: &str = "phoenix-server-store/1";

const MIB: usize = 1024 * 1024;

fn name_byte_is_safe(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')
}

/// Whether a text can be the id of a project: letters, digits, `_`, `-` and `.`, up to 128, and
/// not a dot segment.
pub fn project_id_is_safe(id: &str) -> bool {
    (1..=128).contains(&id.len()) && id != "." && id != ".." && id.bytes().all(name_byte_is_safe)
}

/// Whether a text can be the name of a file of a project: pieces of letters, digits, `_`, `-` and
/// `.` joined by single slashes, none of them empty or a dot segment, up to 240 bytes.
pub fn file_name_is_safe(name: &str) -> bool {
    (1..=240).contains(&name.len())
        && name.split('/').all(|piece| {
            !piece.is_empty()
                && piece != "."
                && piece != ".."
                && piece.bytes().all(name_byte_is_safe)
        })
}

/// The path of a file of the store, with the project and the name checked.
pub fn store_path(project: &str, name: &str) -> Result<String, ServerError> {
    if !project_id_is_safe(project) {
        return Err(ServerError::new(
            PROJECT_NAME_INVALID,
            "a project is letters, digits, _, - and . up to 128 characters",
        ));
    }
    if !file_name_is_safe(name) {
        return Err(ServerError::new(
            PROJECT_NAME_INVALID,
            "a file name is letters, digits, _, - and . in pieces joined by / up to 240 characters",
        ));
    }
    sc::project_file_path(APP, project, name)
        .map_err(|why| ServerError::new(PROJECT_NAME_INVALID, why))
}

/// Whether the settings allow to use the store: the switch, an address and a token. The machine
/// of the jobs is not needed.
pub fn check_store_ready(settings: &ServerSettings) -> Result<(), ServerError> {
    if !settings.allow_network {
        return Err(ServerError::new(
            NETWORK_NOT_ALLOWED,
            "set allow_network = true in the settings",
        ));
    }
    if !settings.link.is_configured() {
        return Err(ServerError::new(NOT_CONFIGURED, "server_address is empty"));
    }
    if settings.link.server_token.is_empty() {
        return Err(ServerError::new(
            NOT_CONFIGURED,
            "server_token is empty: make a token for this program on the server",
        ));
    }
    Ok(())
}

fn store_failed(error: StoreError, lang: Lang) -> ServerError {
    let text = error.text(client_lang(lang));
    match error {
        StoreError::PreconditionRequired { .. } => ServerError::new(PROJECT_NEEDS_TAG, text),
        StoreError::PreconditionFailed { .. } => ServerError::new(PROJECT_CHANGED, text),
        StoreError::Other(_) => ServerError::new(SERVER_FAILED, text),
    }
}

/// What the store did with a file that was saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    /// The project.
    pub project: String,
    /// The name of the file in the project.
    pub name: String,
    /// The size of the file, in bytes.
    pub bytes: usize,
    /// The tag of the version that is now the current one, when the server sent it.
    pub tag: Option<String>,
    /// The number of the version, when the server said it.
    pub version: Option<u64>,
    /// True when the bytes were the ones of the current version and no version was made.
    pub unchanged: bool,
    /// True when the file did not exist and was made.
    pub created: bool,
}

/// A file read from the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    /// The project.
    pub project: String,
    /// The name of the file in the project.
    pub name: String,
    /// The tag of the version read, when the server sent it: the next write names it.
    pub tag: Option<String>,
    /// The bytes of the file.
    pub bytes: Vec<u8>,
}

/// Saves a file to the store. `tag` is the version it was made from: `None` for a file that does
/// not exist yet. The write is sent once. The settings must allow it ([`check_store_ready`]) and
/// the file must not be empty or larger than `project_max_mib`.
pub fn save_project(
    client: &mut sc::Client,
    settings: &ServerSettings,
    project: &str,
    name: &str,
    bytes: &[u8],
    tag: Option<&str>,
    lang: Lang,
) -> Result<Saved, ServerError> {
    check_store_ready(settings)?;
    let path = store_path(project, name)?;
    if bytes.is_empty() {
        return Err(ServerError::new(NOTHING_TO_SEND, "the file is empty"));
    }
    if bytes.len() > settings.project_max_mib as usize * MIB {
        return Err(ServerError::new(
            LIMIT_EXCEEDED,
            format!(
                "the file is {} bytes, more than project_max_mib ({} MiB)",
                bytes.len(),
                settings.project_max_mib
            ),
        ));
    }
    if let Some(tag) = tag {
        sc::etag_header(tag).map_err(|why| ServerError::new(OPTION_INVALID, why))?;
    }
    let written = client
        .put_if_match(&path, tag, bytes, Some("application/octet-stream"))
        .map_err(|e| store_failed(e, lang))?;
    Ok(Saved {
        project: project.to_owned(),
        name: name.to_owned(),
        bytes: bytes.len(),
        tag: written.etag,
        version: written.version,
        unchanged: written.unchanged,
        created: tag.is_none() && !written.unchanged,
    })
}

/// Reads a file from the store with the tag of its version; `version` asks for an older version
/// (from 1). The settings must allow it ([`check_store_ready`]).
pub fn load_project(
    client: &mut sc::Client,
    settings: &ServerSettings,
    project: &str,
    name: &str,
    version: Option<u64>,
    lang: Lang,
) -> Result<Fetched, ServerError> {
    check_store_ready(settings)?;
    let mut path = store_path(project, name)?;
    if let Some(version) = version {
        if version == 0 {
            return Err(ServerError::new(
                OPTION_INVALID,
                "the versions of a file start at 1",
            ));
        }
        path.push_str(&format!("&version={version}"));
    }
    let read = client
        .get_with_etag(&path)
        .map_err(|e| store_failed(e, lang))?;
    Ok(Fetched {
        project: project.to_owned(),
        name: name.to_owned(),
        tag: read.etag,
        bytes: read.bytes,
    })
}

/// The words of the report of the store in one language.
struct Words {
    created: &'static str,
    saved: &'static str,
    unchanged: &'static str,
    loaded: &'static str,
    size: &'static str,
    tag: &'static str,
    version: &'static str,
}

fn words(lang: Lang) -> Words {
    let w = |created, saved, unchanged, loaded, size, tag, version| Words {
        created,
        saved,
        unchanged,
        loaded,
        size,
        tag,
        version,
    };
    match lang {
        Lang::En => w(
            "Created in the store",
            "Saved to the store",
            "Already in the store, unchanged",
            "Read from the store",
            "size (bytes)",
            "version tag",
            "version",
        ),
        Lang::Es => w(
            "Creado en el almacén",
            "Guardado en el almacén",
            "Ya estaba en el almacén, sin cambios",
            "Leído del almacén",
            "tamaño (bytes)",
            "etiqueta de versión",
            "versión",
        ),
        Lang::De => w(
            "Im Speicher angelegt",
            "Im Speicher gesichert",
            "Schon im Speicher, unverändert",
            "Aus dem Speicher gelesen",
            "Größe (Bytes)",
            "Versions-Tag",
            "Version",
        ),
        Lang::Fr => w(
            "Créé dans le magasin",
            "Enregistré dans le magasin",
            "Déjà dans le magasin, inchangé",
            "Lu depuis le magasin",
            "taille (octets)",
            "étiquette de version",
            "version",
        ),
        Lang::It => w(
            "Creato nell'archivio",
            "Salvato nell'archivio",
            "Già nell'archivio, invariato",
            "Letto dall'archivio",
            "dimensione (byte)",
            "etichetta di versione",
            "versione",
        ),
        Lang::Ja => w(
            "ストアに作成しました",
            "ストアに保存しました",
            "既にストアにあり、変更なし",
            "ストアから読み込みました",
            "サイズ（バイト）",
            "バージョンタグ",
            "バージョン",
        ),
        Lang::Zh => w(
            "已在存储中创建",
            "已保存到存储",
            "存储中已有，未更改",
            "已从存储读取",
            "大小（字节）",
            "版本标签",
            "版本",
        ),
        Lang::Pt => w(
            "Criado no armazém",
            "Guardado no armazém",
            "Já estava no armazém, sem alterações",
            "Lido do armazém",
            "tamanho (bytes)",
            "etiqueta de versão",
            "versão",
        ),
    }
}

fn tag_text(tag: &Option<String>) -> &str {
    tag.as_deref().unwrap_or("-")
}

/// What the store did with a saved file, as text in a language.
pub fn saved_text(saved: &Saved, lang: Lang) -> String {
    let w = words(lang);
    let headline = if saved.unchanged {
        w.unchanged
    } else if saved.created {
        w.created
    } else {
        w.saved
    };
    let mut out = format!(
        "{headline}: {}/{}\n{}: {}\n{}: {}\n",
        saved.project,
        saved.name,
        w.size,
        saved.bytes,
        w.tag,
        tag_text(&saved.tag)
    );
    if let Some(version) = saved.version {
        out.push_str(&format!("{}: {version}\n", w.version));
    }
    out
}

/// A file read from the store, as text in a language.
pub fn fetched_text(fetched: &Fetched, lang: Lang) -> String {
    let w = words(lang);
    format!(
        "{}: {}/{}\n{}: {}\n{}: {}\n",
        w.loaded,
        fetched.project,
        fetched.name,
        w.size,
        fetched.bytes.len(),
        w.tag,
        tag_text(&fetched.tag)
    )
}

fn json_tag(tag: &Option<String>) -> String {
    tag.as_deref().map_or("null".to_owned(), quote)
}

/// What the store did with a saved file, as JSON of the format `phoenix-server-store/1`.
pub fn saved_json(saved: &Saved) -> String {
    format!(
        "{{\"format\":\"{STORE_FORMAT}\",\"action\":\"save\",\"project\":{},\"name\":{},\"bytes\":{},\"tag\":{},\"version\":{},\"unchanged\":{},\"created\":{}}}\n",
        quote(&saved.project),
        quote(&saved.name),
        saved.bytes,
        json_tag(&saved.tag),
        saved.version.map_or("null".to_owned(), |v| v.to_string()),
        saved.unchanged,
        saved.created
    )
}

/// A file read from the store, as JSON of the format `phoenix-server-store/1`.
pub fn fetched_json(fetched: &Fetched) -> String {
    format!(
        "{{\"format\":\"{STORE_FORMAT}\",\"action\":\"load\",\"project\":{},\"name\":{},\"bytes\":{},\"tag\":{}}}\n",
        quote(&fetched.project),
        quote(&fetched.name),
        fetched.bytes.len(),
        json_tag(&fetched.tag)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::{DRY_RUN, SETTINGS_INVALID};
    use eh3d_serverclient::sim::SimServer;
    use eh3d_serverclient::Json;

    fn settings() -> ServerSettings {
        let mut s = ServerSettings {
            allow_network: true,
            ..ServerSettings::default()
        };
        s.link.set("server_address", "127.0.0.1:18200").unwrap();
        s.link.set("server_token", "t").unwrap();
        s
    }

    fn client(sim: &SimServer, s: &ServerSettings) -> sc::Client {
        let mut c = sc::Client::new(s.link.clone(), Box::new(sim.clone()))
            .with_sleeper(Box::new(sim.sleeper()))
            .with_clock(Box::new(sim.clock()));
        c.login("admin", "secret-password", APP).unwrap();
        c
    }

    #[test]
    fn the_names_of_a_project_and_of_a_file_are_checked_before_anything_is_sent() {
        for good in ["p-1", "board_2.v3", "A", &"x".repeat(128)] {
            assert!(project_id_is_safe(good), "{good}");
        }
        for bad in [
            "",
            ".",
            "..",
            "a b",
            "a/b",
            "a?b",
            "\u{e9}",
            &"x".repeat(129),
        ] {
            assert!(!project_id_is_safe(bad), "{bad:?}");
        }
        for good in ["board.json", "layers/top.gbr", "a-b_c.d", &"y".repeat(240)] {
            assert!(file_name_is_safe(good), "{good}");
        }
        for bad in [
            "",
            "/a",
            "a/",
            "a//b",
            "./a",
            "a/../b",
            "..",
            "a b",
            "a&b=c",
            "a#b",
            "a\nb",
            &"y".repeat(241),
        ] {
            assert!(!file_name_is_safe(bad), "{bad:?}");
        }
        assert_eq!(
            store_path("p-1", "layers/top.gbr").unwrap(),
            "/api/v1/apps/phoenix-pcb/projects/p-1/files/content?name=layers%2Ftop.gbr"
        );
        assert_eq!(
            store_path("a b", "x").unwrap_err().code,
            PROJECT_NAME_INVALID
        );
        assert_eq!(
            store_path("p", "../x").unwrap_err().code,
            PROJECT_NAME_INVALID
        );
    }

    #[test]
    fn a_new_file_is_created_and_a_second_write_must_name_the_version() {
        let s = settings();
        let sim = SimServer::new();
        let mut c = client(&sim, &s);
        let first = save_project(&mut c, &s, "p-1", "board.json", b"one", None, Lang::En).unwrap();
        assert!(first.created && !first.unchanged);
        assert_eq!((first.bytes, first.version), (3, Some(1)));
        let tag = first.tag.clone().unwrap();
        // the file exists now: a write that names no version is refused
        let e = save_project(&mut c, &s, "p-1", "board.json", b"two", None, Lang::En).unwrap_err();
        assert_eq!(e.code, PROJECT_NEEDS_TAG);
        // naming the version it was made from, it is a new version
        let second = save_project(
            &mut c,
            &s,
            "p-1",
            "board.json",
            b"two",
            Some(&tag),
            Lang::En,
        )
        .unwrap();
        assert!(!second.created && !second.unchanged);
        assert_eq!(second.version, Some(2));
        assert_ne!(second.tag, first.tag);
        assert_eq!(
            sim.file(APP, "p-1", "board.json").as_deref(),
            Some(b"two".as_slice())
        );
        // the same bytes again make no version
        let again = save_project(
            &mut c,
            &s,
            "p-1",
            "board.json",
            b"two",
            second.tag.as_deref(),
            Lang::En,
        )
        .unwrap();
        assert!(again.unchanged && !again.created);
        assert_eq!(sim.file_versions(APP, "p-1", "board.json"), 2);
    }

    #[test]
    fn a_file_that_somebody_changed_is_refused_with_the_code_of_the_conflict() {
        let s = settings();
        let sim = SimServer::new();
        let mut c = client(&sim, &s);
        let first = save_project(&mut c, &s, "p-1", "board.json", b"one", None, Lang::En).unwrap();
        sim.write_file(APP, "p-1", "board.json", b"theirs");
        let e = save_project(
            &mut c,
            &s,
            "p-1",
            "board.json",
            b"mine",
            first.tag.as_deref(),
            Lang::Es,
        )
        .unwrap_err();
        assert_eq!(e.code, PROJECT_CHANGED);
        assert!(e
            .message(Lang::Es)
            .starts_with("PHOENIX-SERVER-016: Alguien cambió"));
        // nothing was overwritten
        assert_eq!(
            sim.file(APP, "p-1", "board.json").as_deref(),
            Some(b"theirs".as_slice())
        );
        // reading it again gives the tag to write from
        let read = load_project(&mut c, &s, "p-1", "board.json", None, Lang::En).unwrap();
        assert_eq!(read.bytes, b"theirs");
        let ok = save_project(
            &mut c,
            &s,
            "p-1",
            "board.json",
            b"mine",
            read.tag.as_deref(),
            Lang::En,
        )
        .unwrap();
        assert_eq!(ok.version, Some(3));
    }

    #[test]
    fn a_file_is_read_with_its_tag_and_an_older_version_can_be_asked_for() {
        let s = settings();
        let sim = SimServer::new();
        let mut c = client(&sim, &s);
        let first = save_project(&mut c, &s, "p", "a.json", b"v1", None, Lang::En).unwrap();
        let second = save_project(
            &mut c,
            &s,
            "p",
            "a.json",
            b"v2",
            first.tag.as_deref(),
            Lang::En,
        )
        .unwrap();
        let now = load_project(&mut c, &s, "p", "a.json", None, Lang::En).unwrap();
        assert_eq!((now.bytes.as_slice(), now.tag), (&b"v2"[..], second.tag));
        let old = load_project(&mut c, &s, "p", "a.json", Some(1), Lang::En).unwrap();
        assert_eq!(old.bytes, b"v1");
        let zero = load_project(&mut c, &s, "p", "a.json", Some(0), Lang::En).unwrap_err();
        assert_eq!(zero.code, OPTION_INVALID);
        let missing = load_project(&mut c, &s, "p", "nope.json", None, Lang::En).unwrap_err();
        assert_eq!(missing.code, SERVER_FAILED);
    }

    #[test]
    fn nothing_goes_out_when_the_settings_or_the_file_do_not_allow_it() {
        let sim = SimServer::new();
        let s = settings();
        let mut c = client(&sim, &s);
        let mut off = settings();
        off.allow_network = false;
        let e = save_project(&mut c, &off, "p", "a.json", b"x", None, Lang::En).unwrap_err();
        assert_eq!(e.code, NETWORK_NOT_ALLOWED);
        let mut no_token = settings();
        no_token.link.set("server_token", "").unwrap();
        let e = load_project(&mut c, &no_token, "p", "a.json", None, Lang::En).unwrap_err();
        assert_eq!(e.code, NOT_CONFIGURED);
        let e = save_project(&mut c, &s, "p", "a.json", b"", None, Lang::En).unwrap_err();
        assert_eq!(e.code, NOTHING_TO_SEND);
        let mut small = settings();
        small.project_max_mib = 1;
        let big = vec![7u8; MIB + 1];
        let e = save_project(&mut c, &small, "p", "a.json", &big, None, Lang::En).unwrap_err();
        assert_eq!(e.code, LIMIT_EXCEEDED);
        let ok = vec![7u8; MIB];
        assert!(save_project(&mut c, &small, "p", "a.json", &ok, None, Lang::En).is_ok());
        let e = save_project(&mut c, &s, "p", "a.json", b"x", Some("a b"), Lang::En).unwrap_err();
        assert_eq!(e.code, OPTION_INVALID);
        assert_eq!(sim.file_versions(APP, "p", "a.json"), 1);
    }

    #[test]
    fn the_report_of_the_store_is_written_in_each_language_with_the_tag() {
        let saved = Saved {
            project: "p-1".into(),
            name: "board.json".into(),
            bytes: 1234,
            tag: Some("v3-abc".into()),
            version: Some(3),
            unchanged: false,
            created: false,
        };
        let mut seen = Vec::new();
        for lang in Lang::ALL {
            let text = saved_text(&saved, lang);
            assert!(text.contains("p-1/board.json"), "{text}");
            assert!(text.contains("1234") && text.contains("v3-abc"), "{text}");
            assert_eq!(text.lines().count(), 4, "{text}");
            assert!(!seen.contains(&text), "{lang:?} repeats a text");
            seen.push(text);
        }
        let unchanged = Saved {
            unchanged: true,
            version: None,
            ..saved.clone()
        };
        assert_eq!(saved_text(&unchanged, Lang::En).lines().count(), 3);
        assert!(saved_text(&unchanged, Lang::En).starts_with("Already in the store"));
        let created = Saved {
            created: true,
            ..saved.clone()
        };
        assert!(saved_text(&created, Lang::Pt).starts_with("Criado no armazém"));
        let fetched = Fetched {
            project: "p-1".into(),
            name: "board.json".into(),
            tag: None,
            bytes: vec![0; 5],
        };
        assert!(fetched_text(&fetched, Lang::De).starts_with("Aus dem Speicher gelesen"));
        assert!(fetched_text(&fetched, Lang::En).contains("version tag: -"));
    }

    #[test]
    fn the_json_of_the_store_is_valid_and_the_same_every_time() {
        let saved = Saved {
            project: "p".into(),
            name: "a\"b".into(),
            bytes: 9,
            tag: None,
            version: None,
            unchanged: false,
            created: true,
        };
        let text = saved_json(&saved);
        assert_eq!(text, saved_json(&saved));
        let json = Json::parse(&text).unwrap();
        assert_eq!(
            json.get("format").and_then(Json::as_str),
            Some(STORE_FORMAT)
        );
        assert_eq!(json.get("name").and_then(Json::as_str), Some("a\"b"));
        assert_eq!(json.get("created").and_then(Json::as_bool), Some(true));
        assert!(matches!(json.get("tag"), Some(Json::Null)));
        let fetched = Fetched {
            project: "p".into(),
            name: "n".into(),
            tag: Some("t1".into()),
            bytes: vec![1, 2, 3],
        };
        let json = Json::parse(&fetched_json(&fetched)).unwrap();
        assert_eq!(json.get("tag").and_then(Json::as_str), Some("t1"));
        assert_eq!(json.get("bytes").and_then(Json::as_u64), Some(3));
        // the codes this file uses are registered ones
        for code in [PROJECT_NEEDS_TAG, PROJECT_CHANGED, PROJECT_NAME_INVALID] {
            assert!(!crate::codes::title(code, Lang::En).is_empty());
        }
        assert_ne!(DRY_RUN, SETTINGS_INVALID);
    }

    /// A generator with a fixed seed, so every run makes the same cases.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
    }

    #[test]
    fn what_is_saved_is_read_back_and_a_chain_of_tags_never_loses_a_version() {
        let s = settings();
        let sim = SimServer::new();
        let mut c = client(&sim, &s);
        let mut rng = Lcg(0x57_0001);
        let mut tag: Option<String> = None;
        let mut current: Vec<u8> = Vec::new();
        let mut versions = 0usize;
        for _ in 0..60 {
            let length = 1 + (rng.next() % 300) as usize;
            let bytes: Vec<u8> = (0..length).map(|_| rng.next() as u8).collect();
            let saved = save_project(
                &mut c,
                &s,
                "p-9",
                "a/b.bin",
                &bytes,
                tag.as_deref(),
                Lang::En,
            )
            .unwrap();
            if bytes != current {
                versions += 1;
            }
            assert_eq!(saved.unchanged, bytes == current);
            current = bytes;
            tag = saved.tag.clone();
            let read = load_project(&mut c, &s, "p-9", "a/b.bin", None, Lang::En).unwrap();
            assert_eq!(read.bytes, current);
            assert_eq!(read.tag, tag);
            // a write from the tag before this one is always refused
            if rng.next().is_multiple_of(5) {
                let stale = save_project(
                    &mut c,
                    &s,
                    "p-9",
                    "a/b.bin",
                    b"stale",
                    Some("no-such-version"),
                    Lang::En,
                )
                .unwrap_err();
                assert_eq!(stale.code, PROJECT_CHANGED);
            }
        }
        assert_eq!(sim.file_versions(APP, "p-9", "a/b.bin"), versions);
    }

    #[test]
    fn random_names_are_safe_exactly_when_every_piece_is_made_of_the_allowed_characters() {
        let pieces = [
            "a", "B2", "x.y", "..", ".", "", "a b", "a?b", "_-.", "\u{e9}",
        ];
        let mut rng = Lcg(0x57_0002);
        let (mut safe, mut unsafe_) = (0, 0);
        for _ in 0..400 {
            let count = 1 + (rng.next() % 3) as usize;
            let name = (0..count)
                .map(|_| pieces[(rng.next() % pieces.len() as u64) as usize])
                .collect::<Vec<_>>()
                .join("/");
            let expected = name.split('/').all(|p| {
                !p.is_empty()
                    && p != "."
                    && p != ".."
                    && p.chars()
                        .all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c))
            });
            assert_eq!(file_name_is_safe(&name), expected, "{name:?}");
            assert_eq!(store_path("p", &name).is_ok(), expected, "{name:?}");
            if expected {
                safe += 1;
            } else {
                unsafe_ += 1;
            }
        }
        assert!(safe > 20 && unsafe_ > 20, "{safe} {unsafe_}");
    }
}
