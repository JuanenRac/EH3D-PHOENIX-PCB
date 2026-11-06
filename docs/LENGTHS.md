# EH3D-PHOENIX-PCB: lengths, groups and differential pairs

The crate `eh3d-phoenix-lengths` and the command `phoenix lengths` give the copper length and the delay of the nets of a board, judge groups of nets that must match in length and judge differential pairs (requirement PPCB-016, length matching and pairs).

**State: prototype.** It has been tried on boards made by hand and by the tests only, never on a real board. The numbers the tests expect were worked out on paper. The delay from the stack-up is a quasi-static estimate, not a measurement.

## The length of a net

The length of a net is the sum of the length of every piece of track of the net and of the barrel of every via of the net. It is the total copper of the net, not the path from one pad to another: a branch counts too.

* **Straight pieces** are measured to the nearest nanometre (halves up) with an exact integer square root.
* **Arcs** are measured as the radius times the sweep, with floating-point trigonometry, and rounded; the report says that arc lengths are an estimate (`PHOENIX-LEN-012`).
* **Vias** are the distance from the middle of the copper of the first layer to the middle of the copper of the last layer in the stack-up, when a stack-up is given and names both layers; otherwise `default_via_length_nm`.

## The delay

The delay of a track is its length times the delay of its layer. The delay of a layer is, in this order:

1. the line `layer_delay_ps_per_mm.<layer name>` of the settings;
2. the delay of the stack-up (`--stackup FILE`, the job file of `phoenix stackup`, see `docs/STACKUP.md`) for the width of the track at `stackup_frequency_ghz`: a microstrip on the outer copper layers, a stripline on the inner ones. A layer the stack-up cannot give gives the warning `PHOENIX-LEN-010` and the default is used;
3. `default_delay_ps_per_mm`, with the information `PHOENIX-LEN-011`.

The delay of a via is its length times `via_delay_ps_per_mm`. Delays are whole femtoseconds, so the sums are exact; the report shows the delay of each net and, for a pair, the difference.

## Groups

A group names nets and a tolerance. Without a target the longest and the shortest net may differ by at most the tolerance (`PHOENIX-LEN-004`); with a target every net must be within the tolerance of the target (`PHOENIX-LEN-018`). A group that gives no tolerance uses `default_group_tolerance_nm`.

## Pairs

A pair names the positive and the negative net and may say:

* `max_skew_nm`: the most the two lengths may differ (`PHOENIX-LEN-005`);
* `width_nm` and `width_tolerance_nm`: the width of every piece of track of both nets (`PHOENIX-LEN-006`);
* `gap_nm` and `gap_tolerance_nm`: the gap between the edges of the two tracks. It is measured on every straight piece of the positive net against the straight pieces of the negative net on the same layer: the distance between the centre lines less half of each width. A piece within `coupling_range_nm` of the other net counts as coupled. The smallest gap of the coupled pieces below the gap asked for less the tolerance is `PHOENIX-LEN-007`, the largest above it plus the tolerance is `PHOENIX-LEN-008`;
* `max_uncoupled_nm`: the most of the positive net that may run uncoupled (`PHOENIX-LEN-009`).

A pair with no coupled stretch gives the warning `PHOENIX-LEN-019` and its gap is not judged. A piece of arc is never coupled. The board editor draws, tunes and checks a pair as one object with its own definitions and codes, and uses this crate for the lengths and the gaps (`docs/DIFFPAIRS.md`).

## The file of groups and pairs

`--groups FILE`, format `phoenix-lengths/1`:

```json
{
  "format": "phoenix-lengths/1",
  "groups": [
    {"name": "data", "nets": ["D0", "D1", "D2"], "tolerance_nm": 500000, "target_nm": null}
  ],
  "pairs": [
    {"name": "usb", "positive": "USB_P", "negative": "USB_N", "max_skew_nm": 150000,
     "gap_nm": 200000, "gap_tolerance_nm": 50000, "width_nm": 150000, "max_uncoupled_nm": 5000000}
  ]
}
```

Nets are named as the board names them; if two nets share a name the one with the lowest id is taken. A net that the board does not have is the error `PHOENIX-LEN-003` and its group or pair is left out of the results; a net with no copper has length zero and the warning `PHOENIX-LEN-013`.

## The command

```
phoenix lengths [--config FILE] [--stackup FILE] [--groups FILE] [--out FILE] [--json] [--lang CODE] <board>
```

`<board>` is an EAGLE board or the JSON board of a project. It is a dry run: it prints the report and writes nothing unless `--out` names a file, which must not exist (an existing file, the board included, is never replaced); the file holds the JSON report. Exit code 0 when done, 1 when a rule of a group or a pair is proven broken, 2 when the arguments or the input are unusable. The report has no date, host name or user name and the same input gives the same bytes: the nets come in the order of their names, the groups and pairs in the order of the file, and every number is an integer (lengths in nanometres, delays in femtoseconds; the text shows millimetres with six decimals, exactly).

## Settings

Every delay, tolerance and limit is in `config/lengths.cfg`, which is also the built-in default; no value is written in the code. A line that is not `key = value`, an unknown key or a value out of range is refused with `PHOENIX-LEN-001`.

## Codes

`PHOENIX-LEN-001` to `PHOENIX-LEN-019`, with titles in the eight languages (`src/codes.rs`) and the table in `docs/ERROR_CODES.md`.

## Limits

* The length is the total copper of the net, not the path from a driver to a receiver; a net with a stub is longer than the signal path.
* The delay of the stack-up is a quasi-static estimate of a microstrip or a stripline of the width of the track; it does not model the solder mask, the reference plane gaps or the frequency dependence of the laminate beyond the frequency set.
* The separation of a pair is measured on straight pieces only, between pieces on the same layer; a pair that changes layer is judged layer by layer.
* The tuning of lengths (meanders) is not made; the report only says how far a net is from the others.
* Nothing has been compared with the length report of the reference tool.
