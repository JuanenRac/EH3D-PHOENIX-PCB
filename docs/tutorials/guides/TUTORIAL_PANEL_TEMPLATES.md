# Keep several panel layouts in one settings file with templates

`examples/guides/panel_templates.cfg` holds a base layout (two boards side by side, joined by a V-score line, no frame) and two named templates. Each template changes only the lines it names, and you choose one with `--template`. The board is the one of the panel tutorial, 30 mm by 20 mm; every command here is a dry run.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. Without a template only the base is read, and the lines under the template sections are ignored: 60 mm by 20 mm, 2 x 1 boards.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg examples/tutorials/kicad/demo.kicad_pcb
```

2. The template `four` changes `columns` to 4 and nothing else: 120 mm by 20 mm, 4 x 1 boards.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four examples/tutorials/kicad/demo.kicad_pcb
```

3. The template `grid` changes `rows` to 2: 60 mm by 40 mm, 2 x 2 boards.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template grid examples/tutorials/kicad/demo.kicad_pcb
```

4. `--columns` and `--rows` still win over a template: with `four` and `--columns 3` the panel is 90 mm by 20 mm, 3 x 1 boards.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --columns 3 examples/tutorials/kicad/demo.kicad_pcb
```

5. A name that the file does not have is refused with the code `PHOENIX-PANEL-001` and exit code 2.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template ring examples/tutorials/kicad/demo.kicad_pcb
```

6. Write the files of the chosen layout into a new folder, as in the panel tutorial.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --out build/panel_four examples/tutorials/kicad/demo.kicad_pcb
```

## How to read the result

* The lines before the first `[template NAME]` line are the base. A name has 1 to 32 letters, digits, `_` or `-`; a section written in any other way is refused with `PHOENIX-PANEL-001` and the number of the line.
* The size line is `panel W mm by H mm, C x R boards`. With the V-score joint and a gap of 0 the boards touch, so the width is the number of columns times 30 mm and the height the number of rows times 20 mm. A template is a way to keep a layout under a name; it is not a setting of any board maker.

## See also

* [docs/PANEL.md](../../../docs/PANEL.md)
* [config/panel.cfg](../../../config/panel.cfg)
