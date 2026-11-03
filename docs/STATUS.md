# EH3D-PHOENIX-PCB: status

## Current state

EH3D-PHOENIX-PCB is early code. Beside the base that every repository of the suite carries, it has a Rust workspace with a library and two commands, `phoenix inventory` and `phoenix eagle-rate` (the second measures how many EAGLE files the reader reads, on copies, in redacted form); the first counts the design files of a private folder in redacted mode (counts by extension, kind and declared version, and one fingerprint). It has run only on folders made by its own tests, never on real design files. It also has the foundations of the design model in the crate `eh3d-phoenix-model`: lengths in integer nanometres and angles in integer micro-degrees with a checked range, orientation predicates in 64-bit integers, arcs as end points plus a sweep, UUID v7 object ids from a generator with an injected clock and random source, and canonical JSON (RFC 8785) with quantised millimetre and degree values. The `.phoenixproj` project file (crate `eh3d-phoenix-project`) and the board model with reversible operations (crate `eh3d-phoenix-board`) also exist, tried on generated data only. The board crate also computes the connectivity of each net and checks sizes and clearances against a rule file, and the crate `eh3d-phoenix-eagle` reads EAGLE XML libraries, boards and schematics into a summary and writes the XML tree back identically; its first measurement over copies of a private corpus reads 734 of 2263 files (the rest are not XML or not EAGLE documents; see PPCB-004); a small crate (`eh3d-phoenix-hub`) is the common interface of the importers, with the EAGLE reader behind it; a crate (`eh3d-phoenix-export`) writes the Gerber X2, Excellon and `.eh3dproj` 1.0.0 files of a board through EH3D-CORE-LIBS and EH3D-COMMON; all of it is tried on generated or hand-made data only. `docs/GATE_F1.md` says what the first gate checks and what it does not. There is no schematic model, no import of foreign designs into the design model, no editor and the rules cover only part of the checks planned.

## Capabilities

The official state of each capability is in REQUIREMENTS.md: 30 requirements: 5 `prototype` (PPCB-001, PPCB-002, PPCB-003, PPCB-029, PPCB-030) and 25 `planned`. This page never states a state that REQUIREMENTS.md does not.

## Open questions

* Which user-interface and rendering stack to use: candidates are egui on wgpu with a custom canvas, iced, Slint, or everything on wgpu. It is decided after a short prototype measured on small, medium and large boards.
* Which 2D geometry library to use for offsets and boolean operations, decided with a benchmark on degenerate input.
* Whether the first golden files can be produced semi-automatically from the command line of the reference tool.
* Which circuit-simulation engine to bind, once its licence and platform support are checked.

## Needs from other repositories

* EH3D-COMMON: the project container (`eh3d-container`) and the diagnostic crate (`eh3d-diagnostic`), already used; this repository builds next to it
* EH3D-CORE-LIBS: geometry, readers and the Gerber and Excellon writers
* EH3D-VALIDATION-LAB: the corpus and the golden-file oracle

## Known technical debt and omitted tests

* 156 tests of the Rust code and 3 of the base; the inventory tests build their own folders and never touch a real project.
* The model foundations are checked with generated and edge cases only: there is no design object on top of them yet, so the identities across save, migration and reopening are untested. Duplicate keys in an input JSON text are not detected before canonical writing. The system random source for ids is not cryptographic (ids need uniqueness, not secrecy).
* Not tested: a corpus of hundreds of thousands of files, folders synchronised by a cloud service, and links (they are skipped and counted; the test for it runs only on Unix-like systems).
* The kind of a file is read from its first bytes only: a file that starts like XML but is damaged later is counted by its start.
* `cargo deny` is not run yet.
