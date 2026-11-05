# 0010: a circuit simulator of our own for linear circuits, diodes, transistors and op-amps (provisional)

Status: proposed. It becomes accepted, or is replaced, after the checks listed below have been made against an existing engine.

## Context

Phase F12 asks for circuit simulation with reference cases that have analytic solutions, and `docs/STATUS.md` left open which existing engine to bind. Binding an engine needs its licence and its platform support checked, a way to run it (a library, or a program that is started and read back), and a netlist writer that the engine accepts. None of that is settled, and the first uses (a divider, a filter, the response of a rectifier, a check of a filter designed by the RF tools) need very little of an engine.

## Decision (provisional)

The first simulator is **our own**, in the crate `eh3d-phoenix-sim`, with no dependency beyond the complex numbers of the RF crate:

* modified nodal analysis of linear circuits (resistors, capacitors, inductors, coupled inductors, independent sources and the four dependent sources) and of circuits with non-linear devices (diodes with series resistance, junction capacitance, transit time and breakdown; bipolar transistors; MOSFETs of level 1; op-amp macromodels and comparators), by Newton-Raphson with junction limiting, with gmin stepping and source stepping as fallbacks;
* a dense solver written for this crate (Gaussian elimination with partial pivoting) that detects a singular matrix, names the unknown it leaves undetermined and has a size limit;
* a netlist of our own in memory, with a reader and a writer of SPICE-style text, and the netlist of a schematic with a report of everything that could not be translated;
* DC operating point, AC sweeps with group delay, noise at an output (thermal and shot) and transient analysis (backward Euler and trapezoidal, fixed step and an adaptive step that stops at the corners of the sources);
* every limit and tolerance in `config/sim.cfg`, the codes `PHOENIX-SIM-<NNN>` with titles in the eight languages;
* every result says what it is: an own numerical approximation, a prototype, not compared with SPICE (`docs/DECISIONS.md`, every result states its kind).

The choice of an existing engine is not closed. It stays the way to **validate** this solver: when an engine is chosen, the reference cases run through both and the differences are recorded.

## Extension: devices

The first version had no transistors, and the last point of the list of things to be shown asked for a decision on them. The decision, still provisional, is to **write the models of our own**, simple ones, and not to bind an engine for everything that is not linear or a diode:

* the diode, the bipolar transistor (the transport form of Ebers-Moll with the forward Early effect, parasitic resistances, depletion and diffusion charges), the level 1 MOSFET and the op-amp and comparator macromodels are a few hundred lines of equations that can be checked against the formulas of the textbooks (rest points, `-gm * RC`, the mirror ratio `BF / (BF + 2)`, the threshold of a CMOS inverter, the gain `1 + R2 / R1`, the ripple of a rectifier, the regulation of a Zener diode), which is the same gate as for the first version;
* the devices share one way of being solved: each is linearised into pieces of the form "the current that leaves a node is a constant plus conductances times voltages", which the operating point, the AC analysis, the transient analysis and the noise analysis all use, so a new device is one function;
* the charge of a junction is a function of its voltage, integrated with the same two methods as a capacitor, so the capacitances and the transit times are not special cases;
* the aids of convergence (limiting of the junction voltages, gmin stepping, source stepping) are the plain ones; they are what the risk below is about;
* the models are not SPICE models. The parameters of a library of a manufacturer will mostly be read and left out (the reader says so), so a manufacturer's model is not a reason to trust a result. This is the main reason to keep the option of an engine for comparison.

## Reasons

* It works today, with no licence to check, on every platform the project builds on, and it needs no program to be started.
* The first cases need only linear algebra and a small non-linear iteration; the cases can be checked against the formulas of the textbooks, which is the gate of the phase.
* The coherence check against the filters of `eh3d-phoenix-rf` compares two unrelated methods (nodal analysis and ABCD matrices), so it finds a mistake in either.
* A netlist of our own is the model that any engine will need to be fed from, so it is not wasted work.

## Risks

* A solver that is correct on a divider may be wrong, or slow, on circuits nobody has tried. The solver is dense: the time grows with the cube of the number of unknowns, which the limit `max_unknowns` bounds.
* Convergence of the non-linear cases is the hard part of a simulator. The techniques are the plain ones (limiting, gmin stepping and source stepping), and a circuit that SPICE solves may not converge here; a loop with a high-gain op-amp that clips is the likely case. The soft limit of the op-amp output (a hyperbolic tangent) is a choice made for convergence, and it makes the open-loop transfer curve rounder than a real one near the rails.
* The reader of netlists covers a part of the SPICE language and says so with a code; it is not a replacement for a netlist written for another program.
* Nothing here has been compared with SPICE or with a measurement.

## What has to be shown before this is accepted

* The reference cases of the phase (a divider, first and second order circuits, a resonant circuit, a rectifier, a transformer with a coupling coefficient) run through an existing engine and through this solver, with the differences inside a tolerance that is justified and written down.
* A transient case with a switch-like source (a pulse or a rectifier) at several steps, with the error against the exact solution measured and the adaptive step compared with a fixed step of the smallest size.
* At least one circuit of a size that matters (a few hundred unknowns) timed, so the limit of the dense solver is known.
* The models of the devices run through an existing engine and through this solver on a set of circuits (an amplifier, a mirror, an inverter, an op-amp circuit, a rectifier, a regulator) with the differences recorded. The decision on transistors and op-amps (models of our own, simple) stays provisional until this is done.
