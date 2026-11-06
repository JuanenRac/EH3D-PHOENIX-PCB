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
* With nothing open, a start screen: a box for the path of a file, the button *Open*, the button *New library*, the list of recent files and the note that a file can be dropped on the window. A file chosen there opens in the mode its extension says: a board (`.brd`, or the JSON of the model) or a schematic (`.sch`) in the viewer, where the editing of boards and the editing of schematics are reached as before, and a library (`.lbr`) in the library mode.

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

### Editing a schematic

The button *Edit schematic* of the left panel starts the schematic editor (`docs/SCHEMATIC_EDIT.md`): the tools to draw wires, put net labels, power ports and bus labels, place symbols from a palette, select by rectangle, drag with the wires that follow the pins, copy and paste, turn and mirror, set values and references, with undo and redo and the electrical check after every change. After every change the schematic is handed to the workspace, so the findings, the transfer to the board and saving work on what was edited.

### Editing a board

The button *Edit board* of the left panel starts the board editor (`docs/BOARD_EDIT.md`): the tools to place a footprint, draw tracks with horizontal, vertical and 45-degree pieces, add vias, delete, tune the length of a net with meanders and select by rectangle, the drag of components with the tracks on their pads following, turning, putting a component on the other side and locking it, the active layer, the lit net with its length and the ratsnest, with undo and redo and the design rules checked after every operation (an operation that adds a rule error is refused). After every change the board is handed to the workspace, so the findings and saving work on what was edited. The state is tested without a screen (`eh3d-phoenix-boardedit`); the window (`board_gui`) has never been seen on a screen or used with a mouse.

The box *Push tool* of the same panel turns a click into a point of a new track of the chosen net, shows the tracks that the push would move before it is accepted and routes the track with the push as one step (`docs/SHOVE.md`); the window has never been seen on a screen or used with a mouse.

The box *Pair tool* of the same panel turns a click into a point of the centre line of a differential pair and adds the buttons to draw the pair, make its lengths equal, correct its gap and check the pairs (`docs/DIFFPAIRS.md`); it has never been used with a mouse.

### Editing a library

A `.lbr` file given to the window, or the button *Library editor*, starts the library mode (`docs/LIBRARY_EDIT.md`): lists of footprints, symbols and device sets, a drawing board with a fine grid and snapping, tools to place pads, holes, lines, arcs, circles, texts and pins, a measure, the properties of the item selected, the openings of the mask and the paste, the gates, variants and connections of a device set, the findings of the check of the library, the footprint generator and the plan to replace an item by its master. Every edit is a reversible operation of `eh3d-phoenix-libedit`, and the library is saved only as a new file that is read back and compared. The state is tested without a screen (`library_session`); the window (`library_gui`) has never been seen on a screen.

## The start screen

The program `phoenix-gui` opens the window with no arguments, and the release archive holds it beside `phoenix`. It is built with the `gui` feature (`cargo build --release -p eh3d-phoenix-cli --features gui` makes both programs) and accepts `phoenix-gui [--settings FILE] [--lang xx] [<file>]`; with a file it opens it at once, without one it shows the start screen. On Windows it is a program of the graphical subsystem, so no console window opens behind it and it prints nothing; an error that keeps the window from opening is written to `last_error.txt` in the settings folder of the user. If no `--settings` is given it reads `config/viewer.cfg` beside the program when that file is there, and the language comes from `--lang` or from the environment (`LC_ALL`, `LC_MESSAGES` or `LANG`), with English when none names one of the eight.

The recent files are a plain text file, one path per line, the newest first, in the settings folder of the user: `%APPDATA%\EH3D-PHOENIX-PCB` on Windows, `$XDG_CONFIG_HOME/eh3d-phoenix-pcb` or `~/.config/eh3d-phoenix-pcb` on the other systems. A file is added when it has been opened (by the start screen, the path box or a drop), a path that repeats moves to the front, and a file that is no longer there is shown as not found and cannot be opened from the list; each entry can be forgotten, or the whole list cleared. The name of the file (`recent_file`) and the number of entries (`recent_max`) are settings of `config/viewer.cfg`. When the list cannot be written the window says so and goes on.

What the start screen decides (a typed path with or without the quotes that Windows adds when it copies a path, which of several dropped files is opened, the list, its limit and its file) is in the modules `launcher_session`, `launcher_recent` and `launcher_text` of the crate `eh3d-phoenix-viewer`, with tests and without a screen; `launcher_gui` only draws it. The arguments of `phoenix-gui` are read by `window_args` in the crate `eh3d-phoenix-cli`. Like the rest of the window, the start screen is a prototype that has not been seen on a screen or tried with a mouse.

## Imported Gerber and Excellon files

A folder of Gerber and Excellon files is shown read-only (`docs/GERBER_IMPORT.md` describes how the files are read). On the start screen type the folder, or drop it, and press *Gerber folder*; a typed or dropped path that is a folder is taken for one. Files that cannot be Gerber or Excellon (documents, pictures, archives) are left aside and counted; a file that cannot be read stays in the list with its code and is never drawn; nothing is written and a folder is not added to the recent files.

* One layer for each file, named by its role (copper top, bottom or inner, solder mask, paste, silkscreen, outline, plated and not plated drills, or "layer not known" when neither the attributes nor the name tell), coloured by role from `config/viewer.cfg` and drawn in the order of the stack of a board: bottom copper, inner layers, top copper, mask, paste, silkscreen, holes, outline.
* Every object is a filled polygon in millimetres: an aperture put at a point (circle, rectangle, oblong, regular polygon, or a macro whose primitives keep their exposure), a line as a capsule, an arc as a curved strip with round ends (the angle of a piece is `gerber_arc_step_deg`), a region as its polygon, the hole of an aperture as a clear circle, a round hole and a slot of a drill file. A layer says how many objects are dark and how many clear, and when the import skipped something that changes the picture (`incomplete`) or the set was cut at `gerber_max_polys`.
* Polarity: the objects are kept in the order of the file; a dark one covers, a clear one takes away. In the drawing a clear object is painted in the colour of the background, so it also hides what lies under it in other layers that are drawn before it.
* The list of layers has a box to show or hide each one, a colour swatch, *Up* and *Down* to change the order, and a slider of the opacity (the colour of a layer keeps its own alpha, so the mask stays see-through). The opacity of overlapping shapes of one layer adds up where they overlap.
* The measure takes two clicks; a click within `gerber_snap_px` of the centre of a flash or a hole, of an end of a line, an arc or a slot, or of a corner of a region goes to that point. The distance, its parts and its angle are shown in millimetres.
* *Compare with the board of the project*: type a board file (`.brd` or the JSON of the model) and press *Compare*. The board is exported with the export of this suite (the generic profile) and read back by the same import, so both sides go through the same code. Each layer of the files is compared with the layer of the board that has the same role, cell by cell (`gerber_compare_cell_mm`, larger when the area would need more than `gerber_compare_max_cells`), after the board is moved so that the least corners of the two outlines meet; the shift is shown. The cells covered only by the files and only by the board are marked in two colours with their areas, a layer that agrees is *identical*, and a file whose role no layer of the board has is listed.

## The 3D window

The button *3D view* of the top bar of an open board builds the assembly of the board with the table of models typed beside it (the CSV of `docs/ASSEMBLY3D.md`; with none, only the board is drawn and the findings say that no component has a model). The scene is the one of `eh3d-phoenix-assembly3d`: the board as a prism of the thickness of its stack-up (or the default thickness) and each component as a box; the checks (collisions, height limit, outside the outline) use the default limits of the crate.

* Camera: drag with the primary button to turn around the board, drag with shift, the right or the middle button to move it, the wheel to zoom, *Reset the view* to return to the first view (the angles, the field of view, the speeds and the closest distance are in `config/viewer.cfg`). The projection is worked out by hand: a point is taken to the axes of the camera and divided by its depth, with the distance of the picture plane from the field of view; a point behind the camera has no pixel and a face with such a corner is not drawn.
* Drawing: only the painter of egui, with no other dependency. The faces that look away from the camera are left out, and the others are sorted from far to near by their middle. A component that hangs under the board is drawn before the board when the camera is above it, and the components on top after it; from below it is the other way round. The light is the angle between a face and the camera, so a face that looks straight at the camera is the lightest.
* Colours: the board, the components on top and the ones on the bottom have their own colour and each component is a little lighter or darker by its reference; a component with a collision is marked in one colour, one that is too tall in another and one outside the outline in a third, in that order of priority, and the selected one in a fourth, with its outline drawn over the rest.
* A click selects the face drawn last under the pointer: a component (its reference, footprint, side, height, how far it reaches out of the board, the heights of its box, the references it collides with and the marks) or the board (its thickness). The side panel counts the components, the collisions, the ones that are too tall and the ones outside the outline, names the component that stands highest and lists the findings of the scene.

## Saving

A document is never saved over the file it was read from. The name to save to is proposed beside the input (`<name>_edited.json`); a name that is the input, even spelt another way, is refused (`PHOENIX-VIEW-011`), and so is a file that exists and that this window did not write. When the window replaces a file it saved before, a copy of the previous one is kept next to it with the ending of `backup_suffix`. A board is written as the JSON of the model, or as EAGLE XML when the name ends in `.brd` (written by the existing writer, read back by the tests of this repository, never opened in EAGLE); a schematic is written as the JSON of the model. The board that a schematic is compared with is saved with its own name box.

## Commands

```text
phoenix view [--settings FILE] [--lang xx] [<board.brd | schematic.sch | board.json>]
phoenix view --summary [--settings FILE] [--lang xx] <file>
phoenix view --bench [--settings FILE] [--lang xx]
```

* `view` opens the window. The window is behind the `gui` feature, which is off by default: `cargo run --release -p eh3d-phoenix-cli --features gui -- view FILE` (or the program `phoenix-gui`, below, which opens it with a start screen). A build without it explains this (code `PHOENIX-VIEW-008`, exit code 2). A file can also be typed in the path box or dropped on the window.
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
| `board_text`, `board_sync` | The texts of the board editor in eight languages and the hand-over of the edited board to the workspace (`Workspace::replace_board`) |
| `board_pair`, `board_pair_text` | The state of the pair tool of the board editor (the pair chosen, the points of the centre line, the operations on the pair and the report of the checks) and its texts in eight languages |
| `board_pair_gui` (feature `gui`) | The section of the pairs in the panel of the board editor and the drawing of the points that were clicked |
| `board_gui` (feature `gui`) | The editing mode of the window for a board: the panel of tools and the canvas that draws the board with the ratsnest and what is in progress and passes on the pointer and the keys to `eh3d-phoenix-boardedit` |
| `schematic_text`, `schematic_sync` | The texts of the schematic editor in eight languages and the hand-over of the edited schematic to the workspace (`Workspace::replace_schematic`) |
| `schematic_gui` (feature `gui`) | The editing mode of the window for a schematic: the panel of tools, the palette, and the canvas that draws the schematic and passes on the pointer and the keys to `eh3d-phoenix-schedit` |
| `library_session`, `library_text` | The state of the library mode (the library with its history, the footprint or symbol on the board, the tool, the grid and the snap, the selection and the drag, the measure, the checks, the plan to replace by a master, the saving as a new file) and its texts in eight languages |
| `library_gui` (feature `gui`) | The library mode of the window: the lists, the drawing board, the properties, the device sets, the findings, the generator and the plan |
| `launcher_session`, `launcher_recent`, `launcher_text` | The state of the start screen (what a typed path, a dropped file or a recent file asks for), the list of recent files and the file that keeps it, and its texts in eight languages |
| `launcher_gui` (feature `gui`) | The start screen of the window: the path box, the buttons and the list of recent files |
| `gerber_scene`, `gerber_text` | The files of a folder as layers of filled polygons (colour by role, order of the stack, polarity, visibility, order, opacity), the snap of the measure, the comparison of a layer with the layer of a board on cells, and the texts of the window of the imported files in eight languages |
| `gerber_gui` (feature `gui`) | The window of the imported files: the list of layers, the measure, the comparison and the canvas |
| `view3d`, `view3d_text` | The orbiting camera and the projection worked out by hand, the faces of the board and of the components, the order of drawing, the selection under a click, the state of each component, the colours, the assembly built from a board and a table, and the texts of the 3D window in eight languages |
| `view3d_gui` (feature `gui`) | The 3D window: the canvas that draws the faces with the painter, the side panel and the pointer |
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

`config/viewer.cfg` holds every limit, tolerance, size and colour: the largest file and the most primitives, the limits and the speed of the zoom, the tolerance of a click in pixels, the error allowed when an arc is drawn as lines, the widths and heights of outlines, wires and texts, the parameters of the spatial index, the sizes of the benchmark boards, for the editing the grid that positions snap to, the step of a rotation, how far from a vertex a press takes it, the step of the width keys, the width of the ratsnest lines, the radius of the ring of a finding, the most findings a panel lists, the ending of the copy kept when a saved file is replaced, the number of recent files and the name of their file, the sides of a circle, the angle of a piece of an arc, the most polygons, the cell and the snap of the imported Gerber layers, the field of view, the start angles, the speeds, the closest distance, the factor of the heights and the colours of the 3D window, and the colour of every layer and mark (`#rrggbb` or `#rrggbbaa`). The behaviour of the router and of strict edits is in `config/route.cfg`, which the window reads as built in (the same values as the file). A missing key keeps its default and an unknown key is an error (`PHOENIX-VIEW-005`).

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
* Editing a board in the older tools has no copy, no placing of new components, no properties and no selection of several items; moving a component there does not move its tracks; a vertex is moved one at a time; there is no push and shove, no arcs in the tracks the router makes and no routing around zones (`docs/ROUTE.md`). The board editor mode (`docs/BOARD_EDIT.md`) places components, selects several items and drags components with their tracks, but it has never been used with a mouse either, and it has no copy, no properties of an item, no push and shove and no router.
* The library mode is a mode of the window too (`docs/LIBRARY_EDIT.md`) with the same limits: it has never been used with a mouse, and it opens libraries only as a whole, with no copy of an item, no selection of several items and no drawing of the polygons that the model keeps as XML.
* The schematic editor is a mode of the window (`docs/SCHEMATIC_EDIT.md`) and has the same limits: it has never been used with a mouse, a part is drawn as the box around its pins and the window keeps the steps of the editor apart from the annotation steps of the top bar.
* A drawn grid and a view of the board from the bottom are not done; the grid only snaps the positions of edits. There is no drawing of the symbols of a schematic beyond the box around the pins, and no text from the stroke font of the board (the window uses the font of egui).
* The drawing on a screen and the time of an edit on a large board were not measured.
* The window of the imported files and the 3D window are written like the rest of the window: their scenes, cameras, selections and comparisons are tested without a screen, and neither has been seen on one or used with a mouse. In the imported files a clear object erases what lies under it in the drawing, also in other layers, and the shapes of one layer that overlap add up when the opacity is below 1; the comparison is made on cells and not on shapes; step and repeat, aperture blocks and the thermal and moire primitives are not drawn because the import skips them; the imported layers are not drawn over the board in one view.
* In the 3D window the faces are sorted by their middle, so two faces that cross each other can be drawn in the wrong order; the camera must stay away from what it looks at (a face with a corner behind it is not drawn); a component is its box; the checks use the default limits of the crate and not `config/assembly3d.cfg`; the thickness is the default one, as the stack-up is not read.
* Saving an `.brd` goes through the EAGLE writer, which leaves out what the model does not hold; the file has never been opened in EAGLE.
