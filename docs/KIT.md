# EH3D-PHOENIX-PCB: the starter kit

The folder `kit/` holds what is needed to try the program and to start a design: libraries, sample boards and schematics, user programs, rule files and output profiles. Every release carries it as its own package, `eh3d-phoenix-pcb-kit-<version>.zip` with its `.sha256`, next to the packages of the program.

**Licence and origin.** Everything in the kit was made for this project and is under GPL-3.0-or-later, like the program. No file, drawing or table of another program, maker or collection was read or copied. The sizes are nominal figures of public standards and of the usual 2.54 mm and 5.0 mm grids, and the land patterns were worked out from them for this kit.

**State: not verified.** Nothing of the kit was compared with a datasheet, ordered from a board maker or soldered, and no file of it was opened in another program. In particular, the footprints have not been checked against datasheets, and no library, board or schematic of the kit has been opened in EAGLE or in KiCad. The catalogue marks every piece with `verified: false`. Check the footprints against the datasheet of your part and the figures of the profiles against the sheet of your maker before you order anything.

## Get it

Unpack the package next to the program `phoenix`: it makes a folder `kit`, which `phoenix kit` finds on its own. In a copy of the repository, run the commands from its root.

```
phoenix kit list [--type TYPE] [--lang xx] [--json]
phoenix kit info [--lang xx] [--json] <identifier>
phoenix kit copy [--all] --out FOLDER [<identifier>...]
```

`list` shows the pieces (`TYPE` is `library`, `board`, `schematic`, `program`, `rules`, `dfm` or `export-profile`), `info` the licence, the origin, the formats and the files with their SHA-256 of one piece, and `copy` copies pieces under `--out` with the folders of the kit. A copy checks the SHA-256 of every file first, writes new files only and copies nothing when one destination is already there. `--kit FOLDER` names another kit, `--config` reads the limits (`config/kit.cfg`). The exit code is 0 when done and 2 otherwise, with a code `PHOENIX-KIT-<NNN>` (`docs/ERROR_CODES.md`).

## What is in it

| Folder | Pieces | Format |
|---|---|---|
| `libraries/` | `passives` (chip 0402, 0603, 0805 and 1206, axial resistors, disc and electrolytic capacitors), `semiconductors` (chip and through hole LEDs, SOD-123 and SOD-323 diodes, SOT-23 transistors, a SOT-223 regulator), `ics` (generic SOIC-8, 14 and 16 and TSSOP-14, 16 and 20), `connectors` (pin headers of one and two rows, screw terminals of 2 and 3 poles), `modules` (a module of the ESP32 type and one of the STM32 type) | EAGLE XML `.lbr` |
| `boards/` and `schematics/` | 12 small synthetic designs, a board and a schematic of each: LED, voltage divider, LED bar, transistor switch, 8 pin circuit, TSSOP-14 and SOIC-16 breakouts, through hole LEDs, protected power input, linear regulator, carriers for the two module types | EAGLE XML `.brd` and `.sch` |
| `ulp/` | 20 user programs, each with the output it prints on one of the designs (`.expected`) | `.ulp` |
| `rules/` | two rule files (one with a class for the supply nets) and four capability profiles (two layers, two layers fine, four layers, made by hand) | `phoenix-rules`, `.cfg` of `check --dfm` |
| `cam/` | two output profiles: Gerber and drill, and the files for assembly | `phoenix-export` |
| `manifest.json` | the catalogue: identifier, name, type, format, licence, origin, whether it is verified, the formats it can be exported to, the titles in eight languages and the files with their SHA-256 | `phoenix-kit/1` |

The two module footprints are **indicative**: a generic module on two pin strips (2 x 15 pins, rows 25.4 mm apart, and 2 x 20 pins, rows 15.24 mm apart). They are not the drawing of any product; measure your module.

## Try it

```
phoenix kit list
phoenix kit copy --out my-kit --all
phoenix check my-kit/boards/led_indicator.brd
phoenix check --dfm my-kit/rules/two_layer_standard.cfg my-kit/boards/led_indicator.brd
phoenix connectivity my-kit/schematics/led_indicator.sch my-kit/boards/led_indicator.brd
phoenix erc my-kit/schematics/led_indicator.sch my-kit/boards/led_indicator.brd
phoenix export --profiles my-kit/cam/gerber_basic.json --profile kit-gerber-basic --out my-gerbers my-kit/boards/led_indicator.brd
phoenix kicad-export board my-kit/boards/led_indicator.brd
phoenix ulp run --board my-kit/boards/led_bar.brd my-kit/ulp/bom_by_value.ulp
```

The designs are exported to KiCad with `phoenix kicad-export` (a board, a schematic, or their footprints and symbols as libraries) and to Gerber, Excellon and IPC-2581 with `phoenix export` and `phoenix ipc2581`. A `.lbr` of the kit has no direct conversion to a KiCad library.

## How it is made

`tools/make_kit.py` writes the libraries, the designs, the expected outputs and the catalogue; the sizes and their sources are named in that script and in the description of each package. Its tracks come from a plain grid search of its own. The user programs, the rule files and the output profiles are written by hand. `python tools/make_kit.py --check` fails when a file of the kit is not what the script writes or is not in the catalogue.

The tests (`crates/eh3d-phoenix-cli/tests/kit.rs` and `tools/test_make_kit.py`) check that every library passes the library check and reads back the same, that every board passes the rule check and every schematic the electrical rule check, that each board agrees with its schematic and is fully routed, that both read back the same after being written, that the profiles check and export them, and that every user program prints its expected output.

## Limits

* The designs are small and synthetic; they show the file formats and the commands, not good practice of layout for a product.
* The pads are the program's own land patterns, not those of IPC-7351 or of a maker; `phoenix footprint` and `phoenix ipc` make others.
* The packages of a board file carry pads only: the silkscreen of the footprints is in the libraries.
* The figures of the profiles are generic round numbers and belong to no maker.
