# EH3D-PHOENIX-PCB: checking a board against the capability profile of a maker

`phoenix check --dfm FILE <board>` checks a board against what a board maker can build. The profile is a plain text file of `key = value` lines (`#` starts a comment); `config/dfm/example.cfg` shows the form. `phoenix check --dfm-profile NAME <board>` does the same with a profile that comes with the program (`jlcpcb`, `pcbway` or `example`, see below). It does two things at once:

* its limits (`min_track_width_nm`, `min_clearance_nm`, `min_drill_nm`, `min_via_diameter_nm`, `min_annular_ring_nm`, `min_edge_clearance_nm`, `min_mask_bridge_nm`, `min_drill_spacing_nm`, `connect_tolerance_nm`, `silkscreen_over_pads`, `silkscreen_over_mask_openings`) are the values of the usual rule check (`docs/RULES.md`) and replace the rules of the board for this run; `--dfm` and `--dfm-profile` cannot be given together, nor with `--rules` or `--rules-dru`;
* it judges what the rule check does not: the number of copper layers, the size of the board and the holes.

**State: prototype.** It has been tried on boards made by hand only. The profiles of two makers come with the program, made from the public, guiding figures of their pages of capabilities (next section); they were not checked against a quote or an order, and every profile says `verified = false`, so the report says so (`PHOENIX-DFM-006`). `config/dfm/example.cfg` is an example whose figures are invented round numbers that show the form.

## The profiles that come with the program

| Name | File | Maker | Where the figures were read |
|---|---|---|---|
| `jlcpcb` | `config/dfm_jlcpcb.cfg` | JLCPCB, standard rigid FR-4, double-sided, 1 oz copper | https://jlcpcb.com/capabilities/pcb-capabilities |
| `pcbway` | `config/dfm_pcbway.cfg` | PCBWay, standard rigid FR-4 (the standard and the advanced columns of the page, copper under 2 oz) | https://www.pcbway.com/capabilities.html |
| `example` | `config/dfm/example.cfg` | none: invented figures that show the form | not applicable |

The figures of the two makers are **public, guiding numbers** taken from those pages; they are not a promise of the maker, they change without notice, and the same page can give two numbers for one thing (the page of JLCPCB names 0.15 mm and 0.1 mm as the smallest drill of boards of two or more layers; the profile uses 0.15 mm). Read the page again before an order. The figures live in the profile files and nowhere else: to follow a change of the maker, edit the file (the program has no copy of them); `--dfm FILE` reads a copy of your own. Each file says in its comments which figure stands for what the page says in more than one way: for JLCPCB the annular ring is the 0.05 mm of a via, while the pads of drilled parts ask 0.15 to 0.25 mm, the mask bridge is the 0.10 mm of the usual colours (black and white ask 0.13 mm), and the board size is the one for two layers (670 x 600 mm); for PCBWay the mask bridge, the distance to the edge and the spacing of holes are the figures of the advanced column, because the standard column gives none, and the size is the one for multilayer boards (560 x 1150 mm). The key `min_via_diameter_nm` of the PCBWay profile is off (0) because its page gives no via diameter.

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
| `max_drill_sizes` | the most different drill sizes (each size costs a tool change at the maker) |
| `board_thickness_nm` | the finished thickness of the board that the maker builds; used only for the aspect ratio of the holes |
| `max_hole_aspect_x10` | the greatest ratio of the thickness to the drill, in tenths (`50` is 5 to 1); the line needs `board_thickness_nm` too |
| `forbid_via_in_pad` | `true` refuses a via whose centre is inside the largest circle of a pad of a part, on a layer both reach (the maker does not fill and cap such vias); the default is `false` |
| `min_silk_text_height_nm` | the least height of a silkscreen letter |
| `min_silk_stroke_nm` | the least width of a printed silkscreen line, of a text and of a stroke |
| `require_even_layers` | `true` refuses a stack of three or more copper layers with an odd count (makers press the cores in pairs); the default is `false` |
| `max_board_aspect_x10` | the greatest ratio of the long side to the short side of the box of the outline, in tenths (`30` is 3 to 1) |
| `forbid_blind_vias` | `true` refuses the vias that do not join the first and the last copper layer (blind and buried vias); the default is `false` |

## The findings

| Code | When |
|---|---|
| `PHOENIX-DFM-001` | more copper layers than `max_copper_layers` |
| `PHOENIX-DFM-002` | a side of the board below its minimum (measured and limit in the finding) |
| `PHOENIX-DFM-003` | a side of the board above its maximum |
| `PHOENIX-DFM-004` | the largest drill above `max_drill_nm` |
| `PHOENIX-DFM-005` | more holes than `max_holes` |
| `PHOENIX-DFM-006` | information, given while the profile says `verified = false` |
| `PHOENIX-DFM-007` | more different drill sizes than `max_drill_sizes` |
| `PHOENIX-DFM-008` | the board thickness divided by the smallest drill is above `max_hole_aspect_x10` (the measured value is the smallest drill, the limit is the least drill that passes) |
| `PHOENIX-DFM-009` | a via inside a pad while `forbid_via_in_pad` is `true` (the location is the pad, the place is the via) |
| `PHOENIX-DFM-010` | the height of a silkscreen text is below `min_silk_text_height_nm` |
| `PHOENIX-DFM-011` | the width of a printed silkscreen line, of a text or a stroke, is below `min_silk_stroke_nm` |
| `PHOENIX-DFM-012` | an odd number (three or more) of copper layers while `require_even_layers` is `true` |
| `PHOENIX-DFM-013` | the long side is longer than `max_board_aspect_x10` tenths of the short side (the limit in the finding is the longest side that passes) |
| `PHOENIX-DFM-014` | a blind or buried via while `forbid_blind_vias` is `true` (the place is the via) |

The first five are errors and make the exit code 1; the sixth is information. The titles are in the eight languages (`--lang`) and the findings go in the same report as the others, with `--json`, `--sarif`, `--csv` and `--html` (the category `capability` in the tables, `docs/DRC_ADVANCED.md`).

## What it does not do

* It does not know any maker beyond the figures of the files. Capability is whatever the file says, and nothing checks the file against a maker or an order.
* The board is the box of its outline: the sides of a board that is not a rectangle are those of its box, and a slot or a cutout is not looked at.
* It does not judge the thickness, the aspect ratio of a hole (it needs a stack-up), the minimum text height of the silkscreen, the number of different drill sizes, or the price.
