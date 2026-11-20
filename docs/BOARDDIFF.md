# EH3D-PHOENIX-PCB: comparison of two boards

The crate `eh3d-phoenix-boarddiff` and the command `phoenix boarddiff` compare two revisions of a board and list what the new one has that the old one has not, what it lost and what it changed. It is for reading a change before it goes to the maker, and for finding what an edit really touched.

**State: prototype.** It has been tried on boards made by hand and by the tests only.

```
phoenix boarddiff [--config FILE] [--csv FILE] [--json] [--lang CODE] <old board> <new board>
```

Each board is a `.json` board, an EAGLE `.brd` or a KiCad `.kicad_pcb`, as for the other commands. The command writes nothing unless `--csv` names a new file (see below). The exit code is 0 when the boards do not differ in anything that was compared, 1 when they do, and 2 when the arguments or an input are unusable.

## What is compared

The objects are matched by what they are called, never by their ids, so two boards made apart from the same design (or the same board read from two programs) can be compared.

| Kind | Matched by | A difference is | Code |
|---|---|---|---|
| Layers | name | a layer only one board has | `PHOENIX-DIFF-005` |
| Nets | name | a net only one board has | `PHOENIX-DIFF-006` |
| Parts | reference | a part only one board has, with its value, footprint and side | `PHOENIX-DIFF-007` |
| Parts in both | reference | another value (`008`), another footprint name (`009`), a place farther apart than `move_tolerance_nm`, another turn or the other side (`010`) | `008` to `010` |
| Pins | reference and pad number, for the parts both boards have | the pin on another net, or on a net in one board and on none in the other (`compare_pins`) | `PHOENIX-DIFF-011` |
| Copper | net and layer | another number of tracks, or a length that differs by more than `length_tolerance_nm` (`compare_tracks`) | `PHOENIX-DIFF-012` |
| Vias | net and place | a via that one board has more of (`compare_vias`) | `PHOENIX-DIFF-013` |
| Zones | net and layer | zones in one board only, or another number of zones or another area (`compare_zones`) | `PHOENIX-DIFF-014` |
| Outline | the outline as a whole | another number of corners or cutouts, another box or another area (`compare_outline`) | `PHOENIX-DIFF-015` |

The order of the report is fixed: the kinds in the order of their codes, and inside a kind the names in order, so the same two boards always give the same report. A reference that two parts of the same board share is noted (`PHOENIX-DIFF-016`) and only the first of them is compared. When there are more differences than `max_differences` the rest are counted and the list ends with a note (`PHOENIX-DIFF-017`).

The length of a track is the sum of its pieces: a straight piece by its two ends and an arc by its radius and sweep, each rounded to a whole nanometre. The area of a zone or of the outline is worked out exactly from the integer corners (twice the area is an integer) with the cutouts taken away, and written in square millimetres with three decimals.

## Settings

`config/boarddiff.cfg` holds every value and is also the built-in default.

| Key | Default | Meaning |
|---|---|---|
| `move_tolerance_nm` | 1000 | a part that moved by no more than this has not moved (a turn or a change of side always counts) |
| `length_tolerance_nm` | 1000 | the copper of a net on a layer whose length changed by no more than this, with the same number of tracks, has not changed |
| `compare_pins`, `compare_tracks`, `compare_vias`, `compare_zones`, `compare_outline` | true | which kinds of object are compared |
| `ignore_parts`, `ignore_nets` | empty | references and net names that are left out (any case; a word that ends in `*` matches every name that starts with the rest) |
| `max_items` | 1000000 | the most parts, pins, tracks, vias and zones a board may hold for the comparison to run |
| `max_differences` | 1000 | the most differences a report lists |
| `csv_guard_formulas` | true | whether a cell of the CSV file that starts with `=`, `+`, `-` or `@` gets a quote mark in front |

## The report

The text report names the two boards, the counts of parts, nets, pins, tracks, vias and zones in each, how many differences there are and each of them with its code, its title in the language of `--lang` (the eight languages of the suite), the part, net, pin or place it is about and what changed (`10k -> 22k`, `moved 2 mm, turned from 0 to 90 degrees`). `--json` prints the format `phoenix-boarddiff/1`, the same bytes for the same two boards.

## The table for a spreadsheet

`--csv FILE` also writes the differences to a new file as a table with the header `code,subject,detail` and one row for each difference of the report, in its order. The file is created exclusively: a name that exists stops the command (`PHOENIX-DIFF-018`) and the file is not touched. A cell that a spreadsheet would run as a formula, such as the net `+5V`, gets a quote mark in front unless `csv_guard_formulas` is false, and a cell with a comma, a quote or a line break is quoted. With the text report the command adds a line `written: FILE`; with `--json` it adds nothing.

## Codes

`PHOENIX-DIFF-001` to `PHOENIX-DIFF-018`, with titles in the eight languages (`src/codes.rs`) and the table in `docs/ERROR_CODES.md`.

## Limits

* A board whose references were renumbered looks like every part removed and added: the match is by reference.
* A track is not matched to a track: only the number and the length of the tracks of a net on a layer are compared, so a track that was rerouted to the same length and number is not seen. A via that moved is a via removed and a via added.
* A zone and the outline are compared by their size and area, not by their corners, so a change of shape that keeps both is not seen.
* The silkscreen, the dimensions, the pads' shapes and the parts' pad positions are not compared.
