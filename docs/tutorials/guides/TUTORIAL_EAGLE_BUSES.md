# Read the buses of an EAGLE schematic

`examples/guides/bus.sch` is the schematic of the first tutorial with a bus `D[0..1]` drawn on the sheet and two nets, `D0` and `D1`, whose wires end on the wire of the bus. The reader turns the name of the bus into a bus object with its members and puts an entry where each net meets the bus. A bus is a way of naming: its members are ordinary nets.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. Read the hierarchy as JSON. The key `buses` counts the buses of the sheet: 1. The list of problems has no code of the family `PHOENIX-BUS`, because both members are used by an entry.

```console
# test
$ phoenix hier --json examples/guides/bus.sch
~ "buses": 1
```

2. The same as text: the list of nets holds only the nets that join at least two pins, here `MID`, so the members `D0` and `D1` of the bus, which have no pin, are not in it, and the last line says whether there are problems.

```console
# test
$ phoenix hier examples/guides/bus.sch
~ nets:
~ MID
```

3. Write the schematic as KiCad files. The bus is written as its label and a line from its anchor through the ends of its entries, and each entry as a `bus_entry` with the label of its member.

```console
$ phoenix kicad-export --out build/bus.kicad_sch schematic examples/guides/bus.sch
```

## How to read the result

* A vector `prefix[first..last]` gives the members `prefix<first>` to `prefix<last>` (`D[0..1]` gives `D0` and `D1`; a vector may go down, as in `A[2..0]`). The reader also accepts the lists of EAGLE (`A[0..3],B[0..3]`) and a name with a colon (`BUS:A[0..3]`), where the part before the colon names the bus itself.
* The bus is anchored where its first wire starts. An entry is made at the end of each net of the sheet that has the name of a member and ends on a wire of the bus. The wires and labels of the bus, names that are not vectors and a bus with no wire are not read and are counted as parts that were left out. The codes `PHOENIX-BUS-001` to `PHOENIX-BUS-007` say what is wrong with a bus, and `phoenix hier` lists them.

## See also

* [docs/BUSES.md](../../../docs/BUSES.md)
* [docs/HIERARCHY.md](../../../docs/HIERARCHY.md)
* [docs/KICAD_EXPORT.md](../../../docs/KICAD_EXPORT.md)
