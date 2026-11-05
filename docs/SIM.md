# EH3D-PHOENIX-PCB: circuit simulation

The crate `eh3d-phoenix-sim` and the command `phoenix sim` are the first step of phase F12 (requirement PPCB-022). They simulate linear circuits and circuits with diodes with a solver of our own, by modified nodal analysis, and link no engine (the decision is `docs/adr/0010-own-circuit-simulator.md`, provisional).

**State: prototype.** The results are an own numerical approximation. The tests use values of the textbooks and properties (superposition, conservation of current, repeatable results), and a coherence check compares the simulation of a filter with the RF crate. Nothing has been compared with SPICE or with a measurement, and there are no transistors, no junction capacitance and no series resistance of the diode.

## The netlist

A SPICE-style text. The first line is the title (as in SPICE it is never an element). A line starting with `*` is a comment, `;` starts a comment in the rest of a line, and a line starting with `+` continues the one before. Node names are not case sensitive (they are kept in lower case); `0` and `gnd` are ground. Element names are not case sensitive either (`r1` and `R1` are the same element).

| Line | Element |
|---|---|
| `Rname n1 n2 value` | resistor (above zero) |
| `Cname n1 n2 value [ic=v]` | capacitor, with an optional initial voltage |
| `Lname n1 n2 value [ic=i]` | inductor (above zero), with an optional initial current |
| `Vname n+ n- [DC v] [AC mag [phase]] [SIN(...) \| PULSE(...) \| PWL(...)]` | voltage source (a bare number is the DC value) |
| `Iname n1 n2 ...` | current source; the current flows from `n1` to `n2` through the source |
| `Ename n+ n- nc+ nc- gain` | voltage-controlled voltage source |
| `Gname n1 n2 nc+ nc- gm` | voltage-controlled current source |
| `Fname n1 n2 Vsource gain` | current-controlled current source |
| `Hname n+ n- Vsource rm` | current-controlled voltage source |
| `Kname L1 L2 k` | coupling of two inductors, `0 < k < 1` |
| `Dname anode cathode model` | diode |
| `Xname n1 n2 ... subckt` | instance of a subcircuit |

* Values take the suffixes of SPICE, not case sensitive: `f p n u m k meg g t` and `mil`. Letters after the suffix are a unit that is ignored (`10kOhm`, `1uF`). As in SPICE, `M` is milli and `F` is femto.
* `SIN(vo va freq td theta phase)`, `PULSE(v1 v2 td tr tf pw per)` and `PWL(t1 v1 t2 v2 ...)`. The DC value of a source with a waveform and no DC value is where the waveform starts (`vo`, `v1`, the first point).
* `.param name=value ...` defines parameters; a value, a gain or an initial condition may be an expression between braces, `{2*r1+1k}`, with `+ - * / ^`, parentheses, `pi`, `sqrt abs exp ln log10 sin cos` and the parameters defined before.
* `.model name D(IS=.. N=..)` is the only model; other parameters of a diode (`RS`, `CJO` ...) are read and left out, and the line is noted in `Netlist::ignored`.
* `.subckt name ports ... / .ends` defines a subcircuit (without parameters, and not inside another). It is expanded when the circuit is compiled: its nodes and elements get the prefix of the instance (`x1.m`, `X1.R1`). The nesting is bounded by `max_subckt_depth`, which also stops a subcircuit that calls itself.
* `.op`, `.ac`, `.tran`, `.print`, `.options` and the other directives are read and left out: the program asks for the analysis.

Errors name a code and the line of the netlist: a line that is not valid (`PHOENIX-SIM-001`), a value (`-002`), an element that is not supported (`-003`), a name used twice (`-004`), a reference to a model, a subcircuit, a parameter, an inductor or a voltage source that is not there (`-005`), a bad subcircuit (`-006`), a limit of the settings (`-008`). `to_spice` writes a netlist back in a fixed order (parameters and models sorted by name, then the subcircuits, then the elements), and what it writes reads back as the same netlist.

## The analyses

The unknowns are the voltages of the nodes (not ground) and the currents of the voltage sources, inductors, and voltage-controlled and current-controlled voltage sources, named `v(node)` and `i(element)`. The current of a voltage source is the one that flows through it from the positive to the negative node, as in SPICE, so a source that feeds a load has a negative current.

* **Operating point** (`--op`). Capacitors are open, inductors are short, sources take their DC value. A circuit with diodes is solved by Newton-Raphson, with the limiting of the junction voltage of SPICE; when the plain iteration does not converge, the sources are raised from zero in `source_steps` steps (the result says which strategy was used). A node with no DC path to ground is reported with its name (`PHOENIX-SIM-010`) before the matrix is built; a matrix that cannot be solved (two voltage sources in parallel, a loop of inductors) names the unknown that is left undetermined (`-009`); an iteration that does not converge is `-011`.
* **AC** (`--ac KIND N F1 F2`). The circuit is linearised at the operating point (a diode becomes its conductance) and solved in complex numbers at each frequency. `lin` gives N points in all, `dec` N per decade, `oct` N per octave; the stop frequency is added when it is not on the grid. For each unknown the table has the magnitude, the dB, the phase in degrees and the group delay `-dφ/dω`, taken from the phase a fraction `delay_step_fraction` of the frequency below and above it.
* **Transient** (`--tran STEP STOP`). Backward Euler or the trapezoidal rule (`--method be|trap`; trapezoidal by default). The first step is always backward Euler, which does not need the current of the capacitors before it. With `--adaptive` the answer of one step is compared with the answer of two half steps; the step shrinks when the difference is above the tolerances (`reltol`, `vntol`, `abstol`) and grows, up to the step asked for, when it is below. A step at its minimum with an error above the tolerance is kept and noted (`PHOENIX-SIM-019`). The analysis starts from the operating point, or with `--uic` from the initial conditions (`ic=` of capacitors and inductors, zero when missing). The sources take the value of their waveform at the time of each step.

The equations are solved by Gaussian elimination with partial pivoting written for this crate. A pivot that is zero, not finite or smaller than `pivot_tolerance` times the largest value of its column means the matrix is singular. The size is bounded by `max_unknowns`, the number of points of an AC sweep by `max_ac_points`, the steps of a transient by `max_tran_steps` and the size of a result by `max_result_cells`.

## The netlist of a schematic

`netlist_from_schematic` (and `phoenix sim schematic.json ...`, for the `design/schematic.json` of a project) turns the pins of each part into nodes through the nets, and the value of the part into the value of the element. The nets whose names are in `schematic_ground` (`0`, `gnd`, `agnd`, `dgnd`, `pgnd`) become node 0; nets with the same name are one node; a name is cleaned to letters, digits and underscores in lower case.

The kind of an element comes from the reference designator, which must be one letter followed by digits: `R C L` (two pins, the value is the number), `V I` (two pins, the first is the positive one; the value is what follows the nodes in a netlist: `5`, `DC 5`, `SIN(0 1 1k)`), `D` (two pins; the anode is the pin `A` when the pins are named `A` and `K`, otherwise the first; the value is the name of the model, and a model with the default parameters of the settings is made for it), `E G` (four pins; the value is the gain), `F H` (two pins; the value is the controlling source and the gain, `V1 2`) and `K` (no pins; the value is `L1 L2 k`). Pins are taken in the order of their numbers.

Nothing is dropped silently. The report lists the parts that were translated and, with a code and the reason, the parts that were not (a kind that is not simulated, a number of pins that does not fit, a value that is missing or that does not read), the pins that are on no net (`PHOENIX-SIM-020`) and a schematic without a ground net (`-015`). The report is printed to the error output by the command and can be read in the eight languages.

## The check against the RF filters

`compare_filter` turns a ladder filter of `eh3d-phoenix-rf` (Butterworth, Chebyshev or Bessel; low-pass, high-pass, band-pass or band-stop) into a netlist, simulates it in AC over `coherence_points` frequencies from a tenth of its lowest edge to ten times its highest, and compares the magnitude (dB), the phase (degrees) and the group delay (as a fraction of a period) of the output with the response that the RF crate computes from ABCD matrices. The tolerances are in the settings (`coherence_db_tolerance`, `coherence_phase_tolerance_deg`, `coherence_delay_tolerance`); a difference above them is `PHOENIX-SIM-018` with the figures. `phoenix sim --check-filter butterworth 3 lowpass 1meg` runs it from the command line. The two methods share nothing, so a difference means that one of them is wrong; agreement shows that they are consistent, not that either is right about a real filter.

## Output

CSV and JSON tables, deterministic: the same circuit and the same request give the same bytes, with no date, host name or program version. Numbers are written in the shortest form that reads back as the same number (`1.5e-3`). The JSON says `"kind": "own numerical approximation (prototype, not compared with SPICE)"`. `--probe v(out),i(v1)` chooses the unknowns; `--out BASE` writes `BASE.csv` and `BASE.json`, each as a temporary file and a rename (the folder is made when it is missing, and the rename is tried again `write_retries` times when another program holds the file).

## The command

```
phoenix sim [--config FILE] [--lang xx] [--json] [--out BASE] [--probe LIST] <input> <analysis>
phoenix sim [--config FILE] [--lang xx] --check-filter FAMILY ORDER BAND F1 [F2] [--ripple-db DB] [--z0 OHM]
```

* `<input>` is a netlist, or a `.json` file with the schematic of a project.
* The analysis is `--op`, `--ac KIND N F1 F2`, `--tran STEP STOP [--uic] [--adaptive] [--method be|trap]` or `--netlist` (prints the netlist and stops).
* Numbers take the suffixes of SPICE (`1k`, `10meg`, `4.7u`).
* Exit code 0 when done, 1 when `--check-filter` finds a difference above the tolerances, 2 when the arguments, the settings, the input or the circuit are unusable.

## Settings and codes

`config/sim.cfg` holds every limit, tolerance and default. A missing line keeps the default; an unknown key or a value out of range is refused with its line number (`PHOENIX-SIM-007`). The codes are `PHOENIX-SIM-001` to `PHOENIX-SIM-021` in `docs/ERROR_CODES.md`, with titles in the eight languages.

## Tests and limits

The tests use the values of the textbooks: the voltage divider, the RC circuit at one time constant (63.2 %, with backward Euler and with the trapezoidal rule), the RL current, the corner of the RC low-pass at `1/(2πRC)` (-3.01 dB, -45 degrees, half the time constant of group delay), a series RLC with its resonant frequency, unit gain at resonance, the half-power points and `Q = f0 / bandwidth`, the half-wave rectifier (a peak one diode drop below the source, compared with the fixed point of the Shockley equation), and a transformer made of two inductors coupled with `k` near 1 (turns ratio `sqrt(L2/L1)` times `k`, turned by half a turn). The properties are superposition in linear circuits, the conservation of current at every node and two runs with the same bytes. There are tests of the reader (units, expressions, continuation lines, subcircuits, parameters, every error with its line, damaged text that must not panic), of the writer (a netlist survives writing and reading), of the settings, of the codes (a title in each language, documented) and of the command.

Not done: nothing was compared with SPICE or with a measurement; there are no transistors, no operational-amplifier models, no junction capacitance, series resistance or breakdown of the diode, no temperature sweep, no noise analysis, no `.step` and no `.measure`; the solver is dense and the number of unknowns is limited; a circuit that SPICE solves with its extra convergence aids may not converge here; the adaptive step does not stop at the corners of a pulse; and the netlist of a schematic knows only the parts whose reference designator is one of the supported letters.
