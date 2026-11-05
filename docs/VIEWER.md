# EH3D-PHOENIX-PCB: the viewer

State: **prototype**. The logic, including the whole state of editing, is tested without a screen; the window has never been seen on a screen. It exists to decide the interface stack with data (phase F6 of the roadmap, requirements PPCB-014 and PPCB-015, record `docs/adr/0009-first-interface-stack.md`) and, with the stack still provisional, to try the editing on a board: the window draws what the editing logic of `docs/ROUTE.md` and the synchronisation of `docs/ERC.md` do, it does not decide anything of its own.

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

## Editing a board

The tools of the left panel (and the keys of the table below) change the board through the reversible operations of the board crate and `eh3d-phoenix-route`. Every edit is tried on a copy of the board first: when the board refuses it, or when it adds an error to the check of the rules and `strict_edits` of `config/route.cfg` is on (it is by default), the board stays as it was and the status bar gives the reason with its code. An edit that is made is one step of the history: undo and redo (the buttons of the top bar, Ctrl+Z and Ctrl+Y) bring the board back byte for byte. After each step the drawing is made again from the board and the view, the layers and the selection stay.

| Tool | What it does |
|---|---|
| Move | Press on a component (a pad or its designator), drag and release: the component goes where the pointer went, snapped to the grid of `snap_mm`; a line and a cross show the place while dragging. Its tracks do not follow: the ratsnest shows what is left to route |
| Rotate | A click turns the component by `rotate_step_deg`; the key R turns the selected one, Shift+R backwards |
| Delete track | A click on a track deletes it; the key Delete deletes the selected one |
| Move vertex | Press near a vertex of a track (within `vertex_pick_px`), drag and release. With `keep_angles` of the route settings a piece that ran at 0, 45 or 90 degrees must still do so |
| Width | A click gives the track the active width of the panel; the keys plus and minus change the width of the selected track by `width_step_mm` (never to zero) |
| Via | A click on copper that has a net adds a via of that net at the snapped place, between the first and the last copper layer, sized by the rules and the net class |
| Route | Click the origin (a pad, a via or the end of a track), then the destination: the router joins them in the mode of the panel (go around obstacles, or stop in front of the first one). A route that stops says so |

| Key | What it does |
|---|---|
| Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z | Undo, redo, redo |
| Ctrl+S | Moves the focus to the box of the name to save to |
| V, M, G, O, D, E, W, I, T | Select, measure, move, rotate, delete track, move vertex, width, via, route |
| R, Delete, plus, minus | Turn, delete, widen, narrow the selection |
| F, Escape | Fit the drawing; drop the drag, the first click of a route and the selection |

The keys are not taken while a text box has the keyboard.

## Rules, ratsnest and nets

* **Findings of the rules.** The right panel lists the proven findings of the rule check of the board (severity, code with its title in the language, what it is about) and, above them, what the last edit, undo or redo added (red) and what it resolved (green): the difference is computed for every step, so the effect of an edit is seen at once. The button next to a finding goes to its place and puts a ring there. A rule file (`phoenix-rules/1` or `/2`, with net classes) is loaded from its path and replaces the default rules; the edits already made stay.
* **Ratsnest.** The connections still to be routed are drawn as thin lines (`colour.ratsnest`, `ratsnest_width_px`) and made again after every step; a box of the left panel hides them. The lines of the highlighted net are drawn thicker in the highlight colour.
* **Nets.** The list of nets of the right panel shows how many connections of each are still to be routed; a click on a name highlights the net on every layer.

## Schematics

The select tool, the highlight of a net and the measure work on a schematic as before. The right panel adds:

* **Electrical rule check.** The result of `eh3d-phoenix-erc` with the default matrix: the proven findings, each with a button that goes to the pin, the net or the part it names (the item is selected, its net highlighted and the view centred), and apart what could not be judged. A button runs the check again.
* **Board and schematic.** The path of a board to compare with is opened (an EAGLE board or the JSON of the model). Then *Transfer to the board* and *Back-annotate* make a dry run with `eh3d-phoenix-sync`: nothing is changed, and the package (the changes and what was left out, as the report of `phoenix sync` writes them in the language of the window) is shown. *Apply the package* makes it as one step that undo reverts, to the board for the transfer and to the schematic for the annotation; *Discard* throws it away. A package is dropped when anything changes, so what is applied is always what was seen.

## Saving

A document is never saved over the file it was read from. The name to save to is proposed beside the input (`<name>_edited.json`); a name that is the input, even spelt another way, is refused (`PHOENIX-VIEW-011`), and so is a file that exists and that this window did not write. When the window replaces a file it saved before, a copy of the previous one is kept next to it with the ending of `backup_suffix`. A board is written as the JSON of the model, or as EAGLE XML when the name ends in `.brd` (written by the existing writer, read back by the tests of this repository, never opened in EAGLE); a schematic is written as the JSON of the model. The board that a schematic is compared with is saved with its own name box.

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
| `viewer` | The state: the scene, the view, the visibility and the colour of each layer, the tool, the selection by position with a tolerance in pixels, the highlight of a net, the lines of the status bar, and the replacement of the scene after an edit keeping the view, the layers and the selection |
| `session` | The document that is edited: a board with its rules, its history, the findings of the rules, the difference of every step and the ratsnest; a schematic with its electrical check, the board it is compared with and the package of a dry run; the saving rules |
| `workspace` | The state of editing: the open document, the active tool, the drag, the first click of a route, the keys, undo and redo, the notes of the status bar, the jump to a finding and the list of nets |
| `grid` | A uniform grid over the boxes of the primitives: the primitives near a point or inside a window without looking at all of them |
| `load` | Reading an EAGLE board or schematic or the JSON of the model, bounded by the settings |
| `bench` | Three synthetic boards and their timing |
| `settings`, `codes`, `msg`, `editmsg` | The settings file, the codes `PHOENIX-VIEW-<NNN>` and the texts in eight languages (`editmsg` those of the editing) |
| `gui` (feature `gui`) | The window: egui on eframe with the theme Forge, the icon of the program and a canvas painted by `Painter` |

The window only passes on what the person does (a pixel under the pointer, a press, a drag, a release, a click, a turn of the wheel, a key) and draws what the state says: the primitives, the ratsnest, the lines of what is in progress, the ring of a finding and the panels. Every rule of editing is in `workspace` and `session`, so replacing egui means replacing `gui.rs`.

An edit rebuilds the scene and the spatial index from the board. That is simple and always right, and it costs the time of the preparation of the primitives that `phoenix view --bench` reports (`convert_ms` and `index_ms`); on the large benchmark board it may not be fast enough for every step, which is one more thing to measure on a real screen (`docs/adr/0009-first-interface-stack.md`).

How a board becomes primitives: a track piece is a line or an arc with the width of the track; an arc given as end points and a sweep gets its centre and radius from the chord (an arc whose end points coincide has no known centre and is counted as not drawn); a rectangular pad is a polygon turned by the rotation of the component and of the pad (mirrored when the component is on the bottom); an oblong pad is a thick line with round ends; a round pad is a circle; a hole is a circle on the layer of the drills; text is drawn by egui with its own font at the height of the text, and hidden when it would be smaller than a few pixels.

## The benchmark

`phoenix view --bench` generates three boards of copper tracks (lines and arcs), vias, zones and silkscreen lines, with the same seed on every computer, of 1 000, 20 000 and 200 000 primitives (the sizes are settings), and prints for each:

* the time to turn the board into primitives (`convert_ms`) and to build the spatial index (`index_ms`);
* the average time of a selection by position with the index (`pick_us`) and by looking at every primitive (`scan_us`), and how many scans gave another answer than the index (`differ`, which has to be 0);
* the time to list the primitives that a window shows, for the whole board (`window_ms`) and for a sixteenth of it (`part_ms`).

What it does **not** measure is the drawing on a screen: that needs a window and a graphics card and was not done. The numbers are those of the computer that runs the command and of the preparation of the data. They say whether the data side can keep a window fluid; they do not say that the window is fluid. The record `0009` lists what has to be measured on a real screen.

## Settings

`config/viewer.cfg` holds every limit, tolerance, size and colour: the largest file and the most primitives, the limits and the speed of the zoom, the tolerance of a click in pixels, the error allowed when an arc is drawn as lines, the widths and heights of outlines, wires and texts, the parameters of the spatial index, the sizes of the benchmark boards, for the editing the grid that positions snap to, the step of a rotation, how far from a vertex a press takes it, the step of the width keys, the width of the ratsnest lines, the radius of the ring of a finding, the most findings a panel lists and the ending of the copy kept when a saved file is replaced, and the colour of every layer and mark (`#rrggbb` or `#rrggbbaa`). The behaviour of the router and of strict edits is in `config/route.cfg`, which the window reads as built in (the same values as the file). A missing key keeps its default and an unknown key is an error (`PHOENIX-VIEW-005`).

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
| PHOENIX-VIEW-010 | The edit could not be made |
| PHOENIX-VIEW-011 | The file may not be written there |
| PHOENIX-VIEW-012 | The file could not be written |
| PHOENIX-VIEW-013 | The rule file cannot be used |

The messages are written in the eight languages of the suite; the registry is in `docs/ERROR_CODES.md`.

## What is not done

* The window is untested on any screen, any graphics card and any operating system; only its code is checked to build and pass the lints in CI. The tools, the drags and the panels have been tried through the state that drives them, never with a mouse.
* Editing a board has no copy, no placing of new components, no properties and no selection of several items; moving a component does not move its tracks; a vertex is moved one at a time; there is no push and shove, no arcs in the tracks the router makes and no routing around zones (`docs/ROUTE.md`).
* The edits of a schematic do not exist yet: it is looked at, checked and annotated, not drawn.
* A drawn grid and a view of the board from the bottom are not done; the grid only snaps the positions of edits. There is no drawing of the symbols of a schematic beyond the box around the pins, and no text from the stroke font of the board (the window uses the font of egui).
* The drawing on a screen and the time of an edit on a large board were not measured.
* Saving an `.brd` goes through the EAGLE writer, which leaves out what the model does not hold; the file has never been opened in EAGLE.
