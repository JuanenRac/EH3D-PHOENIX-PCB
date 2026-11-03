//! `project.json`: the manifest of a `.phoenixproj`.
//!
//! The file is canonical JSON, so the keys appear sorted and the same manifest gives the same
//! bytes. It names the domains the project holds (and whether each is critical), the links between
//! objects (strong or weak), the migrations already applied and where the project came from.
//! Dates are supplied by the caller and are ordinary content; the name and version of the program
//! that wrote the file belong to `audit/`, never here.

use std::collections::BTreeMap;

use eh3d_phoenix_model::ObjectId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The format version this reader writes and fully understands.
pub const FORMAT_VERSION: &str = "0.1.0";
/// Major number of the format.
pub const FORMAT_MAJOR: u64 = 0;
/// Minor number of the format: while the major is 0, compatibility is guaranteed only within the
/// same minor.
pub const FORMAT_MINOR: u64 = 1;
/// Longest project name in bytes.
pub const MAX_NAME_BYTES: usize = 256;

/// The domain folders this reader knows.
pub const KNOWN_DOMAINS: &[&str] = &[
    "design",
    "mechanical",
    "simulation",
    "thermal",
    "rf",
    "results",
    "history",
    "passport",
    "models",
    "exports",
];

/// Folders that never need a domain entry: variable audit data, preserved foreign data and the
/// extension folders (`x-<name>/`).
pub fn is_undeclared_root(root: &str) -> bool {
    root == "audit" || root == "opaque" || root.starts_with("x-")
}

/// One domain of the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Domain {
    /// The folder of the domain without the slash, for example `design`.
    pub name: String,
    /// The version of the schema of the domain.
    pub version: String,
    /// True when the domain changes electrical or geometric meaning: it must be understood or the
    /// project is not edited.
    pub critical: bool,
}

/// How much a reference matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Strength {
    /// Inside and between critical domains: it must never dangle.
    Strong,
    /// From a domain that is not critical: it may point to something deleted later.
    Weak,
}

/// A reference from one object to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// The object that refers.
    pub from: ObjectId,
    /// What kind of relation, for example `footprint_of`.
    pub kind: String,
    /// Strong or weak.
    pub strength: Strength,
    /// The object referred to.
    pub to: ObjectId,
}

/// The manifest of a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Optional creation time as text, supplied by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// The domains the project holds.
    pub domains: Vec<Domain>,
    /// The version of the layout of the project, `major.minor.patch`.
    pub format_version: String,
    /// The identity of the project, which survives copies.
    pub id: ObjectId,
    /// The references between objects.
    #[serde(default)]
    pub links: Vec<Link>,
    /// The names of the migrations already applied, oldest first.
    #[serde(default)]
    pub migrations_applied: Vec<String>,
    /// Optional time of the last change as text, supplied by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    /// The name of the project.
    pub name: String,
    /// Where the project came from (a free object: kind, tool, version, hash of the source).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Value>,
    /// Fields this reader does not know. They are kept and written back; a field that is an object
    /// marked `"must_understand": true` makes the reader refuse the project.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Why a manifest is not acceptable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestProblem {
    /// The name is empty, too long or has control characters.
    BadName,
    /// The version is not `major.minor.patch` with numbers.
    BadVersion(String),
    /// Two domains have the same name.
    DuplicateDomain(String),
    /// A domain name is empty or has characters other than lower-case letters and digits.
    BadDomainName(String),
    /// A domain version is not `major.minor.patch`.
    BadDomainVersion(String),
    /// A link has an empty kind.
    EmptyLinkKind,
    /// An unknown top-level field is marked `must_understand`.
    MustUnderstand(String),
}

/// Reads `major.minor.patch` as three numbers.
pub fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() || text.starts_with('+') {
        return None;
    }
    Some((major, minor, patch))
}

impl Manifest {
    /// A manifest at the current format version with no domains and no links.
    pub fn new(id: ObjectId, name: &str) -> Manifest {
        Manifest {
            created: None,
            domains: Vec::new(),
            format_version: FORMAT_VERSION.to_owned(),
            id,
            links: Vec::new(),
            migrations_applied: Vec::new(),
            modified: None,
            name: name.to_owned(),
            origin: None,
            extra: BTreeMap::new(),
        }
    }

    /// Declares a domain.
    pub fn with_domain(mut self, name: &str, version: &str, critical: bool) -> Manifest {
        self.domains.push(Domain {
            name: name.to_owned(),
            version: version.to_owned(),
            critical,
        });
        self
    }

    /// The domain called `name`, if the manifest declares it.
    pub fn domain(&self, name: &str) -> Option<&Domain> {
        self.domains.iter().find(|d| d.name == name)
    }

    /// Every problem of the manifest (not its relation to the other entries).
    pub fn problems(&self) -> Vec<ManifestProblem> {
        let mut out = Vec::new();
        if self.name.is_empty()
            || self.name.len() > MAX_NAME_BYTES
            || self.name.chars().any(char::is_control)
        {
            out.push(ManifestProblem::BadName);
        }
        if parse_version(&self.format_version).is_none() {
            out.push(ManifestProblem::BadVersion(self.format_version.clone()));
        }
        let mut seen = std::collections::BTreeSet::new();
        for d in &self.domains {
            if d.name.is_empty()
                || !d
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            {
                out.push(ManifestProblem::BadDomainName(d.name.clone()));
            }
            if !seen.insert(d.name.as_str()) {
                out.push(ManifestProblem::DuplicateDomain(d.name.clone()));
            }
            if parse_version(&d.version).is_none() {
                out.push(ManifestProblem::BadDomainVersion(d.name.clone()));
            }
        }
        if self.links.iter().any(|l| l.kind.is_empty()) {
            out.push(ManifestProblem::EmptyLinkKind);
        }
        for (key, value) in &self.extra {
            if value.get("must_understand") == Some(&Value::Bool(true)) {
                out.push(ManifestProblem::MustUnderstand(key.clone()));
            }
        }
        out
    }

    /// The canonical bytes of `project.json`.
    pub fn to_bytes(&self) -> Result<Vec<u8>, eh3d_container::CanonicalError> {
        eh3d_container::to_canonical_file_bytes(self)
    }

    /// Reads a manifest. Unknown fields are kept in `extra`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Manifest, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_phoenix_model::{DeterministicRandom, IdGenerator, StepClock};

    pub(crate) fn id(n: u64) -> ObjectId {
        let mut generator = IdGenerator::new(
            StepClock::new(1_700_000_000_000 + n, 1),
            DeterministicRandom::new(n),
        );
        generator.generate().unwrap()
    }

    #[test]
    fn a_manifest_round_trips_through_canonical_json_with_sorted_keys() {
        let mut m = Manifest::new(id(1), "Example").with_domain("design", "0.1.0", true);
        m.links.push(Link {
            from: id(2),
            kind: "footprint_of".into(),
            strength: Strength::Strong,
            to: id(3),
        });
        let bytes = m.to_bytes().unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.starts_with("{\"domains\":[{\"critical\":true,\"name\":\"design\",\"version\":\"0.1.0\"}],\"format_version\":\"0.1.0\",\"id\":\""));
        assert!(text.ends_with("\n"));
        assert_eq!(Manifest::from_bytes(&bytes).unwrap(), m);
        assert_eq!(
            m.to_bytes().unwrap(),
            bytes,
            "the same manifest gives the same bytes"
        );
    }

    #[test]
    fn unknown_fields_are_kept_and_written_back() {
        let m = Manifest::new(id(1), "x");
        let mut value: Value = serde_json::from_slice(&m.to_bytes().unwrap()).unwrap();
        value["zzz_future"] = serde_json::json!({"a": 1});
        let read = Manifest::from_bytes(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(read.extra["zzz_future"], serde_json::json!({"a": 1}));
        assert!(read.problems().is_empty());
        let again: Value = serde_json::from_slice(&read.to_bytes().unwrap()).unwrap();
        assert_eq!(again["zzz_future"], serde_json::json!({"a": 1}));
    }

    #[test]
    fn a_field_marked_must_understand_is_a_problem() {
        let mut m = Manifest::new(id(1), "x");
        m.extra.insert(
            "future".into(),
            serde_json::json!({"must_understand": true}),
        );
        m.extra.insert(
            "harmless".into(),
            serde_json::json!({"must_understand": false}),
        );
        assert_eq!(
            m.problems(),
            [ManifestProblem::MustUnderstand("future".into())]
        );
    }

    #[test]
    fn bad_names_versions_and_domains_are_found() {
        let mut m = Manifest::new(id(1), "");
        m.format_version = "0.1".into();
        m.domains.push(Domain {
            name: "Design".into(),
            version: "x".into(),
            critical: true,
        });
        m.domains.push(Domain {
            name: "design2".into(),
            version: "0.1.0".into(),
            critical: false,
        });
        m.domains.push(Domain {
            name: "design2".into(),
            version: "0.1.0".into(),
            critical: false,
        });
        let p = m.problems();
        assert!(p.contains(&ManifestProblem::BadName));
        assert!(p.contains(&ManifestProblem::BadVersion("0.1".into())));
        assert!(p.contains(&ManifestProblem::BadDomainName("Design".into())));
        assert!(p.contains(&ManifestProblem::BadDomainVersion("Design".into())));
        assert!(p.contains(&ManifestProblem::DuplicateDomain("design2".into())));
    }

    #[test]
    fn versions_are_three_plain_numbers() {
        assert_eq!(parse_version("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version("10.20.30"), Some((10, 20, 30)));
        for bad in [
            "", "1", "1.2", "1.2.3.4", "a.b.c", "1.2.-3", "+1.2.3", "1..3",
        ] {
            assert_eq!(parse_version(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_text_that_is_not_a_v7_id_does_not_parse() {
        let bad = br#"{"domains":[],"format_version":"0.1.0","id":"00000000-0000-4000-8000-000000000001","name":"x"}"#;
        assert!(Manifest::from_bytes(bad).is_err());
    }
}
