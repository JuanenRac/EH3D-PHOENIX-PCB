# EH3D-PHOENIX-PCB: printing

`eh3d-phoenix-print` turns the sheets of a schematic, the sides of a board and a panel of boards into SVG files and into a PDF, ready to print: a border, a title block, a scale bar and, when asked, the dimensions. The command is `phoenix print`, and the window of the viewer has the button **Export image/PDF**. It is a prototype: the files have been read only by the reader of this crate and, for the SVG, by a browser; **no PDF has been opened in a real PDF reader**, so a defect that only a real reader would show is not excluded.

## What is printed

| Subject | Pages | Content |
|---|---|---|
| Schematic | one for each sheet (`sheet-01.svg`, ...) | the frame of the sheet, the wires, the bodies of the symbols (a box around the pins, as the viewer draws them), the pins with their numbers, the names of the parts and of the nets, junctions (a dot where three or more wires or pins meet), power ports, ports of sheets and buses with their entries |
| Board | the assembly plan of each side (`assembly-top.svg`, `assembly-bottom.svg`) and the layers one over the other (`layers.svg`) | the plan shows the outline, the drills, the silkscreen of the side, a silhouette of each footprint (the box around its pads, a little larger), its reference designator in the middle, a ring on the first pin and the fiducials; the bottom side is drawn as seen through the board (mirrored), and the layers can be chosen |
| Panel | one page (`panel.svg`) | the copies numbered, the frame, the tabs, the V-score lines, the mouse bites, the tooling holes, the fiducials and the holes, with the width and the height of the panel |

The silhouette of a footprint is the box around its pads, not its real body, and the model holds no drawing of a symbol; the schematic is printed as the viewer draws it. A mark of polarity is not drawn on the assembly plan unless it is in the silkscreen of the board. A fiducial is a component whose reference designator starts with `FID` or whose footprint name contains `fiducial`.

## The page

Every page has a border, a **title block** in the lower right corner (project, title of the page, sheet `n/N`, revision, author, scale and paper) whose words come in the language chosen, a **scale bar** in the strip to its left, and, with `dimensions = true`, the dimension lines of the width and the height of the outline. The paper is A4, A3, Letter or a size in millimetres (`400x300`); it is turned to landscape when the drawing is wider than it is tall. The scale is the largest of the usual ones (1, 1.25, 1.5, 2, 2.5, 3, 4, 5, 6, 7.5 and 8 times a power of ten) that fits, or the exact one that fills the paper (`scale_rounding = exact`), or the one that is asked for (`--scale 2:1`, `1:2`, `0.5`), which is refused with `PHOENIX-PRINT-003` when the drawing does not fit. Lines are never thinner than `thin_line_mm`, and texts never smaller than `min_text_mm`.

The colours of the layers come with a transparency (`#rrggbbaa`), so the copper, the mask and the paste can be seen one through the other. The layers that are printed are chosen with `layers` or `--layers outline,top_copper,drill`; the border and the dimensions are always printed.

## The files

* **SVG**: one file for each page, in millimetres, with a group for each layer. The same drawing gives the same bytes every time: no date, nothing random, numbers with at most three decimals and no trailing zeros, text escaped.
* **PDF**: one file with all the pages, written by hand: the header, the objects, the table of offsets and the trailer, with every offset and every length of a stream worked out while writing. The streams are not compressed. The fonts are Helvetica and Courier (the standard fonts that every reader has) with the encoding WinAnsi, the vectors are paths and circles made of four curves, and the transparency is a graphics state. The identifier of the file is a constant and there is no date, so the same pages give the same bytes. A character that these fonts cannot show (Japanese and Chinese text) is drawn as an empty box and counted; the command says so with the warning `PHOENIX-PRINT-007`, and the SVG shows the text.
* **The check**: before a PDF is given back, the reader of the crate (`pdfcheck`) walks the table of offsets and checks that every offset points at its object, that every `/Length` is the real one, that the objects follow each other up to the table, the trailer, the catalog and the tree of pages, and that the content of every page is made of known operators with the right operands and balanced states. A PDF that fails is not written (`PHOENIX-PRINT-008`). The reader knows only what the writer writes; it is not a reader of any PDF.

## The command

```
phoenix print <schematic|board|panel> [--kind svg|pdf] [--out FOLDER|FILE] [--paper A4] [--scale N]
              [--layers LIST] [--side top|bottom|both] [--lang xx] [--config FILE] <file>
phoenix print panel [--boards A,B,...] [--panel-config FILE] ...
```

`esquema` and `placa` are accepted for `schematic` and `board`. Without `--out` it is a dry run: the pages, the paper, the scales and the files that would be written are reported and nothing is written. With `--out` the files are written to a folder that does not exist yet, or, when the name ends in `.svg` or `.pdf`, to one file that does not exist yet; **nothing that exists is ever replaced**, and the files that are read are never written. The options replace the lines of the settings; `--lang` chooses the language of the title block and of the messages (en, es, de, fr, it, ja, zh and pt, the last one the Portuguese of Portugal). Exit code 0 when done, 2 when the arguments, the settings, the input or the output are unusable.

## The settings

Every size, colour, font and text of the title block is in `config/print.cfg` and not in the code: the paper, the orientation, the margin, the scale and how it is rounded, the widths of the lines, the size of the title block and of its texts, the fonts (`helvetica` or `courier`), the project, the revision and the author, the scale bar, the dimensions, the side, the layers and one colour for the paper and for each kind of layer. A missing line keeps its default; an unknown key or a value out of range is refused with its line number (`PHOENIX-PRINT-001`). A line that starts with `#` is a comment, and a `#` inside a value is part of it.

## The window

The top bar of the viewer has a box with the place to write to, a choice between PDF and SVG and the button **Export image/PDF**, with its texts in the eight languages. It writes what is open, as it is now, to a place that does not exist yet, with the settings of `config/print.cfg` when that file exists, and says what it wrote or why it did not. The panel window has no button yet: a panel is printed with `phoenix print panel`. The window has never been seen on a screen; the pages and the writing are made in code that is tested (`print_scene`, `print_export`).

## Codes

`PHOENIX-PRINT-001` to `009`, with their titles in the eight languages, are in `ERROR_CODES.md`; `007` is a warning.

## What it does not do

See `NOT_YET.md`: the PDF has not been opened in a real reader and its text is limited to the characters of WinAnsi, the schematic has no drawing of symbols and the assembly plan no real bodies of footprints, there is no polarity mark on the assembly plan beyond the silkscreen, the panel window has no button and nothing is printed to a printer (the files are made to be printed).
