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
  <img src="https://img.shields.io/badge/Tests-3999-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.5-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust workspace whose command `phoenix inventory` counts the design files of a private folder without showing any name or value: files by extension, kind and declared version, and one fingerprint of the content. It has 3999 tests of its own and they pass on a computer. It has only been run on folders made by its own tests, never on real design files. It also has the foundations of a design model (integer nanometre and micro-degree values with a checked range, object ids and canonical JSON), the `.phoenixproj` project file (manifest, domains, migrations) and a board model with reversible operations (layers, nets, components, tracks, vias, zones), all tried on generated data only. The board library also finds which copper touches which and checks sizes and clearances against a rule file; a fifth library reads EAGLE XML libraries, boards and schematics into a summary and writes the XML tree back identically. All of it was tried on generated or hand-made files only. A schematic model and its consistency check against the board exist as well. There is no import of foreign designs into the model, no editor, and the rules cover only part of the checks planned: most of the EDA function is still planned. The board model also holds silkscreen text and strokes (with a built-in stroke font) that the rule check and the Gerber export use, and the schematic can be drawn on sheets with placed parts and wires that are compared with the net list and with the board; a board can be written as an EAGLE XML file and read back, which has only been checked on files written by this code and never opened in EAGLE; the contract for pairs of EAGLE binary files and their XML is written, but no reader of the binary files exists. A stack-up and transmission-line calculator (`phoenix stackup`: impedance, effective permittivity, losses, delay and track width of microstrip, stripline, coplanar waveguide and coupled pairs from published formulas) is a prototype, never compared with a field solver or with a measurement. With the materials database of EH3D-COMMON (`phoenix materials`, `phoenix stackup --material ID`) the Dk and Df of a layer come from the points a datasheet publishes, never extrapolated; none of these values has been compared with a measurement of a board. A crate for connectivity and rules (`eh3d-phoenix-rules`) reads the limits from a rule file or from the design rules of an EAGLE board, checks a board in a fixed order (width, via, drill, ring, clearance, edge, drill spacing, solder mask, zones, silkscreen), finds shorts and unrouted nets and compares the nets of a board with those of a schematic, and the commands `phoenix check` and `phoenix connectivity` print the report with the confirmed and the inconclusive findings apart; it has only been tried on boards made by hand and has never been compared with the reference tool. Further design rule checks (`phoenix check --advanced`: annular ring, distance to the board edge, acid traps, spacing by voltage, copper islands, stubs, duplicate vias, pads on no net and parts outside the outline, with the limits and levels in `config/drc_advanced.cfg`) are a prototype too, tried only on boards made by hand and never compared with the rules of a manufacturer or with another tool; the figures of the spacing table are not verified against the standard. The manufacturing export (`phoenix export`) makes Gerber X2 files for every layer, plated and not plated Excellon drill files, a placement file and a bill of materials with profiles read from a JSON file, a manifest of SHA-256 sums and a check that reads the files back and compares them with the board; it is a prototype whose files have been read only by the parsers of this repository, never opened in an independent viewer or by a manufacturer. On top of it, `phoenix pnp`, `phoenix stencil` and `phoenix fabpack` write pick and place files, the openings of a solder paste stencil and a deterministic zip package with a manifest; they are prototypes that have never been loaded in a machine, made into a plate or sent to a manufacturer. An RF toolbox (`eh3d-phoenix-rf`, command `phoenix rf`) reads and writes Touchstone files, converts S, Z, Y and ABCD parameters, computes VSWR, losses, group delay and stability, designs L networks, single stubs and quarter-wave sections for complex impedances, gives the coordinates of the Smith chart and designs Butterworth, Chebyshev and Bessel ladder filters of ideal components; it is a prototype checked only with textbook values, never compared with a network analyser, a field solver or another RF program. A KiCad reader (`eh3d-phoenix-kicad`, command `phoenix kicad-read`) reads the symbol libraries, footprints, schematics with their sheets, boards and project files of KiCad 6 to 9 into the same model as the EAGLE files, with an import report and a fidelity level; it is a prototype tried only on files written by hand for its tests, never compared with KiCad or run on a public collection of designs. A library sanitizer (`eh3d-phoenix-sanitizer`, command `phoenix sanitize`) reads a folder of old EAGLE libraries and reports the footprints, symbols and device sets that are duplicated or nearly duplicated, with a canonical geometric hash, a score against a threshold, an HTML and a JSON report and a plan of the master library; it only analyses, never rewrites a library and never applies the plan, and it is a prototype tried only on libraries made by its own tests, never on real libraries. An electrical rule check (`eh3d-phoenix-erc`, command `phoenix erc`) judges a schematic: pins on no net, nets with one pin, pin types that a matrix in a settings file does not allow together, inputs with no driver, supplies with no source, net names that look alike, references used twice, empty values, parts without a footprint, loose wires, nets that nothing drives, power pins of chips without a decoupling capacitor, net names that may be typing mistakes and labels that touch nothing, with the confirmed and the inconclusive findings apart; a synchronisation (`eh3d-phoenix-sync`, command `phoenix sync`) compares a schematic and a board by the identity of their parts and makes the changes of the transfer to the board, or of the annotation back into the schematic, as a package that is tried as a dry run first and can be undone and redone, with a plan of the annotation back that gives each change a state (applicable, conflict or ignored) and a confirmation code (`phoenix annotate`) and a comparison of the two documents (`phoenix sync --diff`); the rule file of the second format also holds net classes with limits of their own for tracks and vias. All of it is a prototype tried only on schematics and boards made by hand and by the tests, never compared with the reference tool, and the schematic editor (the command `phoenix schedit` and an editing mode of the window) is a prototype too, tried without a screen only: operations that can be undone, a net list that follows the drawing, the rule check after each change and the transfer to the board. A circuit simulator (`eh3d-phoenix-sim`, command `phoenix sim`) reads and writes SPICE-style netlists, translates a schematic into one and solves linear circuits and circuits with diodes (with series resistance, junction capacitance and breakdown), bipolar and MOS transistors, voltage-controlled switches and op-amp macromodels by a modified nodal analysis of our own (DC operating point and DC sweep of a source, AC sweeps with group delay, noise and transient analysis with a step that stops at the corners of the sources), with a check against the ladder filters of the RF tools; it also reads the `.op`, `.dc`, `.tran` and `.ac` lines of a netlist and the `.include` of files in the same folder; it is a prototype tested with textbook values and closed-form solutions, never compared with SPICE or with a measurement, and its device models are simple. It also runs a circuit over lists of values, takes measures from the results, makes Monte Carlo studies over tolerances with a fixed seed, and finds noise, poles and zeros from a fit of the AC response; this too is a prototype, checked with closed-form cases and never compared with SPICE. A crate for the editing and routing logic (`eh3d-phoenix-route`, commands `phoenix route` and `phoenix ratsnest`) lists the connections still to be routed and routes between two elements of a net with pieces at 0, 45 and 90 degrees, stopping in front of an obstacle or going around it, with the width of the net class and vias sized by the rules, and its edits are reversible operations that report what they add to the rule check; it is a prototype tried on boards made by hand and by the tests, without arcs or copper zones; the window that edits with it (`phoenix view`) has never been seen on a screen. The routing through an external autorouter goes by files (`phoenix autoroute`): the board is written as a Specctra DSN file, a Specctra session is read as a plan whose routes are checked against the rules and applied all or nothing, and an adapter that starts a program is off by default and starts nothing without a switch in a settings file and an option on the command line; it is a prototype tried with sessions made by hand only, never with a real autorouter. A simple autorouter of our own (`phoenix autoroute native`) routes the open connections with a search on a grid, layer by layer, with vias and with rip-up and reroute within limits that count nodes, and its plan is checked by the rules and never adds a rule error; it is a prototype tried on small boards made by hand and by the tests, and the button for it in the window has never been seen on a screen. A script interpreter (`eh3d-phoenix-script`, commands `phoenix script run`, `phoenix script report` and `phoenix cam-import`) reads script files of the command language of the previous workflow, carries out `WIRE`, `VIA`, `MOVE`, `CHANGE`, `DELETE`, `DRC` and the other commands that have an equivalent as reversible operations of the board, all or nothing and with an undo that gives the original back byte for byte, lists every command of every script as equivalent, adapted or pending with its cause and reads an output job into an export profile; it was built from the public description of the language, is a prototype tried only on scripts written for its tests, and has never been compared with the original tool or run on the author's own scripts. A second script language (`phoenix-script/2`: variables, bounded loops, queries and edits of a board, a schematic or a library as one reversible step, in a sandbox with limits; `phoenix script check`, `list` and `help`) is a prototype tried only on documents made by hand. A generator of RF structures (`eh3d-phoenix-rfgen`, command `phoenix rfgen`) draws microstrip lines, mitered and curved bends, tapers, stubs, matching networks, stepped-impedance low-pass and coupled-line band-pass filters, a Wilkinson divider, a branch-line coupler, patch and dipole antennas and an SMA launch with a via fence from the stack-up and the frequency, using published formulas, and puts them on the board as reversible operations, all or nothing, with a report of the dimensions, the formulas used and the rule check of the result; it is a prototype, tried only with values of textbooks and with boards made by the tests, and has never been compared with an electromagnetic simulator or with a measurement. A KiCad writer (`phoenix kicad-export`) writes a board, a schematic, footprints and symbols as KiCad 8 files, with a report of what the model cannot say; it is a prototype whose files have never been opened in KiCad. Differential pairs (`phoenix pairs`, and the operations `route_pair`, `tune_pair` and `fix_pair_gap` of `phoenix boardedit`) draw the two tracks of a pair side by side, make their lengths equal with meanders, correct their gap and check the gap, the symmetry, the uncoupled stretch, the lengths, the vias, the width and, with a stack-up, the impedance; they are a prototype tried on boards made by hand and by the tests only, and the tool of the window for them has never been seen on a screen. Push and shove (the operation `route_shove` of `phoenix boardedit`) pushes aside, in parallel, the tracks of other nets that a new track would crowd, within a depth and a number of tracks set in a file, all or nothing and refused when it would leave a rule error; arcs in tracks (`route_arc`, `round_corners`) and meanders on pieces at 45 degrees come with it; they are a prototype tried on boards made by hand and by the tests only, and the push tool of the window has never been seen on a screen. Printing (`phoenix print`, `eh3d-phoenix-print`) writes the sheets of a schematic, the assembly plan and the layers of a board and a panel as SVG or as a PDF made by hand; it is a prototype whose PDF has been read only by the checker of its own crate and never opened in a real PDF reader. Signal integrity estimates (`phoenix si`, `eh3d-phoenix-si`) give the crosstalk between parallel tracks, the reflections and terminations of the nets, a simplified eye diagram and the return path of the tracks from closed formulas and the stack-up; they are quasi-static prototypes tried on boards made by the tests only, with nothing compared with a field solver, a simulator or a measurement. A two-dimensional field solver of the cross section of a line (`phoenix field2d`: impedance, permittivity, inductance and the coupling of a pair, beside the closed formulas), the heating of tracks and vias from the current of their net (`phoenix thermal`) and the rounded corners of the zone fill are prototypes, checked against exact solutions and closed forms only, never against another field solver or a measurement.

---

> **Safety and security status:** this repository is at its first steps and is not a safety system. Nothing written here replaces the limits, interlocks and emergency stop of the machine and its controller.

## 📋 At a glance

| | |
|---|---|
| Version | `0.0.5` |
| Maturity | Scaffolding |
| Tests | 3999 |
| Stack | Rust, Python 3.10+ |
| On real hardware | Not run on hardware yet |

**Jump to:** [Overview](#-overview) · [Architecture](#-architecture) · [Quick start](#-quick-start) · [Testing](#-testing) · [Repository Structure](#-repository-structure) · [Related Projects](#-related-projects)

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Repository checks | 🟡 | The repository check, the check of the pictures and the 45 tests of the base run in CI and pass. |
| Tests | 🟡 | The 3999 tests of the Rust code and the 45 of the base pass on a computer. They use only folders made by the tests themselves; none has run on real design files. |
| Real hardware | 🟠 | Not run on any machine or board: nothing here has been tried on hardware. |
| Planned function | 🟠 | Written: the redacted inventory of a corpus, the foundations of the model (units, ids, canonical JSON), the `.phoenixproj` project file, the board model with undo and redo, its connectivity and a first rule check, and a summary reader of EAGLE XML. Not started: the import of formats other than KiCad into the model, the writers of other formats, most of the rules, the editors and the final viewer, the comparison of the simulator with SPICE and everything else planned. The board model also holds silkscreen text and strokes (with a built-in stroke font) that the rule check and the Gerber export use, and the schematic can be drawn on sheets with placed parts and wires that are compared with the net list and with the board; a board can be written as an EAGLE XML file and read back, which has only been checked on files written by this code and never opened in EAGLE; the contract for pairs of EAGLE binary files and their XML is written, but no reader of the binary files exists. A stack-up and transmission-line calculator (`phoenix stackup`: impedance, effective permittivity, losses, delay and track width of microstrip, stripline, coplanar waveguide and coupled pairs from published formulas) is a prototype, never compared with a field solver or with a measurement. With the materials database of EH3D-COMMON (`phoenix materials`, `phoenix stackup --material ID`) the Dk and Df of a layer come from the points a datasheet publishes, never extrapolated; none of these values has been compared with a measurement of a board. An RF toolbox (`phoenix rf`: Touchstone files, two-port parameters, matching, ideal ladder filters and, as estimates, pi and T networks, standard values, masks and stability circles) is a prototype, checked only with textbook values and never compared with a measurement. A KiCad reader (`phoenix kicad-read`: symbols, footprints, schematics with sheets, boards and the project file) is a prototype, tried only on files written by hand. A library sanitizer (`phoenix sanitize`: inventory of a folder of EAGLE libraries, geometric hash, fuzzy matching with a threshold, groups of duplicates, HTML and JSON report and a master-library plan that is never applied) is a prototype, tried only on libraries made by its tests. An electrical rule check (`phoenix erc`) and the synchronisation of a schematic and a board (`phoenix sync`: transfer to the board and annotation back, as a package that can be undone) are prototypes, tried only on schematics and boards made by hand. A circuit simulator of our own (`phoenix sim`: SPICE-style netlists, the netlist of a schematic, and DC, AC and transient analysis of linear circuits and circuits with diodes) is a prototype, checked only with textbook values and never compared with SPICE. The editing and routing logic (`phoenix route`, `phoenix ratsnest`: the ratsnest, a router for pieces at 0, 45 and 90 degrees and reversible edits with the live rule check) is a prototype, tried only on boards made by hand and by the tests. A script interpreter (`phoenix script run`, `phoenix script report`, `phoenix cam-import`: scripts of the command language of the previous workflow run as reversible operations of the board, all or nothing, a compatibility report per command and per script, and output jobs read into an export profile) is a prototype, built from the public description of the language and tried only on scripts written for its tests. A second script language (`phoenix-script/2`: variables, bounded loops, queries and edits of a board, a schematic or a library as one reversible step, in a sandbox with limits; `phoenix script check`, `list` and `help`) is a prototype tried only on documents made by hand. A generator of RF structures (`phoenix rfgen`: microstrip lines, bends, stubs, matching networks, filters, a Wilkinson divider, a branch-line coupler, patch and dipole antennas and an SMA launch with a via fence, drawn from the stack-up and the frequency as reversible operations of the board with a report of the formulas and the rule check) is a prototype, checked only with textbook values and never compared with an electromagnetic simulator or a measurement. A bill of materials with costs (`phoenix bom`: parts grouped by value, footprint and maker, prices by quantity break from a local file, the cost per board and per batch, and an estimate of the cost of the boards from example tariffs) and a 3D assembly of the board with the models of its components (`phoenix assembly3d`: a model per footprint from a table, the box that encloses each STEP solid, collisions and height checked on boxes, and a simplified OBJ and STEP export) are prototypes, tried only on boards and files made by the tests; the tariffs are an example and not a quote, no CAD program has opened the exported files, and the 3D window that draws the scene is a prototype that has never been seen on a screen. A set of reference cases (`phoenix oracle`) checks the calculations against published values with their source and tolerance; it is not a comparison with EAGLE, KiCad, a field solver or a measurement, and none of the states above changes because of it. An own corpus (`phoenix corpus`) of small boards, schematics, libraries and manufacturing files written by hand, with the numbers worked out by hand, files that must be refused, round trips between formats and properties on generated boards, is a prototype; it holds no file of a maker or a user and is not a comparison with EAGLE, KiCad or a CAM program. A two-dimensional field solver of the cross section of a line (`phoenix field2d`: impedance, permittivity, inductance and the coupling of a pair, beside the closed formulas), the heating of tracks and vias from the current of their net (`phoenix thermal`) and the rounded corners of the zone fill are prototypes, checked against exact solutions and closed forms only, never against another field solver or a measurement. |
| Window | 🟠 | The viewer and editor of boards and schematics (`phoenix view`, egui, behind the `gui` feature) is written and its logic is tested: a board is edited with undo and redo and the rule findings of each edit, a schematic is checked, synchronised with a board and edited (place, move, turn, mirror, delete, copy and paste, wires, labels, power ports and bus labels, with the rule check after each change), and nothing is ever saved over the file that was read; but the window has never been seen on a screen: the drawing, the mouse and the speed on a screen are unchecked. The same window has a library mode that edits the symbols, footprints and devices of EAGLE libraries (`phoenix libedit`, `phoenix footprint`, `docs/LIBRARY_EDIT.md`) with undo, a check of the library and footprint generators; it has never been seen on a screen either. A board editing mode (`phoenix boardedit`, `docs/BOARD_EDIT.md`) places components, draws 45-degree tracks and vias, pastes nets and tunes lengths with meanders, each operation checked by the design rules; it has never been seen on a screen or used with a mouse. A window for the Gerber and Excellon files of a folder (`docs/VIEWER.md`: one layer for each file, opacity, a measure and a comparison with the board) and a 3D window of the assembled board (an orbiting camera, a click selects a component, collisions and heights marked) are prototypes; neither has been seen on a screen or used with a mouse. The program `phoenix` (the same one as the command line) opens it with a start screen when it is started with no arguments (open a file, a new library, recent files, drop a file) and, on Windows, without a console window. The common parts of the window (search, measures, layer presets, the list of the history, themes, scale and keys that can be changed, help and remembered preferences) are prototypes as well and have never been seen on a screen. |
| Requirements | 🟠 | 21 of 31 requirements (the redacted mode, the model foundations, the reversible operations, the project file, the diagnostic codes, the analytic RF calculators the connectivity and rule checks, and the manufacturing export with its package, the viewer and editor of boards and schematics, the KiCad readers and writers, the library sanitizer, the schematic checks and synchronisation the circuit simulator, the routing logic and the script interpreter) are at prototype, 10 are only planned and none is validated. |

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

**Download and open the window.** The archive of a release (Windows 64 and 32 bits, Linux and macOS) holds one program, `phoenix`: the command line and, started with no arguments, the window of the viewer and editor (the separate program `phoenix-gui` of the earlier versions no longer exists). Double-click `phoenix` (`phoenix.exe` on Windows): it opens a start screen where you open a board, a schematic or a library, choose one of the recent files or drop a file on the window; `phoenix <file>` opens a file at once. On Windows it has no console window, so when it does not open the reason is written to `last_error.txt` in the settings folder of the user (`%APPDATA%\EH3D-PHOENIX-PCB` on Windows, `~/.config/eh3d-phoenix-pcb` on the other systems), where the list of recent files (`recent.cfg`) is kept too; `config/viewer.cfg` sets how many it keeps. A command (`phoenix check board.brd`) works on the console as always; on Windows a terminal does not wait for the program, so a batch file that needs the exit code starts it with `start /wait`. The window is a prototype that has not been tried with a mouse (see the status above). To build the program with the window: `cargo build --release -p eh3d-phoenix-cli --features gui`. `phoenix --shot FILE.png [--size WxH] [--theme dark|light|contrast] [--window NAME] [--wait N] [--open FILE]` takes a picture of the window and closes by itself (this is how the pictures of the README are made); in that mode no preferences and no list of recent files are read or written.

## 🧪 Testing

The code has 3999 tests (`cargo test`) and the base has 45 (`python -I -m unittest discover -s tools -p "test_*.py"`); all pass on a computer. The CI runs the repository check, the check of the pictures, the tests of the base and, for the Rust code, the format check, clippy with warnings as errors and the tests on Linux and Windows.

## 🏗️ Build & Release

The version lives in the `VERSION` file and in `eh3d.project.json`, and each version has its entry in [CHANGELOG.md](CHANGELOG.md). `cargo build --release` builds the library and the `phoenix` program; releases are published on GitHub. The release archives hold `phoenix`, the command line and the window in one program; `cargo build --release -p eh3d-phoenix-cli --features gui` builds it with the window.

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
│   ├── SECURITY_REVIEW.md                       the review of security and the limits in force
│   ├── DECISIONS.md STATUS.md                   decisions that apply here and the honest status
│   ├── NOT_YET.md RULES.md                      ideas left for later and rules for writing code
│   ├── TESTING.md I18N_AUDIT.md                 how things are tested, and the check of the eight languages
│   ├── ROADMAP.md                               the map of phases, from foundations to the all-in-one suite
│   ├── GLOSSARY.md MIGRATION.md                 the terms and the guide for users of EAGLE and KiCad
│   ├── tutorials/ glossary/ migration/          tutorials run by a test, feature guides and translations
│   ├── ERROR_CODES.md                           the diagnostic codes PHOENIX-<AREA>-<NNN>
│   ├── STACKUP.md                               stack-up and line calculations, formulas
│   ├── RF.md                                    RF tools: Touchstone, two-ports, matching, filters
│   ├── RF_ADVANCED.md                           more RF: pi/T, standard values, masks, stability
│   ├── RFGEN.md                                 RF structures on the board: formulas, sources and limits
│   ├── KICAD.md                                 the KiCad readers: what is read from each format
│   ├── KICAD_IMPORT.md                          reading a whole KiCad project: the command, the report
│   ├── KICAD_EXPORT.md                          writing KiCad files: what is written, the report, the command
│   ├── VIEWER.md                                the viewer and editor prototype and its benchmark
│   ├── SHORTCUTS.md                             keyboard shortcuts of the window and how to change them
│   ├── SANITIZER.md                             the library sanitizer: duplicates, hash, report and plan
│   ├── SCRIPT.md                                the script interpreter: commands, report and output jobs
│   ├── SCRIPT_API.md                            the second script language: reference and limits
│   ├── SCRIPT_EXAMPLES.md                       its example scripts with their output
│   ├── ERC.md                                   electrical rule check and synchronisation
│   ├── ANNOTATE.md                              annotation back into the schematic and comparison
│   ├── SCHEMATIC_EDIT.md                        the schematic editor: operations, nets, window
│   ├── HIERARCHY.md BUSES.md                    sheet symbols, repeated sheets and buses
│   ├── LIBRARY_EDIT.md                          the library editor: edits, checks, generators
│   ├── IPC_LIBRARY.md                           the own library of footprints: families, densities, check
│   ├── LIBINDEX.md                              the bulk import of libraries: quality report, index
│   ├── BOARD_EDIT.md                            the board editor: operations, rule check, window
│   ├── SHOVE.md                                 push and shove, and arcs in tracks: operations, settings, window
│   ├── BOARD_ZONES.md                           copper zones in the board editor: fill, priority, paste
│   ├── SIM.md                                   circuit simulation: netlist, analyses, schematic export
│   ├── SIM_NONLINEAR.md                         simulator: non-linear devices, DC sweep, convergence
│   ├── SIM_ADVANCED.md                          simulator: steps, measures, Monte Carlo, poles
│   ├── NETLIST.md                               the netlist of a schematic: table of models, report
│   ├── ROUTE.md                                 editing and routing: ratsnest, router, reversible edits
│   ├── DRC_ADVANCED.md                          advanced design rule checks and their settings
│   ├── DFM.md                                   checking a board against the capability profile of a maker
│   ├── AUTOROUTE.md                             routing by an external autorouter: DSN, session, adapter
│   ├── ZONES.md                                 filling copper zones: clipping, reliefs, islands, check
│   ├── FIELD2D.md                               two-dimensional field solver of a line: impedance, modes, coupling
│   ├── THERMAL.md                               heating of tracks and vias from the current of a net
│   ├── LENGTHS.md                               lengths, matching groups and differential pairs
│   ├── DIFFPAIRS.md                             differential pairs: definition, operations, checks, the command
│   ├── SI.md                                    signal integrity: crosstalk, terminations, eye, return path
│   ├── EAGLE_DEEP.md                            EAGLE in depth: rules, CAM jobs, variants, layers, loss report
│   ├── GERBER_IMPORT.md                         import of Gerber and Excellon files for review, and the round trip
│   ├── GERBER_X2.md                             Gerber X2 attributes in the export, and their check
│   ├── PANEL.md                                 panelization: boards, frame, tabs, V-score, fiducials, holes, layers
│   ├── PRINT.md                                 printing: SVG and PDF pages with a title block
│   ├── BOM.md                                   bill of materials with costs, CSV, JSON, HTML
│   ├── PARTS.md                                 local parts base: search, check, bill of materials link
│   ├── PNP.md STENCIL.md FABPACK.md             pick and place, stencil and the fabrication package
│   ├── ASSEMBLY3D.md                            3D assembly: STEP models, collisions, OBJ, STEP
│   ├── DXF.md                                   DXF: board outline, board and panel export
│   ├── ENCLOSURE.md                             board against its enclosure, STEP solid
│   ├── SPATIAL.md                               spatial index for the checks, perf report
│   ├── ORACLE.md                                reference cases and the plan to compare with EAGLE and KiCad
│   ├── CORPUS.md                                the own corpus: hand-written pieces, round trips, properties
│   ├── GATE_F1.md                               what the gate of the first phase checks and what it does not
│   └── adr/                                     the decision records
├── AGENTS.md                                    rules for anyone who changes the repository
├── deny.toml                                    licence and advisory policy of the dependencies
├── crates/                                      inventory, model, project, board, schematic, EAGLE, KiCad, rules, ERC, sync, hub, export, Gerber import, panel, stack-up, RF, simulator, router, zones, lengths, bill of materials, 3D assembly, sanitizer, scripts, viewer, CLI
├── config/                                      the settings file (every tuned value)
├── examples/                                    example scripts, their output and the files of tutorials and guides
├── Cargo.toml Cargo.lock                        the Rust workspace and its locked dependencies
├── tools/                                       the repository check, its tests and the picture tools
├── tests/                                       reference cases and the own corpus of hand-written pieces
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
* The final stack of the user interface is still open (a provisional choice is recorded in `docs/adr/`), and so is the geometry library (see `docs/STATUS.md`).

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
* [Tutorials](docs/tutorials/README.md): nine flows with commands that a test runs, in eight languages; `phoenix help` explains every topic and command in the program
* [Feature guides](docs/tutorials/guides/README.md): ten guides on the newer functions (check reports, bill of materials, panel templates, field solver, pours, cutouts, buses, autorouter, selection, pictures of the window), in eight languages
* [Testing](docs/TESTING.md): what is tested and how, where the example files live and how to add a case; [the eight languages](docs/I18N_AUDIT.md): what is checked in every text and every page
* [Glossary](docs/GLOSSARY.md) and [migration guide for users of EAGLE and KiCad](docs/MIGRATION.md)

## 🤝 Contributing & Community

* Before a pull request run `python -I tools/ci_validate.py` and the tests of the base: all must pass. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).
* The README exists in eight languages: a change to the text is made in all eight files, and the two pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`.
* Report a vulnerability privately: [SECURITY.md](SECURITY.md). Questions, ideas and reports: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)). The review of security that the project made of itself (no external audit): [docs/SECURITY_REVIEW.md](docs/SECURITY_REVIEW.md).

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.
