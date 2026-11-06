# EH3D-PHOENIX-PCB: pick and place files

The module `pnp` of the crate `eh3d-phoenix-export` and the command `phoenix pnp` write where every part of a board goes, one file for each face, as CSV and as JSON (`phoenix-pnp/1`). Every choice is in `config/pnp_rotations.cfg` (part of requirement PPCB-010).

**State: prototype.** It has been tried on boards made by hand and by the tests only. The files have never been loaded in a pick and place machine and no assembler has compared them with its own; the table of extra rotations that comes with the program is empty, because the turn a head needs is a fact about a machine or a maker that this repository cannot know.

## The centre of a part

The centre is worked out from the footprint of the part and written in the coordinates of the board:

* `centre = body` (the default): the middle of the box that holds all the pads, each pad with its own size and turn. A footprint whose pads are not symmetric about its origin has its centre away from its origin, and this is the centre a machine looks for;
* `centre = pads`: the mean of the centres of the pads;
* `centre = origin`: the origin of the footprint, as it is placed.

A part on the bottom face is mirrored about the vertical axis before it is turned, as the model does for its pads. A footprint without pads, or a component whose footprint is not on the board, uses its origin as its centre (`PHOENIX-PNP-003`, `PHOENIX-PNP-002`).

## The rotation

The rotation written is the turn of the component plus the extra rotation of the table, brought to the range from 0 up to a full turn, and then the rule for the bottom face (`bottom_rotation`: `keep`, `mirror` or `half_turn`, the same three choices as the placement file of the export profiles).

The table is made of `rotate = PATTERN DEGREES` lines. `PATTERN` is matched against the name of the footprint without regard to the case, with `*` for any run of characters and `?` for one; the first line that matches wins. Lines after `[name]` belong to a machine or a maker and are chosen with `--maker name`; they are tried before the lines above the first section. A part that a line turned is counted in one finding (`PHOENIX-PNP-010`).

```
rotate = SOT23* 180

[machine_a]
rotate = SOT23* 270
rotate = QFN* 90
```

## The origin and the unit

* `origin = lower_left` (the default): the lower left corner of the box of the board outline;
* `origin = aux`: the point `aux_x_mm`, `aux_y_mm` of the coordinates of the board;
* `origin = panel`: the origin of a panel in which the lower left corner of the board is at `panel_x_mm`, `panel_y_mm`. The place is a setting; it is not read from the plan of `phoenix panel`.

`units = mm` (the default) or `mil` (thousandths of an inch). Coordinates are written with `decimals` digits and the rotation with `rotation_decimals`, rounded half away from zero by integer arithmetic, so no number passes through a floating-point text. A part whose centre is outside the box of the outline is reported (`PHOENIX-PNP-007`).

## Parts that are not fitted

The set of designators that are not fitted comes from the file of part data of `phoenix bom` (`--parts`: the lines with `fitted` equal to `no`, by designator or by value and footprint) and from `--dnp R1,R2`. With `not_fitted = exclude` (the default) they are left out of the lines and listed in the JSON file (`not_fitted`); with `mark` they stay in the lines and the CSV gets a `Status` column (`fitted` or `not_fitted`). Each one is reported (`PHOENIX-PNP-006`). Two components with the same designator and a component without one are reported too (`PHOENIX-PNP-004`, `PHOENIX-PNP-005`); the parts stay in the file.

## The files

For each face that has parts, in the order of the natural sort of the designators (`R2` before `R10`):

* `pnp_top.csv`, `pnp_bottom.csv`: `Ref,Value,Package,PosX(mm),PosY(mm),Rot(deg),Side` (the unit is in the header), a header line and `\n` line ends, the separator of the settings and the quoting of the fields of the export;
* `pnp_top.json`, `pnp_bottom.json`: `{"format": "phoenix-pnp/1", "board", "side", "units", "origin", "centre", "parts": [...], "not_fitted": [...]}` with one object for each part (`ref`, `value`, `footprint`, `x`, `y`, `rotation`, `side`, `fitted`); the numbers are the decimals of the CSV.

The same board and settings always give the same bytes.

## The command

```
phoenix pnp [--config FILE] [--maker NAME] [--parts FILE] [--dnp R1,R2,...]
            [--origin lower_left|aux|panel] [--units mm|mil] [--lang CODE] [--out FOLDER] <board>
```

It is a dry run: it prints the number of parts of each face and the findings. With `--out` it writes the files to a folder that does not exist yet; an existing folder is never touched (`PHOENIX-PNP-009`). Exit code 0 is done and 2 is any refusal. The codes `PHOENIX-PNP-001` to `011` are in `docs/ERROR_CODES.md` with their titles in the eight languages.

## What it does not do

* It does not import the placement of a machine or of an assembler, and it writes no machine format beyond the two above.
* It does not take the centre from a drawing of the body of the part: only the pads give it.
* It does not know which rotation a head needs; that is the table you fill.
* The origin of a panel is a setting, not the plan of the panel.
