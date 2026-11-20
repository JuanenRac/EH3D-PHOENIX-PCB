# EH3D-PHOENIX-PCB: decisions that apply here

Decisions of the suite that shape this project, written as project documentation. A decision marked proposed is not final: this project must not rely on it silently, and where a number or a limit depends on it, the code will keep it as one named constant.

## Independent repositories related by contracts

State: **accepted**.

Every repository of the suite is independent. Repositories relate through versioned contracts and build next to each other as sibling folders. Programs never depend on each other: they exchange work through project files or through the server. The design tool does not depend at build time on the satellites that depend on it; it reaches them through their command line or through server tasks. A lower layer never depends on a higher one.

## The PHOENIX project keeps its own format

State: **accepted**.

The design tool stores everything in its own project format, with one folder per domain (design, mechanical, simulation, thermal, RF, history, passport, models, exports). The `.eh3dproj` package stays the exchange format for fabrication and is exported from it; the two are not nested. Domains this project does not know are preserved, and domains that change electrical or geometric meaning must be understood or the project is opened read-only.

## Integer coordinates in the design domain

State: **accepted**.

Design coordinates are integers in nanometres and angles are integers in micro-degrees; arcs are stored as end points plus a signed sweep and the centre is derived. A coordinate is strictly inside plus or minus 2^30 nm (about 1.07 m), so every difference is below 2^31 and the orientation predicates are exact in 64-bit integers; constructions use 128-bit integers and check overflow. A value outside the range is a typed error, never a wrap. The limits are named constants in `eh3d-phoenix-model`. Mechanical geometry follows its own rule so that a board limit never caps a chassis.

## Stable object identities

State: **accepted**.

Every design object has an immutable UUID version 7 in canonical lowercase text. Names are attributes, so renaming never changes identity. Ids are generated when an object is created, never inside a writer: the generator takes an injected clock and random source, so tests are reproducible, and writers only receive ids.

## Canonical JSON and numbers in new formats

State: **accepted**.

New JSON files use the JSON Canonicalization Scheme (RFC 8785) plus UTF-8 without BOM and a final line feed. Design values are integers with the unit in the name; exchange values in millimetres are quantised to six decimals before writing; physical results with uncertainty are compared numerically, not byte for byte. The existing `.eh3dproj` 1.0.0 behaviour does not change.

## One shared project container

State: **accepted**.

The ZIP container with an integrity record, bounded reading and deterministic, atomic writing is shared with the `.eh3dproj` package and lives in EH3D-COMMON (`eh3d-container`); the `.eh3dproj` 1.0.0 behaviour does not change. This repository depends on it: it builds next to EH3D-COMMON, the CI checks out the tag named in `SIBLING_REFS`, and the canonical JSON of the model is the shared one. New formats use the strict name and ZIP policy and the numbered container diagnostics `EH3D-PKG-001` to `EH3D-PKG-014`. An export writes `.eh3dproj` 1.0.0 by default and 1.1.0 only when asked for; a package identity that is derived (not the project id, which survives copies) is a version 5 UUID, made only when it is needed. This project writes no container yet.

## Diagnostic codes `PHOENIX-<AREA>-<NNN>`

State: **accepted**.

Every failure that a reader or a check reports has a stable code with the area in its name, from the start: no code is reused and none is created without its test and its row in `docs/ERROR_CODES.md`. The common diagnostic type is the one of the shared crate `eh3d-diagnostic`. The model foundations already map their errors (areas `UNITS`, `ID` and `JSON`).

## Geometry kernel for mechanical work

State: **proposed**.

No geometry kernel or constraint solver has been chosen. Candidates are compared on the same fixtures, checking robustness with degenerate input, licence compatibility with GPL-3.0-or-later and the ability to export STEP.

## User-interface and rendering stack

State: **accepted**. The stack is egui with the wgpu backend, inside the window frame of the suite (`eh3d-dock` in EH3D-COMMON: docked, tabbed, split and floating panels, named workspaces, menus declared as data, eight languages). The record is `docs/adr/0009-first-interface-stack.md`.

The rule fixed from the start still holds: the interface talks to the engine only through model operations, and no engineering logic lives in the interface. The measurements on a real screen (small, medium and large boards, integrated graphics, 1080p and 4K) stay as the acceptance checks of the window, not as a condition of the choice.

Evidence in the code: the viewer and the editor keep their state outside any window (`eh3d-phoenix-viewer`, `eh3d-phoenix-boardedit`) and only the `gui` feature draws it. The window is laid out with the frame `eh3d-dock` (panels that dock, float and fold, workspaces by task, the menu bar and the arrangement kept in the file of preferences; `docs/VIEWER.md`), and the canvas is drawn behind one interface (`CanvasEngine`, the setting `canvas_engine`). Accepted, not yet done: the window still draws with the painter of egui on the glow backend; the callback of wgpu and the measurements that accept it (below and in the record) are the open item of phase F6.

## Imported scripts are disabled by default

State: **accepted**, with the sandbox as an open item.

Scripts, output jobs and plugins found in a project are never run automatically. Running one needs explicit confirmation, happens inside a sandbox that starts from nothing allowed (read-only access to the object model and pure computation are allowed; output only inside a private per-run folder with a size limit; operating-system commands, network, environment and user folders are denied; limits on time, memory and number of model operations) and is logged with the hash of the script, the parameters and the resulting changes. The sandbox is also a separate low-privilege process; a child process alone is not a boundary.

Evidence in the code: the script interpreter (`eh3d-phoenix-script`) never starts a program and never reaches the network, reaches a file only through three commands that are off in its settings, and refuses a run that needs more operations than the limit (`config/script.cfg`). Accepted, not yet done: the separate low-privilege process, the confirmation step with the hash of the script and the log of changes. The interpreter of the original script language is decided in `docs/adr/0011-ulp-interpreter-for-the-owner-list.md`.

## Every result states its kind

State: **accepted**.

Each numeric or check result says whether it is an analytic estimate, an own numerical approximation, a simulation by an identified engine (name and version), a rule check (rule and version cited), a recommendation by artificial intelligence (always reviewable, never an authority), a measurement from real hardware (instrument and conditions) or an experimentally validated result. They are not equivalent: an estimated temperature is not a validated thermal simulation nor a thermocouple measurement.

## Third-party tools as oracles through golden files

State: **accepted**.

Results of a reference tool (opening without warnings, rule checks, manufacturing outputs) are produced by hand through its normal interface while it still works, stored with checksums and redundant verified copies on more than one medium, and compared by local scripts. The files stay private. How a run is made is recorded with each set: tool version and build, rule and output-job files, settings and operating system.

## Clean-room policy

State: **accepted**.

Formats are learned from public specifications, from files of the author's own and from golden-file comparisons. Executables or libraries of third-party software are never decompiled, disassembled or inspected to derive format details. Help texts, libraries, scripts and output jobs of other vendors are never copied into a repository or redistributed.

## Stable diagnostic codes

State: **accepted**.

Failures that a reader or a check cannot hide become diagnostics with a stable code of the form `PHOENIX-<AREA>-<NNN>` (the registry is `docs/ERROR_CODES.md`, repeated by the `REGISTRY` of each crate and kept in agreement by a test). A code is never reused or renumbered, its message may be translated but the code and field names are not, and a repository never keeps a private list: it asks for numbers in the registry in the same change that uses them, together with a test. The two prefixes are `EH3D-` for shared code and `PHOENIX-` for this tool. See also the section on diagnostic codes above.

## Nothing here moves a machine

State: **accepted**.

Importing, converting, simulating or validating a design never starts motion, even as a side effect. Programs may at most prepare a job in the ready state through the server when a task says so; authorising, starting, confirming and cancelling are actions of a person. Computing tasks run without access to machine connectors.

## Private designs stay private

State: **accepted**.

The real projects of the author are never committed, pasted into issues or sent to a service. Tests use synthetic cases or minimal anonymised reproducers that the owner approved. Scripts that run over a private corpus use redacted mode, which reports only codes, counts, levels and hashes.

## Circuit simulation starts with a solver of our own

State: **accepted**. The simulator is our own (modified nodal analysis, with diodes, transistors, op-amp macromodels and the analyses of the record), and an existing engine is used only to compare: in the CI, as an external program that is started and read back, never linked. The record is `docs/adr/0010-own-circuit-simulator.md`.

Its results say that they are an own numerical approximation and a prototype until the reference cases have run through both and the differences are written down; no such comparison has been made yet.

## Library format of the project, with `.lbr` for import and export

State: **accepted**.

Libraries live in the own project format and the `.lbr` of the reference tool is an import and an export, not the storage.

Evidence in the code: `eh3d-phoenix-libedit` edits a library in memory and writes a new `.lbr` (`phoenix libedit --out`, `docs/LIBRARY_EDIT.md`), the sanitizer plans and applies changes with a backup, and `phoenix libedit --compare` reads two `.lbr` files. Accepted, not yet done: the own library domain inside `.phoenixproj` (the project format knows domains and links, but no library domain is registered yet) and its schema with a device and its versions.

## Object identity is separate from the identity of the bytes

State: **accepted**, not yet implemented.

Besides the identity of a design object (a UUID version 7), a produced file or package has an artifact identity that is separate from the SHA-256 of its bytes: the same design built twice has the same artifact identity and may have different bytes when a writer profile says so, and two different designs never share one.

Evidence in the code: none yet. The packages carry a manifest of SHA-256 sums (`phoenix fabpack`, `docs/FABPACK.md`), which is the identity of the bytes; the artifact identity waits for the contract of the server and the passport of a design.

## Determinism in two guarantees, with named writer profiles

State: **accepted**, partly implemented.

A writer gives the same bytes for the same input and the same profile (first guarantee), and the content of a result does not depend on the order of its inputs or on the machine (second guarantee). The profile that decides the form of a file has a name, and the files carry no date, user name or path.

Evidence in the code: the tests that write twice and compare bytes (`saving_twice_gives_the_same_bytes_and_a_reopened_project_saves_the_same_bytes`, `the_same_files_give_the_same_bytes_in_any_order`), the deterministic zip of `phoenix fabpack`, canonical JSON for every new file and the export profiles of `config/export_profiles.json` and `config/export_profiles_makers.json`. The name of the profile is written into the files as one comment with one rule (`eh3d_phoenix_model::writer_profile`, `docs/EXPORT.md`): the Gerber and Excellon files of the export and of the stencil, the IPC-2581 file and the JSON file of pick and place carry it, and the tests build every file twice and compare the bytes. Accepted, not yet done: the CSV files (placement, bill of materials, the report of the stencil), the IPC-D-356A file and the project file do not carry it, because a loader reads them line by line or the format has no place for it; the manifest of a package names the profile for them.

## Versions 0.x, critical domains and `must_understand`

State: **accepted**.

While the major version is 0 a change of a format may break readers, and says so by its version and a migration. A domain of the project is critical when it changes electrical or geometric meaning: a reader that does not understand a critical domain, or an object marked `must_understand`, refuses the project or opens it read-only, and never drops it.

Evidence in the code: `eh3d-phoenix-project` (critical domains, links, migrations, opaque preservation of the domains it does not know, read-only opening), with the codes `PHOENIX-MODEL-004`, `PHOENIX-MODEL-006` and `PHOENIX-PROJECT-001` to `PHOENIX-PROJECT-006`. No migration is registered yet.

## Exact SPDX identifiers

State: **accepted**.

The software carries the identifier `GPL-3.0-or-later` in the manifest of the workspace and an SPDX line at the head of its source files; hardware uses `CERN-OHL-S-2.0`; a repository that mixes firmware and hardware states the licence of each file.

Evidence in the code: `license = "GPL-3.0-or-later"` in the workspace and a SPDX line in the source files. This repository has no hardware.

## The common diagnostic crate lives in EH3D-COMMON

State: **accepted**.

`eh3d-diagnostic` is in EH3D-COMMON and not in the general library, because the container of EH3D-COMMON emits diagnostics and EH3D-COMMON cannot depend on the general library.

Evidence in the code: the model, project, board and the other crates depend on `eh3d-diagnostic` (workspace dependency) and map their errors to it; the geometric checks of Gerber files map their findings to it too (`finding_diagnostic`, with the `inconclusive` recoverability for a check that could not be made). Several finding types of other crates still have their own shape, with the same code; converting them is a small item per crate.

## Two registries of codes

State: **accepted**.

`EH3D-` (EH3D-COMMON, the general library, the server and the machine tools; registry in the documentation hub) and `PHOENIX-` (this family; registry in `docs/ERROR_CODES.md`).

Evidence in the code: every crate with codes has a `REGISTRY` table and the test that the codes follow the pattern, are unique, and are in `docs/ERROR_CODES.md`.

## Height maps in the exchange package

State: **accepted**, not applicable here yet.

The exchange package 1.1.0 has a root `heightmaps/` for the height maps of the levelling tool. This repository writes and reads none; the container and its policy are in EH3D-COMMON.

Evidence in the code: none in this repository, by design.

## The server owns projects and artifacts

State: **accepted**, implemented in the server and not here.

The server decides the owner and the sharing of projects and artifacts, answers "not found" for what a caller may not see, and filters its events. This repository only sends work and reads the report (`phoenix server`); it keeps no copy of these rules.

Evidence in the code: `eh3d-phoenix-server` sends a job and never assumes more than the routes it uses (`docs/DEPENDENCIES.md`).

## Canonical JSON with quantised values

State: **accepted**. The record is `docs/adr/0005-canonical-json-and-quantised-values.md`.

New JSON files use the canonical form of RFC 8785; exchange values in millimetres are quantised to six decimals before writing; integers with the unit in the name are used in the design domain.

Evidence in the code: `eh3d-phoenix-model::canonical` and `quantise` over the writer of `eh3d-container`, with the vectors of the standard as tests.

## The `.eh3dproj` package is exported at 1.0.0 by default

State: **accepted**.

An export writes the package at version 1.0.0 unless the consumer declares that it reads 1.1.0, and the identity of the exported package is derived deterministically from its content.

Evidence in the code: `phoenix export --eh3dproj` writes the 1.0.0 package (`PPCB-011`). Version 1.1.0 and the derived package identity are not written yet.

## Circuit simulation is compared with an external engine in the CI

State: **accepted**, comparison pending.

The own solver stays the engine of the program. An existing circuit engine, started as an external program and read back (never linked into the program), is used in the CI to compare the reference cases. The difference of each case is written down with its tolerance.

Evidence in the code: none yet; the reference cases are in `eh3d-phoenix-sim` and in the oracle frame. The comparison needs the external engine in the CI image.

## Automatic routing is our own, with an adapter to an external router that is off

State: **accepted**.

The router of the program is our own (`eh3d-phoenix-autoroute`, `docs/AUTOROUTE_NATIVE.md`). The exchange with an external router through the DSN and SES files stays as an adapter that is off by default and runs only on request.

Evidence in the code: the native router and the DSN and SES export and import (`docs/AUTOROUTE.md`), with the adapter off in `config/autorouter.cfg`.

## An interpreter of the original script language, only for the list of the owner

State: **accepted**, not implemented. The record is `docs/adr/0011-ulp-interpreter-for-the-owner-list.md`.

An interpreter of the user programs of the reference tool is written only for the programs the owner lists, with a compatibility report per program, and never as a promise of the whole language.

Evidence in the code: none; the second script language (`phoenix-script/2`) and the command interpreter exist, and `phoenix script` reports what of a script file it could translate.
