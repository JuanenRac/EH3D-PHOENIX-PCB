# EH3D-PHOENIX-PCB: differential pairs

**State: prototype.** The board editor (`docs/BOARD_EDIT.md`) knows differential pairs: a definition of a pair, three operations that draw a pair, make its two lengths equal and correct its gap, the checks of a pair with the codes `PHOENIX-PAIR-001` to `011`, the command `phoenix pairs` and a pair tool in the board mode of the window. All of it has been tried on boards made by hand and by the tests only. No pair has been routed on a real board, nothing has been compared with another tool or with a measurement, the impedance is a quasi-static estimate from published formulas (`docs/STACKUP.md`) and the window has never been seen on a screen or used with a mouse, so the requirement of the board editor (PPCB-015) stays at `prototype`.

## What a pair is

A pair is two nets that are routed side by side. It has a name and:

| Field | Meaning | Default (`config/diffpairs.cfg`) |
|---|---|---|
| `positive`, `negative` | The names of the two nets. They must be two different nets of the board. | none |
| `width_nm` | The width of the tracks of both nets. | `default_width_nm` |
| `gap_nm` | The gap between the edges of the two tracks. | `default_gap_nm` |
| `length_tolerance_nm` | How much the copper lengths of the two nets (tracks and vias) may differ. | `default_length_tolerance_nm` |
| `max_uncoupled_nm` | The most of either net that may run farther than the coupling range from the other. | `default_max_uncoupled_nm` |
| `target_diff_ohm` | The differential impedance asked for, in ohm. Optional: it is only judged when a stack-up is given. | none |

The pairs are written in the settings file, one line for each value (`pair.NAME.field = value`), or in a JSON file of the format `phoenix-diffpairs/1`; a pair of the JSON file wins over a pair of the settings file with the same name, and the pairs are always kept in the order of their names:

```json
{
  "format": "phoenix-diffpairs/1",
  "pairs": [
    {"name": "USB", "positive": "USB_P", "negative": "USB_N",
     "width_nm": 150000, "gap_nm": 150000, "length_tolerance_nm": 100000,
     "max_uncoupled_nm": 5000000, "target_diff_ohm": 90.0}
  ]
}
```

An unknown field, a negative length, a pair with the same net twice or with a name that is used twice is `PHOENIX-PAIR-001`.

### Settings

`config/diffpairs.cfg` holds the tuned values and is the built-in default. A line that is left out keeps the value shown.

| Key | Default | Meaning |
|---|---|---|
| `default_width_nm`, `default_gap_nm` | 150000, 150000 | The width and the gap of a pair that gives none. |
| `default_length_tolerance_nm`, `default_max_uncoupled_nm` | 100000, 5000000 | The tolerance of the lengths and the longest uncoupled stretch of a pair that gives none. |
| `gap_tolerance_nm`, `width_tolerance_nm` | 25000, 10000 | How far the gap and the width may be from the ones asked for. |
| `coupling_range_nm` | 1000000 | The widest gap at which two pieces count as coupled. |
| `symmetry_tolerance_nm` | 500000 | How much the coupled lengths of the two nets may differ. |
| `via_pair_range_nm` | 3000000 | How far apart two vias may be and still be a couple. |
| `via_stagger_nm` | 1000000 | How far the via of the negative net is moved along the pair at a change of layer (0 puts the two vias side by side). |
| `impedance_tolerance_permille`, `impedance_frequency_mhz` | 100, 1000 | How far the impedance may be from the target (parts per thousand) and the frequency it is worked out at. |
| `max_pairs` | 1000 | The most pairs a file may hold. |

## The operations

The three operations are values of the file of operations of `phoenix boardedit` (`docs/BOARD_EDIT.md`). Like the others they are planned into the reversible operations of the board, tried on a copy, checked by the design rules before and after and kept only if the board accepts them and they add no rule error (unless `forced`); undo and redo give back the same bytes.

| Operation (`"op"`) | What it does |
|---|---|
| `route_pair` | Draws the two tracks of a pair side by side along a centre line given by two or more points (`pair`, `layer`, `points`; optional `shape`, `width_nm`, `gap_nm`, `horizontal_first`, `layer_changes`). The centre line is made of horizontal and vertical pieces and, with the shape `diagonal`, 45-degree pieces, like a `route`. The positive net runs on the left of the direction of the centre line and the negative net on the right, at a distance of half the width plus half the gap from it; at a corner the two tracks meet at the crossing of their offset lines. Where the pair changes layer (`layer_changes`: a list of `{"vertex": N, "layer": ID}`, the vertex counted as for `change_layer`) each net gets a via, and the via of the negative net is moved `via_stagger_nm` along the pair so that the two vias do not touch; the vertex must lie inside a straight piece. A piece too short for the offset, or a path that turns back on itself, is refused. |
| `tune_pair` | Makes the shorter net of the pair as long as the other with square meanders, to within the length tolerance of the pair (`pair`; optional `side`). The meanders are measured by `eh3d-phoenix-lengths` and are the ones of `tune_length`; unless a side is asked for, they bend away from the other net, and the other side is tried when the rules refuse the first. A pair already within its tolerance is left as it is. |
| `fix_pair_gap` | Moves the horizontal and vertical pieces of the negative net that run coupled to the positive net with a gap out of tolerance, across, to the gap of the pair (`pair`). The two pieces at the ends of a moved piece must be perpendicular to it; they grow or shrink with it. A piece at the end of a track, a piece with a neighbour at another angle and a piece too short to follow are not moved, and when pieces with a wrong gap exist and none can be moved the operation is refused. |

```json
[
  {"op": "route_pair", "pair": "USB", "layer": "01234567-89ab-7cde-8fed-cba987654322",
   "points": [{"x_nm": 10000000, "y_nm": 20000000}, {"x_nm": 30000000, "y_nm": 25000000}],
   "shape": "diagonal"},
  {"op": "tune_pair", "pair": "USB"},
  {"op": "fix_pair_gap", "pair": "USB"}
]
```

A pair that is not defined, or whose nets the board does not have, is refused (`PHOENIX-PAIR-011`, `PHOENIX-PAIR-002`). The other refusals keep the codes of the editor (`PHOENIX-BEDIT-004` to `008`). The pair operations need the pairs: `phoenix boardedit` takes them with `--pairs` and `--pair-config`.

## The checks

`phoenix pairs` measures each pair with `eh3d-phoenix-lengths` (once for each net, so the two sides can be compared) and judges it:

| Code | Check |
|---|---|
| `PHOENIX-PAIR-003` | The gap between the edges of the coupled pieces is smaller or larger than the gap of the pair by more than `gap_tolerance_nm`. |
| `PHOENIX-PAIR-004` | The coupled length of the positive net and the one of the negative net differ by more than `symmetry_tolerance_nm`. |
| `PHOENIX-PAIR-005` | More of either net than `max_uncoupled_nm` runs outside the coupling range of the other. |
| `PHOENIX-PAIR-006` | The lengths of the two nets differ by more than the length tolerance of the pair. |
| `PHOENIX-PAIR-007` | A via of one net has no via of the other net on the same layers within `via_pair_range_nm` (the nearest is paired first). |
| `PHOENIX-PAIR-008` | A piece of either net is not as wide as the pair asks for, to within `width_tolerance_nm`. |
| `PHOENIX-PAIR-009` | Warning: the differential impedance of the width and the gap of the pair, worked out from the stack-up for the layer where the positive net has most copper, is farther from the target than `impedance_tolerance_permille`, or cannot be worked out. |
| `PHOENIX-PAIR-010` | Warning: no piece of either net runs within the coupling range of the other; the gap and the symmetry are then not judged. |

The separation is measured on straight pieces only; an arc counts in the lengths but is never coupled. The report is the same every time: the pairs come in the order of their names, the findings in the order of the checks and every number is an integer (the impedance in milli-ohm). The titles of the codes are in the eight languages of the suite (`src/pair_codes.rs` of `eh3d-phoenix-boardedit`); the detail of a finding is in English.

## The commands

```text
phoenix pairs [--config FILE] [--pairs FILE] [--stackup FILE] [--json] [--lang xx] <board>
phoenix boardedit [--rules FILE] [--settings FILE] [--pair-config FILE] [--pairs FILE] [--json]
                  [--lang xx] --apply OPERATIONS.json [--out FILE] <board>
```

* `phoenix pairs` is a dry run: it never writes. The board is an EAGLE `.brd` file or the JSON of the model. `--config` is the settings and pairs file (default `config/diffpairs.cfg` when it exists, else the built-in one), `--pairs` a JSON file of pairs, `--stackup` the job file of `phoenix stackup` for the impedance. Exit code 0 when no check is proven broken (warnings do not count), 1 when one is, 2 when the arguments or the input are unusable.
* `phoenix boardedit` applies the pair operations with the rules of the command, as for the other operations.

## The window

In the board mode the panel of the tools has a box *Pair tool*. While it is on, a left click puts a point of the centre line of the pair (on the grid when snapping is on) instead of working the current tool. The panel lists the pairs, shows the points and has the buttons *Draw pair* (on the active layer), *Match lengths*, *Fix gap*, *Check pairs* and *Clear points*, and then the number of errors and warnings and the pairs that have a finding. The pairs are read from `config/diffpairs.cfg` in the folder the window is started in when the file is there and good. The state is in `board_pair` of the viewer crate and its texts, in the eight languages, in `board_pair_text`; the drawing is in `board_pair_gui`. The window cannot ask for a change of layer along a pair (a file of operations can). It has never been seen on a screen or used with a mouse.

## What is not done

The list is in `NOT_YET.md`. In short: no fan-out of the vias of a pair (the pair must be wide enough for two vias, or the rules refuse the operation), no length matching of the pieces inside a pair (the meanders of `tune_pair` leave the coupling), no routing from pad to pad by itself, the correction of the gap only moves horizontal and vertical pieces, the separation of arcs is not measured, no skew budget in time (the lengths are copper lengths), no check of the return path or of the reference plane, and the impedance is an estimate that has not been compared with a field solver or a measurement.
