# EH3D-PHOENIX-PCB: Gerber X2 attributes in the export

Prototype. The attributes below are written from the specification of Gerber X2 as the author knows it, and the files have only been read back by the parsers of EH3D-CORE-LIBS and by the import of this repository (`docs/GERBER_IMPORT.md`). No viewer of a manufacturer, no CAM program and no assembler has opened them, and the names and values have not been compared line by line with the text of the specification. The check `phoenix gerber-check` is a subset chosen for this project, not a validator of the standard.

Nothing changes by default: the profile `generic` keeps the three file attributes this export always wrote. The rest is switched on by the section `gerber.x2` of an export profile (`config/export_profiles.json`, `docs/EXPORT.md`).

## Levels

`gerber.x2.level` says how much of X2 the files carry.

| Level | Layer files | Drill files |
|---|---|---|
| `legacy` (default) | `.GenerationSoftware`, `.FileFunction`, `.FilePolarity` | the same two comments that were always written |
| `x1` | no attribute of any kind | no attribute comment |
| `file` | every file attribute (below) | every file attribute as `; #@! TF...` comments |
| `aperture` | the file attributes and `TA.AperFunction` on each aperture | and `TA.AperFunction` before each tool |
| `full` | and the object attributes `TO` with `TD` to end them | the same as `aperture` |

The profiles `x2-full` and `x1` of `config/export_profiles.json` are the generic profile at the levels `full` and `x1`. The apertures are numbered from 10 in the order of their shapes and, from the level `aperture`, of their functions, so one shape with two functions is two apertures; the drill tools are numbered in the order of their diameters and functions the same way. The same board and profile always give the same bytes.

## File attributes (`TF`)

Written in this order from the level `file`:

| Attribute | Value |
|---|---|
| `.GenerationSoftware` | `EH3D,PHOENIX-PCB,<version of the crate>` |
| `.CreationDate` | only when `gerber.x2.creation_date` gives it (an ISO 8601 date and time with an offset); the files never read the clock |
| `.ProjectId` | the name of the board, a UUID and `gerber.x2.project_revision` (`1`); the UUID is `gerber.x2.project_id` when given, otherwise it is built from the SHA-256 of the name of the board (version bits 5, variant bits of RFC 4122), so the same name always gives the same UUID |
| `.SameCoordinates` | `gerber.x2.same_coordinates` (`Original`; empty for the attribute without an identifier): every file of one export shares the coordinates of the board |
| `.FileFunction` | `Copper,L<n>,Top\|Inr\|Bot`, `Soldermask,Top\|Bot`, `Legend,Top\|Bot`, `Paste,Top\|Bot`, `Profile,NP`; a drill file has `Plated,1,<n>,PTH,Drill` or `NonPlated,1,<n>,NPTH,Drill` (`Mixed` instead of `Drill` when the file holds slots), with `<n>` the number of copper layers |
| `.FilePolarity` | `Positive` for copper, legend and paste, `Negative` for solder mask; not for the outline nor for drill files |
| `.Part` | `Single` |

A function that needs a fact the board does not give is left out, as before (`PHOENIX-EXPORT-003`, `-008`, `-017`).

## Aperture attributes (`TA.AperFunction`)

From the level `aperture` each aperture is defined after the `TA` of its function, and a `TD.AperFunction` follows the last definition.

| Object | `.AperFunction` |
|---|---|
| Pad of a through-hole component (a drilled pad) | `ComponentPad` |
| Pad without a drill | `SMDPad,CuDef` |
| Pad whose number is in `gerber.x2.heatsink_pad_numbers` (`EP`) | `HeatsinkPad` |
| Pad of a footprint whose name holds a word of `gerber.x2.fiducial_names` (`fiducial`) | `FiducialPad,Local` or `FiducialPad,Global` (`gerber.x2.fiducial_scope`) |
| Pad of a via | `ViaPad` |
| Track | `Conductor` |
| Outline | `Profile` |

The strokes of the legend have no function. Pads on the solder mask and paste layers get the function of the pad. A zone has no aperture; it is a region and carries only object attributes. Not written because the board model has no such thing: test points, castellated pads, thermal reliefs, washers, anti-pads, copper balancing, the component outlines and the rest of the list of the specification.

In a drill file the tools carry `; #@! TA.AperFunction,Plated,PTH,ViaDrill` (a via), `...,Plated,PTH,ComponentDrill` (a drilled pad on copper) or `; #@! TA.AperFunction,NonPlated,NPTH,MechanicalDrill` (a drilled pad on no copper layer) before their definition, and `; #@! TD.AperFunction` after the last one. A via and a pad with the same diameter are two tools.

## Object attributes (`TO`, `TD`)

At the level `full`, on copper, solder mask and paste, in the order of `gerber.x2.object_attributes` (all three by default; an empty list writes none):

| Attribute | Where | Value |
|---|---|---|
| `.N` | copper: pads, tracks, vias, zones | the name of the net, `N/C` when there is none |
| `.P` | pads, on every layer they are on | the reference designator and the pad number |
| `.C` | pads, on every layer they are on | the reference designator |

An attribute is set only when it changes and deleted with `TD.N`, `TD.P` or `TD.C` when the next object does not have it; the last objects are followed by the deletes. `.CMfr` and `.CMPN` are not written: the board model has no manufacturer and no part number (see `docs/NOT_YET.md`).

## Text in a field

A field of an attribute may not hold a comma, an asterisk, a percent sign, a backslash, a control character or a character outside printable ASCII. By default such a character is written as `\uXXXX` (four hexadecimal digits) and a note says how many texts were escaped (`PHOENIX-EXPORT-019`); the import reads the escapes back as the characters. With `gerber.x2.strict` the export stops instead (`PHOENIX-EXPORT-020`, no file is written), and so it does for a character beyond the first plane of Unicode, which has no escape of this form. Plain names such as `GND`, `VCC_3V3` or `N/C` are never touched. Only the text that is written is checked: below the level `full` the net names are not in the files and cannot be refused.

## Reading back

The import (`docs/GERBER_IMPORT.md`) keeps the attributes of the apertures (`aperture_attributes`), the attributes of the tools of a drill file (`tool_attributes`) and, for each drawn object, the set of object attributes in force when it was drawn (`attribute_sets` and `item_sets`; each distinct set is kept once). `phoenix gerber-import --roundtrip --profile x2-full <board>` exports the board, reads every file back and compares the attributes with the board and the profile: the file attributes (none at all at `x1`), the function of every aperture and tool, and on copper, solder mask and paste the pin of every pad (place and net included), its component and the nets of vias, tracks and zones. A difference is `PHOENIX-GBR-016`. The tests assert that the round trip of the five levels reports no difference.

## The check

```
phoenix gerber-check [--config FILE] [--json] [--lang CODE] <file or folder> ...
```

It reads Gerber and Excellon files (an Excellon file is told by `M48` on its first line) and reports, by line, what it finds with the codes `PHOENIX-GX2-001` to `-010` in the eight languages (`docs/ERROR_CODES.md`): the end command (`M02`, `M30`) missing or not last, `.FileFunction` missing (an error) and the other file attributes missing (a warning), an aperture or a tool used without a definition, an aperture defined twice or below `D10`, a `TD` that deletes an attribute that was not set, the shape of the values of the attributes this suite writes, characters an attribute field may not hold, drawing before `FS` and `MO`, and a region opened and not closed. Exit code 0 is done with no error (warnings do not count), 1 a file has an error or could not be read, 2 the arguments are unusable. The files are only read.

## Panels

The layers of a panel of boards (`docs/PANEL.md`, `phoenix panel`) are written by this same export from a board made of the copies of the boards and the marks of the panel, so a profile with `gerber.x2.level` at `full` gives every pad, track, via and zone of every copy the net, the pin and the component it has in the board (the names of the nets and of the references carry the number of the copy when `copy_suffix` is on), the fiducials of the panel carry the aperture function of a fiducial pad, and the drill files of the panel carry the functions of their tools. The panel reads its files back with the import and compares the attributes as `--roundtrip` does. The outline and the V-score lines of the panel are written apart, as in `docs/PANEL.md`.

## What is not here

See `docs/NOT_YET.md`: the manufacturer and part number attributes, the attributes of components beyond the reference, the pin and the net, blind and buried vias in their own drill files, the aperture functions of the objects the board model lacks, object attributes in the drill files, and the comparison with the files of real CAD programs and with a viewer or CAM program of a manufacturer.
