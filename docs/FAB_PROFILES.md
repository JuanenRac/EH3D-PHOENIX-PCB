# EH3D-PHOENIX-PCB: profiles of manufacturers

Two kinds of profile of a manufacturer come with the program, both written from numbers and names that the editor EAGLE ships for each maker, and a few public figures:

* **capability profiles** (`phoenix check --dfm-profile NAME`, `--dfm-compare`, `docs/DFM.md`): the limits of the rule check and what the maker can build;
* **export profiles** (`phoenix export --profile NAME`, `docs/EXPORT.md`): the names and the order of the layer files, the format of the coordinates and the names of the drill files.

**State: prototype.** Every figure is `verified = false`: nothing was compared with an order, a quote or the maker. They are the numbers of a file, not a promise of the maker.

## How they are made

`tools/import_eagle_profiles.py --dru FOLDER --cam FOLDER` reads the design rules (`.dru`) and the CAM jobs (`.cam`) of such a folder, only to read, and writes `config/dfm/<name>.cfg` and `config/export_profiles_makers.json`. The result is what is versioned; the tool is not part of the build. It takes only facts: the lengths (converted to integer nanometres exactly: 1 mil is 25400, 1 mm is 1000000, 1 inch is 25400000; a value that is not a whole number of nanometres stops the run), the lists of layers, the file extensions and the coordinate formats. It copies no text of the sources. A key that the source does not give is left out of the profile, and a line that is missing is off. It reads both formats of a CAM job (the old text one and the JSON one).

## Capability profiles from design rules

The line of the profile on the left is taken from the key of the design rules on the right: `min_track_width_nm` from the minimum width of a wire, `min_clearance_nm` from the distance between two wires, `min_drill_nm` from the minimum drill, `min_annular_ring_nm` from the least ring of a via on an outer layer, `min_edge_clearance_nm` from the distance from copper to the outline, `min_drill_spacing_nm` from the distance between two holes, `min_pad_ring_nm` from the least ring of a pad on the top layer, and `max_copper_layers` from the number of layers the rules set up (generic rules set up none). Nothing else is written: a file of design rules has no mask bridge, no via diameter, no size of board and no number of holes, so those lines are off.

| Name | Layers | Track | Clearance | Drill | Via ring | Edge | Hole gap | Pad ring |
|---|---|---|---|---|---|---|---|---|
| `pcbway-dru` | 2 | 152400 | 205000 | 300000 | 203200 | 1016000 | 203200 | 254000 |
| `oshpark-2l-dru` | 2 | 152400 | 152400 | 254000 | 127000 | 381000 | 152400 | 127000 |
| `oshpark-4l-dru` | 4 | 127000 | 127000 | 254000 | 101600 | 381000 | 152400 | 101600 |
| `eurocircuits-1l-dru` | 1 | 150000 | 150000 | 350000 | 300000 | 250000 | 350000 | 300000 |
| `eurocircuits-2l-dru` | 2 | 150000 | 150000 | 350000 | 125000 | 250000 | 350000 | 175000 |
| `eurocircuits-4l-dru` | 4 | 150000 | 150000 | 350000 | 125000 | 250000 | 350000 | 175000 |
| `eurocircuits-6l-dru` | 6 | 150000 | 150000 | 350000 | 125000 | 250000 | 350000 | 175000 |
| `pcbpool-dru` | 2 | 152400 | 152400 | 304800 | 152400 | 508000 | 203200 | 203200 |
| `allpcb-1l-dru` | 1 | 152400 | 152400 | 250000 | 152400 | 304800 | 254000 | 152400 |
| `allpcb-2l-dru` | 2 | 152400 | 152400 | 250000 | 152400 | 304800 | 254000 | 152400 |
| `allpcb-4l-dru` | 4 | 152400 | 152400 | 250000 | 152400 | 304800 | 254000 | 152400 |
| `allpcb-6l-dru` | 6 | 152400 | 152400 | 250000 | 152400 | 304800 | 254000 | 152400 |
| `generic-8mil-dru` | none | 254000 | 203200 | 609600 | 203200 | 1016000 | 254000 | 254000 |
| `generic-10mil-dru` | none | 254000 | 254000 | 609600 | 203200 | 1016000 | 254000 | 254000 |

All lengths are in nanometres. The rules of PCBWay ask other distances too (wire to pad, pad to pad and the like) from 203200 to 205000; the profile uses the one between two wires, which is the one the rule check has. The comparison `phoenix check --dfm-compare` takes from 2 to 8 names, so the table is compared in two lists, for example `jlcpcb,pcbway-dru,oshpark-2l-dru,oshpark-4l-dru,eurocircuits-2l-dru,pcbpool-dru,allpcb-2l-dru,generic-8mil-dru`. Its report is written in the eight languages (`--lang`); the names of the profiles are never translated.

### The profile of JLCPCB and the profile `ncab`

`jlcpcb` (`config/dfm_jlcpcb.cfg`) holds the public figures of the capability page of the maker (see the file for the figures that no line can hold, such as the thicker copper, the multilayer figures, the via and the distances); it now also holds the least width of a slot that is not plated (1.0 mm, `min_cutout_nm`) and the least drill of a hole that is not plated (0.5 mm, `min_unplated_drill_nm`). `ncab` (`config/dfm_ncab.cfg`) holds the public figures of the capability sheet of NCAB: 0.05 mm of track and space (the outer layers; the inner ones are 0.03 mm) and a mechanical drill of 0.1 mm. Both are `verified = false`.

## Export profiles from CAM jobs

`config/export_profiles_makers.json` holds one profile for each job, chosen by `phoenix export --profile NAME` (also `fabpack`, `stencil` and `panel`) when the default profile file has no profile of that name and `--profiles` is not given. Each says in `gerber.names` the template of the file name of every role it knows (`{board}` is the name of the board as a plain file name; `{inner}` is the number of the inner layer from 1; `{layer}` is its place in the copper stack, the first inner layer being 2) and the order of the files, in `gerber.decimal_digits` the decimals of the coordinates when the job gives its format, and in `drill` the names of the drill files and, when the job gives its format, its style.

| Name | Gerber | Excellon | Top copper | Bottom copper | Drill file |
|---|---|---|---|---|---|
| `jlcpcb-2l-cam` | 3.4 | 3.3 | `{board}.GTL` | `{board}.GBL` | `{board}.XLN` |
| `jlcpcb-4l-cam` | 3.4 | 3.3 | `{board}.GTL` | `{board}.GBL` | `{board}_drill.XLN` |
| `jlcpcb-6l-cam` | 3.4 | 3.3 | `{board}.GTL` | `{board}.GBL` | `{board}_drill.XLN` |
| `pcbway-cam` | 3.4 | 3.3 | `{board}.top` | `{board}.bot` | `{board}.drd` |
| `oshpark-2l-cam` | 3.4 | 3.3 | `{board}.toplayer.ger` | `{board}.bottomlayer.ger` | `{board}.drills.xln` |
| `oshpark-4l-cam` | not given | not given | `{board}.toplayer.ger` | `{board}.bottomlayer.ger` | `{board}.drills.xln` |
| `sunstone-2l-cam` | 3.4 | 3.3 | `{board}.top` | `{board}.bot` | `drills_1_16.xln` |
| `elecrow-cam` | not given | not given | `{board}.GTL` | `{board}.GBL` | `{board}.TXT` |
| `apc-2l-cam` | not given | not given | `{board}.TOP` | `{board}.BOTTOM` | `{board}.NcDrill` |
| `apc-4l-cam` | not given | not given | `{board}.TOP` | `{board}.BOTTOM` | `{board}.NcDrill` |
| `seeed-2l-cam` | not given | not given | `{board}.GTL` | `{board}.GBL` | `{board}.TXT` |
| `seeed-4l-cam` | not given | not given | `{board}.GTL` | `{board}.GBL` | `{board}.TXT` |
| `lpkf-2l-cam` | not given | not given | `{board}.TOP` | `{board}.BOT` | `{board}.DRD` |
| `advancedcircuits-2l-cam` | 3.4 | 3.3 | `{board}.cmp` | `{board}.sol` | `drills_1_16.xln` |
| `isocam-cam` | not given | not given | `{board}.TOP` | `{board}.BOTTOM` | `{board}.DRD` |
| `jlcpcb-kicad-names` | default | default | `{board}-F_Cu.gbr` | `{board}-B_Cu.gbr` | `{board}-PTH.drl` |
| `jlcpcb-altium-names` | default | default | `{board}.gtl` | `{board}.gbl` | `{board}.txt` |
| `jlcpcb-eagle-names` | default | default | `Copper_top.gbr` | `Copper_bottom.gbr` | `Drill_1_16.xln` |
| `jlcpcb-thermal-relief` | default | default | numbered | numbered | default |
| `jlcpcb-mixed-drill` | default | default | numbered | numbered | `{board}.drl` (one file) |

The coordinate formats are in millimetres. The export always writes four digits before the point, so only the digits after it come from the job. A job that gives no format keeps the default (six decimals in Gerber, the default style in Excellon). The last three profiles are not from a job: they follow the names that the public page of JLCPCB lists for the files of three programs (KiCad, Altium and EAGLE), and the profiles hold only the names that the page lists; the rest of the files keep the numbered names.

Things to know:

* every job writes the plated and the not plated holes to one drill file, and the export keeps two; the file of the holes that are not plated is the same name with `_npth` before the extension (`-NPTH` in the KiCad style, where the page names it). That name is a choice of this program, not of the job;
* the 4 and 6 layer jobs of JLCPCB put a token between the board name and the extension; the profiles leave it out;
* a name that starts with a dot in a job (`.slk`) is read as the board name followed by it;
* a role with no template (the legend of the bottom side in the job of PCBWay, the paste in several jobs) and a layer without a role (documentation) keep the numbered name with the extension of the profile;
* the files of the roles in `gerber.names.order` come first, in that order, as the jobs list them; the files that are not listed follow in the usual order;
* two files with one name (for example every inner layer with a template that has no `{inner}` or `{layer}`), a name that is not a plain file name (letters, digits, `.`, `_`, `-`, at most 64 characters, no Windows device name) and an unknown role are errors that name the profile and the key (`PHOENIX-EXPORT-011`).

## Thermal relief and paste

These are public figures of the pages of JLCPCB and PCBWay; nothing was compared with a maker.

* **Relief of a pad in a pour:** four spokes (IPC-2221B), a least radius of 0.2 mm. Thin passives: spoke 0.10 to 0.20 mm, gap 0.20 to 0.30 mm. General surface-mount parts: 0.20 to 0.30 mm and 0.25 to 0.40 mm. Holes through a plane: 0.30 to 0.50 mm and 0.40 to 0.60 mm. Above 3 A a pad has no relief. The profile `jlcpcb-thermal-relief` sets `zone_thermal_spokes = 4`, the spoke at 0.30 mm and the gap at 0.40 mm, the least of the ranges for a hole through a plane; the pads that are not round stay solid (`zone_thermal_non_round = false`).
* **Thermal vias under an exposed pad:** drill 0.25 to 0.33 mm (0.2 mm for fine pitch), pitch 1.0 to 1.2 mm, 9 to 25 vias, at least 0.15 mm from the edge of the pad. No line of a profile holds them; they are in the comments of `config/dfm_jlcpcb.cfg`.
* **Paste:** coverage of 50 to 75 % of a large pad with windows, voiding under 25 %, stencil 0.10 to 0.15 mm (0.08 mm ultra fine), area ratio above 0.66, aspect ratio at least 1.5, reduction of the openings 5 to 7 % (10 to 20 % for fine pitch), pads more than 1.27 mm apart and parts at least 1 mm apart. `config/stencil_smt.cfg` (`phoenix stencil --stencil-config config/stencil_smt.cfg`) holds 0.12 mm, 1.5, 0.66 and a reduction of 6 %; the splitting into windows keeps its defaults, and the reduction of fine pitch parts is left to a `reduce` line of your own.

## The job of ISOCAM

`BUNGARD_ISOCAM.cam` is a job of the old text format with four outputs, and `isocam-cam` follows it. These are the facts of the job and nothing more:

| Output | Extension | Device | EAGLE layers |
|---|---|---|---|
| Top | `.TOP` | Gerber RS-274X | 1 Top, 17 Pads, 18 Vias |
| Bottom | `.BOTTOM` | Gerber RS-274X | 16 Bottom, 17 Pads, 18 Vias |
| Drill | `.DRD` | Excellon | 44 Drills, 45 Holes |
| Dimension | `.DIM` | Gerber RS-274X | 20 Dimension |

* The package is for two copper layers: there is no inner layer, no solder mask, no legend and no paste. The extension goes after the board name.
* The text of the description of the job names also 51 and 25 (documentation and names of the top side) for the dimension file, but the output itself draws layer 20 only.
* There is one drill file for all the holes (plated and not), while the export writes two (`.DRD` and `_npth.DRD`); which one the program ISOCAM needs, or whether it reads both, has not been checked.
* The job does not give a coordinate format, so the export keeps its default.

What is not known: that the program ISOCAM accepts the files of the export. Nothing was run with it.

## Tests

The Rust tests build a board of four copper layers and check the names, the order, the formats and the drill names of every export profile, refuse names that clash and device names, and try random board names with a fixed seed; the capability tests check the numbers of every file and shuffle the lines of each file with a fixed seed; the tool has tests with text of its own in the formats of the sources (`tools/test_import_eagle_profiles.py`), and no file of any other program is part of the tests.
