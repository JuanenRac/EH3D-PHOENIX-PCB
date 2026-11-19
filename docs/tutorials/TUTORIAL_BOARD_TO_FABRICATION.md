# From a board to the fabrication files

The KiCad project in `examples/tutorials/kicad/` has two resistors on a board of 30 mm by 20 mm joined by the net `SIG`. You check the schematic and the board against each other, run the design rules and write the manufacturing files with a manifest of checksums.

Run the commands from the root of the repository; see [how to run the tutorials](README.md).

1. Run the electrical rule check of the schematic. Every finding has a code and the confirmed ones are kept apart from the inconclusive ones.

```console
$ phoenix erc examples/tutorials/kicad/demo.kicad_sch
```

2. Compare the schematic with the board by the identity of the parts. `sync` is a dry run: it lists what the transfer would change and writes nothing unless you add `--apply --out FILE`.

```console
$ phoenix sync examples/tutorials/kicad/demo.kicad_sch examples/tutorials/kicad/demo.kicad_pcb
```

3. Run the design rules with the limits of a rule file. The track is 0.25 mm wide and the file asks for 0.2 mm at least, so nothing is wrong and the exit code is 0.

```console
# test
$ phoenix check --rules examples/tutorials/rules_relaxed.json examples/tutorials/kicad/demo.kicad_pcb
Confirmed: 0
Inconclusive: 0
```

4. Write the manufacturing files into a new folder: one Gerber X2 file for each layer, the drill files, the placement file, the bill of materials and `manifest.json` with the SHA-256 of every file. The files are read back and compared with the board before they are written.

```console
# test
$ phoenix export --out build/fab examples/tutorials/kicad/demo.kicad_pcb
~ files written: build/fab
```

5. Make the whole package, with the stencil and the placement of each face, as one deterministic zip file: the same board gives the same bytes. Without `--out` it is a dry run that lists the files.

```console
$ phoenix fabpack --out build/demo_pack.zip examples/tutorials/kicad/demo.kicad_pcb
```

6. Look at the board in numbers before you send it: the size, the counts, the copper of each layer, the drills and the density. Nothing is written.

```console
# test
$ phoenix stats examples/tutorials/kicad/demo.kicad_pcb
Board statistics:
Board: 30.000000 x 20.000000 mm; 600.000000 mm2; Cutouts: 0.000000 mm2
Components: 2 (top: 2; bottom: 0)
```

7. Mark a pad as a test point with an operation of the board editor (`examples/tutorials/testpoint_ops.json` marks the second pad of R1, on the net `MID`) and report the test points and the nets that no probe reaches. The input is never written over.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/testpoint_ops.json --out build/with_test_point.json examples/tutorials/tut.brd
~ written: build/with_test_point.json
$ phoenix testpoints build/with_test_point.json
Test points of the board:
Test points: 1 (probe side: both)
Nets asked: 1; covered: 1; coverage: 100.00 %
```

8. Add two dimensions to a board that has a documentation layer (`examples/tutorials/dimension_ops.json`: the width and the height of the board, with the dimension line 3 mm outside) and write them into a DXF drawing; in the window the same is done with the dimension tool, and *Edit dimension* moves their points. The drawing is a dry run until `--out` names a new file.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/dimension_ops.json --out build/with_dimensions.json examples/tutorials/notes_board.json
~ written: build/with_dimensions.json
$ phoenix dxf-export board --dimensions build/with_dimensions.json
entities: 21
layer DIMENSIONS: 16
```

9. Ask the supplier assistant where to buy the bill. It reads a price file (`examples/tutorials/prices.csv`, one row for each supplier and quantity break) and the suppliers, the shipping and the least order of a settings file (`examples/tutorials/sourcing.cfg`), and prices the bill at each supplier and as a split purchase. The prices are the ones of the file: nothing is fetched from a network.

```console
# test
$ phoenix bom --sourcing --config examples/tutorials/sourcing.cfg --prices examples/tutorials/prices.csv examples/tutorials/kicad/demo.kicad_pcb
~ suppliers: 2 considered, 1 lines, EUR
~ cheapest single supplier: Shop A for 2.210000
~ split purchase: 1 orders for 2.210000
```

## See also

* [docs/EXPORT.md](../EXPORT.md)
* [docs/FABPACK.md](../FABPACK.md)
* [docs/RULES.md](../RULES.md)
* [docs/DRC_ADVANCED.md](../DRC_ADVANCED.md)
* [docs/STATS.md](../STATS.md)
* [docs/TESTPOINTS.md](../TESTPOINTS.md)
* [docs/DIMENSIONS.md](../DIMENSIONS.md)
* [docs/BOM.md](../BOM.md)
