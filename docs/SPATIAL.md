# EH3D-PHOENIX-PCB: the spatial index, and how much work the checks do

A check that compares every object of a board with every other one does work that grows with the square of the number of objects: a board with a few thousand tracks makes millions of comparisons, almost all of them between objects that are far apart. The crate `eh3d-phoenix-spatial` finds the neighbours of a box without those comparisons, and `phoenix perf` shows how much work each check does with it and without it.

**State: prototype.** The index has been compared with the search of every pair on boards and boxes made by the tests from fixed seeds, never on the boards of real designs. The counts say how many comparisons are made; they say nothing about seconds, and no time was measured. The benchmarks of the readers and of the rule check, with a time limit that is a setting, are described in `docs/PERFORMANCE.md`.

## The rule that keeps the results the same

The index only chooses which objects are compared. Every check applies the same exact test it applied before to the candidates, and the candidates are given in ascending order, so a check finds the same things in the same order whether it uses the index or compares every pair. The tests keep that comparison alive:

* the search of every pair stays in the code as a way that can be chosen (`IndexMode::Naive` of the rules crate, `Index::naive` of the index) and is the reference every test compares with;
* for each check there is a test that runs both ways on boards made from fixed seeds (small, medium and of a few thousand objects, with objects on the border of a cell, long pieces that cross many cells, duplicates, negative coordinates and the extremes of the range of the design) and requires the same report, finding for finding and byte for byte in the JSON;
* the index does its arithmetic in `i128`, so the extremes of `i64` do not overflow.

## What uses the index

| Check | What is no longer compared with every object |
|---|---|
| Copper clearance of `phoenix check` (`check_board`) | Each track or via is compared only with the tracks and vias near its box |
| Ratsnest (`phoenix ratsnest`, `ratsnest_with`) | The closest points of two islands with more than 24 anchor points are found with the nearest-point search of the index; smaller islands are compared point by point |
| Advanced checks (`docs/DRC_ADVANCED.md`) | They keep their own grid, with the same rule; `grid_cell_nm` of `config/drc_advanced.cfg` still sets its cell |

The other quadratic parts of the code (the fill of zones, the sharing of nets when a group is pasted, the order of objects when the layers are exported, the neighbours of a push in `route_shove`) have not been moved to the index; they are in `docs/NOT_YET.md` with the reason. The viewer already has an index of its own for the pick of an object and the visible area (`docs/VIEWER.md`).

## How the index works

A uniform grid: the plane is cut into square cells, each box is entered in every cell it covers, and a query returns the boxes in the cells that its own box covers, sorted and without repeats. A box that covers more cells than `max_cells_per_item` is kept in a short list that every query returns. A query that covers more cells than that looks only at the cells that hold boxes, in its columns. The cell is `cell_nm` of the settings, or, when that is 0, twice the mean of the longer sides of the boxes and not less than the extent divided into 4096 parts.

The index counts the candidates it returns (`Index::examined`). That count is the number of comparisons the caller has to make, and it is what the tests of complexity use.

## Tests of complexity, not of time

A test that measured seconds would fail on a slow machine and pass on a fast one. These count instead: for boards of 500, 1000, 2000 and 4000 objects spread over a lattice, the comparisons of the index must stay within a fixed multiple of the number of objects, and the comparisons of every pair must be exactly the square of it. Doubling the objects must not much more than double the comparisons. A change that makes a check quadratic again fails these tests in CI whatever the speed of the machine.

## Settings and limits

`config/spatial.cfg` is a plain `key = value` text (it is also the default built into the program). A line that is left out keeps the default; an unknown key, a key written twice and a value out of range are refused with `PHOENIX-SPATIAL-001`.

| Key | Meaning | Default |
|---|---|---|
| `cell_nm` | the size of a cell in nanometres; 0 chooses it from the boxes | 0 |
| `max_cells` | the most entries (one for each cell that a box covers) the grid may hold; more is refused with `PHOENIX-SPATIAL-002` | 4000000 |
| `max_cells_per_item` | a box that covers more cells than this is kept in the list of big items | 4096 |
| `max_items` | the most boxes one index accepts; more is refused with `PHOENIX-SPATIAL-003` | 4000000 |

When the settings refuse the grid, the copper clearance check compares every pair instead, which gives the same result more slowly; `phoenix perf` shows the refusal and exits with 1.

## The command

```
phoenix perf <board> [--config FILE] [--clearance-mm N] [--json] [--lang CODE]
```

It reads the board and changes nothing. It prints the number of tracks, vias, zones, components and nets, the settings in use and, for the copper clearance check and for the ratsnest, the comparisons made with the index and the comparisons of every pair; for the clearance check also the items, the cells used and the size of a cell. The clearance the check is counted for is `--clearance-mm` (0.2 by default). There is no time in the report, so the same board and settings give the same bytes. `--json` prints the format `phoenix-perf/1`. Exit codes: 0 done, 1 the settings would not let the index be built, 2 the arguments or the input are unusable.

## Codes

`PHOENIX-SPATIAL-001` to `005`, with their titles in the eight languages, are in `docs/ERROR_CODES.md`.
