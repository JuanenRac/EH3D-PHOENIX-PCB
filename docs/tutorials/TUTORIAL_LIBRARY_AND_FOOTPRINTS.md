# Edit a library and generate IPC footprints

You check an EAGLE library, add a footprint to it with a file of operations (all or nothing, and a copy of the original is kept), and generate footprints by the method of IPC-7351B, with their canonical names. The dimensions of the generated footprints are a prototype and have not been checked against a datasheet.

Run the commands from the root of the repository; see [how to run the tutorials](README.md).

1. Check the library. Nothing is written without `--out`; the report counts the footprints, symbols and device sets and lists the findings.

```console
# test
$ phoenix libedit examples/tutorials/parts.lbr
~ 1 footprints, 1 symbols, 1 device sets
~ 0 errors, 0 warnings
~ dry run: nothing was written
```

2. Apply a file of operations that creates the package `HDR1X2` and adds two pads. The new library is written to a new file, read back and compared; a copy of the original is kept next to it.

```console
# test
$ phoenix libedit --apply examples/tutorials/lib_ops.json --out build/parts_hdr.lbr examples/tutorials/parts.lbr
~ operation 3: add pad to HDR1X2
~ the file was read back and is the same library
```

3. Generate the footprint of a chip resistor 0603 at density level B (nominal). Its canonical name is `RESC1608X55N`: the body is 1.6 mm by 0.8 mm and 0.55 mm high. Without `--out` it is a dry run that lists the pads.

```console
# test
$ phoenix ipc generate chip 0603 --density B
RESC1608X55N (density B,
~ dry run: nothing was written
```

4. Write it to a new `.lbr` file (a `.kicad_mod` name writes a KiCad footprint instead).

```console
# test
$ phoenix ipc generate chip 0603 --density B --out build/RESC1608X55N.lbr
~ written: build/RESC1608X55N.lbr
```

5. The simpler generator `footprint` makes a footprint from parameters (families chip, SOIC, TSSOP, QFN, QFP, DIP and header).

```console
# test
$ phoenix footprint chip --param size=0603 --out build/chip.lbr
~ footprint CHIP_0603
~ written:
```

6. Check the libraries you made: each has one footprint.

```console
# test
$ phoenix libedit build/chip.lbr
~ 1 footprints
$ phoenix libedit build/RESC1608X55N.lbr
~ 1 footprints
```

7. Compare a footprint of your own library with the generated one, pad by pad (exit code 1 when pads differ), and list the families of the library.

```console
$ phoenix ipc compare build/chip.lbr chip 0603
$ phoenix ipc list
```

## See also

* [docs/LIBRARY_EDIT.md](../LIBRARY_EDIT.md)
* [docs/IPC_LIBRARY.md](../IPC_LIBRARY.md)
* [docs/LIBINDEX.md](../LIBINDEX.md)
* [docs/SANITIZER.md](../SANITIZER.md)
