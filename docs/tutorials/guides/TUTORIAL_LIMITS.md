# Bound the work of the checks with a limits file

Some checks take time with the square of a count, for example the test that the edges of a polygon do not cross. A limit keeps that time bounded: a polygon over the limit is not tested for crossing and the report says so as information. You run the check of a board with the defaults and then with a limits file that is far too low, and see the note.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. Run the rule file on the KiCad board with the default limits. The track of the board is thinner than the file asks, so there is one confirmed finding and the exit code is 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. Run it again with `--limits examples/guides/limits_small.cfg`, which says `max_polygon_points = 3`. The outline of the board has four points, so it is listed as information with the code `PHOENIX-BOARD-017`, and the exit code stays 1 because information never changes it.

```console
# test
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ PHOENIX-BOARD-017
~ max_polygon_points = 3
~ PHOENIX-CHECK-001
```

3. The same note in another language and as JSON for a script. The code stays in English; the title follows `--lang`.

```console
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --lang pt examples/tutorials/kicad/demo.kicad_pcb
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --json examples/tutorials/kicad/demo.kicad_pcb
```

## How to read the result

* `--limits FILE` is accepted by every command, before or after its name. The file holds `key = value` lines of `config/board.cfg` and `config/eagle_board.cfg` in any order; a line that is not known, or a value out of its range, is refused with the number of the line and exit code 2. The two keys of `config/outline_arcs.cfg` (`tolerance_nm` and `max_pieces`, which say how the arcs of the outline are cut into chords) may stand in the same file.
* A limit changes only what is bounded. With the defaults nothing changes in any output, and a polygon over `max_polygon_points` still gets its repeated points checked.

## See also

* [config/board.cfg](../../../config/board.cfg)
* [docs/ERROR_CODES.md](../../../docs/ERROR_CODES.md)
* [docs/RULES.md](../../../docs/RULES.md)
