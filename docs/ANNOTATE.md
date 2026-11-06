# EH3D-PHOENIX-PCB: annotation back into the schematic and comparison of the two documents

**State: prototype.** Two parts make it: the plan of the annotation back (`eh3d-phoenix-sync`, module `annotate`, command `phoenix annotate`) and the comparison of a schematic and a board in both directions at once (module `diff`, command `phoenix sync --diff`). Both work by the identity of the parts (`Component::symbol` holds the id of the part), never by a name, so a renamed reference designator is a change and not a new part. They have been tried on schematics and boards made by hand and by the tests; they have never been compared with the reference tool and nothing here has been seen on a screen.

The forward annotation (schematic to board) and the plain backward annotation (`phoenix sync`) are described in `docs/ERC.md`. This page is about what is new: a backward annotation that is a **plan** read before anything is written, with a state for each change, and a comparison that names the conflicts.

## The plan, in two steps

`phoenix annotate [--config FILE] [--json] [--lang CODE] [--renumber] <schematic> <board>` is a dry run: it prints the plan and its **confirmation code** and writes nothing. Each item is one change that the board would make to the schematic, with its place, what the schematic holds now, what it would hold, and a state:

| State | Meaning |
|---|---|
| applicable | the schematic can take the change and it clashes with nothing |
| conflict | it clashes with another item or the schematic cannot take it; the reason says why; it is never applied (code PHOENIX-ANNOT-008) |
| ignored | it is listed and not applied, because the settings leave it out (PHOENIX-ANNOT-009) or because it needs a drawing |

The second step is `phoenix annotate --apply --confirm CODE --out FILE [--board-out FILE] <schematic> <board>`. It plans again, refuses with PHOENIX-ANNOT-011 unless CODE is the confirmation code of the plan it makes (so the plan that was read is the plan that is applied; the code is a fingerprint of every item and its state, not a signature), applies the applicable items to a **copy** as one step of the history (so it can be undone and redone) and writes the copy to a **new** file. The command never writes an input file and never writes a file that exists already, and with `--board-out` it writes the board with the components renamed by the renumbering to a second new file. The result is the JSON of the model, or EAGLE XML when the name ends in `.sch` (the schematic) or `.brd` (the board). The same schematic, board and settings always give the same plan, the same text, the same JSON and the same code. Exit code: 0 nothing to apply and nothing that clashes, or the copy was written; 1 a dry run found changes or conflicts; 2 the arguments or the input are unusable.

| Item | Code | What it is |
|---|---|---|
| reference from the board | PHOENIX-ANNOT-001 | the reference designator of a component is not the one of its part; the plan writes the one of the board into the schematic |
| value from the board | PHOENIX-ANNOT-002 | the same for the value |
| footprint from the board | PHOENIX-ANNOT-003 | the same for the footprint (also a part with none chosen) |
| component with no part | PHOENIX-ANNOT-004 | listed and ignored: a part needs a symbol, which only the drawing can give |
| part with no component | PHOENIX-ANNOT-005 | listed and ignored: the schematic keeps the part, and deleting it is for a person |
| equivalent pins or gates swapped | PHOENIX-ANNOT-006 | see below |
| renumbered by position | PHOENIX-ANNOT-007 | see below |

A **conflict** is a reference that would be empty or used twice (two components that carry one reference, for example), a second component made from one part (it is left alone), a footprint the board does not have, and a swap that the net list alone cannot take. A cycle is not a conflict: when R1 and R2 trade names on the board, both changes are applicable, because the names are unique once both are made.

## Renumbering by position

With `--renumber` (or `renumber = true` in `config/annotate.cfg`) the plan also numbers the components of the board by the place they lie in, so that R1, R2, R3 follow the layout, and proposes the new references for the schematic and the board. The rules are in the file, not in the code:

* `order`: `top_to_bottom` numbers row by row (from the top down, inside a row from left to right); `left_to_right` numbers column by column. A row (or column) is the set of components whose coordinate across it lies within `row_tolerance_nm` of the first of the set, so a row that leans by a millimetre is still one row. `y_axis_up` says which way the Y axis of the board grows.
* `start` and `step`, and `start.<PREFIX>` and `step.<PREFIX>` for one prefix (the letters of the reference): `start.R = 100`, `step.R = 5` gives R100, R105, R110.
* `prefixes` names the prefixes that are renumbered (empty means all); `locked` lists references that keep their name; a number held by a locked reference, by a component that has no part or by a part that has no component is never given to another component.
* `by_side` numbers the two sides apart, the bottom starting at `start + bottom_offset`.

The new references are written into the parts of the schematic (an edit that can be undone) and, with `--board-out`, into the components of the board (`SetComponentFields`, also reversible); the plan lists them as PHOENIX-ANNOT-007. Without `--board-out` the command says that the board is not renamed, because a schematic that is numbered differently from its board would be reported again by the next run.

## Equivalent pins and gates (swap)

Some pins and some gates can be exchanged on the board for a shorter route (the two pins of a connector, the gates of a quad NAND). The file lists them per symbol name: `swap.CONN2 = 1 | 2`, `swap.74HC00 = 1,2,3 | 4,5,6 | 9,10,8`. A group is slots separated by `|`; a slot is the pin numbers of one gate separated by commas; the slots of a group have the same width. When the nets that the board connects to the slots are the nets of the schematic **in another order**, the plan proposes the swap: the pins are put on the nets of the board in the net list of the schematic (one edit, reversible). When the nets are not a permutation of each other it is not a swap, and the comparison reports the difference.

The swap changes the net list and not the drawing, so it is a conflict ("swap the wires by hand") when the schematic has wires, and also when a net name of the board is not exactly one net of the schematic. Gates of different symbols and swaps that move a pin to a net that does not exist are not planned.

## Comparing the two documents

`phoenix sync --diff [--json] [--lang CODE] <schematic> <board>` lists, in a fixed order, what differs and what clashes, in either direction: the report does not say which document is right.

| Entry | Code | |
|---|---|---|
| part with no component / component with no part | PHOENIX-ANNOT-013 / 014 | by identity |
| reference, value, footprint differ | PHOENIX-ANNOT-017 / 015 / 016 | a part and its component |
| net that only one document has | PHOENIX-ANNOT-018 | by name |
| pin on a net in one document only | PHOENIX-ANNOT-019 | the pins of paired parts |
| net with other pins in each document | PHOENIX-ANNOT-020 | one entry for each net, listing the pins only here and only there |
| reference used twice | PHOENIX-ANNOT-021 | conflict |
| part made into two components | PHOENIX-ANNOT-022 | conflict |
| reference taken by another item | PHOENIX-ANNOT-023 | conflict: copying a reference would give it to an item that keeps its own |

Differences are warnings and conflicts are errors in the report. Exit code: 0 the documents agree, 1 they differ or clash, 2 unusable input. `--diff` only compares: it does not go with `--apply`, `--package` or `--backward`. The text is in en, es, de, fr, it, ja, zh and pt (the Portuguese of Portugal); the JSON (`phoenix-diff/1`) carries the code and the English detail.

## In the schematic editor

The editor state offers both as data that need no window (`eh3d-phoenix-schedit`, functions `plan_reverse_annotation` and `compare_documents`); the window itself is not changed.

## Settings

`config/annotate.cfg` is also built into the program as the default; a line that is left out keeps the default and a line that is not valid is refused with its number (PHOENIX-ANNOT-010). `values` and `footprints` (true) say whether those changes are taken from the board; `renumber` (false) turns the renumbering on; the rest is described above.

## What is not done

Making a part for a component that has none, deleting a part whose component is gone, swapping pins of different symbols, redrawing the wires after a swap, a three-way comparison with the last common state (so that a change made on both sides could be told from a change made on one), and comparing any of this with the reference tool are listed in `NOT_YET.md`.
