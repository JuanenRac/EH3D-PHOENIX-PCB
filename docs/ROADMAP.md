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

## F5. KiCad reading and public oracle

Readers for KiCad schematics, boards, symbols and footprints; a public oracle in CI with a pinned tool version; a public corpus of open hardware with reviewed licences.

Gate: The rate of identical connectivity does not drop; each failure is classified.

Progress: the readers exist as a prototype (`eh3d-phoenix-kicad`, `docs/KICAD.md`): symbols, footprints, schematics with sheets, boards and the project file, with the import report and a fidelity level; they are tried on files written by hand only. The public oracle in CI and the public corpus are not started.

## F6. PCB viewer and editor

A short interface prototype and a final stack decision (a read-only viewer prototype exists, `docs/VIEWER.md`, and its stack is only provisional, `docs/adr/0009-first-interface-stack.md`), then viewer, editing, routing (45 degree tracks and arcs, vias, widths per class, live checks, ratsnest, zones) and a remote auto-routing task using an external router. The application registers as a module of the suite launcher and opens project files from the command line.

Gate: A corpus board is modified and saved, opens in the reference tool without warnings and its rule check agrees; fluidity is measured with small, medium and large boards; a re-routed board is milled.

## F7. Schematic editor and synchronisation

Viewer and editor for schematics, electrical rule checks, a unified device editor, transfer to the board, forward and backward annotation by stable identity, net ties and net classes.

Gate: A simple project goes from start to finish inside the tool.

## F8. Heritage of the EAGLE workflow

Commands and script files, a script interpreter and its object model built from public language documentation and the author's own scripts, output jobs, design blocks, project and rule files, with a compatibility report per script.

Gate: Every own script and job is listed as equivalent, adapted or pending with its cause; the equivalent rate does not drop.

## F9. Library sanitizer

Batch inventory of old libraries, canonical geometric hash, fuzzy matching with manual confirmation, logical duplicates, an HTML report, master libraries and replace-and-relink with backup.

Gate: Replace-and-relink changes neither layout nor rules; every exception is classified and documented.

## F10. KiCad writing

Writing of KiCad schematics, boards and libraries.

Gate: KiCad opens them and its checks agree.

## F11. Component base and first satellites

The component database (EH3D-COMPONENT-INTELLIGENCE), then the verifier, the manufacturing profiles, the history, the passport and the full validation lab, each a library with a command line and a task kind.

Gate: Regenerating the report of an old revision gives the same result; the passport allows the manufacturing files to be rebuilt.

## F12. Circuit simulation

Netlist from the schematic, binding to an existing engine, basic analyses, a waveform viewer, reference cases with analytic solutions, deterministic convergence techniques.

Gate: The reference set passes with justified tolerances.

## F13. Power-supply assistant

Skeleton of the assistant (specifications, topology table with criteria, reports with result kinds and ranges of validity), small databases with provenance, first validation cases (a buck converter and an isolated flyback), import of oscilloscope measurements, current-capacity and parametrised isolation checks.

Gate: Two real supplies built and measured, with the deviation understood.

## F14. Bounded assistance by artificial intelligence

A server task with a risk-level policy and an allow list of tools; proposals are reviewable differences; it never creates or starts machine jobs.

Gate: Policy tests pass.

## F15. First mechanical steps

3D viewer, STEP/IDF/glTF export, fit against an existing enclosure and simple parametric enclosures in EH3D-PHOENIX-CAD.

Gate: A small real project made entirely inside the tool, from schematic to measured result, with its passport.

## Later

Extended power (resonant, phase-shifted, power-factor correction, inverters), RF by levels with an external electromagnetic solver, progressive thermal analysis, high-speed digital, analog, full parametric mechanical CAD, further levels for other formats, and co-simulation of firmware for one processor family at a time.

See REQUIREMENTS.md for the state of each capability (all planned today) and DEPENDENCIES.md for the repositories that must exist before a phase can start.
