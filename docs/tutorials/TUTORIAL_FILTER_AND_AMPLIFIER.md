# Simulate an RC filter and an inverting amplifier

Two circuits written as SPICE-style netlists: an RC low-pass of 1 kΩ and 1 µF (`rc_step.cir` and `rc_ac.cir`) and an inverting amplifier of gain −R2/R1 with an op-amp (`inv_amp.cir`). You take measures with `--measure`, run the circuit over a list of values with `--step`, find the cutoff and the pole, and run a Monte Carlo study with a fixed seed.

Run the commands from the root of the repository; see [how to run the tutorials](README.md).

1. Measure the 10 % to 90 % rise time of the step response. The table has the column `run` and one column for the measure. For an RC circuit it is ln 9 · R · C, about 2.197 ms.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 10u 10m --measure "tran rise10 rise v(out)"
run,rise10
~ nominal,
```

2. Run the circuit once for each value of a resistor with `--step`. With 2 kΩ the rise time doubles. The table has one row for each value.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 20u 40m --step "R1 list 1k 2k" --measure "tran rise10 rise v(out)"
r1,run,rise10
```

3. Sweep the frequency and ask for the figures of merit of the response. The cutoff of the low-pass is 1 / (2π R C), about 159 Hz.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 100 1 100k --figures "v(out)"
name,value,unit
~ cutoff_3db,
```

4. Fit the poles and zeros to the AC response. The pole of this circuit is real and at −1 / (R C) = −1000 rad/s.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 20 10 100k --poles "v(out)"
~ pole,
```

5. Now the inverting amplifier. With R1 = 1 kΩ and R2 = 10 kΩ the gain is −10; an input of 1 V gives an output of −9.999 V, a little less than 10 V in magnitude, because the gain of the op-amp model is finite (1e5).

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --op --probe "v(out)"
name,value
~ v(out),-9.99
```

6. Step R2 over three values and measure the output at the one point of a DC sweep. The gains are close to −2, −10 and −20.

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --dc V1 1 1 1 --step "R2 list 2k 10k 20k" --measure "dc vout at v(out) 1"
r2,run,vout
```

7. Monte Carlo: a divider whose resistors have a tolerance of 1 % (`tol=1%` in the netlist). The same seed gives the same table on every computer. After the runs come the rows `count`, `mean`, `std`, `min`, `max` and the percentiles.

```console
# test
$ phoenix sim examples/tutorials/div_tol.cir --mc 200 --seed 5
run,vb
```

## See also

* [docs/SIM.md](../SIM.md)
* [docs/SIM_ADVANCED.md](../SIM_ADVANCED.md)
* [docs/SIM_NONLINEAR.md](../SIM_NONLINEAR.md)
* [docs/ORACLE.md](../ORACLE.md)
