# EH3D-PHOENIX-PCB: checking a board against the capability profile of a maker

`phoenix check --dfm FILE <board>` checks a board against what a board maker can build. The profile is a plain text file of `key = value` lines (`#` starts a comment); `config/dfm/example.cfg` shows the form. `phoenix check --dfm-profile NAME <board>` does the same with a profile that comes with the program (`jlcpcb`, `pcbway`, `ncab`, the profiles `*-dru` of `docs/FAB_PROFILES.md` or `example`, see below). It does two things at once:

* its limits (`min_track_width_nm`, `min_clearance_nm`, `min_drill_nm`, `min_via_diameter_nm`, `min_annular_ring_nm`, `min_edge_clearance_nm`, `min_mask_bridge_nm`, `min_drill_spacing_nm`, `connect_tolerance_nm`, `silkscreen_over_pads`, `silkscreen_over_mask_openings`) are the values of the usual rule check (`docs/RULES.md`) and replace the rules of the board for this run; `--dfm` and `--dfm-profile` cannot be given together, nor with `--rules` or `--rules-dru`;
* it judges what the rule check does not: the number of copper layers, the size of the board and the holes.

**State: prototype.** It has been tried on boards made by hand only. The profiles of two makers come with the program, made from the public, guiding figures of their pages of capabilities (next section); they were not checked against a quote or an order, and every profile says `verified = false`, so the report says so (`PHOENIX-DFM-006`). `config/dfm/example.cfg` is an example whose figures are invented round numbers that show the form.

## The profiles that come with the program

| Name | File | Maker | Where the figures were read |
|---|---|---|---|
| `jlcpcb` | `config/dfm_jlcpcb.cfg` | JLCPCB, standard rigid FR-4, double-sided, 1 oz copper | https://jlcpcb.com/capabilities/pcb-capabilities |
| `pcbway` | `config/dfm_pcbway.cfg` | PCBWay, standard rigid FR-4 (the standard and the advanced columns of the page, copper under 2 oz) | https://www.pcbway.com/capabilities.html |
| `ncab` | `config/dfm_ncab.cfg` | NCAB, standard service: 0.05 mm of track and space (outer layers) and a drill of 0.1 mm | the public capability sheet of the maker |
| `pcbway-dru`, `oshpark-2l-dru`, `oshpark-4l-dru`, `eurocircuits-1l-dru`, `eurocircuits-2l-dru`, `eurocircuits-4l-dru`, `eurocircuits-6l-dru`, `pcbpool-dru`, `allpcb-1l-dru`, `allpcb-2l-dru`, `allpcb-4l-dru`, `allpcb-6l-dru`, `generic-8mil-dru`, `generic-10mil-dru` | `config/dfm/<name>.cfg` | the design rules of the maker as shipped with the editor EAGLE, one file for each number of layers (the generic ones set up no layers) | the numbers of the design rules, see `docs/FAB_PROFILES.md` |
| `example` | `config/dfm/example.cfg` | none: invented figures that show the form | not applicable |

The figures of the two makers are **public, guiding numbers** taken from those pages; they are not a promise of the maker, they change without notice, and the same page can give two numbers for one thing (the page of JLCPCB names 0.15 mm and 0.1 mm as the smallest drill of boards of two or more layers; the profile uses 0.15 mm). Read the page again before an order. The figures live in the profile files and nowhere else: to follow a change of the maker, edit the file (the program has no copy of them); `--dfm FILE` reads a copy of your own. Each file says in its comments which figure stands for what the page says in more than one way: for JLCPCB the annular ring is the 0.05 mm of a via, while the pads of drilled parts ask 0.15 to 0.25 mm, the mask bridge is the 0.10 mm of the usual colours (black and white ask 0.13 mm), and the board size is the one for two layers (670 x 600 mm); for PCBWay the mask bridge, the distance to the edge and the spacing of holes are the figures of the advanced column, because the standard column gives none, and the size is the one for multilayer boards (560 x 1150 mm). The key `min_via_diameter_nm` of the PCBWay profile is off (0) because its page gives no via diameter. The profile of JLCPCB also holds the least drill of a hole that is not plated (0.5 mm) and the least width of a slot that is not plated (1.0 mm).

The further lines (`max_drill_sizes`, `board_thickness_nm`, `max_hole_aspect_x10`, `forbid_via_in_pad`, `min_silk_text_height_nm`, `min_silk_stroke_nm`, `require_even_layers`, `max_board_aspect_x10`, `forbid_blind_vias`, `min_smd_pad_nm`, `min_pad_ring_nm`, `min_hole_edge_nm`, `min_cutout_nm`, `min_unplated_drill_nm`) hold only what the page says, and are off where it says nothing. JLCPCB: silkscreen letters 1.0 mm high and lines 0.15 mm wide, surface-mount pads of 0.25 mm, a ring of 0.15 mm for the pads of drilled parts (the page asks 0.15 to 0.25 mm), thickness 1.6 mm (the usual one), and no limit of drill sizes, hole ratio, board proportion or odd layers; vias in pads and blind or buried vias are offered at extra cost, so they are not refused. PCBWay: silkscreen letters 0.8 mm high and lines 0.15 mm wide, thickness 1.6 mm (the usual one) and the hole ratio of the advanced column (up to 8 is normal and up to 10 is built as medium difficulty, so the line uses 10 and with 1.6 mm it refuses a drill below 0.16 mm); the page gives no figure for vias in pads, the size of surface-mount pads, the ring of the pads of parts beyond the general one, the distance from a hole to the edge, drill sizes, board proportion or odd layers, and offers blind and buried vias. Each is a guiding figure, not verified, like the rest.

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
| `forbid_via_in_pad` | `true` refuses a via whose centre is inside a pad of a part, with the real shape of the pad (circle, rectangle with its rounded corners, oblong) turned by the rotation of the pad and of the part, on a layer both reach (the maker does not fill and cap such vias); the default is `false` |
| `min_silk_text_height_nm` | the least height of a silkscreen letter |
| `min_silk_stroke_nm` | the least width of a printed silkscreen line, of a text and of a stroke |
| `require_even_layers` | `true` refuses a stack of three or more copper layers with an odd count (makers press the cores in pairs); the default is `false` |
| `max_board_aspect_x10` | the greatest ratio of the long side to the short side of the box of the outline, in tenths (`30` is 3 to 1) |
| `forbid_blind_vias` | `true` refuses the vias that do not join the first and the last copper layer (blind and buried vias); the default is `false` |
| `min_smd_pad_nm` | the least smaller side of a pad without a drill (a surface-mount pad) |
| `min_pad_ring_nm` | the least annular ring of a plated drilled pad (half of its smaller side less the drill); the rule check has one ring limit for vias and pads together, this line is for the makers that ask more of the pads of parts |
| `min_cutout_nm` | the least smaller side of the box of each cutout of the outline (a slot or a window that the milling bit has to cut); a diagonal slot is judged by its box, which is never narrower, so a cutout that is reported is certainly too narrow |
| `min_unplated_drill_nm` | the least drill of a hole that is not plated (a mounting hole) |
| `min_hole_edge_nm` | the least distance from the wall of a hole (a via or a drilled pad) to the straight pieces of the outline and of its cutouts; the arcs of an outline are not looked at |

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
| `PHOENIX-DFM-015` | the smaller side of a pad without a drill is below `min_smd_pad_nm` (the location is the pad) |
| `PHOENIX-DFM-016` | the ring of a plated drilled pad is below `min_pad_ring_nm` (the location is the pad; the measured value is the ring) |
| `PHOENIX-DFM-018` | the smaller side of the box of a cutout is below `min_cutout_nm` (the location is the number of the cutout, counted from 1; the measured value is the side) |
| `PHOENIX-DFM-019` | the drill of a hole that is not plated is below `min_unplated_drill_nm` (the location is the pad) |
| `PHOENIX-DFM-017` | the wall of a hole is closer to the edge than `min_hole_edge_nm` (the location is the via or the pad; the measured value is the gap) |

Every finding is an error and makes the exit code 1, except `PHOENIX-DFM-006`, which is information. The titles are in the eight languages (`--lang`) and the findings go in the same report as the others, with `--json`, `--sarif`, `--csv` and `--html` (the category `capability` in the tables, `docs/DRC_ADVANCED.md`).

## Comparing the makers

`phoenix check --dfm-compare LIST <board>` checks the same board with the profile of each maker and puts the results side by side. `LIST` is from 2 to 8 profiles separated by commas, each the name of a profile that comes with the program (`jlcpcb`, `pcbway`, `ncab`, `pcbway-dru`, ..., `example`) or a profile file (an entry with a dot or a slash is a file, and its name without the folder is its label). Each profile is used as with `--dfm`: its limits replace the rules of the board and its own lines are judged, nothing else. The output has:

* a line for each profile with whether its figures are verified, whether the board passes (no proven error) and the number of errors, warnings and information and of what could not be judged;
* for each code that is found with any profile, how many times each profile finds it, with the title of the code in the language of `--lang`;
* the codes that only some of the profiles find, with the profiles that do;
* the profiles with the fewest errors.

With `--json` the same is written as `phoenix-dfm-compare/1`: `any_passes`, `profiles` (`name`, `verified`, `passes`, `errors`, `warnings`, `information`, `inconclusive`), `codes` (each with its `counts` by profile), `only_on_some` and `fewest_errors`. The same board and profiles give the same bytes. The exit code is 0 when at least one profile accepts the board, 1 when none does and 2 when the input is unusable. It cannot be given with `--rules`, `--rules-dru`, `--dfm`, `--dfm-profile`, `--advanced`, `--sarif`, `--csv` or `--html`, and it belongs to `check`, not to `connectivity`.

It compares figures, not makers: a profile that is not verified (`verified = false`, as the profiles that come with the program are) gives what its file says and not what the maker builds, so the comparison is no recommendation of a maker and no price. The library is `compare_profiles` of `eh3d-phoenix-rules`, which reads reports that were made already.

## What it does not do

* It does not know any maker beyond the figures of the files. Capability is whatever the file says, and nothing checks the file against a maker or an order.
* The board is the box of its outline: the sides of a board that is not a rectangle are those of its box, and a slot or a cutout is not looked at.
* It does not take the thickness of the board from a stack-up: the aspect ratio of the holes uses the `board_thickness_nm` written in the profile. It does not judge the price.
* A via inside a pad is found when its centre is inside the copper of the pad, whatever its shape and angle; a via whose centre is outside the pad but whose copper overlaps its edge is not found. An odd number of copper layers is judged only for a stack of three or more.
