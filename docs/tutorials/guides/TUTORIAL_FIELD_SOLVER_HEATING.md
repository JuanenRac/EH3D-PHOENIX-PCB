# Solve the impedance of a line with the field solver and estimate how much a track heats up

`examples/guides/stackup_job.json` describes a two-layer stack-up with one line. You solve the cross section with the 2D field solver and compare the result with the closed formulas, then take the line from a board, and finally estimate the temperature rise of the track of a net for two currents. Both tools are quasi-static prototypes that have not been compared with another solver or a measurement.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. Solve the line of the job. The report gives the mesh, the iterations and the residual, then the impedance, effective permittivity, inductance, capacitance, delay and dielectric loss, and the closed formula beside it with the difference in percent. It is a dry run: nothing is written.

```console
$ phoenix field2d examples/guides/stackup_job.json
```

2. Refine the mesh by 2 to see how much the answer moves; a result that changes little is converged. A solution that does not reach the tolerance gives `PHOENIX-FIELD-004` and no number.

```console
$ phoenix field2d examples/guides/stackup_job.json --refine 2
```

3. Take the line from a board. The layer and the width most of the copper of the net `MID` runs on (a track of 0.2 mm on `Top`) give a microstrip, which is solved on the stack-up of the job.

```console
$ phoenix field2d examples/tutorials/tut.brd --stackup examples/guides/stackup_job.json --net MID
```

4. Add the field solver to the report of `stackup`, so that the closed formula and the field solution appear in one report.

```console
$ phoenix stackup examples/guides/stackup_job.json --freq-ghz 1 --field2d
```

5. Estimate the heating of the net `MID` with 0.5 A. The report lists the track by layer, width and copper thickness, and gives the rise by the fit of IPC-2221 (about 4 K for a track of 0.2 mm by 35 um). The convection model gives no number, because with the cooling of the settings a track so thin would pass 125 C (`PHOENIX-HEAT-005`, information). The rise is below the limit of the settings and the exit code is 0.

```console
$ phoenix thermal --currents examples/guides/currents_low.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

6. The same net with 3 A. The rise is above the limit, the track is marked `[above the limit]` with the warning `PHOENIX-HEAT-004` and the exit code is 1.

```console
$ phoenix thermal --currents examples/guides/currents_high.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

## How to read the result

* The solver takes the line as quasi-TEM on a rectangular mesh in a box whose walls are at zero potential, which makes the impedance a little lower than for an open region. The conductors are rectangles and the solder mask and the copper loss are left out. A difference from the closed formula above `compare_warn_percent` is the warning `PHOENIX-FIELD-009`.
* The heating estimate takes the whole current of the net through every track and via, so a net that splits its current is judged on the safe side. IPC-2152 is not applied and every report says so (`PHOENIX-HEAT-012`). A layer that the stack-up does not name takes the defaults of the settings (`PHOENIX-HEAT-008`).
* Both commands are dry runs and write a JSON report only to a new file named with `--out`. The constants and limits live in `config/field2d.cfg` and `config/heating.cfg`, not in the code. The exit code is 0 when all is well, 1 when a line could not be solved or a track is above the limit, and 2 for unusable arguments or input.

## See also

* [docs/FIELD2D.md](../../../docs/FIELD2D.md)
* [docs/THERMAL.md](../../../docs/THERMAL.md)
* [docs/STACKUP.md](../../../docs/STACKUP.md)
