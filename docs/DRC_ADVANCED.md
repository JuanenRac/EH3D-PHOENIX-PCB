# EH3D-PHOENIX-PCB: advanced design rule checks

**State: prototype.** The checks run beside the rule engine (`eh3d-phoenix-rules`) and its live check in the board editor; they add codes and change none of the existing ones. They have been tried on boards made by hand and by the tests only. Nothing here has been compared with the rules of a manufacturer or with another tool, and the limits in `config/drc_advanced.cfg` are examples, not anybody's rules.

## What is checked

| Code | Rule | What it measures |
|---|---|---|
| PHOENIX-DRC-001 | `annular_ring_min` | The ring of a via (pad diameter minus drill, halved) and of a drilled pad (its smaller side minus the drill, halved). |
| PHOENIX-DRC-002 | `copper_edge` | The gap between a track or a via and the outline of the board. |
| PHOENIX-DRC-003 | `pad_edge` | The gap between a pad and the outline of the board. |
| PHOENIX-DRC-004 | `drill_class` | The drill of a via or a drilled pad against the minimum of the class of its net, or the default minimum. |
| PHOENIX-DRC-005 | `silk_hole` | The gap between silkscreen and the drill of a through hole (drilled pads and vias through the whole stack). |
| PHOENIX-DRC-006 | `neck` | A track narrower than `neck_min_width_nm` that joins, at one end, wider copper of its net (a pad, a via or a wider track). |
| PHOENIX-DRC-007 | `acid_trap` | The angle between two pieces of tracks of one net that leave the same point. |
| PHOENIX-DRC-008 | `isolation` | The gap between copper of two nets that have a voltage between them, against the table of spacing. |
| PHOENIX-DRC-009 | `island` | Tracks, vias and zones of a net that are joined to each other but to no pad. |
| PHOENIX-DRC-010 | `stub` | A track with one free end and one joined end. |
| PHOENIX-DRC-011 | `via_duplicate` | Two vias that share a layer and are at most `via_duplicate_max_distance_nm` apart (0: the same place). |
| PHOENIX-DRC-012 | `pad_no_net` | A pad of a placed component that is on no net. |
| PHOENIX-DRC-013 | `outside_outline` | A component with a pad whose centre is outside the outline. |
| PHOENIX-DRC-014 | `advanced_not_judged` | What could not be judged (information, in the inconclusive list). |
| PHOENIX-DRC-015 | | A settings file that is not valid. |

The existing rules keep their own codes (`PHOENIX-CHECK-001` to `015`). Where an advanced check looks like one of them (the annular ring, the distance to the edge, the smallest drill) it has its own limit and its own level in `config/drc_advanced.cfg` and is reported under its own code, so the two never hide each other.

## Settings: `config/drc_advanced.cfg`

Lines are `key = value`; blank lines and lines starting with `#` are ignored. The file is strict: a key the program does not know, a key written twice (`iso_row` is the only one that repeats), a value out of range and a file larger than 1 MiB are refused with `PHOENIX-DRC-015` and the number of the line. A line that is left out keeps the value of the file built into the program.

* `<check>_level = off | info | warning | error` for each check. A level of `off` turns the check off; only `error` makes `phoenix check` exit with 1. The levels are the severity of the findings, so a limit can be reported without failing the board.
* The limits are in nanometres (`annular_ring_min_nm`, `copper_edge_min_nm`, `pad_edge_min_nm`, `silk_hole_min_nm`, `neck_min_width_nm`, `via_duplicate_max_distance_nm`, `drill_default_min_nm`) and in degrees (`acid_trap_min_angle_deg`).
* Drill classes: `drill_class.NAME.nets = A, B` and `drill_class.NAME.min_nm = N`. A net that is in no class has the default minimum; a net in two classes is refused.
* `voltage.NET = V` gives the voltage of a net, in volts. A net that has no voltage is not judged by the spacing check; write the ground net as `voltage.GND = 0`.
* `grid_cell_nm` is the size of the cell of the search grid. It changes the speed only.

## Spacing by voltage

The check takes the difference of the voltages of two nets, picks the row of the table whose highest voltage is not below it, reads the spacing of the column `iso_column` and compares it with the gap between the copper of the two nets on a shared layer. Above the last row the spacing is the figure per volt of `iso_per_volt_nm` times the voltage. The gap is measured on the surface of one layer; it is a clearance for copper on the same layer and not a creepage path through the board, and nothing is computed between layers.

The table in the file is Table 6-1, Electrical Conductor Spacing, of IPC-2221 (Generic Standard on Printed Board Design), with the columns B1 (internal conductors), B2 (external, uncoated, sea level to 3050 m), B3 (external, uncoated, above 3050 m), B4 (external, permanent polymer coating), A5 (external, conformal coating over the assembly), A6 (external component lead or termination, uncoated) and A7 (external component lead or termination, conformal coating). **The figures were written from memory and have not been verified against the standard.** Check each one against your copy of the standard before you rely on it; every finding of this check says so. The standard is not part of this repository and no part of its text is copied.

## Method

* All arithmetic on distances is exact integer arithmetic on nanometres, as in the rule engine. A shape that is too coarse to tell (a pad that is not round is read as its inscribed and its circumscribed circle) or numbers too large for the exact test go to the inconclusive list and never turn into a failure or a pass. Arcs in tracks are measured by their end points: a distance found that way is proven, a distance not found says nothing about the arc.
* The acid trap angle is worked out in floating point from the exact integer directions of the pieces and compared with a tolerance of a billionth of a degree; an exact right angle is therefore not an acute one. Only straight pieces are looked at, and only where two pieces share an end point: a track that ends on the middle of another is not seen.
* A stub is judged only when the other end is joined. A track with both ends free is an island and is reported by the island check.
* Pairs of items that may be close are found with a uniform grid. The grid chooses which pairs are compared and the same exact test decides, so the report equals the one of the search that compares every pair (`IndexMode::Naive`); the tests check it on boards of 170 items with several cell sizes. Items that cover too many cells are kept apart and compared with all.
* The report is in a fixed order (rule, place, position), so the same board and settings give the same bytes.

## Reports

`phoenix check --advanced [--drc-config FILE] [--json | --sarif] [--lang xx] <board>` runs the rule engine and then the advanced checks and prints one report. `--drc-config FILE` implies `--advanced`. Three forms:

* text, with the titles in `en`, `es`, `de`, `fr`, `it`, `ja`, `zh` or `pt` (Portuguese of Portugal), the confirmed and the inconclusive findings in two lists;
* JSON, the same two lists with the codes (`phoenix-check/1`);
* `--sarif`, a JSON layout like SARIF named `phoenix-drc/1`: `rules` (the codes that appear, with name and title), and `results` with `ruleId`, `level` (`note`, `warning` or `error`), `kind` (`fail` for a proven finding, `review` for an inconclusive one), `message`, `location` (the name and `at_nm`) and `properties` (`measured_nm`, `limit_nm`, `detail`). It is not the OASIS SARIF and no SARIF reader has been tried on it.

Exit code: 0 when nothing is proven wrong, 1 when a finding of level `error` is proven, 2 when the input is unusable.

## In the board editor

The key `advanced_drc` of `config/boardedit.cfg` (off by default) adds the proven findings of the advanced checks, with the built-in settings, to the check the editor makes before and after every operation, so `added` and `removed` of the report of an operation include them and an operation that adds a finding of level `error` is refused when `reject_new_errors` is on. With the key off the behaviour of the editor is the one it had before.

## Not done

* Zone outlines and filled zones against the edge, thermal reliefs, and the distance from a hole to copper of another net.
* Creepage along a path (slots, cut-outs, the surface of a coating) and clearance in air between layers.
* A neck inside a zone, or along a track between two obstacles.
* Voltages by net class or from the schematic; the voltage is a list of names in the settings file.
* A check of the IPC-2221 figures against the standard, and of the whole set against a manufacturer's rules or another tool.
