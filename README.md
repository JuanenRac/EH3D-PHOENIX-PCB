<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# 🛠️ EH3D-PHOENIX-PCB

<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB: planned as an EDA environment with schematic capture, PCB layout, verification tools, circuit simulation and specialized modules for switch-mode power supplies and RF. Only the corpus inventory is written so far.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-23-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust workspace with one command, `phoenix inventory`, that counts the design files of a private folder without showing any name or value: files by extension, kind and declared version, and one fingerprint of the content. It has 20 tests of its own and they pass on a computer. It has only been run on folders made by its own tests, never on real design files. There is no design model, no reader or writer of design files, no editor and no rules: all of the EDA function is still planned.

---

> **Safety and security status:** this repository is at its first steps and is not a safety system. Nothing written here replaces the limits, interlocks and emergency stop of the machine and its controller.

## 📋 At a glance

| | |
|---|---|
| Version | `0.0.1` |
| Maturity | Scaffolding |
| Tests | 23 |
| Stack | Rust, Python 3.10+ |
| On real hardware | Not run on hardware yet |

**Jump to:** [Overview](#-overview) · [Architecture](#-architecture) · [Quick start](#-quick-start) · [Testing](#-testing) · [Repository Structure](#-repository-structure) · [Related Projects](#-related-projects)

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Repository checks | 🟡 | The repository check, the check of the pictures and the 3 tests of the base run in CI and pass. |
| Tests | 🟡 | The 20 tests of the Rust code and the 3 of the base pass on a computer. They use only folders made by the tests themselves; none has run on real design files. |
| Real hardware | 🟠 | Not run on any machine or board: nothing here has been tried on hardware. |
| Planned function | 🟠 | Written: the redacted inventory of a corpus. Not started: the design model, the readers and writers, the rules, the viewer and editors, simulation and everything else planned. |
| Requirements | 🟠 | 1 of 30 requirements (the redacted mode) is at prototype, 29 are only planned and none is validated. |

## 🎯 Overview

* **What exists:** a workspace with a library that counts a private corpus of design files in redacted mode (only counts, kinds, declared versions and one fingerprint, never a name or a value) and a command line, `phoenix inventory`. It looks at the first bytes of a file and does not read designs.

* **Planned:** EH3D-PHOENIX-PCB is meant to be an EDA environment with schematic capture, PCB layout, verification tools, circuit simulation and specialized modules for switch-mode power supplies and RF. It is planned as a Rust program with a window for the desktop (a workspace of engine crates, a command-line interface and an application) that works on its own and can use the server for computing tasks. None of this exists yet.

## 🧩 Role in EH3D

EH3D-PHOENIX-PCB is the main project of the PHOENIX family of the EH3D Fabrication Suite; the other repositories of the family build on it. What it will do, what it will not do yet, its requirements and its relations with other repositories are written in the `docs/` folder.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-PHOENIX-PCB architecture: who uses it, what it holds and what it depends on" width="100%">
</p>

How the parts of this repository fit together: who uses it, what it holds and what it depends on. The picture is drawn from `docs/architecture.json` and checked against the code.

## 🚀 Quick start

You need Python 3.10 or newer for the checks and a Rust toolchain for the code.

```bash
cargo build --release
cargo test
cargo run -p eh3d-phoenix-cli -- inventory <folder>
python -I tools/ci_validate.py
python -I tools/make_architecture.py --check
python -I tools/make_structure.py --check --readmes
python -I -m unittest discover -s tools -p "test_*.py"
```

## 🧪 Testing

The code has 20 tests (`cargo test`) and the base has 3 (`python -I -m unittest discover -s tools -p "test_*.py"`); all pass on a computer. The CI runs the repository check, the check of the pictures, the tests of the base and, for the Rust code, the format check, clippy with warnings as errors and the tests on Linux and Windows.

## 🏗️ Build & Release

The version lives in the `VERSION` file and in `eh3d.project.json`, and each version has its entry in [CHANGELOG.md](CHANGELOG.md). `cargo build --release` builds the library and the `phoenix` program; releases are published on GitHub.

## 📂 Repository Structure

<p align="center">
  <img src="images/repository-structure.svg" alt="Repository structure of EH3D-PHOENIX-PCB" width="100%">
</p>

<details>
<summary>Text version of the tree</summary>

```text
EH3D-PHOENIX-PCB/
├── docs/
│   ├── architecture.json structure.json         the data of the two pictures
│   ├── SCOPE.md REQUIREMENTS.md                 its scope and its requirements
│   ├── DEPENDENCIES.md THREAT_MODEL.md          relations with other repositories and the threat model
│   ├── DECISIONS.md STATUS.md                   decisions that apply here and the honest status
│   ├── NOT_YET.md RULES.md                      ideas left for later and rules for writing code
│   └── ROADMAP.md                               the map of phases, from foundations to the all-in-one suite
├── crates/                                      the corpus inventory and the command line
├── config/                                      the settings file (every tuned value)
├── Cargo.toml Cargo.lock                        the Rust workspace and its locked dependencies
├── tools/                                       the repository check, its tests and the picture tools
├── images/                                      banner and the Architecture and Repository Structure pictures
├── .github/                                     CI and dashboard workflows, issue and pull-request templates
├── eh3d.project.json                            manifest: version, role, maturity and build command
├── VERSION                                      the version of the repository
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

</details>

## 🗺️ Roadmap & Known limits

**Known limits today**

* Early code: only the corpus inventory exists, and it has run only on folders made by its own tests, never on real design files.
* There is no design model, no reader or writer of design files, no editor and no rules; no requirement other than the redacted mode has been started.
* The user interface, the geometry library and the coordinate range are still open decisions (see `docs/STATUS.md`).

**Next**

* Wait until the contracts this project depends on are accepted (see `docs/DEPENDENCIES.md`), then write the model and the first readers with tests and replace the status rows with real results.

## 🌍 Languages

README and texts exist in English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese.

## 🔗 Related Projects

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): the map of the suite, its architecture, the conventions and the shared checks
* Planned relations, not built yet (EH3D-COMMON, EH3D-CORE-LIBS, EH3D-SERVER, EH3D-PHOENIX-CAD, EH3D-VALIDATION-LAB, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE): see [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

## 📚 Documentation

* [The conventions of the suite](https://github.com/JuanenRac/EH3D-DOCS/blob/main/docs/CONVENTIONS.md)
* [Changelog](CHANGELOG.md)
* [Scope, requirements, dependencies, threat model and decisions](docs/SCOPE.md)

## 🤝 Contributing & Community

* Before a pull request run `python -I tools/ci_validate.py` and the tests of the base: all must pass. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).
* The README exists in eight languages: a change to the text is made in all eight files, and the two pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`.
* Report a vulnerability privately: [SECURITY.md](SECURITY.md). Questions, ideas and reports: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.
