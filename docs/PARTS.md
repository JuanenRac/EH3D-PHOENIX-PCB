# EH3D-PHOENIX-PCB: the local parts base

The crate `eh3d-phoenix-parts` and the command `phoenix parts` keep the components that you use in files of your own, search them, check them and complete the bill of materials from them (requirement PPCB-010, the data of the bill of materials; the editor of schematics takes a part from the base through `eh3d-phoenix-schedit`).

**State: prototype.** It has been tried on small bases written by the tests only, never on a real collection of parts. The prices are the ones that you write in your files, with no check that they are current. Nothing is ever fetched from a network: the base is a file or a folder on your computer, and the program reads it and, only when you give a new file name, writes a copy of it.

## What a part holds

| Field | What it is |
|---|---|
| `id` | the name of the part in the base, unique; an alternative and a command name a part by it |
| `value`, `tolerance`, `power`, `voltage` | the value (`10k`, `100n`, `NE555`) and its ratings, as text |
| `footprint` | a name of the IPC library (`docs/IPC_LIBRARY.md`) or a name of your own libraries |
| `symbol` | the name of the symbol |
| `manufacturer`, `mpn` | the maker and its part number |
| `status` | `active` (also when empty), `not-recommended` or `obsolete` |
| `params` | free parameters, by name (`dielectric`, `tcr`, ...) |
| `sim_model` | the name of a model of the table of models (`config/sim_models.cfg`, a `.model` line), so that a part of the base points to the model that the simulator uses (`docs/NETLIST.md`) |
| `offers` | the suppliers, each with a supplier code and the price breaks: from `min_qty` parts the unit price is `unit_price` in a currency; the format is the one of the price file of the bill (`docs/BOM.md`) |
| `alternatives` | the ids (or part numbers) of equivalent parts |
| `notes` | free text |

## The files

The base is one file or one folder. A folder is read file by file, in the order of the names, and only the files `.json` and `.csv` of its top level are read (links are not followed and subfolders are not entered). The limits (files, size, parts, offers, price breaks, results) are in `config/parts.cfg`; a file over a limit is refused and a message says which.

**JSON** (`phoenix-parts/1`), the form that holds everything:

```json
{
  "format": "phoenix-parts/1",
  "parts": [
    {"id": "r-10k", "value": "10k", "tolerance": "1%", "power": "0.1W",
     "footprint": "R0603", "symbol": "R", "manufacturer": "Acme", "mpn": "AC-10K",
     "status": "active", "params": {"tcr": "100ppm"}, "sim_model": "",
     "alternatives": ["r-10k-b"],
     "offers": [{"supplier": "Shop A", "supplier_code": "A-1",
                 "prices": [{"min_qty": 1, "unit_price": "0.01", "currency": "EUR"},
                            {"min_qty": 100, "unit_price": "0.004", "currency": "EUR"}]}]}
  ]
}
```

A key that is not known is refused, so a typing mistake is not lost. A price is a decimal with at most six decimals (a string or a number) and is kept as whole millionths, so no price passes through a floating-point number.

**CSV**, a header line and any of the columns `id`, `value`, `tolerance`, `power`, `voltage`, `footprint`, `symbol`, `manufacturer`, `mpn`, `status`, `sim_model`, `notes`, `alternatives` (ids joined by `;`), `params` (`name=value` joined by `;`), `supplier`, `supplier_code`, `min_qty`, `unit_price` and `currency`. One row is one part, or one more supplier or price break of the part with the same `id`; on those rows the cells of the part must be empty or equal to the first row. A row without `id` takes its `mpn` as the id. A cell that starts with `=`, `+`, `-` or `@` is written with a quote mark in front so that a spreadsheet does not read it as a formula, and the quote mark is taken off when the file is read.

A fault is refused with its place: `base.csv: line 7: min_qty is not a whole number from 1`, or `base.json: part 3: unknown key colour` (`PHOENIX-PARTS-003`). The file is refused as a whole; nothing is guessed.

## The commands

```
phoenix parts search [--base PATH] [--value V] [--footprint F] [--mpn TEXT] [--maker TEXT]
                     [--status active|not-recommended|obsolete] [--limit N] [--json] [WORDS...]
phoenix parts show   [--base PATH] [--json] <id>
phoenix parts check  [--base PATH] [--lib PATH ...] [--no-ipc] [--models FILE] [--json] [--lang xx]
phoenix parts import [--out NEW.json] [--config FILE] <file.csv | file.json>
phoenix parts export [--out NEW.csv | NEW.json] [--base PATH]
```

`--base` is the file or folder of the base (default `parts`). `search` gives the parts, in the order of the ids, whose text holds all the words (in the id, value, footprint, symbol, maker, number, ratings, notes, parameters and model) and that pass the filters; a value is compared without case, blanks or the ohm sign, so `10k` finds `10 kΩ`; the same search gives the same list every time. `show` prints one part in full (exit 1 when the id is not in the base, `PHOENIX-PARTS-017`).

`import` reads a CSV (or JSON) file and, with `--out`, writes a **new** `.json` base; without `--out` it is a dry run that says how many parts it read and what the check finds. A base with an error is not written. `export` writes the base as CSV or JSON to a **new** file. Neither command writes over a file that exists or over an input file (`PHOENIX-PARTS-006`), and the files of the base are never changed.

## The check

`phoenix parts check` finds what is wrong in the base and changes nothing. Exit code 1 when it finds an error, 0 when it finds none (warnings do not change the exit code).

| Code | Severity | What |
|---|---|---|
| `PHOENIX-PARTS-007` | error | an id used by two parts; the message gives the place of the first |
| `PHOENIX-PARTS-008` | warning | a footprint that is not in the IPC library (unless `--no-ipc`) or in the libraries `.lbr` given with `--lib` |
| `PHOENIX-PARTS-009` | error | two parts with the same part number (without case) and another value, footprint, maker, tolerance, power or voltage; an empty field is not a difference |
| `PHOENIX-PARTS-010` | warning | a price break without a currency |
| `PHOENIX-PARTS-011` | warning | price breaks whose quantities do not rise |
| `PHOENIX-PARTS-012` | warning | a simulation model that the table of models (`--models`, the built-in table by default) does not have |
| `PHOENIX-PARTS-013` | warning | an alternative that is neither an id nor a part number of the base |

The names of the IPC footprints are those that `phoenix ipc` generates (`docs/IPC_LIBRARY.md`), which are a prototype; a footprint of your own that the IPC library does not have is known through `--lib`. The messages are written in the eight languages (`--lang`), and the report is the same every time.

## The bill of materials from the base

`phoenix bom --parts-db PATH <board>` looks up every part of the design (the components of the board and the parts of `--schematic` that are not on the board) in the base and completes the maker, the part number and the prices of the bill:

* a part for which the file of part data (`--parts`) already gives a part number is looked up **by that number** (and by the maker, when the file gives one). A number that is not in the base is a part without a match, and it is never replaced by another part;
* every other part is looked up **by value and footprint**: the value is compared as described above and the footprint without case; a part of the base with no footprint fits any footprint, unless another one has the footprint of the design;
* among several parts the ones that are `obsolete` are left out when another remains, and then the ones that are not `active` when an active one remains. When more than one remains, the part of the design is reported with all of them and gets none: **nothing is guessed** (`PHOENIX-PARTS-015`);
* what `--parts` and `--prices` say wins over the base: a maker or a part number written there stays, and the price rows of those files come first;
* the offers of the parts that were found are added as price rows, by part number (or by value and footprint for a part without a number), in the currency written in the base.

The text of the bill ends with the report of the link: the counts, the parts without a match (`PHOENIX-PARTS-014`), the parts with several (`PHOENIX-PARTS-015`) and the parts that are obsolete or not recommended (`PHOENIX-PARTS-016`), each with the references. For a part that is obsolete the report **proposes** the active parts that could take its place (the alternatives it names and the parts with the same value and footprint); the proposal is a list, and nothing in the design, the files or the bill is changed by it.

## Assigning a part to a symbol

The editor of schematics without a screen (`eh3d-phoenix-schedit`, module `assign`) makes the plan of assigning a part of the base to a symbol that is placed: the value that the symbol would take, the maker and part number, the footprint and the model the base asks for, and the line of the file of part data (`match,manufacturer,mpn,fitted`) that keeps the part number for the bill. Making the plan changes nothing. Applying it runs one operation of the editor, so **one undo gives the schematic back as it was**. A value that is the same in another notation (`10 kΩ` for `10k`) is left as the person wrote it. The footprint is only named: the schematic keeps the footprint of a part as an id of a footprint of the board. The window has no button for it yet; the logic is tested without a screen.

## The codes

`PHOENIX-PARTS-001` to `PHOENIX-PARTS-018`, with their titles in the eight languages; the table is in `docs/ERROR_CODES.md`.

## What it does not do

* Fetch prices, stock or data sheets from a supplier, or check that a price is current; the base is what you write in it.
* Merge equivalent parts, split an order between suppliers or choose an alternative by itself: it proposes and the person decides.
* Know whether a footprint is the right one for a part; it only checks that the name exists in a library it knows.
* Read a data sheet, or hold a symbol or a simulation model; a part names its symbol and its model, and the files of those are elsewhere.
* Be seen on a real collection: the tests write small bases and check the order, the limits, the faults with their place, the link, the proposals and that no file is changed.
