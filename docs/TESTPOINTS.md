# EH3D-PHOENIX-PCB: test points

The crate `eh3d-phoenix-testpoints` and the command `phoenix testpoints` find the test points of a board, say which nets no probe can reach, and judge the test points themselves: their size, their distance to each other and to the edge of the board.

**State: prototype.** It has been tried on boards made by hand and by the tests only, never on a real board or in a test fixture. The numbers the tests expect were worked out on paper.

```
phoenix testpoints [--config FILE] [--out FILE] [--json] [--lang CODE] <board>
```

The board is a `.json` board, an EAGLE `.brd` or a KiCad `.kicad_pcb`, as for the other commands. The report goes to the output and nothing is written unless `--out` names a new file: it gets the JSON report, created exclusively, and a file that exists is never replaced (`PHOENIX-TP-004`). The exit code is 0 when the report is made, 1 when a finding of the report is an error (see `uncovered_severity`) and 2 when the arguments or the input are unusable.

## What is a test point

A test point is a pad that the board marks or a pad of a part that the settings recognise:

* the designer marked the pad as a test point: the list `test_points` of the board names it (`docs/BOARD_EDIT.md`, the operation `SetTestPoint`); a board made before the list existed has none, and a part called `TP1` is still recognised by its name;

* the reference of the part starts with one of `testpoint_prefixes` (`TP` by default) and goes on with a digit, a mark or nothing, so `TP12` and `TP_GND` are test points and `TPS5` (a chip) is not;
* or the name of the footprint holds one of `testpoint_footprint_words` in any case (`testpoint`, `test_point` and `test-point` by default).

A test point is **probed** when it is on the side of the board that `probe_side` names (`both`, `top` or `bottom`). The side is the side of the part. A via or a track that a probe could touch is not a test point.

## The nets

A net is asked to have test points when it has at least `min_pads_for_test` pads (2 by default) and its name is not in `ignore_nets` (`NC`, `N/C` and the names KiCad gives to unconnected pins by default; a name that ends in `*` matches every net that starts with the rest). It is **covered** when it has at least `required_per_net` probed test points.

A net that is not covered gives one of two findings, with the severity of `uncovered_severity` (a warning by default):

* `PHOENIX-TP-006`: the net has fewer probed test points than are asked, and none or some on the probed side;
* `PHOENIX-TP-007`: the net has test points but all of them are on the side that is not probed.

The report also gives the share of the nets asked that are covered. When no pad of the board is a test point (`PHOENIX-TP-012`) the nets are not judged one by one: the settings are the likely cause.

## The test points one by one

* `PHOENIX-TP-008`: the smallest side of the pad is below `min_pad_nm` (1 mm by default).
* `PHOENIX-TP-009`: two test points on the same side are closer than `min_spacing_nm` between their centres (1.27 mm by default).
* `PHOENIX-TP-010`: the centre is closer than `min_edge_nm` to the outline or to a cutout (1 mm by default).
* `PHOENIX-TP-011`: the pad is on no net.
* `PHOENIX-TP-013`: the pad has no opening in the solder mask on its side, so the mask would cover it. It is judged only with `require_mask_opening = true` (off by default): the pad must list a solder mask layer of the side of its part or of no side (a through-hole pad, which shows on both sides, will do with a mask layer of either side).

* `PHOENIX-TP-014`: a pad of another part on the same side is closer to the centre of the test point than `min_part_clearance_nm` (0 by default, so it is not judged): a probe needs room around the point, and a tall part beside it is in the way. For each part that has a pad within the limit the report names its reference, its nearest pad and the distance. The parts that hold a test point are not counted (the spacing between test points is `PHOENIX-TP-009`), and a part on the other side of the board is not in the way.

A limit of 0 leaves that judgement out. The defaults are common values for probes and fixtures and are not the rules of any maker: set the ones of your fixture.

## The report

The text lists the test points (reference and pad, net, side, place and size), the nets without access and the findings, with its titles in the eight languages (`--lang`). The JSON (`--json`) has the same data with the format `phoenix-testpoints/1`; the places and sizes are integer nanometres. The same board gives the same bytes every time.

## Settings

`config/testpoints.cfg` holds every value and is also the built-in default: `testpoint_prefixes`, `testpoint_footprint_words`, `use_marks` and `use_names` (which of the two ways of recognising a test point count, both on by default), `require_mask_opening`, `probe_side`, `required_per_net`, `min_pads_for_test`, `ignore_nets`, `min_pad_nm`, `min_spacing_nm`, `min_edge_nm`, `min_part_clearance_nm`, `uncovered_severity` and `max_items` (the most parts, connected pads and nets a board may hold for the check to run, `PHOENIX-TP-005`). A line that is not `key = value`, an unknown key or a value out of range is refused with its line number (`PHOENIX-TP-001`).

## Codes

`PHOENIX-TP-001` to `PHOENIX-TP-014`, with titles in the eight languages (`src/codes.rs`) and the table in `docs/ERROR_CODES.md`.

## Limits

* The access to a pad is judged by its solder mask opening only (`PHOENIX-TP-013`, off by default); the size of the opening is not compared with the pad. The room around a test point (`PHOENIX-TP-014`) is the distance to the centres of the pads of the other parts on the same side: the body and the height of a part are not in the model, so a large part with its pads far from the point can still be in the way.
* Only the centre of the test point is checked against the edge, and only the centres against each other.
* A pad of a through-hole part is on the side of the part.
* A mark is on one pad; the other pads of its part are not test points unless the part is recognised by its name.
* Nothing has been compared with the rules of a test fixture maker or with another tool.
