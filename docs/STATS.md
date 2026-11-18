# EH3D-PHOENIX-PCB: statistics of a board

The crate `eh3d-phoenix-stats` and the command `phoenix stats` give the numbers of a board in one report: its size and area, what it holds, the copper of each layer, the drills by diameter and how tightly it is filled.

**State: prototype.** It has been tried on boards made by hand and by the tests only, never on a real board. The numbers the tests expect were worked out on paper. The copper of a layer is an upper bound, as explained below.

```
phoenix stats [--config FILE] [--json] [--lang CODE] <board>
```

The board is a `.json` board, an EAGLE `.brd` or a KiCad `.kicad_pcb`, as for the other commands. The command only reads: the report goes to the output and nothing is written. The exit code is 0 when the report is made, 1 when a finding of the report is an error and 2 when the arguments or the input are unusable.

## What the report holds

* **The board**: the width and the height of the box of the outline, the area inside the outline, the area of its cutouts and the area of the board (the outline less its cutouts). The arcs of the outline are not in the area: it is the area of the polygon of its corners.
* **The counts**: the placed components (and how many are on each side), the nets, the pads (without a drill, with a plated drill, and the holes that are not plated), the tracks with their pieces and total length, the vias, the zones and the items of the silkscreen.
* **The copper of each layer**, from the top of the stack: the pads with copper on it, the tracks, the vias that cross it and the zones, each with its area, and the sum of the four with the share of the board it covers.
* **The drills**, by diameter from the smallest: the vias, the plated pads and the holes that are not plated that have each diameter.
* **The density**: the components, the pads and the holes (vias and drilled pads) per square centimetre of board.

## The copper of a layer

The copper of a layer is the sum of four parts:

* the **pads** of the parts on that layer: the shape (a circle, a rectangle with its rounded corners, an oblong) less the hole of a plated pad; a hole that is not plated has no copper;
* the **tracks**: the length times the width and the two round ends of each track;
* the **vias** that cross the layer: the ring between the drill and the pad. With the order of the copper layers given (`copper_stack`) a via crosses every layer from its first to its last layer; without it, only those two;
* the **zones**: the outline less its cutouts, before the clearances are taken off. The information `PHOENIX-STATS-006` says so when a board has zones; the setting `count_zones = false` leaves the area of the zones out and keeps the count.

The four parts overlap where a track ends on a pad or a zone surrounds both, and the overlap is not taken away, so the sum is an upper bound. The copper of a layer is never counted above the area of the board, so the coverage is at most 100 %.

## The numbers

Lengths are integer nanometres and areas are integer square micrometres; the text shows millimetres and square millimetres with six decimals, exactly, and the coverage in percent with two decimals. The density is shown per square centimetre with two decimals; in the JSON it is the integer times 100. The text has its titles in the eight languages (`--lang`); the JSON keeps its keys. The same board gives the same bytes every time.

## Settings

`config/stats.cfg` holds the limit and the choices, and is also the built-in default:

* `max_items`: the most objects (parts, tracks, vias, zones, connected pads and silkscreen items) a board may hold for the report to be made; above it the command stops with `PHOENIX-STATS-005`;
* `count_zones`: whether the area of the zones is counted as copper.

A line that is not `key = value`, an unknown key or a value out of range is refused with its line number (`PHOENIX-STATS-001`).

## Codes

`PHOENIX-STATS-001` to `PHOENIX-STATS-007`, with titles in the eight languages (`src/codes.rs`) and the table in `docs/ERROR_CODES.md`.

## Limits

* The copper is an upper bound: overlaps are not taken away and a zone is its whole outline before the clearances.
* The arcs of the outline are not in the area of the board.
* A polygon that crosses itself gives the sum of its signed pieces, not the area of its union.
* Nothing has been compared with the report of another tool.
