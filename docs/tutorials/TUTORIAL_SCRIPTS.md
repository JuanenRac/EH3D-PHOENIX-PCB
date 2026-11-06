# Use scripts: an LED matrix and a panel

The scripts of the language `phoenix-script/2` have variables, loops with a limit, queries on a board and edits that are all one reversible step, in a sandbox: no network, no program, no file outside the data folder of the settings. A run is a dry run unless `--out` names a new file. The examples are in `examples/scripts/`.

Run the commands from the root of the repository; see [how to run the tutorials](README.md).

1. List the example scripts that come with the program and print one.

```console
# test
$ phoenix script list
~ led_matrix
~ panelize_matrix
$ phoenix script list --show led_matrix
phoenix-script/2 board
```

2. Check a script without running it: syntax, names, the number of arguments, the document each function applies to and an estimate of the steps.

```console
# test
$ phoenix script check examples/scripts/led_matrix.phs
```

3. Run the LED matrix on a board whose footprint is `LED0603`: six LEDs on a pitch of 2.5 mm, with the second pad of each on one net. The board is not changed: it is a dry run.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --board examples/tutorials/led_board.brd
~ placed 6 LEDs on net LEDCOM
~ pads on the net: 6
~ dry run: nothing was written
```

4. The panelize script copies the two parts of the board in a matrix of 2 by 2: from 2 components to 8.

```console
# test
$ phoenix script run examples/scripts/panelize_matrix.phs --board examples/tutorials/tut.brd
~ copies: 4
~ components: 2 -> 8
```

5. With `--out` the result is written to a new file, once. A file that exists is never replaced.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
~ written
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
# exit 2
~~ never replaced
```

## See also

* [docs/SCRIPT.md](../SCRIPT.md)
* [docs/SCRIPT_API.md](../SCRIPT_API.md)
* [docs/SCRIPT_EXAMPLES.md](../SCRIPT_EXAMPLES.md)
