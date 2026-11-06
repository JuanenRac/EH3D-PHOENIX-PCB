# Panelize a board and write the panel files

You lay the board of 30 mm by 20 mm out twice side by side, joined by a V-score line, and write the Gerber and drill files of the whole panel. The settings of the panel are in `examples/tutorials/panel.cfg` (columns, rows, kind of joint, gap and frame); the full list of settings is `config/panel.cfg`.

Run the commands from the root of the repository; see [how to run the tutorials](README.md).

1. Plan the panel. A dry run prints its size, the joint, the holes and the files that would be written, and writes nothing.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ joint: V-score, 1 lines
~ dry run: nothing written
```

2. Write the layers of every copy, the drill files, the outline with the V-score line, the placement and the bill of materials into a new folder.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ written
```

3. A folder that exists is never written into: the second run is refused with the code `PHOENIX-PANEL-008` and exit code 2.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
# exit 2
~~ PHOENIX-PANEL-008
```

4. Check the structure of a Gerber file of the panel, and draw the panel as a DXF drawing for the mechanical side.

```console
$ phoenix gerber-check build/panel/panel_01_top.gbr
$ phoenix dxf-export panel --out build/panel.dxf --panel-config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
```

## See also

* [docs/PANEL.md](../PANEL.md)
* [docs/GERBER_IMPORT.md](../GERBER_IMPORT.md)
* [docs/DXF.md](../DXF.md)
* [docs/FABPACK.md](../FABPACK.md)
