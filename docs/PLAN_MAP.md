# EH3D-PHOENIX-PCB: where the work sits in the roadmap

This page maps what the repository has to the phases of `docs/ROADMAP.md` and to the requirements of `docs/REQUIREMENTS.md`, and lists the differences from the roadmap that are still open. The state of each requirement is only the one in `docs/REQUIREMENTS.md`; this page states none of its own. Everything listed is a prototype tried on files made by the tests: nothing has run on a design of a maker or a user.

## Work by phase

| Phase | Work in the repository | Requirements | Where |
|---|---|---|---|
| F0 Foundations | Named constants, stable codes with a registry, the common diagnostic, the register of requirements with a test that checks its tests, decisions and records | PPCB-029 | `docs/DECISIONS.md`, `docs/adr/`, `docs/ERROR_CODES.md`, `docs/REQUIREMENTS.md` |
| F1 Model and persistence | Integer nanometre units, ids, canonical JSON, the project format with domains and atomic saving, the board model with reversible operations | PPCB-001, PPCB-002, PPCB-003 | `eh3d-phoenix-model`, `eh3d-phoenix-project`, `eh3d-phoenix-board` |
| F2 Readers, hub, oracle | EAGLE summary and board reader and writer, the format hub, the shared Gerber reader with an adapter to the importer model, the IPC-2581 reader, the corpus and the oracle frame | PPCB-004, PPCB-005, PPCB-006, PPCB-030 | `eh3d-phoenix-eagle`, `eh3d-phoenix-eagle-board`, `eh3d-phoenix-hub`, `eh3d-phoenix-gerber`, `eh3d-phoenix-ipc2581`, `eh3d-phoenix-corpus`, `eh3d-phoenix-oracle` |
| F3 Connectivity and rules | Connectivity, the rule checks, the advanced checks, the electrical rule checks, the geometric checks of Gerber files with the verdict confirmed, inconclusive or failed | PPCB-007, PPCB-008 | `eh3d-phoenix-board`, `eh3d-phoenix-erc`, `eh3d-phoenix-rules`, `docs/GERBER_DFM.md` |
| F4 Export and hand-off | Gerber X2 and Excellon writers, maker profiles, the pick and place files, the bills, the stencil, the panel, the package, the IPC library naming | PPCB-009, PPCB-010, PPCB-011 | `eh3d-phoenix-export`, `eh3d-phoenix-bom`, `eh3d-phoenix-panel`, `eh3d-phoenix-ipclib` |
| F5, F10 KiCad | Reading and writing of KiCad schematics, boards and libraries | PPCB-012, PPCB-013 | `eh3d-phoenix-kicad` |
| F6 Viewer and editor | The state of the viewer and the editor, routing, zones, lengths, the native router and the exchange with an external one | PPCB-014, PPCB-015, PPCB-016 | `eh3d-phoenix-viewer`, `eh3d-phoenix-boardedit`, `eh3d-phoenix-route`, `eh3d-phoenix-zones`, `eh3d-phoenix-autoroute` |
| F7 Schematic editor | Schematic model, editing with a plan, synchronisation with the board | PPCB-017 | `eh3d-phoenix-schematic`, `eh3d-phoenix-schedit`, `eh3d-phoenix-sync` |
| F8, F8b Script heritage | The command interpreter, the second script language, the import of output jobs | PPCB-018, PPCB-019, PPCB-020 | `eh3d-phoenix-script`, `docs/SCRIPT.md` |
| F9 Library sanitizer | Inventory, matching, the plan with a confirmation code, the library editor and the comparison of two libraries | PPCB-021 | `eh3d-phoenix-sanitizer`, `eh3d-phoenix-libedit`, `eh3d-phoenix-parts` |
| F11 Components and satellites | The server mode and the bill with costs | PPCB-027 | `eh3d-phoenix-server`, `eh3d-phoenix-bom` |
| F12 Simulation | The circuit solver, the stack-up and transmission lines, the library of laminates from the materials database, the library of typical stack-ups, RF and signal integrity, the 2D field solver | PPCB-022, PPCB-024 | `eh3d-phoenix-sim`, `eh3d-phoenix-stackup`, `eh3d-phoenix-rf`, `eh3d-phoenix-rfgen`, `eh3d-phoenix-si`, `eh3d-phoenix-field2d` |
| F13 Power-supply assistant | Nothing yet | PPCB-023 | |
| F14 Bounded assistance | Nothing yet | PPCB-025 | |
| F15 Mechanical steps | The 3D scene, the binary glTF, the OBJ and STEP export of boxes, the fit against an enclosure, the DXF reader and writer | PPCB-026 | `eh3d-phoenix-assembly3d`, `eh3d-phoenix-dxf` |
| Transversal | Eight languages from one source, readers with limits and writers that never replace a file | PPCB-028, PPCB-031 | `eh3d-phoenix-cli`, every crate with codes |

## Differences from the roadmap that are still open

| Topic | What the roadmap asked | What there is | Where it is decided or tracked |
|---|---|---|---|
| Circuit engine | An existing engine bound into the program | An own solver, compared in the CI with an external program | `docs/adr/0010-own-circuit-simulator.md` (the comparison is not made yet) |
| Automatic routing | An external router through the DSN and SES files | A native router, and the external one as an adapter that is off | `docs/AUTOROUTE_NATIVE.md`, `docs/DECISIONS.md` |
| Scripts of the reference tool | An interpreter of its user programs | The command interpreter and the second script language; the report and the reader of the programs on a list are written (`docs/ULP.md`), the evaluator and the sandbox are designed, not written | `docs/adr/0011-ulp-interpreter-for-the-owner-list.md` |
| Interface stack | A measured choice | Accepted: egui with wgpu and the window frame of the suite; the window is laid out with the frame and still draws with the painter of egui on the glow backend, behind one interface for the engine | `docs/adr/0009-first-interface-stack.md` |
| Order of F12 and F13 | The power-supply assistant with its sources before RF and signal integrity | RF, signal integrity and the field solver exist; the assistant does not | PPCB-023 |
| Geometry numbers | Integer nanometres in the design | Integers in the design; the geometry library of the checks of Gerber files works in floating point millimetres and converts at its edge | `docs/adr/0003-integer-nanometre-coordinates.md`, `docs/adr/0007-geometry-library-and-degenerate-cases.md` |
| Geometry benchmark | Degenerate cases and a tolerance policy | The tolerance is a setting; the benchmark is not done | `docs/adr/0007-geometry-library-and-degenerate-cases.md` |
| Common diagnostic | One structure for every report | The model, project and board crates use it; the geometric checks of Gerber files convert to it; other finding types keep their own shape with the same code | `docs/DECISIONS.md` |
| Confirmed or inconclusive | A report that tells a confirmed finding from a check that could not be made | Done for the geometric checks of Gerber files; the rule checks mark arcs as inconclusive in their own way | `docs/GERBER_DFM.md` |
| Canonical JSON | RFC 8785 for new files | Done through the shared container crate | `docs/adr/0005-canonical-json-and-quantised-values.md` |
| Artifact identity | An identity separate from the hash of the bytes | Not written | `docs/DECISIONS.md` |
| Package 1.1.0 and the derived package id | Written when the consumers declare 1.1.0 | Only 1.0.0 is written | PPCB-011 |
| Library domain of the project | Own library format in the project | `.lbr` is read and written; no library domain in `.phoenixproj` yet | `docs/DECISIONS.md` |
| Stack-up library materials | Referenced by id of the materials database | Referenced by id where the database has one; the two figures of the maker's calculator stay by name with a note | `docs/MATERIALS_GAPS.md`, `config/stackup_library.json` |
| Reference tool oracle | Golden files from the reference tool | Reference cases with published values; the interim reference is real files of an open tool and IPC-2581 samples once they are provided | `docs/ORACLE.md` |
| Binary files of the reference tool | A reader | Nothing, until pairs of binary and XML files exist | PPCB-005 |
