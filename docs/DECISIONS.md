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

## Geometry kernel for mechanical work

State: **proposed**.

No geometry kernel or constraint solver has been chosen. Candidates are compared on the same fixtures, checking robustness with degenerate input, licence compatibility with GPL-3.0-or-later and the ability to export STEP.

## User-interface and rendering stack

State: **proposed**.

The stack is decided after a one-week prototype measured on small, medium and large reference boards on the author's machine; targets are set after measuring, and a frame rate is only an orientation. The criteria are dense tool interfaces, performance on an integrated GPU, Linux on ARM support, accessibility (keyboard navigation, no colour-only signalling) and eight-language text. The rule is already fixed: the interface talks to the engine only through model operations, and no engineering logic lives in the interface.

## Imported scripts are disabled by default

State: **proposed**.

Scripts, output jobs and plugins found in a project are never run automatically. Running one needs explicit confirmation, happens inside a sandbox that starts from nothing allowed (read-only access to the object model and pure computation are allowed; output only inside a private per-run folder with a size limit; operating-system commands, network, environment and user folders are denied; limits on time, memory and number of model operations) and is logged with the hash of the script, the parameters and the resulting changes. The sandbox is also a separate low-privilege process; a child process alone is not a boundary.

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

State: **proposed** for the numbering and the registry; the two prefixes (`EH3D-` for shared code, `PHOENIX-` for this tool) and the shared diagnostic crate are accepted.

Failures that a reader or a check cannot hide become diagnostics with a stable code of the form `PHOENIX-<AREA>-<NNN>` (the registry is kept in the board tool). The model foundations return typed errors and do not carry codes yet. A code is never reused or renumbered, its message may be translated but the code and field names are not, and a repository never keeps a private list: it asks for numbers in the registry in the same change that uses them, together with a test.

## Nothing here moves a machine

State: **accepted**.

Importing, converting, simulating or validating a design never starts motion, even as a side effect. Programs may at most prepare a job in the ready state through the server when a task says so; authorising, starting, confirming and cancelling are actions of a person. Computing tasks run without access to machine connectors.

## Private designs stay private

State: **accepted**.

The real projects of the author are never committed, pasted into issues or sent to a service. Tests use synthetic cases or minimal anonymised reproducers that the owner approved. Scripts that run over a private corpus use redacted mode, which reports only codes, counts, levels and hashes.
