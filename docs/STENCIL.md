# EH3D-PHOENIX-PCB: solder paste stencil

The module `stencil` of the crate `eh3d-phoenix-export` and the command `phoenix stencil` make the openings of the solder paste plate of each face from the paste layer of the board, with the reduction, the windows and the checks that a stencil maker asks for. Every number is in `config/stencil.cfg`.

**State: prototype.** It has been tried on boards made by hand and by the tests only. No stencil was ever made from the files and no stencil maker has read them; the limits of the settings follow IPC-7525 as the author knows it, which has not been compared line by line with the standard, and they are values to change for the maker you use.

## The openings

For each paste layer that says its face (`PHOENIX-STENCIL-002` otherwise), every pad that lists that layer gives one opening, in the order of the designators and the numbers of the pads. A circle stays a circle, a rectangle a rectangle and an oblong pad an obround opening.

### The reduction

An opening is smaller than its pad. `shrink` is the reduction of every footprint that no line `reduce = PATTERN VALUE` matches; the lines are tried in order against the name of the footprint (`*` for any run of characters, `?` for one, any case). A value is:

* a per cent, `10%`: each side of the pad is that much shorter (the opening keeps 90 per cent of the width and of the height);
* a length, `0.03mm`: each edge is that far inside the pad (the width and the height lose twice the length).

An opening with a side below `min_feature_mm` after the reduction is taken away, listed in the report as `removed` and reported (`PHOENIX-STENCIL-010`).

### Windows

A large pad printed whole puts too much paste on the board. When the area of the opening is larger than `window_min_area_mm2` (0 turns the splitting off), a rectangular or oblong pad is divided in a grid of windows of at most `window_max_mm` per side, joined by bridges of `window_bridge_mm` (the pane is `(side - (n - 1) bridges) / n`). The panes of an oblong pad have square ends. A circle is never split. When the panes would be smaller than `min_feature_mm` the pad stays whole (`PHOENIX-STENCIL-011`); each split is reported (`PHOENIX-STENCIL-005`).

### Corners and turned pads

`corner_radius_mm` rounds the corners of rectangular openings and panes (no more than half the smaller side); each corner is drawn with `corner_segments` straight pieces. An opening with rounded corners, and one turned by an angle that is not a right angle (`PHOENIX-STENCIL-006`), is written as a region; the others are flashes of a circle, rectangle or obround aperture, the sides swapped for a turn of a right angle.

## The checks

With the thickness of the plate (`thickness_mm`) the report gives, for the smallest side of an opening (a window when it is split):

* the **aspect ratio**, the smaller side over the thickness, at least `min_aspect` (1.5 in the settings that come with the program);
* the **area ratio**, the area of the opening over the area of its walls, `w h / (2 (w + h) t)` for a rectangle and `d / (4 t)` for a circle, at least `min_area_ratio` (0.66).

A pad below a limit is flagged in the report (`aspect`, `area` or `aspect+area`) and reported (`PHOENIX-STENCIL-003`, `PHOENIX-STENCIL-004`); the opening is written all the same. The ratios are whole millionths worked out with integers.

## The files

* `stencil_top.gbr`, `stencil_bottom.gbr`: Gerber, millimetres, format 4.6, dark polarity. The Gerber X2 attributes follow the export profile: at the level `legacy` the three attributes `.GenerationSoftware`, `.FileFunction` (`Paste,Top` or `Paste,Bot`) and `.FilePolarity` (`Positive`); at `x1` none; at `file`, `aperture` and `full` the file attributes of `docs/GERBER_X2.md`, and at `full` the object attributes `.C` and `.P` of each opening (`TO`, ended by `TD`). A face with no opening has no file.
* `stencil_report.csv`: one line for each opening, also the ones taken away: `face,reference,pad,footprint,shape,pad_width_mm,pad_height_mm,aperture_width_mm,aperture_height_mm,windows,aspect_ratio,area_ratio,status`.

The same board and settings always give the same bytes. A region with rounded corners or a turn that is not a right angle uses the sine and cosine of the machine once for each point, rounded to the nanometre.

## The command

```
phoenix stencil [--config FILE] [--profile NAME] [--profiles FILE] [--lang CODE] [--out FOLDER] <board>
```

It is a dry run: it prints the number of openings, how many were split, how many are below a ratio and how many were taken away, and the findings. With `--out` it writes the files to a folder that does not exist yet; an existing folder is never touched (`PHOENIX-STENCIL-009`). Exit code 0 is done and 2 is any refusal. The codes `PHOENIX-STENCIL-001` to `011` are in `docs/ERROR_CODES.md` with their titles in the eight languages.

## What it does not do

* It does not step the pads (a smaller opening for a pad beside a through hole or a fine pitch), apply a different reduction to the two sides of an opening, add the relief of a fiducial or the frame of a stencil, or write a plate for a panel.
* It does not know the rules of a stencil maker; the thickness and the limits are settings.
* A pad that is not on the paste layer has no opening, and an opening is never derived from the copper.
