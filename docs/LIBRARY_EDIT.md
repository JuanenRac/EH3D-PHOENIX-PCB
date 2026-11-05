# EH3D-PHOENIX-PCB: the library editor

**State: prototype.** Four parts make the editor: the library in memory with its operations, checks, generators and plan (`eh3d-phoenix-libedit`), the commands `phoenix libedit` and `phoenix footprint`, and a library mode of the window of the viewer (`docs/VIEWER.md`). It edits the symbols, footprints and devices of EAGLE libraries (`.lbr`). It has only been tried on libraries written by hand for its tests: no EAGLE has opened a file written here, it has not been run on real libraries, the window has never been seen on a screen, and the dimensions of the footprint generators have not been checked against a standard text or a datasheet.

## What a library is here

A library holds footprints (EAGLE packages), symbols and device sets. The drawing items of a footprint or a symbol are integers: lengths in nanometres and the angle of an arc in micro-degrees. A footprint holds pads (surface-mount with a layer and a roundness, or through-hole with a drill and a shape: round, square, octagon, long or offset), lines and arcs, rectangles, circles, texts and holes; a symbol holds pins (name, electrical type, length, orientation, visibility, swap level), lines, arcs, rectangles, circles and texts. A device set holds gates (a symbol placed in the device) and devices (a footprint and the connection of each pin of each gate to one or more pads).

What the model does not read exactly is kept as the XML it came from and written back as it was: the parts of the file outside the three sections (settings, layers, the description of the library), the attributes of an item that the model does not use, the description of every footprint, symbol and device set, the technologies of a device, and every item that cannot be held exactly (a polygon, a dimension, a number with more than six decimals). The command reports how many items are kept like this; the window draws what it can and does not edit them. The pad number of a pin is not part of a symbol: as in EAGLE, the connections of each device say which pad each pin reaches, so a pin has a name and the check asks for the connections.

## The operations

An operation is one edit. It is a value that can be written in JSON (`"op"` names it), so the same edit can come from a person in the window or from a file.

| Operation | What it does |
|---|---|
| `create_package`, `create_symbol`, `create_device_set` | Makes an empty footprint, symbol or device set (a device set takes a prefix). |
| `rename_package`, `rename_symbol`, `rename_device_set` | Renames it; the devices that use a footprint and the gates that use a symbol follow. |
| `delete_package`, `delete_symbol`, `delete_device_set` | Deletes it; a footprint that a device uses and a symbol that a gate uses stay (`PHOENIX-LIB-033`). |
| `set_prefix` | Sets the prefix of the references of a device set. |
| `add_item`, `remove_item`, `replace_item`, `move_item` | Adds a drawing item to a footprint or a symbol (at the end, or at a place of the list), removes one, replaces one (this is how a property is edited) or moves one. The item is checked: the right kind for the drawing (no pin in a footprint, no pad in a symbol), values inside the range of the design, sizes above zero, keywords that exist, a rotation written as the file writes it. |
| `shift_origin` | Moves the origin to a point: every item moves the other way. |
| `set_opening` | Gives a pad an opening of the solder mask or of the paste that is larger or smaller than its copper by an expansion, drawn as a shape on the layer of the openings, and switches the automatic opening of the pad off; without an expansion the opening is removed and the pad is left to the automatic one. A through-hole pad is opened on both sides and has no paste. |
| `add_gate`, `remove_gate` | Adds a gate of a symbol to a device set, or removes it with its connections. |
| `add_device`, `remove_device`, `link_device` | Adds a device (a variant, with a footprint or without), removes it, or links it to another footprint or to none. |
| `connect`, `disconnect`, `auto_connect` | Connects a pin of a gate to a pad (or several, separated by spaces), removes the connection, or connects every pin to the pad of the same name when the footprint has one. |

A list of operations is applied all or nothing, on a copy: the first operation that is refused ends the run, names its place and leaves the library as it was. The footprints, symbols and device sets are shared, so a copy of the whole library costs a list of pointers; the history of undo and redo keeps one copy for every step (200 steps by default) and an operation copies only what it touches. The history knows whether the library has changed since it was read or saved.

## The checks

`check_library` goes through the library and reports, in a fixed order (kind, name, code, detail), so the same library always gives the same text and the same JSON:

| Code | Finding |
|---|---|
| `PHOENIX-LIB-001`, `002` | A pin of a symbol without a name, or two pins with one name (error). |
| `PHOENIX-LIB-003` | Two pads of a footprint with one name (error). |
| `PHOENIX-LIB-004` | A surface-mount pad on a layer that is not a copper layer of the settings, or without a size, or a through-hole pad without a drill (error). |
| `PHOENIX-LIB-005` | A drill larger than its pad (error). |
| `PHOENIX-LIB-006` | Silkscreen, line or arc or circle, drawn over the copper of a pad, with the clearance of the settings (warning). |
| `PHOENIX-LIB-007` | A pin of the symbol of a gate that reaches no pad in a device that has a footprint (warning). |
| `PHOENIX-LIB-008`, `009`, `010` | A connection to a gate, pin or pad that does not exist; a device with a footprint that the library does not have; a gate with a symbol that it does not have (error). |
| `PHOENIX-LIB-011` | A footprint whose origin is not inside the box of its pads, within a tolerance, or that has no pads (warning). |
| `PHOENIX-LIB-012` | A footprint without the texts `>NAME` or `>VALUE` (warning). |
| `PHOENIX-LIB-013`, `014` | A device set without gates or devices, or without a prefix (warning). |

The silkscreen is measured against the copper of each pad (a rectangle for a square, long or surface-mount pad, a disc for a round one, the disc that holds a pad turned by an angle that is not a quarter of a turn), less half the width of the line. The titles of the codes exist in the eight languages of the suite, with European Portuguese; the table is in `docs/ERROR_CODES.md`.

## Saving

An edited library is saved as a new file and never over an old one. Before anything is written the library is written to text, read back and compared item by item, and written again: the second text must be the first one (`PHOENIX-LIB-026` when it is not, and nothing is written). A file that exists is never replaced, which includes the file the library came from; a copy of the original, named by the first digits of its SHA-256, is kept next to the new file (`PHOENIX-LIB-032` when it cannot be made). After writing, the file is read from the disk and compared once more. The text is the same for the same library: attributes in a fixed order, no date, no name of a user or a computer.

## Footprint generators

`phoenix footprint` and the window make a footprint from parameters: `chip` (sizes 0201 to 2512), `soic` (narrow and wide), `tssop` (4.4 and 6.1 mm), `qfn` (with an optional exposed pad), `qfp`, `dip` (narrow, wide or a distance) and `header` (one row or two). The land of a lead follows the method of IPC-7351B (Generic Requirements for Surface Mount Design and Land Pattern Standard): with the overall length of the part `L`, the length of a terminal or the foot of a lead `T` and the width of a lead `W`, each with its tolerance, the fabrication tolerance `F` and the placement tolerance `P`,

* `Zmax = Lmin + 2 Jt + sqrt(CL^2 + F^2 + P^2)` (the outer edge of the land),
* `Gmin = Smax - 2 Jh - sqrt(CS^2 + F^2 + P^2)` (the inner edge), with `Smax = Lmax - 2 Tmin` and `CS` the root of `CL^2 + 2 CT^2`,
* `Xmax = Wmin + 2 Js + sqrt(CW^2 + F^2 + P^2)` (the width),

where `Jt`, `Jh` and `Js` are the fillet goals of the toe, the heel and the side and `CL`, `CT`, `CW` the ranges of the tolerances. The land is `(Zmax - Gmin) / 2` long, its centre is `(Zmax + Gmin) / 4` from the axis and its width is `Xmax`; all three are rounded to the grid of the file. The footprint also gets silkscreen kept away from the copper by a gap, a dot at the first pad, an assembly outline (layer 51), a courtyard (layer 39) and the texts of name and value; pads are numbered as the packages usually are (down the left column and up the right one for two rows; left, bottom, right and top for four sides).

Every dimension, tolerance and fillet goal is a value of `config/footprints.cfg`, per family and variant. **The file says that its values are a prototype**: the component dimensions and tolerances were entered by hand from the usual outline drawings of each package, and neither they nor the fillet goals have been checked against the text of the standard or against the datasheet of any part. Compare every footprint with the datasheet of the part before a board is made. A generated footprint passes the checks of the library (a test runs them on every family).

## Replacing an item by its master

The sanitizer (`docs/SANITIZER.md`) writes a plan: for each group of duplicates, the item to keep and the items it would replace. `phoenix libedit --plan` and the window read that plan for one library and list the steps that would replace its footprints and symbols by the master candidate. Nothing is replaced by reading the plan. A step is made only when the person confirms it, and then only if the replacement cannot move a part: the pads (or pins) of the master must be named as those of the item it replaces, a step that needs a turn, a mirror or a shift is left out, and device sets are not replaced. The replacement keeps the name, so the devices that use the item keep working, and it is one step that can be undone. The command never makes a step.

## The commands

```
phoenix libedit [--apply OPERATIONS.json] [--plan SANITIZER.plan.json] [--out NEW.lbr]
                [--config FILE] [--json] [--lang xx] <library.lbr>
phoenix footprint <family> [--param NAME=VALUE ...] [--config FILE] [--into LIBRARY.lbr]
                  [--out NEW.lbr] [--json] [--lang xx]
```

Both are a dry run unless `--out` names a new file: they read, apply, check and report, and write nothing. `--apply` reads a JSON list of operations (or an object with the list under `operations`). `--into` starts a footprint from a copy of a library and refuses a name that is taken. The exit code is 0 when the check finds no error, 1 when it does and 2 when the arguments, the input, an operation or the output are unusable.

## The window

The library mode opens with a `.lbr` file (typed in the path box, dropped on the window or given on the command line) or with the button *Library editor*, which starts an empty library. The left panel lists the footprints, symbols and device sets, makes, renames and deletes them, holds the tools, the grid and the snap, the generator, the plan and the saving as a new file. The canvas draws the footprint or the symbol with a grid of the size of the settings (50 µm for footprints, 50 mil for symbols), the origin, the layers in colours and the names of the pads; it zooms with the wheel, pans with a drag and snaps the points to the grid. The tools are: select and drag, surface-mount pad, through-hole pad, hole, line, arc, circle, text, pin and measure. The right panel edits the properties of the item selected (name, position, size, drill, shape, layer, rotation, roundness, the angle of an arc, the type and the length of a pin), sets the openings of a pad and moves the origin to the cursor, edits a device set (prefix, gates, variants, the footprint of a variant and the pad of each pin) and lists the findings of the check, each with a button that goes to its place. Delete, `R` (turn), `Esc`, undo and redo have keys. The check runs again after each edit while the library has up to a number of items given in the settings; above it, the button runs it. The texts are in the eight languages of the suite.

## Settings

`config/libedit.cfg` holds the limits of the file that is read, the length of the history, the tolerance of the origin and the clearance of the silkscreen, the layers that count, whether the texts of name and value are asked for, the grids and the snap, the distance of a click, the expansions of the openings, what the tools make (the size of a new pad, the width of a new line, the height of a text, the length of a pin) and the limit of the automatic check. `config/footprints.cfg` holds the values of the generators. Both are plain `key = value` text; an unknown key or a value out of range is refused with its line number (`PHOENIX-LIB-029`).

## Codes

The codes are `PHOENIX-LIB-<NNN>`: 001 to 014 are the findings above and 020 to 033 are errors of reading, editing, saving and generating. The titles are in `docs/ERROR_CODES.md` and, in the eight languages, in the crate.

## What is not done

* The EAGLE binary files are not read, and a file written here has not been opened in EAGLE.
* Polygons, dimensions, frames and the other items the model does not read are kept but cannot be edited or drawn.
* A copy of a footprint, symbol or device set, moving several items at once, mirroring, and the bulk edit of a property are not written; an edit works on one item.
* The expansion of a mask or paste opening is a shape drawn on the layer of the openings; the rules for the openings of a board are not applied.
* Footprints with different land rules by density level, and the families not listed above, are not generated; the dimensions are not verified.
* Replacing a device set by its master, and relinking boards and schematics to the replaced items, are not written: a plan changes one library, and the check that no placed part moves needs the boards.
* The window has never been seen on a screen or used with a mouse, and the time of an edit on a library of thousands of items was not measured.
