# EH3D-PHOENIX-PCB: manufacturing export

The crate `eh3d-phoenix-export` turns a board into the files a manufacturer and an assembler ask for, and the command `phoenix export` runs it. It is a prototype: the files have been read back only by the parsers of this repository (EH3D-CORE-LIBS), never opened in two independent viewers and never sent to a manufacturer (PPCB-009, PPCB-010 and PPCB-011 in `REQUIREMENTS.md`).

## What it writes

| File | Content |
|---|---|
| `NN_<layer>.gbr` | One Gerber X2 file per layer of the board, in the order copper, solder mask, paste, legend, profile, documentation |
| `drill_pth.drl` | Excellon file of the plated holes: vias and the drilled pads that appear on a copper layer |
| `drill_npth.drl` | Excellon file of the holes that are not plated: drilled pads that appear on no copper layer (mounting holes) |
| `placement.csv` | Pick and place: reference, value, package, X, Y, rotation, side |
| `bom.csv` | Bill of materials grouped by value and package, with the designators and the quantity |
| `manifest.json` | The SHA-256 and the size of every other file, as canonical JSON (the integrity record of EH3D-COMMON) |

A file name and a column come from the profile; the names above are the defaults. A file that has nothing to hold (no hole of its kind) is not written. A board without components still gets the two CSV files, with only their header, and a note.

## Gerber X2

Every layer file has `.GenerationSoftware` (`EH3D,PHOENIX-PCB,<version of the crate>`), `.FileFunction` and, for copper, solder mask, paste and legend, `.FilePolarity`. This is the level `legacy` of the section `gerber.x2` of a profile and it is the default; the levels `x1` (no attribute), `file`, `aperture` and `full` (the file, aperture and object attributes, with the net, pin and component of every object) are described in `docs/GERBER_X2.md`.

| Layer | `.FileFunction` | `.FilePolarity` |
|---|---|---|
| Copper | `Copper,L<n>,Top\|Inr\|Bot` from the order of `copper_stack` | Positive |
| Solder mask | `Soldermask,Top\|Bot` from the side of the layer | Negative |
| Paste | `Paste,Top\|Bot` | Positive |
| Silkscreen | `Legend,Top\|Bot` | Positive |
| Outline | `Profile,NP` | not written |

A function that needs a fact the board does not give (the order of the copper layers, the side of a mask, paste or legend layer) is left out and a note says so (`PHOENIX-EXPORT-003`, `-008`, `-017`).

What is drawn:

* **Copper:** zones as a region of their outline (the filled copper of a zone goes as one region for each polygon through the board that the board editor pours, `docs/BOARD_ZONES.md`), tracks with lines and arcs (an arc is written with its centre offset and direction), vias and pads flashed (a via has a pad on both of its end layers and on every copper layer between them in the stack-up of the board, so a through via has one on the inner layers too).
* **Pads:** circle, rectangle and obround apertures. A pad turned by a multiple of 90 degrees swaps its sides; any other angle is flashed unrotated and reported (`PHOENIX-EXPORT-001`). The board model has no polygon pad, so none is written.
* **Solder mask and paste:** the pads of the layer, larger by `gerber.mask_expansion_nm` on each side in the mask and smaller by `gerber.paste_shrink_nm` in the paste. An opening is never smaller than 1 micrometre, so no pad is lost from a file. Vias are covered (tented).
* **Legend:** the strokes of the text and lines of the silkscreen layer, from the built-in stroke font.
* **Profile:** the outline polygon as lines of `gerber.outline_width_nm`.
* **Apertures** are numbered from 10 in the order of their sorted shapes, so there is one definition per shape.

Not done: copper pours made by the export itself (a zone is its outline, `PHOENIX-EXPORT-002`; the poured board of `docs/BOARD_ZONES.md` carries the polygons of a fill), clearances, the drill files of blind and buried vias (their pads are written on every copper layer they span, but their holes go into the one plated drill file as through holes), the component attributes beyond the reference, the pin and the net, and a polygon pad.

## Excellon

Units are millimetres and the style is chosen in the profile: a decimal point with 1 to 8 decimals, or a fixed format of 1 to 8 integer and 1 to 8 decimal digits (announced in a `FILE_FORMAT` comment). Tools are numbered from 1 in the order of their diameters; the holes are ordered by tool, then Y, then X. Each file starts with comments in the X2 form for Excellon: `.GenerationSoftware` and `.FileFunction` (`Plated,1,<n>,PTH` or `NonPlated,1,<n>,NPTH`, with `<n>` the number of copper layers).

A drilled pad is plated when its pad appears on a copper layer. With `drill.oblong_as_slot` a drilled oblong pad is written as a slot (`G85`) of the tool of its drill: the slot is as long as the pad is long in proportion to the drill (long side times drill over short side, rounded down to a nanometre), centred on the pad and along its long side. A pad that is not turned by a multiple of 90 degrees stays a round hole (`PHOENIX-EXPORT-016`). Without the option an oblong pad is a round hole (`PHOENIX-EXPORT-006`).

## Placement and bill of materials

Both are plain text with one line per part or group, in the natural order of the designators (`R2` before `R10`), lines ending in a line feed. A field with the delimiter, a quote, or a space at either end is quoted; a line break inside a field becomes a space.

* **Coordinates:** the origin of the component in millimetres, from the origin of the board or, with `placement.origin = outline_min`, from the lower left corner of the bounding box of the outline.
* **Rotation:** the rotation of the component in degrees, from 0 up to a full turn. For a component on the bottom side `placement.bottom_rotation` says what is written: `keep` (the default, the rotation as it is), `mirror` (360 minus the rotation) or `half_turn` (the rotation plus 180). Which one an assembler wants has to be confirmed with its own sample; the rule is a setting, not a belief of the code.
* **Rounding:** nanometres and micro-degrees are shown by integer arithmetic, rounded half away from zero to `placement.coordinate_decimals` and `placement.rotation_decimals`.
* **Columns:** `placement.columns` and `bom.columns` list, in order, a key and the text of its header. The keys of the placement are `ref`, `value`, `package`, `x`, `y`, `rotation` and `side`; those of the bill of materials are `designators`, `quantity`, `value` and `package`.

The files above are the simple ones of a profile. The pick and place files with the centre of each part, the extra rotations of a machine, the origin and the unit (`docs/PNP.md`), the solder paste stencil (`docs/STENCIL.md`), the bills of the assembly by face (`docs/BOM.md`) and a deterministic zip file with a manifest of SHA-256 sums that holds all of them with the Gerber and Excellon files (`docs/FABPACK.md`) are made on top of this export by `phoenix pnp`, `phoenix stencil` and `phoenix fabpack`.

## Profiles

A profile file is JSON: `{"format": "phoenix-export/1", "profiles": [...]}`. `config/export_profiles.json` holds `generic`, `jlc-like` (shaped after the files low-cost assemblers ask for and not compared with any manufacturer), `x2-full` (the generic profile with every Gerber X2 attribute) and `x1` (the generic profile without any attribute). A missing field takes the default of its profile section; an unknown field, a value out of its range or a file name that is not a plain name is an error (`PHOENIX-EXPORT-011`). Lengths are integer nanometres.

| Section | Field | Meaning (default) |
|---|---|---|
| `gerber` | `extension`, `decimal_digits` | Extension of the layer files (`gbr`) and digits after the point, 1 to 6 (6) |
| | `mask_expansion_nm`, `paste_shrink_nm` | Opening past the pad in the mask (50000) and inside the pad in the paste (0), per side, within 1 mm |
| | `outline_width_nm` | Width of the line of the profile (100000) |
| `gerber.x2` | `level`, `object_attributes`, `strict` | `legacy`, `x1`, `file`, `aperture` or `full` (`legacy`); the object attributes `net`, `pin`, `component` of the level `full` (all three); refuse instead of escaping text a field cannot hold (false) |
| | `creation_date`, `project_id`, `project_revision`, `same_coordinates` | `.CreationDate` (not written unless given), the UUID of `.ProjectId` (derived from the name of the board), its revision (`1`) and the identifier of `.SameCoordinates` (`Original`) |
| | `fiducial_names`, `fiducial_scope`, `heatsink_pad_numbers` | Words of a footprint name that make its pads fiducials (`fiducial`), `local` or `global` (`local`) and the pad numbers that are heatsink pads (`EP`) |
| `drill` | `style`, `decimals` | `decimal` or `fixed`, and the decimals of the decimal style (`decimal`, 6) |
| | `fixed_integer_digits`, `fixed_decimal_digits` | The fixed format (3 and 3) |
| | `plated_file`, `non_plated_file`, `oblong_as_slot` | File names and slots for oblong pads (`drill_pth.drl`, `drill_npth.drl`, false) |
| `placement` | `file`, `delimiter` | `placement.csv` and `,` |
| | `coordinate_decimals`, `rotation_decimals` | 4 and 2, each 0 to 6 |
| | `bottom_rotation`, `origin` | `keep` and `absolute` |
| | `side_top`, `side_bottom`, `columns` | Texts for the sides (`top`, `bottom`) and the columns |
| `bom` | `file`, `delimiter`, `designator_separator`, `columns` | `bom.csv`, `,`, a space and the columns |

## Check by reading back

`verify` reads every file back with the parsers of EH3D-CORE-LIBS and compares it with the model, so that a loss shows in a test and not at the manufacturer:

* every layer: the flashes against the pads (and, on copper, the vias), on copper also the regions against the zones and the lines and arcs outside the regions against the track segments;
* the profile: the number of lines and the bounding box against the outline of the board, within half of the last digit the file keeps;
* the drill files: the holes and slots against the vias and the drilled pads, the declared units and the order of the tools by diameter; a hole kind that has no file is a difference;
* the placement file: one line per component with the designators of the board; the bill of materials: quantities that add up to the number of components.

A file that cannot be read is `PHOENIX-EXPORT-018`; a difference is `PHOENIX-EXPORT-014`. Silkscreen and documentation layers are only read back, because their strokes depend on the font. `phoenix export` runs the check before it writes anything and exits with 1 when the files differ.

## Package and folder

`build` makes the files in memory (`FabSet`). `FabSet::write_folder` writes them flat to a folder with `manifest.json`; a file that is already in the folder (a link included) is never replaced, and the call fails before it writes anything. `export_package_with` puts them in an `.eh3dproj` 1.0.0 package under `layers/` (layers and drill files are named in the metadata; the CSV files are entries without a layer) with the identity and the time stamp given by the caller; `export_package_reproducible` takes the identity from the first 128 bits of the SHA-256 of the manifest and the start of the Unix epoch as the time stamp, so the same board and profile give the same bytes anywhere. Nothing reads the clock or makes a random number.

## Command

```
phoenix export [--profile NAME] [--profiles FILE] [--lang CODE] [--eh3dproj FILE] --out <folder> <board>
```

`<board>` is an EAGLE board (`.brd`) or the JSON board of a project (`.json`). `--profiles` defaults to `config/export_profiles.json` when that file exists; without a file only `generic` (the defaults) is known. `--lang` writes the messages in en, es, de, fr, it, ja, zh or pt. Exit codes: 0 done, 1 the check found a difference (nothing is written), 2 an argument, the board, the profile or the output folder is unusable.

## Codes

`PHOENIX-EXPORT-001` to `PHOENIX-EXPORT-020`, with their titles in the eight languages (`messages.rs`) and in `ERROR_CODES.md`. A message is built from the title and a detail; the detail is in English.

## Rounding and units

The model holds integer nanometres and micro-degrees. A length becomes millimetres in floating point only at the moment of writing, and the writers of EH3D-CORE-LIBS round it half to even to the decimals of the format; with six decimals a nanometre is kept exactly. The length of a slot, the sizes of a mask or paste opening and every CSV value are computed with integers, as described above.
