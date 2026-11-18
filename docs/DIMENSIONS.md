# EH3D-PHOENIX-PCB: dimensions

The board model holds dimensions: the distance between two points of the board, drawn as a dimension line with its extension lines, two arrow heads and the measure as text, on a documentation layer. They are in `crates/eh3d-phoenix-board/src/dimension.rs`.

**State: prototype.** It has been tried on boards made by hand and by the tests only. The files of KiCad and of EAGLE that carry dimensions were written by hand for the tests; the programs have never opened what this code writes, and the sign of the height of a KiCad dimension and the layers of an EAGLE one have not been compared with them. The window draws the dimensions of a board but has no tool to place one.

## The model

The board has a map `dimensions`, by id, that is not written when it is empty (a board made before it existed reads as before, byte for byte). A dimension has:

| Field | Meaning |
|---|---|
| `layer` | the layer it is drawn on; it must exist and be a documentation layer |
| `kind` | `aligned`, `horizontal` or `vertical` |
| `from`, `to` | the two points measured |
| `offset` | how far the dimension line is from the measured points, in nanometres (see below) |
| `text_height` | the height of the text, which is also the length of the arrow heads |
| `stroke` | the width of the lines and of the strokes of the text |

* `aligned`: the length of the straight line from `from` to `to`. The dimension line runs parallel to it, `offset` away to the left of the direction from `from` to `to` (to the right when the offset is negative).
* `horizontal`: the distance along x. The dimension line is horizontal, at `from.y + offset`.
* `vertical`: the distance along y. The dimension line is vertical, at `from.x + offset`.

The measure is not stored: it follows from the two points and the kind, so it cannot disagree with them. `length_nm()` gives it in nanometres (the aligned length is rounded to the nearest nanometre with whole-number arithmetic) and `measure_text()` gives it in millimetres up to the micrometre, with the zeros at the end of the decimals left out (`12.5`, `3`, `0.025`).

## The drawing

`items()` gives the drawing as the strokes and the text of the silkscreen font, all on the layer of the dimension and in a fixed order: the extension lines (none when the offset is zero), the dimension line, the four lines of the two arrow heads (as long as the text is high and a third of that wide on each side) and the text. Every division is rounded half away from zero, so the same dimension gives the same bytes on every system. The text is horizontal, centred over the dimension line, when the line is nearer to horizontal than to vertical, and runs from bottom to top, centred beside the line, otherwise. The scene of the window draws these items on the layer of the dimension.

## Operations and checks

* `AddDimension` and `RemoveDimension` are reversible operations of the board (`docs/BOARD_EDIT.md` lists the operations of the editor); the first refuses an id that is used, a missing layer and a layer that is not for documentation. The history of the window names the steps in the eight languages.
* The consistency check (`Board::validate`) reports a dimension on a layer that does not exist (`PHOENIX-BOARD-001`), on a layer that is not for documentation (`PHOENIX-BOARD-007`), and one that measures nothing, has a text height or a line width that is not above zero or does not fit the coordinate range (`PHOENIX-BOARD-004`).

## Files

| Format | Reads | Writes |
|---|---|---|
| KiCad board (`.kicad_pcb`) | `dimension` of type `aligned` and `orthogonal` (with its `orientation`) of KiCad 6 and later, on a documentation layer; the points of `pts`, the `height`, the size and thickness of the font of its text and the `thickness` of its style. The y axis is turned. Other types (leader, centre mark, radial), the format of KiCad 5 and dimensions on a layer that is not for documentation are counted as lost | the same, with the text, on `Dwgs.User`; the height is the offset turned to the other side because the y of the file points down, and the form has not been compared with KiCad |
| EAGLE board (`.brd`) | `dimension` of `dtype` `parallel` (the default), `horizontal` and `vertical`, on a documentation layer or on the layers 47 to 50 (which become documentation layers of the board); the dimension line passes through `x3`, `y3`; the stroke is the `textratio` (8 per cent by default) of `textsize` | the same, on the number of its layer (51 or 52 for a documentation layer) |

Radius, diameter, angle and leader dimensions are not in the model. The EAGLE and KiCad readers count them as not read. A DXF, IPC-2581 or print file does not carry the dimensions yet (`docs/NOT_YET.md`).
