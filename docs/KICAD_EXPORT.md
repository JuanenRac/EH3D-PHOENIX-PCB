# EH3D-PHOENIX-PCB: writing KiCad files

The crate `eh3d-phoenix-kicad` also writes the design model as KiCad 8 files, and the command `phoenix kicad-export` runs it: a board as a `.kicad_pcb` file, a schematic as a `.kicad_sch` file for each sheet, the footprints of a board as `.kicad_mod` files and the symbols of a schematic as a `.kicad_sym` library. It is the counterpart of the readers in `docs/KICAD.md`.

**State: prototype.** The writers are made from the public description of the file formats. The files they write are read back by the test reader of the tests (written apart from the code it checks) and by the readers of the crate, and the counts are compared. **No file written here has been opened in KiCad**, so nothing says that KiCad opens them, that its own checks agree with the checks of this program or that a board written here can be made. The requirement PPCB-013 stays a prototype until the oracle shows it.

## What is written

| Model | File | Written | Left out or simplified (counted in the report) |
|---|---|---|---|
| board | `.kicad_pcb` | the layer table (copper layers named `F.Cu`, `In1.Cu`, ... `B.Cu` from their place in the stack, the standard technical layers), the nets, the footprints of the placed components with their pads (SMD, through-hole and non-plated, circle, rectangle and oval, drill, layers, net), track pieces (lines and arcs), vias, copper zones with their outline, clearance and minimum width, the cutouts of a zone as rule areas on its layer that keep the pour out (and allow tracks, vias and pads), the board outline and the cutouts inside it on `Edge.Cuts`, rounded rectangular pads (`roundrect` with its `roundrect_rratio`), pads that are not plated (`np_thru_hole`), silkscreen text and strokes | design rules, net classes and the stackup (other documents of the project), the drawings of footprints (the model holds pads only), the filled copper of zones (KiCad fills it again; a cutout is written as a rule area, which the report says, and a cutout of fewer than three points is left out and counted), the font of the model (its text is written as text of KiCad), links to the schematic written as a path of the first sheet |
| schematic | `.kicad_sch` | the first sheet in a file and every other sheet in a file of its own that the first refers to with a sheet symbol; the library of the symbols used (a rectangle around the pins, because the model has no drawing), the placed parts with reference, value, footprint, turn, mirror and the identity of their pins, wires, junctions, net labels, power ports (as power symbols), buses (the label of the bus and a line from its anchor through the ends of its entries) and bus entries (a `bus_entry` with the label of its member at the far end) | the net list (KiCad derives nets from the drawing), the connections between sheets (labels are local to a sheet), a turn that is not a right angle (rounded) |
| symbols | `.kicad_sym` | one symbol for each symbol of the model with its pins (number, place, direction towards the body, electrical type) | the drawing of a symbol, units and styles |
| footprints | `.kicad_mod` | one file for each footprint of a board, the pads as above | the same as the board |

Everything that the model cannot say is counted in the report of the export, each kind with a code and a category; nothing is dropped in silence. A part without a place in the drawing is put on a grid and its nets are written as labels on its pins. A pin without a position is put in a column, a pin without a type is written as unspecified, and a layer without a side is written on the top side; each of these is a finding.

## Conventions

* **Lengths and angles.** Nanometres and micro-degrees are written as millimetres and degrees with the fewest decimals that are exact (`1.27`, `-0.000001`), never rounded. A value is written from integers, so two runs give the same bytes.
* **Orientation.** The y axis of the model points up and the one of KiCad points down, so every position of a board and of a schematic sheet gets its y turned, as the readers do. The pins inside a symbol keep their y, as in the files of KiCad. Angles keep their sign. Nothing is moved onto the page of a schematic: a drawing made with y up lands on the negative side of the sheet.
* **Arcs.** The model stores start, end and a signed sweep; KiCad wants a point halfway along the arc, which is computed once with floating point and rounded to the nanometre. A sweep that gives no circle is written as a straight piece and counted.
* **Bottom side.** A footprint of the bottom side is written on `B.Cu` with the x of its pads mirrored, as the connectivity of the model does, and the angle of a pad negated. The rule has not been compared with KiCad and the report says so.
* **Ids.** Almost every item of a KiCad file has a UUID. Here it is derived from the id of the model object and a role by a fixed hash, never from a clock or from chance, so the same model always gives the same UUIDs and the pad of a component and the part of a schematic keep the same one from run to run. The footprint of a board refers to its part of the schematic by the same UUID.
* **Names.** The characters that KiCad does not allow in the name of a footprint or a symbol are replaced by an underscore, two nets or two symbols with one name get the number added, and a net without a name is called `Net-<number>`; each is a finding.
* **Version.** The files carry the version numbers of the formats of KiCad 8 (`pcb_version` and `sch_version` in the settings). No date, no program version and nothing random is written.

## The settings

`config/kicad_export.cfg` holds the values the writers use (the version numbers, the program name, the library name put before a footprint or a symbol, the sizes of text, the width of lines, the pitch of a grid, the length of a pin and the most items of one kind). A missing line keeps its default; an unknown key is an error that names the line.

## The check after writing

After the text is made, the writers parse it with the s-expression reader of the crate and read it back with the reader of the same kind (the board reader for a board, the symbol reader for a library, the schematic reader for the first sheet). The number of footprints, pads with a net, track pieces, vias, zones and nets, or of parts, symbols and pins, is compared with what was written. A difference is a finding `PHOENIX-KICADX-006` and the exit code of the command is 1. This shows that the files are well formed and agree with the readers of this crate; it does not show that KiCad agrees.

## The command

```text
phoenix kicad-export [--settings FILE] [--lang xx] [--json] [--board FILE] [--out PATH]
                     <board|schematic|library> <input>
```

* The input is a JSON file of the design model, an EAGLE file or a KiCad file. `placa`, `esquema` and `biblioteca` are accepted for `board`, `schematic` and `library`.
* `library` takes a board (one `.kicad_mod` for each footprint) or a schematic (one `.kicad_sym`).
* `--board` gives the footprints a schematic refers to, so that the part gets `Library:Name` in its footprint field.
* A dry run unless `--out` is given: the files and the report are printed and nothing is written. `--out` is a folder, or the file itself when there is exactly one file of that kind to write. The input is never overwritten and neither is a file that exists (`PHOENIX-KICADX-005`, exit code 2); the files are created with the rule that they must not exist.
* `--lang` writes the titles of the findings in en, es, de, fr, it, ja, zh or pt; `--json` prints the report as JSON.

Exit codes: 0 done, 1 a file does not read back the same, 2 the arguments, the input or the output are unusable.

## Findings

The codes `PHOENIX-KICADX-001` to `PHOENIX-KICADX-007` are in `docs/ERROR_CODES.md`, with their titles in the eight languages in `eh3d-phoenix-kicad` (`export_title`).

## What is next

Opening the files in KiCad and running its checks on them, the comparison of the bottom side and of the arcs with what KiCad writes, the drawing of symbols and footprints, the filled copper of zones, and the other items in `docs/NOT_YET.md`.
