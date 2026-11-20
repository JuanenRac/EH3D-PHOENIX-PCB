# EH3D-PHOENIX-PCB: geometric checks of Gerber files against a maker

`phoenix check --gerber-dfm FOLDER|ZIP --profile NAME|FILE` reads the Gerber and Excellon files of a folder, or of a ZIP archive (unpacked in memory, never written to disk), and checks their **geometry** against the figures of a capability profile of a maker (`docs/DFM.md`, the profiles in `config/dfm_*.cfg`). It is the check for files that come from outside, where there is no board model: the same figures the check of a board uses, applied to what was drawn.

**State: prototype.** It has been tried on synthetic files only. The figures of the profiles that come with the program are not verified against the makers, and the report says so (`PHOENIX-GDF-011`).

## What it checks

The order is the one of the rule check: copper to copper, minimum width, drill and ring, board edge, mask, legend over pads.

| Check | Code | Figure of the profile | How |
|---|---|---|---|
| Spacing | `PHOENIX-GDF-001` | `min_clearance_nm` | every flash, track, arc and region is a thing; things that touch or overlap are one, and two separate things closer than the limit are found by growing each by half the limit |
| Conductor width | `PHOENIX-GDF-002` | `min_track_width_nm` | the width of the pen of every track and arc |
| Annular ring, worst case | `PHOENIX-GDF-003` | `min_annular_ring_nm` | the copper of the outer layers round each plated hole, less `registration_nm` of the settings for the error of the drill |
| Copper at the edge | `PHOENIX-GDF-004` | `min_edge_clearance_nm` | copper within the limit of what the outline layer draws |
| Mask bridge | `PHOENIX-GDF-005` | `min_mask_bridge_nm` | the openings of the mask layer, as for the spacing |
| Legend over pads | `PHOENIX-GDF-006` | `silkscreen_over_pads` | legend over copper that the mask leaves open |
| Via in a pad | `PHOENIX-GDF-007` | `forbid_via_in_pad` | the centre of a via (a tool told `ViaDrill`, or a plated drill not over `via_max_drill_nm`) inside copper that the mask leaves open |
| Floating copper | `PHOENIX-GDF-008` | none | a piece of copper with no flash, no end of a track and no hole inside it (a warning) |

The shapes are made with the 2D geometry of EH3D-CORE-LIBS (unions, differences and offsets of regions). The layers are told from the X2 attributes of each file or from its name, as `gerber-import` does.

## What it does not know

* A Gerber file has no nets unless X2 says them, so the spacing is the spacing between any two separate things, the same net included.
* The mask layers are taken as positive: a flash is an opening.
* The width is the width of tracks and arcs; a narrow neck inside a region is not found.
* A layer with clear polarity is not checked for spacing or mask bridge (`PHOENIX-GDF-009` says so); the other checks use the layer with the clear parts taken off.
* The width of a spacing finding is an estimate, good for a long narrow gap. Arcs are cut into straight pieces within `arc_chord_nm`.
* Nothing here is a certification: the maker's own review is what counts.

## The settings

`config/gerber_dfm.cfg` (or `--config FILE`) holds the numbers of the method, never the limits of the maker: the tolerance and the grid of the geometry, the registration error of the drill, the size under which a drill is taken as a via, the area of the rounding that is ignored, how many findings are kept for each check, and the limits of the ZIP archive. A key that is not known, a key twice and a value that is not a whole number are refused with the number of the line.

## The report and the exit code

Text (the default), `--json` (format `phoenix-gerber-dfm/1`, with the title of every finding in the eight languages) or `--csv FILE` (a new file; one is never replaced), in the language of `--lang` (en, es, de, fr, it, ja, zh, pt of Portugal). Exit code **0** when the files pass, **2** when the verification is not passed (at least one error), **1** when the command could not run (a wrong call, an input that cannot be read or a profile that does not exist). Warnings and notes do not fail the files.
