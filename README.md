<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-PCB
<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB: planned as an EDA environment with schematic capture, PCB layout, verification tools, circuit simulation and specialized modules for switch-mode power supplies and RF. Only the corpus inventory is written so far.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-431-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust workspace whose command `phoenix inventory` counts the design files of a private folder without showing any name or value: files by extension, kind and declared version, and one fingerprint of the content. It has 552 tests of its own and they pass on a computer. It has only been run on folders made by its own tests, never on real design files. It also has the foundations of a design model (integer nanometre and micro-degree values with a checked range, object ids and canonical JSON), the `.phoenixproj` project file (manifest, domains, migrations) and a board model with reversible operations (layers, nets, components, tracks, vias, zones), all tried on generated data only. The board library also finds which copper touches which and checks sizes and clearances against a rule file; a fifth library reads EAGLE XML libraries, boards and schematics into a summary and writes the XML tree back identically. All of it was tried on generated or hand-made files only. A schematic model and its consistency check against the board exist as well. There is no import of foreign designs into the model, no editor, and the rules cover only part of the checks planned: most of the EDA function is still planned. The board model also holds silkscreen text and strokes (with a built-in stroke font) that the rule check and the Gerber export use, and the schematic can be drawn on sheets with placed parts and wires that are compared with the net list and with the board; a board can be written as an EAGLE XML file and read back, which has only been checked on files written by this code and never opened in EAGLE; the contract for pairs of EAGLE binary files and their XML is written, but no reader of the binary files exists. A stack-up and transmission-line calculator (`phoenix stackup`: impedance, effective permittivity, losses, delay and track width of microstrip, stripline, coplanar waveguide and coupled pairs from published formulas) is a prototype, never compared with a field solver or with a measurement. A crate for connectivity and rules (`eh3d-phoenix-rules`) reads the limits from a rule file or from the design rules of an EAGLE board, checks a board in a fixed order (width, via, drill, ring, clearance, edge, drill spacing, solder mask, zones, silkscreen), finds shorts and unrouted nets and compares the nets of a board with those of a schematic, and the commands `phoenix check` and `phoenix connectivity` print the report with the confirmed and the inconclusive findings apart; it has only been tried on boards made by hand and has never been compared with the reference tool. The manufacturing export (`phoenix export`) makes Gerber X2 files for every layer, plated and not plated Excellon drill files, a placement file and a bill of materials with profiles read from a JSON file, a manifest of SHA-256 sums and a check that reads the files back and compares them with the board; it is a prototype whose files have been read only by the parsers of this repository, never opened in an independent viewer or by a manufacturer. An RF toolbox (`eh3d-phoenix-rf`, command `phoenix rf`) reads and writes Touchstone files, converts S, Z, Y and ABCD parameters, computes VSWR, losses, group delay and stability, designs L networks, single stubs and quarter-wave sections for complex impedances, gives the coordinates of the Smith chart and designs Butterworth, Chebyshev and Bessel ladder filters of ideal components; it is a prototype checked only with textbook values, never compared with a network analyser, a field solver or another RF program.

---

> **Safety and security status:** this repository is at its first steps and is not a safety system. Nothing written here replaces the limits, interlocks and emergency stop of the machine and its controller.

## 📋 At a glance

| | |
|---|---|
| Version | `0.0.2` |
| Maturity | Scaffolding |
| Tests | 431 |
| Stack | Rust, Python 3.10+ |
| On real hardware | Not run on hardware yet |

**Jump to:** [Overview](#-overview) · [Architecture](#-architecture) · [Quick start](#-quick-start) · [Testing](#-testing) · [Repository Structure](#-repository-structure) · [Related Projects](#-related-projects)

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Repository checks | 🟡 | The repository check, the check of the pictures and the 3 tests of the base run in CI and pass. |
| Tests | 🟡 | The 552 tests of the Rust code and the 3 of the base pass on a computer. They use only folders made by the tests themselves; none has run on real design files. |
| Real hardware | 🟠 | Not run on any machine or board: nothing here has been tried on hardware. |
| Planned function | 🟠 | Written: the redacted inventory of a corpus, the foundations of the model (units, ids, canonical JSON), the `.phoenixproj` project file, the board model with undo and redo, its connectivity and a first rule check, and a summary reader of EAGLE XML. Not started: the schematic editor, the import of other formats into the model, the writers of other formats, most of the rules, the viewer and editors, simulation and everything else planned. The board model also holds silkscreen text and strokes (with a built-in stroke font) that the rule check and the Gerber export use, and the schematic can be drawn on sheets with placed parts and wires that are compared with the net list and with the board; a board can be written as an EAGLE XML file and read back, which has only been checked on files written by this code and never opened in EAGLE; the contract for pairs of EAGLE binary files and their XML is written, but no reader of the binary files exists. A stack-up and transmission-line calculator (`phoenix stackup`: impedance, effective permittivity, losses, delay and track width of microstrip, stripline, coplanar waveguide and coupled pairs from published formulas) is a prototype, never compared with a field solver or with a measurement. An RF toolbox (`phoenix rf`: Touchstone files, two-port parameters, matching and ideal ladder filters) is a prototype, checked only with textbook values. |
| Requirements | 🟠 | 10 of 30 requirements (the redacted mode, the model foundations, the reversible operations, the project file, the diagnostic codes, the analytic RF calculators the connectivity and rule checks, and the manufacturing export with its package) are at prototype, 20 are only planned and none is validated. |

## 🎯 Overview

* **What exists:** a workspace with a library that counts a private corpus of design files in redacted mode (only counts, kinds, declared versions and one fingerprint, never a name or a value) and a command line, `phoenix inventory`. It looks at the first bytes of a file and does not read designs. A second library holds the foundations of the design model: lengths and angles as integers with a checked range, object ids (UUID v7) and canonical JSON. A third library reads and writes the `.phoenixproj` project on the shared container of EH3D-COMMON: the manifest with its domains, links and migrations, domains it does not understand kept untouched, and the `must_understand` rule. A fourth library holds the board model: layers, nets, footprints, components, tracks with lines and arcs, vias and zones linked by stable ids, with reversible operations and an undo/redo history. Every failure has a stable `PHOENIX-<AREA>-<NNN>` code. The board library also computes the islands of each net and checks widths, drills, annular rings and clearances against a rule file in millimetres, keeping confirmed and inconclusive findings apart. A fifth library reads EAGLE XML libraries, boards and schematics into a summary, writes the XML tree back identically and measures the read rate over copies of a corpus checked by SHA-256, without changing the originals; the command `phoenix eagle-rate` runs it. A sixth library writes the Gerber X2, Excellon and `.eh3dproj` 1.0.0 files of a board through EH3D-CORE-LIBS and EH3D-COMMON, the same bytes for the same board; the files are only read back by the parsers of the same library. A seventh library holds the schematic (symbols, parts and nets linked by stable ids) and checks it against a board: parts without a component, and differing designators, values, footprints and nets, with the codes `PHOENIX-LINK-<NNN>`. A further library, `eh3d-phoenix-rules`, derives the nets from the copper (pads, tracks, vias and zones joined by geometry with a tolerance), compares them with those of the schematic, and runs the design rules in a fixed order against limits read from a rule file or from the design rules of an EAGLE board, with the confirmed and the inconclusive findings in two lists; the commands `phoenix check` and `phoenix connectivity` print its report as text in eight languages or as JSON.

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

The code has 552 tests (`cargo test`) and the base has 3 (`python -I -m unittest discover -s tools -p "test_*.py"`); all pass on a computer. The CI runs the repository check, the check of the pictures, the tests of the base and, for the Rust code, the format check, clippy with warnings as errors and the tests on Linux and Windows.

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
│   ├── ROADMAP.md                               the map of phases, from foundations to the all-in-one suite
│   ├── ERROR_CODES.md                           the diagnostic codes PHOENIX-<AREA>-<NNN>
│   ├── STACKUP.md                               stack-up and line calculations, formulas
│   ├── RF.md                                    RF tools: Touchstone, two-ports, matching, filters
│   ├── GATE_F1.md                               what the gate of the first phase checks and what it does not
│   └── adr/                                     the accepted decision records
├── AGENTS.md                                    rules for anyone who changes the repository
├── deny.toml                                    licence and advisory policy of the dependencies
├── crates/                                      inventory, model, project, board, schematic, EAGLE, rules, hub, export, stack-up, RF, CLI
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

* Early code: only the corpus inventory and the foundations of the model exist; the inventory has run only on folders made by its own tests, never on real design files.
* There is no schematic, no reader or writer of foreign design files, no editor and no rules; the project file and the board model have only been tried on generated data, never on a real design.
* The user interface and the geometry library are still open decisions (see `docs/STATUS.md`).

**Next**

* Wait until the contracts this project depends on are accepted (see `docs/DEPENDENCIES.md`), then write the model and the first readers with tests and replace the status rows with real results.

## 🌍 Languages

README and texts exist in English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese.

## 🔗 Related Projects

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): the map of the suite, its architecture, the conventions and the shared checks
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): built next to this repository; the shared project container (`eh3d-container`) and the common diagnostic (`eh3d-diagnostic`) that this project uses
* Sibling PHOENIX repositories: [EH3D-VALIDATION-LAB](https://github.com/JuanenRac/EH3D-VALIDATION-LAB) (corpus and golden-file oracle) and [EH3D-PHOENIX-CAD](https://github.com/JuanenRac/EH3D-PHOENIX-CAD) (mechanical side), both early code
* Planned relations, not built yet (EH3D-CORE-LIBS, EH3D-SERVER, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE): see [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

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
