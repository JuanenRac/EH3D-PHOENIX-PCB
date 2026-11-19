# EH3D-PHOENIX-PCB: the text netlist

The command `phoenix nets` writes the nets of a board as a small text file, reads such a file again, compares it with a board or with another file, looks for nets that are probably a mistake and writes the pins as a CSV file. The file says only which pins are joined: it is the plain way to hand the connections of a design to another tool, to keep them in a version control system as a readable list, or to check that a board still has the connections of its schematic.

**State: prototype.** It has been tried on boards made by hand and by the tests only, never on a real board or on the netlist of another program.

```
phoenix nets [--config FILE] [--out FILE] [--csv FILE] [--compare NETLIST] [--import NETLIST] [--lint] [--json] [--lang CODE] [<board>]
```

The board is a `.json` board, an EAGLE `.brd` or a KiCad `.kicad_pcb`, as for the other commands. Nothing is written unless `--out` or `--csv` names a new file, which is created exclusively: a file that exists is never replaced (`PHOENIX-NETS-009`), and when two files are asked for and either exists, neither is written. The exit code is 0 when the work is done, 1 when a comparison found a difference or the lint found something and 2 when the arguments or the input are unusable.

## The format

```
# a comment
NET GND U1.1 C1.2 C2.2
NET SIG R1.1 R2.2
    R3.1 R4.1
NET UNUSED
```

* A line is the word `NET` (any case), the name of the net and the pins, separated by blanks. A pin is `PART.PAD`: the reference of the part, a dot and the pad. The part is what comes before the first dot, so a pad may hold dots (`J1.1.2` is the pad `1.2` of `J1`).
* A line that starts with a blank goes on with the pins of the net above it, which is how a net with many pins is written over several lines (the writer wraps at 100 characters).
* A `#` starts a comment that runs to the end of the line. Blank lines and a byte order mark at the start are ignored, and so are the line ends of Windows.
* A net may have no pins. A pin written twice on the same net is one pin; a pin on two nets, a net written twice or a line that is none of these is refused with its line number (`PHOENIX-NETS-002` to `PHOENIX-NETS-005`).
* The writer gives the same text every time: the nets in natural order of their names (`N2` before `N10`) and the pins in natural order (`R2.1` before `R10.1`).

Names have no blanks, a part has no dot and nothing starts with `#`. When a board has a name that cannot be written, the writer does not stop: a pin whose part or pad cannot be written is left out, a net name with a blank or a leading `#` is written with underscores, two nets that get the same written name are joined, and a pin that two nets claim stays on the first. Each of these is a note with the code `PHOENIX-NETS-007` (or `PHOENIX-NETS-005` for the pin), written as a comment line after the netlist, so the netlist says that it is not exactly the board. Only the pads that a net names are written: a net with no pad (a net of tracks alone) is not.

## The ways to use it

* `phoenix nets board.json` prints the netlist of the board. With `--out FILE` the netlist goes to the new file and the output says how many nets and pins were written; with `--json` the output is the nets and the notes as JSON.
* `phoenix nets --import FILE` reads a netlist file instead of a board, checks it against the limits of the settings and prints it again in its canonical form (sorted, wrapped, comments taken out). It is the way to normalise a file that was edited by hand, or to see why it is refused.
* `phoenix nets --compare FILE board.json` compares the file with the board. Each difference is one line with its code, the title in the language of `--lang`, and the net, the pin and the other net:

  | Code | What | Line |
  |---|---|---|
  | `PHOENIX-NETS-011` | a net of the file (with pins) is not on the board | `NET` |
  | `PHOENIX-NETS-012` | a net of the board (with pins) is not in the file | `NET` |
  | `PHOENIX-NETS-013` | a pin of the file is on another net on the board | `NET PIN -> BOARD-NET` |
  | `PHOENIX-NETS-014` | a pin of the file is on no net of the board | `NET PIN` |
  | `PHOENIX-NETS-015` | a pin of a net of the board is on no net of the file | `NET PIN` |
  | `PHOENIX-NETS-016` | a net has another name on the board and exactly the same pins | `NET -> BOARD-NET` |

  A net that only changed its name is one line (`PHOENIX-NETS-016`) and not one line for each of its pins. A pin that is on another net is reported once, from the side of the file. The last line says how many differences there are, and `--out` writes the JSON report to a new file.
* `phoenix nets --compare OLD.nets --import NEW.nets` compares two netlist files: the second one takes the place of the board, so the report reads "board" as "the second file" (the header line says `second netlist`, and the JSON keeps the key `board`). It is how the netlists of two revisions of a schematic are compared, with the same codes, the same exit code and the same settings.

* `phoenix nets --lint board.json` (or `--lint --import FILE`) looks for nets that are probably a mistake and gives one line for each: a net with exactly one pin (`PHOENIX-NETS-017`: an input that nothing drives, or a misspelt name that split a net in two), a net with no pin (`PHOENIX-NETS-019`, which only a file can have) and nets whose names are the same once the case and the separators `_`, `-`, `.`, `/` and blank are taken away, such as `GND` and `Gnd` or `VCC_3V3` and `VCC-3V3` (`PHOENIX-NETS-018`, one line for each group, written `FIRST ~ OTHER, OTHER`). The nets of `ignore_nets` are not looked at, and `max_differences` cuts the list. The exit code is 1 when there is a finding; `--out` writes the JSON report. The lint is not given with `--compare` or `--csv`.
* `--csv FILE` writes the pins of the board (or of the file of `--import`) as CSV under the header `net,part,pad`, one row for each pin in the order of the netlist (a net with no pin has no row), and with `--compare` the differences under `code,net,pin,other` (the ones that are listed, at most `max_differences`). A cell with a comma, a double quote or a line end is quoted. A spreadsheet reads a cell that starts with `=`, `+`, `-`, `@`, a tab or a carriage return as a formula, which turns a net called `+5V` or `-12V` into an error or worse, so with `csv_guard_formulas` (true by default) such a cell gets a single quote in front of it; set it to false when a program that reads the file back needs the names as they are.

## Settings

`config/nets.cfg` holds every value and is also the built-in default: `max_input_bytes` (the largest file that is read, 16 MiB), `max_nets` (100000), `max_pins_per_net` (10000), `max_pins` (1000000), `max_name_len` (128 characters for a net, a part or a pad), `max_differences` (the most differences a comparison, or findings the lint, list, 1000: the total is counted all the same and a line says how many are not listed), `ignore_nets` (nets that a comparison leaves out on both sides and the lint does not look at, any case, and a name that ends in `*` matches every net that starts with the rest; empty by default) and `csv_guard_formulas` (true by default). A line that is not `key = value`, an unknown key or a value out of its range is refused with its line number (`PHOENIX-NETS-010`). A file that is over a limit is refused (`PHOENIX-NETS-006`), and the file is read only up to one byte past `max_input_bytes`, so a huge file or a device that never ends cannot fill the memory.

## Codes

`PHOENIX-NETS-001` to `PHOENIX-NETS-019`, with titles in the eight languages (`src/nets_codes.rs`) and the table in `docs/ERROR_CODES.md`.

## Limits

* The format holds connections only: no values, footprints, net classes, properties or sheets. A netlist of a simulator is `phoenix netlist` (`docs/NETLIST.md`).
* Comparing is by the name of the part and of the pad. A part that was renamed in the schematic is a difference for each of its pins; the netlist does not know that two names are one part.
* The netlist is not written back into a board: `--import` and `--compare` only read. Giving a board the nets of a file is not done.
* The lint knows names and pin counts only: it does not know which nets must have more than one pin (a net of a connector to the outside has one) or which names the designer meant to be different, so a finding is a place to look. `ignore_nets` quiets the ones that are meant.
* The quote that guards a cell of the CSV file is part of the cell for a program that reads it back; the guard is a setting for that reason.
