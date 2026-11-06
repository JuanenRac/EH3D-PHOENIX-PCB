# Review the signal integrity of a differential pair

The board `examples/tutorials/diffpair.kicad_pcb` has two tracks of 0.25 mm, `D_P` and `D_N`, side by side for 20 mm with 0.75 mm between their edges. You check the pair against a file that says what the pair should be, see the finding when the gap is wrong, read it in Spanish, and estimate the crosstalk. The signal integrity tools are quasi-static estimates from closed formulas, not a field solver.

Run the commands from the root of the repository; see [how to run the tutorials](README.md).

1. The file `pairs.json` says that the pair is `D_P` and `D_N`, with tracks of 0.25 mm and a gap of 0.75 mm. The board agrees: no error and no warning.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs.json examples/tutorials/diffpair.kicad_pcb
~ pairs: 1, errors: 0, warnings: 0
~ pair p (D_P / D_N)
```

2. Now the file asks for a gap of 0.3 mm and the board has 0.75 mm: the gap is out of tolerance, the code is `PHOENIX-PAIR-003` and the exit code is 1.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ PHOENIX-PAIR-003
```

3. The same report in Spanish. `--lang` accepts en, es, de, fr, it, ja, zh and pt.

```console
# test
$ phoenix pairs --lang es --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ La separación del par está fuera de tolerancia
```

4. Estimate the crosstalk between the two tracks from the length that runs side by side, the distance between centres and the stack-up. The estimate is in percent of the step of the aggressor.

```console
# test
$ phoenix si crosstalk examples/tutorials/diffpair.kicad_pcb
~ crosstalk: 1 coupled pairs of nets
```

5. The other analyses: the reflections and terminations of a net, a simplified eye diagram, and all of them together with the return path (exit code 1 when a threshold of `config/si.cfg` is exceeded).

```console
$ phoenix si reflect --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si eye --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si check examples/tutorials/diffpair.kicad_pcb
```

## See also

* [docs/DIFFPAIRS.md](../DIFFPAIRS.md)
* [docs/SI.md](../SI.md)
* [docs/LENGTHS.md](../LENGTHS.md)
* [docs/STACKUP.md](../STACKUP.md)
