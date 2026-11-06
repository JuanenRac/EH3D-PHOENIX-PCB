# EH3D-PHOENIX-PCB: example scripts

**State: prototype.** The scripts below are in `examples/scripts/` and are built into the program: `phoenix script list` shows them and `phoenix script list --show NAME` prints one. Each is run by a test on a board, a schematic or a library made by hand, and its output is compared with the text written next to it, so an example cannot drift from the language. The language is in `docs/SCRIPT_API.md`.

Run one on a file of your own, as a dry run first:

```
phoenix script list --show led_matrix > led_matrix.phs
phoenix script run led_matrix.phs --board my_board.json
phoenix script run led_matrix.phs --board my_board.json --out my_board_with_leds.json
```

The board of the tests has two resistors `R1` and `R2` of the footprint `R0603` on two copper layers; the scripts that need more (a footprint called `LED0603`, capacitors and an IC) are tried on that board with those added. A script that uses a footprint or a designator that your board lacks stops with PHOENIX-SCRIPT-059 and changes nothing.

## `bom_resistors`

A bill of materials of the resistors as CSV text, written to the data folder when applied. For a board; `examples/scripts/bom_resistors.phs`.

```
phoenix-script/2 board
# A bill of materials of the resistors (R*) as CSV. It is written to the data folder (data_dir of
# config/script.cfg) only when the run is applied with --out, and never over a file that exists.
let rows = ["designator,value,footprint"]
for part in sort_refdes(select("component", "R*")) {
    rows = push(rows, join([part.refdes, part.value, part.footprint], ","))
}
for row in rows {
    print(row)
}
print(len(rows) - 1, "resistors")
if data_enabled() {
    let size = write_text("bom_resistors.csv", join(rows, "\n") + "\n")
    print("wrote", size, "bytes")
} else {
    print("data_dir is not set: the file is not written")
}
```

Output on the document of the tests:

```
designator,value,footprint
R1,10k,R0603
R2,1k,R0603
2 resistors
wrote 52 bytes
```

## `circle_placement`

Places components on a circle of a radius around a centre, in integer arithmetic. For a board; `examples/scripts/circle_placement.phs`.

```
phoenix-script/2 board
# Place count components of a footprint on a circle of a radius around a centre, turned to face
# outwards. The angles come from the whole turn divided by the count, in integers.
let count = 4
let radius = 10mm
let center_x = 20mm
let center_y = 15mm
for k in 0..count {
    let angle = muldiv(k, deg(360), count)
    let x = center_x + polar_x(radius, angle)
    let y = center_y + polar_y(radius, angle)
    let name = "D" + str(k + 1)
    place(name, "R0603", x, y, angle)
    print(name, mm(x), mm(y))
}
```

Output on the document of the tests:

```
D1 30 15
D2 20 25
D3 10 15
D4 20 5
```

## `decoupling_check`

Reports the capacitors that are too far from the nearest IC (changes nothing). For a board; `examples/scripts/decoupling_check.phs`.

```
phoenix-script/2 board
# A report of the capacitors (C*) that are farther than max_distance from the nearest IC (U*).
# It changes nothing.
let max_distance = 5mm
let ics = select("component", "U*")
let far = 0
let caps = sort_refdes(select("component", "C*"))
for cap in caps {
    let ic = nearest(cap, ics)
    if ic == nil {
        print(cap.refdes, "has no IC to be near")
        far = far + 1
    } else if distance(cap, ic) > max_distance {
        print(cap.refdes, "is", mm(distance(cap, ic)), "mm from", ic.refdes)
        far = far + 1
    }
}
print(far, "of", len(caps), "capacitors are too far")
```

Output on the document of the tests:

```
C2 is 10 mm from U1
1 of 3 capacitors are too far
```

## `header_footprint`

Makes the footprint of a single-row pin header of N pins in a library. For a library; `examples/scripts/header_footprint.phs`.

```
phoenix-script/2 library
# A footprint for a single-row pin header of N pins on a 2.54 mm pitch, with round pads.
let pins = 6
let pitch = 2.54mm
let name = "HEADER_1X" + str(pins)
lib_create("package", name)
for k in 0..pins {
    lib_add_pad(name, str(k + 1), k * pitch, 0mm, 1mm, 1.7mm)
}
print(name, "with", lib_pad_count(name), "pads")
```

Output on the document of the tests:

```
HEADER_1X6 with 6 pads
```

## `led_matrix`

Places an N x M matrix of LEDs on a pitch, with consecutive designators and a common net. For a board; `examples/scripts/led_matrix.phs`.

```
phoenix-script/2 board
# An LED matrix: columns x rows LEDs on a pitch, designators D1, D2, ... in rows, and the second pad
# of every LED on one common net. The board needs a footprint with the name below.
let columns = 3
let rows = 2
let pitch = 2.5mm
let origin_x = 5mm
let origin_y = 5mm
let footprint = "LED0603"
let common = "LEDCOM"

for r in 0..rows {
    for c in 0..columns {
        let n = r * columns + c + 1
        let led = place("D" + str(n), footprint, origin_x + c * pitch, origin_y + r * pitch, 0, "LED")
        connect(led, "2", common)
    }
}
print("placed", rows * columns, "LEDs on net", common)
print("pads on the net:", select("net", common)[0].pads)
```

Output on the document of the tests:

```
placed 6 LEDs on net LEDCOM
pads on the net: 6
```

## `panelize_matrix`

Panelizes the board as an N x M matrix of copies. For a board; `examples/scripts/panelize_matrix.phs`.

```
phoenix-script/2 board
# Panelize the board as a matrix of columns x rows copies with a gap between them. The designators
# of the copies end with _1, _2, ... The run replaces the board as a whole, so it has no batch that
# undoes it; phoenix panel makes the frame, the tabs and the files of the fabrication.
let columns = 2
let rows = 2
let gap = 2mm
let before = len(select("component", "*"))
let copies = panelize(columns, rows, gap)
let after = len(select("component", "*"))
print("copies:", copies)
print("components:", before, "->", after)
```

Output on the document of the tests:

```
copies: 4
components: 2 -> 8
```

## `renumber_by_position`

Renumbers the designators of a family from left to right. For a board; `examples/scripts/renumber_by_position.phs`.

```
phoenix-script/2 board
# Renumber the capacitors C1, C2, ... from left to right (then from bottom to top on a tie). The
# names are changed in two passes, so a new name never clashes with an old one on the way.
let caps = sort_position(select("component", "C*"), "xy")
let renamed = renumber(caps, "C", 1)
print("renumbered", renamed, "components")
for cap in sort_position(select("component", "C*"), "xy") {
    print(cap.refdes, cap.x)
}
```

Output on the document of the tests:

```
renumbered 3 components
C1 10000000
C2 20000000
C3 30000000
```
