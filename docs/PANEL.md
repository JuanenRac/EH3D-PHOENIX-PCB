# EH3D-PHOENIX-PCB: panelization

The crate `eh3d-phoenix-panel` and the command `phoenix panel` lay a board out in a matrix of columns and rows, joined by tabs with a row of small holes ("mouse bites") or separated by V-score lines, inside an optional frame that carries fiducials and tooling holes. Every size and count is in `config/panel.cfg`. The result is a plan and the text of the outline, the V-score lines, the fiducials and the drills of the panel.

**State: prototype.** It has been tried on boards made by hand and by the tests, never sent to a manufacturer, and the files have only been read back by the import of this suite (`docs/GERBER_IMPORT.md`). The copper, the solder mask and the legend of the boards are not copied into one file per layer.

## The layout

All lengths are integer nanometres and products are computed in `i128`, so the same board and settings always give the same panel; the origin is the lower left corner of the panel.

* **The board.** Its outline gives a box (at least three points, a width and a height). An outline that is not a rectangle is spaced by its box, with `PHOENIX-PANEL-009`. The vias and the drilled pads give the holes that are copied to each board, plated when a via or a pad is on copper.
* **The matrix.** `columns` by `rows` boards, `gap_nm` apart (also between a board and the frame). The frame is `none`, `top_bottom`, `left_right` or `all`, with rails of `frame_width_nm`. `max_boards` and `max_panel_nm` limit the size (`PHOENIX-PANEL-003`).
* **Mouse bites** (`joint = mouse_bites`). Across each routed gap, `tabs_per_edge` tabs of `tab_width_nm` sit at the middle of equal parts of the edge, each with `mouse_bite_holes` holes of `mouse_bite_drill_nm` at `mouse_bite_pitch_nm`, in a row along the edge and centred in the gap. Tabs that do not fit on the edge are `PHOENIX-PANEL-004`; holes that do not fit the tab, or a gap narrower than a hole, `PHOENIX-PANEL-012` or a settings error. The outline that is routed is the edge of the panel and the edges of every board that has a neighbour, broken where the tabs are.
* **V-score** (`joint = v_score`). The boards touch (`gap_nm` is not used, with `PHOENIX-PANEL-010`) and one line goes from edge to edge of the panel along every line between boards, and between boards and rails when there is a frame. The routed outline is only the edge of the panel.
* **Fiducials.** `fiducial_count` (0, 3 or 4) at the corners of the frame, `fiducial_step_nm` along the rail from the tooling hole of the corner, with a copper circle of `fiducial_diameter_nm` and a solder mask opening of `fiducial_mask_diameter_nm`; on the bottom side too with `fiducials_both_sides`.
* **Tooling holes.** `tooling_hole_count` (0, 2, 3 or 4) of `tooling_hole_diameter_nm` at `tooling_hole_margin_nm` from the two edges of a corner.
* **Checks.** Every mark must lie entirely inside a rail (`PHOENIX-PANEL-005`) and no two marks may overlap (`PHOENIX-PANEL-006`). Without a frame the marks are left out with `PHOENIX-PANEL-011`.

## The files

`panel_files` returns the text of each file, always in this order and with the same bytes for the same plan:

* `panel_profile.gbr`: the outline that is routed, Gerber X2 with `.FileFunction` `Profile,NP`, lines of `outline_width_nm`;
* `panel_vscore.gbr`: the V-score lines (V-score only), `Other,VScore`;
* `panel_fiducial_copper_top.gbr`, `panel_fiducial_mask_top.gbr` (and `_bot` with both sides): the flashes of the fiducials;
* `panel_drill_pth.drl`: the plated holes of the boards, Excellon in millimetres with `Plated,1,2,PTH`;
* `panel_drill_npth.drl`: the holes of the boards that are not plated, the mouse bites and the tooling holes, `NonPlated,1,2,NPTH`.

The files carry the version of this program in a `GenerationSoftware` attribute and nothing else that changes between runs.

## The command

```
phoenix panel [--config FILE] [--columns N] [--rows N] [--json] [--lang CODE] [--out FOLDER] <board>
```

It reads the board (an EAGLE `.brd` or a JSON board) and prints a summary; it is a dry run unless `--out` names a folder. The files are written only if none of them exists (`PHOENIX-PANEL-008`) and the board is never written. Exit code 0 is done and 2 is any refusal. The codes `PHOENIX-PANEL-001` to `012` are in `docs/ERROR_CODES.md` with their titles in the eight languages.

## What is not here

See `docs/NOT_YET.md`: copying the copper, the solder mask and the legend of the boards into the panel files, boards rotated or mirrored in the matrix, a panel of different boards, routing slots as an outline with the width of the router bit, break-away rails with their own tabs and a check by a manufacturer.
