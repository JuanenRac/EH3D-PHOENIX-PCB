# EH3D-PHOENIX-PCB: editing and routing logic

The crate `eh3d-phoenix-route` and the commands `phoenix route` and `phoenix ratsnest` are the part of phase F6 that needs no window (requirement PPCB-016): the connections still to be routed, a router for tracks at 0, 45 and 90 degrees, the editing operations with undo and redo, and the live check that says what an edit adds or removes.

**State: prototype.** It has been tried on boards made by hand and by the tests only, never on a real board and never compared with the router of the reference tool. The window of the viewer (`docs/VIEWER.md`) puts these functions behind tools, with undo and redo and a panel of the findings that each step adds and resolves, but that window has never been seen on a screen.

## The ratsnest

`ratsnest(board, settings)` takes the islands of each net from the connectivity of the board (which copper touches which, with exact integer arithmetic and no tolerance) and joins them with a minimum spanning tree built with Prim's algorithm. The distance between two islands is the distance between their closest points: the centre of a pad, a via, the vertices of a track or of a zone outline. Every tie is broken by the lowest index, so the same board always gives the same lines, in the order of the nets. The number of pairs of points compared is limited by `ratsnest_max_pairs`; beyond it the result is an error with the code `PHOENIX-ROUTE-002`, never a partial list.

## The router

`route(board, rules, settings, request, new_id)` joins two elements of the same net: a pad (`pad:R1.2`, or by the ids of its component and pad), a via (`via:<id>`) or an end of a track (`track:<id>`, `track:<id>:start`). It returns a plan, and the board is not changed: the plan holds the operation that adds the tracks and the vias, so the caller applies it, undoes it and redoes it like any other edit.

* **The width and the vias.** A track has the width its net class asks for, or `default_width_nm`, and never less than the narrowest track of the rules. A via has the drill and the diameter of the settings, raised to the minimum drill and via diameter of the rules and of the class, and the diameter is raised to the drill plus two annular rings.
* **The clearance.** The router keeps away from the pads, tracks and vias of other nets on the layer by the largest of the clearance of the rules, the clearance of the class and the contact tolerance plus one nanometre, plus `clearance_margin_nm`, and from the edge of the board by the clearance to the edge plus the same margin. A pad is avoided as the smallest circle around it and an arc of another track as a short polyline widened by its sagitta, so a way that the router takes is always clear and a way that it refuses may be cautious. Distances are compared exactly, with no square root.
* **Mode `stop`.** It walks the straight way (one piece, or two when the angle is not a multiple of 45 degrees, in both orders) and stops, to the nanometre, in front of the first obstacle. The result is a valid track that does not reach the second element; the plan says so (`PHOENIX-ROUTE-008`, exit code 1 of the command). A way blocked at the start, before `stop_min_length_nm`, is an error.
* **Mode `around`.** It searches a coarse mesh, `grid_nm` apart and anchored at the start of the track, in eight directions, with a priority queue. The cost is the length plus `corner_penalty_nm` for every corner and `via_penalty_nm` for every via, so the route with the fewest corners wins among equal lengths. Turns sharper than 90 degrees are not allowed unless `allow_acute` says so. Near the end the search finishes with one or two straight pieces to the exact end, so the end need not be on the mesh. When the ends are on different layers, or a via makes the way shorter, it changes layer with a via. When the area needs more than `max_cells` points the pitch is doubled until it fits, and the search stops with `PHOENIX-ROUTE-002` after `max_expansions` steps.
* **Never a broken track.** The plan is applied to a copy of the board and judged by the rules the board is checked with (`PHOENIX-ROUTE-009` if they find a new error), and a complete plan must really join the two elements in the connectivity of the board (`PHOENIX-ROUTE-010`). An impossible route is an error with its code (`PHOENIX-ROUTE-007`), and the board is as it was. The same board and the same request always give the same plan.

## Editing

The functions of `edit` build operations of the board (`eh3d_phoenix_board::Operation`) from its state: `move_component`, `rotate_component`, `delete_track`, `move_vertex`, `set_track_width`, `add_via` and `via_for_layer_change` (the via a change of layer needs, sized by the rules and the class). Replacing a track is a batch (remove it, add it back changed with the same id), so it is all or nothing and its inverse brings the board back byte for byte. Moving a vertex with `keep_angles` refuses to turn a piece that ran at 0, 45 or 90 degrees into another angle (`PHOENIX-ROUTE-016`); a vertex that does not exist or a piece that would shrink to a point is `PHOENIX-ROUTE-015`. Moving a component does not move its tracks: the ratsnest shows what has to be routed again.

`Editor` applies an operation with the history of the board (undo and redo) and returns the live check: the findings of the rules that the edit added and removed. The operation is tried on a copy first, so a failure leaves the board untouched and records nothing; with `strict_edits` an edit that would add a rule error is refused (`PHOENIX-ROUTE-014`).

## The live check

`findings(board, rules)` gathers the proven findings of the rule check, of the net class check and the copper of different nets that touches; `diff(before, after)` returns what appeared and what went away. Inconclusive findings are not part of the difference: they say that something could not be judged, not that something changed.

## The commands

```
phoenix ratsnest [--settings FILE] [--json] [--lang CODE] <board>
phoenix route [--settings FILE] [--rules FILE] [--layer NAME] [--mode around|stop] [--json]
              [--lang CODE] --from ELEMENT --to ELEMENT [--out FILE] <board>
```

`<board>` is an EAGLE board or the JSON board of a project. `--rules` is a rule file (`phoenix-rules/1` or `/2`, with net classes); an EAGLE board gives its own design rules when the option is left out, and a JSON board without rules is routed with the width, the edge of the board and the copper of other nets only, which the command says. `route` prints the plan; with `--out FILE` it writes the board with the new tracks and vias to a new file, and an existing file (the input included) is never replaced. Exit code 0: done (for `route`, the track reaches the second element); 1: the track stopped in front of an obstacle; 2: the arguments, the input or the route are unusable.

## Settings

Every pitch, penalty and limit is in `config/route.cfg`, which is also the built-in default; no value is written in the code of the router. A line that is not `key = value`, an unknown key or a value out of range is refused with `PHOENIX-ROUTE-001`.

## Codes

`PHOENIX-ROUTE-001` to `PHOENIX-ROUTE-020`, with titles in the eight languages (`src/codes.rs`) and the table in `docs/ERROR_CODES.md`.

## Limits

* Zones are not routed around: a track through a zone of another net is refused by the rules, not avoided by the search.
* Tracks have no arcs, only the straight pieces at 0, 45 and 90 degrees; arcs of other tracks are avoided.
* A via joins exactly the two layers it is placed between, as the board model says; a through via on a board of four layers is not made.
* The ratsnest uses the connectivity of the board with no tolerance; the rule check may treat copper that nearly touches as touching.
* The search is a dense mesh with no push and shove: it never moves other tracks.
* Nothing has been compared with the reference tool, and the speed on a large board has not been measured.
