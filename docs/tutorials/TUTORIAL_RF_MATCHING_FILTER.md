# Match an impedance and synthesise a filter

The RF tools work from formulas of textbooks and ideal components. You design an L network that matches 100 Ω to 50 Ω at 100 MHz, a quarter-wave transformer, and ladder filters from the prototype tables. The results are a prototype that has not been compared with a network analyser or a field solver.

Run the commands from the root of the repository; see [how to run the tutorials](README.md).

1. Match a load of 100 Ω to a source of 50 Ω at 0.1 GHz. The L network has a series inductor of 79.5775 nH and a shunt capacitor of 15.9155 pF in the low-pass form, and the dual in the high-pass form.

```console
# test
$ phoenix rf match l --source 50 --load 100 --freq-ghz 0.1
~ 79.5775 nH
~ 15.9155 pF
~ low-pass
~ high-pass
```

2. A quarter-wave transformer between 50 Ω and a load of 200 Ω has the impedance √(50 × 200) = 100 Ω.

```console
# test
$ phoenix rf match qw --load 200 --z0 50 --freq-ghz 1
~ transformer: 100.0000 ohm
```

3. Synthesise a low-pass filter: Butterworth, order 3, cutoff 0.1 GHz, with a sweep of three points. The prototype values are 1, 2 and 1, so the filter has capacitors of 31.8310 pF and an inductor of 159.1549 nH between them.

```console
# test
$ phoenix rf filter butterworth 3 lowpass --fc-ghz 0.1 --sweep-ghz 0.01,0.2,3
~ prototype g: 1.0000 2.0000 1.0000
~ capacitor 31.8310 pF
~ inductor 159.1549 nH
~ frequency, insertion loss, return loss
```

4. A band-pass filter: Chebyshev, order 5, 0.1 dB of ripple, from 90 MHz to 110 MHz, in a system of 75 Ω.

```console
# test
$ phoenix rf filter chebyshev 5 bandpass --f1-ghz 0.09 --f2-ghz 0.11 --ripple-db 0.1 --z0 75
~ Chebyshev (0.1 dB ripple) filter of order 5, band-pass
```

5. Draw a matching network on a board from the stack-up and the frequency, as reversible operations with a report. The list of structures (lines, bends, stubs, filters, couplers, antennas) comes with `--list`.

```console
$ phoenix rfgen --list
```

## See also

* [docs/RF.md](../RF.md)
* [docs/RFGEN.md](../RFGEN.md)
* [docs/STACKUP.md](../STACKUP.md)
* [docs/ORACLE.md](../ORACLE.md)
