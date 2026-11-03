# EH3D-PHOENIX-PCB: gate of phase F1

The gate of phase F1 (see ROADMAP.md) has three parts. This page says, for each, what is checked, by which test, and what is **not** yet covered. A part is only called passed when the test exists and runs in the CI; nothing here claims correctness on real design files, because none has been used.

| Part of the gate | Checked by | State |
|---|---|---|
| Round trips over thousands of generated models | `tests/board.rs` in `eh3d-phoenix-board`: 400 generated boards are written, read and written again byte for byte, and are consistent; the same seed gives the same bytes in two runs. `tests/project.rs` in `eh3d-phoenix-project`: a project saves and opens with everything it had, saving twice gives the same bytes, and a reopened project saves the same bytes | Covered for the board and for the project container. The generated boards are small (tens of parts); no generated board uses every kind of object at its limits |
| Corrupt files give diagnostics, not panics | `damaged_board_text_is_an_error_and_never_a_panic` cuts and mutates the text of a board; `damaged_files_get_a_code_and_never_panic` damages the container of a project; every refusal carries a code from `docs/ERROR_CODES.md` | Covered for truncation and for the damage the tests make; no fuzzing run with an external fuzzer yet |
| Reversible operations | 120 generated sequences of 60 operations: undoing every change gives back the same bytes, redoing gives the final board, the saved history replays to the same board, a batch is all or nothing, removing a net in use is refused | Covered for the board model only (nets, components, pad nets, tracks, vias); the schematic has no model yet |
| Conformance vectors pass | The canonical JSON vectors of the standard run against the shared writer in the tests of the model; the reference packages of the container run in `EH3D-COMMON` | Covered for canonical JSON and the container; there are no vectors for the board file itself yet because its format is not frozen |
| Diagnostics for every reader | Every code of every registry is in `docs/ERROR_CODES.md` and a test of each crate compares the two lists | Covered. Not every code is produced by a test of its own yet (PPCB-029) |

What the gate does not claim:

* nothing was run on a real design: the generators make the boards;
* the model has no schematic, no zones with holes, no polygons with arcs and no text;
* the project format has no registered migration yet, so the migration machinery is tested with test migrations only;
* the performance of large boards is not measured.

The gate of F2 (EAGLE input and output) is separate: the reader of libraries, boards and schematics exists as a summary and a measured read rate over copies (`eh3d-phoenix-eagle`), and the XML tree can be written and read back identically; the comparison with the reference tool through golden files is the work of EH3D-VALIDATION-LAB and has not been done.
