# EH3D-PHOENIX-PCB: the autorouter that needs no outside program

**State: prototype.** The crate `eh3d-phoenix-autoroute`, the command `phoenix autoroute native` and the button of the board mode of the window route the open connections of a board with a search of our own. It has been tried on small boards made by hand and by a generator of the tests with a fixed seed. It has not been tried on a board of a real design, its speed on a large board has not been measured, and the button of the window has never been seen on a screen or used with a mouse. Nothing here has been compared with another autorouter or with the routes of a person.

The exchange with an external program (`docs/AUTOROUTE.md`) stays as it was; this is a second, separate way to route, and it starts no program.

## What it does

1. **The connections.** They are the lines of the ratsnest of the board (`eh3d-phoenix-route`): for each net a minimum spanning tree over its islands. Tracks, vias and pads that are on the board are never changed: the plan only adds.
2. **The grid.** The nodes are at a fixed pitch (`grid_nm`), at multiples of the pitch from the origin, over the box of the outline. The width of a track, the size of a via and the clearance come from the rules, the net classes and `config/route.cfg`, as the interactive router takes them; the clearance is the largest of the rule, the class and the contact tolerance plus one nanometre, plus the margin of the router settings. A board that would need more nodes than `max_nodes` is refused.
3. **The obstacles.** For each copper layer: the pads (a round pad as a circle, a rectangular or oblong pad turned by a right angle as its rectangle, any other angle as the circle around it), the vias on the layers they span, the tracks (an arc as the disc around it), the zones of other nets (`zones_block`) and the edge of the board, kept away by the larger of the edge clearance and the clearance. The drill holes are a list of their own for the spacing of vias. The copper that was on the board is in a spatial index (`eh3d-phoenix-spatial`); the copper that the run puts down is in a map of squares that can take a route away again. Copper of the same net is not an obstacle. A via keeps away from the pads of its own net too unless `via_in_pad = true`.
4. **The search.** A* over the nodes, layer by layer, with a via as a move between two layers of the stack. A move costs its length in nanometres; a corner costs `corner_penalty_nm`, a via `via_penalty_nm` (nothing at a via of the same net that is already on the board and sits exactly on a node of the grid and joins both layers: the path changes layer there and puts no new via; a via off the grid is not used), a step against the preferred direction of its layer (the first layer prefers horizontal, the second vertical, and so on) `wrong_way_percent` more, and a node of a route that was torn up `history_penalty_nm` for each time. Tracks run at 0, 45 and 90 degrees (`diagonal`), and turns sharper than 90 degrees are refused. The start and the end of a track are the free nodes nearest to the centre of a pad (`terminal_candidates` of them on each layer of the pad), reached by one piece along an axis and one at 45 degrees. Every distance is compared exactly in integers, with 128 bits in the products; the order of expansion is fixed by (estimate, cost so far, state), so the same board, rules and settings give the same plan byte for byte.
5. **The order.** `order = shortest` routes the nets whose connections are shortest in total first, `widest` the widest tracks first, `critical` the nets named in `critical_nets` first (then the shortest).
6. **Rip-up and reroute.** A connection that finds no way is searched again through the copper that the run put down: the nets it crosses are torn up, the connection is routed, and the torn nets go back to the queue. A node of a torn route costs more in the next search. It stops at `max_ripup_iterations` in all and `max_rips_per_connection` for one connection.
7. **The limits** count nodes and passes and never time: `max_expansions_per_search`, `max_expansions_total`, `max_ripup_iterations`, `max_rips_per_connection`, `max_nodes`, `max_connections`.

## The plan and the rules

The result is a plan: the tracks and vias to add, as one operation of the board (`Operation::Batch`, with an exact inverse, one step of the history) and as one group of operations of the board editor (`AutoroutePlan::edit_ops`), which the editor keeps or undoes as one step. Before it is handed out it is applied to a copy of the board and the design rules are checked before and after. If all the routes together add a rule error, the nets are added one by one, in the order of the run, and a net whose routes add an error is left out and reported with `PHOENIX-AROUTE-015`. The plan never adds an error that the board did not have.

The report is the same for the same input: the nets and connections routed, one line for each connection left unrouted with its reason, the tracks, vias and length added, the rip-ups, the expanded nodes, the errors before and after and the connections still open, the pairs that were coupled (`coupled pair: NAME`) and a note for each pair that was routed as two ordinary nets with the code `PHOENIX-AROUTE-017` and the reason (`note: ...`; in the JSON, `pairs_coupled` and `notes`).

| Reason of an unrouted connection | Code |
|---|---|
| No free place near a pad to start or end a track | `PHOENIX-AROUTE-011` |
| No path exists on the grid | `PHOENIX-AROUTE-012` |
| The search reached its limit of expanded nodes | `PHOENIX-AROUTE-013` |
| Rip-up reached its limit | `PHOENIX-AROUTE-014` |
| The routes of the net add a rule error | `PHOENIX-AROUTE-015` |

## The command

```
phoenix autoroute native [--nets N1,N2] [--layers L1,L2] [--config FILE] [--settings FILE]
                         [--rules FILE] [--json] [--lang CODE] [--require-complete]
                         [--out FILE] <board>
```

A dry run: without `--out` only the report is printed. With `--out` the board with the routes is written to a file that must not exist; the input is never overwritten. `--nets` and `--layers` name nets and copper layers; the default is every net with an open connection and every copper layer. `--pairs FILE` (a JSON file of differential pairs, format `phoenix-diffpairs/1`) and `--pair-config FILE` (the pairs of a settings file such as `config/diffpairs.cfg`) ask for coupled routing of each pair (see below). `--require-complete` makes a plan that leaves a connection unrouted an error (`PHOENIX-AROUTE-016`, exit code 2). Exit code 0 is done and 2 is any refusal. The settings are `config/autoroute_native.cfg` (strict: an unknown key, a key twice or a value out of range is `PHOENIX-AROUTE-001`) and the width of the tracks and size of the vias are those of `config/route.cfg` and the rules.

## The window

The board mode has a section *Autorouter* in its panel. The button *Autoroute* (the key `A`) makes the plan for the board as it is and draws it as a preview, with the number of connections routed, tracks, vias and connections left open; the board is not changed. *Apply routes* (`Shift+A`) adds it as one step of the editor, checked by the rules, and it can be undone with Ctrl+Z; *Discard preview* (`Esc`) drops it. A preview is offered only while the board is as it was when it was made. The texts are in the eight languages of the suite, European Portuguese included. The state (`board_autoroute`) is tested without a screen; the drawing (`board_autoroute_gui`) has never been seen on a screen.

## Differential pairs

Without `--pairs` or `--pair-config` the nets of a pair are routed as two ordinary nets, and the coupling, the gap and the symmetry of a pair are judged after the run by the pair check of the board editor (`docs/DIFFPAIRS.md`).

A pair that is asked for (`autoroute_with_pairs` in the crate, `--pairs` and `--pair-config` in the command) is searched first, before the other connections, as one fat track: the width of the two tracks plus the gap between them, on one layer, with horizontal and vertical steps only. Its path is the centre line; the two tracks are that line moved sideways by half the distance between them, one to each side, with the corners where the moved lines cross, so both tracks have the same length. Each track leaves its pad by a stub of two pieces (tried in both orders), the fat track starts and ends at grid nodes away from the four pads, and every piece of both tracks is checked against the copper of the board and of the run, with the width and the clearance of its own net, before anything is kept. The width of the tracks is the width of the nets, which must be the same for both, and the gap is the `gap_nm` of the pair. A pair is coupled only when each net has exactly one open connection and the four pads share a layer; otherwise, or when there is no room (the nets change sides between their ends, a stub or a track is blocked, the fat track finds no way), both nets are routed as ordinary nets and the report has a note with the code `PHOENIX-AROUTE-017` and the reason. The plan stays one reversible step checked by the rules: a net whose routes add a rule error is left out as for any other net. A pair never changes layer (it would need two vias that do not touch), so a pair that needs a via is routed as two ordinary nets.

## What is not here

See `docs/NOT_YET.md`: pairs routed as coupled bodies on more than one layer, with a via, or on slanted pieces; vias between inner layers with the stack of vias checked; pouring zones around the new tracks; arcs and teardrops; an existing via of the net that is not on a node of the grid as a layer change; a comparison with another autorouter or with real boards; and the speed on a large board.
