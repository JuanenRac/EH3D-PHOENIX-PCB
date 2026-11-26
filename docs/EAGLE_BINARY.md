# EH3D-PHOENIX-PCB: binary libraries of EAGLE versions 3 to 5

Crate `eh3d-phoenix-eaglebin`. It reads the `.lbr` files of the versions before the XML format into the library model of the library editor and writes a report of what was not read as it is. It is used by `phoenix libedit`, `phoenix libindex`, `phoenix ipc compare`, the library commands of the user programs and the library window. It only reads: no binary file is ever written. Boards and schematics in the binary form are not read.

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
| `0x1D`, `0x1E` | symbol, footprint | bytes 2 and 3: records of the drawing. The name of a symbol starts at byte 16. The name of a footprint starts at byte 18 with its description at byte 13; in version 3 the name starts at byte 16 and the description has bytes 13 to 15 |

## The drawing

Coordinates are signed integers in tenths of a micrometre. Widths, pad sizes, drills and text sizes are stored as halves. Angles are 12 bits of a turn; bit 12 mirrors and bit 14 spins.

| Byte 0 | Object |
|---|---|
| `0x21` | polygon: bytes 2 and 3: vertices, which follow as line records; width, spacing and a value that the XML does not hold at bytes 12, 14 and 16 |
| `0x22`, `0x24` | line or arc: 24-bit coordinates at bytes 4, 8, 12 and 16; width at byte 20; byte 22: style, cap and direction; byte 23: the kind of arc. A general arc stores one coordinate of its centre in bytes 7, 11 and 15 |
| `0x25`, `0x26`, `0x28` | circle, rectangle, hole |
| `0x2A`, `0x2B` | pad, surface-mount pad: angle at bytes 16 and 17, flags at byte 18 and name from byte 19; in version 3 there is no angle and no flags, the name starts at byte 16 and the diameter of a long pad is stored in full, not as its half |
| `0x2C` | pin: byte 12: direction, length and turn; byte 2: visibility and function; byte 13: swap level |
| `0x31`, `0x32` | text and, in version 3, the rest of its content. The half of the size has 18 bits: bytes 12 and 13 and the two low bits of byte 14, above which is the ratio. Byte 2 is the font in versions 4 and 5; in version 3 a text on a copper layer (1 to 16) is vector and any other one is proportional |
| `0x43` | drawing frame: byte 22 turns borders off, one bit each: 1 left, 2 top, 4 right, 8 bottom |

## Connections

In versions 4 and 5 the connection records of a device hold one entry for each pad, in the order of the pads: the gate (from 1) in the high bits and the pin (from 1) in the low bits. An entry is one or two bytes, and the split depends on the number of gates and on the most pins of a gate. In version 3 there is one group of records for each gate with the pad of each pin. When any entry of a device does not name an existing gate, pin and pad, none of its connections is converted and the report says so.

The technology names are packed in one string: either the last character of each name has its high bit set, or each name follows the byte `0x04`. When the devices have attributes, their names come first, each after the byte `0x01`, and each technology holds after its name the values in the same order, each after `0x02`, or after `0x03` for one that a part cannot change.

## Compared with XML saved from the same libraries

Two binary libraries (versions 3.55 and 5.6) were compared object by object with the XML that EAGLE saved from them: 70 footprints, 36 symbols and 33 device sets are in both. Every pad, surface-mount pad, pin, line, circle, rectangle, hole, polygon and frame of an object in both has a partner, and the 379 connections of the devices agree. The differences left are 8 texts with a byte (`0x99`) that the XML holds as the replacement character, 3 arcs of version 3 that the XML has with a flat cap and the order of the attributes of one device. This comparison corrected the text sizes above 13 mm, the font, the arc caps and the long pads of version 3, a field of polygons that had been written as their isolation, and the attributes of technologies.

The line styles of byte 22 of a line (its two low bits: 0 continuous, 1 long dash, 2 short dash, 3 dash-dot), the flat cap of an arc (bit `0x10`, versions 4 and 5) and the bits of the right and bottom borders of a frame agree with the XML. In version 3 that bit does not say the cap, and the arcs are read as round.

## What was measured

The program was run, one library at a time, on a private collection of 3779 binary libraries (versions 3.2 to 5.12):

* all 3779 are read;
* 3773 are saved as XML and read back as the same library; the other 6 need more than 300 MB of memory to be saved;
* 3734 have nothing that was not read as it is; the 45 others have a note, mostly about a value that is only inferred (the bit of the thermals of a pad, the flags of a polygon, a constant attribute, the left and top borders of a frame); in 4 of them the connections of some device do not fit its pins and pads and in 3 the long strings do not match the fields.

A library of many megabytes can need more than 300 MB of memory, most of it when it is saved and read back as XML.

## What is not read or only inferred

* The settings and grid of the editor, the name stored for the library and the 12 bytes after the strings.
* The user value flag of version 3 (read as off).
* The angle of a general arc is computed from its ends and the stored coordinate of the centre.
* The pour, thermals and orphans of a polygon (the defaults are used).
* The fixed font and, in versions 4 and 5, the bit that turns off the thermals of a pad follow the order of the XML values and were not seen against an XML file. In version 3 that bit does not turn thermals off.
* The mark `0x03` of a constant attribute: no XML file with such an attribute was compared.
* Boards and schematics are not read.
* The bits of the left and the top border of a frame follow from the ones of the right and the bottom: inferred.

## Limits and damaged files

Every count is checked against the records that are left. `Limits` bounds the size of the file, the records, the strings and their length, the items and the work of resolving references. A damaged file gives an error with its code or a report with notes; the tests change bytes with a fixed seed to check that it never panics.
