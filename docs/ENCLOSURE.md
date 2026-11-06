# EH3D-PHOENIX-PCB: the board against its enclosure, and the board as a STEP solid

The command `phoenix enclosure` puts the board, its components and the screw posts of an enclosure in one frame and says how far each is from the walls, the lid and the floor and whether anything runs into a post. It also writes the board alone as a STEP solid, with its holes through it. The code is in `eh3d-phoenix-assembly3d` (the modules `enclosure`, `enclosure_codes` and `prism`), next to the 3D assembly (`docs/ASSEMBLY3D.md`) whose scene it uses.

**State: prototype.** It has been tried on boards and enclosures made by the tests, with the numbers worked out by hand, and no CAD program has opened the STEP file it writes. A component is only its enclosing box, as in the assembly: a box that is bigger than the part reports a clearance that the part would not have.

## The file of the enclosure

A plain `key = value` text (`config/enclosure.cfg` is an example). Every length is in millimetres with up to six decimals, in the coordinates of the board. A line that is not valid is refused with its number (`PHOENIX-ENCL-001`).

| Key | Meaning | Default |
|---|---|---|
| `box` | the inside of a box, `LxWxH` (also `box:LxWxH`): length along x, width along y and height from the floor to the lid | |
| `contour`, `contour_layer`, `height_mm` | instead of a box, the inner contour of a DXF drawing next to the file (the largest closed contour of the layer, or of every layer) and the inner height | |
| `wall_mm` | the thickness of the walls; the checks use the inside, so this only gives the outside size | 2 |
| `standoff_mm` | the height of the underside of the board above the floor, which is the height of the posts | 3 |
| `x_mm`, `y_mm` | the lower left corner of a box, or how far a contour is moved | the box centred on the board; the contour not moved |
| `min_clearance_mm` | the least distance to keep from the walls, the lid and the floor | 0.5 |
| `post_hole_tolerance_mm` | how far the centre of a hole of the board may be from the centre of a post | 0.3 |
| `post` | a screw post, `x, y, diameter[, hole[, top]]`; as many lines as posts, at most 64 | |

Either `box` or `contour` is given, not both. The path of a contour is relative and holds no `..`. The floor is at the height `-standoff` (the underside of the board is at 0 and the top of the board at its thickness) and the lid at `-standoff + height`. A post is a cylinder from the floor to `top` above the underside of the board (to the underside when `top` is left out).

## What is checked

The board is the polygon of its outline and its thickness; each component is the box of the assembly (`docs/ASSEMBLY3D.md`: the table of models, `--models`). Without a table only the board is checked and a finding says so.

* **Walls** (`PHOENIX-ENCL-005`): the polygon of the board and the four corners of each component are tested against the polygon of the cavity, which may be concave. A shape with a corner outside, or one of whose edges crosses a wall, is an error and says by how much (a crossing says it crosses); one that is inside but nearer to a wall than the least clearance is a warning. The distance is the least between the corners of one and the edges of the other, in both directions.
* **Lid** (`PHOENIX-ENCL-006`) and **floor** (`PHOENIX-ENCL-007`): the lid minus the top of a component on the top side (and minus the top of the board), and the underside of a component on the bottom side (and of the board) minus the floor. Negative is an error, below the least clearance a warning.
* **Posts** (`PHOENIX-ENCL-008` to `010`): a component whose box enters the cylinder of a post by more than `min_overlap_nm` of `config/assembly3d.cfg`, in the plane and in height, is an error (a component on the top side is above the post and does not meet it); a post with no hole of the board (a via or the drill of a pad) within the tolerance of its centre, or with a hole smaller than the one the screw needs, is a warning; a post whose centre is not under the board is a warning.
* **Least clearances** (`PHOENIX-ENCL-011`): always reported, as information, to the walls, the lid, the floor and the posts. A negative number means that something is through.

The findings of one code are limited to `max_findings`; the rest are counted. The errors come first and the report is the same every time. Exit code: 0 nothing is through, 1 an error finding, 2 the arguments or the input are unusable.

## `phoenix enclosure`

```
phoenix enclosure <board> --enclosure FILE [--models FILE] [--stackup FILE] [--config FILE]
                  [--step FILE] [--json] [--lang CODE]
```

`--stackup` gives the thickness of the board as in `phoenix assembly3d`; without it `config/assembly3d.cfg` gives a default and a finding says so. `--json` prints the report as JSON (`phoenix-enclosure/1`: the board, the cavity, the posts, the least clearances and the findings).

## The board as a STEP solid

`--step FILE` writes the board as one solid in a STEP file (ISO 10303-21): the prism of the outline from height 0 to the thickness, with a cylindrical hole through it for each via and each drill of a pad. The top and the bottom face have a circle as an inner bound for each hole, and the wall of each hole is a cylinder face turned inside out; the lengths are exact in nanometres, the file is the same every time and it names no date and no host. The reader of the assembly reads it back (a solid with circles is of the kind "curved", whose box is larger than the solid).

A hole whose centre is outside the outline, that touches an edge, or that overlaps a hole before it, is left out of the solid with a finding (`PHOENIX-ENCL-012`); the radius is half the diameter, rounded down to the nanometre. The file always carries `PHOENIX-ENCL-013`: copper, pads, components and slots are not in it, the outline has straight edges only, and the file is written to a new place only (an existing file is never replaced). The STEP box of all the components, `phoenix assembly3d --step`, is the other export.

## Codes

`PHOENIX-ENCL-001` to `014`, with their titles in the eight languages, are in `docs/ERROR_CODES.md`.
