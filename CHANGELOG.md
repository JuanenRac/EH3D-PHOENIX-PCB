# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- Use the object identity (`ObjectId`, the id generator and its clock and random source) of EH3D-COMMON instead of an own copy: the model re-exports it, and its two codes are now `EH3D-ID-001` and `EH3D-ID-002` of the shared container.
- Depend on EH3D-COMMON for the project container and the common diagnostic: the canonical JSON of the model is now the shared writer, the CI checks out the tag of EH3D-COMMON that the manifest names, and a duplicated copy of the writer is gone.
- Give every failure of the model foundations a stable code (`PHOENIX-UNITS-*`, `PHOENIX-JSON-*`) and a common diagnostic, with `docs/ERROR_CODES.md` kept in step with the registry by a test.
- Add the crate `eh3d-phoenix-project`: the `.phoenixproj` manifest as canonical JSON (domains, strong and weak links, migrations, origin), saving and opening on the shared container, migrations registered by version, domains that are not understood kept untouched, an unknown critical domain refused or opened read-only by a setting, `must_understand` objects refused when unknown, and the codes `PHOENIX-MODEL-*` and `PHOENIX-PROJECT-*`.
- Add the crate `eh3d-phoenix-board`: layers, nets, footprints, components, pads, tracks with lines and arcs, vias and zones linked by stable ids in integer nanometres, a consistency check with the codes `PHOENIX-BOARD-001` to `PHOENIX-BOARD-009`, reversible operations with an exact inverse, all-or-nothing batches and a serialisable undo/redo history.
- Test the board model on hundreds of generated boards (round trip byte for byte, consistency), on generated sequences of operations (undo back to the same bytes, redo to the same board, replay of the saved history) and on damaged text (an error, never a panic).
- Add the ordered stack of copper layers to the board (`Board::copper_stack`, from the top to the bottom): the function of a layer (top, inner, bottom) follows from its place, an empty stack is not written and a board without it reads as before, the checks have the codes `PHOENIX-BOARD-012` and `PHOENIX-BOARD-013`, `SetCopperStack` is an operation with an exact inverse, and the Gerber files of the export carry the right `.FileFunction` (`Copper,L1,Top`, `Copper,L2,Inr`, `Copper,L3,Bot`).
- Add the connectivity of the board (`Board::connectivity`): the islands of each net from pads, tracks, vias and zones that touch on a layer, with exact integer arithmetic, information about the connections still to route (`PHOENIX-BOARD-010`) and about arcs judged by their end points (`PHOENIX-BOARD-011`).
- Add the rule check of the board (`Board::check_rules`) with a rule set read from a text file in millimetres: minimum track width, drill, via diameter, annular ring and clearance of straight track pieces and vias, in a fixed order, with confirmed and inconclusive findings kept apart and the codes `PHOENIX-RULE-001` to `PHOENIX-RULE-006`.
- Write the XML tree of an EAGLE file back (`write_document`): the same tree always gives the same text and reading it gives the tree back, which the corpus measure checks on every copy.
- Add `docs/GATE_F1.md`: what the gate of the first phase checks, by which test, and what it does not claim.
- Add the crate `eh3d-phoenix-export`: the manufacturing export of a board, one deterministic Gerber X2 file per layer and one Excellon drill file through the writers of EH3D-CORE-LIBS, assembled into an `.eh3dproj` 1.0.0 package through EH3D-COMMON with the identity and time stamp given by the caller, and the codes `PHOENIX-EXPORT-001` to `PHOENIX-EXPORT-007` for what the files cannot say; EH3D-CORE-LIBS is now a dependency of the manifest.
- Add `phoenix eagle-rate`: copies the EAGLE files of a folder (the originals are only read; each copy is checked by SHA-256 and the folder for the copies must be outside the originals), reads the copies and prints counts, versions and diagnostic codes only.
- Add the crate `eh3d-phoenix-hub`, the common interface of the format importers: detection by content and by extension, one outcome with a quality report of what was kept, transformed and dropped, and the codes `PHOENIX-HUB-001` to `PHOENIX-HUB-003`; the EAGLE reader is registered through it.
- Add the crate `eh3d-phoenix-eagle`, a read-only reader of EAGLE XML files (versions 6 to 9): a bounded XML parser (limits on size, depth and elements, no recursion), a summary of libraries (packages, symbols, device sets, with checks that devices, gates and connections point at things that exist), boards and schematics (parts, nets, routed items, sheets, embedded libraries), the codes `PHOENIX-EAGLE-001` to `PHOENIX-EAGLE-011`, and a corpus helper that copies files with a SHA-256 check, proves the originals did not change and measures the read rate with a report of counts and codes only.
- Add `AGENTS.md` with the rules for anyone who changes the repository, `docs/adr/` with the four accepted decision records written as ordinary documents, and `deny.toml` with the licence and advisory policy of the dependencies (the CI step that runs it comes once it has been tried locally).
- Add the Rust workspace with a library and the `phoenix` command line.
- Add `phoenix inventory`: counts the design files of a private folder by extension, kind and declared version, with one fingerprint of the content.
- Keep the inventory in redacted mode: no name, path or value from a file reaches the report, with a leak test.
- Bound the inventory (number of files, folder depth, file size) with a settings file, and stop with an error instead of giving a partial count.
- Run format, lints and tests of the Rust code on Linux and Windows in the CI.
- Add the crate `eh3d-phoenix-model` with lengths in integer nanometres inside a checked range of plus or minus 2^30 nm and angles in integer micro-degrees, with typed errors when a value leaves the range.
- Add orientation and segment-intersection predicates that use only 64-bit integers, tested at the corners of the range, and arcs stored as end points plus a signed sweep.
- Add exact decimal reading of millimetres and degrees, with half-to-even rounding that tells the caller when digits were dropped.
- Add UUID v7 object ids in canonical lowercase text with a generator whose clock and random source are injected, so tests are reproducible and ids grow even when the clock stands still or goes back.
- Add canonical JSON (RFC 8785) with the vectors of the standard, a final line feed, and quantisation of millimetre and degree values to six decimals before writing.
- **The workflows are hardened:** every action is pinned by its commit with its version in a comment, and the checkouts keep no credentials.

## [0.0.1] - The base of the repository

- **The base that every repository of the suite has:** the manifest, the README files in eight languages, this changelog, the CI workflow, the issue and pull-request templates, and the shared checks of EH3D-DOCS (the repository check and the drawers of the two README pictures).
- **A planned project, described honestly:** the scope, the requirements (all planned), the relations with other repositories, the threat model, the decisions that apply and the rules for writing code are in `docs/`; there is no code yet.
