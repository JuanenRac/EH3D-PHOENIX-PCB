# EH3D-PHOENIX-PCB: arcs in the outline of the board

The outline of a board (`Board::outline`) is a closed polygon of corners. An edge of that polygon may be a circular arc instead of a straight piece: the board keeps the arcs apart, in `Board::outline_arcs`, and the corners stay as they were.

**State: prototype.** It has been tried on boards made by the tests only. The Gerber outline file writes the arcs as arcs; the checks of distance, the pour of the zones and the router see the polygon that the arcs are cut into; the viewer draws them as the chords they are cut into; the writers of the other formats and the board editor do not know the arcs yet (see the limits).

## The model

Each entry of `outline_arcs` is an `OutlineArc`:

| Field | Meaning |
|---|---|
| `edge` | The index in `outline` of the corner where the edge starts. The edge runs to the next corner (the last corner joins the first). |
| `sweep` | The sweep of the arc from the first corner to the next, in micro-degrees. Positive is counter-clockwise. It must be above zero and below a full turn. |

The end points of an arc are the two corners of its edge, so an arc has a radius, a centre and a side that follow from the chord and the sweep. For a counter-clockwise outline a positive sweep bulges outwards and a negative sweep bulges into the board. The field is not written when it is empty, so a board without arcs is written with the same bytes as before and a board made before it existed reads as before.

## The polygon of straight pieces

`Board::outline_polygon(settings)` is the outline as a polygon of straight edges: the corners with, between the two corners of each arc, the points that cut the arc into chords at most `tolerance_nm` from it. Number of chords: `ceil(sweep / (2 * acos(1 - tolerance / radius)))`, at least one and at most `max_pieces`. Every point is rounded to the nanometre and the corners are never moved. A quarter circle of 1 mm radius with a tolerance of 1 micrometre is 18 chords of 5 degrees (17 chords would be 1067 nm from the arc, 18 are 951.8 nm). The chords are inscribed, so the area is a little smaller than the true area (a half circle of 5 mm radius loses 0.017 percent).

The two numbers are in `config/outline_arcs.cfg`, which is also the built-in default (`ArcSettings`):

```
tolerance_nm = 1000
max_pieces = 1024
```

A line that is not `key = value`, an unknown key or a value out of range (1 to 1000000 for the tolerance, 1 to 65536 for the pieces) is refused with its line number. Every command reads the two numbers from the option `--limits FILE`, whose file may hold them together with the lines of `config/board.cfg` and `config/eagle_board.cfg` in any order; the rule check, the zone fill, the own autorouter, the window and the writers (KiCad, EAGLE, DXF, Specctra) cut the arcs with them, and without the option the defaults are used.

## The checks

`Board::validate` raises `PHOENIX-BOARD-009` (an arc that has no shape) for an arc of the outline that names an edge the outline does not have, that has a zero sweep or a full turn or more, whose end points cannot make an arc, or that is the second arc of an edge; the detail says which arc (counting from 1) and why. An arc that cannot be used is left as the straight edge. The test of crossing edges (`PHOENIX-BOARD-006`, bounded by `max_polygon_points`) is made on the polygon of straight pieces, so an arc that crosses another edge of the outline is found.

## Where the arcs are used

* **Gerber outline file:** an arc edge is written as a circular interpolation (`G02` or `G03` with the offset to the centre, as the arcs of a track), the other edges as lines; the check of the export counts lines and arcs separately.
* **Rule check:** the distance of tracks, vias and pads to the edge of the board is measured against the polygon of straight pieces.
* **Zone fill:** the edge of the board that the fill keeps away from, and the test that a piece of copper is inside the board, use the same polygon.
* **Own autorouter:** the obstacle of the board edge and the size of the grid use the same polygon.
* **KiCad reader:** an arc drawn on `Edge.Cuts` that is the only piece between its two end points is read as an arc of the outline with the sweep computed from its three points; a circle, an arc of an opening and an arc that shares its end points with another piece (two half circles that make a circle, a half disc) are cut into straight pieces as before (`docs/KICAD.md`).
* **Viewer:** the drawing of the board (`Scene::from_board`) draws the outline as the same polygon.

## Limits

* There is no editing operation and no tool of the window for the arcs of the outline: the field is set by a reader or by the code that builds the board. The drawing of the editor window (`board_gui`) and the printed scenes still draw the outline by its corners.
* The writers of KiCad, EAGLE, Specctra and DXF files write the outline as the polygon of straight pieces that the arcs are cut into, so the arc is not an arc in them; the router space of `phoenix route` uses the same polygon. The panel, the placement file, the 3D views and the bill of materials read the corners only, so an arc becomes a straight edge in them.
* The cutouts inside the outline are polygons of straight pieces; they have no arcs.
* The cut is made with floating point and rounded to the nanometre; it is the same on the machines that round the same way and has not been compared on several.
* Nothing has been compared with the outline that another tool draws for the same arc.
