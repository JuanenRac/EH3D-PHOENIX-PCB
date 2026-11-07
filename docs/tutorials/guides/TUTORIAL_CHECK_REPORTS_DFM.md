# Write the findings of a check as a table and a page, and check a board against a maker's profile

You run the design rules, keep the findings as a CSV table and as an HTML page grouped by category, and then check a small EAGLE board against a capability profile of a board maker. The profile of the example is invented: it shows the form of the file and is not the capability of any maker.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. Run the rules of a rule file and write both reports. Each option writes a new file: a file that exists is never replaced, and nothing is written when one of the outputs is refused (exit code 2). The track of the board is thinner than the file asks, so there is one confirmed finding and the exit code is 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json --csv build/check.csv --html build/check.html examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. Write the same reports in another language. The title column of the CSV and the headings of the page follow `--lang`; the keys of the columns, of the categories and of the severities stay in English so that a script can read them in any language.

```console
$ phoenix check --lang es --rules examples/tutorials/rules_strict.json --csv build/check_es.csv --html build/check_es.html examples/tutorials/kicad/demo.kicad_pcb
```

3. The comparison of a schematic and a board takes the same two options.

```console
$ phoenix connectivity --csv build/connectivity.csv --html build/connectivity.html examples/tutorials/tut.sch examples/tutorials/tut.brd
```

4. Check the EAGLE board against the profile `examples/guides/dfm_small_shop.cfg`. With `--dfm` the limits of the profile replace the rules of the board, so `--rules` is not given. The board is 30 mm by 20 mm and the profile allows 10 mm on the short side: an error with the code `PHOENIX-DFM-003` and exit code 1. A board with more than one copper layer also gets `PHOENIX-DFM-001`, and the information `PHOENIX-DFM-006` says that the profile is not verified.

```console
$ phoenix check --dfm examples/guides/dfm_small_shop.cfg --csv build/dfm.csv --html build/dfm.html examples/tutorials/tut.brd
```

5. Now the example profile of the repository, `config/dfm/example.cfg`. The board is inside every limit of it, so the only finding of the capability category is the information that the profile is not verified.

```console
$ phoenix check --dfm config/dfm/example.cfg --html build/dfm_example.html examples/tutorials/tut.brd
```

## How to read the result

* The CSV has the header `list,category,severity,code,title,location,x_nm,y_nm,measured_nm,limit_nm,detail` and one row for each finding, the proven ones first (`list` is `confirmed` or `inconclusive`). A text cell that a spreadsheet would run as a formula gets a quote mark in front.
* The HTML page has no script and no outside file and escapes every text that comes from the board, so it is safe to open in a browser. It starts with a count of the proven and inconclusive findings of each category and goes on with one table for each category that has findings; the findings of the profile are in the category `capability`.
* The profile is a text file of `key = value` lines in nanometres; a line that is missing or 0 is off. Copy `config/dfm/example.cfg`, put in the figures that you read in the documentation of your maker, write where you read them in `source` and only then set `verified = true`. Nothing checks the file against a maker.

## See also

* [docs/DRC_ADVANCED.md](../../../docs/DRC_ADVANCED.md)
* [docs/DFM.md](../../../docs/DFM.md)
* [docs/RULES.md](../../../docs/RULES.md)
