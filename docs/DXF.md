# EH3D-PHOENIX-PCB: DXF, the outline of a board and the export of boards and panels

The crate `eh3d-phoenix-dxf` and the commands `phoenix dxf-import` and `phoenix dxf-export` read and write DXF drawings, the format that every mechanical CAD program opens, for the outline of a board, its holes and the layout of a panel (requirement PPCB-026, the mechanical export).

**State: prototype.** It has been tried on drawings written by the tests and by the crate itself, with the results worked out by hand (`tests/dxf`), and no CAD program has opened a file it writes or has written a file it reads. The cutouts inside the outline of a drawing are kept as the cutouts of the board outline (`outline_cutouts`), and a board with cutouts is written as one closed contour on the layer of the outline for the outline and for each of them.

Nothing is dropped without a word. What the reader does not understand is counted and listed with its layers; what it approximates says so; what the outline cannot keep is listed.

## The dialect

The writer produces DXF release 12 in ASCII, the dialect that the most programs read and that needs no dependency: header with `$ACADVER`, `$INSUNITS` (millimetres) and `$MEASUREMENT`, a table of layers, and the entities. The text is a pure function of the drawing: no date, no host, no version of the program, the layers in the order of their names, the entities in the order they were made, lengths in millimetres with up to six decimals, the bulge of an arc with up to twelve.

* a path (an outline, a polygon, a run of lines and arcs) is a `POLYLINE` with `VERTEX` entities and the bulge of an arc on the vertex it starts at; a closed path has the flag 1;
* a hole is a `CIRCLE`; a mark is a `POINT` or a cross of two lines; a text is a `TEXT` on one line (without its rotation);
* the names of the layers come from `config/dxf.cfg` (`layer_outline`, `layer_drills`, `layer_pads`, `layer_silk`, `layer_vscore`, `layer_fiducials`, `layer_tabs`, `layer_tooling`, `layer_mouse_bites`, `layer_text`). They must differ from each other without regard to case and may not hold `< > / \ " : ; ? * | = `` ` `` or a comma.

## The reader

`read_dxf` reads release 12 and later in ASCII (also with Windows line ends and a byte order mark) and understands:

| Entity | How it is read |
|---|---|
| `LINE` | a straight piece |
| `ARC` | a piece with its sweep (from the start angle to the end angle, counter-clockwise); a start angle equal to the end angle is a full circle |
| `CIRCLE`, `POINT` | as they are |
| `LWPOLYLINE` | vertices with their bulges, open or closed |
| `POLYLINE` with `VERTEX` | the same; the vertices of the frame of a spline-fit polyline are skipped; a 3D polyline and a mesh are listed as not understood |
| `SPLINE` | a B-spline or NURBS (degree 1 to 10, with its knots and weights) cut into straight pieces so that no piece is further than `spline_tolerance_nm` from the curve, with a finding; one that has only fit points is joined through them, with a finding that says the curve between the points is not the spline |
| `INSERT` | a block with its scale (the same along x and y, or a mirror), its turn and the base point of the block, nested to `max_block_depth`; the entities on layer `0` of a block take the layer of the insert |

The unit is the one of `$INSUNITS` (inch, foot, millimetre, centimetre, metre, kilometre, microinch, mil, yard, micron, decimetre and the rest of the table that the reader knows); a file without it, or with 0, is read in `default_unit` (`mm` or `inch`) with a finding of information, and a number of units that is not known is read in the default with a warning.

What is not understood (ellipses, texts of the drawing, hatches, dimensions, meshes, an array of inserts, an arc or a circle in a block scaled differently along x and y) is counted, and the report lists the kind, how many and on which layers (`PHOENIX-DXF-004`, `PHOENIX-DXF-007`). A file that goes past a limit (`max_file_bytes`, `max_entities`, `max_blocks`, `max_block_depth`) is refused (`PHOENIX-DXF-002`); one that is not DXF is refused with its line (`PHOENIX-DXF-003`).

## The outline of a board from a drawing

`extract_outline` takes the entities of a layer (or of every layer) and finds the outline:

1. the lines and arcs are joined end to end into chains; two ends no further apart than `join_tolerance_nm` (or `--join-tolerance-mm`) are the same end, whatever the order and the direction of the pieces;
2. a chain that returns to its start, a closed polyline and a circle are closed contours; a chain that does not is reported with its ends (`PHOENIX-DXF-011`) and is not part of the outline;
3. a piece that repeats another in either direction, a piece with no length and a contour that repeats another are dropped, and the count is reported (`PHOENIX-DXF-012`); a closed chain with no area is left out (`PHOENIX-DXF-017`);
4. the contours are put in order by containment: one inside no other is an outer contour, one inside one other is a cutout, one inside two is an outer contour again, and so on. The largest outer contour is the outline, counter-clockwise; the others are listed and left out (`PHOENIX-DXF-016`); the cutouts inside it are clockwise, and are kept as the cutouts of the board outline (`PHOENIX-DXF-015`, now an information and no longer a warning; the operation `outline_from_dxf` replaces the outline and the cutouts as one step that one undo takes back);
5. the arcs and circles become straight pieces within `arc_tolerance_nm` (`PHOENIX-DXF-018`);
6. the edges of the outline and of each cutout are checked for crossing themselves and each other, with exact integer arithmetic (`PHOENIX-DXF-013`, an error); a contour with more edges than `max_check_edges` is not checked and the report says so, and two contours whose boxes do not meet are not compared edge by edge (they cannot cross). Before that, a drawing that makes more closed contours than `max_contours` (5000 by default; the containment of the contours is put in order in a time that grows with the square of their number) is refused: the report has the error `PHOENIX-DXF-002`, there is no outline and nothing else is done. A crossing along an overlap and a corner on another edge are not crossings.

A drawing with no closed contour on the layer gives `PHOENIX-DXF-014`. The drawing is used in the coordinates it has: it is not moved to the origin of the board.

## `phoenix dxf-import`

```
phoenix dxf-import <drawing.dxf> --board BOARD [--layer LAYER] [--join-tolerance-mm N]
                   [--config FILE] [--rules FILE] [--force] [--out FILE.json] [--json] [--lang CODE]
```

It applies the operation `outline_from_dxf` of the board editor (`docs/BOARD_EDIT.md`) to the board: the plan is one reversible operation of the board (`set_outline`), tried on a copy, with the design rules counted before and after, and kept only if it adds no rule error (`--force` keeps it anyway). It is all or nothing: a drawing with no closed contour, with a contour that crosses itself or that is not DXF is refused and nothing changes. The default is a dry run: the report shows the entities read, the pieces, chains, contours and duplicates, the findings, the outline before and after (corners and size), the rule errors before and after and `dry run: nothing was written`. `--out` writes the board to a new `.json` file; the board that was read, the drawing and an existing file are never written over. Exit code: 0 done and the rules prove no error, 1 done and an error remains, 2 the arguments or the input are unusable or the operation was refused.

The operation can also be written in a file of operations (`phoenix boardedit --apply`):

```
{"op": "outline_from_dxf", "dxf": "<text of the file>", "layer": "EDGE", "join_tolerance_nm": 10000}
```

`layer` and `join_tolerance_nm` may be left out; `settings` may hold the text of a `config/dxf.cfg`.

## `phoenix dxf-export`

```
phoenix dxf-export <board|panel> [--out FILE.dxf] [--config FILE] [--panel-config FILE]
                   [--boards A,B,...] [--silk] [--json] [--lang CODE] <file>
```

A board gives:

* the outline, a closed polyline on `layer_outline`;
* the drilled holes, a circle for each via and each drill of a pad, on `layer_drills`;
* the outlines of the pads, on `layer_pads`: a circle, a rectangle or a rounded rectangle (two lines and two half turns) turned with the component, with the pad mirrored along x on the bottom side;
* with `--silk`, the strokes of the silkscreen as polylines (with their arcs) and the texts as texts on `layer_silk` (a text is written without its rotation and its mirror);
* with `write_text = true`, the name of the board as a text below the lower left corner.

A panel (`phoenix print panel` takes the same files and the same `--panel-config`, `docs/PANEL.md`) gives the frame as a closed polyline and the routed outline as lines on `layer_outline`, the V-score lines on `layer_vscore`, a cross for each tab on `layer_tabs`, the holes of the boards on `layer_drills`, the mouse bites on `layer_mouse_bites`, the tooling holes on `layer_tooling`, and each fiducial as a circle and a cross on `layer_fiducials`; with `write_text = true`, the name of each board at its lower left corner.

It is a dry run unless `--out` names a file that ends in `.dxf` and does not exist; it never writes over a file or an input. The report counts the entities of each layer and always carries `PHOENIX-DXF-019`: what is in the file is the outlines, the holes and the marks, not the copper, the mask, the paste or the widths.

## Settings

`config/dxf.cfg` holds the names of the layers, the size of the text and of the crosses, the tolerances (`join_tolerance_nm`, `spline_tolerance_nm`, `arc_tolerance_nm`), the unit of a drawing without one and the limits of the reader (`max_file_bytes`, `max_entities`, `max_block_depth`, `max_blocks`, `max_spline_points`, `max_findings`, `max_check_edges`, `max_contours`). Every line is optional; an unknown key, a name that DXF forbids or a value out of range is refused with its line number (`PHOENIX-DXF-001`).

## Codes

`PHOENIX-DXF-001` to `019`, with their titles in the eight languages, are in `docs/ERROR_CODES.md`.
