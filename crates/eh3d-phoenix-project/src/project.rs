//! A `.phoenixproj` in memory, and reading and writing it on the shared container.
//!
//! Opening follows the rules of the format: a project of a newer layout is refused; an older one is
//! brought up by registered migrations or refused; a critical domain that is unknown or newer is
//! refused (or opened read-only when the setting allows it); a domain that is not critical and not
//! understood is kept untouched; an object marked `must_understand` that the reader does not know
//! makes the load fail; an entry in a domain the manifest does not declare is refused.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io;
use std::path::Path;

use eh3d_container::spec::{INTEGRITY_ENTRY, PROJECT_ENTRY};
use eh3d_container::{AtomicOptions, Container, ContainerError, ContainerSpec, ContainerWriter};
use eh3d_diagnostic::{Diagnostic, Severity};
use eh3d_phoenix_model::ObjectId;
use serde_json::Value;

use crate::codes::{self, diagnostic};
use crate::manifest::{
    is_undeclared_root, parse_version, Manifest, ManifestProblem, Strength, FORMAT_MAJOR,
    FORMAT_MINOR, KNOWN_DOMAINS,
};

/// A step that brings a project from one minor version to the next.
#[derive(Clone)]
pub struct Migration {
    /// The name recorded in `migrations_applied`.
    pub name: &'static str,
    /// The minor version it starts from.
    pub from_minor: u64,
    /// The minor version it ends at.
    pub to_minor: u64,
    /// The change. It may rewrite the manifest and the entries.
    pub apply: fn(&mut Project) -> Result<(), String>,
}

/// Settings of opening a project.
#[derive(Clone)]
pub struct OpenOptions {
    /// What the container accepts (roots, limits, budget). The default is the `.phoenixproj` preset.
    pub spec: ContainerSpec,
    /// When a critical domain is unknown or newer: false refuses the project (the conservative
    /// default), true opens it read-only.
    pub read_only_if_unknown_critical: bool,
    /// The object types that carry `must_understand` and that this reader knows.
    pub understood_types: BTreeSet<String>,
    /// The migrations this reader has, by the minor version they start from.
    pub migrations: Vec<Migration>,
    /// The largest JSON entry that is scanned for `must_understand`.
    pub max_scan_bytes: u64,
}

impl Default for OpenOptions {
    fn default() -> Self {
        OpenOptions {
            spec: ContainerSpec::phoenixproj(FORMAT_MAJOR, FORMAT_MINOR),
            read_only_if_unknown_critical: false,
            understood_types: BTreeSet::new(),
            migrations: Vec::new(),
            max_scan_bytes: 32 * 1024 * 1024,
        }
    }
}

/// Why a project could not be opened or written.
#[derive(Debug)]
pub enum ProjectError {
    /// The container refused it (a code `EH3D-PKG-*`).
    Container(ContainerError),
    /// The project layer refused it, with the diagnostic that says why.
    Refused(Box<Diagnostic>),
    /// The project is read-only and cannot be saved.
    ReadOnly,
    /// The file system failed.
    Io(io::Error),
}

impl ProjectError {
    /// The diagnostic of the failure, when it has a code.
    pub fn diagnostic(&self) -> Option<Diagnostic> {
        match self {
            ProjectError::Container(ContainerError::Rejected(v)) => Some(v.diagnostic()),
            ProjectError::Refused(d) => Some((**d).clone()),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::Container(e) => e.fmt(f),
            ProjectError::Refused(d) => f.write_str(&d.message),
            ProjectError::ReadOnly => f.write_str("the project was opened read-only"),
            ProjectError::Io(e) => write!(f, "input/output error: {e}"),
        }
    }
}

impl std::error::Error for ProjectError {}

impl From<ContainerError> for ProjectError {
    fn from(e: ContainerError) -> Self {
        ProjectError::Container(e)
    }
}

impl From<io::Error> for ProjectError {
    fn from(e: io::Error) -> Self {
        ProjectError::Io(e)
    }
}

fn refuse(code: &'static str, detail: &str) -> ProjectError {
    ProjectError::Refused(Box::new(diagnostic(code, Severity::Error, detail)))
}

/// A project: its manifest and the bytes of every other entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    /// The manifest (`project.json`).
    pub manifest: Manifest,
    entries: BTreeMap<String, Vec<u8>>,
    read_only: bool,
}

/// A project that was opened, with what the reader had to say about it.
#[derive(Debug)]
pub struct Opened {
    /// The project.
    pub project: Project,
    /// Information and warnings found while opening (errors refuse the project instead).
    pub diagnostics: Vec<Diagnostic>,
}

fn root_of(name: &str) -> &str {
    name.split('/').next().unwrap_or(name)
}

impl Project {
    /// An empty project.
    pub fn new(manifest: Manifest) -> Project {
        Project {
            manifest,
            entries: BTreeMap::new(),
            read_only: false,
        }
    }

    /// Adds or replaces an entry. The name is checked when the project is written.
    pub fn set_entry(&mut self, name: &str, bytes: Vec<u8>) {
        self.entries.insert(name.to_owned(), bytes);
    }

    /// The bytes of an entry.
    pub fn entry(&self, name: &str) -> Option<&[u8]> {
        self.entries.get(name).map(Vec::as_slice)
    }

    /// The names of the entries other than `project.json`, in order.
    pub fn entry_names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// True when the project was opened read-only and cannot be saved.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Writes the project to `path`: canonical `project.json`, the entries in the order of the
    /// preset and a canonical `integrity.json`, replaced atomically. The bytes depend only on the
    /// content, so two saves of the same project are identical.
    pub fn save_to_path(&self, path: &Path, atomic: &AtomicOptions) -> Result<(), ProjectError> {
        self.save_with(
            path,
            atomic,
            &ContainerSpec::phoenixproj(FORMAT_MAJOR, FORMAT_MINOR),
        )
    }

    /// Like [`Project::save_to_path`] with the container settings of the caller.
    pub fn save_with(
        &self,
        path: &Path,
        atomic: &AtomicOptions,
        spec: &ContainerSpec,
    ) -> Result<(), ProjectError> {
        if self.read_only {
            return Err(ProjectError::ReadOnly);
        }
        let folder = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut writer = ContainerWriter::new(spec, folder);
        let manifest = self
            .manifest
            .to_bytes()
            .map_err(|e| refuse(codes::MANIFEST_INVALID, &e.to_string()))?;
        writer.add(PROJECT_ENTRY, &manifest)?;
        for (name, bytes) in &self.entries {
            writer.add(name, bytes)?;
        }
        writer.write_to_path(path, atomic)?;
        Ok(())
    }

    /// Opens the project at `path`.
    pub fn open_path(path: &Path, options: &OpenOptions) -> Result<Opened, ProjectError> {
        let file = File::open(path)?;
        let mut container = Container::open_checked(file, &options.spec, version_of)?;
        let names: Vec<String> = container
            .entries()
            .map(|e| e.name.clone())
            .filter(|n| n != INTEGRITY_ENTRY)
            .collect();
        let mut diagnostics = Vec::new();

        let manifest_bytes = container.read_limited(
            PROJECT_ENTRY,
            options.spec.limits.max_entry_bytes_for(PROJECT_ENTRY),
        )?;
        let manifest = Manifest::from_bytes(&manifest_bytes)
            .map_err(|_| refuse(codes::MANIFEST_INVALID, "project.json cannot be read"))?;
        if let Some(problem) = manifest.problems().into_iter().next() {
            return Err(match problem {
                ManifestProblem::MustUnderstand(key) => refuse(
                    codes::MUST_UNDERSTAND,
                    &format!("the top-level field '{key}'"),
                ),
                other => refuse(codes::MANIFEST_INVALID, &format!("{other:?}")),
            });
        }

        let mut project = Project::new(manifest);
        for name in names.iter().filter(|n| n.as_str() != PROJECT_ENTRY) {
            let bytes =
                container.read_limited(name, options.spec.limits.max_entry_bytes_for(name))?;
            project.entries.insert(name.clone(), bytes);
        }

        migrate(&mut project, options)?;
        check_domains(&mut project, options, &mut diagnostics)?;
        check_entry_domains(&project)?;
        scan_must_understand(&project, options)?;
        Ok(Opened {
            project,
            diagnostics,
        })
    }
}

/// The `(major, minor)` of `format_version` in `project.json`, for the container's version check.
fn version_of(project_json: &[u8]) -> Option<(u64, u64)> {
    let value: Value = serde_json::from_slice(project_json).ok()?;
    let (major, minor, _) = parse_version(value.get("format_version")?.as_str()?)?;
    Some((major, minor))
}

/// Brings an older project up to the current minor by the registered steps.
fn migrate(project: &mut Project, options: &OpenOptions) -> Result<(), ProjectError> {
    let (major, mut minor, _) =
        parse_version(&project.manifest.format_version).expect("checked by the manifest");
    if major != FORMAT_MAJOR {
        return Err(refuse(
            codes::MIGRATION_MISSING,
            &format!("format version {}", project.manifest.format_version),
        ));
    }
    while minor < FORMAT_MINOR {
        let Some(step) = options.migrations.iter().find(|m| m.from_minor == minor) else {
            return Err(refuse(
                codes::MIGRATION_MISSING,
                &format!("format version {}", project.manifest.format_version),
            ));
        };
        (step.apply)(project).map_err(|why| refuse(codes::MIGRATION_FAILED, &why))?;
        minor = step.to_minor;
        project.manifest.format_version = format!("{FORMAT_MAJOR}.{minor}.0");
        project
            .manifest
            .migrations_applied
            .push(step.name.to_owned());
    }
    Ok(())
}

/// Decides what to do with domains this reader does not understand.
fn check_domains(
    project: &mut Project,
    options: &OpenOptions,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), ProjectError> {
    for domain in project.manifest.domains.clone() {
        let understood = KNOWN_DOMAINS.contains(&domain.name.as_str())
            && parse_version(&domain.version)
                .is_some_and(|(major, minor, _)| major == FORMAT_MAJOR && minor == FORMAT_MINOR);
        if understood {
            continue;
        }
        if domain.critical {
            if options.read_only_if_unknown_critical {
                project.read_only = true;
                diagnostics.push(diagnostic(
                    codes::OPENED_READ_ONLY,
                    Severity::Warning,
                    &format!("domain '{}' version {}", domain.name, domain.version),
                ));
            } else {
                return Err(refuse(
                    codes::CRITICAL_DOMAIN_UNKNOWN,
                    &format!("domain '{}' version {}", domain.name, domain.version),
                ));
            }
        } else {
            diagnostics.push(diagnostic(
                codes::DOMAIN_PRESERVED,
                Severity::Info,
                &format!("domain '{}' version {}", domain.name, domain.version),
            ));
        }
    }
    Ok(())
}

/// Every entry must live in a declared domain, unless its folder never needs a declaration.
fn check_entry_domains(project: &Project) -> Result<(), ProjectError> {
    for name in project.entry_names() {
        let root = root_of(name);
        if !is_undeclared_root(root) && project.manifest.domain(root).is_none() {
            return Err(refuse(
                codes::DOMAIN_UNDECLARED,
                &format!("the entry is in the folder '{root}'"),
            ));
        }
    }
    Ok(())
}

/// The domains whose entries this reader reads as data: declared, understood or not.
fn scanned(project: &Project, options: &OpenOptions, name: &str) -> bool {
    let root = root_of(name);
    if is_undeclared_root(root) || !name.ends_with(".json") {
        return false;
    }
    project.manifest.domain(root).is_some_and(|d| {
        KNOWN_DOMAINS.contains(&d.name.as_str())
            && parse_version(&d.version)
                .is_some_and(|(major, minor, _)| major == FORMAT_MAJOR && minor == FORMAT_MINOR)
    }) && project
        .entry(name)
        .is_some_and(|b| b.len() as u64 <= options.max_scan_bytes)
}

fn first_must_understand(value: &Value, understood: &BTreeSet<String>) -> Option<String> {
    match value {
        Value::Object(map) => {
            if map.get("must_understand") == Some(&Value::Bool(true)) {
                let kind = map.get("type").and_then(Value::as_str).unwrap_or("");
                if !understood.contains(kind) {
                    return Some(kind.to_owned());
                }
            }
            map.values()
                .find_map(|v| first_must_understand(v, understood))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|v| first_must_understand(v, understood)),
        _ => None,
    }
}

/// An object marked `must_understand` whose type is unknown makes the load fail.
fn scan_must_understand(project: &Project, options: &OpenOptions) -> Result<(), ProjectError> {
    for name in project.entry_names() {
        if !scanned(project, options, name) {
            continue;
        }
        // A JSON entry that does not parse is the domain's own business; the scan only looks at
        // what it can read.
        let Ok(value) = serde_json::from_slice::<Value>(project.entry(name).unwrap_or_default())
        else {
            continue;
        };
        if let Some(kind) = first_must_understand(&value, &options.understood_types) {
            return Err(refuse(
                codes::MUST_UNDERSTAND,
                &format!("an object of type '{kind}' in the entry '{name}'"),
            ));
        }
    }
    Ok(())
}

/// Checks the links of a manifest against the objects that exist. A strong link with a missing end
/// is an error (`PHOENIX-MODEL-001`); a weak one is information (`PHOENIX-MODEL-005`) and is never
/// repaired by guessing.
pub fn check_links(manifest: &Manifest, existing: &BTreeSet<ObjectId>) -> Vec<Diagnostic> {
    manifest
        .links
        .iter()
        .filter(|l| !existing.contains(&l.from) || !existing.contains(&l.to))
        .map(|l| match l.strength {
            Strength::Strong => diagnostic(
                codes::STRONG_REFERENCE_DANGLES,
                Severity::Error,
                &format!("a '{}' link", l.kind),
            ),
            Strength::Weak => diagnostic(
                codes::WEAK_REFERENCE_STALE,
                Severity::Info,
                &format!("a '{}' link", l.kind),
            ),
        })
        .collect()
}
