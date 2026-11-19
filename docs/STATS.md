# EH3D-PHOENIX-PCB: statistics of a board

The crate `eh3d-phoenix-stats` and the command `phoenix stats` give the numbers of a board in one report: its size and area, what it holds, the copper of each layer, the drills by diameter and how tightly it is filled.

**State: prototype.** It has been tried on boards made by hand and by the tests only, never on a real board. The numbers the tests expect were worked out on paper. The copper of a layer is an upper bound, as explained below.

```
phoenix stats [--config FILE] [--out FILE] [--json] [--lang CODE] <board>
```

The board is a `.json` board, an EAGLE `.brd` or a KiCad `.kicad_pcb`, as for the other commands. The report goes to the output and nothing is written unless `--out` names a new file: it gets the JSON report, created exclusively, and a file that exists is never replaced (`PHOENIX-STATS-004`). The exit code is 0 when the report is made, 1 when a finding of the report is an error and 2 when the arguments or the input are unusable.

## What the report holds

* **The board**: the width and the height of the box of the outline, the area inside the outline, the area of its cutouts and the area of the board (the outline less its cutouts). The arcs of the outline are not in the area: it is the area of the polygon of its corners.
* **The counts**: the placed components (and how many are on each side), the nets, the pads (without a drill, with a plated drill, and the holes that are not plated), the tracks with their pieces and total length, the vias, the zones and the items of the silkscreen.
* **The copper of each layer**, from the top of the stack: the pads with copper on it, the tracks, the vias that cross it and the zones, each with its area, and the sum of the four with the share of the board it covers.
* **The drills**, by diameter from the smallest: the vias, the plated pads and the holes that are not plated that have each diameter.
* **The balance**: for each pair of layers that mirror each other, the coverage of both and the difference.
* **The regions** (only when the settings ask for them): the copper of each layer in each cell of a grid over the board.
* **The density**: the components, the pads and the holes (vias and drilled pads) per square centimetre of board.

## The copper of a layer

The copper of a layer is the sum of four parts:

* the **pads** of the parts on that layer: the shape (a circle, a rectangle with its rounded corners, an oblong) less the hole of a plated pad; a hole that is not plated has no copper;
* the **tracks**: the length times the width and the two round ends of each track;
* the **vias** that cross the layer: the ring between the drill and the pad. With the order of the copper layers given (`copper_stack`) a via crosses every layer from its first to its last layer; without it, only those two;
* the **zones**: the outline less its cutouts, before the clearances are taken off. The information `PHOENIX-STATS-006` says so when a board has zones; the setting `count_zones = false` leaves the area of the zones out and keeps the count.

The four parts overlap where a track ends on a pad or a zone surrounds both, and the overlap is not taken away, so the sum is an upper bound. The copper of a layer is never counted above the area of the board, so the coverage is at most 100 %.

## The balance of the copper

A board whose copper is uneven across the middle of the stack bends when it is heated, so the layers that mirror each other should carry about the same share of copper. The layers are paired from the outside in: the first with the last, the second with the one before the last, and so on; the layer in the middle of an odd stack has no pair. The difference of a pair is the difference of the coverage of its two layers, in percentage points.

A pair whose difference is above `balance_max_difference_x100` (in hundredths of a point, 1500 by default) gives `PHOENIX-STATS-008`. Its severity is the one that `balance_severity` names: `warning` by default, `info` to only note it, `error` to make `phoenix stats` exit with 1. `balance_check = false` leaves the comparison out. Without the order of the copper layers on the board the pairs follow the order of the layer ids, and `PHOENIX-STATS-009` says so. The coverage comes from the copper of the layers, so it has the limits of that number (zones as their whole outline, no overlap taken away).

## The copper by region

A board whose copper is heavy in one corner and thin in another warps when it is heated, even when the coverage of the whole layer is right. With `region_columns` and `region_rows` above 1 (both 1 by default, so there are no regions) the box of the outline is cut into that many equal columns and rows, and the report adds the copper of each layer up in every cell. A cell is the part of the box inside the board: its area is the outline less its cutouts, clipped to the cell, and its coverage is the copper over that area, in hundredths of a percent. The columns go from the least x and the rows from the least y; the text prints the row with the greatest y first, as the board is seen, and the JSON keeps the cells in order, row by row (`column` and `row` count from 0).

The areas of a layer are shared between the cells like this:

* a **zone** and each of its cutouts are clipped to every cell, so a zone is shared exactly;
* a **track** is shared in proportion to the length of each of its pieces that lies in each cell, and its two round ends go to the cells of the two ends of the track;
* a **pad** and a **via** go whole to the cell that holds their centre (a centre on the edge between two cells is in the one with the greater column or row).

The copper of a cell is never counted above its area, as for a whole layer, and a cell whose area is zero (a grid cell that lies wholly in a cutout, or outside a board that is not a rectangle) has coverage zero and is left out of the judgement. A grid has at most 16 columns and 16 rows.

With `region_max_difference_x100` above 0 (hundredths of a percentage point, 0 by default, which leaves the regions unjudged) a layer whose most and least covered cells differ by more than that gives `PHOENIX-STATS-010`, with the severity of `balance_severity`. Like the rest of the report it is a prototype: the copper is an upper bound, and the split of a pad or a via by its centre is coarse where the pad is large.

## The numbers

Lengths are integer nanometres and areas are integer square micrometres; the text shows millimetres and square millimetres with six decimals, exactly, and the coverage in percent with two decimals. The density is shown per square centimetre with two decimals; in the JSON it is the integer times 100. The text has its titles in the eight languages (`--lang`); the JSON keeps its keys. The same board gives the same bytes every time.

## Settings

`config/stats.cfg` holds the limit and the choices, and is also the built-in default:

* `max_items`: the most objects (parts, tracks, vias, zones, connected pads and silkscreen items) a board may hold for the report to be made; above it the command stops with `PHOENIX-STATS-005`;
* `count_zones`: whether the area of the zones is counted as copper;
* `balance_check`, `balance_max_difference_x100` and `balance_severity`: the comparison of the layers that mirror each other, its limit and how serious a pair above it is.
* `region_columns`, `region_rows` and `region_max_difference_x100`: the grid of the regions (1 by 1, none, by default) and the most that the coverage of the cells of one layer may differ (0, unjudged, by default).

A line that is not `key = value`, an unknown key or a value out of range is refused with its line number (`PHOENIX-STATS-001`).

## Codes

`PHOENIX-STATS-001` to `PHOENIX-STATS-010`, with titles in the eight languages (`src/codes.rs`) and the table in `docs/ERROR_CODES.md`.

## Limits

* The copper is an upper bound: overlaps are not taken away and a zone is its whole outline before the clearances.
* The arcs of the outline are not in the area of the board.
* A polygon that crosses itself gives the sum of its signed pieces, not the area of its union.
* Nothing has been compared with the report of another tool.
