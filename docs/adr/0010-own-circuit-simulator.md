# 0010: a circuit simulator of our own for linear circuits and simple diodes (provisional)

Status: proposed. It becomes accepted, or is replaced, after the checks listed below have been made against an existing engine.

## Context

Phase F12 asks for circuit simulation with reference cases that have analytic solutions, and `docs/STATUS.md` left open which existing engine to bind. Binding an engine needs its licence and its platform support checked, a way to run it (a library, or a program that is started and read back), and a netlist writer that the engine accepts. None of that is settled, and the first uses (a divider, a filter, the response of a rectifier, a check of a filter designed by the RF tools) need very little of an engine.

## Decision (provisional)

The first simulator is **our own**, in the crate `eh3d-phoenix-sim`, with no dependency beyond the complex numbers of the RF crate:

* modified nodal analysis of linear circuits (resistors, capacitors, inductors, coupled inductors, independent sources and the four dependent sources) and of circuits with diodes (the Shockley equation, Newton-Raphson with voltage limiting and source stepping as a fallback);
* a dense solver written for this crate (Gaussian elimination with partial pivoting) that detects a singular matrix, names the unknown it leaves undetermined and has a size limit;
* a netlist of our own in memory, with a reader and a writer of SPICE-style text, and the netlist of a schematic with a report of everything that could not be translated;
* DC operating point, AC sweeps with group delay and transient analysis (backward Euler and trapezoidal, fixed step and a simple adaptive step);
* every limit and tolerance in `config/sim.cfg`, the codes `PHOENIX-SIM-<NNN>` with titles in the eight languages;
* every result says what it is: an own numerical approximation, a prototype, not compared with SPICE (`docs/DECISIONS.md`, every result states its kind).

The choice of an existing engine is not closed. It stays the way to **validate** this solver: when an engine is chosen, the reference cases run through both and the differences are recorded.

## Reasons

* It works today, with no licence to check, on every platform the project builds on, and it needs no program to be started.
* The first cases need only linear algebra and a small non-linear iteration; the cases can be checked against the formulas of the textbooks, which is the gate of the phase.
* The coherence check against the filters of `eh3d-phoenix-rf` compares two unrelated methods (nodal analysis and ABCD matrices), so it finds a mistake in either.
* A netlist of our own is the model that any engine will need to be fed from, so it is not wasted work.

## Risks

* A solver that is correct on a divider may be wrong, or slow, on circuits nobody has tried. The solver is dense: the time grows with the cube of the number of unknowns, which the limit `max_unknowns` bounds.
* Convergence of the non-linear cases is the hard part of a simulator. There are only diodes, without series resistance or junction capacitance, and the techniques are the plain ones (limiting and source stepping); a circuit that SPICE solves may not converge here.
* The reader of netlists covers a part of the SPICE language and says so with a code; it is not a replacement for a netlist written for another program.
* Nothing here has been compared with SPICE or with a measurement.

## What has to be shown before this is accepted

* The reference cases of the phase (a divider, first and second order circuits, a resonant circuit, a rectifier, a transformer with a coupling coefficient) run through an existing engine and through this solver, with the differences inside a tolerance that is justified and written down.
* A transient case with a switch-like source (a pulse or a rectifier) at several steps, with the error against the exact solution measured and the adaptive step compared with a fixed step of the smallest size.
* At least one circuit of a size that matters (a few hundred unknowns) timed, so the limit of the dense solver is known.
* A decision on transistors and operational amplifiers: models of our own, or the existing engine for everything that is not linear or a diode.
