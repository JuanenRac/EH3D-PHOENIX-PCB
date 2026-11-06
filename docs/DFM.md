# EH3D-PHOENIX-PCB: checking a board against the capability profile of a maker

`phoenix check --dfm FILE <board>` checks a board against what a board maker can build. The profile is a plain text file of `key = value` lines (`#` starts a comment); `config/dfm/example.cfg` shows the form. It does two things at once:

* its limits (`min_track_width_nm`, `min_clearance_nm`, `min_drill_nm`, `min_via_diameter_nm`, `min_annular_ring_nm`, `min_edge_clearance_nm`, `min_mask_bridge_nm`, `min_drill_spacing_nm`, `connect_tolerance_nm`, `silkscreen_over_pads`, `silkscreen_over_mask_openings`) are the values of the usual rule check (`docs/RULES.md`) and replace the rules of the board for this run; `--dfm` cannot be given together with `--rules` or `--rules-dru`;
* it judges what the rule check does not: the number of copper layers, the size of the board and the holes.

**State: prototype.** It has been tried on boards made by hand only. **No profile of a real maker comes with the program.** The file in the repository is an example whose figures are invented round numbers that show the form; they are not the capability of any maker, and the file says `verified = false`.

## The profile

All lengths are in nanometres (1 mm is 1000000). A line that is missing, or has 0, is off. A key written twice, a key that is not known and a value out of range are refused with the number of the line (`PHOENIX-RULEFILE-002`).

| Key | Meaning |
|---|---|
| `name`, `source` | the name shown in the report, and where you read the figures (your own words) |
| `verified` | `true` only when you checked the figures against the maker; the default is `false` |
| the limits above | the values of the rule check |
| `max_copper_layers` | the most copper layers (0 to 256) |
| `min_board_short_nm`, `min_board_long_nm` | the least short and long side of the box of the outline |
| `max_board_short_nm`, `max_board_long_nm` | the greatest short and long side |
| `max_drill_nm` | the largest drill of the vias and the drilled pads |
| `max_holes` | the most holes (vias and drilled pads of the placed components) |

## The findings

| Code | When |
|---|---|
| `PHOENIX-DFM-001` | more copper layers than `max_copper_layers` |
| `PHOENIX-DFM-002` | a side of the board below its minimum (measured and limit in the finding) |
| `PHOENIX-DFM-003` | a side of the board above its maximum |
| `PHOENIX-DFM-004` | the largest drill above `max_drill_nm` |
| `PHOENIX-DFM-005` | more holes than `max_holes` |
| `PHOENIX-DFM-006` | information, given while the profile says `verified = false` |

The first five are errors and make the exit code 1; the sixth is information. The titles are in the eight languages (`--lang`) and the findings go in the same report as the others, with `--json`, `--sarif`, `--csv` and `--html` (the category `capability` in the tables, `docs/DRC_ADVANCED.md`).

## What it does not do

* It does not know any maker. Capability is whatever the file says, and nothing checks the file against a maker.
* The board is the box of its outline: the sides of a board that is not a rectangle are those of its box, and a slot or a cutout is not looked at.
* It does not judge the thickness, the aspect ratio of a hole (it needs a stack-up), the minimum text height of the silkscreen, the number of different drill sizes, or the price.
