# EH3D-PHOENIX-PCB: 3D assembly with component models

The crate `eh3d-phoenix-assembly3d` and the command `phoenix assembly3d` put the components of a board on the board in three dimensions, with a model for each footprint taken from a table on the computer, and check the result for collisions and height (requirement PPCB-026, the 3D view and the mechanical export). The viewer has the scene as data in `eh3d_phoenix_viewer::scene3d`.

**State: prototype.** It has been tried on boards made by hand and on STEP files written by the tests, never on a model from a component maker, and no CAD program has opened the files it writes. A component is only its enclosing box: the checks are between boxes, not between the real parts.

Nothing is invented. A STEP file that is not a plain box is replaced by the box that encloses it and a finding says so; a footprint with no model is left out of the scene and a finding says so.

## The table of models

A CSV file (`--models FILE`) with a header line and the columns `footprint`, `model`, `dx_mm`, `dy_mm`, `dz_mm` and `rot_z_deg`.

```
footprint,model,dx_mm,dy_mm,dz_mm,rot_z_deg
R0603,box:1.6x0.8x0.5,,,,
SOIC8,parts/soic8.step,-2.45,-1.95,,90
```

* `model` is the name of a STEP file, relative to the folder of the table (an absolute path and `..` are refused, `PHOENIX-3D-008`), or `box:LxWxH`, a plain box in millimetres that you give: centred on the origin of the footprint in the board plane and standing on the board from height 0.
* `dx_mm`, `dy_mm` and `dz_mm` move the model on the footprint; empty is 0.
* `rot_z_deg` turns the model around the vertical axis of the footprint before it is moved. It must be a multiple of 90 so that the box stays exact.

## What is read from a STEP file

Only the least that a component needs: the box that encloses its solid, in nanometres, from the decimal text of the file and its length unit (milli, centi, deci or micro metres, metres, and the inch), with no floating-point number. The reader looks at the entities of the `DATA` section, the points of the vertices, the lines, the planes and the circles. It sorts the solid in four kinds:

| Kind | What the file has | The box |
|---|---|---|
| box | 8 vertices on 2 values of each axis and 6 plane faces | the solid itself |
| polyhedron | straight edges and plane faces | exact: the box of the vertices |
| curved | circles and cylinders as well | the circles taken by their centre plus and minus the radius on each axis, so it may be larger than the solid |
| approximate | a surface or a curve this reader does not know (a spline, a sphere, a cone) | the box of every point of the file; it may not enclose the solid |

Every kind but the first gives `PHOENIX-3D-005` and names what was replaced. A file with no vertex and no circle has no geometry (`PHOENIX-3D-006`); a file that is not a STEP file or is malformed is `PHOENIX-3D-003`; a unit it does not know is `PHOENIX-3D-004`. A model that cannot be read leaves its components out, once for the file and not once for each component.

## The scene

The scene has no screen and no window. The board lies between height 0 and its thickness, the thickness of the stack-up (`--stackup FILE`, the job file of `phoenix stackup`) or `default_board_thickness_nm` with the information `PHOENIX-3D-013`.

* A component on the top side stands on the top of the board, from the height its model starts at.
* A component on the bottom side hangs below height 0 with its model turned upside down; its box is mirrored along x before it is turned by the rotation of the component (the rotation the board file stores).
* The box of a component is the box of its model, turned by the table, moved by the table, turned by the footprint and put at the place of the component. Rotations that are a multiple of 90 degrees are exact; any other angle goes through the sine and the cosine and is rounded to the nanometre, and everything after it is integer.

## The checks

* **Collisions** (`PHOENIX-3D-010`, an error): two boxes that overlap by more than `min_overlap_nm` in the board plane and in height. Boxes that only touch never collide. The plane test is the separating-axis test on the edges of both quadrilaterals, in 128-bit integers.
* **Height** (`PHOENIX-3D-011`, an error): a component that stands higher above the top of the board, or hangs lower below the bottom, than `max_component_height_nm` (the room of your enclosure; 0 turns it off).
* **Outline** (`PHOENIX-3D-012`, a warning): a corner of a box outside the outline of the board (`check_outline`).

A run lists at most `max_findings` findings of each kind and says how many more there were (`PHOENIX-3D-017`).

## The command

```
phoenix assembly3d [--config FILE] --models FILE [--stackup FILE] [--obj FILE] [--step FILE]
                   [--json-file FILE] [--json] [--lang CODE] <board>
```

It is a dry run: without `--obj`, `--step` and `--json-file` nothing is written, and the text report goes to the screen (`--json` prints the JSON instead). Each of the three options writes a new file; a file that exists is never replaced, the input included, and when one of them exists none is written. The default settings file is `config/assembly3d.cfg` when it exists. Exit codes: 0 done, 1 a check found an error, 2 the arguments or the input are unusable.

## The export

The OBJ and the STEP file hold the board as a prism of its outline and thickness and each component as a box, in millimetres, in the same order every time and with no date or program version. `PHOENIX-3D-018` says that this is a simplified model: no copper, no pads, no holes and no real shape of the parts. The STEP file is a boundary representation with plane faces and straight edges (one solid for each prism); it has only been checked for its structure (header, entity numbers, references, counts).

The JSON report (`phoenix-assembly3d/1`) has the board, each component with its corners, heights and where its box came from, the collisions and the findings; every length is an integer number of nanometres.

## The codes

`PHOENIX-3D-001` to `PHOENIX-3D-018`, with their titles in the eight languages; the table is in `docs/ERROR_CODES.md`.

## What it does not do

* It does not model the pins, the leads, the solder or the real shape of a component; two parts that fit one inside the box of the other are reported as colliding.
* It does not read colours, assemblies, curved or free-form surfaces, or the position inside a STEP assembly.
* It does not check a component against an enclosure; the enclosure lives in EH3D-PHOENIX-CAD.
* It draws nothing: the window that shows the scene is not written yet.
