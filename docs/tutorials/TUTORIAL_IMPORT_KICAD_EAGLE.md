# Import a KiCad or EAGLE project and review it

You read a whole KiCad project into the design model of the program, see what was kept and what was left out, write the model as new files and check the board against a rule file. Then you do the same review for an EAGLE board.

Run the commands from the root of the repository; see [how to run the tutorials](README.md).

1. Read the project (the `.kicad_pro` with the `.kicad_sch` and the `.kicad_pcb` of the same name). It is a dry run: the report says what was read, how much was kept, and what the model does not hold.

```console
# test
$ phoenix kicad-import examples/tutorials/kicad/demo.kicad_pro
~ usable: yes
~ components: 2
~ tracks: 1
~ vias: 1
~ dry run: nothing was written
```

2. Write the model as new files next to the name you give. A file that exists, or an input, is never written over.

```console
# test
$ phoenix kicad-import --out build/demo.json examples/tutorials/kicad/demo.kicad_pro
~ written:
```

3. Check the imported board. A KiCad board gives no design rules to the program, so the limits come from a rule file. The file asks for tracks of 0.3 mm at least and the track of the board is 0.25 mm: one confirmed finding and exit code 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
Confirmed: 1
~ PHOENIX-CHECK-001
```

4. An EAGLE file: ask what a rewrite keeps. The report proves that writing the XML tree and reading it again changes nothing, and lists what the design model simplifies or leaves out.

```console
# test
$ phoenix eagle-check examples/tutorials/tut.brd
board (version 9.6.2)
~ rewrite of the tree: identical
```

5. Run the same rule file on the EAGLE board. The track of the net `MID` is 0.2 mm wide.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/tut.brd
# exit 1
Confirmed: 1
```

6. Compare the EAGLE schematic with the EAGLE board: the report lists nets that are missing, pins that are not connected and the copper that is not routed yet.

```console
$ phoenix connectivity examples/tutorials/tut.sch examples/tutorials/tut.brd
```

## See also

* [docs/KICAD_IMPORT.md](../KICAD_IMPORT.md)
* [docs/KICAD.md](../KICAD.md)
* [docs/EAGLE_DEEP.md](../EAGLE_DEEP.md)
* [docs/RULES.md](../RULES.md)
