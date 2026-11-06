# Carry the openings of the outline and the cutouts of a pour through the file formats

`examples/guides/cutout.brd` is an EAGLE board with two kinds of opening: a closed square of wires on the layer 20 inside the outline (an opening of the board, a slot or a window) and a polygon on the layer 41 inside the pour of the layer `Top` (a cutout of the pour, where copper must not be). You write the board to DXF, read the DXF back, and write KiCad and Gerber files, and see where each opening goes.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. List what a DXF drawing of the board would hold. The outline and the opening are two closed contours on the layer `OUTLINE`; the report says `layer OUTLINE: 2` and that nothing was written.

```console
$ phoenix dxf-export board examples/guides/cutout.brd
```

2. Write the drawing to a new file.

```console
$ phoenix dxf-export board examples/guides/cutout.brd --out build/cutout.dxf
```

3. Read the drawing back into the board. The largest outer contour is the outline and the contour inside it is kept as a cutout of the outline (`PHOENIX-DXF-015`, information). The operation is a dry run, checked by the design rules before and after, so the rule file is given.

```console
$ phoenix dxf-import build/cutout.dxf --board examples/guides/cutout.brd --layer OUTLINE --rules examples/tutorials/rules_relaxed.json
```

4. Write a KiCad board. The outline and the opening go on `Edge.Cuts`, and the cutout of the pour becomes a rule area on its layer that keeps the pour out and allows tracks, vias and pads. The report lists what was written and what the format cannot say.

```console
$ phoenix kicad-export --out build/cutout.kicad_pcb board examples/guides/cutout.brd
```

5. Write the manufacturing files. The profile file has the outline and then the opening, each a closed contour, and in the copper file the cutout of the pour is written as a region of clear polarity after the pour.

```console
$ phoenix export --out build/cutout_fab examples/guides/cutout.brd
```

## How to read the result

* Where an opening comes from: in an EAGLE board, wires of the layer 20 that follow each other and close, inside the largest such ring, are openings of the board; a polygon on the layer 41 (`tRestrict`, the top) or 42 (the bottom) inside a pour of that layer is a cutout of the pour. In a KiCad board the closed contours on `Edge.Cuts` inside the largest one are the openings, and in a DXF the contours inside the largest outer contour.
* Each format says what it can. The cutouts of a pour are rule areas in KiCad, restrict areas in EAGLE and keep-out areas in the Specctra file of the autorouter (which has no hole in a plane, so tracks and vias are kept out of it too). The panel carries both kinds of opening to every copy, moved, turned and mirrored with it.

## See also

* [docs/DXF.md](../../../docs/DXF.md)
* [docs/KICAD_EXPORT.md](../../../docs/KICAD_EXPORT.md)
* [docs/EXPORT.md](../../../docs/EXPORT.md)
* [docs/PANEL.md](../../../docs/PANEL.md)
