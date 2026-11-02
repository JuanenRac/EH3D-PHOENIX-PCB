# EH3D-PHOENIX-PCB: scope

Status: **early code**. EH3D-PHOENIX-PCB has a workspace with a redacted inventory of a private corpus of design files; everything else below is planned. This page says what it does, what it is meant to do, what it will not do, and where it sits. Nothing here has been used on real design files or on a machine.

## Purpose

EH3D-PHOENIX-PCB is planned as an EDA environment with schematic capture, PCB layout, verification tools, circuit simulation and specialized modules for switch-mode power supplies and RF.

EDA environment: schematic capture, PCB layout, verification tools, circuit simulation and specialized modules for switch-mode power supplies and RF. It is the main project of the PHOENIX family.

## What it does today and what it is planned to do

Written, and tried only on folders made by its own tests: the command `phoenix inventory`, which counts the design files of a folder by extension, kind and declared version and gives one fingerprint of the content, without showing any name or value. Everything else in this list is planned.

* Schematic capture and multilayer PCB layout on one design model where every object has a stable identity and the model, not the window, is the source of truth.
* Every change to a project is an explicit, validated and reversible operation, so undo and redo work across the schematic, the board and the other domains.
* A project format of its own (one folder per domain) and the `.eh3dproj` package as the way to hand a design to the CAM programs of the suite.
* A format hub with honest support levels: EAGLE (versions 4 to 9, read and write where documented), KiCad (read first, write later), Gerber X2 and Excellon, and the assembly formats confirmed with real samples. Every import produces a quality report of what was kept, transformed or dropped.
* Verification: connectivity of schematic and board, schematic-to-board consistency, design-rule checks in a fixed order, and later power-oriented checks (track width against current, clearance and creepage).
* Circuit simulation through an existing, validated engine, with reference cases that have analytic solutions.
* Specialised modules: an assistant for power-supply design (from simple regulators up, each model declaring its range of validity) and RF tools by levels.
* Heritage of the previous EAGLE-based workflow: commands, scripts, a script interpreter run inside a sandbox, output jobs and library maintenance (a library sanitizer with replace-and-relink and backups).
* Computing tasks that can also run on the EH3D server; the program works without it.
* Later, bounded assistance by artificial intelligence whose proposals are always reviewable differences, and the first mechanical steps with the sister repository EH3D-PHOENIX-CAD.

## What it will not do

* It does not compete with every function of other EDA, CAD, SPICE and RF tools from the first version, and it makes no claim of being better than any of them without a fair, reproducible comparison.
* No solver of its own is written just to keep everything in Rust: existing engines are evaluated.
* No format is declared fully supported without tests; support levels are published per format and version.
* An estimate is never presented as a validated simulation: every result states its kind.
* Artificial intelligence never changes connections or critical parameters without review.
* It never starts a machine job. CAM, machine control, vision, tools and pick and place live in their own repositories and consume the project it exports.
* The server is never required to edit an ordinary project.
* A feature is not done because it has a window: it needs tests and evidence.

## Planned shape

A Rust program with a window for the desktop, built from a workspace of engine crates (model, EAGLE, KiCad, rules, engine), a command-line interface and an application. It works on its own and can use the EH3D server for computing tasks. The crate names are the planned ones; no crate exists yet.

Crates, names and file layout will be decided together with the first code; no crate exists yet.

## Where it sits

It is the main project of the PHOENIX family of the EH3D Fabrication Suite: the other repositories of the family build on its design model and its project format, and it never depends at build time on the ones that build on it.
See DEPENDENCIES.md for the relations, REQUIREMENTS.md for the requirements and STATUS.md for the honest state.
