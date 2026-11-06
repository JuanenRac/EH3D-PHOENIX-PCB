# Signal integrity estimates

Crate `eh3d-phoenix-si`, command `phoenix si`, settings `config/si.cfg`, codes `PHOENIX-SI-001` to `020` (`ERROR_CODES.md`).

**State: prototype, quasi-static.** Every number is an estimate from a closed formula applied to the geometry of the board and to its stack-up. It has been tried on boards made by hand and by the tests only. It has not been compared with a field solver, with a circuit simulator, with another tool or with a measurement of a real board, and no board has been made from it. The reference cases of `ORACLE.md` check the parts that follow from a definition (the reflection coefficient, the bounces of a lattice diagram, the rise time of a single pole, the delay of a line); the rest is listed there as pending, each with its reason.

## The command

```
phoenix si <crosstalk|reflect|eye|check> [--stackup FILE] [--net NAME] [--config FILE]
           [--out FILE] [--json] [--lang CODE] <board>
```

* `crosstalk`: the near-end (NEXT) and far-end (FEXT) crosstalk of every pair of nets that runs side by side on a layer, in percent of the step of the aggressor, with the victims of each aggressor ranked and the sum on each victim.
* `reflect`: for each net the impedance, the delay, the length in time against the rise time (long line), the diagram of bounces, the overshoot, the ringback and the settling of the voltage at the load, the proposals of termination with the result of each one, and the loss and the slowing of the edge along the line.
* `eye`: a simplified eye diagram of each net: height, width and jitter.
* `check`: all of them, the return path of the tracks and the stubs of the vias. The exit code is 1 when a finding is an error.

`--stackup` takes a stack-up job file (the file of `phoenix stackup`, with the materials of the layers); without it the defaults of `config/si.cfg` stand in for the line and the report says so. `--net` limits the report to one net: it is the aggressor in `crosstalk` and the line in `reflect` and `eye`. The command is a dry run: nothing is written unless `--out` names a new file for the JSON report, and an existing file is never replaced. Exit codes: 0 done, 1 an error finding, 2 the arguments or the input are unusable. The text of the messages is in eight languages (`--lang`); the report is the same every time.

## What comes from where

The length and the delay of a net come from `eh3d-phoenix-lengths`. The impedance, the effective permittivity, the delay and the loss of a line come from `eh3d-phoenix-stackup` for the layer and the width most of the net runs on (microstrip on an outer layer, stripline on an inner one), with the permittivity and the loss tangent of the materials of the job; the height of the dielectric between the track and the nearest reference plane and its permittivity come from the same stack-up. When the stack-up is missing or a layer cannot be taken from it, the defaults of the settings are used and a finding (`PHOENIX-SI-009`) says it: information when none was given, a warning for a layer that could not be used.

## Crosstalk

The straight pieces of two nets that run parallel (within `parallel_tolerance_deg`) and closer than `coupling_range_nm` between edges, on the same layer, are added up by pair: the length that runs parallel (at least `min_parallel_nm`), the mean distance between centres and the narrowest gap. Then:

* the mutual inductance of two thin wires over a plane by the method of images, `Lm = (mu0 / 4 pi) ln(1 + (2 h / d)^2)` with `h` the height over the reference plane and `d` the distance between centres, and the self inductance `L = Z0 sqrt(eps_eff) / c` of the line; their ratio is limited to 0.95 and a ratio above `weak_coupling_limit` is reported (`PHOENIX-SI-005`) because the formulas below are for weak coupling;
* the capacitive ratio `Cm/C` equals `Lm/L` in a homogeneous medium (a stripline); for a microstrip it is taken as `Lm/L * eps_eff / er`, an estimate made here that has no source;
* `Kb = (Cm/C + Lm/L) / 4` and `Kf = (Cm/C - Lm/L) / 2`, the coefficients of weakly coupled lines (H. Johnson and M. Graham, *High-Speed Digital Design*; C. R. Paul, *Analysis of Multiconductor Transmission Lines*; the pages are not verified). The near-end crosstalk is `Kb` times the step and saturates once the parallel stretch is longer than half the length of the edge (`2 Td >= tr`); the far-end crosstalk is `Kf * Td / tr` and is negative when the inductive coupling wins, as in a microstrip.

A pair that exceeds `next_warn_pct` or `fext_warn_pct` is a warning, and one that reaches `next_error_pct` or `fext_error_pct` an error (`PHOENIX-SI-003`, `004`). The rise time of each aggressor is `rise_time_ps` or the line `rise_time_ps.<net>`.

## Terminations and reflections

For each net: the impedance `Z0`, the delay `Td`, the bandwidth of the edge `0.35 / tr` and the criterion of a long line, `tr < 2 Td` (`PHOENIX-SI-006`, information). Only a long line is simulated.

* The diagram of bounces (method of Bergeron, the lattice diagram) is exact for a step that rises at once into the resistance of the load or an open end: the first wave is `Z0 / (Rs + Z0)`, each arrival at the load is reflected with `(RL - Z0) / (RL + Z0)` and each arrival at the driver with `(Rs - Z0) / (Rs + Z0)`.
* The voltage at the load is simulated with the method of characteristics: the driver is a ramp of duration `tr` through `Rs`, the line is lossless, and the load is any mix of a resistance, a capacitance and a series RC branch, the capacitors as trapezoidal companions. The overshoot and the ringback are in percent of the final level; the line is settled when the voltage stays within `settle_band_pct` of it.
* Proposals, each simulated: a series resistor `Z0 - Rs` at the driver, a resistor `Z0` to ground at the load, a Thevenin pair of `2 Z0` (for the steps the two resistors are in parallel and the bias is left out), and an RC branch of `Z0` and a capacitor that gives a time constant of `rc_tau_over_td` delays (a rule of thumb chosen here). The best is the one with the least ringing if it is better than the line as it is.
* The loss at the bandwidth of the edge over the length of the net is turned into a low-pass of one pole that has the same loss at that frequency; its rise time `0.35 / fc` adds in quadrature to the rise time of the driver. The report lists the rise time after a quarter, a half, three quarters and the whole length (`PHOENIX-SI-018` when the edge slows by more than `rise_degradation_warn_pct`).

The driver and the load are the settings `rs_ohm`, `rl_ohm` (0 is an open end) and `cl_pf`, or their per-net lines. Branches, the pads and the packages of the parts and the vias are not in the model.

## Eye diagram

A pseudo-random sequence with a generator of its own and a fixed seed (PRBS7 `x^7 + x^6 + 1`, 127 bits, or PRBS9 `x^9 + x^5 + 1`, 511 bits; `prbs`) at `bit_rate_mbps` goes through the rise time of the driver and a filter of one or two poles (`eye_order`) whose loss at half the bit rate is the loss of the line at that frequency. The delay of the line only shifts the picture and is left out. The second of two periods is folded over one bit and measured: the height is the widest opening between the lowest one and the highest zero, as a fraction of the swing; the jitter is the spread of the crossings of the middle level, which the interference between bits makes; the width is `1 - jitter / bit`. A height or a width below `eye_height_warn` or `eye_width_warn` is a warning, and below half of it an error (`PHOENIX-SI-010`, `011`). There are no reflections, no crosstalk and no dispersion in it.

## Return path and stubs (`check`)

A track on a copper layer is looked at over the zones of the copper layers next to it in `copper_stack`. It is sampled every `plane_sample_nm`; the zone each sample lies in (by the outline of the zone, not by its fill with antipads and thermal reliefs) is compared with the one of the sample before. A track is reported (`PHOENIX-SI-012`) when every neighbour that has zones changes along it, which is the edge of the plane or a split between zones of two nets, or when it runs over no copper of them. When no neighbour has a zone, or the board has no order of copper layers, a note says so (`PHOENIX-SI-019`).

A via stub is the part of the barrel beyond the last layer that has the end of a track of the net at the via, measured in the stack-up between the middles of the copper (or a share of `default_via_length_nm` without one). It is reported (`PHOENIX-SI-013`) when it is longer than `max_stub_nm` or when its round trip takes more than `stub_tr_fraction` of the rise time; it is an error when the quarter-wave resonance, `1 / (4 Td)` of the stub with `via_delay_ps_per_mm`, is below the bandwidth of the edge. The effect of a crossing or a stub on the signal is not estimated.

## What is not in it

Pieces on different layers (broadside coupling), arcs, branches and the pads of the parts are not looked at in the crosstalk; the lines are lossless in the reflections and have a loss that is a single pole in the rise time and the eye; skin effect and dielectric loss are taken at one frequency; the driver is a resistance and the receiver a resistance and a capacitor; there are no packages, no vias in the reflections, no power or ground noise, no jitter other than the interference between bits, and no statistical eye. See `NOT_YET.md`.
