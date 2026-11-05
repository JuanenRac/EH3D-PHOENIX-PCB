# EH3D-PHOENIX-PCB: map of phases

The planned order of work, from the foundations to the all-in-one suite. It is a map, not a promise: nothing below has been started, a phase starts only when the contracts it depends on are accepted, and each phase ends with a gate. A gate never has a fixed pass mark: the measured rate must not drop between runs, every failure is classified (error of this project, limitation of the format, or error of the file), and the measured figure is reported.

## F0. Foundations

Repository with the shared base, a workspace of engine crates (model, EAGLE, KiCad, rules, engine, command line, application), CI, decision records for the interface and the model, a tool that inventories the private corpus, the first hand-made run of the golden-file oracle, and the skeleton of EH3D-VALIDATION-LAB.

Gate: CI green; an inventory report of the real projects; the oracle run by hand once.

Progress: the workspace, the CI and the corpus inventory tool exist (the inventory has been tried only on folders made by its own tests). The model crates, the interface and model decision records, the hand-made oracle run and the VALIDATION-LAB skeleton are not part of this repository's code yet.

## F1. Model and persistence

Stable identities, integer nanometres, the electrical and board model, symbol-footprint-pad links, reversible operations with undo and redo, the project format with a folder per domain, migrations, opaque data and integrity validation, with diagnostics for every reader.

Gate: Round trips over thousands of generated models; corrupt files give diagnostics, not panics; conformance vectors pass.

## F2. EAGLE input and output, format hub, oracle

XML reader (versions 6 to 9) for libraries, boards and schematics; writer and round trip; clean-room reader of the old binary versions (4 and 5); a common importer and exporter interface with quality reports and export profiles; the golden-file oracle; a corpus panel by level and by version in redacted mode; a case reducer.

Gate: The measured rate of each level does not drop and every failure is classified (error of this project, limitation of the format, error of the file).

## F3. Connectivity and rule engine

Connectivity of schematic and board, consistency between them, rule checks in a fixed order, reading of rule files, a report that separates confirmed from inconclusive results, comparison with the reference tool.

Gate: Connectivity and rule agreement rates do not drop; each difference is fixed or documented; deliberate errors are always found.

## F4. Manufacturing export and hand-off

Gerber X2 and Excellon from the shared writers, assembly formats confirmed with real samples, an `.eh3dproj` 1.0.0 package with layers, deterministic exports, and server tasks for import, check and fabrication export.

Gate: Output opens in two independent viewers and matches the model; a package is milled through the CAM chain.

Progress: the export, the import of Gerber and Excellon files for review with its round trip against the export (`docs/GERBER_IMPORT.md`) and the panelization of a board (`docs/PANEL.md`) exist as prototypes tried on files and boards made by the tests only; no independent viewer or manufacturer has opened a file of them.

## F5. KiCad reading and public oracle

Readers for KiCad schematics, boards, symbols and footprints; a public oracle in CI with a pinned tool version; a public corpus of open hardware with reviewed licences.

Gate: The rate of identical connectivity does not drop; each failure is classified.

Progress: the readers exist as a prototype (`eh3d-phoenix-kicad`, `docs/KICAD.md`): symbols, footprints, schematics with sheets, boards and the project file, with the import report and a fidelity level; they are tried on files written by hand only. The public oracle in CI and the public corpus are not started.

## F6. PCB viewer and editor

A short interface prototype and a final stack decision (a viewer and editor prototype exists, `docs/VIEWER.md`, and its stack is only provisional, `docs/adr/0009-first-interface-stack.md`), then viewer, editing, routing (45 degree tracks and arcs, vias, widths per class, live checks, ratsnest, zones) and a remote auto-routing task using an external router. The application registers as a module of the suite launcher and opens project files from the command line.

Gate: A corpus board is modified and saved, opens in the reference tool without warnings and its rule check agrees; fluidity is measured with small, medium and large boards; a re-routed board is milled.

Progress: the logic that needs no window exists as a prototype (`eh3d-phoenix-route`, `docs/ROUTE.md`): the ratsnest, a router for pieces at 0, 45 and 90 degrees with the width of the net class, vias sized by the rules and the two modes (stop in front of an obstacle, go around it), reversible editing operations with undo and redo, and the live check of the rules after every edit. It is tried on boards made by hand and by the tests only. The editing window of the viewer puts them behind tools (move and turn a component, delete a track, move a vertex, change a width, add a via and route by clicking the origin and the destination, with undo and redo, the ratsnest as thin lines, the net list and a panel of the rule findings with the difference of each step); it has never been seen on a screen. Arcs, copper zones, push and shove, copying and placing components and the properties of an item are not written. The remote auto-routing task exists as an exchange of files (`docs/AUTOROUTE.md`): a Specctra DSN export, a session import that is checked against the rules and an adapter for an external program that is off by default; it has never been tried with a real autorouter. A board editor (`eh3d-phoenix-boardedit`, `docs/BOARD_EDIT.md`, command `phoenix boardedit`) adds the operations that the editing window above lacks, all or nothing and checked by the rules after each one: placing, mirroring and locking components, a drag with the tracks following, orthogonal and 45-degree tracks, change of layer, deleting a stretch or a net, pasting a net with the shortest route between two pads and tuning a net to a length with meanders; its window mode has never been seen on a screen or used with a mouse. The same window has a library mode (`docs/LIBRARY_EDIT.md`) that edits the footprints, symbols and devices of EAGLE libraries on a fine grid, with the properties of each item, the check of the library and the footprint generators; it has never been seen on a screen either. The fill of copper zones (`eh3d-phoenix-zones`, `docs/ZONES.md`) and the lengths, matching groups and differential pairs (`eh3d-phoenix-lengths`, `docs/LENGTHS.md`) exist as prototypes with the commands `phoenix zones` and `phoenix lengths`, tried on boards made by hand and by the tests only; the fill is not yet written in the Gerber files and a pair is not tuned as one object.

## F7. Schematic editor and synchronisation

Viewer and editor for schematics, electrical rule checks, a unified device editor, transfer to the board, forward and backward annotation by stable identity, net ties and net classes.

Gate: A simple project goes from start to finish inside the tool.

Progress: the logic that needs no window exists as a prototype (`docs/ERC.md`): the electrical rule check (`eh3d-phoenix-erc`), forward and backward annotation by stable identity as a package that can be undone (`eh3d-phoenix-sync`) and net classes for the track width and the vias in the rule file. They are tried on schematics and boards made by hand and by the tests only. The window of the viewer shows the electrical check with a jump to each pin, net or part, and the dry run of the transfer to the board and of the annotation back with a preview of the package before it is applied (undo and redo included); it has never been seen on a screen. The editing of a schematic is a prototype too (`docs/SCHEMATIC_EDIT.md`): reversible operations (place, move, turn, mirror, delete, copy and paste, values and references, wires with a T where one ends on another, net labels, power ports and bus labels) with the nets made again from the drawing, its state without a screen (grid and snapping, selection, wires that follow the pins, a palette of symbols, the electrical check after each change, the transfer to the board and the net list for the simulator), the command `phoenix schedit` and an editing mode of the window that has never been seen on a screen. The drawing of symbols, the unified device editor, net ties and the clearance of a net class are not written.

## F8. Heritage of the EAGLE workflow

Commands and script files, a script interpreter and its object model built from public language documentation and the author's own scripts, output jobs, design blocks, project and rule files, with a compatibility report per script.

Gate: Every own script and job is listed as equivalent, adapted or pending with its cause; the equivalent rate does not drop.

Progress: the interpreter and the report exist as a prototype (`eh3d-phoenix-script`, `docs/SCRIPT.md`): a reader of script files, a run of the commands with an equivalent in the board model as reversible operations (all or nothing, with an undo), the listing of every command of every script as equivalent, adapted or pending with its cause per script and per folder (text, JSON and HTML), and a reader of output jobs into an export profile. It is built from the public description of the language and tried on scripts and jobs written for its tests only; it has not been run on the author's own scripts and jobs, so the gate is not met. Design blocks, project files and rule files of the old workflow are not started.

## F9. Library sanitizer

Batch inventory of old libraries, canonical geometric hash, fuzzy matching with manual confirmation, logical duplicates, an HTML report, master libraries and replace-and-relink with backup.

Gate: Replace-and-relink changes neither layout nor rules; every exception is classified and documented.

Progress: the analysis half exists as a prototype (`eh3d-phoenix-sanitizer`, `docs/SANITIZER.md`): the batch inventory, the canonical geometric hash, the fuzzy matching with a threshold, the groups of duplicates, the HTML and JSON report and the plan of the master library, which is written and never applied; it is tried on libraries made by its own tests only. The library editor (`eh3d-phoenix-libedit`, `docs/LIBRARY_EDIT.md`) reads that plan for one library and replaces one footprint or symbol by its master only when each step is confirmed, as one step that can be undone and only when the pads or pins are named the same way and no turn is needed. Replace-and-relink with backup across boards and schematics is not written.

## F10. KiCad writing

Writing of KiCad schematics, boards and libraries.

Gate: KiCad opens them and its checks agree.

## F11. Component base and first satellites

The component database (EH3D-COMPONENT-INTELLIGENCE), then the verifier, the manufacturing profiles, the history, the passport and the full validation lab, each a library with a command line and a task kind.

Gate: Regenerating the report of an old revision gives the same result; the passport allows the manufacturing files to be rebuilt.

## F12. Circuit simulation

Netlist from the schematic, binding to an existing engine, basic analyses, a waveform viewer, reference cases with analytic solutions, deterministic convergence techniques.

Gate: The reference set passes with justified tolerances.

Progress: the first solver exists as a prototype (`eh3d-phoenix-sim`, `docs/SIM.md`, provisional decision in `docs/adr/0010-own-circuit-simulator.md`): the netlist of a schematic with a report of what could not be translated, a reader and writer of SPICE-style netlists, the operating point (with gmin and source stepping), AC with group delay, noise and transient analysis (with an adaptive step that stops at the corners of the sources) of linear circuits and circuits with diodes, bipolar and MOS transistors and op-amp macromodels, and reference cases from the textbooks that pass with the tolerances written in the tests. It is not compared with an existing engine, the device models are simple and there is no waveform viewer.

## F13. Power-supply assistant

Skeleton of the assistant (specifications, topology table with criteria, reports with result kinds and ranges of validity), small databases with provenance, first validation cases (a buck converter and an isolated flyback), import of oscilloscope measurements, current-capacity and parametrised isolation checks.

Gate: Two real supplies built and measured, with the deviation understood.

## F14. Bounded assistance by artificial intelligence

A server task with a risk-level policy and an allow list of tools; proposals are reviewable differences; it never creates or starts machine jobs.

Gate: Policy tests pass.

## F15. First mechanical steps

3D viewer, STEP/IDF/glTF export, fit against an existing enclosure and simple parametric enclosures in EH3D-PHOENIX-CAD.

Progress: the scene of the board with the models of its components as data (a model per footprint from a table, the box that encloses each STEP solid, collisions and height checked on boxes) and a simplified OBJ and STEP export exist as a prototype (`docs/ASSEMBLY3D.md`); the 3D window, the IDF and glTF export and the fit against an enclosure do not.

Gate: A small real project made entirely inside the tool, from schematic to measured result, with its passport.

## Later

Extended power (resonant, phase-shifted, power-factor correction, inverters), RF by levels with an external electromagnetic solver, progressive thermal analysis, high-speed digital, analog, full parametric mechanical CAD, further levels for other formats, and co-simulation of firmware for one processor family at a time.

See REQUIREMENTS.md for the state of each capability (all planned today) and DEPENDENCIES.md for the repositories that must exist before a phase can start.
