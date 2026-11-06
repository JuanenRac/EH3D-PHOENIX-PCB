# EH3D-PHOENIX-PCB: panelization

The crate `eh3d-phoenix-panel` and the command `phoenix panel` lay a board out in a matrix of columns and rows, or lay a list of different boards out, joined by tabs with a row of small holes ("mouse bites") or separated by V-score lines, inside an optional frame that carries fiducials and tooling holes. Every size and count is in `config/panel.cfg`. The result is a plan, the text of the outline and the V-score lines of the panel, and the files of the layers of every copy: the copper, the solder mask, the paste, the legend and the drills, written by the export of this suite.

**State: prototype.** It has been tried on boards made by hand and by the tests, never sent to a manufacturer, and the files have only been read back by the import of this suite (`docs/GERBER_IMPORT.md`).

## The layout

All lengths are integer nanometres and products are computed in `i128`, so the same board and settings always give the same panel; the origin is the lower left corner of the panel.

* **The board.** Its outline gives a box (at least three points, a width and a height). An outline that is not a rectangle is spaced by its box, with `PHOENIX-PANEL-009`. The vias and the drilled pads give the holes that are copied to each board, plated when a via or a pad is on copper.
* **The matrix.** `columns` by `rows` copies of one board, `gap_nm` apart (also between a board and the frame). The frame is `none`, `top_bottom`, `left_right` or `all`, with rails of `frame_width_nm`. `max_boards` and `max_panel_nm` limit the size (`PHOENIX-PANEL-003`).
* **Turn and mirror.** `copy_rotation_deg` (0, 90, 180 or 270, counter-clockwise) and `copy_mirror` apply to every copy of a matrix. A board is mirrored about the vertical axis of its box first and then turned; a turn of 90 or 270 degrees swaps the width and the height of the box. A mirrored copy is the board seen from its other side: its layers change sides (the top copper layer becomes the bottom one, the second layer the one before the last, a top legend layer the bottom one), its components change sides and its text is written back to front.
* **A list of different boards.** `phoenix panel --boards A,B,C` (a board that appears twice is placed twice) lays the boards out in the order of the list, packed by rows from left to right with `gap_nm` between them; a row is at most `pack_row_width_nm` wide. Instead of packing, one line `place = BOARD,X_nm,Y_nm,ROTATION_DEG,MIRROR` for each board puts it where it is told (BOARD counts from 0; X and Y count from the lower left corner of the area inside the frame). Boards that overlap or are closer than the gap are refused (`PHOENIX-PANEL-014`), and a list or a place that cannot be used is `PHOENIX-PANEL-013`. A board with no tab on any side is reported with `PHOENIX-PANEL-020`. All the boards need the same layers (the same kind, name and side) and the same number of copper layers as the first one, with `PHOENIX-PANEL-018` otherwise.
* **Mouse bites** (`joint = mouse_bites`). Across each routed gap between two boards, or between a board and a rail, `tabs_per_edge` tabs of `tab_width_nm` sit at the middle of equal parts of the edge they share, each with `mouse_bite_holes` holes of `mouse_bite_drill_nm` at `mouse_bite_pitch_nm`, in a row along the edge and centred in the gap. Tabs that do not fit on the edge are `PHOENIX-PANEL-004`; holes that do not fit the tab, or a gap narrower than a hole, `PHOENIX-PANEL-012` or a settings error. The outline that is routed is the edge of the panel and the edges of every board that has a neighbour, broken where the tabs are.
* **V-score** (`joint = v_score`). The boards touch (`gap_nm` is not used, with `PHOENIX-PANEL-010`) and one line goes from edge to edge of the panel along every line between boards, and between boards and rails when there is a frame. The routed outline is only the edge of the panel. In a list of boards a line is drawn only where it crosses no board and separates two boards that touch (or a board and a rail); when no line can separate two of the boards, because they are not in a grid, the panel is refused with `PHOENIX-PANEL-016`.
* **V-score and copper.** A line may not pass closer than `v_score_clearance_nm` to a pad, a track, a via or a zone of a board (0 turns the check off): the panel is refused with `PHOENIX-PANEL-015` and the message names the line and the object. Every object is held in a box a little larger than itself, so a line that passes close to an arc, to a pad turned by an angle that is not a right angle or to a zone may be refused although it is clear; a zone is held in the box of its outline.
* **Fiducials of the frame.** `fiducial_count` (0, 3 or 4) at the corners of the frame, `fiducial_step_nm` along the rail from the tooling hole of the corner, with a copper circle of `fiducial_diameter_nm` and a solder mask opening of `fiducial_mask_diameter_nm`; on the bottom side too with `fiducials_both_sides` (when the board has a bottom copper layer).
* **Fiducials of the boards.** `local_fiducial_count` (0, 2 or 3) on every board, `local_fiducial_inset_nm` from the two edges of its corner, with the same sizes as the others; they are on the same sides as the fiducials of the frame. A board too small for them is `PHOENIX-PANEL-017`.
* **Tooling holes.** `tooling_hole_count` (0, 2, 3 or 4) of `tooling_hole_diameter_nm` at `tooling_hole_margin_nm` from the two edges of a corner.
* **Checks.** Every mark of the frame must lie entirely inside a rail (`PHOENIX-PANEL-005`) and no two marks may overlap (`PHOENIX-PANEL-006`). Without a frame the marks are left out with `PHOENIX-PANEL-011`.
* **How much the boards use.** The part of the area of the panel that the boxes of the boards take is given in the summary, in thousandths in the JSON (`utilisation_permille`) and in the window.

## The files

`phoenix panel --out FOLDER` writes, to a folder that does not exist yet:

* the layers: one Gerber file for each layer of the first board (but the outline), `panel_<number>_<name>.gbr`, with the pads, tracks, vias, zones and legend of every copy moved to its place, turned and mirrored as the plan says, and the fiducials of the frame and of the boards on the copper and the solder mask of their side, through `eh3d-phoenix-export` with the export profile of `--profile` (default `generic`, from `config/export_profiles.json`), so the Gerber X2 attributes the profile asks for are written too (`docs/GERBER_X2.md`: the net, the pin and the component of every object, the function of every aperture, with the fiducials as fiducial pads);
* the drills: `panel_drill_pth.drl` with the plated holes of the copies and `panel_drill_npth.drl` with the holes that are not plated, the mouse bites and the tooling holes, named and written as the profile says;
* the placement file and the bill of materials of the copies, `panel_placement.csv` and `panel_bom.csv` (the marks of the panel are not parts);
* `panel_profile.gbr`: the outline that is routed, Gerber X2 with `.FileFunction` `Profile,NP`, lines of `outline_width_nm`;
* `panel_vscore.gbr`: the V-score lines (V-score only), `Other,VScore`.

The outline layers of the boards are not written: the routed outline of the panel is `panel_profile.gbr`. The same plan always gives the same bytes; nothing reads the clock. The identity of every part of the panel is made by the program in a fixed order, so it is the same every time.

With `copy_suffix = true` the reference designators and the net names of the copies are followed by `copy_suffix_separator` and the number of the copy (`R1_1`, `R1_2`, `SIG_1`); without it every copy keeps the names of the board, as they do in a panel that a manufacturer receives.

### The check before anything is written

The files are made from a board of the panel (`compose`): the copies of the boards with new identities, and the marks as parts of their own (a copper circle and a mask opening for a fiducial, a pad with a drill and on no layer for a hole). Before anything is written:

* the holes of that board are compared with the holes of the plan (kind, diameter and place), so the geometry of the panel is the geometry of the copies plus the marks;
* the files are read back with the import of this suite and compared with that board, as in the round trip of `docs/GERBER_IMPORT.md`: the flashes, regions, lines and arcs of every layer, the holes, and the Gerber X2 attributes of the profile.

A difference is `PHOENIX-PANEL-019` and nothing is written. A refusal of the export itself is `PHOENIX-PANEL-021`.

## The command

```
phoenix panel [--config FILE] [--columns N] [--rows N] [--boards A,B,...] [--profile NAME] [--profiles FILE] [--json] [--lang CODE] [--out FOLDER] [<board>]
```

It reads the board (an EAGLE `.brd`, a KiCad board or a JSON board) or the list of boards and prints a summary; it is a dry run unless `--out` names a folder. The folder must not exist (`PHOENIX-PANEL-008` otherwise), the files are never written over and the boards are never written. `--columns` and `--rows` apply to a matrix of one board. Exit code 0 is done and 2 is any refusal. The codes `PHOENIX-PANEL-001` to `021` are in `docs/ERROR_CODES.md` with their titles in the eight languages.

## The window

`phoenix view --panel [--panel-config FILE] [BOARD[,BOARD...]]` (or the button *Panel* of the start screen, which takes the board files typed in the box) opens a window that draws the panel: the copies with their numbers, the frame, the routed outline, the tabs, the V-score lines, the mouse bites, the fiducials, the tooling holes and the holes of the boards, each on a layer that can be hidden, with the measures of the panel, the part of its area that the boards take, the notes of the layout and the columns, the rows and the joint to change. A click on a board marks it. `phoenix view --panel --summary BOARD[,BOARD...]` prints the measures without a window. The window only draws the panel; the files are written by `phoenix panel`. See `docs/VIEWER.md`.

## Printing

`phoenix print panel` writes the panel as an SVG or PDF page, with the copies numbered, the tabs, the V-score lines, the mouse bites, the tooling holes, the fiducials and the width and the height (`docs/PRINT.md`). The panel window has no button for it yet.

## What is not here

See `docs/NOT_YET.md`: routing slots as an outline with the width of the router bit, break-away rails with their own tabs, the layers of a panel of boards with different layers, and a check by a manufacturer. A zone is written as its outline, as in the export of a board, and the window has never been seen on a screen.
