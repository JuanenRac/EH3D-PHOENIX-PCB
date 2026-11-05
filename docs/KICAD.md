# EH3D-PHOENIX-PCB: KiCad readers

The crate `eh3d-phoenix-kicad` reads the files of KiCad 6 to 9 into the same design model that the EAGLE files go into: the symbols and the schematic of `eh3d-phoenix-schematic` and the footprints and the board of `eh3d-phoenix-board`, with lengths in integer nanometres and angles in integer micro-degrees. It only reads; nothing is written back to KiCad (PPCB-013 stays planned).

**State: prototype.** The readers are written from the public description of the file formats (the file format pages of the KiCad developer documentation) and tried on files written by hand for the tests, in the forms of KiCad 6, 7 and 8 and on damaged files. They have not been compared with KiCad itself and have not run on a public collection of designs, so nothing here is `validated`: the oracle of requirement PPCB-012 (the same connectivity as the reference tool on a public corpus with reviewed licences) is still to be run. Where a rule below says what KiCad does, it is the reading of the public description; the first comparison with the oracle may change it.

## What is read from each format

| File | Into the model | Read | Not read |
|---|---|---|---|
| `.kicad_sym` (`kicad_symbol_lib`) | symbols of the schematic model | each symbol with its pins (number and position, y up as in the file), the pins of every unit and of the first drawing style, `extends` of KiCad 7 and later (the pins of the symbol it extends) | the drawing (rectangles, lines, circles, arcs, texts), the names and the electrical types of the pins (the type is only used to find hidden power inputs), the fields other than the reference and the value, the second drawing style |
| `.kicad_mod` (`footprint`, or `module` of older files) | a footprint of the board model | the name, the reference and the value, the pads: number, shape (circle, rectangle, oblong), position, angle relative to the footprint, size, drill, layers | the silkscreen, fabrication and courtyard drawing, the texts, the 3D model, the zones inside the footprint, the shapes of custom pads |
| `.kicad_sch` (`kicad_sch`) | the schematic model | the sheet (paper size, title), the symbols of `lib_symbols`, the placed symbols (reference, value, footprint name, position, turn of a right angle, mirror, unit), wires, junctions, local, global and hierarchical labels, power symbols, sheet symbols with their pins and, through a loader the caller gives, the sheets they refer to | graphics (lines, rectangles, circles, arcs, texts, images), buses and bus entries, no-connect flags, rule areas, net classes of the sheet, the fields other than the reference, the value and the footprint |
| `.kicad_pcb` (`kicad_pcb`) | the board model | the layer table and from it the order of the copper layers, the nets, the footprints with their pads and the net of each pad, tracks (lines and arcs), vias, zones (outline, clearance, one zone for each layer), the outline on `Edge.Cuts`, lines, arcs, circles, rectangles, polygons and texts on the silkscreen layers | the setup (stack-up, rules, plot settings), the filled copper of zones, keep-out zones, zones, tracks and vias without a net, graphics on layers other than the outline and the silkscreen, dimensions, groups, images and generated items |
| `.kicad_pro` (JSON) | net classes and design rules (not in the board model) | the version, the net classes with clearance, track width, via diameter and via drill, the numeric minimum values of the design rules, the number of text variables | the rest (editor settings, ERC rules, colours, plot settings, the sheet list) |

Everything that is left out is counted: the quality report of the import has three lists, what was kept, what was changed and what was left out, each category with its reason in its name (for example `filled copper of zones (computed again by the tool)`), and the elements the readers do not interpret are counted by name with a note `PHOENIX-KICAD-006`.

## Conventions of the model

* **Lengths and angles.** The decimals of the files are millimetres and degrees; they become integer nanometres and integer micro-degrees (half to even beyond six decimals). A number outside the checked range of the model (about 1073 mm) is a finding `PHOENIX-KICAD-005` and the item is left out; an A0 sheet therefore keeps the largest size the model holds.
* **Orientation.** The y axis of KiCad points down and the model's points up, so every position of a file gets its y turned. Angles keep their sign: a positive angle is a counter-clockwise turn on the screen in both. The pins of a symbol are already y up in the file and are kept as they are.
* **Footprints.** KiCad writes the angle of a pad as an angle on the board, so the angle of the footprint is taken off to get the angle relative to the footprint. A rounded rectangle, a trapezoid or a custom shape is read as the rectangle of its size (finding `PHOENIX-KICAD-007` and the category `pads with a simpler shape` in the report). An oval drill is read as a round one of the larger size. A non-plated pad keeps its drill but the model has no plating flag. Two placed footprints that are the same except for their nets share one footprint of the model. The pad positions of a footprint on the bottom side are taken as stored; whether the mirroring is applied twice is one of the things the oracle must check.
* **Tracks.** Straight pieces and arcs are joined into runs of one net, one layer and one width when they follow each other in the file and end where the next starts. An arc is stored by KiCad as start, middle and end; the model stores start, end and a signed sweep, which is computed from the three points (floating point, rounded once to a micro-degree). Three points on a line are read as a straight piece (a finding).
* **Outline.** The pieces on `Edge.Cuts` give the largest closed ring: a rectangle, a circle, a polygon, or lines and arcs that join end to end. Arcs and circles are cut into straight pieces (`outline_arc_steps_per_quarter` in the settings). Cut-outs, and pieces that do not close, are counted as left out.
* **Layers.** A layer gets its kind and side from its name (`F.Cu`, `In1.Cu`, `B.SilkS`, `F.Mask`, `F.Paste`, `Edge.Cuts`; the rest is documentation). `*.Cu`, `F&B.Cu` and `*.Mask` stand for the layers they say. A copper layer the file names in the table keeps its place in the stack: the top first, the inner layers by number, the bottom last.
* **Ids.** The model's objects get their ids from the caller, so the same file gives the same ids when the caller's generator is deterministic.

## The nets of a schematic

KiCad does not store the nets in a schematic file, so they are computed from the drawing, and this is the part that most needs the oracle:

* a pin, a wire end, a junction and a label that share a position are one node; a wire joins its two ends; a point that lies in the middle of a horizontal or vertical wire is joined to it (a crossing without a junction is not a connection; wires that are neither horizontal nor vertical do not join points in their middle, which is reported);
* the labels with one name in one sheet are joined; global labels and power symbols (the value of the power symbol is the name of the net) are joined in every sheet; a hierarchical label is joined to the pin of the same name on the sheet symbol above; a hidden power input pin joins the net of its pin name;
* the name of a net is the name of a global label or a power symbol, otherwise the name of a local label with the path of its sheet (`/NAME`, `/Sub/NAME`), otherwise `Net-(R1-Pad1)` after its first pin; a net with one pin and no name is not stored;
* the symbols that are power symbols or whose reference starts with `#` are not parts of the model; they only name nets;
* a symbol drawn as several units is one part whose symbol holds the pins of all its units, placed relative to the first unit (a finding); a mirrored symbol gets a mirrored copy of its symbol; a rotation that is not a right angle is rounded (a finding);
* an unannotated reference (`R?`) is replaced by the one in the `instances` of KiCad 7 and later, or the `symbol_instances` of KiCad 6, when there is one.

The sheets of a hierarchy are read through a function the caller gives, which receives the name as the schematic writes it. A name with `..`, an absolute name or a drive is never passed to it; a sheet that cannot be read, that leaves the folder or that contains itself is a finding and the rest is read. The command `phoenix kicad-read` gives a loader that reads below the folder of the file. The depth and the number of sheets are limited by the settings.

## Differences between versions that the readers know

| Item | KiCad 6 | KiCad 7 | KiCad 8 and later |
|---|---|---|---|
| Header | `(version 20211014) (generator pcbnew)`, unquoted | `(version 20221018) (generator pcbnew)` | `(version 20240108) (generator "pcbnew") (generator_version "8.0")`, quoted |
| Footprint | `(footprint "Lib:Name" ...)` with `fp_text reference` and `fp_text value` (files of KiCad 5 use `module`) | the same | `(property "Reference" "R1" ...)` and `(property "Value" ...)` |
| Width of a drawn line | `(width 0.1)` | `(stroke (width 0.1) (type default))` | the same (tracks keep `(width 0.25)`) |
| Net of a pad or a track | `(net 3 "GND")` and `(net 3)` | the same | the same; a net by name only, `(net "GND")`, is accepted whatever the version |
| Symbol reference | `Reference` property, `symbol_instances` at the end | `instances` inside the symbol | the same |
| Symbols that extend another | no | `(extends "Name")` | the same |
| Pin visibility | the word `hide` | the word `hide` | `(hide yes)` |

A version outside the range the tests cover (`min_tested_version` and `max_tested_version` in the settings) is read anyway, with a finding `PHOENIX-KICAD-009`.

## Safety of the readers

Every file is hostile until proven otherwise. The s-expression reader keeps no recursion (the nesting is a counter, checked against `max_depth`), refuses a file larger than `max_file_bytes`, more than `max_nodes` nodes and a text longer than `max_string_bytes`, and gives a typed error with the line and the column for a parenthesis that closes nothing, text outside the list, a string that is never closed and a list that is still open at the end. A file that is not UTF-8 is read with the invalid bytes replaced and a finding. Nothing panics on its input; the tests cut a valid file at every point and read what is left. The number of items of one kind, of sheets and of findings is limited too (`max_items`, `max_sheets`, `max_sheet_depth`, `max_diagnostics`). Every limit is a line of `config/kicad.cfg`.

## Findings

The codes `PHOENIX-KICAD-001` to `PHOENIX-KICAD-014` are in `docs/ERROR_CODES.md`, with their titles in the eight languages in `eh3d-phoenix-kicad` (`title`). Every finding that comes from a place in a file says the line (and, for a syntax error, the column).

## The importer and the command

`KicadImporter` registers in the format hub (`eh3d-phoenix-hub`). It finds the kind of file from the first bytes (the name of the root list, or a JSON object together with the extension `.kicad_pro`), reads it, and returns the quality report and a last finding that says the **fidelity level**: `complete` (nothing changed or left out), `partial` (something was; the report says what) or `unusable` (the file could not be read).

```text
phoenix kicad-read [--settings FILE] [--lang xx] [--redacted] <file>
```

It prints the format, the kind, the version, whether the file was usable, the fidelity level, the three lists of the report and the findings with their codes, lines and titles in the language asked for. `--redacted` leaves out the name of the file and every message and prints only counts and codes, for runs over a private collection. Exit codes: 0 the file was read, 1 it could not be read, 2 the arguments or the files are unusable.

## What is next

The comparison with the oracle: the same connectivity as KiCad on a public corpus of open hardware with reviewed licences, a classification of every difference, and the check of the points marked above (the nets of a schematic, the mirroring of the pads of the bottom side, the arcs). Writing KiCad files (requirement PPCB-013) and the items the readers leave out are in `docs/NOT_YET.md`.
