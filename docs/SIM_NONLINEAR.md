# EH3D-PHOENIX-PCB: non-linear devices and analyses of the simulator

This page describes what the circuit simulator (`eh3d-phoenix-sim`, command `phoenix sim`) does with devices whose equations are not linear, how it finds and reports a DC solution, the sweep of a source and the analyses a netlist can ask for. `docs/SIM.md` has the netlist language, the linear elements, the AC and noise analyses and the output tables.

**State: prototype.** The models are simple, the solver is our own and none of the results has been compared with SPICE or with a measurement. The reference cases (`docs/ORACLE.md`) and the tests compare the results with closed-form solutions and with fixed-point iterations written in the tests; that shows the equations are solved as written, not that the models describe a real part.

## Devices

| Element | Model line | Equation | Left out |
|---|---|---|---|
| Diode `D` | `.model name D(IS N RS CJO VJ M FC TT BV IBV NBV)` | `I = IS (exp(v / (N Vt)) - 1)`, a series resistance `RS`, depletion and diffusion capacitance, reverse breakdown from `BV` | high-level injection, temperature dependence of `IS`, noise of the breakdown |
| LED | `.model name LED(...)` | a diode whose `IS` and `N` default to `led_is` and `led_n` of `config/sim.cfg` (1e-17 A and 2, about 1.8 V at 10 mA); every parameter of the diode is accepted | the light output, the colour |
| Zener diode | `.model name ZENER(BV=... [IBV NBV ...])` | a diode that must give `BV`; reverse, the current is `-IBV exp(-(v + BV) / (NBV Vt))` | the temperature coefficient, the knee of a real part |
| Bipolar transistor `Q` | `.model name NPN(...)` or `PNP(...)` | transport form of Ebers-Moll with the forward Early effect (`VAF`), `RB RC RE`, junction capacitances and transit times | reverse Early voltage, high injection, leakage, substrate |
| MOSFET `M` | `.model name NMOS(...)` or `PMOS(...)` | level 1 (Shichman-Hodges): cut-off, triode and saturation, `beta = KP W / L`, `LAMBDA`; the drain and the source swap when `vds` changes sign | body effect, sub-threshold current, junction diodes (the bulk is read and not used) |
| Voltage-controlled switch `S` | `.model name SW(VT VH RON ROFF)` | the conductance goes from `1 / ROFF` to `1 / RON` as `s = (1 + tanh((vc - VT) / VH)) / 2` goes from 0 to 1, on a logarithmic scale; at `VT` it is the geometric mean of the two | hysteresis, a finite switching time, thermal noise of the switch |
| Op-amp `U` | `.model name OPAMP(GAIN GBW VPOS VNEG ROUT)` | a transconductance into a pole at `GBW / GAIN` hertz, then an output stage with a smooth limit at the rails | slew rate, input offset, bias currents, noise |
| Comparator `U` | `.model name COMP(VHIGH VLOW GAIN ROUT)` | a smooth step between the two levels | hysteresis, delay |

The element lines are in `docs/SIM.md`; the switch is `Sname n1 n2 nc+ nc- model` (the two switched nodes, then the control pair). Parameters that are not given take the defaults of `config/sim.cfg`; a parameter that is not known is read and left out, and the line is noted.

The switch is smooth on purpose: a hard switch has no derivative and the Newton-Raphson iteration needs one. `VH` is the half width of the transition (the switch is 88 % of the way at `VT + VH`). A circuit that depends on the exact instant of a switch (a very sharp edge, a very small `VH`) may need a short time step or fail to converge, and the failure is reported.

## How a DC solution is found

The operating point solves the non-linear equations by Newton-Raphson, one linearised system per iteration:

1. The plain iteration starts from zero. The junction voltages of diodes and transistors are limited between iterations (the limiting of SPICE, `pnjlim`), so the exponential cannot throw the iteration far away. A conductance `gmin` is added across every junction.
2. The change must stay below `reltol` times the value plus `vntol` (volts) or `abstol` (amperes) in two iterations in a row, and no junction voltage may have been limited.
3. If it does not converge in `newton_max_iter` iterations, **gmin stepping** is tried: a conductance from every node to ground starts at `gmin_start` and is divided by `gmin_factor` at each of `gmin_steps` steps, each solution starting the next, and then it is taken away.
4. If that fails too, **source stepping**: the sources are raised from zero in `source_steps` steps.
5. If the last one fails, the error is `PHOENIX-SIM-011` and says the number of iterations.

The result reports how it went: `iterations` (the Newton-Raphson iterations of all the attempts), `strategy` (`direct`, `gmin-stepping` or `source-stepping`) and `residual`, the largest residual of the equations at the solution recomputed with the devices evaluated at that very point (amperes in the node equations, volts in the branch equations). The residual is the honest measure of how well the devices agree with Kirchhoff's current law where the iteration stopped; because the iteration stops when the change is small and not when the residual is, the residual of a diode circuit is of the order of the conductance times the last change, not zero. The JSON of `--op` has the three fields; the CSV has the unknowns only.

A node with no DC path to ground is reported before the matrix is built (`PHOENIX-SIM-010`) unless `floating_is_error = false`; the channel of a MOSFET, the junctions of a transistor, the diode and the switch count as paths.

## DC sweep

`--dc SRC START STOP STEP` (or `.dc src start stop step` in the netlist) solves the operating point while one independent voltage or current source takes the values `START + k STEP` for the whole `k` up to `STOP` (a small slack absorbs the rounding of the division; a negative `STEP` sweeps downwards). Each point starts from the solution of the one before, which keeps the iteration on the same branch of a transfer curve; if that does not converge the point is solved again from zero with the aids above and counted in `restarts`. The table has the value of the source and the unknowns (`sweep(vin),v(out)`), and the JSON also has the total iterations, the restarts and the largest residual.

A source that does not exist, is not independent, a step of zero or going away from the stop value, numbers that are not finite and a sweep with more than `max_dc_points` points are refused (`PHOENIX-SIM-005`, `-012`, `-008`). A sweep moves one source; there is no nested sweep and no sweep of a parameter or of the temperature.

## Transient and AC

The transient analysis (`--tran STEP STOP [--adaptive] [--method be|trap]`, `docs/SIM.md`) uses the trapezoidal rule, or backward Euler, and with `--adaptive` compares one step with two half steps: a step whose difference is above the tolerances is rejected and made again smaller, and the step grows when the error is small. The step stops at the corners of `PULSE` and `PWL` sources and at the delay of a `SIN` source. The sources `SIN(vo va freq td theta phase)`, `PULSE(v1 v2 td tr tf pw per)` and `PWL(t1 v1 ...)` act in the transient analysis and give their start value to the operating point. The non-linear devices are linearised at every time point; a switch driven by a source takes its control voltage from that source, and the adaptive step does not know the instant a device-driven control crosses its threshold.

The AC analysis linearises the circuit at the operating point (a diode, a transistor, a switch or an output stage becomes its conductances, and a junction its capacitance) and solves it in complex numbers at each frequency.

## The netlist

Besides `docs/SIM.md`:

* `.op`, `.dc src start stop step`, `.tran tstep tstop [uic]` and `.ac lin|dec|oct points start stop` are read into the netlist (`Netlist::analyses`) and written back by `to_spice`. A start time or a largest step on the `.tran` line, a step of zero in `.dc`, a source that is not independent and a frequency range that is not ordered are `PHOENIX-SIM-024` (or `-005` for the source) and name the line. `phoenix sim` runs the first of them when no analysis is named on the command line.
* `.param name=value` and expressions between braces are as before.
* `.include name` is allowed only for a file in the same folder as the file that contains it: the name is a plain file name (no folder, no `..`, no drive, no absolute path) and the file, with its links resolved, must still be in that folder. The nesting is bounded by `max_include_depth` (which also stops a file that includes itself) and the files read by `max_include_files`. Anything else is `PHOENIX-SIM-023`. The `.end` of an included file is dropped. Read from text and not from a file, an `.include` is refused with the same code.
* `phoenix sim --model FILE` reads a file of `.model` and `.subckt` lines (the file may be in any folder; its own `.include` lines follow the same rule relative to it) and puts it after the title of the netlist, before the elements. The file is only read, never written. A schematic takes no file of models.
* An error names the line of the file it comes from; when it is in an included file or in the file of models the detail starts with the name of the file in brackets.

## Settings

Every default of convergence and size is in `config/sim.cfg`, not in the code. The ones that matter here:

| Key | Default | Meaning |
|---|---|---|
| `reltol`, `vntol`, `abstol` | 1e-3, 1e-6 V, 1e-12 A | tolerances of the Newton-Raphson iteration and of the adaptive step |
| `gmin` | 1e-12 S | conductance across every junction |
| `newton_max_iter` | 100 | iterations of one solution |
| `gmin_steps`, `gmin_start`, `gmin_factor` | 10, 1e-2 S, 10 | gmin stepping (zero steps turns it off) |
| `source_steps` | 20 | source stepping |
| `step_safety`, `step_max_growth`, `step_min_shrink`, `min_step_fraction`, `initial_step_fraction` | 0.9, 2, 0.25, 1e-6, 0.1 | adaptive step |
| `max_unknowns`, `max_elements`, `max_lines`, `max_file_bytes` | 1500, 20000, 200000, 4 MiB | size limits (the solver is dense) |
| `max_dc_points`, `max_tran_steps`, `max_ac_points`, `max_result_cells` | 1e5, 1e6, 1e5, 1e7 | analysis limits |
| `max_include_depth`, `max_include_files` | 4, 16 | `.include` limits |
| `sw_ron`, `sw_roff`, `sw_vt`, `sw_vh` | 1 ohm, 1e9 ohm, 0 V, 0.1 V | switch defaults |
| `led_is`, `led_n` | 1e-17 A, 2 | LED defaults |
| `diode_*`, `bjt_*`, `mos_*`, `opamp_*`, `comparator_*` | see the file | defaults of the other models |

An unknown key or a value out of range is refused with its line (`PHOENIX-SIM-007`).

## The command

```
phoenix sim [--config FILE] [--lang xx] [--json] [--out BASE] [--probe LIST] [--model FILE] <input> [<analysis>]
```

`<analysis>` is `--op`, `--dc SRC START STOP STEP`, `--ac KIND N F1 F2`, `--tran STEP STOP [--uic] [--adaptive] [--method be|trap]` or `--netlist`. Nothing is written unless `--out BASE` is given (it writes `BASE.csv` and `BASE.json`, each as a temporary file and a rename). The model file and the netlist are only read. Exit code 0 when done, 2 when the arguments, the settings, the input or the circuit are unusable.

## Reference cases

The cases of `tests/oracle/sim.json` (`phoenix oracle`) with non-linear devices, each with the formula it uses, all closed-form and none compared with SPICE:

* a diode with a series resistor on a voltage source (three sets of `IS`, `n`, `R`, `Vs`), against the Lambert W solution `I = (n Vt / R) W((IS R / (n Vt)) exp((Vs + IS R) / (n Vt))) - IS`, with `Vt = kT/q` at the temperature of the settings; the tests solve `W` in the test by Newton iteration and check it against a bisection of the same equation; the tolerance is 1e-4 V, wider than the error the stop rule of Newton-Raphson leaves (about 1e-5 V);
* an LED with the defaults of the settings, by the same formula;
* a half-wave rectifier at the peak of the sine and in the blocking half cycle (a circuit without reactive elements, so every step is the static circuit);
* the transfer table of an NMOS inverter with a resistive load, from a sweep of the input: cut-off, saturation (`Vout = Vdd - Rd beta Vov^2 / 2`) and triode (the smaller root of `Vds^2 - (2 Vov + 1 / (beta Rd)) Vds + Vdd / (beta Rd) = 0`, written for the circuit of the case);
* an inverting amplifier with an op-amp of high gain, `Vout / Vin = -R2 / R1` (the tolerance allows for the finite gain of the model);
* a voltage-controlled switch closed and opened into a load, as a divider of the on or off resistance and the load.

The tests of the crate repeat these with more points (the whole transfer table of the NMOS and CMOS inverters, every sample of the rectifier against the static diode curve, the closed-loop gain of the op-amp at 100 kHz from its pole, the switch in the AC analysis, the charging of a capacitor through a switch closed by a pulse).

## Not done

* No comparison with SPICE or with a measurement; the models are simple (the lists above).
* The solver is dense and the number of unknowns is limited by `max_unknowns`.
* A circuit that SPICE solves with more convergence aids may not converge here (a high-gain op-amp circuit that clips in a feedback loop, a switch with a very small `VH`).
* No hysteresis of switches, no temperature sweep (the temperature is one setting), no sweep of a parameter (`.step`), no `.measure`, no nested DC sweep, no `.lib` or `.global`, no flicker noise, no models of a manufacturer.
* The netlist of a schematic knows only the parts whose designator is one of the supported letters, and it has no transistors, switches or op-amps.
