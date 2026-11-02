# EH3D-PHOENIX-PCB: status

## Current state

EH3D-PHOENIX-PCB is planned. Version 0.0.1 holds only the base that every repository of the suite carries (manifest, README files in eight languages, changelog, CI, shared checks) and the written scope, requirements, dependencies, threat model and decisions. There is no code, nothing has been run on a machine, and no requirement is implemented.

## Capabilities

The official state of each capability is in REQUIREMENTS.md: 30 requirements, all `planned`. This page never states a state that REQUIREMENTS.md does not.

## Open questions

* Which user-interface and rendering stack to use: candidates are egui on wgpu with a custom canvas, iced, Slint, or everything on wgpu. It is decided after a short prototype measured on small, medium and large boards.
* Which 2D geometry library to use for offsets and boolean operations, decided with a benchmark on degenerate input.
* The exact coordinate range of the design domain (about one metre with 64-bit predicates, or larger with 128-bit predicates).
* Whether the first golden files can be produced semi-automatically from the command line of the reference tool.
* Which circuit-simulation engine to bind, once its licence and platform support are checked.

## Needs from other repositories

* EH3D-COMMON: the project container and the diagnostic crate
* EH3D-CORE-LIBS: geometry, readers and the Gerber and Excellon writers
* EH3D-VALIDATION-LAB: the corpus and the golden-file oracle

## Known technical debt and omitted tests

None yet. The only tests are those of the base (the repository check and the checks of the two README pictures).
