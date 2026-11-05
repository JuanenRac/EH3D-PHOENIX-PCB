# EH3D-PHOENIX-PCB: the viewer

State: **prototype, read only**. The logic is tested without a screen; the window has never been seen on a screen. It exists to decide the interface stack with data (phase F6 of the roadmap, requirement PPCB-014, record `docs/adr/0009-first-interface-stack.md`). It edits nothing.

## What it does

* Opens a board (an EAGLE `.brd`, read by the existing reader, or the JSON of the model, `design/board.json`) or a schematic (an EAGLE `.sch` or the JSON of the model, `design/schematic.json`).
* Draws a board by layers: copper (top, bottom and inner layers, in the order of the stack), solder mask, paste, silkscreen, the outline, the drill holes and the reference designators. Pads are drawn on every layer they list, tracks with their width (lines and arcs), vias on every copper layer they span, zones as filled polygons.
* Draws a schematic: the frame and the name of each sheet (sheets are laid side by side), each placed part, its pins with their numbers, the wires, and the name of every net next to its first pin that has a place. The model holds no drawing of a symbol, only its pins, so a part is drawn as the box around its pins.
* Colours: one colour per layer, taken from `config/viewer.cfg` and changed in the window with a colour button next to each layer. Each layer can be shown or hidden, and all of them at once.
* Zoom with the wheel about the pointer, move by dragging, fit the drawing to the window with the button or a double click.
* Click an item (the select tool): the item is selected, its net is highlighted on every layer and on the schematic, and the status bar names the item and the net. A click on nothing clears both. When two things lie under the click, a thing that is not a text wins over a text, then the layer drawn last, then the nearest, then the smallest.
* Measure: with the measure tool the first click sets a point, the second the other one, and the third starts again; the distance, its parts and its angle are shown in millimetres.
* The position of the pointer is shown in millimetres.
* Eight languages (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese), chosen with `--lang` or in the window.

## Commands

```text
phoenix view [--settings FILE] [--lang xx] [<board.brd | schematic.sch | board.json>]
phoenix view --summary [--settings FILE] [--lang xx] <file>
phoenix view --bench [--settings FILE] [--lang xx]
```

* `view` opens the window. The window is behind the `gui` feature, which is off by default: `cargo run --release -p eh3d-phoenix-cli --features gui -- view FILE`. A build without it explains this (code `PHOENIX-VIEW-008`, exit code 2). A file can also be typed in the path box or dropped on the window.
* `--summary` prints what the file holds without a window: the primitives, the items and the nets, the primitives of each layer and the size in millimetres.
* `--bench` runs the benchmark below.

Exit codes: 0 done, 2 the arguments, the settings or the input are unusable.

## How it is built

The crate `eh3d-phoenix-viewer` holds all of the logic, with no dependency on egui:

| Module | What it holds |
|---|---|
| `prim` | The drawing primitives in millimetres: line, arc (centre, radius, start and sweep), polygon, circle and text, with their box, their distance from a point and the cutting of an arc into short lines within a tolerance, and the triangles of a polygon (ear clipping) |
| `scene` | A board or a schematic turned into primitives on layers, each primitive linked to its item of the document and to its net |
| `view` | The pan, the zoom about a point, the limits of the zoom and the fit to a window |
| `measure` | The distance, its parts and its angle between two clicks |
| `viewer` | The state: the scene, the view, the visibility and the colour of each layer, the tool, the selection by position with a tolerance in pixels, the highlight of a net and the lines of the status bar |
| `grid` | A uniform grid over the boxes of the primitives: the primitives near a point or inside a window without looking at all of them |
| `load` | Reading an EAGLE board or schematic or the JSON of the model, bounded by the settings |
| `bench` | Three synthetic boards and their timing |
| `settings`, `codes`, `msg` | The settings file, the codes `PHOENIX-VIEW-<NNN>` and the texts in eight languages |
| `gui` (feature `gui`) | The window: egui on eframe with the theme Forge, the icon of the program and a canvas painted by `Painter` |

The window only passes on what the person does (a pixel under the pointer, a click, a turn of the wheel) and draws what the state says. Replacing egui means replacing `gui.rs`.

How a board becomes primitives: a track piece is a line or an arc with the width of the track; an arc given as end points and a sweep gets its centre and radius from the chord (an arc whose end points coincide has no known centre and is counted as not drawn); a rectangular pad is a polygon turned by the rotation of the component and of the pad (mirrored when the component is on the bottom); an oblong pad is a thick line with round ends; a round pad is a circle; a hole is a circle on the layer of the drills; text is drawn by egui with its own font at the height of the text, and hidden when it would be smaller than a few pixels.

## The benchmark

`phoenix view --bench` generates three boards of copper tracks (lines and arcs), vias, zones and silkscreen lines, with the same seed on every computer, of 1 000, 20 000 and 200 000 primitives (the sizes are settings), and prints for each:

* the time to turn the board into primitives (`convert_ms`) and to build the spatial index (`index_ms`);
* the average time of a selection by position with the index (`pick_us`) and by looking at every primitive (`scan_us`), and how many scans gave another answer than the index (`differ`, which has to be 0);
* the time to list the primitives that a window shows, for the whole board (`window_ms`) and for a sixteenth of it (`part_ms`).

What it does **not** measure is the drawing on a screen: that needs a window and a graphics card and was not done. The numbers are those of the computer that runs the command and of the preparation of the data. They say whether the data side can keep a window fluid; they do not say that the window is fluid. The record `0009` lists what has to be measured on a real screen.

## Settings

`config/viewer.cfg` holds every limit, tolerance, size and colour: the largest file and the most primitives, the limits and the speed of the zoom, the tolerance of a click in pixels, the error allowed when an arc is drawn as lines, the widths and heights of outlines, wires and texts, the parameters of the spatial index, the sizes of the benchmark boards and the colour of every layer and mark (`#rrggbb` or `#rrggbbaa`). A missing key keeps its default and an unknown key is an error (`PHOENIX-VIEW-005`).

## Codes

| Code | Meaning |
|---|---|
| PHOENIX-VIEW-001 | The file cannot be read or is larger than the settings allow |
| PHOENIX-VIEW-002 | The file is neither a board nor a schematic that the viewer reads |
| PHOENIX-VIEW-003 | The EAGLE file is refused by the reader |
| PHOENIX-VIEW-004 | The document has nothing to draw |
| PHOENIX-VIEW-005 | The settings are not valid |
| PHOENIX-VIEW-006 | The scene has more primitives than the settings allow |
| PHOENIX-VIEW-007 | A benchmark option is not valid |
| PHOENIX-VIEW-008 | The window was asked for in a build without the `gui` feature |
| PHOENIX-VIEW-009 | The window could not be opened |

The messages are written in the eight languages of the suite; the registry is in `docs/ERROR_CODES.md`.

## What is not done

No editing of any kind, no grid and no rule list (PPCB-014 asks for them), no view of a board from the bottom, no ratsnest, no drawing of the symbols of a schematic beyond the box around the pins, no text from the stroke font of the board (the window uses the font of egui), and no measurement of the drawing on a screen. The window is untested on any screen, any graphics card and any operating system; only its code is checked to build and pass the lints in CI.
