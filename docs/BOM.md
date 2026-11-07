# EH3D-PHOENIX-PCB: bill of materials with costs

The crate `eh3d-phoenix-bom` and the command `phoenix bom` list the parts of a board, group them, price them from a local file and estimate the cost of a batch of boards (requirement PPCB-010, the bill of materials, beyond the simple one that the manufacturing export writes).

**State: prototype.** It has been tried on boards made by hand and by the tests only, never on a real design and never against a real quote. The numbers the tests expect were worked out on paper. The tariffs that come with the program are an example of the shape of a quote and are not the prices of any board maker; the prices are the ones of the file you give, and nothing checks that they are current.

Nothing is fetched from a network: every price, tariff and part number comes from a file on the computer.

## The lines

The parts are the components of the board. With `--schematic FILE` (a JSON schematic) the parts of the schematic whose reference is not on the board are added, with the finding `PHOENIX-BOM-011`, and the footprint they name. Parts are grouped by **value, footprint, maker and part number** (`group_by` in `config/costs.cfg` takes the maker or the part number out of the grouping, so parts that only they told apart become one line with neither; the value and the footprint always count); each line has its quantity for one board and its references in natural order (`R2` before `R10`). The maker and the part number come from the file of part data (`--parts FILE`); a part that the file does not mention has neither.

A part without a footprint is listed and flagged (`PHOENIX-BOM-006`); a component that names a footprint that is not on the board is an error (`PHOENIX-BOM-010`).

### The file of part data

A CSV with a header line and the columns `match`, `manufacturer`, `mpn` and `fitted`.

```
match,manufacturer,mpn,fitted
R1,Acme,AC-R1,
10k|R0603,Yageo,RC0603FR-0710KL,yes
C9,,,no
```

`match` is a reference (`R1`) or a value and a footprint joined by a bar (`10k|R0603`); the line of a reference wins over the line of a value and a footprint. `fitted` is `yes` (also when empty) or `no`; a part that is not fitted is left out of the bill and listed as information (`PHOENIX-BOM-009`).

## The prices

A price file (`--prices FILE`) is CSV or JSON. One row is one quantity break of one part at one supplier.

| Column | Meaning |
|---|---|
| `value`, `footprint` | the parts a row without a part number is for; an empty `footprint` matches any |
| `manufacturer`, `mpn` | the part a row with a part number is for (the number is compared without regard to case) |
| `supplier`, `supplier_code` | who sells it and the code of the part at that supplier |
| `min_qty` | the quantity from which the price holds (1 when missing) |
| `unit_price` | the price of one part, a decimal with at most six decimals |
| `currency` | the currency of the row (the one of the settings when empty) |

A JSON file is a list of objects with the same names, or an object with the list under `prices`; a price may be a string or a number. Rows in a currency other than the one of the settings are left out and counted in one finding (`PHOENIX-BOM-007`). Two rows of one supplier for the same part and the same quantity break with different prices give `PHOENIX-BOM-013` and the lower price is used.

## What is bought and what it costs

For each line:

1. The quantity needed is the quantity per board times the boards of the batch, plus the waste in per cent (`parts_waste_percent`), rounded up.
2. At each supplier that has a row for the part, the price is the one of the greatest quantity break that the quantity needed reaches. When the quantity needed is below the first break, that first break is bought (`PHOENIX-BOM-008`) at its price.
3. The supplier with the least cost for the whole quantity is chosen; on a tie, the first by name.
4. The cost of the line for the batch is the quantity bought times the unit price; for one board it is that cost divided by the boards of the batch, rounded half up.

A line with no offer costs nothing in the totals, is listed (`PHOENIX-BOM-005`) and the totals say that they are not complete.

Money and percentages are whole millionths in the program (`0.0125` is `12500`): no price passes through a floating-point number.

## The boards

The cost of the bare boards is an estimate from `config/costs.cfg`:

* the area of the outline in square centimetres, divided by the part of the panel that holds boards (`pcb_utilisation_percent`);
* times the price of a square centimetre for the number of copper layers (`pcb_area_rate.<layers>`; a number of layers with no line cannot be costed, `PHOENIX-BOM-012`);
* plus a surcharge in per cent when the stack-up (`--stackup FILE`, the job file of `phoenix stackup`) is thicker than `pcb_thickness_reference_nm`;
* plus `pcb_drill_cost` for each hole, the vias and the through-hole pads;
* for the batch, `pcb_setup_cost` plus the boards times the cost of one board, and at least `pcb_min_order_cost`.

Every estimate says so (`PHOENIX-BOM-018`). The total of the batch is the parts plus the boards.

## The command

```
phoenix bom [--config FILE] [--prices FILE] [--parts FILE] [--import FILE [--import-format NAME] [--import-map FILE]] [--parts-out FILE] [--parts-db PATH] [--schematic FILE]
            [--stackup FILE] [--boards N] [--csv FILE] [--json-file FILE] [--html FILE] [--json] [--lang CODE]
            <board>
```

It is a dry run: without `--csv`, `--json-file` and `--html` nothing is written, and the text report goes to the screen (`--json` prints the JSON instead). Each of the three options writes a new file; a file that exists is never replaced, the input included, and when one of them exists none is written. The default settings file is `config/costs.cfg` when it exists. Exit codes: 0 done, 1 the bill has an error (a component whose footprint is not on the board), 2 the arguments or the input are unusable.

## The outputs

The same input always gives the same bytes: the order is fixed, money is written as decimals with six places, and nothing depends on the time or the machine.

* **CSV**: one line for each bill line with the columns `references`, `quantity`, `value`, `footprint`, `manufacturer`, `mpn`, `supplier`, `supplier_code`, `unit_price`, `price_break`, `quantity_to_buy`, `line_cost_batch`, `line_cost_per_board`, `price_breaks` and `status` (`ok`, `no_price`, `no_footprint` or two of them joined by `+`). A text cell is quoted when it holds a comma, a quote or a line break, and a cell that a spreadsheet would run as a formula (it starts with `=`, `+`, `-` or `@`) gets a quote mark in front.
The columns of the CSV file, and their order, are the `csv_columns` of `config/costs.cfg` (all fifteen by default; a name that is not in the list, or one named twice, is refused with `PHOENIX-BOM-001`). The HTML page and the JSON file keep all of them.

* **JSON** (`phoenix-bom/1`): the lines with their offers and price breaks, the totals of the parts, the estimate of the boards, the total, whether it is complete and the findings; money is a string with six decimals.
* **HTML**: one page with a table, the totals and the findings, with no script and no outside file; every text that comes from a file is escaped.

## Importing the bill of another program

`--import FILE` reads the bill of materials that another program wrote and uses what it says about the parts of the board. `--import-format NAME` says which format it is (the default `auto` looks at the file):

| Format | The file | How it is read |
|---|---|---|
| `auto` | any of the following | an EAGLE partlist (it starts with its title line `Partlist exported from`, or with a header line `Part Value ...`) is `eagle`; an XML file (it starts with `<?xml` or `<export`) is `kicad-xml`; every other text is `csv` |
| `csv` | a generic CSV file | with the map of columns of `--import-map FILE` (default `config/bom_import.cfg` when it exists): the delimiter (comma, semicolon or tab), the names of the columns for the references, value, footprint, maker, part number and quantity (several names for one column are separated by a bar, case does not matter, and for each line the first of those columns that has a cell is used), a column that says the part is fitted and one that says it is not to be fitted (with the cells that mean so), and the characters that separate several references in one cell. Only the references are required |
| `eagle` | the partlist that the EAGLE schematic editor writes as text | in columns of a fixed width: each cell starts where the name of its column starts in the header line (`Part`, `Value`, `Device`, `Package`, `MF`, `MPN` and the other attributes). The part is the reference; the value, the footprint (the first column called `Package`), the maker (`MF`, else `MANUF`) and the part number (`MPN`, else `MP`) are read, and a column `DNP` marks parts not to be fitted. A file that is not UTF-8 is read one byte for each character |
| `kicad-csv` | the CSV bill of KiCad | the columns `Reference`, `Refs`, `Reference(s)` or `Designator`, `Value`, `Footprint`, `Qty` or `Quantity`, `Manufacturer`, `MPN` and `DNP`; the delimiter (comma, semicolon or tab) is taken from the header line, and the block of information that an older export writes before the header is left out |
| `kicad-xml` | the XML bill of KiCad | the elements `comp` (attribute `ref`), `value`, `footprint`, the `field` elements of `fields` (`Manufacturer`, `MPN`) and the `property` elements named `dnp` or `exclude_from_bom`, which make the part not fitted. The five named entities and the numeric ones are decoded; entity declarations are refused and nothing is expanded or fetched |

The formats other than `csv` have the column names written above, which the map does not change; from the map they take the limits (`max_rows`, `max_range`, `max_references`), the characters that separate references and the cells that mean not fitted. The EAGLE reader was tried on two partlists that EAGLE wrote (they are not in the repository), on all their lines; a partlist of another version of EAGLE, or a file edited by hand so that a cell does not start under its header, is read as it is and may give wrong cells. The KiCad formats are written from the public description of the files and have not been compared with a file of every version of KiCad.

A cell of references may hold single references and ranges. `R1-R5` is `R1`, `R2`, `R3`, `R4` and `R5`, and `R1-5` is the same; `C08-C10` keeps the width of its first number (`C08`, `C09`, `C10`); spaces around the dash are allowed (`R1 - R5`). A text with a dash that does not start like a part (`TP-1`) is a reference as it is. A range is refused, and the import stops with the line in the message, when it is badly formed (`R1-`, `R1-C5`, a number of more than nine digits: `PHOENIX-BOM-022`), when it ends below its start (`R5-R1`: `PHOENIX-BOM-023`) or when it names more parts than `max_range` (1000 by default: `PHOENIX-BOM-024`); the references of all the lines together are limited by `max_references` (`PHOENIX-BOM-017`). The quantity of a line is compared with the references after the ranges are expanded.

What is done with the bill:

* the maker, the part number and the parts that are not fitted fill the references that `--parts` does not name (what `--parts` says wins), so they take part in the grouping and the prices as if they were in a file of part data;
* the report gets the findings `PHOENIX-BOM-019` (a quantity that is not the number of references of its line), `PHOENIX-BOM-020` (a reference in two lines, the first is used) and `PHOENIX-BOM-021` (a part that is in the bill and not on the board, on the board and not in the bill, or whose value differs, compared without regard to case); all are warnings and change nothing;
* `--parts-out FILE` writes the data as a file of part data, one line for each reference that has a maker, a part number or is not fitted (a new file; one that exists is never replaced), to be used later as `--parts`.

Nothing is fetched from a network.

## Completing the bill from the parts base

With `--parts-db PATH` (a file or a folder of the local parts base, `docs/PARTS.md`) every part of the design is looked up in the base, by the part number that `--parts` gives or else by value and footprint, and the maker, the part number and the prices of the bill are completed from it. What `--parts` and `--prices` say wins over the base. The text of the bill ends with the parts without a match (`PHOENIX-PARTS-014`), the parts that match several parts of the base, which get none (`PHOENIX-PARTS-015`), and the parts that are obsolete or not recommended with the alternatives proposed for them (`PHOENIX-PARTS-016`); the proposals change nothing.

## The bills of the assembly

A manufacturer that fits the two faces in two passes asks for one list for each pass. The module `assembly` of the crate `eh3d-phoenix-bom` (used by `phoenix fabpack`) makes three bills with the same grouping, prices and parts that are not fitted as the plain bill: the top face, the bottom face and the whole board. Each face is a bill of a copy of the board that holds only the components of that face; the schematic is used for the whole board only, because its parts have no face. The cost of the bare boards is left out.

The CSV file of a face has the columns `references`, `quantity_board`, `quantity_panel`, `value`, `footprint`, `manufacturer`, `mpn`, `supplier`, `supplier_code`, `unit_price`, `quantity_to_buy` and `status`. The quantity of the panel is the quantity of a board times the boards in the panel (`--panel-boards`, 1 when there is no panel). `status` is `fitted`, `no_footprint` or `dnp`: the parts that the file of part data says are not fitted come after the others, each group with the status `dnp`, so that the assembler sees them. The JSON file (`phoenix-assembly-bom/1`) holds the three views with their lines, their parts that are not fitted and their findings. The same input always gives the same bytes.

## The codes

`PHOENIX-BOM-001` to `PHOENIX-BOM-024`, with their titles in the eight languages; the table is in `docs/ERROR_CODES.md`.

## What it does not do

* It does not fetch prices, stock or part numbers from a supplier, and it does not order anything.
* It does not merge lines of equivalent parts, split a quantity between suppliers or choose by stock or lead time.
* It does not know the real tariff of any maker: the cost of the boards is the shape of a quote, with the numbers you put in the settings.
* It reads the bills of EAGLE and KiCad and, through a map of columns that you write, any other CSV bill; it does not read the formats of other programs, nor spreadsheets (`.xlsx`).
