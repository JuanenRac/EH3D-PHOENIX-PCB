# Tutorials

## How to run the tutorials

Run every command from the root of the repository. The commands write only to new files or to a new folder inside `build/`, which you create first with `mkdir build`; nothing that exists is replaced. The output shown is the beginning of what each command prints. The blocks that start with `# test` are run by the tests of the repository, which compare the beginning of the output (a line that starts with `~` has to appear somewhere in the output). Every tool of this program is a prototype that has not been compared with a reference tool.

The help of the program is in the program: `phoenix help` lists the topics and the commands, `phoenix help sim` and `phoenix help --lang es sim` explain a topic or a command.

## Tutorials

1. [From a schematic to a netlist and a simulation](TUTORIAL_NETLIST_AND_SIMULATION.md)
1. [From a board to the fabrication files](TUTORIAL_BOARD_TO_FABRICATION.md)
1. [Import a KiCad or EAGLE project and review it](TUTORIAL_IMPORT_KICAD_EAGLE.md)
1. [Panelize a board and write the panel files](TUTORIAL_PANEL.md)
1. [Simulate an RC filter and an inverting amplifier](TUTORIAL_FILTER_AND_AMPLIFIER.md)
1. [Edit a library and generate IPC footprints](TUTORIAL_LIBRARY_AND_FOOTPRINTS.md)
1. [Match an impedance and synthesise a filter](TUTORIAL_RF_MATCHING_FILTER.md)
1. [Review the signal integrity of a differential pair](TUTORIAL_DIFFERENTIAL_PAIR.md)
1. [Use scripts: an LED matrix and a panel](TUTORIAL_SCRIPTS.md)

## Feature guides

* [Guides on the newer functions](guides/README.md): check reports and capability profiles, bill of materials, panel templates, field solver and heating, pours, cutouts, buses, autorouter, multiple selection and pictures of the window; in eight languages

## Glossary and migration guide

* [Glossary](../GLOSSARY.md)
* [Migration guide: coming from EAGLE or KiCad](../MIGRATION.md)

## Languages

* [Español](es/README.md)
* [Deutsch](de/README.md)
* [Français](fr/README.md)
* [Italiano](it/README.md)
* [日本語](ja/README.md)
* [中文](zh/README.md)
* [Português (Portugal)](pt/README.md)
