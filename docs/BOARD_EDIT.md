# EH3D-PHOENIX-PCB: the board editor

**State: prototype.** Four parts make the editor: the operations, the rule check after each one and the state of the tools, all without a screen (`eh3d-phoenix-boardedit`), the command `phoenix boardedit` and a board editing mode of the window of the viewer (`docs/VIEWER.md`). It has been tried on boards made by hand and by the tests only. The window has never been seen on a screen or used with a mouse, and no real board has been routed with it, so requirement PPCB-015 stays at `prototype`. It does not route by itself: a track is made of the points it is given (the routing through an external autorouter is `docs/AUTOROUTE.md`). A track can push the neighbour tracks aside and a corner can be rounded with a native arc (`docs/SHOVE.md`).

## The operations

An operation is one change that is asked for. It is a value (`EditOp`) that can be written in JSON, so the same change can come from a person in the window or from a file.

| Operation (`"op"`) | What it does |
|---|---|
| `place` | Places a new component of a footprint the board has, at a point, turned, on the top or the bottom side. The reference must not be used by another component. |
| `move`, `rotate`, `mirror` | Moves (and turns), turns or puts a component on the other side (mirrored about the vertical axis). The ends of the tracks that were on its pads follow them (`follow`, by default the setting `follow_tracks`), so a drag keeps the connections. |
| `lock`, `unlock` | Locks a component: moving, turning, mirroring and removing it is refused (`PHOENIX-BEDIT-006`). The locks belong to the editor and are not saved in the board. |
| `remove` | Removes a component with the nets of its pads. |
| `route` | Draws a track of a net on a copper layer through two or more points, with horizontal and vertical pieces and, with the shape `diagonal`, 45-degree pieces. Between two points that are not in line there is one corner; both orders of the corner are tried when `horizontal_first` is left out. The width is the one of the net class (never below the narrowest track of the rules) unless `width_nm` is given. |
| `via` | Adds a via of a net, sized by the rules and the net class, between the top and the bottom copper layer unless two layers are given. |
| `change_layer` | Splits a track at a vertex: the part after it goes to another copper layer and a via is put at the vertex. |
| `delete_segment`, `delete_track`, `delete_via`, `delete_net` | Deletes one piece of a track (the rest stays in one or two tracks), a track, a via, or every track and via of a net (its zones and the nets of its pads stay). |
| `route_between` | Draws the shortest route between two pads without a router: a straight piece when they are in line (or at 45 degrees with the diagonal shape), an L otherwise. A pad that has no net takes the net of the other; two pads on different nets are refused. |
| `paste_net` | Pastes a net: the pads get the net of that name (made when the board has none) and are joined in the order given, each with the shortest route to the next. A pad that is on another net is refused. |
| `route_shove` | Draws a track like `route` and pushes aside, in parallel, the tracks of other nets that its clearance would invade: only horizontal, vertical and 45-degree pieces, the pads, the vias and the edge never move, within the depth and the number of tracks of the settings, all or nothing and refused with a `PHOENIX-SHOVE-<NNN>` code when it cannot be done or would leave a rule error (`docs/SHOVE.md`). |
| `route_arc`, `round_corners` | Put an arc of a given radius, tangent to both pieces, at a vertex of a track, or round every corner of a track or of a net with the largest radius that fits (never below `arc_min_radius_nm`). The arc is the native arc piece of the model and is checked against the clearance of the rules by the editor (`PHOENIX-ARC-<NNN>`, `docs/SHOVE.md`). |
| `tune_length` | Adds square meanders to the longest straight pieces (horizontal, vertical or at 45 degrees, the meanders turned with the piece) of a net until its copper length (measured by `eh3d-phoenix-lengths`: tracks and vias) is the target, to within `length_tolerance_nm`. Both sides of the piece are tried in turn. A net already within the tolerance is left as it is; a net longer than the target, or with no piece long enough, is refused (`PHOENIX-BEDIT-008`). |
| `route_pair`, `tune_pair`, `fix_pair_gap` | The operations on a differential pair: the two tracks drawn side by side, the shorter net made as long as the other with meanders, and the pieces with a wrong gap moved to the gap of the pair. They are described in `docs/DIFFPAIRS.md`. |
| `zone_add`, `zone_edit`, `zone_remove`, `zone_fill`, `zone_priority`, `zone_cutout`, `paste_group` | Copper zones with a priority, cutouts and a thermal relief of their own, their fill, and the paste of a net or a group with an offset, a turn and a mirror. They are described in `docs/BOARD_ZONES.md`. |
| `select` | Changes the selection of the editor by steps (see *The selection of several objects*). It changes nothing on the board and is not a step of the history. |
| `move_selection`, `delete_selection`, `align_selection` | Move, delete or line up every selected object as one step (see below). |
| `move_items`, `delete_items` | The same on objects that the operation names, each with its own offset. |
| `group` | Several operations that are one step of the history and are refused as a whole when one is. |
| `forced` | An operation that is kept even if it adds a rule error. |

## The rule check after each operation

An operation is planned into the reversible operations of `eh3d-phoenix-board` (one batch), tried on a copy of the board and checked by the design rules (`docs/RULES.md`): the findings before and after are compared. An operation that the board refuses, or that adds a finding that is an error, is refused and changes nothing, not the board, not the history and not the ids (`PHOENIX-BEDIT-004`, `PHOENIX-BEDIT-005`), unless it is `forced` or `reject_new_errors` is off. The result of every operation carries a report: the rule errors before and after, and the findings added and removed. Some operations have alternatives (the other corner, the other side of the meanders); the first one that the board accepts and the rules pass is kept.

Undo and redo give back the same bytes. The ids of the parts the editor makes come from a supply that skips every id the board uses, so the same board and the same operations always give the same bytes.

## The selection of several objects

The selection is an ordered set of references to components, tracks, vias, zones and silkscreen items (`selection` in `eh3d-phoenix-boardedit`), kept by the editor next to the board. It is not part of the history, and what an operation or an undo removes from the board is dropped from it. An object is written `{"kind": "component", "id": "..."}` (`component`, `track`, `via`, `zone` or `silk`). The operation `select` takes a list of steps, applied in order and all or nothing:

| Step (`"select"`) | What it does |
|---|---|
| `add`, `remove`, `toggle` | The objects of `items`; `add` and `toggle` refuse an object the board does not have (`PHOENIX-BEDIT-010`). |
| `net` | Every object of a net (`net`): its tracks, vias and zones and the components with a pad on it. `action` is `add` (by default), `remove` or `toggle`. |
| `rect` | What the rectangle between `from` and `to` takes in. `mode` is `contained` (by default: the whole extent of the object is inside) or `crossed` (also what it touches; a track is judged by its centre line, the rest by its extent). `action` as above. |
| `all`, `invert`, `clear` | Every object of the board; what is not selected instead of what is; nothing. |

The extent of an object is the box of what it covers: the pads of a component with half of their largest side around each centre, the pieces of a track and of a silkscreen item with half of their width, a via with half of its diameter, the corners of a zone. An arc counts as its chord.

The operations on the selection are one step of the history each, with an exact inverse (undo gives the same bytes):

* `move_selection` (`dx_nm`, `dy_nm`, `snap`) moves every selected object by one offset. The offset is rounded to a multiple of the grid (halves upwards) when `snap` is true, or when it is left out and the setting `snap` is on. A component keeps its rotation; a track, a silkscreen item and a via are drawn again at the new place with the same id; a zone is moved by the zone editor with its cutouts and its fill is dropped (fill it again). The tracks that are not selected do not follow a component: select the net first to carry the copper along. Up is positive.
* `delete_selection` removes every selected object (a component with the nets of its pads; a zone with its properties and fill).
* `align_selection` (`mode`) lines the extents up with the box that holds them all: `left`, `right`, `top` (the largest y) and `bottom` (the smallest y) put the matching edge of each object on the matching edge of the group, `center_x` and `center_y` put the centres on the centre of the group to within a nanometre. `distribute_x` and `distribute_y` (three objects or more) leave the first and the last by centre where they are and spread the others so that the gaps between extents are equal, to within a nanometre.
* `move_items` (`moves`: a list of `{"item": ..., "dx_nm": ..., "dy_nm": ...}`) and `delete_items` (`items`) are the same operations on objects that are named, which the first three are made of.

A locked component refuses the whole operation (`PHOENIX-BEDIT-006`), an empty selection is `PHOENIX-BEDIT-009`, an object the board does not have is `PHOENIX-BEDIT-010`, and fewer than two objects to align, or fewer than three to distribute, is `PHOENIX-BEDIT-011`. An example of a file of operations:

```json
[
  {"op": "select", "steps": [{"select": "net", "net": "<net id>"}]},
  {"op": "move_selection", "dx_nm": 1000000, "dy_nm": 0, "snap": false},
  {"op": "align_selection", "mode": "left"},
  {"op": "delete_selection"}
]
```

The window does not offer the selection of several objects yet (see `docs/NOT_YET.md`): its own selection is the one of the tools, with components, tracks and vias only.

## The state without a screen

`eh3d-phoenix-boardedit` holds what the window shows and does:

* **Tools:** select, place, route, via, delete and tune. The route starts on a pad, a track or a via (the net comes from there, or from the lit net), goes on with a click for each corner and ends on a pad of the same net, on `finish_route`, or is dropped; the corner goes first along x or along y (`corner_horizontal_first`, and the space key), with 45-degree pieces when `diagonal_routes` is on.
* **Grid and snapping:** a point lands on the grid when `snap` is on.
* **Selection:** by a click (shift adds or takes away) or by a rectangle (what lies wholly inside); a via, then a track, then a component is picked, the nearest first.
* **Drag:** the selected components follow the pointer on the grid; `drag_preview` gives the board as it would be, and letting go makes one step in which the tracks on their pads follow.
* **Active layer, lit net and ratsnest:** tracks are drawn on the active copper layer; the lit net is the one of the last thing picked; the ratsnest is the list of connections that the nets of the pads call for and no copper joins yet, so the nets that a transfer from the schematic gave are drawn as lines to route.
* **Turn, mirror, lock and delete** of the selection, and the placing of a footprint with the reference and the value asked for.

### Settings

`config/boardedit.cfg` is the file of the editor, and the built-in default. The settings file of the viewer is not used for these values because it refuses keys it does not know.

| Key | Default | Meaning |
|---|---|---|
| `grid_nm`, `snap` | 250000, true | The grid and whether points snap to it. |
| `pick_tolerance_nm`, `part_padding_nm` | 300000, 500000 | How near a click picks a track or a via, and the space around the pads that picks a component. |
| `live_drc`, `reject_new_errors` | true, true | The rule check after each operation, and the refusal of an operation that adds a rule error. |
| `advanced_drc` | false | Also run the advanced checks (`docs/DRC_ADVANCED.md`) in that rule check, so that the findings they add or remove are in the report of an operation. |
| `follow_tracks` | true | The tracks on the pads of a component follow it. |
| `diagonal_routes`, `corner_horizontal_first` | true, true | 45-degree pieces, and which way a corner goes. |
| `track_width_nm`, `via_drill_nm`, `via_diameter_nm` | 250000, 300000, 600000 | The width of a track and the size of a via when the net class asks for no other; the rules and the classes raise them. |
| `meander_amplitude_nm`, `meander_max_amplitude_nm`, `meander_pitch_nm` | 1000000, 3000000, 400000 | The amplitude the meanders start with, the largest amplitude and the run of one half period. |
| `length_tolerance_nm`, `meander_candidates` | 100000, 3 | How far from the target the length may end, and how many of the longest straight pieces are tried. |
| `shove_max_depth`, `shove_max_moved`, `shove_margin_nm` | 3, 8, 1000 | How many levels a push may go, the most tracks it may move and the gap added to the clearance of the rules (`docs/SHOVE.md`). |
| `arc_min_radius_nm`, `arc_steps_per_quarter`, `arc_margin_nm` | 500000, 8, 1000 | The smallest radius of an arc, the chords of a quarter turn when an arc is checked and the gap added to the clearance of the rules. |

A line that is not `key = value`, an unknown key or a value out of range is `PHOENIX-BEDIT-001`.

## The command

```text
phoenix boardedit [--rules FILE] [--settings FILE] [--pair-config FILE] [--pairs FILE] [--zones FILE] [--zones-config FILE] [--zones-out FILE] [--poured-out FILE] [--json] [--lang xx] --apply OPERATIONS.json [--out FILE] <board>
```

The board is an EAGLE `.brd` file or the JSON of the model. The operations are a JSON list, or an object with the list under `ops`:

```json
[
  {"op": "route", "net": "01234567-89ab-7cde-8fed-cba987654321", "layer": "01234567-89ab-7cde-8fed-cba987654322",
   "points": [{"x_nm": 5000000, "y_nm": 14000000}, {"x_nm": 15000000, "y_nm": 20000000}], "shape": "diagonal"},
  {"op": "forced", "with": {"op": "via", "net": "01234567-89ab-7cde-8fed-cba987654321",
   "at": {"x_nm": 20000000, "y_nm": 20000000}}}
]
```

* It is a dry run: the operations are applied in memory, the design rules are checked after each one and the counts and the findings are printed. Nothing is written unless `--out` names a **new** file (`.json` for the model, `.brd` for EAGLE). An input file (the board, the file of operations, the rules and the settings) and a file that exists are never written over (`PHOENIX-BEDIT-003`).
* All or nothing: the first operation that is refused ends the run, names its number and writes nothing (`PHOENIX-BEDIT-004` to `PHOENIX-BEDIT-011`; a file that cannot be read is `PHOENIX-BEDIT-002`).
* `--pair-config` and `--pairs` give the settings and the pairs for the pair operations (`docs/DIFFPAIRS.md`): the settings file (default `config/diffpairs.cfg` when it exists) and a JSON file of pairs.
* `--rules` is the JSON rule file (`docs/RULES.md`); without it an EAGLE board gives its own design rules and any other board is checked only for shorts.
* Exit codes: 0 done and the rules prove no error on the result, 1 done and they prove an error (one that was there before, or one that a forced operation added), 2 the arguments, the input, an operation or the output are unusable.

## The window

In a board, the button *Edit board* of the left panel starts the editor on the board that is open. The panel then shows the tools, the active layer, what the tool needs (the footprint, the reference and the value; the corner and the 45-degree pieces; the target length), the selection actions (turn, other side, lock, delete), the count of rule errors and of connections still to route, the length of the lit net, the grid, snapping and the rule check; the right panel keeps the findings of the rules. The mouse draws: the left button works the tool, the right or middle button pans, the wheel zooms about the pointer. The keys are V select, P place, T route, I via, D delete, L tune, R turn (shift: back), X other side, K lock, Delete, Enter ends the route, Space the corner, Escape drops the route or the drag, Ctrl+Z and Ctrl+Y.

After every change the window gives the workspace the board as it is (`Workspace::replace_board`): the findings, the ratsnest and *Save as* work on what was edited, and the file that was read is never written over. The steps of the editor are its own; the undo of the top bar is switched off while the editor is on. The window reads `config/boardedit.cfg` from the folder it is started in when the file is there and good. A box *Pair tool* adds the section of the differential pairs (`docs/DIFFPAIRS.md`), and a box *Zone tool* the one of the copper zones (`docs/BOARD_ZONES.md`). The texts are in the eight languages of the suite (`board_text`, `board_pair_text`).

## Codes

`PHOENIX-BEDIT-001` to `PHOENIX-BEDIT-011` are in `docs/ERROR_CODES.md`, with their titles in the eight languages in `eh3d-phoenix-boardedit` (`src/codes.rs`). The codes `PHOENIX-PAIR-001` to `PHOENIX-PAIR-011` of the differential pairs are there too (`src/pair_codes.rs`), and so are `PHOENIX-SHOVE-001` to `006` and `PHOENIX-ARC-001` to `003` of the push and the arcs (`src/shove_codes.rs`) and `PHOENIX-BZONE-001` to `011` of the zones (`src/zone_codes.rs`).

## What is not done

The list is in `NOT_YET.md`. In short: no automatic routing, only a basic push and shove (`docs/SHOVE.md`: straight pieces at 0, 45 and 90 degrees, never a pad, a via or a component), arcs only at corners, meanders only on straight pieces, differential pairs only as far as `docs/DIFFPAIRS.md` says, copper zones only as far as `docs/BOARD_ZONES.md` says and no silkscreen in the editor, no check of the pieces of a drag while it is drawn (the rules run when it is let go), locks that are not saved, and a window that has not been seen on a screen or used with a mouse.
