# Reference cases and the plan to compare with other tools

Everything that EH3D-PHOENIX-PCB calculates is a prototype until it is compared with a reference. This page has two parts: the reference cases that run today (`phoenix oracle`, crate `eh3d-phoenix-oracle`, files in `tests/oracle/`), and the plan to compare the results with EAGLE, KiCad and other tools, which has **not been run**.

## What the cases are, and what they are not

A case says: with these inputs, a check of this repository must give this number within this tolerance, and the number comes from this source. The number is one of three kinds:

* a value that an open source publishes (a standard, a textbook example, a table of constants);
* a value that follows exactly from a definition or a closed-form solution (the speed of light, the gain of an RC filter at its corner frequency, the area of a rectangle);
* a value of a published closed formula, evaluated for the inputs of the case (the formulas of IPC-2141 for the impedance of a microstrip), used to cross-check an implementation that is built on a different, usually more accurate, formula.

Passing the cases shows that a calculation reproduces a published formula or a closed-form solution within the tolerance written next to it. It does **not** show that the calculation agrees with a field solver, with the simulator of another maker, with EAGLE or KiCad, or with a measurement of a board. The state of the capabilities in `REQUIREMENTS.md` does not change because the cases pass; it changes only after the comparisons planned below are made on cases that someone else can repeat.

A number is never entered from memory. When a value cannot be cited with certainty the case is **pending**: it names the source that will be used, says why it cannot be run, and carries no number. A cited case needs a source with its place in it, the unit, the tolerance and a note that says who chose the tolerance and why. Where the source publishes an accuracy, the tolerance follows it; where it does not, the tolerance is chosen here, says so, and is wider than the known gap between the two formulas only when that gap is named.

## The files

| File in `tests/oracle/` | Suite | What it holds |
|---|---|---|
| `stackup.json` | `stackup` | constants (c, mu0, eta0), the complete elliptic integral, the zero-thickness stripline at the width where Z0 = eta0/4, an air microstrip, the 50 ohm microstrip of a textbook example, closed-form cross-checks of microstrip and stripline against IPC-2141, the differential fits, the skin depth of copper and the delay of a stripline |
| `rf.json` | `rf` | VSWR, return loss and mismatch loss of a mismatched load, the quarter-wave transformer, the input impedance of lossless lines of a quarter, an eighth and a half wave, the guided wavelength |
| `sim.json` | `sim` | dividers, the corner frequency and phase of an RC filter, series RLC circuits at resonance (gain across the resistor and Q across the capacitor), the step response of an RC and of an underdamped RLC circuit |
| `board.json` | `board` | boards whose answer follows from plane geometry: the clearance of two parallel tracks against a rule, the length and the delay of a straight track, the copper area of a zone with and without a track across it |
| `pending.json` | `pending` | the cases that cannot be run yet, each with its reason |

Each file has the format `phoenix-oracle/1`. A case has `id` (`ORACLE-<AREA>-<NNN>`, unique), `title`, `area`, `check` (the name of the check that is run), `inputs`, `expected`, `unit`, `tolerance` (`abs`, `rel` or both; the larger bound counts), `tolerance_note`, `source`, `status` (`cited` or `pending`) and, for a pending case, `reason`. Unknown fields are refused.

The checks (`phoenix oracle list` shows the cases, the crate lists the checks) call the real functions: the models and the lines of a stack-up of `eh3d-phoenix-stackup`, the figures of lines and loads of `eh3d-phoenix-rf`, the netlist parser, the operating point, the AC and the transient analysis of `eh3d-phoenix-sim`, the rule check of `eh3d-phoenix-rules`, the length measure of `eh3d-phoenix-lengths` and the zone fill of `eh3d-phoenix-zones` on boards built by the check.

## The command

```
phoenix oracle run  [--dir FOLDER] [--suite NAME] [--out FILE] [--json] [--lang CODE]
phoenix oracle list [--dir FOLDER] [--suite NAME]
```

`run` gives each case a verdict: ok, outside the tolerance, failed (the calculation under test returned an error, or the case is not valid) or pending. The report is the same every time, in text (eight languages) and in JSON (`phoenix-oracle-report/1`). The exit code is 0 when every case that can run is inside its tolerance (pending cases are listed and do not count), 1 when a case is outside its tolerance or failed, 2 when the arguments or the input are unusable. The command starts no other program and writes nothing unless `--out` names a new file; an existing file is never replaced. Without `--dir` it runs the cases that are built into the program; with `--dir` it reads the `.json` files of a folder, which is how new cases are tried before they are added.

The tests of the crate require every cited case to pass, every pending case to carry a reason and no number, every case to name its source, and the files of the folder to be the ones built into the program. The diagnostic codes `PHOENIX-ORACLE-001` to `010` are in `ERROR_CODES.md`.

## Adding a case

1. Find the value in a source that can be cited with its place (book and chapter, standard and clause, table of constants). If it cannot be cited with certainty, write the case as pending with the reason.
2. Choose the tolerance before running the check, write who chose it and why, and write the unit.
3. Run `phoenix oracle run --dir <folder>`. A case that fails is not adjusted until it passes: the cause is found first (the formula, the input, the tolerance or the code) and written in the commit.
4. Add the file to `tests/oracle/` and to the list in `crates/eh3d-phoenix-oracle/src/bundled.rs`.

## What is pending

The cases in `pending.json` are the open list: the current capacity of tracks (IPC-2221 and IPC-2152), the resistance of a track, the spacing by voltage of IPC-2221, the tables of microstrip impedance of the handbooks, a coplanar worked example, the loss against published curves, the ladder filter tables, the clearance at exactly the limit, the zone fill with round obstacles, the length of arcs, the comparison with a field solver and with a SPICE program, and the scattering parameters against a data sheet. Several of them need a calculation that the suite does not have yet (`NOT_YET.md`), several need a book or a standard that is to be consulted, and several need another tool.

## Plan: comparison with EAGLE, KiCad and other tools (not run)

Nothing in this section has been run. It says what will be exported, what will be compared and with which tolerance, so that the tolerance is fixed before the results are seen.

**Rules of the comparison**

* The reference tool and its version are written in the `source` of every case made from it. A claim of correctness names the corpus, the tolerance and the tool version it holds for (`REQUIREMENTS.md`).
* The boards are generated by the tests of this repository or are public designs whose licence allows it. No private design file is stored here.
* Every difference is classified (a bug here, a convention of the other tool, a limit of the model, an error in the case) and fixed or documented. A tolerance is changed only with the reason written.
* The comparison is made on a computer that has the other tool installed; this repository never starts it.

| Area | What is exported or generated | What the other tool gives | What is compared | Proposed tolerance |
|---|---|---|---|---|
| Rule check | sweeps of generated boards: clearance, track width, drill, annular ring, edge distance, written as EAGLE XML by `eh3d-phoenix-eagle-board` | the DRC report of EAGLE for the same board and rules | the number of findings per rule, the measured value and the limit of each | counts equal; measured value within the digits the tool prints (about 1 micrometre) |
| Rule check at the limit | the same sweep with the distance exactly at the limit | the verdict of the tool | whether a distance equal to the limit passes | exact match; the convention is written once found |
| Connectivity | a public KiCad project read with `phoenix kicad-read`, and the generated EAGLE boards | the net list exported by the tool | the nets as sets of reference and pin | identical sets; the rate of identical projects is reported |
| Zone fill | generated boards with a zone, a track, a via and a pad inside | the filled area from the Gerber plot of the filled zone in KiCad and in EAGLE | the copper area, the number of islands, the position of the cut edges | area within 0.5 % for shapes with straight edges, within 2 % with round obstacles, to be settled by the first measurement; island count equal |
| Lengths | generated boards with straight tracks and arcs | the length of the net in the tool that reports it | the length per net | straight pieces within 1 micrometre; arcs within 0.1 % |
| Impedance of a stack-up | microstrip, stripline, coplanar and coupled lines of the stack-ups of the tests | the transmission line calculator of KiCad (same family of formulas) and a two-dimensional field solver | impedance, effective permittivity, delay, loss | calculator of KiCad within 0.5 %; field solver within 3 % single-ended and 5 % differential, to be settled by the first measurement |
| Simulator | the netlists of the tests, written with `phoenix sim` | the operating point, AC and transient results of a SPICE program (ngspice) | node voltages, gain and phase, the waveform at sample points | linear circuits within 1e-9 relative; transient within 1e-3 of the final value; circuits with devices within 1 %, with the model parameters written equal on both sides |
| Gerber and Excellon export | the files written for a generated board | two independent viewers, and the import of `phoenix gerber-import` | apertures, coordinates, bounding box, drill count | coordinates exact in the format of the file; no warning in the viewers |
| EAGLE round trip | the XML written for a generated board | EAGLE opens it | warnings of the import, the signature of the board after saving | no warning; signature equal |
| KiCad write | a board written for KiCad (not written yet, PPCB-013) | KiCad opens it and runs its checks | as for the EAGLE round trip | waits for the writer |
| Touchstone and RF figures | the files and figures of the tests | a network analyser or a published file | S parameters, VSWR, group delay | to be settled with the measurement |

**Steps for each row**

1. Write the cases as pending, with the tool and its version, the tolerance and the files to be made.
2. Generate the files with a command of this repository and store them in `tests/oracle/` (small ones only).
3. Run the other tool by hand, write its result in the case with the tool and its version in the `source`, and change the case from pending to cited.
4. Run `phoenix oracle run`; classify every case that is outside its tolerance.
5. Only then change the state of the capability in `REQUIREMENTS.md`, naming the corpus, the tolerance and the tool version.

**State of this plan:** not run. No file has been exported to EAGLE or KiCad for this purpose and no result of another tool is in the cases.
