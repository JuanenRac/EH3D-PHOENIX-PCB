# EH3D-PHOENIX-PCB: advanced circuit analyses

This page describes what the simulator of `docs/SIM.md` does on top of one analysis: runs of a circuit over a list of values (`.step`), values taken from the results (`.measure`), Monte Carlo over the tolerances of resistors, capacitors and inductors, the sensitivity of a measure to a parameter, the thermal noise at an output, the poles and zeros of a linear circuit, and the figures of merit of a filter or an amplifier. It all lives in the library `eh3d-phoenix-sim` (modules `measure`, `step`, `montecarlo`, `random`, `study`, `poles` and `report`), needs no screen, and is reached from the command `phoenix sim` and from the schematic by `phoenix sim --schematic`.

**State: prototype.** The results are an own numerical approximation. They are checked against closed-form solutions of textbooks (each reference case in `tests/oracle/sim.json` names its source and its tolerance) and against properties (the same seed gives the same table, a step on an element equals the circuit written with that value). Nothing has been compared with SPICE or with a measurement, and the device models are the simple ones of `docs/SIM_NONLINEAR.md`.

## Measures

A measure is one line, in a netlist after `.measure` and on the command line after `--measure`:

```text
tran NAME at SIGNAL TIME
tran NAME max|min|avg|rms|pp SIGNAL [from=T1] [to=T2]
tran NAME cross SIGNAL LEVEL [rise|fall|any] [n=K] [from=T]
tran NAME rise|fall SIGNAL [low=0.1] [high=0.9] [from=T1] [to=T2]
dc   NAME ...                          the same words; the horizontal axis is the swept value
ac   NAME gain|gaindb|phase SIGNAL FREQ
ac   NAME bw SIGNAL [ref=first|max]
ac   NAME peak|peakfreq|qband|qlow|ugf|phasemargin|gainmargin SIGNAL
```

`SIGNAL` is `v(node)` or `i(source)`. Numbers take the suffixes of SPICE (`1k`, `10meg`, `4.7u`). A line may have spaces around `=`; it is written back normalised.

| Kind | Result |
|---|---|
| `at` | the value at a time (or at a swept value), by a straight line between the points |
| `max`, `min`, `pp` | the largest value, the smallest value, and their difference, in the window |
| `avg`, `rms` | the mean and the root mean square over the window; the integrals are exact for straight lines between the points |
| `cross` | the time of the K-th crossing of a level, in a direction, after `from` |
| `rise`, `fall` | the time between two fractions of the swing of the window (the smallest to the largest value of the window); `low` and `high` default to `measure_low` and `measure_high` of `config/sim.cfg` (10 % and 90 %). The window has to reach the settled value, or the swing is the one it sees |
| `gain`, `gaindb`, `phase` | the magnitude, the magnitude in dB and the phase in degrees at a frequency; the analysis is run at that frequency itself |
| `bw` | the -3 dB bandwidth. With `ref=first` (default) the level is 3.01 dB under the gain at the first frequency and the result is the frequency where the gain falls through it (the cutoff of a low-pass response). With `ref=max` it is the width between the two -3 dB frequencies around the peak (a band-pass response) |
| `peak`, `peakfreq` | the largest magnitude and its frequency, refined by the parabola through the point and its two neighbours |
| `qband` | the quality factor of a band-pass response: the peak frequency over the `ref=max` bandwidth |
| `qlow` | the quality factor of a second-order low-pass response: the gain at the frequency where the phase is -90 degrees over the gain at the first frequency |
| `ugf` | the frequency where the magnitude falls through 1 |
| `phasemargin` | 180 degrees plus the phase at the unity-gain frequency, with the phase unwrapped from the principal value of the first point; `SIGNAL` is the loop gain, positive at low frequency |
| `gainmargin` | minus the gain in dB at the frequency where the unwrapped phase falls through -180 degrees |

The kinds that look for a crossing interpolate between the points of the sweep in the logarithm of the frequency, so the error depends on the density of the sweep: about 1e-4 relative with 100 points per decade for the cases of `tests/oracle/sim.json`, which is what the tolerances there allow for. A measure that is well formed and finds nothing (a level never reached, no -3 dB point in the sweep) has no value: an empty cell in the CSV, `null` in the JSON. A measure that cannot be taken (a signal that is not in the circuit, an analysis nobody asked for) stops the run with `PHOENIX-SIM-027`; a line that is not valid stops it with `PHOENIX-SIM-026` and names its line.

The analyses a measure reads are the ones the netlist asks for (`.tran`, `.ac`, `.dc`, the first of each kind) or the one named on the command line.

## Steps

```text
.step [param] TARGET lin START STOP STEP
.step [param] TARGET dec POINTS START STOP        (POINTS per decade)
.step [param] TARGET oct POINTS START STOP        (POINTS per octave)
.step [param] TARGET list V1 V2 V3 ...
```

With `param` the target is a `.param` of the netlist, and the netlist is read again with the new value, so every expression that uses it follows. Without it the target is an element of the top level: the value of a resistor, a capacitor or an inductor, the DC value of an independent source (a waveform is not changed), or the gain of a controlled source of voltage or current. Several `.step` lines make a grid, the first the outermost. `max_step_points` limits one grid and `max_step_runs` the number of combinations. A step on a parameter needs a netlist file (it is read again); a schematic can step its elements.

## Monte Carlo

A tolerance is a fraction below 100 %: `tol=5%` or `tol=0.05` after the value of a resistor, a capacitor or an inductor (`dist=uniform` or `dist=normal` after it chooses the distribution), `tol=5%` in a rule of `config/sim_models.cfg`, the attribute `SIM.TOL` of a part of a schematic, or `--tol R=1%,C=10%,L=20%` for the elements that have none. A run multiplies each of those values by a random factor:

* uniform: `1 + tol u`, with `u` evenly in `[-1, 1)`; the deviation is `tol/sqrt(3)`;
* normal: `1 + (tol/k) z`, with `z` of the standard normal distribution cut at `k` (`mc_sigmas`, 3 by default): the tolerance is the band of `k` standard deviations and no draw falls outside it.

`mc_normal` in `config/sim.cfg` chooses the distribution of a tolerance that does not say. The generator is SplitMix64 written in `random.rs`, with the normal numbers from the Box-Muller transform; nothing reads the clock or the system. Run number `r` has a stream of its own made from the seed and `r`, so the same seed gives the same table on every machine, run 7 does not depend on the runs before it, and every point of a `.step` grid sees the same draws. `phoenix sim --mc [N] --seed S` sets the number of runs and the seed (`mc_runs` and `mc_seed` of `config/sim.cfg` when they are not given); `max_mc_runs` limits N and `max_study_cells` the size of the table.

The table has one row per run, and after the runs of each point of the grid the rows `count`, `mean`, `std` (the sample deviation, divided by `n - 1`), `min`, `max` and one for each percentile of `mc_percentiles` (by linear interpolation between the sorted values). A tolerance inside a subcircuit is refused; model parameters of devices have no tolerance; tolerances are independent (no correlation between two parts, no worst-case analysis).

## Sensitivity

`--sens TARGET[,TARGET]` runs the circuit twice around the nominal value of each target (an element, or `param:NAME`), `sens_rel_step` (0.001) of the value either side, and gives for each measure the derivative (central difference) and the relative sensitivity, the derivative times the value over the measure. A target whose value is zero is refused.

## Noise

`phoenix sim --ac KIND N F1 F2 --noise v(out)` gives the noise at a node over the sweep: the density in V/sqrt(Hz), the power in V^2/Hz, the contribution of each source, and in the JSON the total in the band (the density integrated by the trapezoidal rule over the frequencies of the sweep). Each resistor contributes the thermal noise `4 k T R`, summed in power (the sources are not correlated) after the current of each is carried to the output through the circuit; `k` is the Boltzmann constant of the SI (1.380649e-23 J/K) and the temperature is `temperature_c` of `config/sim.cfg`. Diodes, bipolar transistors and MOSFETs add their shot and channel noise. There is no flicker noise, no noise of the op-amp macromodel, and no noise referred to the input. With a dense sweep over the whole band, the total of a resistor across a capacitor is `sqrt(kT/C)` (a reference case).

## Poles and zeros

`--poles SIGNAL` fits the AC response of a signal by a ratio of two polynomials of the same degree `n`, `N(s)/D(s)` with `D(0) = 1`, by least squares on the relative error (Sanathanan-Koerner reweighting, `pole_fit_iterations` passes, Householder reflections, the frequencies scaled by the geometric mean of the sweep). The orders 1 to `max_pole_order` are tried in turn (`--pole-order N` fixes one) and the first whose largest relative error is within `pole_fit_tolerance` is kept; the poles and zeros are the roots of `D` and `N` (Durand-Kerner iteration), in rad/s, with the frequency in Hz and the quality factor `|p|/(2|Re p|)` of a complex pair. A term of `N` that adds less than `pole_trim` of the largest term at the top of the band is taken as zero.

This is a fit, not an eigenvalue problem of the matrices of the circuit, and it is only as good as the response it is given:

* the sweep must hold the poles and zeros you want; one far outside the band has almost no effect on the response and is not found accurately;
* the model has `D(0) = 1`, so a pole at the origin (an ideal integrator) is not covered;
* a response that no rational function of up to `max_pole_order` poles fits within the tolerance (a transmission line, a pure delay) is refused with `PHOENIX-SIM-029`, with the best error in the message. The fit never returns poles of a model whose error is above the tolerance, and the largest relative error is in the output.

## Figures of merit

`--figures SIGNAL` writes one table for a response: the gain at the first frequency, the peak gain and its frequency, the -3 dB cutoff of a low-pass response, the quality factors of a second-order low-pass and of a band-pass response, the unity-gain frequency and the phase and gain margins. A figure that does not exist for the response is empty. They are the same computations as the measures above, so the same limits apply: a figure that assumes a shape (the quality of a second-order low-pass from the phase at -90 degrees) is meaningless for another shape.

## The tables

| Output | Format |
|---|---|
| a study (`--step`, `--measure`, `--mc`, `--sens`, the `.step` and `.measure` lines) | CSV: one column per step, the column `run` (`nominal`, the number of the run, or the name of a statistic or `d(target)` and `rel(target)`), one column per measure; JSON `phoenix-sim-measure/1` |
| `--noise` | CSV with the frequency, the density, the power and the power of each source; JSON `phoenix-sim-noise/1` |
| `--poles` | CSV with kind, index, real and imaginary parts, frequency and quality; JSON `phoenix-sim-poles/1` with the order, the gain at zero frequency and the fit error |
| `--figures` | CSV of name, value and unit; JSON `phoenix-sim-figures/1` |

All of them are deterministic: no date, no host and no program version, and the numbers are written in the shortest form that reads back as the same number. `--out BASE` writes `BASE.csv` and `BASE.json` as for the other analyses (a file that another program holds is tried again `write_retries` times).

## Settings

`config/sim.cfg` holds the limits and defaults of this page, each with a comment: `max_step_points`, `max_step_runs`, `max_mc_runs`, `mc_runs`, `mc_seed`, `mc_sigmas`, `mc_normal`, `mc_percentiles`, `max_measures`, `max_study_cells`, `measure_low`, `measure_high`, `sens_rel_step`, `max_pole_order`, `pole_fit_iterations`, `pole_fit_tolerance`, `pole_root_iterations`, `pole_root_tolerance` and `pole_trim`. An unknown key or a value out of range is refused with its line (`PHOENIX-SIM-007`). The codes of this page are `PHOENIX-SIM-025` (a `.step`), `-026` (a `.measure`), `-027` (a measure that cannot be computed), `-028` (Monte Carlo and tolerances) and `-029` (poles and zeros), in `docs/ERROR_CODES.md` with titles in the eight languages; a wrong option of the command is `-016`.

## Reference cases

The cases in `tests/oracle/sim.json` (`phoenix oracle run --suite sim`) are closed-form solutions with their sources: the 10 % to 90 % rise time of an RC circuit (`ln 9 RC`, also with `R` stepped), the cutoff of an RC low-pass (`1/(2 pi R C)`), the bandwidth and the quality factor of a series RLC band-pass circuit (`R/(2 pi L)`, `sqrt(L/C)/R`), the quality factor of a second-order low-pass with `Q = 1/sqrt(2)`, the phase margin of a two-pole loop with a Butterworth closed loop (65.53 degrees), the gain and phase margins of the loop `K/(1 + s/w0)^3`, the peak of the step response of an underdamped RLC circuit, the thermal noise `sqrt(4 k T R)` and the total noise `sqrt(kT/C)`, the mean and the deviation of a divider with tolerances against the propagation of uncertainty of the Guide to the expression of uncertainty in measurement, and the frequency and quality factor of the poles of an RC and of a series RLC circuit. Two cases wait in `tests/oracle/pending.json` with their reason: the poles of a fifth-order ladder against the handbook values, and the percentiles of a Monte Carlo study against the normal distribution.

## What is left out

Flicker noise and the noise of the op-amp macromodel; correlated tolerances, tolerances of model parameters and of sources, and worst-case analysis; steps of a model parameter or of the temperature; poles at the origin, a Padé or delay model, and the eigenvalues of the matrices of the circuit; the poles of a non-linear circuit other than at its operating point; the plot of a table (the viewer does not draw it). See `docs/NOT_YET.md`.
