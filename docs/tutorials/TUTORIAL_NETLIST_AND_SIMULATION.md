# From a schematic to a netlist and a simulation

The file `examples/tutorials/divider.json` is a schematic of a source of 10 V and two resistors of 1 kΩ and 3 kΩ in series. You make its simulation netlist without writing one by hand, simulate it and check the result against the divider formula: 10 V × 3 / (1 + 3) = 7.5 V.

Run the commands from the root of the repository; see [how to run the tutorials](README.md).

1. Print the netlist of the schematic. Nothing is written (a dry run) and the electrical rule check runs first. The first line is the title; the elements follow, with the values in the notation of the simulator.

```console
# test
$ phoenix netlist examples/tutorials/divider.json
divider
~ R1 in out 1e3
~ R2 out 0 3e3
~ V1 in 0 1e1
```

2. Write it to a new file. A file that exists is never written over.

```console
# test
$ phoenix netlist examples/tutorials/divider.json --out build/divider.cir
~ written:
```

3. Find the operating point. The divider formula gives 7.5 V at the node `out`.

```console
# test
$ phoenix sim build/divider.cir --op --probe "v(out)"
name,value
v(out),7.5e0
```

4. Skip the file: `--schematic` makes the netlist from the schematic and gives it straight to the solver.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --op --probe "v(out)"
name,value
v(out),7.5e0
```

5. Sweep the source from 0 V to 10 V in steps of 5 V: the output is always three quarters of the input.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --dc V1 0 10 5 --probe "v(out)"
~ 5e0,3.75e0
~ 1e1,7.5e0
```

## See also

* [docs/SIM.md](../SIM.md)
* [docs/NETLIST.md](../NETLIST.md)
* [docs/ERC.md](../ERC.md)
