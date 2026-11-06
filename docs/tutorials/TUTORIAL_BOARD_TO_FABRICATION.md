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

## See also

* [docs/EXPORT.md](../EXPORT.md)
* [docs/FABPACK.md](../FABPACK.md)
* [docs/RULES.md](../RULES.md)
* [docs/DRC_ADVANCED.md](../DRC_ADVANCED.md)
