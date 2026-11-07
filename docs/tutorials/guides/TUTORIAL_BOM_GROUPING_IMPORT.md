# Group and export the bill of materials, and import the bill of another program

The board `examples/guides/bom_four.brd` has three resistors of 10 k and one of 4k7. You choose how the parts are grouped and which columns the CSV file has, both in the settings file, and then read a bill written by another program with a map of its columns. Nothing is fetched from a network and the tariff in the settings is only an example of the shape of a quote.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. List the bill with the default grouping (value, footprint, maker and part number) and the maker data of `bom_parts.csv`. R1 and R2 share maker and number, R3 has another maker, so there are three lines. There is no price file, so each line is flagged with `PHOENIX-BOM-005`.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

2. The same board with `group_by = value,footprint` in `bom_costs_joined.cfg`: the maker and the part number leave the grouping, so R1, R2 and R3 become one line of quantity 3 that has neither.

```console
$ phoenix bom --config examples/guides/bom_costs_joined.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

3. Write the CSV file. Its columns are the `csv_columns` of the settings, in that order (here `references,quantity,value,footprint,manufacturer,mpn,status`); the HTML page and the JSON file keep all columns. A file that exists is never replaced.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv --csv build/bom.csv examples/guides/bom_four.brd
```

4. Import the bill of another program. `bom_other_program.csv` uses semicolons and its own column names; `bom_import_map.cfg` says how to read it. The report compares the bill with the board: R5 is in the bill and not on the board (`PHOENIX-BOM-021`), and R4 is marked not to be fitted, so it is left out of the bill and listed as information (`PHOENIX-BOM-009`). `--parts-out` saves what the bill says about makers and numbers as a file of part data.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --import examples/guides/bom_other_program.csv --import-map examples/guides/bom_import_map.cfg --parts-out build/parts_from_bill.csv examples/guides/bom_four.brd
```

5. Use the saved file of part data in later runs instead of the original bill.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts build/parts_from_bill.csv examples/guides/bom_four.brd
```

## How to read the result

* The default settings file is `config/costs.cfg` and the default map is `config/bom_import.cfg`, when they exist. A map names the delimiter (comma, semicolon or tab) and, for each column, the names it may have in the file; only the references are required. The names that come with the program are a guess and are not the layout of any program: copy the file and write the names of your own file.
* The codes `PHOENIX-BOM-019`, `PHOENIX-BOM-020` and `PHOENIX-BOM-021` are warnings and change nothing: a quantity that is not the number of references of its line, a reference in two lines (the first is used) and a part that only one side has or whose value differs. A range such as `R1-R5` is expanded to its single parts; a range badly formed, upside down or longer than `max_range` stops the import (`PHOENIX-BOM-022` to `PHOENIX-BOM-024`). A dash after a part name whose end is not a number (`C1-A`, `J1-B`) is kept as one reference and only warned about with `PHOENIX-BOM-022`. `--import-format` also reads an EAGLE partlist or a KiCad bill (`docs/BOM.md`). What `--parts` says wins over the imported bill.
* Without `--csv`, `--json-file` and `--html` the command writes nothing. The exit code is 0 when done, 1 when a component names a footprint that is not on the board and 2 when the arguments or the input are unusable.

## See also

* [docs/BOM.md](../../../docs/BOM.md)
* [docs/PARTS.md](../../../docs/PARTS.md)
* [docs/ERROR_CODES.md](../../../docs/ERROR_CODES.md)
