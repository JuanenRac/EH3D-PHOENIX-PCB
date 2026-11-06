# EH3D-PHOENIX-PCB: filling copper zones

The crate `eh3d-phoenix-zones` and the command `phoenix zones` fill the copper zones of a board and measure the result against the rules (requirement PPCB-016, copper zones). A zone is a polygon with a net, a layer and a clearance; the fill is the copper that is left inside it once everything it has to keep away from is taken out.

**State: prototype.** It has been tried on boards made by hand and by the tests only, never on a real board and never compared with the fill of the reference tool. The areas the tests expect were worked out on paper. Nothing written here has been opened by a viewer of Gerber files: the export (`docs/EXPORT.md`) writes a zone as its outline, except for the board that the board editor pours with the polygons of a fill (`docs/BOARD_ZONES.md`).

## What the fill does

For each zone, in the order of priority (higher first) and then of the id of the zone:

1. **Convex pieces.** The outline is checked (at least three points, an area, no two sides touching) and cut into convex pieces: the outline itself when it is convex, else triangles made by cutting off ears. An outline that cannot be used gives the finding `PHOENIX-ZONE-004`, the zone gets no copper and the others are filled.
2. **What to keep away from.** The copper of other nets on the layer (pads, tracks, vias), the edge of the board, the zones of a higher priority and the pads and vias of the net that are not to be joined. Each is turned into a convex polygon that holds the copper grown by the clearance. A circle is the circumscribed polygon of 8 or 16 sides, so the gap is never smaller than the rule and a little wider at the corners. A pad at a right angle keeps its shape (a rectangle by its four corners, an oblong by its axis); a pad at another angle is the smallest circle around it. An arc is a short polyline whose pieces are widened by their sagitta (`PHOENIX-ZONE-013`).
3. **Clipping.** Each polygon is taken out of every piece it touches. A convex polygon taken out of a convex piece leaves convex pieces, found with one cut per side of the polygon; a polygon that only touches a piece, or lies outside it, changes nothing. All the arithmetic is on integer nanometres with products in 128 bits, and the orientation tests are exact. The only inexact step is the point where a line cuts a side: it is rounded to the nearest nanometre, the same way on both sides of the cut.
4. **Thermal reliefs.** Around each pad of the net (`pad_connection = thermal`) four blocks (or two, `thermal_spokes = 2`) are taken out, leaving spokes of `thermal_spoke_width_nm` along the axes. The inner radius is the radius of the smallest circle around the pad, and at least `thermal_inner_radius_nm`; the outer radius is the inner radius plus `thermal_gap_nm`, and at least `thermal_outer_radius_nm`. Spokes too wide for the ring give `PHOENIX-ZONE-011` and the pad is joined solid. A pad joined `solid` is left as it is; one joined `none` is kept apart like copper of another net.
5. **Islands.** Pieces outside the board are dropped. The rest are grouped into islands by the stretches of border they share. An island is dropped when its area is less than `min_island_area_nm2` (`PHOENIX-ZONE-009`), when its convex hull is narrower than `min_width_nm` (`PHOENIX-ZONE-008`) or when it touches no copper of the net (`PHOENIX-ZONE-010`, unless `keep_unconnected_islands`). These three are information: the island is gone and the report says so.
6. **Order.** Each polygon is made counter-clockwise, loses the points that lie exactly on a side and starts at its smallest point; the polygons are sorted. The same board always gives the same bytes.

The result is a set of convex polygons that touch along their sides and never overlap. It is not merged into one outline per island.

## Priority and limits of single zones

The board model has no priority for a zone, so it comes from an optional file, `--zones FILE`, of the format `phoenix-zones/1`:

```json
{
  "format": "phoenix-zones/1",
  "zones": [
    {"zone": "<id of the zone>", "priority": 1, "clearance_nm": 300000,
     "min_width_nm": 200000, "min_island_area_nm2": 500000000, "connection": "solid"}
  ]
}
```

The board model holds the `cutouts` of a zone too (a list of closed polygons in the zone, not written when empty); the Gerber export writes each as a region of clear polarity. A zone may also give the thermal relief of its own pads (`thermal_gap_nm`, `thermal_spoke_width_nm`, `thermal_spokes` of 2 or 4) and `cutouts`, a list of polygons (each a list of `[x, y]` in nanometres, at least three points) that the fill keeps away from, up to their exact edge. A cutout that is not a polygon gives the finding `PHOENIX-ZONE-004` and the zone gets no copper. Every field but `zone` is optional; a zone the file does not name has priority 0 and the values of the board and of the settings. A zone of a higher priority is filled first; a zone below it keeps away from its whole outline, by the clearance when the nets differ and by nothing when they are the same. A zone the file names that the board does not have is the error `PHOENIX-ZONE-003` and nothing is filled. Two zones of different nets on one layer with the same priority whose outlines overlap give the warning `PHOENIX-ZONE-012`.

## The clearance

The gap kept from copper of another net is the largest of the clearance of the zone (or of its entry in the file), the clearance of the rule file, and the clearance of the net classes of both nets. The gap to the edge of the board is the clearance to the edge of the rule file. A board without rules is filled with the clearance of its zones only; the command says so.

## The check

`check_fill` (and `phoenix zones check`) measures a fill against the board, independently of the way it was made. With a tolerance of `snap_tolerance_nm` for the rounded cut points it proves, for each zone:

* every polygon lies inside the outline of the zone and inside the board (`PHOENIX-ZONE-007`);
* every side of every polygon is at least the clearance away from each pad, track and via of another net, from each pad or via of the net that is not joined and from the edge of the board, and no such copper is inside a polygon (`PHOENIX-ZONE-006`);
* no island is smaller than the minimum area or narrower than the minimum width (`PHOENIX-ZONE-009`, `PHOENIX-ZONE-008`);
* an island that touches no copper of the net is a warning (`PHOENIX-ZONE-010`).

Distances between segments are compared with a limit without any square root, so a polygon exactly at the clearance passes and one a nanometre closer (beyond the tolerance) fails. The fill file carries the rules it was made with (`clearance_nm`, `min_width_nm`, `min_island_area_nm2`, `connection`), so the check can be run later, on another machine, against the board.

## The commands

```
phoenix zones fill  [--config FILE] [--rules FILE] [--zones FILE] [--out FILE] [--json] [--lang CODE] <board>
phoenix zones check [--config FILE] [--rules FILE] [--zones FILE] [--fill FILE] [--json] [--lang CODE] <board>
```

`<board>` is an EAGLE board or the JSON board of a project. `fill` is a dry run: it prints the report and writes nothing unless `--out` names a file, which must not exist (an existing file, the board included, is never replaced). `check` measures the file given with `--fill`, or, without it, the fill it makes itself. Exit code 0 when done, 1 when a zone could not be filled (`fill`) or a rule is proven broken (`check`), 2 when the arguments or the input are unusable.

The file written by `--out` has the format `phoenix-zone-fill/1`: the name of the board, the tolerance, and for each zone its id, net, layer, priority, rules, area and polygons as lists of `[x, y]` in nanometres.

## Settings

Every radius, width, limit and default is in `config/zones.cfg`, which is also the built-in default; no value is written in the code. A line that is not `key = value`, an unknown key or a value out of range is refused with `PHOENIX-ZONE-001`.

## Codes

`PHOENIX-ZONE-001` to `PHOENIX-ZONE-019`, with titles in the eight languages (`src/codes.rs`) and the table in `docs/ERROR_CODES.md`.

## Limits

* The width test looks at the convex hull of a whole island: it finds a strip or a sliver, not a narrow neck inside a wide island.
* A pad at an angle that is not a multiple of 90 degrees is the smallest circle around it, which keeps more copper away than needed.
* A circle is a polygon around it, so the gap at the corners is a little wider than the rule; arcs are widened by their sagitta.
* Same-net zones of equal priority are not merged, and the zone below another keeps away from the whole outline of the zone above, not from its fill.
* A via joins the copper layers between its first and its last layer when the board gives the stack, else only those two.
* Keep-out areas are not read; the board model has none. A hole that is not plated has no copper, so the fill does not look at it as a pad.
* The export writes a zone as its outline; the board editor makes a board whose zones are the polygons of the fill for the Gerber files (`docs/BOARD_ZONES.md`). The rule check of the board still judges a zone by its outline.
* Nothing has been compared with the fill of the reference tool, and the speed on a large board has not been measured.
