# EH3D-PHOENIX-PCB: reading a whole KiCad project

`phoenix kicad-import` reads a KiCad 6 to 9 project (the project file, the schematic with its sheets and the board) into the same design model as the EAGLE files, with one report for the set. It is built on the readers of `docs/KICAD.md`, which say what is read from each format; this page is about the project as a whole and the command. It only reads: nothing is written back to KiCad (PPCB-013 stays planned).

**State: prototype.** The reading was tried on one small project written by hand for the tests (`crates/eh3d-phoenix-cli/tests/kicad_files/`), whose values were worked out by hand, and on damaged copies of it. It has not been tried on any project drawn in KiCad, so nothing here is `validated`. Where a rule says what KiCad does, it is the reading of the public description of the formats.

## The command

```text
phoenix kicad-import [--settings FILE] [--lang xx] [--json] [--out FILE.json] <project>
```

`<project>` is one of:

| Input | What is read |
|---|---|
| `name.kicad_pro` | the project file, `name.kicad_sch` (with the sheets below its folder) and `name.kicad_pcb`, the two last taken from the same folder with the same name |
| a folder | the same, when the folder holds exactly one `.kicad_pro`; with none or with several the command asks for a file |
| `name.kicad_sch` | the schematic alone, with the sheets it refers to that lie below its folder |
| `name.kicad_pcb` | the board alone |

It is a **dry run**: it prints the report and writes nothing. `--out FILE.json` writes the models as new files and never writes over an input or over a file that exists:

* for a single schematic or board, `FILE.json` holds the model (the JSON of the board or the schematic, the file the other commands read);
* for a project, `FILE.board.json` and `FILE.schematic.json` are written beside the name given (the part that could not be read is listed as not written);
* the name must end in `.json`; anything else, an input, or a name that exists is refused before anything is read, with `PHOENIX-KICAD-016` and exit code 2.

`--json` prints one document (`phoenix-kicad-import/1`) with the board and the schematic models, the net classes and rules of the project file in nanometres, the report of each file and the findings; the same input gives the same document. `--settings FILE` takes the limits of the readers (`config/kicad.cfg`) and `--lang` writes the titles of the findings in en, es, de, fr, it, ja, zh or pt.

Exit codes: 0 something was read (the report says what was changed or left out), 1 nothing could be read, 2 the arguments, the files or the output are unusable.

## The report

For each file that was read the report has three lists, as in `docs/KICAD.md`: what was kept, what was changed and what was left out, each with its reason in its name. The elements the readers do not interpret are counted by name (`PHOENIX-KICAD-006`); nothing is dropped without a count. Beyond the readers, the set adds:

| Finding | When |
|---|---|
| `PHOENIX-KICAD-015` | the schematic or the board of the project is not next to it (the rest is read) |
| `PHOENIX-KICAD-017` | both were read and the references of the parts differ; the finding names the references on one side only (up to ten, then a count). A reference that is not annotated (`R?`) is not compared |
| the finding of the part | a file that cannot be read is listed as `not readable` with its own finding (for example a damaged project file), and the others are still read |

The **fidelity level** of the set is `complete` when every file was read and nothing was changed, left out, missing or different; `partial` when something was (a board always has its setup left out, so a board is `partial` today); and `unusable` when no file could be read.

## What a project gives that a single file does not

* The net classes and the numeric rules of the project file, next to the models, in nanometres. They are not yet turned into the rule file of the checks (`docs/NOT_YET.md`): `phoenix check` on a `.kicad_pcb` file needs `--rules`.
* The comparison of the parts of the schematic and the board.
* One sequence of ids for both models, so no id is used twice.

## Buses

A sheet symbol of the schematic is kept as a sheet symbol of the model (its name, corner and size, a pin for each pin with its shape as direction), the file it names is the sheet it shows and each hierarchical label of that file is a port (`docs/HIERARCHY.md`); the nets are still computed for the whole design as described here.

A bus is not a net: the members of a bus join through the labels of their wires, which the reader already joins by name. A label that names a bus (`D[0..7]`) or a group of signals (`USB{DP DM}`) would only make a net with a wrong name, so it is not read as a net: it becomes a `Bus` of the sheet (`docs/BUSES.md`), expanded to its members (`D0` to `D7`, `USB.DP` and `USB.DM`), and is counted under `buses`. A `bus_entry` whose far end meets a net label with the name of a member (or a wire that ends there and has such a label at its other end) becomes a `BusEntry` of that bus (`bus entries`). What is reported and not read: a label that names a bus that cannot be expanded (`bus labels that cannot be expanded`), an entry with no label of a member at its end (`bus entries with no net label at their end`), an entry whose member is in no bus of the sheet (`bus entries whose member is in no bus of the sheet`), and the wires of the buses, which the model does not draw (`` `bus` elements ``). Members that are joined only through a bus label between sheets are therefore not joined; this is listed in `docs/NOT_YET.md`.

## Other commands

The commands that take a board read a `.kicad_pcb` file too, with the default limits of the readers: `export`, `route`, `ratsnest`, `autoroute`, `boardedit`, `zones`, `lengths`, `bom`, `assembly3d`, `panel`, `rfgen`, `script`, `gerber-import` and the commands that read the board through the same function; `erc`, `sync` and `schedit` read a `.kicad_sch` file. `phoenix check` reads a `.kicad_pcb` file only with `--rules`, because a KiCad board gives no design rules to that command. `phoenix connectivity` reads EAGLE files only. The ids of these reads are made in order, so the same file gives the same board.

## The round trip through the manufacturing files

The test of the command takes the board of the small project, makes the manufacturing files with the export of this repository and reads them back with the Gerber and Excellon import (`docs/GERBER_IMPORT.md`). The two agree: the flashes of the pads and the via, the line of the track, the outline and the holes. By hand: the top copper has five flashes (two round pads, two rectangular pads and the via) and one line, and the plated drill file has three holes (two through-hole pads and the via). This proves that the board read from KiCad is a board the export can use; it does not prove that KiCad would draw it the same way.

## Limits

Every file is hostile until proven otherwise: the limits of size, depth, nodes, text, items, sheets and findings of `docs/KICAD.md` apply to each file of the project, and a sheet is only opened from below the folder of the schematic. A folder is not searched beyond its own files.

## What is not done

The writing of KiCad files, the comparison with KiCad on a public corpus, the trapezoid, chamfered and custom shapes of pads, the filled copper of zones, the members of buses between sheets and the rules of the project as a rule file are in `docs/NOT_YET.md`.
