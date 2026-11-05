# EH3D-PHOENIX-PCB: the script interpreter

**State: prototype.** The crate `eh3d-phoenix-script` and the commands `phoenix script run`, `phoenix script report` and `phoenix cam-import` read script files and output jobs of the command language of the previous EAGLE-based workflow, carry the scripts out on a board and say, for every command of every script, how well it is covered. It was built from the public description of the language (the names of the commands and their general syntax) and from nothing else: no manual was copied and no script of a third party was used. Every test uses scripts written by hand for it. It has never been compared with the original tool, and it has not yet been run on the author's own scripts and jobs, so the equivalences below are claims about the design of the interpreter and not results of a comparison (requirement PPCB-019; phase F8 of `docs/ROADMAP.md`).

## The commands

```
phoenix script run [--config FILE] [--rules FILE] [--dry-run] [--out FILE] [--json] [--lang xx] <script> <board or schematic>
phoenix script report [--config FILE] [--json] [--html FILE] [--lang xx] <folder or script>
phoenix cam-import [--config FILE] [--json] [--out FILE] [--lang xx] <job.cam>
```

* `script run` carries a script out on a board (an EAGLE `.brd` file or the JSON of the model) or, for the commands that do not edit a board, on a schematic (a `.sch` file or JSON). It is a **dry run** unless `--out FILE` is given and `--dry-run` is not: the script runs on a copy and only the outcome is printed (the commands and the operations counted, the diagnostics, the text of `DRC`, `ERC` and `RATSNEST`). With `--out` the board that results is written to a new file as canonical JSON; a file that exists is never replaced, so the input is never overwritten. A schematic is never written. Exit codes: 0 done, 1 the script was not carried out (its diagnostics are printed), 2 the arguments or the input are unusable.
* `script report` takes a script or a folder (every `.scr` file below it, in the order of the paths, links not followed) and prints the compatibility report; see below. `--html FILE` also writes the page to a new file.
* `cam-import` reads an output job (`.cam`) into a profile of the export and reports what could be translated; `--out FILE` writes the profile as a profile file of `phoenix export` (`phoenix-export/1`) to a new file.

`--config` names the settings (default `config/script.cfg` when it exists), `--rules` the JSON rule file that `DRC` checks against (without it every rule is off, as in `phoenix check`; an EAGLE board gives its own design rules), `--lang` one of en, es, de, fr, it, ja, zh, pt.

## The language

A script is a list of commands. A command is a word followed by arguments and ends at a semicolon or at a line break (a backslash at the end of a line joins the next one; with `newline_ends_command = false` only the semicolon ends a command). A `#` that starts a token begins a comment that runs to the end of the line. An argument is a word (`0.2mm`, `R90`, `GND`), a text in single quotes (`'a b'`, two quotes in a row are one quote) or a coordinate in parentheses (`(10 5.5)`). A length is a decimal number with an optional unit written right after it: `mm`, `mil`, `inch` (`in`), `micron` (`mic`) or `grid` (steps of the current grid). A number without a unit is in the unit of the last `GRID` command (millimetres at the start). A length becomes integer nanometres with one rounding, half away from zero.

Objects are named: a component by its reference designator (`R1`), a signal by its name (`GND`). `GROUP` selects objects and the commands that name none act on that selection; there is no selection by pointing at a coordinate.

A text that is not closed, a coordinate that is not valid, a missing command name and every limit of the settings are diagnostics with the line and the column; the command that caused one is dropped, the reading goes on, and a script with such a problem is not started.

## Commands and their state

Each command is `equivalent` (the same meaning and the same effect on the design), `adapted` (carried out, with a difference that is named) or `pending` (not carried out, with the cause). The state belongs to the form of the command that is written: `WIRE` is equivalent, `WIRE` with a bend is pending. The report and the run use the same function to understand a command, so they always agree.

| Command | What it does | State |
|---|---|---|
| `GRID` | `GRID mm 0.5 on`, `GRID inch`, `GRID 50mil`, `GRID last`, `GRID default`: sets the unit of the numbers, the size and the switch of the grid | equivalent |
| `GRID LINES` / `GRID DOTS` | how the grid is drawn | adapted: only recorded (PHOENIX-SCRIPT-033) |
| `GRID ALT`, `FINEST`, `MULTIPLE` | | pending: form not implemented (PHOENIX-SCRIPT-019) |
| `LAYER name` / `LAYER n` | chooses the layer new objects go to; a number is looked up in `layer_numbers` | equivalent |
| `LAYER n name` | defines a layer | pending: the board has no operation to create a layer (PHOENIX-SCRIPT-030) |
| `DISPLAY` | `ALL`, `NONE`, `LAST`, `name`, `+name`, `-name` | adapted: only recorded, the display is not part of the board (PHOENIX-SCRIPT-033) |
| `CHANGE WIDTH` / `DRILL` / `SIZE` / `LAYER` | sets the default for what is drawn next and, for the named objects (`'GND'`, `R1`) or the selection, changes what exists: the width of tracks and strokes, the drill of vias, the height of texts, the layer of tracks and silkscreen | equivalent |
| `CHANGE` with a coordinate, or another option | | pending: PHOENIX-SCRIPT-028, PHOENIX-SCRIPT-019 |
| `WIRE ['net'] [width] (x y) (x y) ...` | a track of the net on the current copper layer (the net is made when it does not exist), or a stroke on a silkscreen layer | equivalent; a line style word is ignored (adapted, PHOENIX-SCRIPT-037); a bend is pending (PHOENIX-SCRIPT-029) |
| `VIA ['net'] [diameter] (x y)` | a round via of the default drill between the outer copper layers | equivalent; a shape word is ignored (adapted, PHOENIX-SCRIPT-041) |
| `CIRCLE [width] (cx cy) (px py)` | two half turns through the point, exactly, on a silkscreen layer | adapted: an outline stroke (PHOENIX-SCRIPT-035) |
| `RECT (x y) (x y)` | the four sides of the rectangle on a silkscreen layer | adapted: an outline stroke, not filled (PHOENIX-SCRIPT-035) |
| `TEXT 'text' [R90 / MR90] (x y)` | a text of the built-in stroke font on a silkscreen layer; the height is the last `CHANGE SIZE`, the stroke is `text_ratio_percent` of it; the angle is a multiple of 90 degrees | adapted: the letters differ from the original font (PHOENIX-SCRIPT-036) |
| `MOVE name (x y)` | puts a component at a place, keeping its rotation | equivalent; by pointing, pending (PHOENIX-SCRIPT-028) |
| `ROTATE R90 [names]` / `=R45` | turns components (or the selection), relative or absolute | equivalent; `MR90` pending (PHOENIX-SCRIPT-031) |
| `DELETE [names]` | removes components (with their pad nets) or the selection | equivalent |
| `NAME old new` | renames a component or, when none has that name, a signal; refused when the new name is in use | equivalent |
| `VALUE name 'value'` | sets the value of a component | equivalent |
| `GROUP names / ALL / NONE` | selects components, and the tracks and vias of a signal | adapted: by name, not by a rectangle (PHOENIX-SCRIPT-040) |
| `SET option ...` | the options in `set_ignored` (`CONFIRM`, `BEEP`, `WIRE_BEND`, ...) | adapted: accepted and ignored (PHOENIX-SCRIPT-037); the others pending (PHOENIX-SCRIPT-032) |
| `RATSNEST` | the connections still to be routed (`eh3d-phoenix-route`) | adapted: delegated to the engine of this tool (PHOENIX-SCRIPT-034) |
| `DRC` | the rule check (`eh3d-phoenix-rules`) with the rules given to the run | adapted: PHOENIX-SCRIPT-034 |
| `ERC` | the electrical rule check (`eh3d-phoenix-erc`) of a schematic | adapted: PHOENIX-SCRIPT-034 |
| `WRITE 'file.json'` | writes the board as it is at that point | adapted: relative paths allowed by the settings, boards are JSON (PHOENIX-SCRIPT-038) |
| `OPEN 'file.json'` | replaces the board of the run by a board file | adapted: PHOENIX-SCRIPT-038; the replacement is not an operation, so such a run has no undo |
| `SCRIPT 'file.scr'` | runs another script below the folder of the script | adapted: read at run time, the report does not follow it (PHOENIX-SCRIPT-039) |
| the commands in `known_pending` (`ADD`, `ROUTE`, `AUTO`, `COPY`, `PAD`, `SIGNAL`, ...) | | pending: known command without an equivalent yet (PHOENIX-SCRIPT-027) |
| any other word | | pending in the report, an error in a run: unknown command (PHOENIX-SCRIPT-006) |

On a schematic only `GRID`, `DISPLAY`, `SET`, `ERC` and `SCRIPT` apply; the schematic model has no reversible operations yet. Any other command is a diagnostic (PHOENIX-SCRIPT-016).

## A run

A run works on a copy of the document and changes it only through the reversible operations of `eh3d-phoenix-board`, so:

* **all or nothing.** A script with a syntax problem, an unknown command or, by default, a pending command is not started; all the problems are reported at once. An error in the middle drops the copy: the original is left as it was and the diagnostic names the line and the column of the command. With `skip_pending = true` a pending command is skipped with a warning; an unknown command still stops the run.
* **undo.** The run returns one batch that does the whole script and one batch that undoes it. Applying the first to the original gives the result and applying the second to the result gives the original, byte for byte; both also run through the history of the board (undo and redo).
* **deterministic.** The same script on the same board gives the same bytes. The identities of new objects come from a generator whose seed and clock are in the settings, and an identity in use is never taken again.
* **bounded.** The settings limit the size and the lines of a script, its commands, the arguments of a command, the length of a text, the nesting of included scripts and the operations of a run.

Nothing is written by a run. A `WRITE` is only noted; it is written, together with the result of `--out`, only when the run is applied (`--out` given, `--dry-run` not).

## Files and safety

A script never starts a program and never reaches the network. It reaches a file only through `SCRIPT`, `OPEN` and `WRITE`, and all three are off in the settings. When one is turned on, a path must be relative, without drive, root or `..`, must not pass through a link, and must lie inside one of the folders of `allowed_dirs` (relative to the folder of the script; the list is empty at the start, so nothing is allowed). A board is read from and written to a `.json` file only, a file that is read has a size limit, and a file that exists is never replaced. This is the explicit confirmation and the starting point of nothing allowed that `docs/DECISIONS.md` asks for imported scripts; a script found inside a project is not run by this command or by any other.

## The compatibility report

For each command of each script the report gives `equivalent`, `adapted` (with the difference) or `pending` (with the cause), computed from the commands alone, without running anything and without a board. It is given as text, as JSON (`phoenix-script-report/1`) and as one HTML page that has its style inside and nothing from outside, in the eight languages. Per script it counts the commands of each state and the share that is equivalent and the share that is equivalent or adapted, in tenths of a percent; a folder gets one line per script, the totals over all its commands (not the mean of the rates) and the pending commands by cause. A text that cannot be read is a pending entry at its place. A script without commands counts as 100.0 % so that an empty file does not lower the rate of a folder. Paths in the report are relative to the folder that was analysed.

The gate of phase F8 asks that every own script and job is listed as equivalent, adapted or pending with its cause and that the equivalent rate does not drop. The listing exists; the rate has no baseline yet because the author's scripts have not been run through it.

## Output jobs

`cam-import` reads the text of a job: sections with `key=value` lines; a section with a `Device` key is one output file, with its name pattern (`Name`), its layers (`Layers`, numbers or names) and its options. The reader is built from the public description of the file and is tolerant; it has not been checked against a file saved by the original tool. It translates into the profile of `eh3d-phoenix-export` (`Gerber` and `drill` choices; the placement and the bill of materials keep their defaults):

* a Gerber RS-274X device: the extension of the file (the first one; a second, different extension is a named difference because the profile has one);
* an Excellon device: the name of the plated drill file (the first) and of the file of the holes that are not plated (the second); a device named `EXCELLON_24` gives the fixed format of 2 integer and 4 decimal digits, taken from its name;
* several layers in one file: written as one file per layer, a named difference (PHOENIX-SCRIPT-026), since the export writes one Gerber file for each layer;
* any other device (plotters, photoplotters with wheels, printers): no equivalent (PHOENIX-SCRIPT-024);
* any other key whose value is not its default (mirror, rotation, offsets, flags, ...): not translated (PHOENIX-SCRIPT-025).

Nothing runs a job and no file of the job's output is made: the result is a profile and a report. The profile always passes `Profile::validate`.

## Settings

`config/script.cfg` holds every limit (script size, lines, commands, arguments, text length, nesting, operations, files and depth of a report, job size and sections), the three file permissions and `allowed_dirs`, `skip_pending`, `newline_ends_command`, the state at the start (unit, grid, width, drill, via diameter, text height, stroke ratio), the seed and the clock of the identities, the table of layer numbers, the list of known commands without an equivalent and the list of options of `SET` that are ignored. An unknown key or a value out of range is refused with its line number. The codes `PHOENIX-SCRIPT-001` to `PHOENIX-SCRIPT-042` are in `docs/ERROR_CODES.md`, with titles in the eight languages.

## What is not done

See `docs/NOT_YET.md`: selection by pointing, curved wires, mirrored placement, creating layers, the rest of the commands of the language, scripts on a schematic that edit it, the comparison with the original tool and with the author's own scripts, running an output job to make its files, and the sandbox of `docs/DECISIONS.md`: a separate low-privilege process, limits on time and memory and a log with the hash of the script (the restrictions above are checks inside the program).
