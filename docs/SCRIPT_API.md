# EH3D-PHOENIX-PCB: the script language `phoenix-script/2`

**State: prototype.** The crate `eh3d-phoenix-script` reads two languages. The first is the command language of the previous workflow (`docs/SCRIPT.md`), which is unchanged. The second, described here, is a small language with variables, bounded loops and conditions on measures that selects objects of a board, a schematic or a library by a query and changes them through the same reversible operations as the editors. A script is of the second version when its first significant line is `phoenix-script/2`; any other script is of the first version, so every script that ran before runs as before. It has been tried on boards, schematics and libraries made by hand for its tests, and never on a design of the author's own.

The API is stable and versioned: the header names the version (`phoenix-script/2`), a function that is added to the table does not change it, and a function that is renamed, removed or changed in meaning would be a new version. The list of functions below is written from the one table the interpreter, the static check and `phoenix script help` use, and a test fails when this page lacks a function of the table.

## The commands

```
phoenix script run   [--config FILE] [--rules FILE] [--parts FILE] [--dry-run] [--out FILE] [--json] [--lang xx]
                     <script> [--board FILE | --schematic FILE | --library FILE]
phoenix script check [--config FILE] [--json] [--lang xx] <script>
phoenix script list  [--json] [--show NAME]
phoenix script help
```

* `script run` works on a copy of the document. It is a **dry run** unless `--out FILE` is given and `--dry-run` is not: the report says what the script printed, how many steps and operations it took and whether it is one step of the history, and nothing is written. With `--out` the result is written to a **new** file (`.json` for a board or a schematic, EAGLE text for a library); a file that exists is never replaced, and the input is never overwritten. The files that the script asked to write with `write_text` are written at the same moment and not before. Exit codes: 0 done, 1 the script was not carried out (its diagnostic is printed with the line and the column), 2 the arguments or the input are unusable. The document is given by `--board`, `--schematic` or `--library` (or, as for the first version, as the second argument; a `.lbr` file is a library). `--rules` names the JSON rule file that `drc_count()` and `drc_errors()` check against, and `--parts` the parts base that `assign_part` reads. The report is deterministic: no time, no path of the computer, no identity that changes.
* `script check` reads a script without running it and without a document: the syntax, that every variable is defined before it is used, that every function exists, takes that many arguments and applies to the kind of document the header names, and an estimate of the steps (an upper bound by the limits; a loop whose count the script writes is counted exactly). Exit code 0 without errors, 1 with them.
* `script list` shows the example scripts (`--show NAME` prints one; they are in `examples/scripts/`, see `docs/SCRIPT_EXAMPLES.md`). `script help` prints the reference of the language, written from the table of functions.

## The language

A script starts with the header `phoenix-script/2`, optionally followed by `board`, `schematic` or `library`, the kind of document it is written for (a run on another kind is refused with PHOENIX-SCRIPT-058). A statement ends at a line break or at a semicolon; a `#` starts a comment; a block is written between braces and `else` follows the closing brace on the same line. Names are letters, digits and underscores.

**Values.** Integers of 64 bits, `true` and `false`, `nil`, texts in single or double quotes (`\n`, `\t` and an escaped quote are understood), lists written `[a, b]` and objects of the document (components, tracks, vias, zones, nets and parts), which a query returns. There are no decimal numbers: a length is written with a unit and is the integer count of nanometres (`0.2mm`, `8mil`, `1in`, `25um`), an angle is written with `deg` and is the integer count of micro-degrees (`90deg`, `22.5deg`), and a number without a unit is a plain integer. The same script therefore gives the same bytes on every computer. A length written outside the range of the model, or a decimal number without a unit, is a syntax error.

**Operators**, from the loosest: `or`, `and`, `not`, `==` `!=` `<` `<=` `>` `>=`, `+` `-` (also `+` on two texts), `*` `/` `%` (integer division towards zero), unary `-`, then `a[i]` (a list or a text, from 0) and `object.field`. `and` and `or` evaluate the right side only when needed. A condition must be a bool.

**Statements.** `let name = expression` defines a variable in the block (a name once per block; a name from a block around is hidden, not changed); `name = expression` changes a variable that exists; `if c { } else if d { } else { }`; `for i in first..end { }` (the numbers `first` to `end - 1`) and `for item in list { }`; `while c { }`; `break` and `continue` in a loop; and an expression, usually a call `name(arguments)`. There are no user functions, so nothing recurses.

**Objects and fields.** `select("component", "C*")` returns the components whose designator matches (`*` any run, `?` one character); the same for `track`, `via` and `zone` (by the net), `net` (by the name) and `part` (a part of a schematic). The filters `filter_net`, `filter_layer`, `filter_rect`, `filter_kind` and `filter_value` narrow a list, `sort_refdes` and `sort_position` order it, and `distance`, `nearest`, `net_length`, `drc_count`, `erc_count` and `ratsnest_count` measure. A field is read as `object.name` or `field(object, "name")`:

| Kind | Fields |
|---|---|
| component | `refdes`, `value`, `footprint`, `x`, `y`, `rot`, `side` (`top` or `bottom`), `pads` |
| track | `net`, `layer`, `width`, `length`, `segments` |
| via | `net`, `x`, `y`, `drill`, `diameter` |
| zone | `net`, `layer`, `clearance`, `points` |
| net (board) | `name`, `tracks`, `vias`, `pads`, `length` |
| part (schematic) | `refdes`, `value`, `symbol`, `x`, `y`, `rot`, `pins` |
| net (schematic) | `name` |

The operations take an object or the designator as a text (`move_to("R1", 5mm, 6mm)`), and the ones that make something return it (`let t = route(...)`). An object that was removed is refused with PHOENIX-SCRIPT-059.

## One step of the history, all or nothing

Every operation of a script goes through the reversible operations of the model of the document (the board's `Operation`, the schematic's `EditOp`, the library's `Operation`), one at a time on a copy. The first operation that the model refuses ends the run: the document is not touched, the outcome holds no result and the diagnostic names the line and the column. A run that ends well holds the result and **one batch** that does the whole script, so applying it to the original gives the result, one undo gives the original back byte for byte, and in the history of the editor the script is a single step. A script that changes nothing is no step. `panelize` replaces the whole board, so a run with it has no batch that undoes it (`replaced` in the JSON report). The identities of new objects come from the seed and the clock of the settings, so the same script on the same document gives the same bytes.

## The sandbox and the limits

A script has no function that reaches the network, starts a program, reads the environment or the clock, or opens a file; a name for any of those is an unknown function. The one way to a file is `read_text` and `write_text`, and only inside the data folder: `data_dir` of the settings, a folder below the folder of the script (empty by default, so no file is reachable). A path must be relative, without a drive, a root or `..`, with no link on the way, inside that folder; a file that exists is never replaced; a write is only noted until the run is applied with `--out`; the number and the size of the files are bounded. Time is counted in steps of the interpreter, not in seconds, so a script stops at the same place on every computer. Every limit is a key of `config/script.cfg` (an unknown key or a value out of range is refused with its line):

| Key | Default | What it bounds |
|---|---|---|
| `max_steps` | 1000000 | steps of the interpreter: one for each statement and each part of an expression |
| `max_loop_iterations` | 10000 | turns of one loop |
| `max_block_depth`, `max_expr_depth` | 16, 32 | nesting of blocks and of an expression |
| `max_value_cells` | 20000 | the memory of the variables, in cells (a number or an object is one, a list one and its items, a text one and one for every 16 characters) |
| `max_list_len`, `max_text_bytes` | 5000, 262144 | one list, one text made while running |
| `max_output_bytes` | 65536 | everything `print` writes |
| `max_operations` | 100000 | operations made on the document |
| `max_script_bytes`, `max_lines`, `max_string_chars` | 1048576, 20000, 256 | the text of the script |
| `data_dir` | empty | the only folder a script can read or write |
| `max_data_files`, `max_data_bytes` | 8, 1048576 | files used in one run, the size of one |

## Errors

Every error is a diagnostic with a code `PHOENIX-SCRIPT-043` to `PHOENIX-SCRIPT-064`, the line and the column, and a detail in English; the title is in the eight languages (`--lang`). The codes are in `docs/ERROR_CODES.md`. The syntax is reported at the character that cannot be read; a runtime error at the statement, the call or the operator that caused it.

## The functions

### Values, text and lists

| Function | Applies to | Changes the document | What it does |
|---|---|---|---|
| `len(x)` | any | no | The number of items of a list or of characters of a text. |
| `str(x)` | any | no | The text of any value; an object is written by its designator or its net. |
| `int(text)` | any | no | The integer a text holds; an error when it holds anything else. |
| `abs(n)` | any | no | The absolute value. |
| `min(a, b)` | any | no | The smaller of two integers. |
| `max(a, b)` | any | no | The larger of two integers. |
| `isqrt(n)` | any | no | The integer square root, rounded down. |
| `muldiv(a, b, c)` | any | no | a * b / c computed in 128 bits and rounded half away from zero. |
| `sin_ppm(angle)` | any | no | The sine of an angle in micro-degrees, in millionths (1000000 is 1). |
| `cos_ppm(angle)` | any | no | The cosine of an angle in micro-degrees, in millionths. |
| `polar_x(radius, angle)` | any | no | radius * cos(angle), rounded to the nanometre. |
| `polar_y(radius, angle)` | any | no | radius * sin(angle), rounded to the nanometre. |
| `deg(n)` | any | no | A whole number of degrees as micro-degrees (the same as writing 90deg). |
| `mm(length)` | any | no | A length in nanometres as a text in millimetres, without trailing zeros. |
| `push(list, item)` | any | no | A new list with the item added at the end. |
| `concat(a, b)` | any | no | A new list with the items of both lists. |
| `reverse(list)` | any | no | A new list in the opposite order. |
| `sum(list)` | any | no | The sum of a list of integers. |
| `join(list, separator)` | any | no | The items of a list written as one text. |
| `contains(list_or_text, x)` | any | no | Whether a list has the item, or a text has the piece. |
| `sort(list)` | any | no | A new list of integers or of texts in ascending order. |
| `range(first, end)` | any | no | The list first, first+1, ... end-1. |
| `upper(text)` | any | no | The text in capitals. |
| `lower(text)` | any | no | The text in small letters. |
| `starts_with(text, prefix)` | any | no | Whether the text starts with the prefix. |
| `ends_with(text, suffix)` | any | no | Whether the text ends with the suffix. |
| `zpad(n, width)` | any | no | The number written with zeros in front up to the width. |
| `repeat(text, n)` | any | no | The text written n times. |
| `glob(pattern, text)` | any | no | Whether the text matches the pattern (* any run, ? one character). |
| `split(text, separator)` | any | no | The pieces of a text between the separators, as a list. |
| `lines(text)` | any | no | The lines of a text, as a list. |

### Output and checks

| Function | Applies to | Changes the document | What it does |
|---|---|---|---|
| `print(x, ...)` | any | no | Writes a line with the values separated by a blank. |
| `assert(condition, message)` | any | no | Stops the script with an error when the condition is false. |
| `fail(message)` | any | no | Stops the script with an error. |

### Queries and measures (board and schematic)

| Function | Applies to | Changes the document | What it does |
|---|---|---|---|
| `select(kind, pattern)` | board, schematic | no | The objects of a kind (component, track, via, zone, net, part) whose designator or net matches the pattern. |
| `field(object, name)` | board, schematic | no | A field of an object, also written object.name: refdes, value, footprint, x, y, rot, side, pads of a component; net, layer, width, length of a track; net, x, y, drill, diameter of a via; name, tracks, vias, pads, length of a net; refdes, value, symbol, x, y, rot, pins of a part. |
| `exists(object)` | board, schematic | no | Whether the object is still in the document. |
| `names(list)` | board, schematic | no | The designators or net names of a list of objects. |
| `filter_kind(list, kind)` | board, schematic | no | The objects of a list that are of a kind. |
| `filter_net(list, pattern)` | board | no | The objects of a list on a net that matches the pattern (a component when one of its pads is on it). |
| `filter_layer(list, pattern)` | board | no | The tracks and zones on a layer that matches the pattern, the vias that start or end on it, the components on that side (top or bottom). |
| `filter_value(list, pattern)` | board, schematic | no | The components or parts whose value matches the pattern. |
| `filter_rect(list, x1, y1, x2, y2)` | board, schematic | no | The objects with a place inside the rectangle (a track or a zone when one of its points is inside). |
| `sort_refdes(list)` | board, schematic | no | The objects by name the way a person sorts them (C2 before C10). |
| `sort_position(list, order)` | board, schematic | no | The objects by place: xy (by x, then y) or yx (by y, then x). |
| `distance(a, b)` | board, schematic | no | The distance in nanometres between the places of two objects (components, vias or parts), rounded down. |
| `nearest(object, list)` | board, schematic | no | The object of the list closest to the object (nil for an empty list); the first one on a tie. |
| `pad_nets(component)` | board | no | The names of the nets on the pads of a component, sorted. |
| `net_length(name)` | board | no | The copper length of a net in nanometres; an arc is counted by its chord. |
| `outline_bounds()` | board | no | The box of the board outline as [x1, y1, x2, y2]. |
| `layers()` | board | no | The names of the layers of the board. |
| `drc_count()` | board | no | The findings the rule check proves on the board now, with the rules of the run. |
| `drc_errors()` | board | no | The proven findings of the rule check that are errors. |
| `erc_count()` | schematic | no | The findings the electrical rule check proves on the schematic now. |
| `ratsnest_count()` | board | no | The connections still to be routed. |

### Board operations

| Function | Applies to | Changes the document | What it does |
|---|---|---|---|
| `move_to(target, x, y)` | board, schematic | yes | Puts a component (or a part of the schematic) at a place; a component keeps its angle. |
| `move_by(target, dx, dy)` | board, schematic | yes | Moves a component (or a part) by an offset. |
| `rotate_to(component, angle)` | board | yes | Sets the angle of a component, in micro-degrees (90deg). |
| `rotate_by(target, angle)` | board, schematic | yes | Turns a component by an angle; a part of the schematic turns by whole quarter turns only. |
| `mirror(target)` | board, schematic | yes | Puts a component on the other side of the board (its pads keep their nets; tracks do not follow), or mirrors a part. |
| `rename(target, name)` | board, schematic | yes | Renames a component, a part or a net; refused when the name is taken. |
| `set_value(target, text)` | board, schematic | yes | Sets the value of a component or a part. |
| `remove(target)` | board, schematic | yes | Removes a component, a track, a via or a zone (a part of the schematic). |
| `place(refdes, footprint, x, y[, angle[, value]])` | board | yes | Places a new component of a footprint that the board has, on the top side; returns it. |
| `route(net, layer, width, points)` | board | yes | Draws a track of a net on a copper layer through points given as a flat list [x1, y1, x2, y2, ...]; the net is made when missing; returns the track. |
| `via(net, x, y[, diameter[, drill]])` | board | yes | Adds a via between the outer copper layers; the sizes default to the ones in the settings; returns it. |
| `zone(net, layer, points, clearance)` | board | yes | Adds a copper zone of a net with a polygon given as a flat list of points; returns it. |
| `connect(component, pad, net)` | board | yes | Joins the pad with that number of a component to a net (made when missing). |
| `renumber(list, prefix, first)` | board, schematic | yes | Names the objects of a list prefix+first, prefix+first+1, ... in the order of the list, without a clash on the way; returns how many. |
| `panelize(columns, rows, gap)` | board | yes | Replaces the board by a matrix of copies with a gap between them, each designator ending with _1, _2, ... (the frame, the tabs and the tooling are the business of phoenix panel, which also makes the files). The run then has no batch that undoes it. |
| `assign_part(target, part_id)` | board, schematic | yes | Sets the value of a component or a part from a part of the parts base (--parts); returns the footprint the base names. |

### Schematic and library operations

| Function | Applies to | Changes the document | What it does |
|---|---|---|---|
| `place_part(refdes, symbol, x, y[, value])` | schematic | yes | Places a part of a symbol that the schematic has on its first sheet; returns it. |
| `draw_wire(points)` | schematic | yes | Draws a wire of horizontal and vertical pieces through points given as a flat list on the first sheet. |
| `label(name, x, y)` | schematic | yes | Puts a net label on the first sheet. |
| `lib_names(kind, pattern)` | library | no | The names of the footprints (package), symbols (symbol) or device sets (deviceset) that match the pattern. |
| `lib_create(kind, name)` | library | yes | Creates an empty footprint, symbol or device set. |
| `lib_rename(kind, from, to)` | library | yes | Renames a footprint, a symbol or a device set. |
| `lib_delete(kind, name)` | library | yes | Deletes a footprint, a symbol or a device set. |
| `lib_add_pad(footprint, name, x, y, drill, diameter)` | library | yes | Adds a through-hole pad to a footprint. |
| `lib_add_smd(footprint, name, x, y, width, height)` | library | yes | Adds a surface-mount pad to a footprint. |
| `lib_pad_count(footprint)` | library | no | The number of pads of a footprint. |

### Data files

| Function | Applies to | Changes the document | What it does |
|---|---|---|---|
| `data_enabled()` | any | no | Whether the settings give the script a data folder (data_dir) to read and write. |
| `read_text(path)` | any | no | Reads a text file of the data folder (data_dir); an error when there is none, the path is not allowed or the file is too large. |
| `write_text(path, text)` | any | no | Asks to write a new text file in the data folder; it is written only when the run is applied, and never over a file that exists. |

## What is not done

See `docs/NOT_YET.md`: functions defined by the script, decimal numbers, selecting by pointing, the operations of the board editor that need its rule checks (the push of neighbour tracks, differential pairs, tuning of lengths) and the schematic operations on buses and hierarchy, a process of its own with lower privileges for the sandbox, and a run on a design of the author's own.
