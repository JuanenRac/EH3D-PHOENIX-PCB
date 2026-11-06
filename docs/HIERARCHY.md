# Sheet symbols, repeated sheets and hierarchical names

`eh3d-phoenix-schematic` (modules `hier`, `hier_edit` and `hier_expand`) lets a sheet show another sheet, also the same sheet several times, and draws the hierarchy out for the checks that need a flat schematic. `phoenix hier` prints it. The reading of KiCad and EAGLE files keeps the structure they have.

**State: prototype.** It has been tried on schematics made by hand and by the tests only, never on a design from a real project, and the window that draws it has never been seen on a screen. The expansion has not been compared with the hierarchy of KiCad or of EAGLE. The settings are in `config/hierarchy.cfg`.

## The model

* A **sheet** is a page of the drawing, as before. A sheet that no sheet symbol shows is a **top sheet**.
* A **sheet symbol** (`SheetBlock`) is a block drawn on one sheet that shows another sheet. It has a name (`U1`, `FILT`), a corner and a size. The same sheet may be shown by several symbols with different names: each symbol is an **instance** of the sheet, and the parts of the sheet exist once in the file and once for each instance in the expansion.
* A **pin** (`HierPin`) sits on a sheet symbol, at a point of the sheet the symbol is drawn on, and has a name and a direction (input, output, bidirectional, passive).
* A **port** (`SheetPort`) sits on the sheet that is shown, with a name and a direction. A pin and a port of the same name are one connection.
* None of the four is written when a schematic has no sheet symbol, so schematics without them keep their bytes.

## Operations

All are `EditOp` variants of `eh3d-phoenix-schematic`: all or nothing, one step for undo and redo, ids made by the supply of ids that depends only on the schematic and the number of ids asked for (the same operations on the same schematic give the same bytes). They are used by the window and by the file of operations of `phoenix schedit`.

| Operation | What it does |
|---|---|
| `remove_sheet`, `rename_sheet` | Removes a sheet with everything drawn on it (parts, wires, labels, power ports, junctions, the symbols on it with their pins, its ports and buses), or renames it. A sheet that a symbol still shows is not removed. |
| `add_sheet_block`, `remove_sheet_block`, `rename_sheet_block` | Draws a symbol that shows a sheet, removes it with its pins, or renames it. A symbol that would show its own sheet, directly or through other sheets, is refused; so is a name that is empty, has a slash or is used by another symbol of the same sheet. |
| `add_hier_pin`, `remove_hier_pin` | A pin on a symbol (one name once on a symbol). |
| `add_sheet_port`, `remove_sheet_port` | A port on a sheet (one name once on a sheet). |
| `move_to_sheet` | Moves parts, wires, labels, power ports and junctions to another sheet, shifted by an offset; the nets are made again from the drawing. |

## Expansion and names

`Schematic::expand(&HierConfig)` gives a flat schematic (`Expanded`) with every instance drawn out. The rules, in `config/hierarchy.cfg` where a value can be chosen:

* the **reference** of a part of an instance is made from the names of the symbols from the top sheet and its reference on the sheet: `U1/R1` (`refdes_style = prefix`) or `R1_U1` (`refdes_style = suffix`, with `refdes_separator`). A part on a top sheet keeps its reference;
* a **net label** is local to its instance (`local_labels = true`): the net `VOUT` of the instance `U1` is called `/U1/VOUT`, and on a top sheet `/VOUT` (`net_separator`). The same label on two instances is two nets. A **power port** is always global, and so is a label whose name is in `global_nets`. With `local_labels = false` every label is global, as in a schematic without sheet symbols;
* a pin and the port of its name get one hidden label, so the nets on the two sides are one net. A net that only a pin and a port name is called after the port (`/U1/IN`);
* the **name of a net** is the shallowest name that a label or a power port gives it (`GND` before `/VIN` before `/U1/SENSE`); a net no name reaches keeps its own name behind the name of its instance. The names do not change when something else is edited;
* the connections that only the net list gave (pins with no wire) are kept inside the instance they are in; a connection of the net list between two sheets is not kept, because the symbols say how sheets are connected;
* a schematic without sheet symbols is returned as it is: all its sheets are top sheets and its labels stay global.

`Expanded` also says, for each part of the flat schematic, the instance it is in and the part it was copied from, and for each net the instances where it has a pin (the report of `phoenix hier`). The netlist of the simulator (`phoenix netlist`, `phoenix sim --schematic`, `docs/NETLIST.md`), the electrical rule check (`phoenix erc`, `docs/ERC.md`) and the live check of the schematic editor draw the hierarchy out first; the rules of the table of models and the checks that look at the letters of a reference use the reference on the sheet (`R1` of `U1/R1`).

## Checks

`PHOENIX-HIER-001` to `009` (`docs/ERROR_CODES.md`, with the titles in the eight languages): a symbol, pin or port that refers to something that does not exist, a sheet that holds itself, a symbol with a name that cannot be used, a pin with no port and a port with no pin, a pin or port name used twice, a reference that repeats after the expansion, a hierarchy too deep or with too many instances, and a pin and a port with directions that do not fit. `Schematic::validate` runs them.

## Readers

* **KiCad** (`docs/KICAD_IMPORT.md`): a `sheet` of a `.kicad_sch` file becomes a symbol (its name, corner and size, and a pin for each pin of the symbol with the shape as direction), the file it names becomes the sheet it shows, and each hierarchical label of that file becomes a port. The reader still computes the nets of the whole design as before; the sheet of a file shown twice is read twice, one sheet for each symbol.
* **EAGLE** (`eh3d-phoenix-eagle-board`): a `module` becomes a sheet with its ports, and each `moduleinst` a symbol with a pin for each port, placed on the edge the port names. The parts and the nets inside a module are not read; they are counted in the note of what was not read.

## Command and window

`phoenix hier [--config FILE] [--json] [--lang xx] <schematic>` prints the sheets and symbols, the instances, the nets by hierarchical name with the instances where they have a pin, and the problems. It only reads. Exit code 0 without errors, 1 when the report has an error, 2 for unusable input.

The schematic editor window (`schematic_hier`, `schematic_hier_gui`) shows a row of tabs with the sheets in the order of the tree (a sheet shown N times once, with its number of instances), draws the symbols, their pins and the ports of the sheet, opens the sheet a symbol shows with a double click and goes back with a button. Its texts are in the eight languages. The logic is tested without a screen; the window has never been seen on one.

## Not done

The list is in `docs/NOT_YET.md`. In short: no pin is made for a port by the window (the operations make it), the window has no tool to draw a symbol (the operations and the file of operations do), a sheet shown by two symbols is drawn once and edited in one place, the variants of EAGLE modules are ignored, and the KiCad writer does not write the structure back.
