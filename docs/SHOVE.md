# EH3D-PHOENIX-PCB: push and shove, and arcs in tracks

**State: prototype.** Three operations of the board editor (`docs/BOARD_EDIT.md`, crate `eh3d-phoenix-boardedit`, command `phoenix boardedit`) and a tool of the board mode of the window (`docs/VIEWER.md`) make tracks that give way to each other and tracks with arcs. They have been tried on boards made by hand and by the tests only. The tool of the window has never been seen on a screen or used with a mouse, and no real board has been routed with them.

## `route_shove`: a track that pushes its neighbours

`route_shove` draws a track like `route` (the same fields: `net`, `layer`, `points`, and optional `shape`, `width_nm` and `horizontal_first`) and, where its clearance would invade the clearance of the tracks of **other** nets on the layer, pushes those tracks aside, in parallel, until the rules are kept again.

```json
{"op": "route_shove", "net": "01234567-89ab-7cde-8fed-cba987654321", "layer": "01234567-89ab-7cde-8fed-cba987654322",
 "points": [{"x_nm": 15000000, "y_nm": 10300000}, {"x_nm": 25000000, "y_nm": 10300000}], "shape": "orthogonal"}
```

How the push works:

* **Which pieces are pushed.** Only straight pieces that are horizontal, vertical or at 45 degrees. A piece moves along its normal (a 45-degree piece moves on both axes, so that it stays at 45 degrees) by the least whole number of nanometres that restores the gap: the clearance of the rules, plus `shove_margin_nm`, plus half the width of each track.
* **What keeps its place.** The pads, the vias and the edge of the board are never moved: a piece that would have to be pushed against them stops the whole push (`PHOENIX-SHOVE-001`). An end of a pushed track, or a vertex, that lies on a pad, a via or another track of the same net also stays; a corner is added to join it to the pushed piece. An end that is joined to nothing moves with its piece. A piece with a via, a pad or the end of another track of its net along its length is not pushed (`PHOENIX-SHOVE-004`), because moving it would cut that connection.
* **The cascade.** A pushed piece can crowd the next track, which is pushed in turn. The depth of the cascade (`shove_max_depth`) and the number of tracks that move (`shove_max_moved`) are settings in `config/boardedit.cfg`; going past either is refused (`PHOENIX-SHOVE-002`, `PHOENIX-SHOVE-003`).
* **What cannot be pushed.** An arc, or a piece at any other angle (`PHOENIX-SHOVE-004`). A route that crosses or touches a track of another net cannot be solved by pushing in parallel (`PHOENIX-SHOVE-005`).
* **All or nothing.** The new track and the moved tracks are one batch of reversible operations, one step of the history: undo gives back the same bytes. The result is tried on a copy and checked by the design rules (`docs/RULES.md`) before and after; a result that adds a rule error is refused with `PHOENIX-SHOVE-006`, **also** when `reject_new_errors` is off and also for a `forced` operation. When no neighbour is in the way the operation is a plain `route`.
* **The report.** The notes of the result name every track that moved, how many pieces were pushed and the largest shift in nanometres (`shove: track <id> moved, <n> piece(s) pushed, largest shift <nm> nm`), and the usual report of the rule errors before and after follows.

The planning uses floating point and takes a pad as a disc of its larger half side; every point that is kept is a whole number of nanometres and the rules judge the result exactly, so a mistake of the planning can refuse a push but cannot let an error through.

## Arcs in tracks

The model has a native arc piece (end points and a signed sweep), and the editor writes it. Two operations:

| Operation (`"op"`) | What it does |
|---|---|
| `route_arc` | Puts an arc of the given `radius_nm`, tangent to both pieces, at the vertex `vertex` of a track (counted as for `change_layer`: the vertex 1 is the end of the first piece). The two pieces are shortened to the points of tangency. Both pieces must be straight and must have the room the radius needs (`tan(turn / 2) * radius` of each). |
| `round_corners` | Rounds every corner between two straight pieces of one `track`, or of every track of a `net`, with the radius given or with the largest one that fits when the room is smaller; two corners that share a piece share it in halves. A corner that has no room for `arc_min_radius_nm` stays as it is, and the notes say so. It is refused when no corner could be rounded. |

The radius is at least `arc_min_radius_nm` (`PHOENIX-ARC-001`); a vertex that is not between two straight pieces, pieces on one line, a turn back, or pieces too short are `PHOENIX-ARC-002`.

The design rules see an arc by its end points only (`docs/RULES.md`), so the editor checks the arc itself before it keeps it: the arc is cut into chords (`arc_steps_per_quarter` for each quarter turn), each chord widened by its sagitta, and kept the clearance of the rules plus `arc_margin_nm` away from the copper of other nets, the pads, the vias and, with the clearance to the edge, the board outline (`PHOENIX-ARC-003`). The length of a net counts the arc (`docs/LENGTHS.md`).

## Meanders on pieces at 45 degrees

`tune_length` and `tune_pair` now put their square meanders on horizontal, vertical **and 45-degree** pieces. On a 45-degree piece the bumps are turned with the piece: every piece of the serpentine is at 45 degrees, the amplitude and the run are measured across and along the piece, and the length it adds is within a few nanometres of the length asked for.

## Settings

All in `config/boardedit.cfg` (the file of the board editor); the file refuses a key it does not know.

| Key | Default | Meaning |
|---|---|---|
| `shove_max_depth` | 3 | How many levels a push may go (1 to 16). |
| `shove_max_moved` | 8 | The most tracks a push may move (1 to 256). |
| `shove_margin_nm` | 1000 | The gap added to the clearance of the rules when a track is pushed. |
| `arc_min_radius_nm` | 500000 | The smallest radius of an arc. |
| `arc_steps_per_quarter` | 8 | The chords for each quarter turn when an arc is checked (1 to 64). |
| `arc_margin_nm` | 1000 | The gap added to the clearance of the rules when an arc is checked. |

## The command

`phoenix boardedit --apply OPERATIONS.json [--out NEW] <board>` accepts the three operations like any other (`docs/BOARD_EDIT.md`): a dry run unless `--out` names a new file, nothing is written over an input or over a file that exists, and the first operation that is refused ends the run, names its number and writes nothing. The notes of the push and of the arcs are in the report (`--json` gives them in the field `notes` of `result`).

## The window

The box *Push tool* of the panel of the board mode turns a click into a point of the new track and adds a list of the nets. Every click makes the preview again, without changing the board: the tracks that would move are drawn in their place now (thin) and where the push would put them (thick), with the number of tracks that would move, or the reason why the push cannot be done. *Route and push* accepts it as one step of the history; the points can be removed one by one or all together. The texts are in the eight languages of the suite (European Portuguese included). The state (`board_shove`) is tested without a screen; the drawing (`board_shove_gui`) has never been seen on a screen or used with a mouse.

## Codes

`PHOENIX-SHOVE-001` to `006` and `PHOENIX-ARC-001` to `003` are in `docs/ERROR_CODES.md`, with their titles in the eight languages in `eh3d-phoenix-boardedit` (`src/shove_codes.rs`).

## What is not done

The list is in `NOT_YET.md`. In short: vias, pads and components are never pushed; a piece that is not horizontal, vertical or at 45 degrees and an arc are never pushed; there is no walk-around of an obstacle (a push either works or is refused); no pair is pushed as a pair; the router and the autorouter never push; the arcs are drawn at corners only (no arc from point to point, no arc in a meander); the pad shape of the planning is a disc; nothing has been compared with another tool or with a board that was built; and the window has never been seen on a screen or used with a mouse.
