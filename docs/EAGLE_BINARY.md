# EH3D-PHOENIX-PCB: binary libraries of EAGLE versions 3 to 5

Crate `eh3d-phoenix-eaglebin`. It reads the `.lbr` files of the versions before the XML format into the library model of the library editor and writes a report of what was not read as it is. It only reads: no binary file is ever written. Boards and schematics in the binary form are not read.

## Where the description comes from

It was worked out from the bytes of library files, comparing what was read with XML libraries that hold objects with the same names. No program was disassembled or decompiled and no code of another reader was used. It describes what was observed, it is not a specification, and it can be wrong where nothing could be compared.

## The container

All numbers are little-endian.

| Part | Content |
|---|---|
| Record 0 | byte 0 is `0x10`; bytes 2 and 3: the records of the preamble; bytes 4 to 7: the number of records; bytes 8 and 9: the version (major, minor) |
| Records 1 to N-1 | one object each, 24 bytes; byte 0 says which |
| Strings (optional) | the bytes `13 12 99 19`, a 32-bit length, strings that end with a zero, one more zero, and 12 bytes that are not interpreted |

A text field whose first byte is `0x7F` takes the next long string. The strings follow the order of the records and, inside a record, go from the last field to the first.

## The tree

| Byte 0 | Object | Counts and fields |
|---|---|---|
| `0x11`, `0x12`, `0x13` | settings, grid, layer (preamble) | |
| `0x15` | library | bytes 4, 8 and 12: records of the sections of device sets, symbols and footprints, each with its header |
| `0x17`, `0x18`, `0x19` | section | bytes 4 to 7: records that follow; bytes 8 and 9: items |
| `0x37` | device set | bytes 2 and 3: gates; bytes 4 and 5: records of its devices; byte 6 bit 0: the user types the value |
| `0x36` | device | bytes 2 and 3: connection records; bytes 4 and 5: footprint from 1. In version 3 it is a device set of its own with its gates and connections |
| `0x3C` | connections | |
| `0x2D` | gate | bytes 14 and 15: symbol from 1 |
| `0x1D`, `0x1E` | symbol, footprint | bytes 2 and 3: records of the drawing |

## The drawing

Coordinates are signed integers in tenths of a micrometre. Widths, pad sizes, drills and text sizes are stored as halves. Angles are 12 bits of a turn; bit 12 mirrors and bit 14 spins.

| Byte 0 | Object |
|---|---|
| `0x21` | polygon: bytes 2 and 3: vertices, which follow as line records |
| `0x22`, `0x24` | line or arc: 24-bit coordinates at bytes 4, 8, 12 and 16; width at byte 20; byte 22: style, cap and direction; byte 23: the kind of arc. A general arc stores one coordinate of its centre in bytes 7, 11 and 15 |
| `0x25`, `0x26`, `0x28` | circle, rectangle, hole |
| `0x2A`, `0x2B` | pad, surface-mount pad: angle at bytes 16 and 17, flags at byte 18 and name from byte 19; in version 3 there is no angle and no flags and the name starts at byte 16 |
| `0x2C` | pin: byte 12: direction, length and turn; byte 2: visibility and function; byte 13: swap level |
| `0x31`, `0x32` | text and, in version 3, the rest of its content |
| `0x43` | drawing frame |

## Connections

In versions 4 and 5 the connection records of a device hold one entry for each pad, in the order of the pads: the gate (from 1) in the high bits and the pin (from 1) in the low bits. An entry is one or two bytes, and the split depends on the number of gates and on the most pins of a gate. In version 3 there is one group of records for each gate with the pad of each pin. When any entry of a device does not name an existing gate, pin and pad, none of its connections is converted and the report says so.

The technology names are packed in one string: either the last character of each name has its high bit set, or each name follows the byte `0x04`.

## What was measured

On a private set of 3779 binary libraries (versions 3.50 to 5.12), measured with a separate throwaway script and not with this crate:

* the record counts of the tree agree in all 3779 files;
* the long strings match the fields that ask for them in 3778;
* the connection rule of versions 4 and 5 explains 88011 of 88013 devices, and the rule of version 3 explains 4350 of 4356 devices of a smaller batch;
* against XML libraries that hold objects with the same names, most fields agree exactly; the differences looked like later changes of those libraries.

The crate itself has only been run on small libraries made by hand for its tests.

## What is not read or only inferred

* The settings and grid of the editor, the name stored for the library and the 12 bytes after the strings.
* The user value flag of version 3 (read as off).
* The angle of a general arc is computed from its ends and the stored coordinate of the centre.
* The pour, thermals and orphans of a polygon and the mixed borders of a frame (the defaults are used).
* The line styles long dash and dash-dot, the fixed font and the bit that turns off the thermals of a pad follow the order of the XML values and were not seen against an XML file.

## Limits and damaged files

Every count is checked against the records that are left. `Limits` bounds the size of the file, the records, the strings and their length, the items and the work of resolving references. A damaged file gives an error with its code or a report with notes; the tests change bytes with a fixed seed to check that it never panics.
