# EH3D-PHOENIX-PCB: the heating of tracks and vias

The module `heating` of the crate `eh3d-phoenix-stackup` and the command `phoenix thermal` estimate how much a track or a via warms up when the net it belongs to carries a current, and warn when the rise is above a limit.

**State: prototype.** Two estimates are set side by side, both quasi-static and neither compared with a measurement of a board. The constants of the first are the ones usually printed with its chart, and the standard itself has not been at hand to check them. The convection coefficient of the second is a value of the settings, not a property of the board.

## The two estimates

1. **The fit of IPC-2221** to its chart of current against cross-section and temperature rise: `I = k dT^0.44 A^0.725`, with `I` in ampere, `dT` in kelvin, `A` the cross-section of the track in square mil, and `k` 0.048 for a track on an outer layer and 0.024 for one on an inner layer. The report gives the rise for the current of the net and the current that the fit allows at the limit. The range of the chart as it is usually quoted (a rise of 10 to 100 K, up to 35 A) is in the settings; a result outside it is marked (`PHOENIX-HEAT-006`). IPC-2152, which replaces the chart with measured charts and modifiers for the plane and the thickness of the board, is **not applied** and every report says so (`PHOENIX-HEAT-012`).
2. **The convection model of EH3D-THERMAL-LAB** for a track: the Joule heating `P = I^2 rho (1 + a (T - 20)) L / A` cooled by convection from the top face and the two edges, `G = h (w + 2 t) L`. The temperature is the fixed point of `T = T_amb + P(T) / G`; because the length cancels it is solved in closed form, `dT = K (1 + a (T_amb - 20)) / (1 - a K)` with `K = I^2 rho / (A h (w + 2 t))`, which is the value that repository finds by iteration. It ignores the conduction into the board and radiation, so it is high. A track whose heating grows faster than its cooling, or that would pass `max_temperature_c`, has no number from this model (`PHOENIX-HEAT-005`), as in THERMAL-LAB. The resistivity and its coefficient are the defaults of that repository, 1.724e-8 ohm metre and 0.00393 per kelvin.

A **via** is a plated barrel: the annulus of copper of the thickness of the plating (`via_plating_um`) around the drill is its cross-section, it is taken as an inner conductor, and only the first estimate applies to it.

## The command

```
phoenix thermal --currents FILE [--stackup FILE] [--net NAME] [--settings FILE]
                [--out FILE] [--json] [--lang CODE] <board>
```

The file of currents has one line `net = amps` for each net (`#` starts a comment); only those nets are looked at. For each one the tracks are grouped by layer and width, with the copper thickness and the placement (outer or inner layer) taken from the stack-up of `--stackup FILE` (the job file of `phoenix stackup`); a layer that is not in it takes `default_copper_um` and `default_placement` from the settings and the report says so (`PHOENIX-HEAT-008`). The vias of the net are judged at their smallest drill. The **whole current of the net** is taken through every track and every via, so a net that splits its current among parallel paths is judged on the safe side.

The limit `max_rise_k` is judged by the estimate named in `limit_basis` (`ipc2221`, `convection` or `larger`); a track or a via above it is the warning `PHOENIX-HEAT-004`, and a net of the file that the board does not have is `PHOENIX-HEAT-007`. The command is a dry run: it writes nothing unless `--out` names a file, which must not exist. Exit code 0 when no track or via is above the limit, 1 when one is, 2 when the arguments or the input are unusable.

## Settings

Every constant and limit is in `config/heating.cfg`, which is also the built-in default: the ambient temperature, the resistivity of the copper and its coefficient, the convection coefficient, the hottest temperature of the convection model, the limit and its basis, the constants and exponents of the fit and the range of the chart, the defaults for a layer the stack-up does not give, the plating of a via and the most nets in a file of currents. An unknown key or a value out of range is refused with `PHOENIX-HEAT-001`.

## What the tests check

* The fit and its inverse, with the rises of three tracks worked out by hand, and the reference cases of `phoenix oracle` (`docs/ORACLE.md`) for the convection model against its closed form and for the resistance per millimetre. The cases of the printed chart of IPC-2221 and the charts of IPC-2152 stay pending there until they can be checked against the standard.
* The convection model agrees with the closed form that the tests of THERMAL-LAB use, and the rise grows with the ambient temperature.
* The cases that have no number: a track that would pass the hottest temperature, a heating that runs away, a current of zero (no rise), a width, a thickness or a current that is not valid.
* The file of currents and the options, and the placement of the layers of a stack-up.

## Limits

* The temperature of a track is taken as uniform, with no conduction along the track or into the planes, no neighbouring tracks that heat each other and no radiation.
* The copper area of a zone, of a plane under a track and the thermal vias are not part of the model, and the chart of IPC-2221 is for tracks in a board of a standard thickness.
* The current of a net is not split among the tracks and vias of the net.
* The resistivity is the one of annealed copper at 20 degrees with a linear coefficient; the skin effect is not part of the model, so alternating currents of high frequency are not covered.
* The constants of the fit, its range and the convection coefficient have not been checked against a standard or a measurement.
