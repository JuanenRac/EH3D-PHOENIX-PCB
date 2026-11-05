# EH3D-PHOENIX-PCB: diagnostic codes

Codes have the form `PHOENIX-<AREA>-<NNN>`. A code is stable: it never changes its meaning and is never reused. Programs show the text through the key `diag.<code>`; the codes themselves are never translated. The object identity (UUID v7) is the one of the shared container, with its own codes `EH3D-ID-001` and `EH3D-ID-002`. The container codes `EH3D-PKG-001` to `EH3D-PKG-014` belong to the shared container and are listed in the registry of the documentation repository.

Each table repeats the `REGISTRY` of a crate (`eh3d-phoenix-model` in `src/diagnostic.rs`, `eh3d-phoenix-project` and `eh3d-phoenix-eagle` in `src/codes.rs`, `eh3d-phoenix-hub` in `src/lib.rs`, `eh3d-phoenix-export` in `src/codes.rs`, `eh3d-phoenix-schematic` in `src/model.rs` and `src/link.rs`, `eh3d-phoenix-board` in `src/validate.rs`, `src/connectivity.rs` and `src/rules.rs`, `eh3d-phoenix-rules` in `src/codes.rs`, `eh3d-phoenix-stackup` and `eh3d-phoenix-rf` in `src/error.rs`, `eh3d-phoenix-viewer` and `eh3d-phoenix-sanitizer` in `src/codes.rs`); a test of each crate checks that every code of its registry is in the table with the same title.

## Model foundations

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-UNITS-001 | Coordinate outside the design range | a coordinate not strictly inside plus or minus 2^30 nm |
| PHOENIX-UNITS-002 | Angle outside the range | an angle outside the range in micro-degrees |
| PHOENIX-UNITS-003 | Arithmetic overflow | an intermediate result that did not fit |
| PHOENIX-UNITS-004 | Invalid decimal text | a decimal that cannot be read exactly |
| PHOENIX-UNITS-005 | Degenerate arc | an arc with no defined shape |
| PHOENIX-UNITS-006 | Number is not finite | a floating-point input that is NaN or infinite |
| PHOENIX-JSON-001 | Integer not exactly representable as a JSON number | an integer no double equals |
| PHOENIX-JSON-002 | Serialisation failed | a value that could not be turned into JSON |
| PHOENIX-JSON-003 | Number is not finite | NaN or infinity in canonical JSON |
| PHOENIX-JSON-004 | Number too large to quantise exactly | a millimetre or degree value too large to round exactly |

## Project layer (`.phoenixproj`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-MODEL-001 | Strong reference dangles | a link between critical domains whose object does not exist |
| PHOENIX-MODEL-004 | Critical domain unknown or newer | a critical domain whose name or version this reader does not understand: the project is refused, or opened read-only when the setting allows it |
| PHOENIX-MODEL-005 | Weak reference is stale | a link from a non-critical domain to an object deleted later (information, never repaired by guessing) |
| PHOENIX-MODEL-006 | Field marked must_understand is not understood | an object marked `must_understand` whose type the reader does not know |
| PHOENIX-PROJECT-001 | Manifest invalid | `project.json` cannot be read or is inconsistent |
| PHOENIX-PROJECT-002 | No migration registered for this version | a project older than this reader with no registered step to bring it up |
| PHOENIX-PROJECT-003 | Entry in a domain the manifest does not declare | an entry under a folder that the manifest does not list as a domain |
| PHOENIX-PROJECT-004 | Domain preserved untouched | a domain that is not critical and is not understood: kept as it is |
| PHOENIX-PROJECT-005 | Project opened read-only | an unknown critical domain with the read-only setting on |
| PHOENIX-PROJECT-006 | Migration failed | a registered step that returned an error |

## Board model

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-BOARD-001 | Reference to a part that does not exist | a component, pad assignment, track, via or zone that names a footprint, component, pad, net or layer that is not in the board |
| PHOENIX-BOARD-002 | Track pieces do not join | a track with no pieces, or a piece that does not start where the one before ends |
| PHOENIX-BOARD-003 | Straight track piece off the 45-degree grid | a straight piece that is not horizontal, vertical or at 45 degrees (a warning) |
| PHOENIX-BOARD-004 | Size is zero or negative | a width, drill, diameter, clearance, pad dimension or reference designator that is not positive or not empty |
| PHOENIX-BOARD-005 | Pad assigned twice or pad nets out of order | the list of pad nets is not strictly sorted by pad, so a pad appears twice or the order is wrong |
| PHOENIX-BOARD-006 | Outline is not a closed polygon | an outline with fewer than three points, repeated points or crossing edges |
| PHOENIX-BOARD-007 | Layer of the wrong kind | a track, via or zone on a layer that is not copper, or a via that starts and ends on one layer |
| PHOENIX-BOARD-008 | Duplicate reference designator | two components with the same reference designator |
| PHOENIX-BOARD-009 | Arc has no shape | an arc piece with a zero sweep or end points that do not fit its sweep |
| PHOENIX-BOARD-010 | Net is split into islands | a net whose pads, tracks, vias and zones do not all touch: the connections still to route (information, in `src/connectivity.rs`) |
| PHOENIX-BOARD-011 | Arc judged by its end points only | a track with an arc piece, whose connections are judged by its end points (information, in `src/connectivity.rs`) |
| PHOENIX-BOARD-012 | Copper stack entry is invalid | an entry of the copper stack that names a layer that does not exist, is not copper or is listed twice (in `src/stack.rs`) |
| PHOENIX-BOARD-013 | Copper stack leaves out a copper layer | a stack that is given but does not list every copper layer of the board (in `src/stack.rs`) |
| PHOENIX-BOARD-014 | Silkscreen text rotation is not a right angle | no longer raised: a silkscreen text may have any rotation; the code stays so that its number is not reused (in `src/silk.rs`) |
| PHOENIX-BOARD-015 | Silkscreen text does not fit the coordinate range | a silkscreen text whose characters reach outside the checked coordinate range (in `src/silk.rs`) |
| PHOENIX-BOARD-016 | Silkscreen text has characters without a glyph | a silkscreen text with characters that the built-in font does not have, drawn as boxes (a warning, in `src/silk.rs`) |

## EAGLE reader (`eh3d-phoenix-eagle`, `src/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-EAGLE-001 | Not well-formed XML | a file that the XML parser refuses (an open tag, a mismatched closing tag, a repeated attribute, two roots) |
| PHOENIX-EAGLE-002 | Not an EAGLE document | a root element that is not `eagle`, a file with no drawing, or a drawing with no library, board or schematic |
| PHOENIX-EAGLE-003 | EAGLE version outside 6 to 9 | a missing or unsupported declared version (a warning: the file is still read) |
| PHOENIX-EAGLE-004 | Size, depth or node limit exceeded | an input larger than the byte limit, nested deeper than the depth limit or with more elements than the node limit |
| PHOENIX-EAGLE-005 | Not valid UTF-8, read as Latin-1 | bytes that are not UTF-8 (a warning: the text is read byte by byte as Latin-1) |
| PHOENIX-EAGLE-006 | Device names a missing package | a device whose package is not in the library (a warning) |
| PHOENIX-EAGLE-007 | Gate names a missing symbol | a gate whose symbol is not in the library (a warning) |
| PHOENIX-EAGLE-008 | Connection names a missing pad or pin | a connection to a pad that the package does not have (a warning) |
| PHOENIX-EAGLE-009 | Element counted but not interpreted | an element of a board, schematic or library that the reader counts under its name and does not interpret (information) |
| PHOENIX-EAGLE-010 | File could not be read | a copy that cannot be read from disk |
| PHOENIX-EAGLE-011 | Copy differs from the original | a copy whose SHA-256 differs from the original, or an original that changed while it was copied |
| PHOENIX-EAGLE-012 | EAGLE binary format is not read | a file that starts with the bytes 0x10 0x80 like the binary files of the old versions, which this reader does not read |
| PHOENIX-EAGLE-013 | File of another program | a file with an EAGLE extension that starts like a file of KiCad (the old text formats) or of P-CAD; the message names the program |
| PHOENIX-EAGLE-014 | Binary variant is not read | a file that starts with the byte 0x10 but not with 0x10 0x80, a binary variant that this reader does not read |
| PHOENIX-EAGLE-015 | Pair is incomplete | a binary file without its `.xml` file, or an `.xml` file without its binary file (in `src/pairs.rs`) |
| PHOENIX-EAGLE-016 | Side of a pair is not what it says | a binary side that does not start like a binary file, or an XML side that is not an EAGLE XML file |
| PHOENIX-EAGLE-017 | Hash of a pair does not match | a file of a pair that is not in `PAIRS.sha256`, whose SHA-256 differs from it, or a manifest that is missing or has a line that is not `<sha256>  <path>` |
| PHOENIX-EAGLE-018 | The two sides of a pair differ | a field of the summary of the binary side that differs from the summary of the XML side |
| PHOENIX-EAGLE-019 | Pairs verified but not compared | the folder of pairs was checked without a reader for the binary side (information) |
| PHOENIX-EAGLE-020 | Item written in a simpler form | an item of the board that the EAGLE file cannot say as it is, written in a simpler form: a drilled rectangle as a square, a drilled oblong pad as a round one, a surface pad on two layers on one, a silkscreen text as the strokes that are printed, a name used twice made unique (information, in `eh3d-phoenix-eagle-board`) |
| PHOENIX-EAGLE-021 | Item left out of the EAGLE file | an item with no place in the file: a component without a footprint, a track, zone, via or silkscreen item on a layer that has no EAGLE number, a silkscreen item that cannot be drawn, a seventeenth copper layer (a warning) |
| PHOENIX-EAGLE-022 | Part of the EAGLE file not read into the board | elements of a board file that the board reader does not interpret, counted by name (information) |
| PHOENIX-EAGLE-023 | EAGLE file cannot be read into a board | text that is not XML, whose root is not `eagle` or that has no board in its drawing (an error) |

## Rule check (`eh3d-phoenix-board`, `src/rules.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-RULE-001 | Rule file line not understood | a line of the rule file that is not `name = value`, has an unknown name or a value that is not a non-negative number of millimetres |
| PHOENIX-RULE-002 | Track narrower than the minimum | a track whose width is below `min_track_width` |
| PHOENIX-RULE-003 | Drill or via smaller than the minimum | a via or pad drill below `min_drill`, or a via diameter below `min_via_diameter` |
| PHOENIX-RULE-004 | Annular ring thinner than the minimum | a via or pad whose ring of copper around the drill is thinner than `min_annular_ring` |
| PHOENIX-RULE-005 | Clearance violated | straight track pieces of different nets on one layer, or such a piece and a via of another net, closer than `min_clearance` edge to edge (proven with integer arithmetic) |
| PHOENIX-RULE-006 | Clearance could not be judged | a track with an arc, or numbers too large for the exact test (information, listed apart from the confirmed findings) |
| PHOENIX-RULE-007 | Copper too close to the edge of the board | a track, via or pad closer to an edge of the outline than `min_edge_clearance` (proven with integer arithmetic) |
| PHOENIX-RULE-008 | Solder mask bridge too thin | two pads with an opening on the same mask layer and less than `min_mask_bridge` of mask between them |
| PHOENIX-RULE-009 | Pad shape too coarse to judge | a rectangular or oblong pad whose distance to something lies between what its inscribed circle and the circle around it prove (information, listed apart from the confirmed findings) |
| PHOENIX-RULE-010 | Rule asks for what the board does not say | the rule `silkscreen_over_pads` while a silkscreen layer does not say its side, while the board gives no copper stack and a pad has no drill (information, in `src/rules_more.rs`) |
| PHOENIX-RULE-011 | Zone clearance smaller than the minimum | a zone whose own clearance is below `min_clearance` |
| PHOENIX-RULE-012 | Zones of different nets overlap | two zones of different nets on one copper layer whose outlines cross or contain each other |
| PHOENIX-RULE-013 | Silkscreen printed over a pad | a stroke or a character of the silkscreen closer to a pad that shows on the same side than the width of the printed line allows (proven; a pad that is not round can also be listed apart as `PHOENIX-RULE-009`) |
| PHOENIX-RULE-014 | Silkscreen printed over a solder mask opening | a stroke or a character of the silkscreen closer than the width of the printed line allows to a pad that has a solder mask layer of the side of the silkscreen layer, so that the pad is left without mask there (the rule `silkscreen_over_mask_openings`; proven, listed apart when a pad that is not round or an arc cannot be judged) |

## Format hub (`eh3d-phoenix-hub`, `src/lib.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-HUB-001 | No importer recognises the file | a file whose name and first bytes no registered importer claims |
| PHOENIX-HUB-002 | Several importers recognise the file equally | two importers that claim a file with the same confidence (the message names them) |
| PHOENIX-HUB-003 | Importer registered twice | a second importer with the format name of one already registered |

## Manufacturing export (`eh3d-phoenix-export`, `src/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-EXPORT-001 | Pad rotation is not a right angle | a pad turned by an angle that is not a multiple of 90 degrees, flashed unrotated (a warning) |
| PHOENIX-EXPORT-002 | Zone written as its outline | a copper zone, written as a region of its outline because there is no pour yet (information) |
| PHOENIX-EXPORT-003 | Copper stack order not given | a copper layer written without a file function because the board does not give the order of its copper layers (information) |
| PHOENIX-EXPORT-004 | Arc left out | a track arc whose centre cannot be computed, not written (a warning) |
| PHOENIX-EXPORT-005 | File could not be written | a layer or drill file that the writer refuses (a number that does not fit, an aperture that is not defined) |
| PHOENIX-EXPORT-006 | Non-round pad drilled as a round hole | a drilled pad that is not round, written as a round hole (information) |
| PHOENIX-EXPORT-007 | Package could not be assembled | a package whose metadata or entries are refused |
| PHOENIX-EXPORT-008 | Silkscreen side not given | a silkscreen layer written without a file function because the layer does not say which side it is on (information) |
| PHOENIX-EXPORT-009 | Silkscreen characters without a glyph | characters of a silkscreen text that the font does not have, written as boxes (a warning) |
| PHOENIX-EXPORT-010 | Silkscreen item left out | a silkscreen item that cannot be drawn (a text whose strokes fall outside the coordinate range), not written (a warning) |
| PHOENIX-EXPORT-011 | Export profile is not valid | a profile file that is not JSON, has an unknown field, a value out of its range, a file name that is not a plain name or a column that does not exist |
| PHOENIX-EXPORT-012 | Export profile not found | a profile name that the profile file does not hold |
| PHOENIX-EXPORT-013 | Output could not be written | the output folder or one of its files could not be created or written |
| PHOENIX-EXPORT-014 | Written file differs from the model | the check that reads the files back found a count (pads, vias, holes, lines, regions) or the bounding box of the outline different from the board |
| PHOENIX-EXPORT-015 | Board has no components | a board without a placed component, whose placement and bill of materials files hold only their header (information) |
| PHOENIX-EXPORT-016 | Slot pad not on a right angle | a drilled oblong pad turned by an angle that is not a multiple of 90 degrees, written as a round hole when slots are asked for (a warning) |
| PHOENIX-EXPORT-017 | Layer side not given | a solder mask or paste layer written without a file function because the layer does not say which side it is on (information) |
| PHOENIX-EXPORT-018 | Written file could not be read back | the check could not read a Gerber or Excellon file it has just made (an error) |

## Schematic and its link to the board (`eh3d-phoenix-schematic`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SCH-001 | Reference to a part that does not exist | a part that names a symbol, or a net that lists a pin, that the schematic does not have (in `src/model.rs`) |
| PHOENIX-SCH-002 | Duplicate reference designator | two parts with the same reference designator |
| PHOENIX-SCH-003 | Pin on more than one net | a pin listed on two nets, or twice |
| PHOENIX-SCH-004 | Wire is not a path of two or more points | a wire with fewer than two points or two equal points in a row (in `src/draw.rs`) |
| PHOENIX-SCH-005 | Wires join pins that are on different nets | pins that wires join and the net list puts on different nets, or some on a net and some on none (an error) |
| PHOENIX-SCH-006 | Net is not joined by wires | a net whose placed pins are not all joined by the wires of the drawing (information: a connection may be given by the net list alone) |
| PHOENIX-SCH-007 | Pin position is not given | a pin of a placed part whose symbol does not give the position of the pin, so its connections cannot be judged (information) |
| PHOENIX-SCH-008 | Point in the middle of a wire looks connected | a wire point or a pin that lies in the middle of a piece of another wire without being a point of it, so it is not connected (a warning) |
| PHOENIX-SCH-009 | Placement is not valid | a placed part whose rotation is not a multiple of 90 degrees or whose pins fall outside the coordinate range |
| PHOENIX-LINK-001 | Part has no component on the board | a part of the schematic that no component of the board was made from (in `src/link.rs`) |
| PHOENIX-LINK-002 | Component names a part that does not exist | a component whose link names a part that the schematic does not have |
| PHOENIX-LINK-003 | Reference designators differ | a part and its component with different designators |
| PHOENIX-LINK-004 | Values differ | a part and its component with different values (a warning: it may be intended) |
| PHOENIX-LINK-005 | Footprints differ | a part that chose a footprint and a component that uses another |
| PHOENIX-LINK-006 | Nets differ between schematic and board | the pins of one schematic net on several nets of the board or without one, or one net of the board that joins pins of two schematic nets |
| PHOENIX-LINK-007 | Part used by two components | two components made from one part |
| PHOENIX-LINK-008 | Pin has no pad | a pin whose number is not the number of a pad of the footprint |
| PHOENIX-LINK-009 | Wires join pads that are on different nets of the board | pins that wires join whose pads are on different nets of the board, or on none (an error, in `src/link.rs`) |
| PHOENIX-LINK-010 | Net of the board is not joined by wires | pads of one net of the board that the drawing leaves in several pieces (a warning) |

## Rule engine and connectivity (`eh3d-phoenix-rules`, `src/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-CHECK-001 | Track narrower than the minimum | a track whose width is below `min_track_width_nm`; the finding carries the measured width and the limit |
| PHOENIX-CHECK-002 | Via diameter smaller than the minimum | a via whose pad is below `min_via_diameter_nm` |
| PHOENIX-CHECK-003 | Drill smaller than the minimum | the drill of a via or of a pad below `min_drill_nm` |
| PHOENIX-CHECK-004 | Annular ring thinner than the minimum | the ring of copper around the drill of a via or a pad below `min_annular_ring_nm` |
| PHOENIX-CHECK-005 | Copper clearance violated | tracks and vias of different nets on one layer closer than `min_clearance_nm` edge to edge (measured value and limit given; an arc counts by its end points), and the pad findings of the board crate (limit only) |
| PHOENIX-CHECK-006 | Copper too close to the board edge | a track or via closer to an edge of the outline than `min_edge_clearance_nm`, and the pad findings of the board crate |
| PHOENIX-CHECK-007 | Drill holes too close to each other | the walls of two drill holes (vias and through-hole pads) closer than `min_drill_spacing_nm` |
| PHOENIX-CHECK-008 | Solder mask bridge too thin | two pad openings with less mask between them than `min_mask_bridge_nm` (from the board crate, limit only) |
| PHOENIX-CHECK-009 | Zone clearance smaller than the minimum | a zone whose own clearance is below `min_clearance_nm` (from the board crate, limit only) |
| PHOENIX-CHECK-010 | Zones of different nets overlap | two zones of different nets on one copper layer whose outlines cross or contain each other (from the board crate) |
| PHOENIX-CHECK-011 | Silkscreen printed over a pad | the rule `silkscreen_over_pads` (from the board crate) |
| PHOENIX-CHECK-012 | Silkscreen printed over a solder mask opening | the rule `silkscreen_over_mask_openings` (from the board crate) |
| PHOENIX-CHECK-013 | Check could not be judged | a finding in the inconclusive list: an arc, a pad shape too coarse to tell, numbers too large for the exact test, a board without an outline for the edge rule (information, never a failure) |
| PHOENIX-CHECK-014 | Track narrower than the minimum of its net class | a track of a net that belongs to a class of a rule file of the second format whose width is below the `min_track_width_nm` of that class |
| PHOENIX-CHECK-015 | Via smaller than the minimum of its net class | a via of a net of a class whose diameter is below `min_via_diameter_nm` or whose drill is below `min_via_drill_nm` of the class |
| PHOENIX-RULEFILE-001 | Rule file cannot be read | a rule file that is too large, is not JSON of the format `phoenix-rules/1`, has a field it does not know, or an EAGLE board file that cannot be read |
| PHOENIX-RULEFILE-002 | Rule value is not valid | a length that is negative or outside the design range |
| PHOENIX-RULEFILE-003 | Design rule parameter not used | a parameter of the `<designrules>` of an EAGLE board that has no rule here, or clearances and rings that differ and were folded into the largest value (information) |
| PHOENIX-RULEFILE-004 | Board file holds no design rules | an EAGLE board file without `<designrules>`: the limits have to come from a rule file |
| PHOENIX-RULEFILE-005 | Net class is not valid | a class without a name, two classes with one name, or a net in two classes |
| PHOENIX-CONN-001 | Schematic net missing from the board | a schematic net of two or more pins none of whose pins is on a net of the board |
| PHOENIX-CONN-002 | Pin not connected on the board | a pin of a schematic net whose pad does not exist on the board, is on no net, or is on another net than the other pins |
| PHOENIX-CONN-003 | Board net joins pins of several schematic nets | one net of the board that holds the pins of two or more nets of the schematic |
| PHOENIX-CONN-004 | Copper of different nets touches | pads, tracks and vias of different nets that touch on a layer within the joining tolerance, or a group of joined copper that holds two nets |
| PHOENIX-CONN-005 | Net has unrouted connections | a net whose copper is in more than one group; listed as inconclusive when an arc or a pad shape may join the groups |
| PHOENIX-CONN-006 | Net of the same name holds different pins | a net of the schematic and a net of the board with one name whose pins differ (a warning) |
| PHOENIX-CONN-007 | Net name differs for the same pins | the pins of a schematic net are exactly those of a board net with another name (a warning) |
| PHOENIX-CONN-008 | Connection could not be judged | a contact that the shape of a pad or the size of the numbers does not decide, or a track with an arc (information) |
| PHOENIX-CONN-009 | Pad on a board net but its pin is on no schematic net | a pad of a component whose part is in the schematic but whose pin is on no net there (a warning) |

## Stack-up and transmission lines (`eh3d-phoenix-stackup`, `src/error.rs`)

The titles of these codes also exist in the eight languages of the suite (`title` in `src/error.rs`); the detail of a message is in English. The codes 001 to 011 are errors, 012 to 016 are warnings that go with a result.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-STACK-001 | A number is not valid | a value that is not finite or is outside the range accepted (a frequency, a width, a copper thickness, a target impedance) |
| PHOENIX-STACK-002 | The stack-up is not valid | no layers or more than 64, a repeated or empty name, two copper layers next to each other, solder mask in the middle, a thickness or a roughness out of range, no copper |
| PHOENIX-STACK-003 | Unknown material | a layer that names a material that was not given |
| PHOENIX-STACK-004 | Frequency outside the published points | a Dk or Df asked for below the lowest or above the highest published frequency (never extrapolated) |
| PHOENIX-STACK-005 | Material data is not valid | a material with no name, no points, unsorted or repeated frequencies, or a Dk or Df out of range |
| PHOENIX-STACK-006 | The line is not valid | a line on a layer that does not exist or is not copper, a kind that does not fit the layers around it, a missing or unwanted gap, or a width and a target together or neither |
| PHOENIX-STACK-007 | No width reaches the target impedance | a target that the widest and the narrowest allowed track do not enclose |
| PHOENIX-STACK-008 | Synthesis did not converge | a search that ran out of steps or of nanometres before reaching the tolerance |
| PHOENIX-STACK-009 | The settings are not valid | a line of the settings text that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-STACK-010 | The job file is not valid | a job file that is too large, is not JSON of the expected shape, has an unknown field, no lines or a repeated line name |
| PHOENIX-STACK-011 | Geometry outside the domain of the formula | a slot closed by the thickness of the copper, a track as thick as the gap between planes, or a result that is not a positive number |
| PHOENIX-STACK-012 | Outside the validity range of the formula | a value outside the range for which the source publishes the formula (a warning; the number is still given) |
| PHOENIX-STACK-013 | Conductor thin compared with the skin depth | copper thinner than the number of skin depths set (a warning) |
| PHOENIX-STACK-014 | Quasi-static limit exceeded | a substrate height above the fraction of the guided wavelength set (a warning) |
| PHOENIX-STACK-015 | Solder mask over the line is not modelled | solder mask over the open side of a microstrip or coplanar line (a warning) |
| PHOENIX-STACK-016 | Approximate model | an offset stripline, a coupled pair, or a coplanar line whose plane under the substrate is left out (a warning) |

## RF tools (`eh3d-phoenix-rf`, `src/error.rs`)

The titles of these codes also exist in the eight languages of the suite (`title` in `src/error.rs`); the detail of a message is in English. Every code is an error.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-RF-001 | A number is not valid | a value that is not finite or is outside what the calculation accepts, a complex number written wrongly, a port or a reference impedance that is not above zero |
| PHOENIX-RF-002 | The Touchstone file has a syntax error | a line of a Touchstone file that cannot be read (an unknown option, a value that is not a number, a record with too many values, the keywords of version 2); the message names the line |
| PHOENIX-RF-003 | The Touchstone data is not valid | values that are read but cannot be used (a frequency that does not increase, a negative magnitude, an incomplete record, a file with no data, noise data in the wrong place); the message names the line |
| PHOENIX-RF-004 | The calculation is singular | a division by zero, a matrix with no inverse, a conversion that does not exist for the network (a series impedance has no Z matrix) or a result that is not a number |
| PHOENIX-RF-005 | The network is not valid | a matrix of the wrong size or with an entry that is not finite, a pair of lists of different length, a port that the file does not have |
| PHOENIX-RF-006 | No network gives the match | impedances for which no L network, stub or quarter-wave section of the kind asked for exists |
| PHOENIX-RF-007 | The filter is not valid | an order below 1 or above the Bessel table, a ripple given to a filter that has none or missing for a Chebyshev one, band edges out of order, a specification whose stop band does not exceed the pass band |
| PHOENIX-RF-008 | The settings are not valid | a line of the settings text that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-RF-009 | A value is outside the configured limit | more ports, points, bytes, filter order, ripple, impedance, frequency or sweep points than the settings allow |
| PHOENIX-RF-010 | Frequency outside the data of the file | a frequency below the first or above the last point of a Touchstone file (nothing is extrapolated) |

## Viewer (`eh3d-phoenix-viewer`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English. Every code is an error.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-VIEW-001 | File cannot be read | a file that does not exist, cannot be read or is larger than the limit of the settings |
| PHOENIX-VIEW-002 | File is not a board or a schematic that the viewer reads | a file that is not UTF-8 text, or whose content is neither an EAGLE board or schematic nor the JSON of a board or of a schematic of the model |
| PHOENIX-VIEW-003 | EAGLE file cannot be read | an EAGLE file named `.brd` or `.sch` that the reader refuses (not XML, no board or no schematic in the drawing) |
| PHOENIX-VIEW-004 | Nothing to draw | a board or a schematic that gives no primitive |
| PHOENIX-VIEW-005 | Viewer settings are not valid | a line of the settings text that is not `key = value`, an unknown key or colour, or a value out of range |
| PHOENIX-VIEW-006 | Too many drawing primitives | a document that would give more primitives than the settings allow |
| PHOENIX-VIEW-007 | Benchmark option is not valid | a benchmark board below the smallest size, or more scans than picks |
| PHOENIX-VIEW-008 | The window is not built in | `phoenix view` asked to open the window by a program built without the `gui` feature |
| PHOENIX-VIEW-009 | The window could not be opened | the graphics or windowing system refused to open the window |

## KiCad readers (`eh3d-phoenix-kicad`, `src/error.rs`)

The titles of these codes also exist in the eight languages of the suite (`title` in `src/error.rs`); the detail of a message is in English. A finding that comes from a place in a file says the line, and the column when it has one.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-KICAD-001 | The KiCad file has a syntax error | a parenthesis that closes nothing, text outside the root list, a second root list or a text that holds no list; the message names the line and the column |
| PHOENIX-KICAD-002 | The KiCad file is larger or deeper than the configured limit | a file, a nesting of lists, a number of nodes, an atom or a quoted text, a number of sheets or a nesting of sheets beyond the settings; a project file larger than `max_project_bytes` |
| PHOENIX-KICAD-003 | The file is not a KiCad file of the expected kind | a file whose root list is not the one the reader reads (a board given to the schematic reader), or text that starts with something else |
| PHOENIX-KICAD-004 | The KiCad file ends before it is complete | a list or a quoted text that is still open at the end of the file (a truncated file); the message says how many lists were open |
| PHOENIX-KICAD-005 | A value in the KiCad file is not valid | a number that is not a length or an angle, a position outside the range of the model, a pad, track, via, zone or wire with a missing or unusable part; the message names the line (a warning) |
| PHOENIX-KICAD-006 | Part of the KiCad file not read into the model | elements the readers do not interpret, counted by name (information) |
| PHOENIX-KICAD-007 | Item read in a simpler form | a pad of a shape the model does not have, an oval drill, a rotation that is not a right angle, a part drawn as several units, an arc through three points on a line, a wire that is neither horizontal nor vertical, a zone without a clearance (information) |
| PHOENIX-KICAD-008 | Reference to something the file does not have | a net number that is not in the net table, a placed symbol that is not in `lib_symbols`, a symbol that extends one that is not there, a sheet file that cannot be read, that leaves the folder or that contains itself (a warning) |
| PHOENIX-KICAD-009 | File version outside the tested range | a `version` older than `min_tested_version`, newer than `max_tested_version` or missing; the file is read anyway (a warning) |
| PHOENIX-KICAD-010 | The KiCad reader settings are not valid | a line of the settings text that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-KICAD-011 | The KiCad project file is not valid | a `.kicad_pro` file that is not JSON, is not an object or nests deeper than the JSON reader allows; the message says where the JSON stops |
| PHOENIX-KICAD-012 | The file is not valid UTF-8 text | bytes that are not UTF-8; they are replaced and the file is read (a warning) |
| PHOENIX-KICAD-013 | Nets derived from the drawing | a schematic: the nets are computed from wires, junctions, labels and power symbols because the file stores no net list (information) |
| PHOENIX-KICAD-014 | Fidelity level of the import | the last finding of every import by the hub: `complete`, `partial` or `unusable` (information) |

## Library sanitizer (`eh3d-phoenix-sanitizer`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English. A note about a library file carries the path of the file relative to the folder that was analysed.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SAN-001 | Library folder cannot be read | a folder that does not exist or cannot be opened, more files than `max_files` or folders deeper than `max_depth`; a sub-folder that cannot be opened is noted and the walk goes on |
| PHOENIX-SAN-002 | Library file cannot be read | a `.lbr` file that cannot be opened or read; the file is left out and the run goes on |
| PHOENIX-SAN-003 | Library file is larger than the limit | a `.lbr` file larger than `max_file_bytes`; it is left out |
| PHOENIX-SAN-004 | File is not an EAGLE library that the reader accepts | a `.lbr` file that is not XML, is an EAGLE binary file, belongs to another program, is a board or a schematic, or has no library in its drawing; the detail holds the code of the EAGLE reader or the kind found |
| PHOENIX-SAN-005 | Items of a library were skipped | footprints, symbols or drawing items with a missing or unusable coordinate, or without a name; the detail says how many |
| PHOENIX-SAN-006 | Sanitizer settings are not valid | a line of the settings text that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-SAN-007 | Comparison limit reached | more pairs than `max_pairs` (the later pairs are not compared) or items with more than `max_anchors` pads or pins (they are compared by hash only) |
| PHOENIX-SAN-008 | Output file cannot be written | a report or plan file that could not be written after the bounded tries |
| PHOENIX-SAN-009 | Duplicates name their pads or pins differently | a group whose members have the same shape and number their pads or pins in another way: replacing one by the other would rewire the board |
| PHOENIX-SAN-010 | Group joined by a weak match | a group with a member whose score against the item to keep is below the threshold and which is in the group through other members |
| PHOENIX-SAN-011 | Name used twice in a library | two footprints, symbols or device sets with one name in one library; the later one is left out |
| PHOENIX-SAN-012 | Command option is not valid | no folder or two folders, an option that does not exist or lacks its value, a threshold that is not a whole number from 0 to 100, a language that is not one of the eight |
| PHOENIX-SAN-013 | The reader reported problems in a library | the EAGLE reader found a dangling reference or another problem in a library that is read all the same; the detail lists the codes of the reader |
