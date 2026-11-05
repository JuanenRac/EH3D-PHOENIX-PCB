# EH3D-PHOENIX-PCB: import of Gerber and Excellon files

The crate `eh3d-phoenix-gerber` and the command `phoenix gerber-import` read manufacturing files for review: Gerber RS-274X layers and Excellon drill files. What they read is kept as read-only geometry, apart from the board model: it is never merged into a board and never written back. The same crate compares the files that the export of this suite writes for a board with that board (the round trip).

**State: prototype.** It has been tried on files written by the tests and on the files that the export of this suite writes, never on files from a CAD program or from a manufacturer, and the geometry has never been compared with a reference viewer. Nothing is shown in the window of the viewer yet.

## What is read

* **Gerber.** The format (`FS`, absolute coordinates, either zero omission, 1 to 8 digits) and the units (`MO`, and the old `G70` and `G71`); standard apertures `C`, `R`, `O` and `P` with their holes; macro apertures (`AM`) with variables and expressions (`+ - x / ( )`, `$n`, `$n=`) and the primitives circle (1), vector line (20), centre line (21), lower-left line (22), outline (4) and polygon (5); the polarity (`LPD`, `LPC`) kept on each object; lines, flashes and arcs (`G01`, `G02`, `G03`, with `G75` multi quadrant and `G74` single quadrant); regions (`G36`, `G37`) with several contours and arcs in them; the file attributes (`TF`, so `.FileFunction`); coordinates with a decimal point. Everything is converted to integer nanometres with `i128` arithmetic, so a coordinate that fits the declared format is exact.
* **Excellon.** The units (`METRIC`, `INCH`, `M71`, `M72`), the zeros that are left out (`LZ`, `TZ`) and the digits (a `FILE_FORMAT=i:d` comment or a pattern such as `000.000`), tools with their diameter (`T01C0.8`), holes, slots (`G85`) and routed paths (`G00`, `M15`, `G01`, `M16`) as slots, and the `TF.FileFunction` attribute that a comment may carry. A file that does not say its units, its zeros or its digits is read with the defaults of `config/gerber_import.cfg` and a finding.
* **Outline files.** A profile layer is an ordinary Gerber file; it is told from the others by its attribute or its name, and the round trip compares its lines and its bounding box with the outline of the board.

## What each file is

The layer comes from the `.FileFunction` attribute when the file has one (`Copper,L1,Top`, `Soldermask,Bot`, `Paste`, `Legend`, `Profile`, `Plated`, `NonPlated`), and otherwise from the name: the extensions of Protel (`.gtl`, `.gbl`, `.gts`, `.gbs`, `.gto`, `.gbo`, `.gtp`, `.gbp`, `.gko`, `.gm1`, `.g1` to `.g30`), of EAGLE (`.cmp`, `.sol`, `.stc`, `.sts`, `.plc`, `.pls`, `.crc`, `.crs`, `.ly2` to `.ly30`, `.drd`) and the names KiCad writes (`-F_Cu`, `-B_Cu`, `-In2_Cu`, `-F_Mask`, `-B_Paste`, `-F_Silkscreen`, `-Edge_Cuts`, `-PTH`, `-NPTH`). A file whose layer cannot be told is read all the same, with `PHOENIX-GBR-014`; a name that says a drill file over Gerber content, or the reverse, is a warning.

## What is not read

Step and repeat (`SR`), aperture blocks (`AB`) and aperture transforms (`LM`, `LR`, `LS`), negative image polarity and the obsolete image commands, the thermal and moire primitives of a macro, incremental coordinates, and arcs and repeats in a drill file are skipped with `PHOENIX-GBR-010` and the layer is marked incomplete. A line drawn with an aperture that is not round takes the smaller side of the aperture as its width. Arcs inside a region are replaced by straight pieces that stay within `arc_chord_nm` of the curve. The geometry is not drawn, filled or merged: a flash is a point and an aperture, a line is two points and a width.

## Limits and errors

`config/gerber_import.cfg` holds the limits: the size of a file and the number of files, the commands, the apertures and tools, the drawn objects, the points of the regions, the primitives and the length of a macro, and the largest coordinate. A file over a limit is `PHOENIX-GBR-002` or `PHOENIX-GBR-003` and is not read further. A file that cannot be read gives an error with a stable code and its line; in a set of files the others are still read. The codes `PHOENIX-GBR-001` to `019` are in `docs/ERROR_CODES.md` with their titles in the eight languages. Text that is not valid UTF-8 is read with its bad bytes replaced.

## The round trip

`round_trip(board, profile, settings)` writes the files of a board with the export, reads each layer and drill file back with this import and compares:

* the layer told by the attribute of each file with the kind of the layer;
* on copper, the positions of the flashes (pads and vias), the number of regions (zones with three points or more), lines and arcs of the tracks; on solder mask and paste, the positions of the flashes of the pads;
* on the outline, the number of lines and the bounding box;
* in the drill files, the units, the number of holes, and the diameters and positions of the holes (as long as no slot is in the file); and that a file exists for every kind of hole the board has.

Positions are compared within `compare_tolerance_nm`, so a coarser coordinate format is not a difference. Silkscreen and documentation layers are only read, because their strokes depend on the font. A difference is `PHOENIX-GBR-016`; a file of the export that cannot be read back is a difference too. `compare_files` does the same for files that were made from a board and changed afterwards.

## The command

```
phoenix gerber-import [--config FILE] [--json] [--lang CODE] [--out FILE] <file or folder> ...
phoenix gerber-import [--config FILE] [--json] [--lang CODE] [--profile NAME] [--profiles FILE]
                      --roundtrip <board>
```

A folder is read file by file, not below. The files are only read: the summary goes to the screen, and `--out` writes the JSON summary to a new file (an existing file, and any input, is refused). With `--roundtrip` the board is exported in memory and nothing is written. Exit code 0 is done, 1 a file could not be read or the round trip found a difference, 2 the arguments, the settings or the output are unusable.

## What is not here

See `docs/NOT_YET.md`: showing the imported layers in the viewer, the step and repeat and aperture blocks, a comparison with a reference viewer and with files of real CAD programs, and the comparison of copper pours and silkscreen text.
