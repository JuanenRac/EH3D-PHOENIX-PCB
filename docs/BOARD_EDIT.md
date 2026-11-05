# EH3D-PHOENIX-PCB: the board editor

**State: prototype.** Four parts make the editor: the operations, the rule check after each one and the state of the tools, all without a screen (`eh3d-phoenix-boardedit`), the command `phoenix boardedit` and a board editing mode of the window of the viewer (`docs/VIEWER.md`). It has been tried on boards made by hand and by the tests only. The window has never been seen on a screen or used with a mouse, and no real board has been routed with it, so requirement PPCB-015 stays at `prototype`. It does not route by itself: a track is made of the points it is given (the routing through an external autorouter is `docs/AUTOROUTE.md`).

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
| `tune_length` | Adds square meanders to the longest straight pieces of a net until its copper length (measured by `eh3d-phoenix-lengths`: tracks and vias) is the target, to within `length_tolerance_nm`. Both sides of the piece are tried in turn. A net already within the tolerance is left as it is; a net longer than the target, or with no piece long enough, is refused (`PHOENIX-BEDIT-008`). |
| `group` | Several operations that are one step of the history and are refused as a whole when one is. |
| `forced` | An operation that is kept even if it adds a rule error. |

## The rule check after each operation

An operation is planned into the reversible operations of `eh3d-phoenix-board` (one batch), tried on a copy of the board and checked by the design rules (`docs/RULES.md`): the findings before and after are compared. An operation that the board refuses, or that adds a finding that is an error, is refused and changes nothing, not the board, not the history and not the ids (`PHOENIX-BEDIT-004`, `PHOENIX-BEDIT-005`), unless it is `forced` or `reject_new_errors` is off. The result of every operation carries a report: the rule errors before and after, and the findings added and removed. Some operations have alternatives (the other corner, the other side of the meanders); the first one that the board accepts and the rules pass is kept.

Undo and redo give back the same bytes. The ids of the parts the editor makes come from a supply that skips every id the board uses, so the same board and the same operations always give the same bytes.

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
| `follow_tracks` | true | The tracks on the pads of a component follow it. |
| `diagonal_routes`, `corner_horizontal_first` | true, true | 45-degree pieces, and which way a corner goes. |
| `track_width_nm`, `via_drill_nm`, `via_diameter_nm` | 250000, 300000, 600000 | The width of a track and the size of a via when the net class asks for no other; the rules and the classes raise them. |
| `meander_amplitude_nm`, `meander_max_amplitude_nm`, `meander_pitch_nm` | 1000000, 3000000, 400000 | The amplitude the meanders start with, the largest amplitude and the run of one half period. |
| `length_tolerance_nm`, `meander_candidates` | 100000, 3 | How far from the target the length may end, and how many of the longest straight pieces are tried. |

A line that is not `key = value`, an unknown key or a value out of range is `PHOENIX-BEDIT-001`.

## The command

```text
phoenix boardedit [--rules FILE] [--settings FILE] [--json] [--lang xx] --apply OPERATIONS.json [--out FILE] <board>
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
* All or nothing: the first operation that is refused ends the run, names its number and writes nothing (`PHOENIX-BEDIT-004` to `PHOENIX-BEDIT-008`; a file that cannot be read is `PHOENIX-BEDIT-002`).
* `--rules` is the JSON rule file (`docs/RULES.md`); without it an EAGLE board gives its own design rules and any other board is checked only for shorts.
* Exit codes: 0 done and the rules prove no error on the result, 1 done and they prove an error (one that was there before, or one that a forced operation added), 2 the arguments, the input, an operation or the output are unusable.

## The window

In a board, the button *Edit board* of the left panel starts the editor on the board that is open. The panel then shows the tools, the active layer, what the tool needs (the footprint, the reference and the value; the corner and the 45-degree pieces; the target length), the selection actions (turn, other side, lock, delete), the count of rule errors and of connections still to route, the length of the lit net, the grid, snapping and the rule check; the right panel keeps the findings of the rules. The mouse draws: the left button works the tool, the right or middle button pans, the wheel zooms about the pointer. The keys are V select, P place, T route, I via, D delete, L tune, R turn (shift: back), X other side, K lock, Delete, Enter ends the route, Space the corner, Escape drops the route or the drag, Ctrl+Z and Ctrl+Y.

After every change the window gives the workspace the board as it is (`Workspace::replace_board`): the findings, the ratsnest and *Save as* work on what was edited, and the file that was read is never written over. The steps of the editor are its own; the undo of the top bar is switched off while the editor is on. The window reads `config/boardedit.cfg` from the folder it is started in when the file is there and good. The texts are in the eight languages of the suite (`board_text`).

## Codes

`PHOENIX-BEDIT-001` to `PHOENIX-BEDIT-008` are in `docs/ERROR_CODES.md`, with their titles in the eight languages in `eh3d-phoenix-boardedit` (`src/codes.rs`).

## What is not done

The list is in `NOT_YET.md`. In short: no automatic routing, no push and shove (a track never moves another), no arcs in the tracks the editor draws, meanders only on horizontal or vertical pieces, no differential pairs, no copper zones or silkscreen in the editor, no check of the pieces of a drag while it is drawn (the rules run when it is let go), locks that are not saved, and a window that has not been seen on a screen or used with a mouse.
