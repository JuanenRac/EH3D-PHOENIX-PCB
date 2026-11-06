# Migration guide: coming from EAGLE or KiCad

This page sets the ideas and commands of EAGLE and KiCad beside those of this program, says what is read and written, and what is lost. The program is a prototype: nothing here has been compared with EAGLE or KiCad on real designs. Start with `phoenix help`.

## Equivalent commands

| What you do | In EAGLE | In KiCad | Here |
|---|---|---|---|
| Design rule check | DRC | Design Rules Checker | `phoenix check`, `phoenix check --advanced` |
| Electrical rule check | ERC | Electrical Rules Checker | `phoenix erc` |
| Board against schematic | Forward & Back Annotation | Update PCB from Schematic | `phoenix connectivity`, `phoenix sync`, `phoenix annotate` |
| Connections still to route | RATSNEST | ratsnest | `phoenix ratsnest` |
| Route | ROUTE, AUTO | interactive router, Freerouting | `phoenix route`, `phoenix boardedit`, `phoenix autoroute` |
| Manufacturing files | CAM Processor | Plot, Fabrication Outputs | `phoenix export`, `phoenix fabpack` |
| Bill of materials and placement | ULP | BOM and position files | `phoenix bom`, `phoenix pnp`, `phoenix stencil` |
| Panel | ULP | KiKit and similar | `phoenix panel` |
| Run a script | SCRIPT, RUN | Python console | `phoenix script run` |
| Edit a library | Library editor | Symbol and Footprint editors | `phoenix libedit`, `phoenix footprint`, `phoenix ipc` |
| Simulate | SPICE export | ngspice | `phoenix netlist`, `phoenix sim` |

## What is read and written

| File | Read | Written |
|---|---|---|
| EAGLE `.sch`, `.brd`, `.lbr` (XML) | `eagle-check` and every command that takes a board or a schematic | boards and schematics as EAGLE XML: `boardedit`, `sync --apply`, `annotate --apply`; libraries: `libedit`, `footprint`, `ipc` |
| EAGLE binary | no | no |
| EAGLE `.dru` | `check --rules-dru`, `eagle-convert dru-to-rules` | `eagle-convert rules-to-dru` |
| EAGLE `.cam` | `cam-import`, `eagle-convert cam-to-profile` | - |
| EAGLE `.scr` | `script run`, `script report` | - |
| EAGLE ULP | no | - |
| KiCad 6 - 9 `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_sym`, `.kicad_mod` | `kicad-read`, `kicad-import`; `.kicad_pcb` and `.kicad_sch` also in the commands that take a board or a schematic | KiCad 8: `kicad-export` |
| Gerber X2, Excellon | `gerber-import`, `gerber-check` | `export`, `fabpack`, `panel` |
| DXF R12 | `dxf-import` | `dxf-export` |
| Specctra DSN, SES | SES: `autoroute` | DSN: `autoroute` |

## What is lost

* The design model does not hold KiCad graphics, buses, no-connect flags, the setup of a board, the filled copper of zones, keep-out zones or 3D models; the readers count them as left out and say so in the report.
* EAGLE: the binary files are not read, ULP programs do not run, and the elements that the model simplifies are listed by `phoenix eagle-check` with codes in eight languages.
* Nothing is written over an input: a conversion always makes a new file.

## First steps

1. Read your project with `phoenix kicad-import` (KiCad) or `phoenix eagle-check` (EAGLE) and read the report.
2. Check it with `phoenix check --rules FILE` (the limits of your rules, or `phoenix eagle-convert dru-to-rules` for an EAGLE rule file) and `phoenix erc`.
3. Write the manufacturing files with `phoenix export` and compare them with the ones of your usual tool before you trust them.

## See also

* [docs/KICAD_IMPORT.md](KICAD_IMPORT.md)
* [docs/KICAD_EXPORT.md](KICAD_EXPORT.md)
* [docs/EAGLE_DEEP.md](EAGLE_DEEP.md)
* [docs/SCRIPT.md](SCRIPT.md)
* [docs/EXPORT.md](EXPORT.md)
* [Glossary](GLOSSARY.md)
* [Tutorials](tutorials/README.md)

## Languages

* [Español](migration/es/MIGRATION.md)
* [Deutsch](migration/de/MIGRATION.md)
* [Français](migration/fr/MIGRATION.md)
* [Italiano](migration/it/MIGRATION.md)
* [日本語](migration/ja/MIGRATION.md)
* [中文](migration/zh/MIGRATION.md)
* [Português (Portugal)](migration/pt/MIGRATION.md)
