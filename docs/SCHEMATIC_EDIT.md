# EH3D-PHOENIX-PCB: the schematic editor

**State: prototype.** Four parts make the editor: the operations and the nets that follow the drawing (`eh3d-phoenix-schematic`, module `edit`), the state of the editor without a screen (`eh3d-phoenix-schedit`), the command `phoenix schedit` and an editing mode of the window of the viewer (`docs/VIEWER.md`). It has been tried on schematics made by the tests only. The window has never been seen on a screen, and no project of any size has been drawn with it, so requirement PPCB-017 stays at `prototype`.

## The operations

An operation is one change that is asked for. It is a value (`EditOp`) that can be written in JSON, so the same change can come from a person in the window or from a file.

| Operation (`"op"`) | What it does |
|---|---|
| `add_sheet` | Adds an empty sheet of a name and a size. |
| `place_symbol` | Places a symbol as a new part. The symbol is named by its id or by its name; when the schematic has none of that name and a definition (`define`: name and pins) is given, the symbol is defined first. The reference is the next free one of the first letter of the symbol (`R1`, `R2`, ...) unless one is given; a given reference that is used is refused. |
| `move_items` | Moves parts, wires, labels, power ports and junction dots by an offset. The wires that end on a pin of a moved part follow the pin and stay horizontal and vertical (a bend point is added where a piece would become slanted), unless `detach` is true. |
| `rotate_parts` | Turns parts about their own origin by quarter turns. The wires that ended on their pins follow. |
| `mirror_parts` | Mirrors parts left to right. The wires follow. |
| `delete` | Deletes parts, wires, labels, power ports and junction dots. |
| `paste` | Pastes a clip (`Clip::copy` makes one) on a sheet, moved by an offset, as new items. The pasted parts get new references: the next free number after the letters they had. |
| `set_value`, `set_refdes` | Sets the value, or the reference (refused when another part has it). |
| `add_wire` | Draws a wire: a path of pieces that are horizontal or vertical. `orthogonal_path` makes the path between two points, with the corner first along x or first along y. |
| `add_label`, `add_power_port` | Puts a net label or a power port with a name (not empty, at most 128 characters, no control characters). |
| `add_bus` | Puts the labels of a bus such as `D[0..7]`, one per member, the first at a point and each next one an offset further (2.54 mm down when both offsets are zero). A bus is a way of naming: its members are ordinary nets. |
| `rename_mark` | Renames a net label or a power port. |
| `batch` | Several operations that are one step. |

## All or nothing, undo and redo

The `Editor` applies an operation on a copy of the schematic. The copy is tidied (below), the nets are made again, and only then does the schematic change; an operation that is refused (code `PHOENIX-SCH-010`: a part, sheet, symbol or item that does not exist, a reference that is used, a name that is empty or too long, a wire that is not orthogonal, a bus that is not of the form `D[0..7]`, a coordinate out of range) changes nothing, not the history and not the supply of ids. A batch is refused as a whole when one of its operations is.

Each step is kept as the differences between the maps of the schematic (symbols, parts, nets, sheets, wires, pin types, labels, junctions, power ports and the set of mirrored parts): for every id, what it was and what it became. Undo puts back exactly what was there, redo exactly what was made; a new operation after an undo drops the steps that could be redone. The history keeps at most `history_limit` steps.

New ids come from a supply that knows every id the schematic already uses and gives ids in a fixed order, so the same schematic and the same operations always give the same bytes.

## The drawing is tidied after an operation that touches the wiring

* A wire that ends in the middle of another wire's piece, and a wire point, a pin, a net label or a power port that lies in the middle of a piece, give that wire a point there. The two are then joined: a T.
* The junction dots are exactly at the points where three or more pieces and pins meet (an end counts one, a point in the middle of a wire two, a pin one). A dot where fewer meet is removed.

## The nets follow the drawing

After an operation that touches the wiring the net list is made again from what the drawing joins:

* pins that wires join, or that carry the same label or power port name, are on one net;
* a connection that only the net list gave (pins that no wire joined before the operation) is kept;
* a net keeps its identity: a group of pins takes the old net that holds most of its pins (the smallest id on a tie), and a group that no old net claims gets a new id;
* the name of a net is the smallest label or power port name on it; without one it keeps the name of its old net, or gets `N$1`, `N$2`, ...;
* a lone pin has no net, except a pin that was alone on its net already, or that carries a name.

So deleting the wire that made a connection removes the connection, and a label put on a pin names the net.

## The state without a screen

`eh3d-phoenix-schedit` holds what the window shows and does:

* **Tools:** select, wire, net label, power port, bus and place. The wire in progress goes on from its end until it ends on a pin, a junction, a label or another wire, or until the same point is clicked again; the corner goes first along x or along y (`wire_horizontal_first`, and the space key).
* **Grid and snapping:** a click lands on a pin or wire point within `pick_tolerance_nm`, otherwise on the grid when `snap` is on.
* **Selection:** by a click (shift adds or takes away) or by a rectangle (a part whose origin is inside, a wire all of whose points are inside, the labels and power ports inside).
* **Drag:** the selection follows the pointer on the grid; `drag_preview` gives the schematic as it would be if the drag were let go, wires included, and letting go makes one `move_items` step.
* **Palette:** the symbols of the schematic and of the libraries that were loaded (a JSON list of symbol definitions, or any schematic file), with a search that ranks an exact name, then a name that starts with the words, then a name that contains them, then a library that does. A symbol with the name of another one but other pins is defined as `library:name`.
* **Copy, cut, paste, turn, mirror, delete, values and references** of the selection.
* **The rule check** (`docs/ERC.md`) after every change, when `live_erc` is on.
* **The annotation back and the comparison** (`docs/ANNOTATE.md`) as data: the plan of what the board would change in the schematic, each change with its state, and the differences and conflicts of the two documents (`plan_reverse_annotation`, `compare_documents`); the window does not show them yet.
* **The transfer to the board** as a dry run with a preview of the package (`docs/ERC.md`, synchronisation), applied as one step of the history of the board, and **the net list for the simulator** (`docs/NETLIST.md`: made from a table of models, with the operating point net by net as data).

### Settings

`config/schedit.cfg` is the file of the editor, and the built-in default. The settings file of the viewer is not used for these values because it refuses keys it does not know.

| Key | Default | Meaning |
|---|---|---|
| `grid_nm`, `snap` | 2540000, true | The grid and whether points snap to it. |
| `pick_tolerance_nm`, `part_padding_nm` | 1270000, 1270000 | How near a click picks or joins, and the space around the pins that picks a part. |
| `paste_offset_nm`, `bus_step_nm` | 2540000, 2540000 | The offset of a paste without a target and the distance between the labels of a bus. |
| `history_limit` | 1000 | Steps kept for undo. |
| `live_erc` | true | The rule check after each change. |
| `wire_horizontal_first` | true | Which way a wire turns its corner. |
| `palette_results` | 50 | How many symbols a search lists. |
| `new_sheet_name`, `new_sheet_width_nm`, `new_sheet_height_nm` | `Sheet 1`, 297 mm, 210 mm | The sheet added to a schematic that has none. |

A line that is not `key = value`, an unknown key or a value out of range is `PHOENIX-SCHEDIT-001`.

## The command

```text
phoenix schedit [--erc-config FILE] [--json] [--lang xx] --apply OPERATIONS.json [--out FILE] <schematic>
```

The schematic is an EAGLE `.sch` file or the JSON of the model. The operations are a JSON list, or an object with the list under `ops`:

```json
[
  {"op": "set_value", "part": "01234567-89ab-7cde-8fed-cba987654321", "value": "4k7"},
  {"op": "add_label", "sheet": "01234567-89ab-7cde-8fed-cba987654322",
   "at": {"x_nm": 22540000, "y_nm": 20000000}, "name": "SIG"}
]
```

* It is a dry run: the operations are applied in memory, the nets are made again, the rule check is run on the result and the counts and the findings are printed. Nothing is written unless `--out` names a **new** file (`.json` for the model, `.sch` for EAGLE; the library of an EAGLE file written here has no footprints because there is no board). An input file, the file of operations and a file that exists are never written over (`PHOENIX-SCHEDIT-003`).
* All or nothing: the first operation that is refused ends the run, names its number and writes nothing (`PHOENIX-SCHEDIT-002`).
* Exit codes: 0 done and the rule check proves no error, 1 done and it proves an error, 2 the arguments, the input, an operation or the output are unusable.

## The window

In a schematic, the button *Edit schematic* of the left panel starts the editor on the schematic that is open. The panel then shows the tools, the selection actions, the value and the reference of a selected part, the palette with its search and the loading of a library, the grid, snapping and the rule check, and the sheet; the right panel keeps the findings of the rule check and the transfer to the board. The mouse draws: the left button works the tool, the right or middle button pans, the wheel zooms about the pointer. The keys are V select, W wire, L label, P power port, B bus, R turn (shift: back), X mirror, Delete, Space the corner of the wire, Escape stops the wire or the drag, Ctrl+C, Ctrl+X, Ctrl+V and Ctrl+Z, Ctrl+Y.

After every change the window gives the workspace the schematic as it is (`Workspace::replace_schematic`): the findings, the transfer to the board and *Save as* work on what was edited, and the file that was read is never written over. The steps of the editor are its own; the undo of the top bar is switched off while the editor is on. The window reads `config/schedit.cfg` from the folder it is started in when the file is there and good. The texts are in the eight languages of the suite (`schematic_text`).

## Codes

`PHOENIX-SCH-010` (edit refused) and `PHOENIX-SCHEDIT-001` to `PHOENIX-SCHEDIT-003` are in `docs/ERROR_CODES.md`.

## What is not done

The list is in `NOT_YET.md`. In short: a part is drawn as the box around its pins (the model holds no drawing of a symbol), a bus is only a list of names, only the points of wires that end on a moved pin follow it, a piece of a wire cannot be cut or joined, references are not renumbered, sheets are not added or removed in the window, a footprint cannot be chosen in the editor, and the window has not been seen on a screen.
