# EAGLE in depth

Crate `eh3d-phoenix-eagle-deep`, commands `phoenix eagle-check` and `phoenix eagle-convert`, options `--rules-dru` of `phoenix check` and `--variant` of `phoenix bom` and `phoenix pnp`, settings `config/eagle_layers.cfg`, codes `PHOENIX-EAGLEX-001` to `024` (`ERROR_CODES.md`, with the titles in the eight languages).

**State: prototype.** Every format here is written from its documentation as the author knows it. It has been tried on small files written for the tests (`tests/corpus/eagle` and the folder `tests/data` of the crate) and on nothing else: no file saved by EAGLE, no real board, rule file or CAM job of a user, and no result of EAGLE has been compared with it. Where the author was not sure of a form (the CAM job), the reader accepts three and says so.

## The principle: the tree keeps everything

An EAGLE file is read into the XML tree of `eh3d-phoenix-eagle` and written back from the same tree, so what is read and written is the same file whatever this crate does or does not understand. The records of this crate (variants, attributes, layers, entities) are views on the tree; they keep every attribute of the elements they cover, and what they do not cover stays in the tree as it is. `phoenix eagle-check` proves it for a file: it writes the tree, reads the text again and compares, and compares the text of the nodes that nothing covers.

The design model of a board or a schematic (`eh3d-phoenix-board`, `eh3d-phoenix-schematic`) is smaller than the format. Reading a file into it and writing the model back is the way that loses things, and the report names them (below).

## Design rules (`.dru`)

`DruFile` is a list of lines: blank lines, comments, `description[xx] = text` and `key = value` parameters, in the order of the file. `to_text` writes them and reading that text gives the same lines. A file larger than 1 MiB, with more than 20000 lines, with a NUL byte or with a line that is not `key = value` is refused (`PHOENIX-EAGLEX-001`).

Lengths are `mil`, `mm`, `inch` or `micron` (a bare number is millimetres), read exactly in nanometres; `nm_text` writes them back in millimetres with up to six decimals.

| EAGLE parameter | Rule of this tool |
|---|---|
| `msWidth` | minimum track width |
| `msDrill` | minimum drill |
| `mdCopperDimension` | distance from copper to the board edge |
| `mdDrill` | distance between drills |
| the nine `mdWireWire`, `mdWirePad`, `mdWireVia`, `mdPadPad`, `mdPadVia`, `mdViaVia`, `mdSmdPad`, `mdSmdVia`, `mdSmdSmd` | one copper clearance: the largest (`PHOENIX-EAGLEX-004` when they differ) |
| the five `rlMinPadTop`, `rlMinPadInner`, `rlMinPadBottom`, `rlMinViaOuter`, `rlMinViaInner` | one annular ring: the largest |

Every other parameter that EAGLE has (the layer set-up, the thicknesses, the ratios `rv...`, the maximum rings, the shapes of the pads `ps...`, the mask and cream frames, the thermals, the differential pair settings and the `check...` switches) is named in one finding (`PHOENIX-EAGLEX-003`), a key that is not an EAGLE parameter in another (`002`) and a value that is not a length in a third (`005`). Nothing is dropped without a finding.

The other way, `rules_to_dru`, writes the rules that are not zero (zero is off). With `--base FILE.dru` the lines of that file are kept and only the parameters the rules give are set, because an EAGLE file has many parameters these rules do not have. The rules that EAGLE has no parameter for (the size of vias, the bridge of the mask, the tolerance of connections, the two checks of the silkscreen and the net classes, which EAGLE keeps in the board file) are named (`PHOENIX-EAGLEX-006`). The `<designrules>` of a board are the same lines in XML (`DruFile::from_designrules`, `to_designrules`).

`phoenix check --rules-dru FILE <board.brd>` runs the rules of a `.dru` instead of the JSON rule file (`--rules`) or the `<designrules>` of the board; the findings of the conversion are printed as notes. Giving both `--rules` and `--rules-dru` is refused, and so is `--rules-dru` for `connectivity`.

## CAM jobs (`.cam`)

A job is read from any of three forms: the text of the old versions (`[CAM Processor Job]` and one `[Section]` for each output file with `Device=`, `Output=`, `Layers=` and the options), an XML form (`<section>` or `<output>` elements with attributes, `<layer number=>` and `<param name= value=>`) and a JSON form (`{"sections": [...]}`). A file larger than 1 MiB, with more than 1000 sections, a section without a device or a layer outside 1 to 255 is refused (`PHOENIX-EAGLEX-007`).

`phoenix eagle-convert cam-to-profile` turns it into a profile file for the export (`config/export_profiles.json`, format `phoenix-export/1`):

* the extension of the Gerber files when all of them have the same, and the decimals of a `Format` such as `2.5`;
* the style of the drill files from a device such as `EXCELLON_24` (fixed, 2 digits before the point and 4 after).

A profile cannot say more, so the rest is named: the options of each section that are not respected (`PHOENIX-EAGLEX-008`: the names of files, the unit, the wheel, the flags; as a warning the mirror, rotation, scale, offset and tolerance when they are not neutral), the layers that the export does not write as a layer of its own (`009`) and the devices that are not Gerber or Excellon or Gerber without RS-274X (`010`, the section is not converted).

## Part attributes and assembly variants

`read_variants` reads the `<variantdefs>`, the `<attribute>` elements of the file and of each part (`<part>` of a schematic, `<element>` of a board) and the `<variant>` elements of each part, keeping every attribute as it is. A part is fitted in a variant unless its `<variant>` of that name says `populate="no"`, and a `value` of that `<variant>` replaces the value of the part. `view("NAME")` gives that for every part (an empty name is the base design); a variant the file does not define is refused with the names it does define (`PHOENIX-EAGLEX-011`). `check` names the inconsistent data (`012`): a variant defined twice, a name used by two parts, a `<variant>` of a name that is not defined or given twice in one part. When two parts have the same name the first one counts.

`write_variants` writes a changed set back into the tree. Elements that already say the same are not touched, so writing what was read changes nothing; a part of the set that the file does not have is skipped and named.

* `phoenix bom --variant NAME` takes the variants from the board when it is an EAGLE `.brd` or `.sch`, otherwise from `--variant-file FILE`; the parts that the variant does not fit are left out of the bill and the values it changes replace the values of the board.
* `phoenix pnp --variant NAME` (and `--variant-file`) leaves the parts that the variant does not fit out of the placement files like `--dnp`, and lists them.
* `phoenix eagle-convert variants [--variant NAME] <file>` lists the variants and, for one, the parts that are not fitted and the values it changes.

## Layers

`standard_table` is the table of the 63 standard layers (1 to 54 and 91 to 99) with their EAGLE names and a role (`copper`, `outline`, `silk_top`, `mask_bottom`, `drill`, `milling`...). The user layers 100 to 255 have a role only when `config/eagle_layers.cfg` gives them one, by number (`layer.100 = silk_top`) or by the name they have in the file (`name.Milling2 = milling`); the roles are the words listed in that file. Numbers 55 to 90 are not standard layers.

`read_layers` reads the `<layers>` of a file and names a number that is not from 1 to 255 or is used twice (`PHOENIX-EAGLEX-015`), a standard layer with another name (`016`) and a user layer without a role (`014`, kept in the file and not exported). The map is also what decides, for `cam-to-profile`, whether a layer of a job is exported. The command `phoenix eagle-convert layers [file]` prints the table or the layers of a file with their roles.

## The other entities

`read_entities` lists, from the tree, the dimensions, the holes, the vias with their extent and shape, the polygons with `isolate`, `pour`, `thermals`, `orphans` and `rank`, the elements that are smashed or locked, the signals with their class, the net classes with their clearances, the `<designrules>` of a board, the passes of the autorouter with their parameters, and the modules with their ports and instances (`<module>`, `<moduleinst>`). The contents of `<libraries>` are not looked at. They are a view: the elements stay in the tree.

## The loss report: `phoenix eagle-check`

```
phoenix eagle-check [--layers FILE] [--json] [--lang CODE] <file.brd|file.sch|file.lbr>
```

It reads a file and reports:

* whether rewriting the tree changes the file (it must not; if it does, `PHOENIX-EAGLEX-020` is an error and the exit code is 1) and how many of the nodes that nothing covers have the same text after it;
* the layers and the variants, and the findings of both;
* for the way through the design model, one finding for each kind of element that is simplified (`018`, for example the shape of a pad, the roundness of an SMD pad, a smashed or locked element, a net class, the pour, thermals, orphans and rank of a polygon, the contents of a module), one for each kind that is left out and that the records of this crate cover (`019`, for example attributes, variants, dimensions, holes, design rules, the autorouter, net classes and modules), and one for each kind that nothing interprets (`017`, kept byte for byte in the tree and left out by the model).

The list of what the model reads is written from the readers of `eh3d-phoenix-eagle-board` as they stand and is approximate for libraries, which have no reader into a model. `phoenix eagle-convert` has the subcommands `dru-to-rules`, `rules-to-dru`, `cam-to-profile`, `layers` and `variants`; the result is printed (the findings go to the error output) or, with `--out`, written to a new file (an existing file is never replaced, `PHOENIX-EAGLEX-023`). Exit codes: 0 done, 1 an error finding, 2 the arguments or the input are unusable.

## Tests and corpus

The unit tests of each module, `crates/eh3d-phoenix-eagle-deep/tests/deep.rs` and `crates/eh3d-phoenix-cli/tests/eagle_deep.rs` use the files in `crates/eh3d-phoenix-eagle-deep/tests/data` (a rule file of two layers, a CAM job of nine sections and the same small job in its three forms) and six files added to `tests/corpus/eagle` with their expectations (a board with variants and attributes, a schematic with variants and a module, a board with user layers, a renamed layer, a repeated number and a number out of range, a board with polygons of several pours, holes and a dimension, a board with net classes, design rules and an autorouter, and a board with odd attributes: entities, line breaks, non-ASCII names, a name used twice and a variant that is not defined). A test rewrites every file of the EAGLE corpus through the tree and requires the same tree, or a refusal with a code.

## What is not done

See `NOT_YET.md`: the check against files saved by EAGLE (and with it the form of the CAM job that EAGLE saves), a profile of the export that respects more of a CAM job, and a way through the design model that keeps what the model has no place for. A variant is applied to the bill of materials and the placement file; the electrical rule check of a schematic does not know variants.
