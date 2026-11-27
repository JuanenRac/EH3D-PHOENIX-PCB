# EH3D-PHOENIX-PCB: push and shove, and arcs in tracks

**State: prototype.** Three operations of the board editor (`docs/BOARD_EDIT.md`, crate `eh3d-phoenix-boardedit`, command `phoenix boardedit`) and a tool of the board mode of the window (`docs/VIEWER.md`) make tracks that give way to each other and tracks with arcs. They have been tried on boards made by hand and by the tests only. The tool of the window has never been seen on a screen or used with a mouse, and no real board has been routed with them.

## `route_shove`: a track that pushes its neighbours

`route_shove` draws a track like `route` (the same fields: `net`, `layer`, `points`, and optional `shape`, `width_nm` and `horizontal_first`) and, where its clearance would invade the clearance of the copper of **other** nets, makes room for it.

```json
{"op": "route_shove", "net": "01234567-89ab-7cde-8fed-cba987654321", "layer": "01234567-89ab-7cde-8fed-cba987654322",
 "points": [{"x_nm": 15000000, "y_nm": 10300000}, {"x_nm": 25000000, "y_nm": 10300000}], "shape": "orthogonal"}
```

Two ways are tried, in this order, for each order of the corner. The first that passes the checks is kept.

### 1. Pushing in parallel

* **Which pieces are pushed.** Only straight pieces that are horizontal, vertical or at 45 degrees. A piece moves along its normal (a 45-degree piece moves on both axes, so that it stays at 45 degrees) by the least whole number of nanometres that restores the gap: the clearance, plus `shove_margin_nm`, plus half the width of each track.
* **What keeps its place.** Pads, vias and the edge of the board do not move in this way. An end of a pushed track, or a vertex, that lies on a pad, a via or another track of the same net also stays; a corner is added to join it to the pushed piece. An end that is joined to nothing moves with its piece.
* **The cascade.** A pushed piece can crowd the next track, which is pushed in turn.

### 2. Walking around obstacles and moving vias

It is used when the first way cannot do it (`shove_walk_around` and `shove_vias`).

* **The new track goes around** the pads, the arcs and the vias that cannot move of other nets, with horizontal, vertical and 45-degree pieces, from the same first point to the same last point.
* **A crowded track is walked around** the octagons of what has already found its place (the new track and what was pushed before it) and of the fixed copper of other nets. Obstacles that overlap are walked as one. The points where a via, a pad or the end of another track of its net join it along its length stay where they are. A track that is not at 0, 45 or 90 degrees can be walked too; a track with an arc never moves and is walked around.
* **A track does not jump over the new track:** it gives way on its own side. A new track that crosses a track of another net is refused.
* **A crowded via is moved** to the nearest place, in one of eight directions and no further than `shove_max_via_shift_nm`, that is clear on every layer it joins (copper, drill spacing and board edge). The tracks that end on it follow it and are walked in turn, on every layer. A via in a pad of its net, or in the middle of a track of its net, does not move.
* **The order.** The nearest levels are placed first; a track or a via is placed once and later pushes go around it, so the work always ends.
* **The ways round.** The shorter way is taken at every obstacle; when that is refused, one way round and then the other are tried. A track that the way asked for would take over the new track is walked each way round, and the shorter that does not cross is kept.

### What both ways keep

* **Limits.** The depth of the cascade (`shove_max_depth`) and the number of tracks and vias that move (`shove_max_moved`) are settings in `config/boardedit.cfg`; going past either is refused (`PHOENIX-SHOVE-002`, `PHOENIX-SHOVE-003`).
* **Net classes.** Between two nets the gap is the larger of the clearance of the rules and the clearance of their net classes, which the design rules check too (`PHOENIX-CHECK-016`): a result that places copper nearer than a class asks is refused.
* **All or nothing.** The new track, the moved tracks and the moved vias are one batch of reversible operations, one step of the history: undo gives back the same bytes.
* **The checks.** The result is tried on a copy. Every pair of pads, tracks, vias and zones that a net joined before must still be joined (`PHOENIX-SHOVE-007`), and the design rules (`docs/RULES.md`) must find no new error (`PHOENIX-SHOVE-006`), **also** when `reject_new_errors` is off and also for a `forced` operation. When no neighbour is in the way the operation is a plain `route`.
* **Only what changed is checked.** The rules judge the zone of the pushed tracks and vias (`local_checks`, `docs/BOARD_EDIT.md`) and the connectivity is worked out for their nets only; both give the same answer as the whole board.
* **The report.** The notes of the result name every track and via that moved and say when the new track goes around something.
* **What a refusal says** is the reason the parallel push gave.

The pads, vias and tracks near a place are looked up in grids (`eh3d-phoenix-spatial`), so the time of the walk grows little with the board; a test compares the result with the one of looking at every object. The planning takes a pad as its box (or the octagon around a round pad) when it walks and as a disc of its larger half side when it pushes in parallel; every point that is kept is a whole number of nanometres and the checks judge the result, so a mistake of the planning can refuse a push but cannot let an error through.

### Times and results on synthetic boards

The tests `crates/eh3d-phoenix-boardedit/tests/shove_bench.rs` (a long test, left out of a normal run) and `shove_props.rs` make their own boards. The second pushes on boards made with a fixed seed and checks, after every push, that no rule error was added, that no connection was cut, that the clearance of the classes holds and that undo gives the same bytes back. The times are in `docs/PERFORMANCE.md`. No real board has been used.

## Arcs in tracks

The model has a native arc piece (end points and a signed sweep), and the editor writes it. Two operations:

| Operation (`"op"`) | What it does |
|---|---|
| `route_arc` | Puts an arc of the given `radius_nm`, tangent to both pieces, at the vertex `vertex` of a track (counted as for `change_layer`: the vertex 1 is the end of the first piece). The two pieces are shortened to the points of tangency. Both pieces must be straight and must have the room the radius needs (`tan(turn / 2) * radius` of each). |
| `round_corners` | Rounds every corner between two straight pieces of one `track`, or of every track of a `net`, with the radius given or with the largest one that fits when the room is smaller; two corners that share a piece share it in halves. A corner that has no room for `arc_min_radius_nm` stays as it is, and the notes say so. It is refused when no corner could be rounded. |

The radius is at least `arc_min_radius_nm` (`PHOENIX-ARC-001`); a vertex that is not between two straight pieces, pieces on one line, a turn back, or pieces too short are `PHOENIX-ARC-002`.

The design rules measure an arc along its curve, as chords with the doubt of their sagitta, and the editor also checks the arc itself before it keeps it: the arc is cut into chords (`arc_steps_per_quarter` for each quarter turn), each chord widened by its sagitta, and kept the clearance of the rules plus `arc_margin_nm` away from the copper of other nets, the pads, the vias and, with the clearance to the edge, the board outline (`PHOENIX-ARC-003`). The length of a net counts the arc (`docs/LENGTHS.md`).

## Meanders on pieces at 45 degrees

`tune_length` and `tune_pair` now put their square meanders on horizontal, vertical **and 45-degree** pieces. On a 45-degree piece the bumps are turned with the piece: every piece of the serpentine is at 45 degrees, the amplitude and the run are measured across and along the piece, and the length it adds is within a few nanometres of the length asked for.

## Settings

All in `config/boardedit.cfg` (the file of the board editor); the file refuses a key it does not know.

| Key | Default | Meaning |
|---|---|---|
| `shove_max_depth` | 3 | How many levels a push may go (1 to 16). |
| `shove_max_moved` | 8 | The most tracks and vias a push may move (1 to 256). |
| `shove_margin_nm` | 1000 | The gap added to the clearance of the rules when a track is pushed. |
| `shove_vias` | true | Whether a push moves the vias in its way. |
| `shove_walk_around` | true | Whether a push walks the new track and the pushed tracks around obstacles. |
| `shove_max_via_shift_nm` | 2000000 | The furthest a push moves a via. |
| `arc_min_radius_nm` | 500000 | The smallest radius of an arc. |
| `arc_steps_per_quarter` | 8 | The chords for each quarter turn when an arc is checked (1 to 64). |
| `arc_margin_nm` | 1000 | The gap added to the clearance of the rules when an arc is checked. |

## The command

`phoenix boardedit --apply OPERATIONS.json [--out NEW] <board>` accepts the three operations like any other (`docs/BOARD_EDIT.md`): a dry run unless `--out` names a new file, nothing is written over an input or over a file that exists, and the first operation that is refused ends the run, names its number and writes nothing. The notes of the push and of the arcs are in the report (`--json` gives them in the field `notes` of `result`).

## The window

The box *Push tool* of the panel of the board mode turns a click into a point of the new track and adds a list of the nets. Every click makes the preview again, without changing the board: the tracks and vias that would move are drawn in their place now (thin) and where the push would put them (thick), with their number, or the reason why the push cannot be done. *Route and push* accepts it as one step of the history; the points can be removed one by one or all together. The texts are in the eight languages of the suite (European Portuguese included). The state (`board_shove`) is tested without a screen; the drawing (`board_shove_gui`) has never been seen on a screen or used with a mouse.

## Codes

`PHOENIX-SHOVE-001` to `007` and `PHOENIX-ARC-001` to `003` are in `docs/ERROR_CODES.md`, with their titles in the eight languages in `eh3d-phoenix-boardedit` (`src/shove_codes.rs`).

## What is not done

The list is in `NOT_YET.md`. In short: pads, components and arcs are never moved; the walk goes around the octagon of obstacles that overlap, which can be wider than needed; a via moves only in eight directions and a limited distance; the result is not made shorter afterwards; no pair is pushed as a pair; the router and the autorouter never push; the arcs are drawn at corners only (no arc from point to point, no arc in a meander); zones are not refilled by a push; nothing has been compared with another tool or with a board that was built; and the window has never been seen on a screen or used with a mouse.
