# Round the corners of a copper pour and write thermal reliefs and island limits in the Gerber files

The board `examples/guides/pour.brd` has a ground pour on the layer `Top` around two resistors and a track. You fill it with sharp corners and with rounded ones and compare, then write the Gerber files with a profile that gives a thermal relief to the rectangular pads of the pour and leaves out its small islands. The rule file `rules_relaxed.json` is given because an EAGLE board without design rules has no limits of its own.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. Fill the zone with the settings of the repository, which leave every corner sharp (`corner_radius_nm = 0`). The report gives the polygons and the square nanometres of copper.

```console
$ phoenix zones fill --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

2. Fill it again with `zones_round.cfg`, which sets `corner_radius_nm = 1000000` (an arc of 1 mm). The corners of the pour that face outwards and the corners of the gaps of the thermal relief are rounded; compare the copper area with the first run.

```console
$ phoenix zones fill --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

3. Measure the rounded fill against the board and the rules. Rounding takes copper away from the pour or adds it only inside a gap of a thermal relief, so the fill does not come closer to copper of another net and the check finds no clearance error.

```console
$ phoenix zones check --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

4. Write the Gerber files with the generic profile first. The pad `R2.2` of the ground net is inside the pour and stays joined to it with no relief.

```console
$ phoenix export --out build/pour_plain examples/guides/pour.brd
```

5. Now with the profile `pour` of `export_pour.json`: four spokes (`gerber.zone_thermal_spokes`), a gap and a spoke of 0.25 mm, the same relief for rectangular and oblong pads (`gerber.zone_thermal_non_round`) and no island below 1 square millimetre (`gerber.zone_island_min_area_nm2`). Compare the copper file of the layer `Top` in the two folders: the second has the gaps of the relief written as regions of clear polarity around the pad.

```console
$ phoenix export --profiles examples/guides/export_pour.json --profile pour --out build/pour_relief examples/guides/pour.brd
```

## How to read the result

* In the settings of the fill, `corner_radius_nm` is the radius of the arc (zero leaves every corner sharp) and `corner_tolerance_nm` how far the chords of the arc may be from it. An arc takes at most half of each side next to the corner, so a radius that does not fit is made smaller. The corners of the holes that other copper leaves in the pour are not rounded.
* In the export, a relief has four spokes or two (`gerber.zone_thermal_spokes` is 0, 2 or 4; 0 leaves the pads joined solid). A pad turned by an angle that is not a right angle stays solid and says so (`PHOENIX-EXPORT-022`). An island smaller than `gerber.zone_island_min_area_nm2` is not written and the report says so (`PHOENIX-EXPORT-021`); zero writes every zone.

## See also

* [docs/ZONES.md](../../../docs/ZONES.md)
* [docs/EXPORT.md](../../../docs/EXPORT.md)
* [docs/BOARD_ZONES.md](../../../docs/BOARD_ZONES.md)
