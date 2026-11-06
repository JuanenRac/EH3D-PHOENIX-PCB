# EH3D-PHOENIX-PCB: diagnostic codes

Codes have the form `PHOENIX-<AREA>-<NNN>`. A code is stable: it never changes its meaning and is never reused. Programs show the text through the key `diag.<code>`; the codes themselves are never translated. The object identity (UUID v7) is the one of the shared container, with its own codes `EH3D-ID-001` and `EH3D-ID-002`. The container codes `EH3D-PKG-001` to `EH3D-PKG-014` belong to the shared container and are listed in the registry of the documentation repository.

Each table repeats the `REGISTRY` of a crate (`eh3d-phoenix-model` in `src/diagnostic.rs`, `eh3d-phoenix-project` and `eh3d-phoenix-eagle` in `src/codes.rs`, `eh3d-phoenix-hub` in `src/lib.rs`, `eh3d-phoenix-export` in `src/codes.rs` and `src/fabcodes.rs`, `eh3d-phoenix-schematic` in `src/model.rs`, `src/draw.rs`, `src/edit.rs`, `src/link.rs`, `src/hier.rs` and `src/bus.rs`, `eh3d-phoenix-board` in `src/validate.rs`, `src/connectivity.rs` and `src/rules.rs`, `eh3d-phoenix-rules` in `src/codes.rs` and `src/advanced/codes.rs`, `eh3d-phoenix-stackup` and `eh3d-phoenix-rf` in `src/error.rs`, `eh3d-phoenix-viewer`, `eh3d-phoenix-sanitizer`, `eh3d-phoenix-erc`, `eh3d-phoenix-sync`, `eh3d-phoenix-schedit`, `eh3d-phoenix-boardedit` (also in `src/pair_codes.rs`, `src/shove_codes.rs` and `src/zone_codes.rs`), `eh3d-phoenix-libedit`, `eh3d-phoenix-zones`, `eh3d-phoenix-autoroute`, `eh3d-phoenix-lengths`, `eh3d-phoenix-gerber`, `eh3d-phoenix-panel`, `eh3d-phoenix-bom`, `eh3d-phoenix-assembly3d`, `eh3d-phoenix-si`, `eh3d-phoenix-eagle-deep` and `eh3d-phoenix-oracle` in `src/codes.rs`, `eh3d-phoenix-corpus` in `src/suite/codes.rs`); a test of each crate checks that every code of its registry is in the table with the same title.

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
| PHOENIX-BOARD-006 | Outline is not a closed polygon | an outline with fewer than three points, repeated points or crossing edges (the crossing test is made on a polygon of at most `max_polygon_points` points, see PHOENIX-BOARD-017) |
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
| PHOENIX-BOARD-017 | Polygon has more points than the limit, its edges were not tested for crossing | a warning for an outline, a cutout or a zone with more points than `max_polygon_points` of `config/board.cfg` (`BoardLimits`, `Board::validate_with`; 10000 by default): the test of repeated points is made, the test of crossing edges, which takes time with the square of the number of points, is not; `phoenix check` lists each such polygon as information, which never changes its exit code (`board_notes`) |

The same titles in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese), as the program prints them:

| Code | en | es | de | fr | it | ja | zh | pt |
|---|---|---|---|---|---|---|---|---|
| PHOENIX-BOARD-017 | Polygon has more points than the limit, its edges were not tested for crossing | El polígono tiene más puntos que el límite, no se comprobó si sus aristas se cruzan | Das Polygon hat mehr Punkte als das Limit, seine Kanten wurden nicht auf Überschneidung geprüft | Le polygone a plus de points que la limite, ses arêtes n'ont pas été testées pour les croisements | Il poligono ha più punti del limite, i suoi lati non sono stati controllati per gli incroci | ポリゴンの点数が上限を超えているため、辺の交差は検査されませんでした | 多边形的点数超过上限，未检查其边是否相交 | O polígono tem mais pontos do que o limite, as suas arestas não foram verificadas quanto a cruzamentos |

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
| PHOENIX-EAGLE-021 | Item left out of the EAGLE file | an item with no place in the file: a component without a footprint, a track, zone, via or silkscreen item on a layer that has no EAGLE number, a silkscreen item that cannot be drawn, a seventeenth copper layer, a cutout of a zone with fewer than three points or on an inner layer, buses and bus entries of a schematic (a warning) |
| PHOENIX-EAGLE-022 | Part of the EAGLE file not read into the board | elements of a board file that the board reader does not interpret, counted by name, and the wires and labels of a bus, the names of a bus that are not vectors and a restrict area that lies in no pour (information); also what went over a limit of `config/eagle_board.cfg` (`EagleLimits`), counted under the name `outline ring (over max_outline_rings)`, `polygon (over max_restrict_checks)` or `bus entry (over max_bus_checks)` |
| PHOENIX-EAGLE-023 | EAGLE file cannot be read into a board | text that is not XML, whose root is not `eagle` or that has no board in its drawing (an error) |

The same titles in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese), as the program prints them:

| Code | en | es | de | fr | it | ja | zh | pt |
|---|---|---|---|---|---|---|---|---|
| PHOENIX-EAGLE-020 | Item written in a simpler form | Elemento escrito de forma más simple | Element in einfacherer Form geschrieben | Élément écrit sous une forme plus simple | Elemento scritto in forma più semplice | 項目はより単純な形で書き出されました | 项目以更简单的形式写入 | Elemento escrito de forma mais simples |
| PHOENIX-EAGLE-021 | Item left out of the EAGLE file | Elemento omitido del fichero EAGLE | Element nicht in die EAGLE-Datei geschrieben | Élément omis du fichier EAGLE | Elemento omesso dal file EAGLE | 項目は EAGLE ファイルに書き出されませんでした | 项目未写入 EAGLE 文件 | Elemento omitido do ficheiro EAGLE |
| PHOENIX-EAGLE-022 | Part of the EAGLE file not read into the board | Parte del fichero EAGLE no leída en la placa | Teil der EAGLE-Datei nicht in die Platine eingelesen | Partie du fichier EAGLE non lue dans la carte | Parte del file EAGLE non letta nella scheda | EAGLE ファイルの一部は基板に読み込まれませんでした | EAGLE 文件的一部分未读入电路板 | Parte do ficheiro EAGLE não lida para a placa |
| PHOENIX-EAGLE-023 | EAGLE file cannot be read into a board | El fichero EAGLE no se puede leer como placa | Die EAGLE-Datei kann nicht als Platine gelesen werden | Le fichier EAGLE ne peut pas être lu comme une carte | Il file EAGLE non può essere letto come scheda | EAGLE ファイルを基板として読み込めません | 无法将 EAGLE 文件读取为电路板 | O ficheiro EAGLE não pode ser lido como placa |

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
| PHOENIX-EXPORT-019 | Attribute text written with escapes | a net name, a reference designator, a pin number or a project name with a comma, an asterisk, a percent sign, a backslash or a character outside printable ASCII, written as `\uXXXX` in a Gerber X2 attribute (information; the strict mode refuses it instead) |
| PHOENIX-EXPORT-020 | Attribute text not allowed | the same kind of text in the strict mode of `gerber.x2`, or a character beyond the first plane of Unicode, which has no escape; the export stops (an error) |
| PHOENIX-EXPORT-021 | Small copper island left out | zones of one net and one layer that share a stretch of border and have less area than `gerber.zone_island_min_area_nm2`, not written with their clearing (information) |
| PHOENIX-EXPORT-022 | Thermal relief not made for a turned pad | a rectangular or oblong pad of the net of a pour that is turned by an angle that is not a right angle, left solid when `gerber.zone_thermal_non_round` asks for a relief (information) |

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
| PHOENIX-SCH-010 | Edit refused | an editing operation that was refused and changed nothing: a part, sheet, symbol or item that does not exist, a reference that is already used, a name that is empty or too long, a wire that is not made of horizontal and vertical pieces, a bus that is not of the form `D[0..7]`, a sheet symbol whose name is used twice on its sheet or that would show its own sheet, a sheet removed while a symbol still shows it, or a coordinate out of range (in `src/edit.rs` and `src/hier_edit.rs`) |
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

## Advanced design rule checks (`eh3d-phoenix-rules`, `src/advanced/codes.rs`)

The checks of `phoenix check --advanced`, with the limits and the levels of `config/drc_advanced.cfg`. The level of a check (off, info, warning or error) is the severity of its findings.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-DRC-001 | Annular ring thinner than the advanced minimum | An annular ring (via pad or drilled pad against its drill) thinner than annular_ring_min_nm |
| PHOENIX-DRC-002 | Copper too close to the board edge | A track or a via closer to the outline than copper_edge_min_nm |
| PHOENIX-DRC-003 | Pad too close to the board edge | A pad closer to the outline than pad_edge_min_nm |
| PHOENIX-DRC-004 | Drill smaller than the minimum of its net class | A via or a drilled pad whose drill is below the minimum of the class of its net (or the default) |
| PHOENIX-DRC-005 | Silkscreen too close to a drill hole | Silkscreen closer to the drill of a through hole than silk_hole_min_nm |
| PHOENIX-DRC-006 | Neck: narrow track joined to wider copper | A track narrower than neck_min_width_nm whose end joins wider copper of the same net |
| PHOENIX-DRC-007 | Acid trap: acute angle between tracks of one net | Two pieces of tracks of one net that meet at an angle below acid_trap_min_angle_deg |
| PHOENIX-DRC-008 | Spacing below the minimum for the voltage between nets | Copper of two nets with a voltage between them closer than the table of IPC-2221 asks (figures not verified against the standard) |
| PHOENIX-DRC-009 | Copper island joined to no pad | Tracks, vias or zones of a net that are joined together but to no pad |
| PHOENIX-DRC-010 | Dead-end track (stub) | A track with one end that touches nothing of its net while the other end is joined |
| PHOENIX-DRC-011 | Duplicate or stacked vias | Two vias that share a layer and are at most via_duplicate_max_distance_nm apart |
| PHOENIX-DRC-012 | Pad on no net | A pad of a placed component that is assigned to no net |
| PHOENIX-DRC-013 | Part outside the board outline | A component with a pad whose centre is outside the outline of the board |
| PHOENIX-DRC-014 | Advanced check could not be judged | An item the advanced checks cannot judge: a shape too coarse to tell, numbers too large, a board without an outline (information, never a failure) |
| PHOENIX-DRC-015 | Advanced check settings are not valid | A settings file that is too large, has a key it does not know or written twice, or a value out of range |
| PHOENIX-DFM-001 | More copper layers than the profile offers | `phoenix check --dfm`: the board has more copper layers than `max_copper_layers` of the capability profile |
| PHOENIX-DFM-002 | Board smaller than the profile accepts | `phoenix check --dfm`: the short or the long side of the outline is below `min_board_short_nm` or `min_board_long_nm` |
| PHOENIX-DFM-003 | Board larger than the profile accepts | `phoenix check --dfm`: the short or the long side of the outline is above `max_board_short_nm` or `max_board_long_nm` |
| PHOENIX-DFM-004 | Drill larger than the profile accepts | `phoenix check --dfm`: the largest drill of the vias and the drilled pads is above `max_drill_nm` |
| PHOENIX-DFM-005 | More holes than the profile accepts | `phoenix check --dfm`: the vias and the drilled pads are more than `max_holes` |
| PHOENIX-DFM-006 | The capability profile is not verified against the maker | `phoenix check --dfm`: information, given while the profile says `verified = false`; the profile that comes with the program is an example with invented figures |

Titles of the same codes in the other seven languages (Portuguese is the Portuguese of Portugal):

| Code | es | de | fr | it | ja | zh | pt |
|---|---|---|---|---|---|---|---|
| PHOENIX-DRC-001 | Anillo anular más fino que el mínimo avanzado | Restring dünner als das erweiterte Minimum | Anneau de cuivre plus fin que le minimum avancé | Anello anulare più sottile del minimo avanzato | アニュラリングが詳細チェックの最小値より薄い | 环形焊环比高级检查的最小值更薄 | Anel anular mais fino do que o mínimo avançado |
| PHOENIX-DRC-002 | Cobre demasiado cerca del borde de la placa | Kupfer zu nah am Platinenrand | Cuivre trop près du bord de la carte | Rame troppo vicino al bordo della scheda | 銅箔が基板の端に近すぎる | 铜箔离板边太近 | Cobre demasiado perto do limite da placa |
| PHOENIX-DRC-003 | Pad demasiado cerca del borde de la placa | Pad zu nah am Platinenrand | Pastille trop près du bord de la carte | Pad troppo vicino al bordo della scheda | パッドが基板の端に近すぎる | 焊盘离板边太近 | Pad demasiado perto do limite da placa |
| PHOENIX-DRC-004 | Taladro menor que el mínimo de su clase de red | Bohrung kleiner als das Minimum ihrer Netzklasse | Perçage plus petit que le minimum de sa classe de réseau | Foro più piccolo del minimo della sua classe di rete | ドリル径がネットクラスの最小値より小さい | 钻孔小于其网络类别的最小值 | Furo mais pequeno do que o mínimo da sua classe de rede |
| PHOENIX-DRC-005 | Serigrafía demasiado cerca de un taladro | Bestückungsdruck zu nah an einer Bohrung | Sérigraphie trop près d'un perçage | Serigrafia troppo vicina a un foro | シルクが穴に近すぎる | 丝印离钻孔太近 | Serigrafia demasiado perto de um furo |
| PHOENIX-DRC-006 | Cuello: pista estrecha unida a cobre más ancho | Engstelle: schmale Leiterbahn an breiterem Kupfer | Goulot : piste étroite reliée à du cuivre plus large | Strozzatura: pista stretta unita a rame più largo | ネック：細い配線が太い銅箔につながっている | 瓶颈：窄走线连接到更宽的铜箔 | Estrangulamento: pista estreita ligada a cobre mais largo |
| PHOENIX-DRC-007 | Trampa de ácido: ángulo agudo entre pistas de una red | Säurefalle: spitzer Winkel zwischen Leiterbahnen eines Netzes | Piège à acide : angle aigu entre pistes d'un réseau | Trappola d'acido: angolo acuto tra piste di una rete | アシッドトラップ：同一ネットの配線間の鋭角 | 酸液陷阱：同一网络走线间的锐角 | Armadilha de ácido: ângulo agudo entre pistas de uma rede |
| PHOENIX-DRC-008 | Separación inferior al mínimo para la tensión entre redes | Abstand unter dem Minimum für die Spannung zwischen Netzen | Distance inférieure au minimum pour la tension entre réseaux | Distanza inferiore al minimo per la tensione tra reti | ネット間の電圧に対する間隔が最小値未満 | 间距小于网络间电压要求的最小值 | Distância inferior ao mínimo para a tensão entre redes |
| PHOENIX-DRC-009 | Isla de cobre sin ningún pad | Kupferinsel ohne Pad | Îlot de cuivre sans aucune pastille | Isola di rame senza alcun pad | パッドにつながらない銅箔の島 | 未连接任何焊盘的铜岛 | Ilha de cobre sem nenhum pad |
| PHOENIX-DRC-010 | Pista muerta (sin pad ni vía en un extremo) | Blinde Leiterbahn (Stummel) | Piste borgne (tronçon sans issue) | Pista cieca (moncone) | 行き止まりの配線（スタブ） | 断头走线（残桩） | Pista sem saída (cotoco) |
| PHOENIX-DRC-011 | Vías duplicadas o apiladas | Doppelte oder gestapelte Durchkontaktierungen | Vias dupliqués ou empilés | Via duplicate o impilate | 重複または積み重なったビア | 重复或堆叠的过孔 | Vias duplicadas ou empilhadas |
| PHOENIX-DRC-012 | Pad sin ninguna red | Pad ohne Netz | Pastille sans réseau | Pad senza alcuna rete | どのネットにも属さないパッド | 不属于任何网络的焊盘 | Pad sem nenhuma rede |
| PHOENIX-DRC-013 | Pieza fuera del contorno de la placa | Bauteil außerhalb des Platinenumrisses | Pièce hors du contour de la carte | Componente fuori dal contorno della scheda | 部品が基板外形の外にある | 元件超出板框 | Peça fora do contorno da placa |
| PHOENIX-DRC-014 | Comprobación avanzada sin poder juzgar | Erweiterte Prüfung nicht entscheidbar | Contrôle avancé non concluant | Controllo avanzato non conclusivo | 詳細チェックを判定できない | 高级检查无法判定 | Verificação avançada sem conclusão |
| PHOENIX-DRC-015 | Ajustes de las comprobaciones avanzadas no válidos | Einstellungen der erweiterten Prüfungen ungültig | Réglages des contrôles avancés non valides | Impostazioni dei controlli avanzati non valide | 詳細チェックの設定が正しくない | 高级检查的设置无效 | Definições das verificações avançadas inválidas |

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

## Heating of tracks and vias (`eh3d-phoenix-stackup`, `src/heating.rs`)

The titles of these codes also exist in the eight languages of the suite (`heat_title` in `src/heating.rs`); the detail of a message is in English. The codes 004 to 008 and 012 go with a result and do not stop it.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-HEAT-001 | Heating settings are not valid | a line of `config/heating.cfg` that is not `key = value`, an unknown key, a number that cannot be read or a value out of range |
| PHOENIX-HEAT-002 | A number given to the heating calculation is not valid | a width, a thickness or a drill that is not above zero, or a current that is not between 0 and 100000 ampere |
| PHOENIX-HEAT-003 | The file of currents is not valid | a line that is not `net = amps`, an empty name, a current out of range, a net given twice, more nets than `max_nets` or a file with no net |
| PHOENIX-HEAT-004 | The temperature rise of a track is above the limit | warning: a track or a via whose rise, by the estimate chosen with `limit_basis`, is above `max_rise_k`; the exit code of `phoenix thermal` is 1 |
| PHOENIX-HEAT-005 | The convection model has no number for the track | information: the heating grows faster than the cooling, or the track would pass `max_temperature_c` |
| PHOENIX-HEAT-006 | The chart of IPC-2221 does not reach this track | information: the rise by the fit is outside `ipc_min_rise_k` to `ipc_max_rise_k` or the current is above `ipc_max_current_a` |
| PHOENIX-HEAT-007 | A net of the file of currents is not on the board | warning: a net with a current that the board does not have |
| PHOENIX-HEAT-008 | The layer is not in the stack-up: the defaults of the settings are used | warning: the copper thickness and the placement come from `default_copper_um` and `default_placement` |
| PHOENIX-HEAT-009 | A command option is not valid | an option that `phoenix thermal` does not know, an option without its value, a language that is not one of the eight, a missing `--currents` or the wrong number of boards |
| PHOENIX-HEAT-010 | The output file exists or cannot be written | `--out` names a file that exists or cannot be created |
| PHOENIX-HEAT-011 | An input file cannot be read | a board, a stack-up, a file of currents or a settings file that is missing, too large or not of the expected shape |
| PHOENIX-HEAT-012 | IPC-2152, with its modifiers for the plane and the board, is not applied | information in every report: only the fit of IPC-2221 and the convection model are used |

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
| PHOENIX-RF-011 | The network description has a syntax error | a line of a network description (`phoenix rf sparams --net`) that is not an element, a value that is not a number or a length without its frequency or permittivity |
| PHOENIX-RF-012 | The mask has a syntax error | a line of a mask that is not `parameter min start stop limit [limit]` (or `max`), a frequency or a limit that is not a number or a segment whose start is above its stop |
| PHOENIX-RF-013 | The series of standard values is not known | a series other than E12, E24 or E96 |

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
| PHOENIX-VIEW-010 | The edit could not be made | an edit that the board refuses, that the rules refuse in strict mode, or that needs something the board does not have (a net, two copper layers, a companion board) |
| PHOENIX-VIEW-011 | The file may not be written there | a save as onto the input file, or onto a file that exists and was not written by this window |
| PHOENIX-VIEW-012 | The file could not be written | a folder that does not exist, a disk that refuses, or a document that cannot be written in the format asked |
| PHOENIX-VIEW-013 | The rule file cannot be used | a rule file or a settings file of a check that cannot be read or that the check refuses |
| PHOENIX-VIEW-014 | The panel cannot be laid out | the boards or the settings of the panel window that the panel crate refuses (its own code follows in the message), or a board file that cannot be read |

## KiCad readers (`eh3d-phoenix-kicad`, `src/error.rs`)

The titles of these codes also exist in the eight languages of the suite (`title` in `src/error.rs`); the detail of a message is in English. A finding that comes from a place in a file says the line, and the column when it has one.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-KICAD-001 | The KiCad file has a syntax error | a parenthesis that closes nothing, text outside the root list, a second root list or a text that holds no list; the message names the line and the column |
| PHOENIX-KICAD-002 | The KiCad file is larger or deeper than the configured limit | a file, a nesting of lists, a number of nodes, an atom or a quoted text, a number of sheets or a nesting of sheets beyond the settings; a project file larger than `max_project_bytes` |
| PHOENIX-KICAD-003 | The file is not a KiCad file of the expected kind | a file whose root list is not the one the reader reads (a board given to the schematic reader), or text that starts with something else |
| PHOENIX-KICAD-004 | The KiCad file ends before it is complete | a list or a quoted text that is still open at the end of the file (a truncated file); the message says how many lists were open |
| PHOENIX-KICAD-005 | A value in the KiCad file is not valid | a number that is not a length or an angle, a position outside the range of the model, a pad, track, via, zone or wire with a missing or unusable part; the message names the line (a warning) |
| PHOENIX-KICAD-006 | Part of the KiCad file not read into the model | elements the readers do not interpret, counted by name (information); also what went over `max_keepout_checks` (rule areas that keep the pour out, not given to a zone) or `max_outline_rings` (rings of the board outline not tested as openings), counted under the names `keep-out assignment (over max_keepout_checks)` and `outline ring (over max_outline_rings)` |
| PHOENIX-KICAD-007 | Item read in a simpler form | a pad of a shape the model does not have, an oval drill, a rotation that is not a right angle, a part drawn as several units, an arc through three points on a line, a wire that is neither horizontal nor vertical, a zone without a clearance (information) |
| PHOENIX-KICAD-008 | Reference to something the file does not have | a net number that is not in the net table, a placed symbol that is not in `lib_symbols`, a symbol that extends one that is not there, a sheet file that cannot be read, that leaves the folder or that contains itself (a warning) |
| PHOENIX-KICAD-009 | File version outside the tested range | a `version` older than `min_tested_version`, newer than `max_tested_version` or missing; the file is read anyway (a warning) |
| PHOENIX-KICAD-010 | The KiCad reader settings are not valid | a line of the settings text that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-KICAD-011 | The KiCad project file is not valid | a `.kicad_pro` file that is not JSON, is not an object or nests deeper than the JSON reader allows; the message says where the JSON stops |
| PHOENIX-KICAD-012 | The file is not valid UTF-8 text | bytes that are not UTF-8; they are replaced and the file is read (a warning) |
| PHOENIX-KICAD-013 | Nets derived from the drawing | a schematic: the nets are computed from wires, junctions, labels and power symbols because the file stores no net list (information) |
| PHOENIX-KICAD-014 | Fidelity level of the import | the last finding of every import by the hub: `complete`, `partial` or `unusable` (information) |
| PHOENIX-KICAD-015 | A file of the KiCad project is missing | `phoenix kicad-import` of a `.kicad_pro` whose schematic or board of the same name is not next to it; the rest of the project is read (a warning) |
| PHOENIX-KICAD-016 | The output of the import is not allowed | `--out` of `phoenix kicad-import` that is not a `.json` file, is an input or already exists; nothing is read or written (an error, exit code 2) |
| PHOENIX-KICAD-017 | The schematic and the board list different parts | a project whose schematic and board were both read and whose references differ; the finding names the references on one side only (a warning) |

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
| PHOENIX-SAN-014 | The plan was not confirmed with its code | `phoenix sanitize --apply --confirm CODE` was given a code that is not the one of the plan; nothing is applied and nothing is written |
| PHOENIX-SAN-015 | The plan cannot be applied | a plan whose format is not `phoenix-sanitize-plan/1`, or `--group` names a group that the plan does not have |
| PHOENIX-SAN-016 | A step of the plan was left out | information in the report of an application: the step turns, mirrors or shifts the part, renames pads or pins, is a device set, is only close (without `--allow-near`), or its library or item is missing, named twice or not UTF-8 text; the reason is in the report |

## Library editor (`eh3d-phoenix-libedit`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English. Codes 001 to 014 are the findings of the check of a library (`phoenix libedit`, and the window in the library mode); codes 020 and above are errors of reading, editing, saving and generating.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-LIB-001 | Pin of a symbol without a name | a pin of a symbol whose name is empty (error); the pad it reaches is given by the connections of each device |
| PHOENIX-LIB-002 | Pins of a symbol with the same name | two pins of one symbol with one name (error) |
| PHOENIX-LIB-003 | Pads of a footprint with the same name | two pads, of either kind, with one name in one footprint (error) |
| PHOENIX-LIB-004 | Pad without a copper layer or a size | a surface-mount pad on a layer that is not in `copper_layers`, or with no width or height, and a through-hole pad without a drill (error) |
| PHOENIX-LIB-005 | Drill larger than the pad | a through-hole pad whose drill is larger than its diameter, the automatic one when it has none (error) |
| PHOENIX-LIB-006 | Silkscreen drawn over a pad | a line, an arc or a circle on a layer of `silk_layers` whose distance to the copper of a pad, less half its width, is below `silk_clearance_nm` (warning) |
| PHOENIX-LIB-007 | Pin of a device without a pad | a device with a footprint in which a pin of the symbol of a gate has no connection (warning) |
| PHOENIX-LIB-008 | Connection to a gate, pin or pad that does not exist | a connection of a device that names a gate, a pin of the symbol of the gate or a pad of the footprint that is not there (error) |
| PHOENIX-LIB-009 | Device uses a footprint that the library does not have | a device whose footprint is not in the library (error) |
| PHOENIX-LIB-010 | Gate uses a symbol that the library does not have | a gate whose symbol is not in the library (error) |
| PHOENIX-LIB-011 | Footprint without an origin inside its pads | the origin is further than `origin_tolerance_nm` from the box of the centres of the pads, or the footprint has no pads (warning) |
| PHOENIX-LIB-012 | Footprint without a name or value text | no text `>NAME` or no text `>VALUE`; turned off with `require_name_texts` (warning) |
| PHOENIX-LIB-013 | Device set without gates or devices | a device set with no gate or no device (warning) |
| PHOENIX-LIB-014 | Device set without a reference prefix | a device set whose prefix is empty (warning) |
| PHOENIX-LIB-020 | Library file cannot be used | a file that cannot be read, is larger than `max_file_bytes`, is an EAGLE binary file, is not XML, or is a board or a schematic and not a library |
| PHOENIX-LIB-021 | Name is not valid | a name that is empty, longer than 127 characters or has white space, control characters or one of `<>&"` |
| PHOENIX-LIB-022 | Name is already used | creating or renaming a footprint, symbol, device set, gate or device to a name that is taken, or generating a footprint whose name is in the library given with `--into` |
| PHOENIX-LIB-023 | Item does not exist | an operation about a footprint, symbol, device set, gate, device, pad, pin or item that is not there |
| PHOENIX-LIB-024 | Value is not valid for the item | a value out of the range of the design, a size that is zero or negative, a keyword that does not exist, a rotation that is not written as the file does, a text with space at its ends, an item in the wrong kind of drawing, or an item that cannot be moved |
| PHOENIX-LIB-025 | Library cannot be written | the target exists (the input exists too, so it is never replaced either) or cannot be made or written |
| PHOENIX-LIB-026 | Written library does not read back the same | the file written and read again is not the library in memory, or writing it again gives another text; nothing is written |
| PHOENIX-LIB-027 | File of operations is not valid | a file that is not JSON, is not a list of operations (or an object with `operations`) or has an operation that does not exist; also a plan of the sanitizer that is not valid |
| PHOENIX-LIB-028 | Footprint parameter is not valid | a family that does not exist, a parameter that the family does not have or lacks, a number out of range or lands that would touch |
| PHOENIX-LIB-029 | Library editor settings are not valid | a line of `config/libedit.cfg` or `config/footprints.cfg` that is not `key = value`, an unknown key or a value out of range, or a value that the family needs and the file lacks |
| PHOENIX-LIB-030 | Replacement by the master is not confirmed | a step of a plan of the sanitizer applied without the confirmation of the owner (nothing is changed) |
| PHOENIX-LIB-031 | Command option is not valid | an option that does not exist or lacks its value, no library or family, two libraries, a language that is not one of the eight |
| PHOENIX-LIB-032 | Backup of the original cannot be made | the copy of the original next to the new file cannot be made (nothing is written) |
| PHOENIX-LIB-033 | Item is still used by a device | deleting a footprint that a device uses or a symbol that a gate uses |

## Library of footprints by IPC-7351B (`eh3d-phoenix-ipclib`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English. Codes 001 to 010 are the findings of the checker of the library (`phoenix ipc check`, and `phoenix ipc generate` on its own footprint); 020 and above are errors of the generator and of the commands; 040 to 042 are the differences that `phoenix ipc compare` reports.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-IPC-001 | Family file is not valid | a family file that is not JSON, has another format than `phoenix-ipc-family/1`, an unknown layout or naming, a missing density level, a variant without a value its layout needs, a number that is negative or not finite, or a family that appears twice |
| PHOENIX-IPC-002 | Two footprints have the same name | two footprints of the library with one name, compared without regard to case (error) |
| PHOENIX-IPC-003 | Pads overlap | the boxes of two pads of one footprint overlap (error) |
| PHOENIX-IPC-004 | Distance between pads is not the pitch of the family | the closest two pad centres are not at the pitch of the family, within `pitch_tol_mm` of `config/ipc.cfg` (or are closer than the pitch, in the layouts that skip places) (error) |
| PHOENIX-IPC-005 | Origin is not at the centre of the pads | the middle of the box of the pads is farther from the origin than `origin_tol_mm` (error) |
| PHOENIX-IPC-006 | Solder mask or paste openings are not coherent | a surface-mount pad without an open solder mask, without paste or with more than 100 per cent of paste, a reduced paste above `ep_paste_max_percent`, or a through-hole pad with paste or without a mask opening (error) |
| PHOENIX-IPC-007 | Silkscreen is too close to a pad | a silkscreen line or dot closer to a pad than `silk_clearance_mm` (error) |
| PHOENIX-IPC-008 | Courtyard does not follow the density level | a courtyard that is not the box of the pads and the body grown by the excess of the density level, or an excess smaller than the one the density level of the family asks for (error) |
| PHOENIX-IPC-009 | Name does not follow the naming of IPC-7351B | a name with characters other than upper case letters, digits, `_` and `-`, that does not start with the prefix of the family or does not end with the letter of the density level (M, N or L) (error) |
| PHOENIX-IPC-010 | Pad without a size, or a hole larger than the pad | a pad with no width or height, or a hole that is not smaller than the pad (error) |
| PHOENIX-IPC-020 | Family does not exist | a family that the folder of families does not have; the detail lists the ones it has |
| PHOENIX-IPC-021 | Variant does not exist | a variant that the family does not have; the detail lists the ones it has |
| PHOENIX-IPC-022 | Parameter of the generator is not valid | a parameter that the family does not have, a number of pins out of range, odd where it must be even or not a whole number, rows that the family cannot have, or a prefix that is not one of the family |
| PHOENIX-IPC-023 | Density level is not A, B or C | a density level that is not A, B or C (or M, N or L) |
| PHOENIX-IPC-024 | The dimensions give lands that cannot be made | dimensions that give lands that touch, have no size or cross the axis, an exposed pad that touches the lands, a hole with too thin a ring of copper, or balls whose pads would touch |
| PHOENIX-IPC-025 | Command option is not valid | an option that does not exist, lacks its value or does not apply to the command, a missing or extra argument, a language that is not one of the eight, an output file that is not `.lbr` or `.kicad_mod` |
| PHOENIX-IPC-026 | File cannot be read | a settings, family, library or footprint file that cannot be read, is too large, is not an EAGLE library or a KiCad footprint, or a footprint that the library does not have |
| PHOENIX-IPC-027 | File cannot be written, or it exists | the output file exists (it is never written over) or cannot be written, or the library written does not read back the same |
| PHOENIX-IPC-028 | Settings file is not valid | a line of `config/ipc.cfg` that is not `key = value`, an unknown key or a value out of range; the message names the line |
| PHOENIX-IPC-040 | Pad differs from the generated one | a pad of the footprint of the owner whose position, size or hole differs from the generated pad by more than `compare_tol_mm`; the report gives the differences |
| PHOENIX-IPC-041 | Pad of the generated footprint is missing | a pad of the generated footprint that has no pad of the same name in the footprint of the owner |
| PHOENIX-IPC-042 | Pad has no counterpart in the generated footprint | a pad of the footprint of the owner that has no pad of the same name in the generated footprint |

## Bulk import of libraries (`eh3d-phoenix-ipclib`, `src/codes.rs`)

Codes 001 to 010 are the findings of the quality report of an imported footprint or symbol, listed in the index `phoenix-libindex/1` (`phoenix libindex`); 020 and above are about the files, the folder and the command. A file that cannot be read does not stop the others.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-LIBIDX-001 | Pads with the same name | two or more pads of an imported footprint with one name (empty names of mounting holes do not count) |
| PHOENIX-LIBIDX-002 | Pad without copper | a pad with no size, with no copper layer or on no layer of copper |
| PHOENIX-LIBIDX-003 | Hole larger than the pad | a pad whose hole is larger than its smaller side |
| PHOENIX-LIBIDX-004 | Origin far from the pads | a footprint whose origin is farther than `origin_far_mm` of `config/import_bulk.cfg` from the box of the centres of its pads |
| PHOENIX-LIBIDX-005 | Name is empty | a footprint or a symbol whose name is empty |
| PHOENIX-LIBIDX-006 | Footprint without pads or symbol without pins | a footprint without pads or a symbol without pins |
| PHOENIX-LIBIDX-007 | Pins of a symbol with the same name | two or more pins of a symbol with one name or number (the pins of every unit of a symbol of KiCad are counted together) |
| PHOENIX-LIBIDX-008 | Same name used more than once | a footprint or a symbol whose name another item of the folder also has, in the same library or in another one; the detail lists the members |
| PHOENIX-LIBIDX-009 | Same geometry under more than one name | a footprint with the same pads (positions, sizes and holes, rounded to `geometry_round_mm`) as one with another name, in either format; the detail lists the members |
| PHOENIX-LIBIDX-010 | Pin of a symbol without a name | a pin of a symbol whose name (EAGLE) or number (KiCad) is empty |
| PHOENIX-LIBIDX-020 | File cannot be read or is not a library | a file that cannot be read or is not a library of the kind its extension says; the file is listed with the code and nothing of it is indexed |
| PHOENIX-LIBIDX-021 | File is larger than the limit and was skipped | a file larger than `max_file_bytes`; it is not read |
| PHOENIX-LIBIDX-022 | Folder has more files than the limit | more library files than `max_files`; the first ones in the order of the paths are read |
| PHOENIX-LIBIDX-023 | Folder is deeper than the limit | a folder more than `max_depth` levels below the one given; it is not entered |
| PHOENIX-LIBIDX-024 | Link or path outside the folder was skipped | a link, or a path whose real path is outside the folder; it is never followed or read |
| PHOENIX-LIBIDX-025 | More items than the limit | more footprints and symbols than `max_items`; the rest are not indexed |
| PHOENIX-LIBIDX-026 | Command option is not valid | an option that does not exist, lacks its value or does not apply, no folder or two, a language that is not one of the eight, an `--out` that does not end in `.json` |
| PHOENIX-LIBIDX-027 | Index cannot be written, or the file exists | the file named with `--out` exists (it is never written over) or cannot be written |
| PHOENIX-LIBIDX-028 | Settings file of the import is not valid | a line of `config/import_bulk.cfg` that is not `key = value`, an unknown key or a value out of range; the message names the line |
| PHOENIX-LIBIDX-029 | Folder cannot be read | a folder that does not exist, is not a folder or cannot be read |

## Electrical rule check (`eh3d-phoenix-erc`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a finding is in English. A rule can be turned off or given another level in `config/erc.cfg`.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-ERC-001 | Pin is not connected | a pin of a part that is on no net, unless its type is `not_connected` |
| PHOENIX-ERC-002 | Net has a single pin | a net with one pin or none (a warning) |
| PHOENIX-ERC-003 | Pin types in conflict on a net | two pins on one net whose types the matrix of `config/erc.cfg` does not allow together; the level is the one of the pair in the matrix |
| PHOENIX-ERC-004 | Input pins have no driver | a net with input pins and no pin of the types listed as `drivers` |
| PHOENIX-ERC-005 | Power input has no source | a net with power input pins and no pin of the types listed as `power_sources` |
| PHOENIX-ERC-006 | Net names look alike | two names that differ only by case or by a number on the end, when the stem has at least `similar_name_min_stem` characters (a warning) |
| PHOENIX-ERC-007 | Reference designator used twice | two or more parts with one reference designator |
| PHOENIX-ERC-008 | Part has no value | a part with an empty value (a warning) |
| PHOENIX-ERC-009 | Part has no reference designator | a part with an empty reference designator |
| PHOENIX-ERC-010 | Part has no footprint | a part with no footprint chosen (a warning) |
| PHOENIX-ERC-011 | Symbol pin has no number | a pin of a symbol with an empty number |
| PHOENIX-ERC-012 | Pin number used twice in a symbol | two or more pins of one symbol with one number |
| PHOENIX-ERC-013 | Wires join no pin | wires joined to each other but to no pin of a placed part (a warning) |
| PHOENIX-ERC-014 | Junction on wires that join no pin | three or more wire pieces meeting at a point of wires that join no pin (a warning) |
| PHOENIX-ERC-015 | Check could not be judged | a finding in the inconclusive list: pins without a type, pins without a position, a list cut at `max_findings` (information, never a failure) |
| PHOENIX-ERC-016 | Settings are not valid | a line of the settings text that is not `key = value`, an unknown key or rule, or a value out of range |
| PHOENIX-ERC-017 | Net has no driver | a net with two pins or more and no pin of a type listed as `strong_drivers` (a passive pin does not drive); off by default |
| PHOENIX-ERC-018 | Power pin has no decoupling capacitor near it | a power input of a part whose prefix is in `decoupling_prefixes` on a net with no capacitor to a ground net within `decoupling_max_distance_nm`; off by default |
| PHOENIX-ERC-019 | Net names may be a typing mistake | two net names one character apart (one added, dropped, changed or two neighbours swapped) that the rule of similar names does not name; a change of one digit to another is a bus (a warning) |
| PHOENIX-ERC-020 | Label touches no wire or pin | a net label or a power port that lies on no wire and no pin of its sheet (a warning) |

## Synchronisation (`eh3d-phoenix-sync`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`). A change (001 to 008) is in the confirmed list of the report, what could not be judged or done (009 to 013) in the inconclusive list.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SYNC-001 | Part has no component on the board | a part of the schematic that no component was made from; forward annotation adds the component |
| PHOENIX-SYNC-002 | Component has no part in the schematic | a component that names a part that is gone; it is kept, or removed when `remove_missing_components` is true (information) |
| PHOENIX-SYNC-003 | Reference designators differ | a part and its component with different reference designators |
| PHOENIX-SYNC-004 | Values differ | a part and its component with different values |
| PHOENIX-SYNC-005 | Footprints differ | a part with a footprint chosen that is not the one of its component (backward, also a part with none chosen) |
| PHOENIX-SYNC-006 | Net is new on the board | a net of the schematic that the board does not have |
| PHOENIX-SYNC-007 | Net of the board is renamed | a net of the board that the pads of a schematic net all sit on, with another name |
| PHOENIX-SYNC-008 | Pad changes net | a pad on another net than its pin, or on a net when its pin is on none |
| PHOENIX-SYNC-009 | Footprint is not chosen or not on the board | a part without a footprint, or with one the board does not have, so no component is made or no footprint changed (inconclusive) |
| PHOENIX-SYNC-010 | Pin has no pad in the footprint | a pin of a net whose number no pad of the footprint of the component has (inconclusive) |
| PHOENIX-SYNC-011 | Part used by two components | two components made from one part; the second is left alone (inconclusive) |
| PHOENIX-SYNC-012 | Reference designator would be empty or used twice | a change of reference designator that would leave it empty or give it to two parts (inconclusive) |
| PHOENIX-SYNC-013 | Two nets of the schematic share a name | a second net with the name of another; it is left alone (inconclusive) |
| PHOENIX-SYNC-014 | Settings are not valid | a line of the settings text that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-SYNC-015 | Change could not be applied | an operation that does not apply to the document it is applied to (a package made for another board or schematic) |

## Annotation back into the schematic and comparison (`eh3d-phoenix-sync`, `src/annot_codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/annot_codes.rs`). The codes 001 to 009 are the items of the plan of `phoenix annotate` (001 to 007 the changes, 008 the state conflict, 009 the state left out by the settings), 010 to 012 are errors of the command, and 013 to 023 are the entries of the comparison of the two documents (`phoenix sync --diff`); 021 to 023 are conflicts.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-ANNOT-001 | Reference designator differs on the board | a component whose reference designator is not the one of its part; the plan writes the one of the board into the schematic |
| PHOENIX-ANNOT-002 | Value differs on the board | a component whose value is not the one of its part |
| PHOENIX-ANNOT-003 | Footprint differs on the board | a component whose footprint is not the one chosen for its part |
| PHOENIX-ANNOT-004 | Component on the board has no part | a component with no part in the schematic; listed and ignored, a part needs a symbol that only the drawing can give |
| PHOENIX-ANNOT-005 | Part has no component on the board | a part with no component; listed and ignored |
| PHOENIX-ANNOT-006 | Equivalent pins or gates swapped on the board | pins or gates that `config/annotate.cfg` lists as equivalent and that the board connects to the nets of the schematic the other way round |
| PHOENIX-ANNOT-007 | Reference designator renumbered by position | a reference that the renumbering by position (`renumber = true`) changes, on the board and in the schematic |
| PHOENIX-ANNOT-008 | Change conflicts with the rest of the plan | the state `conflict` of an item: a reference that would be empty or used twice, a second component of one part, a footprint the board does not have, a swap that the net list alone cannot take |
| PHOENIX-ANNOT-009 | Change left out by the settings | the state `ignored` of a value or footprint change when `values` or `footprints` is false |
| PHOENIX-ANNOT-010 | Annotation settings are not valid | a line of the settings text that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-ANNOT-011 | Confirmation does not match the plan | `--apply` with a confirmation code that is not the digest of the plan that was read |
| PHOENIX-ANNOT-012 | Plan could not be applied | an edit that does not apply to the schematic or the board it is applied to |
| PHOENIX-ANNOT-013 | Part has no component (schematic only) | a part of the schematic with no component on the board |
| PHOENIX-ANNOT-014 | Component has no part (board only) | a component of the board with no part in the schematic |
| PHOENIX-ANNOT-015 | Values differ | a part and its component with different values |
| PHOENIX-ANNOT-016 | Footprints differ | a part and its component with different footprints |
| PHOENIX-ANNOT-017 | Reference designators differ | a part and its component with different reference designators |
| PHOENIX-ANNOT-018 | Net exists in one document only | a net name that the other document does not have |
| PHOENIX-ANNOT-019 | Pin is on a net in one document only | a pin of a paired part on a net in one document and on none in the other |
| PHOENIX-ANNOT-020 | Net holds other pins in each document | a net name that both have, with other pins of paired parts on it |
| PHOENIX-ANNOT-021 | Reference designator used twice | one reference designator on two parts or on two components (a conflict) |
| PHOENIX-ANNOT-022 | Part used by two components | two components made from one part (a conflict) |
| PHOENIX-ANNOT-023 | Reference designator taken by another item | a change that would give a reference to an item while another item of the other document keeps it (a conflict) |

## Schematic editor (`eh3d-phoenix-schedit`, `src/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SCHEDIT-001 | Settings file is not valid | a line of `config/schedit.cfg` that is not `key = value`, an unknown key, or a value out of range |
| PHOENIX-SCHEDIT-002 | File of operations is not valid | a file of operations or a library of symbols that cannot be read, or an operation of the file that the editor refused (the message gives its number) |
| PHOENIX-SCHEDIT-003 | Output file is not allowed | `phoenix schedit` asked to write onto its input file or onto a file that exists, or in a format the editor does not write |

## Sheet hierarchy (`eh3d-phoenix-schematic`, `src/hier.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-HIER-001 | Sheet symbol, pin or port refers to something that does not exist | a sheet symbol that is drawn on, or shows, a sheet that does not exist, a pin on a symbol that does not exist, or a port on a sheet that does not exist |
| PHOENIX-HIER-002 | Sheet contains itself | a sheet symbol that shows the sheet that holds it, directly or through other sheets (a loop; the operation that would make one is refused) |
| PHOENIX-HIER-003 | Sheet symbol name is empty, repeated on its sheet or has a slash | a sheet symbol whose name is empty, has a slash or surrounding spaces, or is the name of another symbol of the same sheet |
| PHOENIX-HIER-004 | Pin of a sheet symbol has no port in the sheet it shows | a pin of a sheet symbol whose name is not the name of any port of the sheet that the symbol shows (an error) |
| PHOENIX-HIER-005 | Port of a sheet has no pin on the sheet symbol | a port of a sheet with no pin of its name on a symbol that shows the sheet (a warning) |
| PHOENIX-HIER-006 | Pin or port name is repeated | two pins of one sheet symbol, or two ports of one sheet, with the same name |
| PHOENIX-HIER-007 | Reference designator repeated after expanding the sheets | two parts with the same reference after the sheets are expanded (found by the expansion) |
| PHOENIX-HIER-008 | Hierarchy is too deep or has too many instances | a hierarchy deeper than `max_depth` or with more instances than `max_instances` in `config/hierarchy.cfg` |
| PHOENIX-HIER-009 | Pin and port of one name have different directions | a pin and a port of one name whose directions do not fit, such as an input against an output (a warning) |

The titles of the hierarchy codes in the eight languages of the suite:

| Code | English | Español | Deutsch | Français | Italiano | 日本語 | 中文 | Português (Portugal) |
|---|---|---|---|---|---|---|---|---|
| `PHOENIX-HIER-001` | Sheet symbol, pin or port refers to something that does not exist | El símbolo de hoja, el pin o el puerto se refiere a algo que no existe | Blattsymbol, Pin oder Port verweist auf etwas, das nicht existiert | Le symbole de feuille, la broche ou le port renvoie à quelque chose qui n'existe pas | Il simbolo di foglio, il pin o la porta rimanda a qualcosa che non esiste | シートシンボル、ピン、またはポートが存在しないものを参照しています | 图纸符号、引脚或端口引用了不存在的对象 | O símbolo de folha, o pino ou a porta refere algo que não existe |
| `PHOENIX-HIER-002` | Sheet contains itself | Una hoja se contiene a sí misma | Ein Blatt enthält sich selbst | Une feuille se contient elle-même | Un foglio contiene se stesso | シートが自分自身を含んでいます | 图纸包含自身 | Uma folha contém-se a si própria |
| `PHOENIX-HIER-003` | Sheet symbol name is empty, repeated on its sheet or has a slash | El nombre del símbolo de hoja está vacío, repetido en su hoja o lleva una barra | Name des Blattsymbols ist leer, auf dem Blatt doppelt oder enthält einen Schrägstrich | Le nom du symbole de feuille est vide, répété sur sa feuille ou contient une barre oblique | Il nome del simbolo di foglio è vuoto, ripetuto nel foglio o contiene una barra | シートシンボル名が空、同じシート上で重複、またはスラッシュを含んでいます | 图纸符号名称为空、在图纸上重复或含有斜杠 | O nome do símbolo de folha está vazio, repetido na sua folha ou tem uma barra |
| `PHOENIX-HIER-004` | Pin of a sheet symbol has no port in the sheet it shows | Un pin del símbolo de hoja no tiene puerto en la hoja que muestra | Ein Pin des Blattsymbols hat keinen Port auf dem gezeigten Blatt | Une broche du symbole de feuille n'a pas de port dans la feuille qu'il montre | Un pin del simbolo di foglio non ha porta nel foglio che mostra | シートシンボルのピンに対応するポートが参照先のシートにありません | 图纸符号的引脚在其引用的图纸中没有对应端口 | Um pino do símbolo de folha não tem porta na folha que mostra |
| `PHOENIX-HIER-005` | Port of a sheet has no pin on the sheet symbol | Un puerto de la hoja no tiene pin en el símbolo de hoja | Ein Port des Blatts hat keinen Pin am Blattsymbol | Un port de la feuille n'a pas de broche sur le symbole de feuille | Una porta del foglio non ha pin sul simbolo di foglio | シートのポートに対応するピンがシートシンボルにありません | 图纸的端口在图纸符号上没有对应引脚 | Uma porta da folha não tem pino no símbolo de folha |
| `PHOENIX-HIER-006` | Pin or port name is repeated | Nombre de pin o de puerto repetido | Pin- oder Portname doppelt | Nom de broche ou de port répété | Nome di pin o di porta ripetuto | ピン名またはポート名が重複しています | 引脚或端口名称重复 | Nome de pino ou de porta repetido |
| `PHOENIX-HIER-007` | Reference designator repeated after expanding the sheets | Designador de referencia repetido al expandir las hojas | Bauteilkennung nach dem Auffalten der Blätter doppelt | Référence en double après le dépliage des feuilles | Designatore di riferimento ripetuto dopo l'espansione dei fogli | シートの展開後に参照指定子が重複しています | 展开图纸后位号重复 | Designador de referência repetido depois de expandir as folhas |
| `PHOENIX-HIER-008` | Hierarchy is too deep or has too many instances | La jerarquía es demasiado profunda o tiene demasiadas instancias | Die Hierarchie ist zu tief oder hat zu viele Instanzen | La hiérarchie est trop profonde ou a trop d'instances | La gerarchia è troppo profonda o ha troppe istanze | 階層が深すぎるか、インスタンスが多すぎます | 层次过深或实例过多 | A hierarquia é demasiado profunda ou tem demasiadas instâncias |
| `PHOENIX-HIER-009` | Pin and port of one name have different directions | El pin y el puerto del mismo nombre tienen direcciones distintas | Pin und Port gleichen Namens haben verschiedene Richtungen | La broche et le port du même nom ont des directions différentes | Il pin e la porta con lo stesso nome hanno direzioni diverse | 同名のピンとポートの方向が異なります | 同名的引脚和端口方向不同 | O pino e a porta com o mesmo nome têm direções diferentes |

## Buses (`eh3d-phoenix-schematic`, `src/bus.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-BUS-001 | Bus name is not valid | a bus name that is not a vector such as `D[0..7]`, a group such as `USB{DP DM}` or the name of an alias, or that expands to more than 4096 members |
| PHOENIX-BUS-002 | Bus member has no connection | members of a bus that no bus entry uses (a warning that lists the first eight) |
| PHOENIX-BUS-003 | Bus has repeated members | a bus with two members of one name, as in `U{A A}` |
| PHOENIX-BUS-004 | Bus range is not valid | a vector with a negative number or with more than 256 members |
| PHOENIX-BUS-005 | Bus is connected to a simple net | a bus whose anchor is the position of a pin or a point of a wire, so it would be joined to a simple net (an error) |
| PHOENIX-BUS-006 | Bus entry names a bus or a member that does not exist | a bus entry that names a bus that does not exist or a member that the bus does not have |
| PHOENIX-BUS-007 | Bus alias is not valid | an alias with an empty list, a name that cannot be used or the name of another alias, or items that do not expand |

The titles of the bus codes in the eight languages of the suite:

| Code | English | Español | Deutsch | Français | Italiano | 日本語 | 中文 | Português (Portugal) |
|---|---|---|---|---|---|---|---|---|
| `PHOENIX-BUS-001` | Bus name is not valid | El nombre del bus no es válido | Busname ist ungültig | Le nom du bus n'est pas valide | Il nome del bus non è valido | バス名が正しくありません | 总线名称无效 | O nome do barramento não é válido |
| `PHOENIX-BUS-002` | Bus member has no connection | Un miembro del bus no tiene conexión | Ein Busmitglied hat keine Verbindung | Un membre du bus n'a aucune connexion | Un membro del bus non ha connessioni | バスのメンバーが接続されていません | 总线成员没有连接 | Um membro do barramento não tem ligação |
| `PHOENIX-BUS-003` | Bus has repeated members | El bus tiene miembros repetidos | Der Bus hat doppelte Mitglieder | Le bus a des membres en double | Il bus ha membri ripetuti | バスにメンバーの重複があります | 总线含有重复成员 | O barramento tem membros repetidos |
| `PHOENIX-BUS-004` | Bus range is not valid | El intervalo del bus no es válido | Der Busbereich ist ungültig | La plage du bus n'est pas valide | L'intervallo del bus non è valido | バスの範囲が正しくありません | 总线范围无效 | O intervalo do barramento não é válido |
| `PHOENIX-BUS-005` | Bus is connected to a simple net | Un bus está conectado a una red simple | Ein Bus ist mit einem einfachen Netz verbunden | Un bus est relié à un réseau simple | Un bus è collegato a una rete semplice | バスが単一のネットに接続されています | 总线连接到了普通网络 | Um barramento está ligado a uma rede simples |
| `PHOENIX-BUS-006` | Bus entry names a bus or a member that does not exist | La entrada de bus nombra un bus o un miembro que no existe | Der Buseintrag nennt einen Bus oder ein Mitglied, das nicht existiert | L'entrée de bus nomme un bus ou un membre qui n'existe pas | L'ingresso di bus nomina un bus o un membro che non esiste | バスエントリが存在しないバスまたはメンバーを指しています | 总线入口指向不存在的总线或成员 | A entrada de barramento indica um barramento ou um membro que não existe |
| `PHOENIX-BUS-007` | Bus alias is not valid | El alias de bus no es válido | Der Busalias ist ungültig | L'alias de bus n'est pas valide | L'alias di bus non è valido | バスのエイリアスが正しくありません | 总线别名无效 | O alias de barramento não é válido |

## Board editor (`eh3d-phoenix-boardedit`, `src/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-BEDIT-001 | Settings file is not valid | a line of `config/boardedit.cfg` that is not `key = value`, an unknown key, or a value out of range |
| PHOENIX-BEDIT-002 | File of operations is not valid | a file of operations that cannot be read or that has more than the largest number of operations, or a settings file that cannot be opened |
| PHOENIX-BEDIT-003 | Output file is not allowed | `phoenix boardedit` asked to write onto an input file or onto a file that exists, or in a format the editor does not write |
| PHOENIX-BEDIT-004 | Operation was refused | an operation that refers to a component, track, via, net, layer, footprint or pad the board does not have, a reference that is used, two pads on different nets, a layer that is not copper, or that the board refuses; the message of a run gives the number of the operation |
| PHOENIX-BEDIT-005 | Operation adds a rule error | an operation that adds a finding of the design rules that is an error (a clearance, a short, a width) while `reject_new_errors` is on and the operation is not forced |
| PHOENIX-BEDIT-006 | Component is locked | moving, turning, mirroring or removing a component that was locked |
| PHOENIX-BEDIT-007 | Shape cannot be made | a route of fewer than two points or of no length, a width that is not above zero, a vertex or a piece that does not exist, a point out of the design range |
| PHOENIX-BEDIT-008 | Net cannot be tuned to the length asked for | a target that is not above zero, a net with no copper, a net already longer than the target, no horizontal or vertical piece long enough for the meanders, an amplitude above the largest allowed, or a result that would end farther from the target than the tolerance |
| PHOENIX-BEDIT-009 | Nothing is selected | `move_selection`, `delete_selection` or `align_selection` while the selection of the editor is empty |
| PHOENIX-BEDIT-010 | Item is not on the board | an object named in `select` (`add` or `toggle`), `move_items`, `delete_items` or an alignment, or a net named in `select`, that the board does not have, or an object named twice |
| PHOENIX-BEDIT-011 | Too few items to align or distribute | `align_selection` with fewer than two objects (`left`, `right`, `top`, `bottom`, `center_x`, `center_y`) or fewer than three (`distribute_x`, `distribute_y`) |

## Differential pairs (`eh3d-phoenix-boardedit`, `src/pair_codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/pair_codes.rs`); the detail of a message is in English and names the pair. The codes 003 to 010 are the findings of `phoenix pairs`; the others are errors of the files and of the pair operations of `phoenix boardedit`.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-PAIR-001 | Pair definitions are not valid | a line of `config/diffpairs.cfg` that is not `key = value` or `pair.NAME.field = value`, an unknown key or field, a value out of range, a pair without both nets or with the same net twice, or a JSON file of pairs (format `phoenix-diffpairs/1`) that cannot be read |
| PHOENIX-PAIR-002 | Net of a pair is not on the board | a pair whose positive or negative net the board does not have |
| PHOENIX-PAIR-003 | Separation of the pair is out of tolerance | the gap between the edges of the two tracks, where they run coupled, is smaller or larger than the gap of the pair by more than `gap_tolerance_nm` |
| PHOENIX-PAIR-004 | Pair is not symmetric | the coupled length of the positive net and the one of the negative net differ by more than `symmetry_tolerance_nm` |
| PHOENIX-PAIR-005 | Pair runs uncoupled for too long | more of either net than the maximum of the pair runs farther than `coupling_range_nm` from the other net |
| PHOENIX-PAIR-006 | Lengths of the pair differ too much | the copper lengths of the two nets (tracks and vias) differ by more than the length tolerance of the pair |
| PHOENIX-PAIR-007 | Vias of the pair are not paired | a via of one net with no via of the other net on the same layers within `via_pair_range_nm` |
| PHOENIX-PAIR-008 | Tracks of the pair have the wrong width | a piece of either net that is not as wide as the pair asks for, to within `width_tolerance_nm` |
| PHOENIX-PAIR-009 | Differential impedance is off the target | warning: the impedance worked out from the stack-up for the width and the gap of the pair is farther from the target of the pair than `impedance_tolerance_permille`, or cannot be worked out |
| PHOENIX-PAIR-010 | Pair has no coupled stretch | warning: no piece of either net runs within `coupling_range_nm` of the other net, so the gap and the symmetry are not judged |
| PHOENIX-PAIR-011 | Operation on a pair cannot be done | `route_pair`, `tune_pair` or `fix_pair_gap` on a pair that is not defined, or `fix_pair_gap` when pieces with a wrong gap exist and none of them can be moved |

## Push of neighbour tracks and arcs (`eh3d-phoenix-boardedit`, `src/shove_codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/shove_codes.rs`); the detail of a message is in English. They are the errors of the operations `route_shove`, `route_arc` and `round_corners` of `phoenix boardedit` and of the window, and each one leaves the board as it was.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SHOVE-001 | Push is blocked by fixed copper | a track that would have to be pushed against a pad, a via or the edge of the board (they are never moved), or that cannot be pushed far enough |
| PHOENIX-SHOVE-002 | Push goes deeper than allowed | a pushed track that would have to push a track that pushes another, past `shove_max_depth` levels |
| PHOENIX-SHOVE-003 | Push would move too many tracks | a push that would move more tracks than `shove_max_moved` |
| PHOENIX-SHOVE-004 | Track in the way cannot be pushed | a track in the way that is an arc or has a piece that is neither horizontal, vertical nor at 45 degrees, or that has a via, a pad or a branch of its net along the piece |
| PHOENIX-SHOVE-005 | Route crosses a track of another net | a route that crosses or touches a track of another net: no parallel push can solve it |
| PHOENIX-SHOVE-006 | Push would leave a rule error | a result (the new track and the tracks pushed) that the design rules judge to add an error, also when `reject_new_errors` is off and also for a forced operation |
| PHOENIX-ARC-001 | Radius of the arc is not valid | a radius below `arc_min_radius_nm` or above 2^29 nm |
| PHOENIX-ARC-002 | Corner cannot be rounded | a vertex that does not exist, that is not between two straight pieces, whose pieces run on one line or turn back, or whose pieces are shorter than the arc needs; `round_corners` when no corner of the track or the net could be rounded |
| PHOENIX-ARC-003 | Arc is too close to other copper or to the edge | an arc that, cut into short chords, comes nearer than the clearance of the rules (plus `arc_margin_nm`) to the copper of another net, a pad, a via or the edge of the board |

## Zones in the board editor (`eh3d-phoenix-boardedit`, `src/zone_codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/zone_codes.rs`); the detail of a message is in English. The notes of the operations on zones carry them, and so do the errors of those operations. Codes of the fill itself (`PHOENIX-ZONE-<NNN>`) appear in the notes as they are.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-BZONE-001 | Zone is not on the board | `zone_edit`, `zone_remove`, `zone_fill`, `zone_priority` or `zone_cutout` on a zone the board does not have |
| PHOENIX-BZONE-002 | Outline of the zone cannot be used | an outline with fewer than three points, with two equal neighbour points, with no area, with a side that folds back or with two sides that touch, or one that the fill cannot cut into convex pieces |
| PHOENIX-BZONE-003 | Property of the zone is out of range | a negative length, a number of spokes that is not 2 or 4, or a clearance out of the design range |
| PHOENIX-BZONE-004 | Cutout of the zone is not valid | a cutout that is not a polygon, that has a point outside the zone, that crosses a side of the outline, or a cutout to remove that does not exist |
| PHOENIX-BZONE-005 | Fill breaks a clearance or joins nets | a fill that the check of `eh3d-phoenix-zones` proves to be nearer than the clearance to copper of another net, to lie outside the zone or the board, or (in a note) two zones of one priority that overlap; also copper added after the fill |
| PHOENIX-BZONE-006 | Part of the fill is dropped or joined to nothing | an island that the fill dropped for its size, its width or because it touches no copper of its net, or a zone with nothing left after the fill (a note) |
| PHOENIX-BZONE-007 | Zone has no fill | a zone that was never filled or whose fill was dropped because it was edited (a note) |
| PHOENIX-BZONE-008 | Group cannot be pasted | a paste that adds a rule error, that names a layer the board does not have, that turns by something other than quarter turns, that gives both a clipboard and a net or neither, or that is empty |
| PHOENIX-BZONE-009 | Clipboard cannot be used | a clipboard of another format, a net without a name, an item the board does not have, or nothing to copy |
| PHOENIX-BZONE-010 | Zone data cannot be written to the format | the fill, the cutouts, the priority, the connection or the thermal relief of a zone, in a Gerber file without a fill, in a Specctra DSN file or in a KiCad board (a note for each zone) |
| PHOENIX-BZONE-011 | File of zone data is not valid | a file of zone data that cannot be read, of another format, with an unknown field or with a property out of range, or a fill that cannot be poured |

## Circuit simulator (`eh3d-phoenix-sim`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English. A message about a netlist names the line.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SIM-001 | Netlist line is not valid | a line with too few or too many fields, a brace or a continuation line that has nothing to continue, an option that is not understood in a source; the message names the line |
| PHOENIX-SIM-002 | Value is not valid | a number, a unit or an expression between braces that cannot be read, a resistance or an inductance that is not above zero, a coupling coefficient outside 0 and 1, a waveform with values that do not fit; the message names the line |
| PHOENIX-SIM-003 | Element, directive or model is not supported | an element letter other than `R C L V I E G F H K D Q M S U X`, a model type other than `D LED ZENER NPN PNP NMOS PMOS SW OPAMP COMP`, a transistor with a fourth (substrate) node, the parameters of a subcircuit; the message names the line |
| PHOENIX-SIM-004 | Name is used twice | two elements, two models of the same scope (the netlist, or one subcircuit), two subcircuits or two parameters with the same name (element names are compared without regard to case) |
| PHOENIX-SIM-005 | Reference to something that is not defined | a diode, transistor or amplifier with a model that is not there, an instance of a subcircuit that is not defined, a parameter that is not defined, an inductor of a coupling or a voltage source of a controlled source that is not in the same circuit, a probe that is not an unknown of the circuit |
| PHOENIX-SIM-006 | Subcircuit is not valid | a `.subckt` inside another (a `.model` inside one is allowed), without ports, with a ground or repeated port, not closed by `.ends`, closed with another name, an instance with the wrong number of nodes, or nested deeper than `max_subckt_depth` (a subcircuit that calls itself) |
| PHOENIX-SIM-007 | Simulator settings are not valid | a line of the settings text that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-SIM-008 | Limit of the settings exceeded | a netlist larger than `max_file_bytes` or with more than `max_lines` lines, more than `max_elements` elements, more than `max_unknowns` unknowns, more than `max_ac_points` frequencies, more than `max_tran_steps` steps, more than `max_breakpoints` corners of the sources in an adaptive transient analysis or a result of more than `max_result_cells` values |
| PHOENIX-SIM-009 | Circuit equations cannot be solved (singular matrix) | a pivot that is zero, not finite or below `pivot_tolerance` of its column; the message names the unknown left undetermined (voltage sources in parallel, a loop of voltage sources or inductors, a node held only by current sources) |
| PHOENIX-SIM-010 | Node has no path to ground | nodes joined to ground by nothing but capacitors (in DC), current sources, the gate of a MOSFET or the inputs of an amplifier; the message names the nodes; with `floating_is_error = false` a small conductance holds them instead |
| PHOENIX-SIM-011 | Newton-Raphson iteration did not converge | more than `newton_max_iter` iterations of a circuit with diodes, transistors or limited amplifiers, after the gmin stepping and the source stepping have also failed |
| PHOENIX-SIM-012 | Analysis request is not valid | a noise output that is not the voltage of a node, a time step or stop time that is not above zero or a step larger than the stop time, a sweep with no points, a start frequency that is not above zero or a stop frequency below the start, a frequency that is not above zero, an output that is not in the circuit, an RF filter that cannot be designed |
| PHOENIX-SIM-013 | Output file cannot be written | a table or a JSON file that could not be written |
| PHOENIX-SIM-014 | Schematic part could not be translated | a part whose reference designator is not one letter of `R C L V I D E G F H K` followed by digits, whose symbol is missing or has the wrong number of pins, that has no value, whose value does not read, or that repeats a reference designator; the message names the part and the reason (information) |
| PHOENIX-SIM-015 | No ground node | a circuit in which no element is connected to node 0 (or `gnd`), or a schematic in which no translated element is on a ground net |
| PHOENIX-SIM-016 | Command option is not valid | no analysis or no input, an option that does not exist or lacks its value, a number that is not a number, a language that is not one of the eight, a filter family or band that does not exist, a Chebyshev filter without `--ripple-db` |
| PHOENIX-SIM-017 | Result is not a finite number | a solution of the equations that overflows or is not a number |
| PHOENIX-SIM-018 | Simulation disagrees with the RF filter response | a difference of magnitude, phase or group delay above the tolerances of the settings in the check against a ladder filter of the RF tools; the message gives the figures |
| PHOENIX-SIM-019 | Time step reached the minimum | an adaptive step at its smallest size whose estimated error is still above the tolerances; the step is kept (a warning) |
| PHOENIX-SIM-020 | Schematic pin is on no net | a pin of a part that no net lists; it gets a node of its own and the message names the part and the pin (information) |
| PHOENIX-SIM-021 | Input file cannot be read | a netlist or schematic file that cannot be opened, is larger than the limit, is not UTF-8 text or is not a schematic of the project format |
| PHOENIX-SIM-022 | Model does not fit the element | a model of another kind than the element asks for (a diode model on a transistor, a transistor model on a diode, an amplifier or a MOSFET); the message names the element, the model and its type |
| PHOENIX-SIM-023 | Include file is not allowed | an `.include` or a file of models (`--model`) that is refused: a name with a folder, `..`, a drive or an absolute path, a file that is not in the same folder, a file that cannot be found, nesting deeper than `max_include_depth` (a file that includes itself), more than `max_include_files` files, or an `.include` read from text and not from a file; the message names the line and the file |
| PHOENIX-SIM-024 | Analysis directive is not valid | a `.op`, `.dc`, `.tran` or `.ac` line of a netlist with the wrong number of fields, a step of zero or going away from the stop value, a step or a stop time that is not above zero, a start time or a largest step on `.tran`, a number of points that is not a whole number above zero, or a frequency range that is not ordered; the message names the line |
| PHOENIX-SIM-025 | Parameter step (.step) is not valid | a `.step` line or `--step` text without a target or without values, a word that is not `lin`, `dec`, `oct` or `list`, a number that is not a number, a step of zero or going away from the stop value, a number of points that is not a whole number above zero, a start that is not above zero for `dec` and `oct`, a stop below the start, a resistance or an inductance that would not be above zero, a capacitance below zero, an element that cannot be stepped (only resistors, capacitors, inductors and independent and controlled sources can), or a sensitivity to a parameter whose value is zero; the message names the line |
| PHOENIX-SIM-026 | Measurement (.measure) is not valid | a `.measure` line or `--measure` text with fewer than four words, a domain that is not `tran`, `dc` or `ac`, a name that is not a name, a signal that is not `v(node)` or `i(source)`, a kind that does not exist for its analysis, a missing or invalid number, an option the kind does not use, `low` and `high` that are not fractions with `low` below `high`, the name of a measure used twice, or a study with no measure; the message names the line |
| PHOENIX-SIM-027 | Measurement cannot be computed | a measure that reads an analysis (transient, DC sweep or AC sweep) that was not asked for, or a signal that is not in the circuit; a measure that is well formed but finds nothing (a level never crossed) is not an error: its cell is empty |
| PHOENIX-SIM-028 | Monte Carlo request or tolerance is not valid | a number of runs below one or above `max_mc_runs`, a tolerance (`tol=`, `SIM.TOL`, `--tol`) that is not a fraction below 100 % (`5%` or `0.05`), a `dist=` that is not `uniform` or `normal`, a `dist=` with no `tol=`, or a Monte Carlo study in which no resistor, capacitor or inductor has a tolerance; the message names the line |
| PHOENIX-SIM-029 | Poles and zeros cannot be found reliably | a response with fewer than six frequencies or a zero or non-finite value, a requested order outside `1` to `max_pole_order`, or a response that no rational function of up to that many poles fits within `pole_fit_tolerance` (the message gives the best error): poles are never returned from a fit above the tolerance |

## Netlist made from a schematic (`eh3d-phoenix-sim`, `src/netcodes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/netcodes.rs`); the detail of a message is in English and names the part, the pin or the net. Each message has a severity: an error stops a simulation, a warning and information do not.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-NETLIST-001 | Table of simulation models is not valid | a line of the table of models (`config/sim_models.cfg` or the file of `--models`) that is not a rule, an `attr` line, a `.model` line or `upper_m_is_mega`, an unknown device or option, a file name with a folder, `..` or a drive, a pattern that appears twice or a model defined twice; the message names the line |
| PHOENIX-NETLIST-002 | Part has no simulation model | a part that no rule of the table matches (warning, the part is left out), or a part whose value names no model of the table so that the generic model of the settings is used (information) |
| PHOENIX-NETLIST-003 | Pin is on no net | a pin that the device of the part uses and that no net lists (warning); it gets a node of its own |
| PHOENIX-NETLIST-004 | Net has a single connection | a net, other than ground, that only one pin of the translated elements is joined to (warning) |
| PHOENIX-NETLIST-005 | Source is short-circuited | a voltage source or ammeter whose two pins are on one net, or two voltage sources between the same two nets (error) |
| PHOENIX-NETLIST-006 | Value or attribute is not valid | a value that is empty or not a number, a source value that is not a source value, an attribute `SIM.DEVICE`, `SPICE_MODEL` or `SIM.FILE` that is not valid, or an element line that the netlist reader refuses (error, the part is left out) |
| PHOENIX-NETLIST-007 | Pins do not fit the device | a symbol whose pins cannot be given to the roles of the device by their names or their order, or a `pins=` list that names a pin the symbol does not have, repeats one or has the wrong length, or a part whose symbol does not exist (error, the part is left out) |
| PHOENIX-NETLIST-008 | Reference designator is used twice | two parts that give the same element name (error, the second is left out) |
| PHOENIX-NETLIST-009 | Circuit has no ground net | no translated element is joined to a net named `0`, `GND`, `AGND`, `DGND` or `PGND` (the setting `schematic_ground`) (error) |
| PHOENIX-NETLIST-010 | Electrical rule check found errors | `phoenix netlist` and `phoenix sim --schematic` run the electrical rule check first and list its errors under this code (error); `--no-erc` skips the check |
| PHOENIX-NETLIST-011 | Model is not defined | a model that a rule or an attribute names and that neither a `.model` line of the table nor a file defines, a Zener diode with no model (it needs the breakdown voltage), or one model name used for two types (error, the part is left out) |
| PHOENIX-NETLIST-012 | Output file is not allowed | `phoenix netlist --out` names an input file, a file that exists, or a file whose extension is one of the schematic formats |

## Editing and routing (`eh3d-phoenix-route`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-ROUTE-001 | Route settings are not valid | a line of the settings text that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-ROUTE-002 | A limit of the route settings was exceeded | a ratsnest that compares more pairs of points than `ratsnest_max_pairs`, a search that takes more steps than `max_expansions` or an area that does not fit the mesh even with the coarsest pitch |
| PHOENIX-ROUTE-003 | Element to connect was not found | a pad, via or track that the board does not have, a pad name (`R1.2`) that matches no pad or several |
| PHOENIX-ROUTE-004 | Element name is not valid | text that is not `pad:...`, `via:<id>` or `track:<id>[:start]`, or an id that cannot be read |
| PHOENIX-ROUTE-005 | Elements are not on the same net | two elements of different nets, or a pad that has no net |
| PHOENIX-ROUTE-006 | Layer cannot be used | a layer that is not a copper layer, a first element with no copper on the layer asked for, or an element on no layer of the stack |
| PHOENIX-ROUTE-007 | No path was found | no way at 0, 45 and 90 degrees keeps the clearances between the two ends, or the straight way is blocked before the shortest track that is made |
| PHOENIX-ROUTE-008 | Route stopped before an obstacle | a note on a plan made by the mode `stop` whose track ends in front of an obstacle (the exit code of the command is 1) |
| PHOENIX-ROUTE-009 | The rules refuse the track | the rule check on the board with the plan applied finds a new error; the message names the code and the place |
| PHOENIX-ROUTE-010 | The route does not join the two elements | a complete plan whose copper does not touch both elements in the connectivity of the board |
| PHOENIX-ROUTE-011 | Both elements are at the same place | two elements with the same centre |
| PHOENIX-ROUTE-012 | No free id could be made | `id_attempts` ids were tried and all were in use |
| PHOENIX-ROUTE-013 | The board refused the operation | an operation of an edit or of a plan that the board does not apply (a part that is not there, an id in use) |
| PHOENIX-ROUTE-014 | The edit would add a rule error | an edit that adds a proven error of the rules while `strict_edits` is on; the board is not changed |
| PHOENIX-ROUTE-015 | The edit refers to something that is not there | a component, track or net that does not exist, a vertex beyond the end of a track, a piece that would shrink to a point |
| PHOENIX-ROUTE-016 | The edit would break the 45 degree angles | moving a vertex so that a piece that ran at 0, 45 or 90 degrees runs at another angle while `keep_angles` is on |
| PHOENIX-ROUTE-017 | Command option is not valid | an option the command does not know or needs a value for, a mode that is not `around` or `stop`, a layer that is not on the board |
| PHOENIX-ROUTE-018 | Output file exists or cannot be written | an output file that already exists (an input file is never replaced) or cannot be created |
| PHOENIX-ROUTE-019 | A coordinate is out of range | a point or a length that does not fit the checked range of the model |
| PHOENIX-ROUTE-020 | Board or rule file cannot be read | a board or a rule file that cannot be opened, is too large or is not valid |
| PHOENIX-ROUTE-021 | The board cannot be exported to Specctra DSN | a name with a quote or a control character, two nets or two copper layers with one name, two pads of a component with one number, a reference designator with a hyphen, a board with no outline or no copper layer, a pad on no copper layer, a track with an arc piece or a via that does not join the first and the last copper layer |
| PHOENIX-ROUTE-022 | The Specctra session cannot be read | text that is not a session, brackets that do not close or are nested too deep, a number that cannot be read, a coordinate that is missing, or a session larger than the limit |
| PHOENIX-ROUTE-023 | The session does not match the board | a net, a copper layer or a via padstack that the board or the export does not have, a wire that is not a path of straight pieces, or a wire with no width or no length |
| PHOENIX-ROUTE-024 | The autorouter settings are not valid | a line of `config/autorouter.cfg` that is not `key = value`, an unknown key, a value out of range, an address in a path or an argument, a variable whose name suggests a secret, or an enabled adapter without an absolute program, an absolute folder or the design and the session in its arguments |
| PHOENIX-ROUTE-025 | The external autorouter is switched off | `phoenix autoroute run --execute` while `enabled = false`; nothing is started |
| PHOENIX-ROUTE-026 | The external autorouter could not be run | a program that is not a file, a temporary folder that is not there, a program that cannot be started or that ends with an error |
| PHOENIX-ROUTE-027 | The external autorouter ran out of time | a program that is still running after `timeout_s` seconds; it is ended |
| PHOENIX-ROUTE-028 | The external autorouter left no session | a program that ended well and wrote no session file at the place given |

## Script interpreter (`eh3d-phoenix-script`, `src/codes.rs` and `src/v2/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English and a message that is about a place of a script carries its line and column. Codes 027 to 041 are the causes and the differences that the compatibility report gives to a command that is pending or adapted (see `docs/SCRIPT.md`).

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SCRIPT-001 | Script settings are not valid | a line of `config/script.cfg` that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-SCRIPT-002 | A limit of the settings was exceeded | a script larger than `max_script_bytes`, with more than `max_lines` lines, `max_commands` commands, `max_args` arguments in a command, a text longer than `max_string_chars`, more than `max_operations` operations in a run, an output job with more than `max_cam_sections` sections |
| PHOENIX-SCRIPT-003 | Text in quotes is not closed | a single quote with no closing quote on the same line; the command is dropped and the next one is read |
| PHOENIX-SCRIPT-004 | Coordinate is not valid | a parenthesis that is not closed, nested, empty or that holds other than two numbers |
| PHOENIX-SCRIPT-005 | A command name was expected | a command that starts with a text in quotes or a coordinate instead of a word |
| PHOENIX-SCRIPT-006 | Unknown command | a word that is neither a command of the interpreter nor in `known_pending`; the script is not started |
| PHOENIX-SCRIPT-007 | Command argument is missing or not valid | a command with too few, too many or unexpected arguments |
| PHOENIX-SCRIPT-008 | Number or unit is not valid | a coordinate that is not two lengths, a length that does not fit the range of the model, a width, a drill or a grid that is not positive, a drill that is not smaller than its via |
| PHOENIX-SCRIPT-009 | Object not found | a component, a signal or a group name that the board does not have |
| PHOENIX-SCRIPT-010 | Layer not found | a layer name or number that the board (or the layer table of the settings) does not have, or no layer chosen yet |
| PHOENIX-SCRIPT-011 | The layer cannot hold this object | a wire on a layer that is neither copper nor silkscreen, a circle, a rectangle or a text off a silkscreen layer, a track moved to a layer that is not copper |
| PHOENIX-SCRIPT-012 | The board refused the operation | an operation that the board does not apply (a name already used, a part that is not there); the run is dropped |
| PHOENIX-SCRIPT-013 | File path is not allowed | a path that is absolute, has a drive, `..` or a link, lies outside `allowed_dirs`, a file command that its `allow_*` setting does not permit, or a file without the extension `.json` for a board |
| PHOENIX-SCRIPT-014 | File cannot be read or written | a file that cannot be opened, is larger than its limit, is not UTF-8 text or not a board, or an output file that exists |
| PHOENIX-SCRIPT-015 | Scripts are nested too deeply | `SCRIPT` inside `SCRIPT` beyond `max_depth` |
| PHOENIX-SCRIPT-016 | The command does not apply to this document | a command that edits a board run on a schematic, or `ERC` run on a board |
| PHOENIX-SCRIPT-017 | No new identity could be made | the generator of ids failed or every id it tried was in use |
| PHOENIX-SCRIPT-018 | Rotation is not supported here | a text turned by an angle that is not a multiple of 90 degrees |
| PHOENIX-SCRIPT-019 | This form of the command is not implemented yet | a form of a command that is understood but not carried out, as a cause in the report (`GRID ALT`, `CHANGE FONT`) |
| PHOENIX-SCRIPT-020 | Command option is not valid | an option that does not exist or lacks its value, a language that is not one of the eight, the wrong number of files, `--out` with a schematic |
| PHOENIX-SCRIPT-021 | The input cannot be read | a script, a board, a schematic, a rule file or a settings file that cannot be opened, is too large or is not valid |
| PHOENIX-SCRIPT-022 | Output file cannot be written | an output file that cannot be created |
| PHOENIX-SCRIPT-023 | The output job cannot be read | an output job larger than `max_cam_bytes`, with a zero byte, a section header that is not closed or no section with a `Device` key |
| PHOENIX-SCRIPT-024 | The output device has no equivalent | a section of an output job whose device is neither a Gerber RS-274X nor an Excellon device |
| PHOENIX-SCRIPT-025 | An option of the output job was not translated | a key of a section that the profile cannot hold, a second Gerber extension, a third drill file, a file name that is not plain |
| PHOENIX-SCRIPT-026 | Several layers in one file are written as one file per layer | a Gerber section that lists more than one layer |
| PHOENIX-SCRIPT-027 | Known command without an equivalent yet | a word of `known_pending`: the command exists in the original language and has no equivalent in this tool |
| PHOENIX-SCRIPT-028 | Selecting by pointing at a coordinate is not implemented | `MOVE`, `ROTATE`, `DELETE`, `CHANGE` or `GROUP` with a coordinate in place of a name |
| PHOENIX-SCRIPT-029 | Curved wires are not implemented | a number or an `@` between two points of `WIRE` |
| PHOENIX-SCRIPT-030 | The board has no operation to create a layer | `LAYER` with a number and a name |
| PHOENIX-SCRIPT-031 | Mirrored placement is not implemented | `ROTATE` with a mirrored rotation (`MR90`) |
| PHOENIX-SCRIPT-032 | This option of SET is not implemented | an option of `SET` that is not in `set_ignored` |
| PHOENIX-SCRIPT-033 | Only recorded: the display is not part of the board | `DISPLAY`, and `GRID LINES` or `GRID DOTS` (adapted) |
| PHOENIX-SCRIPT-034 | Delegated to the engine of this tool, not compared with the original tool | `RATSNEST`, `DRC` and `ERC` (adapted) |
| PHOENIX-SCRIPT-035 | Drawn as an outline stroke on a silkscreen layer | `CIRCLE` and `RECT` (adapted) |
| PHOENIX-SCRIPT-036 | Text uses the built-in stroke font | `TEXT` (adapted) |
| PHOENIX-SCRIPT-037 | Accepted and ignored: it only changes how the original tool looks or behaves | an option of `SET` listed in `set_ignored`, a line style of `WIRE` (adapted) |
| PHOENIX-SCRIPT-038 | Only relative paths allowed by the settings; boards are JSON files | `WRITE` and `OPEN` (adapted) |
| PHOENIX-SCRIPT-039 | The included script is read at run time and the report does not follow it | `SCRIPT` (adapted) |
| PHOENIX-SCRIPT-040 | Selects objects by name, not by a rectangle | `GROUP` (adapted) |
| PHOENIX-SCRIPT-041 | The shape is ignored: vias are round and join the outer copper layers | `VIA` with a shape word (adapted) |
| PHOENIX-SCRIPT-042 | A limit of the folder walk was reached | more scripts than `max_files` or folders deeper than `max_walk_depth` in a report |

Codes 043 to 064 belong to the scripts of the second version (`phoenix-script/2`, `docs/SCRIPT_API.md`, `eh3d-phoenix-script`, `src/v2/codes.rs`); their titles are in the eight languages too, and every message names the line and the column of the statement or of the part of the expression that caused it.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SCRIPT-043 | Syntax error in the script | a character, a number, a unit or a construction that is not part of the language; a text not closed; a bracket not closed; `break` or `continue` outside a loop; a statement that does not end where it should |
| PHOENIX-SCRIPT-044 | The script does not start with the line phoenix-script/2 | an empty script, another version in the header, a word after the version that is not `board`, `schematic` or `library`, or one that comes twice |
| PHOENIX-SCRIPT-045 | Variable is not defined | a name used, or assigned to, before it is defined in this block or in one around it |
| PHOENIX-SCRIPT-046 | Variable is already defined in this block | `let` of a name that the same block already has |
| PHOENIX-SCRIPT-047 | Unknown function | a call of a name that is not in the table of functions (there is no function that reaches the network, a program or a file, so such a name is this error) |
| PHOENIX-SCRIPT-048 | Wrong number of arguments | a call with fewer or more arguments than the function takes |
| PHOENIX-SCRIPT-049 | A value has the wrong type | an operator or a function given a type it does not take (`1 + "a"`, a condition that is not a bool, an angle that is not a multiple of 90deg for a part of a schematic, an unknown kind) |
| PHOENIX-SCRIPT-050 | Division by zero or number out of range | `/` or `%` by zero, a result beyond 64 bits, a place outside the range of the model, a count or a width out of its range |
| PHOENIX-SCRIPT-051 | The script took more steps than the limit | more than `max_steps` steps of the interpreter |
| PHOENIX-SCRIPT-052 | A loop turned more times than the limit | a loop with more than `max_loop_iterations` turns |
| PHOENIX-SCRIPT-053 | The values of the script grew beyond the limit | variables over `max_value_cells`, a list over `max_list_len`, a text over `max_text_bytes`, or lists nested deeper than 8 |
| PHOENIX-SCRIPT-054 | The output grew beyond the limit | `print` beyond `max_output_bytes` |
| PHOENIX-SCRIPT-055 | Blocks or expressions are nested too deeply | blocks deeper than `max_block_depth` or an expression deeper than `max_expr_depth` |
| PHOENIX-SCRIPT-056 | Data file path is not allowed | `read_text` or `write_text` with an absolute path, a drive, `..`, a link on the way or a path outside `data_dir`, or with no `data_dir` in the settings |
| PHOENIX-SCRIPT-057 | Data file cannot be used or a file limit was reached | a data file that is missing, too large (`max_data_bytes`), not UTF-8 or that exists when it should be new, or more than `max_data_files` files in a run |
| PHOENIX-SCRIPT-058 | The function does not apply to this document | a function called on a kind of document it does not apply to, or a script written for one kind (`board`, `schematic`, `library`) run on another |
| PHOENIX-SCRIPT-059 | Object not found | a designator, a net, a layer, a footprint, a pad or an object that is not (or no longer) in the document |
| PHOENIX-SCRIPT-060 | Assertion failed | `assert` with a false condition, or `fail` |
| PHOENIX-SCRIPT-061 | The editor refused the operation | an operation that the model refuses (a name taken, a layer that is not copper, a footprint in use, ...); nothing of the script is kept |
| PHOENIX-SCRIPT-062 | Index out of range | `list[i]` or `text[i]` outside the list or the text |
| PHOENIX-SCRIPT-063 | No parts base was given or the part is not in it | `assign_part` without `--parts`, with a part that is not in the base or that has no value |
| PHOENIX-SCRIPT-064 | The estimated steps are over the limit | warning of `phoenix script check`: the estimate of the steps of the script is above `max_steps` |

## RF structure generator (`eh3d-phoenix-rfgen`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English. The last three are warnings in the report of a structure; the others are errors.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-RFGEN-001 | RF generator settings are not valid | a line of `config/rfgen.cfg` that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-RFGEN-002 | A parameter is missing, unknown or out of range | a parameter the type of structure does not take, one that is not a finite number or not above zero, a needed parameter that is missing, a rotation that is not a multiple of 90 degrees, a length or arm that does not fit |
| PHOENIX-RFGEN-003 | Unknown type of structure | a type that is not in `phoenix rfgen --list` |
| PHOENIX-RFGEN-004 | The stack-up cannot carry a microstrip on this layer | a layer that is missing or not copper, an inner layer, a layer with no plane under its dielectric, a frequency or stack-up the stack-up crate refuses |
| PHOENIX-RFGEN-005 | A track width could not be found for the impedance | an impedance outside what the widths of `config/stackup.cfg` reach |
| PHOENIX-RFGEN-006 | The design cannot be built in microstrip | a stub shorter than the fringing field of its open end, a series element that a line cannot stand for, equal impedances for a transformer, an arm too short for its pitch, a fence that does not fit, a feed impedance above the edge resistance of a patch |
| PHOENIX-RFGEN-007 | The search for the width and gap of a coupled pair did not converge | even and odd impedances the model of coupled microstrips cannot give, or the iteration limit |
| PHOENIX-RFGEN-008 | The board has no copper layer of that name | a board whose copper layers are not those of the stack-up |
| PHOENIX-RFGEN-009 | A coordinate or a length is out of range | a length or a placement that does not fit the checked range of the model |
| PHOENIX-RFGEN-010 | The board refused an operation | an operation of the structure that the board does not apply (an id in use, a part that is not there) |
| PHOENIX-RFGEN-011 | The rules find a new error in the result | the rule check of the board with the structure finds more errors than before while `rules_gate` is on |
| PHOENIX-RFGEN-012 | No free id could be made | `id_attempts` ids were tried and all were in use |
| PHOENIX-RFGEN-013 | A command option is not valid | an option that needs a value or a number, an unknown language code, an output file that does not end in `.json` or `.brd` |
| PHOENIX-RFGEN-014 | The output file exists or cannot be written | an output file that already exists (an input file is never replaced) or cannot be created |
| PHOENIX-RFGEN-015 | The stack-up or the board cannot be read | a stack-up, board or settings file that cannot be opened, is too large or is not valid |
| PHOENIX-RFGEN-016 | A limit was exceeded | a structure with more items than `max_items` |
| PHOENIX-RFGEN-017 | A value is outside the range of a formula | warning: an input of a formula beyond the range the program checks for it |
| PHOENIX-RFGEN-018 | A shape is smaller than the minimum | warning: a copper shape narrower than `min_feature_nm` |
| PHOENIX-RFGEN-019 | The result rests on an approximation | warning: a discontinuity that is not modelled, a short-circuited stub that grounds the net, a resistor or a connector land pattern that is not placed |

## Zone filling (`eh3d-phoenix-zones`, `src/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-ZONE-001 | Zone settings are not valid | a line of `config/zones.cfg` that is not `key = value`, an unknown key, a number that cannot be read or a value out of range |
| PHOENIX-ZONE-002 | Zone rules file is not valid | a file of zone rules (`phoenix-zones/1`) that is not JSON, has another format or an unknown field, names a zone twice or has a negative length |
| PHOENIX-ZONE-003 | Zone is not on the board | a zone named in the file of zone rules that the board does not have; nothing is filled |
| PHOENIX-ZONE-004 | Zone outline cannot be filled | an outline with fewer than three points, with no area, with sides that touch, that cannot be cut into convex pieces, or on a layer that is not copper; the zone gets no copper and the others are filled |
| PHOENIX-ZONE-005 | A limit of the fill was exceeded | an outline with more points than `max_outline_points`, or a zone cut into more pieces than `max_fragments`; the zone gets no copper |
| PHOENIX-ZONE-006 | Fill is closer to other copper than the clearance | check: a side of a polygon nearer than the clearance (less the tolerance of the cut points) to a pad, track or via of another net, to a pad or via that is not joined, or to the edge of the board, or copper of another net inside a polygon |
| PHOENIX-ZONE-007 | Fill lies outside the zone or the board | check: a polygon with a point beyond the outline of the zone or of the board |
| PHOENIX-ZONE-008 | Copper island narrower than the minimum width | fill: information, the island is dropped; check: error, an island whose convex hull is narrower than the minimum width |
| PHOENIX-ZONE-009 | Copper island smaller than the minimum area | fill: information, the island is dropped; check: error, an island with less area than the minimum |
| PHOENIX-ZONE-010 | Copper island not connected to its net | fill: information, the island is dropped unless `keep_unconnected_islands` is on; check: warning |
| PHOENIX-ZONE-011 | Thermal relief cannot be made | warning: spokes too wide for the ring around a pad, which is then joined solid |
| PHOENIX-ZONE-012 | Zones of different nets share a priority and overlap | warning: two zones of different nets on one layer with the same priority whose outlines overlap |
| PHOENIX-ZONE-013 | Arc kept away from with a safe margin | information: an arc of a track near the zone, kept away from as short pieces widened by their sagitta |
| PHOENIX-ZONE-014 | Zone has no copper after the fill | warning: nothing is left of the zone |
| PHOENIX-ZONE-015 | A command option is not valid | an option the command does not know or needs a value for, an unknown language code, `--fill` with `fill` or `--out` with `check`, a number of board files other than one |
| PHOENIX-ZONE-016 | The output file exists or cannot be written | an output file that already exists (an input file is never replaced) or cannot be created |
| PHOENIX-ZONE-017 | Board, rule or fill file cannot be read | a board, rule, zone or fill file that cannot be opened, is too large or is not valid |
| PHOENIX-ZONE-018 | A coordinate is out of range | check: a polygon of a fill file with a coordinate outside the design range |
| PHOENIX-ZONE-019 | The fill file does not match the board | check: a fill with another format or for a board of another name, a zone that is not on the board or whose net or layer changed, a polygon with fewer than three points |

## Lengths and pairs (`eh3d-phoenix-lengths`, `src/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-LEN-001 | Length settings are not valid | a line of `config/lengths.cfg` that is not `key = value`, an unknown key, a number that cannot be read or a value out of range |
| PHOENIX-LEN-002 | Length groups file is not valid | a file of groups and pairs (`phoenix-lengths/1`) that is not JSON, has another format or an unknown field, a name used twice or empty, a group with too few nets, a pair with one net twice, a negative length |
| PHOENIX-LEN-003 | Net is not on the board | error: a net named in a group or a pair that the board does not have; the group or pair is left out of the results |
| PHOENIX-LEN-004 | Nets of a group differ in length by more than the tolerance | error: the longest and the shortest net of a group without a target differ by more than its tolerance |
| PHOENIX-LEN-005 | Pair differs in length by more than allowed | error: the two nets of a pair differ in length by more than `max_skew_nm` |
| PHOENIX-LEN-006 | Track width of a pair is not the width asked for | error: a piece of track of either net whose width is not `width_nm` within the tolerance |
| PHOENIX-LEN-007 | Pair is closer together than the separation asked for | error: the gap between the edges of coupled pieces is smaller than `gap_nm` less the tolerance |
| PHOENIX-LEN-008 | Pair is farther apart than the separation asked for | error: the gap between the edges of coupled pieces is larger than `gap_nm` plus the tolerance |
| PHOENIX-LEN-009 | Pair runs uncoupled for more than allowed | error: more of the positive net runs farther than the coupling range from the negative net than `max_uncoupled_nm` |
| PHOENIX-LEN-010 | Delay could not be worked out from the stack-up | warning: a layer the stack-up does not name, a layer the line model cannot take (a stripline without planes on both sides) or a width it refuses; the default delay is used |
| PHOENIX-LEN-011 | Layer has no delay of its own, the default is used | information: no stack-up was given or `use_stackup` is off and the layer has no `layer_delay_ps_per_mm` line |
| PHOENIX-LEN-012 | Length of an arc is an estimate | information: arc pieces are measured with floating-point trigonometry and rounded |
| PHOENIX-LEN-013 | Net has no copper | warning: a net named in a group or a pair that has no track and no via; its length is zero |
| PHOENIX-LEN-014 | A command option is not valid | an option the command does not know or needs a value for, an unknown language code, a number of board files other than one |
| PHOENIX-LEN-015 | The output file exists or cannot be written | an output file that already exists (an input file is never replaced) or cannot be created |
| PHOENIX-LEN-016 | Board, stack-up or lengths file cannot be read | a board, stack-up job, settings or groups file that cannot be opened, is too large or is not valid |
| PHOENIX-LEN-017 | A limit of the length report was exceeded | more tracks and vias, or more groups and pairs, than `max_items` |
| PHOENIX-LEN-018 | Net is not the target length | error: a net of a group with a target whose length is farther from the target than the tolerance |
| PHOENIX-LEN-019 | Pair has no coupled stretch | warning: no straight piece of the positive net runs within the coupling range of the negative net, so the separation is not judged |

## Signal integrity (`eh3d-phoenix-si`, `src/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SI-001 | Signal integrity settings are not valid | a line of `config/si.cfg` that is not `key = value`, an unknown key, a number that cannot be read or a value out of range, a warning limit above its error limit |
| PHOENIX-SI-002 | Net is not on the board | a net named with `--net` that the board does not have (error of the command); warning: the net has no copper |
| PHOENIX-SI-003 | Near-end crosstalk is above the limit | warning from `next_warn_pct`, error from `next_error_pct`: the estimate of the near-end crosstalk of an aggressor on a victim |
| PHOENIX-SI-004 | Far-end crosstalk is above the limit | warning from `fext_warn_pct`, error from `fext_error_pct`: the estimate of the far-end crosstalk, judged by its absolute value |
| PHOENIX-SI-005 | Coupling is too strong for the weak-coupling estimate | warning: the mutual inductance reaches more than `weak_coupling_limit` of the self inductance, so the crosstalk of the pair is not reliable |
| PHOENIX-SI-006 | Line is electrically long, so its reflections matter | information: the rise time of the driver is under twice the delay of the net |
| PHOENIX-SI-007 | Overshoot at the load is above the limit | warning from `overshoot_warn_pct`, error from `overshoot_error_pct`: the line as it is, with the best termination named |
| PHOENIX-SI-008 | Ringback below the final level is above the limit | warning from `ringback_warn_pct`: the voltage at the load falls below the final level after reaching it |
| PHOENIX-SI-009 | Stack-up cannot be used, the defaults of the settings are used | information: no stack-up was given; warning: a layer that the stack-up does not name, a stripline without planes on both sides, a width the model refuses or a dielectric without a known permittivity |
| PHOENIX-SI-010 | Eye height is below the limit | warning below `eye_height_warn`, error below half of it |
| PHOENIX-SI-011 | Eye width is below the limit | warning below `eye_width_warn`, error below half of it |
| PHOENIX-SI-012 | Track crosses the edge or a split of its reference plane | warning: the zones of the copper layers next to the track change along it (the edge of a plane or a split between two nets), or it runs over no copper of them |
| PHOENIX-SI-013 | Via stub is too long for the rise time | warning: a stub longer than `max_stub_nm` or whose round trip takes more than `stub_tr_fraction` of the rise time; error when its quarter-wave resonance is inside the bandwidth of the edge |
| PHOENIX-SI-014 | A command option is not valid | an option the command does not know or needs a value for, an unknown subcommand or language code, a number of board files other than one |
| PHOENIX-SI-015 | The output file exists or cannot be written | an output file that already exists (an input file is never replaced) or cannot be created |
| PHOENIX-SI-016 | Board, stack-up or settings file cannot be read | a board, stack-up job or settings file that cannot be opened, is too large or is not valid |
| PHOENIX-SI-017 | A limit of the analysis was exceeded | more tracks, vias or pieces of track than `max_items`, or more nets than `max_nets` |
| PHOENIX-SI-018 | Line loss slows the rise time by more than the limit | warning: the loss of the line at the bandwidth of the edge makes the rise time at the end grow by `rise_degradation_warn_pct` or more |
| PHOENIX-SI-019 | No reference plane found next to a layer | information: the board has no order of copper layers, or no zone on the layers next to a signal layer, so its tracks are not looked at over a plane |
| PHOENIX-SI-020 | Simulation was cut at the limit of steps | warning: a simulation of reflections needed more than `max_steps` steps and its result covers a shorter time |

## Field solver (`eh3d-phoenix-field2d`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`title` in `src/codes.rs`); the detail of a message is in English. The codes 009 to 013 that say "warning" go with a result; the others stop the line they belong to.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-FIELD-001 | Field solver settings are not valid | a line of `config/field2d.cfg` that is not `key = value`, an unknown key, a number that cannot be read or a value out of range |
| PHOENIX-FIELD-002 | The geometry of the cross section is not valid | a box or a rectangle with sides out of order or outside the box, a permittivity below one, a conductor with no electrode, lines of the mesh that do not increase, or a conductor number above the count |
| PHOENIX-FIELD-003 | The mesh has more cells than the limit | a section that needs more than `max_cells` cells |
| PHOENIX-FIELD-004 | The solver did not reach its tolerance | a solution that has not reached `tolerance` within `max_iterations`; no number is given |
| PHOENIX-FIELD-005 | The line cannot be taken from the stack-up | a layer that does not exist or is not copper, a missing material, no plane under a microstrip, a stripline without a plane on both sides, a missing gap, a width out of range, or a frequency outside the published points |
| PHOENIX-FIELD-006 | A command option is not valid | an option that `phoenix field2d` does not know, an option without its value, a language that is not one of the eight, or the wrong number of files |
| PHOENIX-FIELD-007 | The output file exists or cannot be written | `--out` names a file that exists or cannot be created |
| PHOENIX-FIELD-008 | An input file cannot be read | a job, a board or a settings file that is missing, too large or not of the expected shape |
| PHOENIX-FIELD-009 | The field solver and the closed formula differ by more than the limit | warning: the impedance differs from the closed formula of `phoenix stackup` by more than `compare_warn_percent` |
| PHOENIX-FIELD-010 | The net has no track on the board | `--net` names a net with no track |
| PHOENIX-FIELD-011 | The solder mask over the line is left out of the model | warning: solder mask lies over the open side of a microstrip or a coplanar line |
| PHOENIX-FIELD-012 | The model is an approximation of the real structure | warning: a coplanar line without a plane, whose substrate is followed by air down to a wall at zero potential |
| PHOENIX-FIELD-013 | Two conductors are closer than one cell of the mesh | two conductors, or a conductor and a wall at zero potential, that share a node of the mesh |

## Gerber and Excellon import (`eh3d-phoenix-gerber`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English and names the line of the file when there is one.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-GBR-001 | Gerber import settings are not valid | a line of `config/gerber_import.cfg` that is not `key = value`, an unknown key, a number that cannot be read or a value out of range |
| PHOENIX-GBR-002 | Input file is larger than the limit | a Gerber, Excellon, settings or profile file with more bytes than `max_file_bytes` (or than the limit of a settings file); the file is not read |
| PHOENIX-GBR-003 | A limit on commands or objects was exceeded | more commands, apertures or tools, drawn objects, region points or macro statements than the limits of the settings, a command longer than the limit, or more files than `max_files` |
| PHOENIX-GBR-004 | Gerber syntax error | a block that does not end with `*`, an extended command not closed by `%`, a command or word that is not known, coordinates without an operation, an operation code that does not exist, a file with no command, or text that is neither Gerber nor Excellon |
| PHOENIX-GBR-005 | Coordinate format or units missing or not valid | `FS` or `MO` that cannot be read, a size or coordinate before the units or the format, incremental coordinates (`G91`, `FS...I`), or, as a warning in a drill file, units or digits that were assumed |
| PHOENIX-GBR-006 | Aperture not defined or not valid | an aperture used before it is defined, a definition with the wrong number of parameters or a size that is not a number, a number below D10, a polygon with other than 3 to 12 corners, or, as a warning, a line drawn with an aperture that is not round |
| PHOENIX-GBR-007 | Aperture macro not valid | an aperture that names a macro that was not defined, a macro without a name, or a primitive or an expression that cannot be evaluated (a variable without a value, a division by zero, a wrong number of parameters, a primitive code that is not known) |
| PHOENIX-GBR-008 | Arc not valid | a full circle in single quadrant mode, no centre of single quadrant mode that fits the arc, and, as a warning, an arc whose two radii differ by more than `arc_radius_tolerance_nm` |
| PHOENIX-GBR-009 | Region not valid | `G36` inside a region, `G37` without `G36`, and, as warnings, a contour that is not closed, a contour with fewer than three corners, a flash inside a region or a file that ends inside a region |
| PHOENIX-GBR-010 | Feature not supported and skipped | warning: step and repeat, aperture blocks, aperture transforms, negative image polarity, an obsolete image command, a thermal or moire primitive of a macro, arcs or repeats in a drill file; the layer is marked incomplete |
| PHOENIX-GBR-011 | Coordinate out of range | a coordinate or size with an absolute value above `coordinate_limit_nm` |
| PHOENIX-GBR-012 | Excellon syntax error | a drill line or word that is not known, a `METRIC` or `INCH` line with a part that is not `LZ`, `TZ` or a format, a coordinate that is not a number, a file with no line |
| PHOENIX-GBR-013 | Drill tool not defined or not valid | a tool used before it is defined, a tool without a number, a diameter that is not positive, a hole, slot or route cut with no tool |
| PHOENIX-GBR-014 | Layer role not recognised | information: neither the attributes nor the name of a file tell its layer; warning: the name says a drill file and the content is Gerber, or the reverse |
| PHOENIX-GBR-015 | File could not be read | a file, folder, settings file or profile file that cannot be opened or listed |
| PHOENIX-GBR-016 | Imported files differ from the board | the round trip: the files that the export wrote differ from the board in the number or the positions of flashes, regions, lines or arcs, in the outline, in the number, diameters or positions of holes, in the units, or in a file that cannot be read back or is missing |
| PHOENIX-GBR-017 | File ends without an end command | warning: no `M02` in a Gerber file or no `M30` in a drill file; what was read is kept |
| PHOENIX-GBR-018 | Command option not valid | an option of `phoenix gerber-import` that the command does not know or needs a value for, an unknown language code, files together with `--roundtrip`, `--profile` without `--roundtrip`, a profile that is not in the profile file, an output file that exists or is an input |
| PHOENIX-GBR-019 | Export for the round trip failed | the export of the board with the profile refused it, so there is nothing to read back |

## Gerber X2 check (`eh3d-phoenix-gerber`, `src/x2check.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/x2check.rs`); the command is `phoenix gerber-check` (`docs/GERBER_X2.md`). The check is a subset chosen for this project and no manufacturer's tool has been used to compare it.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-GX2-001 | End command missing or not last | error: no `M02` in a Gerber file or no `M30` in a drill file, or a command after it |
| PHOENIX-GX2-002 | Required file attribute missing | error: a file without `.FileFunction` (a plain X1 file has none) |
| PHOENIX-GX2-003 | Recommended file attribute missing | warning: `.GenerationSoftware`, `.Part` or, for copper, solder mask, legend and paste, `.FilePolarity` missing |
| PHOENIX-GX2-004 | Aperture or tool used without a definition | error: an aperture selected that no `AD` defined, an object drawn with no aperture selected, or a drill tool selected that no line defined |
| PHOENIX-GX2-005 | Attribute deleted that was not set | warning: a `TD` that names an attribute no `TA` or `TO` set, or a `TD` with nothing in force |
| PHOENIX-GX2-006 | Attribute name or value not valid | error: an attribute without a name, a standard name used with the wrong command, or the values of `.FileFunction`, `.FilePolarity`, `.Part`, `.ProjectId`, `.GenerationSoftware`, `.N`, `.P` or `.C` with the wrong shape; warning: a `.FileFunction` or `.AperFunction` this check does not know |
| PHOENIX-GX2-007 | Character not allowed in an attribute field | error: a character outside printable ASCII, or a backslash that does not begin a `\uXXXX` escape, in an attribute |
| PHOENIX-GX2-008 | Drawing command before the format and units | error: a coordinate or an operation before `FS` and `MO` |
| PHOENIX-GX2-009 | Region not opened or not closed | error: `G36` inside a region, `G37` without `G36`, or a region still open at the end |
| PHOENIX-GX2-010 | Aperture defined twice or numbered below 10 | error: an aperture defined again, or a number below `D10` |

## Panelization (`eh3d-phoenix-panel`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-PANEL-001 | Panel settings are not valid | a line of `config/panel.cfg` that is not `key = value`, an unknown key, a number that cannot be read, a value out of range, tabs or mouse bite holes that the settings themselves contradict, a turn that is not 0, 90, 180 or 270, a `place` line that is not `BOARD,X_nm,Y_nm,ROTATION_DEG,MIRROR`, a separator of the copy number with other characters than letters, digits, `_`, `-` and `.`, or `--columns` and `--rows` out of range |
| PHOENIX-PANEL-002 | Board outline cannot be used for a panel | an outline with fewer than three points, or whose box has no width or no height |
| PHOENIX-PANEL-003 | Panel too large or too many boards | more boards than `max_boards`, a side of the panel above `max_panel_nm`, a size that does not fit, or more than 99 drill diameters |
| PHOENIX-PANEL-004 | Tabs do not fit on the edge | `tabs_per_edge` tabs of `tab_width_nm` that are longer together than the edge of the board |
| PHOENIX-PANEL-005 | Mark does not fit in the frame | a fiducial or a tooling hole that is not entirely inside a rail of the frame |
| PHOENIX-PANEL-006 | Marks overlap | two fiducials or tooling holes whose circles overlap |
| PHOENIX-PANEL-007 | Command option not valid | an option of `phoenix panel` that the command does not know or needs a value for, an unknown language code, a number of board files other than one (or a board given both as an argument and with `--boards`, or a name missing in the list of `--boards`), an export profile that cannot be found |
| PHOENIX-PANEL-008 | Output file is not allowed | an output folder that exists (nothing is written then), or a folder or file that cannot be created |
| PHOENIX-PANEL-009 | Outline is not a rectangle | information: the box around the outline is used for the spacing of the boards |
| PHOENIX-PANEL-010 | Gap ignored for V-score | information: with `joint = v_score` the boards touch and `gap_nm` is not used |
| PHOENIX-PANEL-011 | Marks skipped: the panel has no frame | warning: fiducials or tooling holes were asked for and `frame = none`, so none was put on the panel |
| PHOENIX-PANEL-012 | Mouse bite holes do not fit the tab | `mouse_bite_holes` holes at `mouse_bite_pitch_nm` that take more than `tab_width_nm` |
| PHOENIX-PANEL-013 | Board list or placement is not valid | an empty list of boards, a board number in a `place` line that is not in the list, a rotation that is not 0, 90, 180 or 270, a position below zero, or some boards placed and others not |
| PHOENIX-PANEL-014 | Boards overlap or are too close | two boards of the panel whose boxes overlap or are closer than `gap_nm` |
| PHOENIX-PANEL-015 | V-score line too close to copper | a V-score line that passes closer than `v_score_clearance_nm` to a pad, a track, a via or a zone of a board |
| PHOENIX-PANEL-016 | V-score lines cannot separate the boards | with `joint = v_score`, two boards for which no line of the panel runs between them from edge to edge (the boards are not in a grid) |
| PHOENIX-PANEL-017 | Local fiducial does not fit on the board | `local_fiducial_count` marks that do not fit inside the box of a board at `local_fiducial_inset_nm` from its edges |
| PHOENIX-PANEL-018 | Boards of the panel have different layers | boards of a panel with another number of copper layers, or a layer that has no layer of the same kind and name in the first board |
| PHOENIX-PANEL-019 | Panel files differ from the plan when read back | the files of the panel read back with the importer of this suite do not hold the copies, the holes and the marks that the plan has |
| PHOENIX-PANEL-020 | Board is not joined to the panel | warning: with `joint = mouse_bites` a board with no tab on any side (no board or rail at the distance `gap_nm` from an edge) |
| PHOENIX-PANEL-021 | The layers of the panel could not be written | the export of the copper, solder mask, paste, legend and drill files of the copies refused the panel (the message of the export follows) |

## Bill of materials with costs (`eh3d-phoenix-bom`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-BOM-001 | Cost settings are not valid | a line of `config/costs.cfg` that is not `key = value`, an unknown key, a number that cannot be read or a value out of range |
| PHOENIX-BOM-002 | Input file is larger than the limit | a price, part data, schematic, stack-up or settings file with more bytes than `max_file_bytes` (or than the limit of a settings file); the file is not read |
| PHOENIX-BOM-003 | Price file is not valid | a CSV or JSON price file with a missing column, a row without a supplier, without a part number and a value, with a quantity break that is not a whole number from 1 or a price that is not a decimal, JSON that cannot be read, or more rows than `max_items` |
| PHOENIX-BOM-004 | File of part data is not valid | a CSV with no `match` column, an empty match, a `fitted` that is not yes or no, a match given twice, or more lines than `max_items` |
| PHOENIX-BOM-005 | Part has no price | warning: no row of the price file is for the part (by part number, or by value and footprint) in the currency of the run; the part costs nothing in the totals and the totals say they are not complete |
| PHOENIX-BOM-006 | Part has no footprint | warning: a part whose footprint has no name, or that the schematic gives no footprint |
| PHOENIX-BOM-007 | Price in another currency was ignored | warning: rows of the price file in a currency other than the one of the settings |
| PHOENIX-BOM-008 | Quantity to buy was raised to the minimum of the supplier | information: the quantity of the batch is below the first quantity break of the supplier chosen, so that quantity is bought |
| PHOENIX-BOM-009 | Part that is not fitted is left out | information: a reference marked `no` in the file of part data |
| PHOENIX-BOM-010 | Component refers to a footprint that is not on the board | error: the footprint of a component is not one of the footprints of the board |
| PHOENIX-BOM-011 | Part of the schematic is not placed on the board | warning: a part of the schematic given with `--schematic` whose reference is not on the board; it is added to the bill |
| PHOENIX-BOM-012 | Board cannot give a PCB cost | warning: an outline of fewer than three points or with no area, no copper layer, or a number of copper layers that has no tariff in the settings |
| PHOENIX-BOM-013 | Two prices for one quantity of one part disagree | warning: two rows of one supplier for the same part and the same quantity break with different prices; the lower is used |
| PHOENIX-BOM-014 | A command option is not valid | an option of `phoenix bom` that the command does not know or needs a value for, an unknown language code, `--boards` out of range, `--schematic` that is not a JSON file, a number of board files other than one |
| PHOENIX-BOM-015 | The output file exists or cannot be written | a file named by `--csv`, `--json-file` or `--html` that exists, is named twice or cannot be created (nothing is written then) |
| PHOENIX-BOM-016 | An input file cannot be read | the board, a price file, a file of part data, a schematic, a stack-up or a settings file that cannot be opened or read |
| PHOENIX-BOM-017 | A limit of the bill of materials was exceeded | more components than `max_items` |
| PHOENIX-BOM-018 | The PCB cost is an estimate from the tariffs of the settings | information, always given with a PCB cost: the tariffs are an example and not the prices of any maker |
| PHOENIX-BOM-019 | A quantity of the imported bill differs from its references | `phoenix bom --import`: a line states a quantity that is not the number of references in its cell (warning) |
| PHOENIX-BOM-020 | A reference is in two lines of the imported bill | `phoenix bom --import`: the first line that names it is used (warning) |
| PHOENIX-BOM-021 | The imported bill and the board disagree about a part | `phoenix bom --import`: a part on one side only, or a value that differs (warning); nothing is changed |

## Parts base (`eh3d-phoenix-parts`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English and gives the file and the line (CSV) or the part (JSON) where the fault is.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-PARTS-001 | Parts settings are not valid | a line of `config/parts.cfg` that is not `key = value`, an unknown key or a value out of range |
| PHOENIX-PARTS-002 | File of the parts base cannot be read | a file or folder that cannot be opened, a file larger than `max_file_bytes` or a file that is not UTF-8 text |
| PHOENIX-PARTS-003 | File of the parts base is not valid | JSON that does not parse, a key that is not known, a part without an id, a state that is not active, not-recommended or obsolete, a price or a quantity that is not a number, a CSV column that is not known, or a row whose cells disagree with the first row of the same part; also an imported base that has an error, which is not written |
| PHOENIX-PARTS-004 | A limit of the parts base was exceeded | more files, parts, offers or price breaks than the limits of `config/parts.cfg` |
| PHOENIX-PARTS-005 | Command option is not valid | a subcommand or an option of `phoenix parts` that is not known or lacks its value, or an output file of the wrong kind |
| PHOENIX-PARTS-006 | Output file exists or cannot be written | the file given with `--out` exists (it is never replaced), is an input file, or cannot be created |
| PHOENIX-PARTS-007 | Part id used twice | error: two parts of the base have the same id; the message gives the place of the first |
| PHOENIX-PARTS-008 | Footprint is not in the IPC or own library | warning: the footprint of a part is not a footprint of the IPC library or of the libraries given with `--lib` |
| PHOENIX-PARTS-009 | Same part number with different data | error: two parts have the same part number (without case) and another value, footprint, maker, tolerance, power or voltage |
| PHOENIX-PARTS-010 | Price has no currency | warning: a price break without a currency |
| PHOENIX-PARTS-011 | Price breaks are not in ascending order | warning: the quantities of the price breaks of an offer do not rise |
| PHOENIX-PARTS-012 | Simulation model is not in the table of models | warning: the model of a part is not a `.model` line of the table of models |
| PHOENIX-PARTS-013 | Alternative names a part that is not in the base | warning: an alternative that is neither an id nor a part number of the base |
| PHOENIX-PARTS-014 | Part has no match in the parts base | warning: a part of the design for which `phoenix bom --parts-db` found no part, or whose given part number is not in the base |
| PHOENIX-PARTS-015 | Part matches several parts of the base | warning: several parts of the base fit the value and footprint; none is taken |
| PHOENIX-PARTS-016 | Part is obsolete or not recommended | warning: the part found in the base is obsolete or not recommended; the report proposes alternatives and changes nothing |
| PHOENIX-PARTS-017 | Part is not in the parts base | `phoenix parts show` was given an id that is not in the base |
| PHOENIX-PARTS-018 | The part cannot be assigned to the symbol | the plan of the schematic editor refuses to assign a part of the base: the symbol is not a part of the schematic, the selection is empty or the part has no value |

## Pick and place files (`eh3d-phoenix-export`, `src/fabcodes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-PNP-001 | Pick and place settings are not valid | a line of `config/pnp_rotations.cfg` that is not `key = value`, an unknown key, a section given twice, a value that cannot be read or is out of range, a maker that is not a section |
| PHOENIX-PNP-002 | Component has no footprint on the board | warning: a component that names a footprint the board does not have; its origin is used as its centre |
| PHOENIX-PNP-003 | Footprint has no pads, the origin is the centre | information: a footprint without pads |
| PHOENIX-PNP-004 | Reference designator is used twice | warning: two components with the same designator (one finding for each designator) |
| PHOENIX-PNP-005 | Component has no reference designator | warning: a component whose designator is empty |
| PHOENIX-PNP-006 | Part is not fitted | information: a part that the file of part data or `--dnp` says is not fitted, left out of the lines or marked |
| PHOENIX-PNP-007 | Part is outside the board outline | warning: the centre of a part is outside the box of the outline |
| PHOENIX-PNP-008 | Option is not valid | an option of `phoenix pnp` that is unknown, has no value or a value out of its set |
| PHOENIX-PNP-009 | Output file exists or cannot be written | the folder named by `--out` exists or cannot be created (nothing is written then) |
| PHOENIX-PNP-010 | Rotation corrected for the machine | information: the number of parts whose footprint matched a `rotate` line with an extra rotation |
| PHOENIX-PNP-011 | Board has no components | warning: a board with no placed component |

## Solder paste stencil (`eh3d-phoenix-export`, `src/fabcodes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-STENCIL-001 | Stencil settings are not valid | a line of `config/stencil.cfg` that is not `key = value`, an unknown key, a reduction that is neither a per cent nor a length, a value that cannot be read or is out of range |
| PHOENIX-STENCIL-002 | Paste layer has no side | warning: a paste layer that does not say its face; its pads are left out |
| PHOENIX-STENCIL-003 | Opening is below the minimum aspect ratio | warning: the smaller side of an opening over the thickness of the plate is below `min_aspect` (one finding for each pad) |
| PHOENIX-STENCIL-004 | Opening is below the minimum area ratio | warning: the area of an opening over the area of its walls is below `min_area_ratio` (one finding for each pad) |
| PHOENIX-STENCIL-005 | Large pad split into windows | information: a pad larger than `window_min_area_mm2` written as a grid of windows with bridges |
| PHOENIX-STENCIL-006 | Rotated pad written as a region | information: an opening turned by an angle that is not a right angle, written as a region and not as a flash |
| PHOENIX-STENCIL-007 | Board has no paste openings | warning: no pad is on a paste layer, so no file is made |
| PHOENIX-STENCIL-008 | Option is not valid | an option of `phoenix stencil` that is unknown, has no value or a value out of its set |
| PHOENIX-STENCIL-009 | Output file exists or cannot be written | the folder named by `--out` exists or cannot be created (nothing is written then) |
| PHOENIX-STENCIL-010 | Reduction took an opening away | warning: after the reduction a side of the opening is smaller than `min_feature_mm`; the opening is in the report and not in the file |
| PHOENIX-STENCIL-011 | Pad is too small to be split into windows | information: a pad over the area limit whose windows would be smaller than `min_feature_mm`; it stays whole |

## Fabrication package (`eh3d-phoenix-export`, `src/fabcodes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-FABPACK-001 | Export profile is not valid or not found | the profile asked for with `--profile` that is not in the profile file, or a profile file that is not valid |
| PHOENIX-FABPACK-002 | Package could not be assembled | a file name that is not a plain relative path, a name given twice, a name that is the manifest, or an export that failed |
| PHOENIX-FABPACK-003 | Manifest read back differs from the files | the package read back has a file whose size or SHA-256 differs from the manifest, a file the manifest does not list, a listed file that is missing, or a zip file that cannot be read |
| PHOENIX-FABPACK-004 | Output exists or cannot be written | the destination named by `--out` exists or cannot be created (nothing is written then) |
| PHOENIX-FABPACK-005 | Option is not valid | an option of `phoenix fabpack` that is unknown, has no value or a value out of its set |
| PHOENIX-FABPACK-006 | File or package is too large for the package format | more than 65535 files, or a file or the package of 4 GiB or more |
| PHOENIX-FABPACK-007 | What the files of the package cannot say | information, always given: the number of notes that the manifest carries (the standing notes and the warnings of the run) |
| PHOENIX-FABPACK-008 | Input file cannot be read | the board, a settings file, a file of part data or prices or the profile file that cannot be opened or read |

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/fabcodes.rs`); the detail of a message is in English.

## 3D assembly (`eh3d-phoenix-assembly3d`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-3D-001 | Assembly settings are not valid | a line of `config/assembly3d.cfg` that is not `key = value`, an unknown key, a number that cannot be read or a value out of range |
| PHOENIX-3D-002 | Input file is larger than the limit | a model, table, stack-up or settings file with more bytes than `max_file_bytes` (or than the limit of a settings file); the file is not read |
| PHOENIX-3D-003 | STEP file is not valid | a file that does not start with `ISO-10303-21` or has no `DATA` section, an entity that is not written as `#n=NAME(...)`, an entity number given twice, a point that is not three numbers or a number that cannot be read |
| PHOENIX-3D-004 | Length unit of the STEP file is not supported | a file with no length unit, a unit with a prefix other than milli, centi, deci, micro or none, or a unit based on a conversion that is not the inch |
| PHOENIX-3D-005 | STEP solid replaced by its enclosing box | warning: a file whose solid is not a plain box; the detail says what it is (straight edges and plane faces, circles and cylinders too, or something this reader does not know, for which the box of every point of the file is used and may not enclose the solid) |
| PHOENIX-3D-006 | STEP file has no geometry | a file with no vertex and no circle |
| PHOENIX-3D-007 | Footprint has no model | warning: a footprint that the table of models does not name, or a component whose footprint is not on the board; its components are left out of the scene and no model is made up |
| PHOENIX-3D-008 | Table of models is not valid | a CSV with no `footprint` or `model` column, an empty footprint, a model that is an absolute path, has `..` or is not `box:LxWxH` with sizes above 0, a shift that is not a number, a `rot_z_deg` that is not a multiple of 90, a footprint given twice, or more lines than `max_components` |
| PHOENIX-3D-009 | Model file cannot be read | a STEP file named by the table that cannot be opened; its components are left out |
| PHOENIX-3D-010 | Components collide | error: the boxes of two components overlap by more than `min_overlap_nm` in the board plane and in height |
| PHOENIX-3D-011 | Component is taller than the limit | error: a component stands higher above the board (or hangs lower below it) than `max_component_height_nm` |
| PHOENIX-3D-012 | Component is not inside the board outline | warning: a corner of the box of a component is outside the outline of the board (with `check_outline` on) |
| PHOENIX-3D-013 | Board thickness is not known, the default is used | information: no stack-up was given with `--stackup`, so `default_board_thickness_nm` is used |
| PHOENIX-3D-014 | A command option is not valid | an option of `phoenix assembly3d` that the command does not know or needs a value for, an unknown language code, no `--models`, a number of board files other than one |
| PHOENIX-3D-015 | The output file exists or cannot be written | a file named by `--obj`, `--step` or `--json-file` that exists, is named twice or cannot be created (nothing is written then) |
| PHOENIX-3D-016 | An input file cannot be read | the board, the table of models, a stack-up or a settings file that cannot be opened or read |
| PHOENIX-3D-017 | A limit of the assembly was exceeded | more components than `max_components`, more entities in a STEP file than `max_step_entities`, or more findings of one kind than `max_findings` (the rest are counted, not listed) |
| PHOENIX-3D-018 | The export is a simplified model | information: the OBJ and STEP files hold boxes and the prism of the board, not the copper, the pads, the holes or the real shape of the parts |

## DXF (`eh3d-phoenix-dxf`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English. A finding is information, a warning or an error; the reader and the chaining list what they leave out and never drop it silently.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-DXF-001 | DXF settings are not valid | a line of `config/dxf.cfg` that is not `key = value`, an unknown key, a layer name with a character DXF forbids, two layers with the same name or a value out of range |
| PHOENIX-DXF-002 | A limit of the DXF reader was exceeded | a file with more bytes than `max_file_bytes`, more entities (once the blocks are expanded) than `max_entities`, more blocks than `max_blocks` or blocks nested deeper than `max_block_depth`; the file is refused; also a drawing that makes more closed contours than `max_contours` (an error of the report, the outline is not looked for) |
| PHOENIX-DXF-003 | DXF file is not valid | an empty file, an odd number of lines, a group code that is not a number, a coordinate that is not a number, a section with no name, a file with no section or with no `ENTITIES` section; the message names the line |
| PHOENIX-DXF-004 | DXF entity is not understood and was left out | warning: an entity of a type the reader does not know (ellipse, text, hatch, dimension, mesh and so on), a spline that cannot be evaluated, a polyline with fewer than two vertices or a coordinate out of the range of the design; the detail counts them and lists their layers |
| PHOENIX-DXF-005 | Drawing unit is not known, the default unit is used | information: a file with no `$INSUNITS` or with 0; warning: a number of units the reader does not know; the unit of `default_unit` is used |
| PHOENIX-DXF-006 | Spline was approximated by straight pieces | warning: a spline cut into pieces within `spline_tolerance_nm`, or only joined through its fit points (then the curve between the points is not the spline) |
| PHOENIX-DXF-007 | Block or insert cannot be used | warning: an insert of a block that is not defined or that holds itself, with a scale of 0 or in an array of rows and columns, and an arc or a circle in a block scaled differently along x and y |
| PHOENIX-DXF-008 | A command option is not valid | an option of `phoenix dxf-import` or `phoenix dxf-export` that the command does not know or needs a value for, an unknown language code, a tolerance out of its range, no board or no drawing |
| PHOENIX-DXF-009 | Output file exists or cannot be written | a file named by `--out` that exists, is an input, has not the extension the command writes or cannot be created; nothing is written |
| PHOENIX-DXF-010 | Input file cannot be read | the drawing, the board or a settings file that cannot be opened or read |
| PHOENIX-DXF-011 | Chain of lines and arcs is not closed | warning: pieces that join end to end (within `join_tolerance_nm`) and do not return to their start; they are not part of the outline |
| PHOENIX-DXF-012 | Duplicate entity was dropped | information: a piece that repeats another in either direction, a piece with no length or a contour that repeats another |
| PHOENIX-DXF-013 | Contour crosses itself or another contour | error: an edge of the outline crosses another edge of it or an edge of a cutout, or two cutouts cross; information when the check was skipped because the contour has more edges than `max_check_edges` |
| PHOENIX-DXF-014 | No closed contour was found | error: no chain of pieces, closed polyline or circle on the layer asked for (or in the whole drawing) closes |
| PHOENIX-DXF-015 | Cutouts inside the outline | information: a contour inside the outline is kept as a cutout of the board outline |
| PHOENIX-DXF-016 | More than one outer contour, the largest is used | warning: outer contours that are not inside another, besides the largest; they are left out with the cutouts of theirs |
| PHOENIX-DXF-017 | Contour is degenerate | warning: a closed chain with fewer than three corners or with no area; it is left out |
| PHOENIX-DXF-018 | Arcs were approximated by straight pieces | information: arcs and circles of the outline replaced by straight pieces within `arc_tolerance_nm` |
| PHOENIX-DXF-019 | The DXF holds outlines and marks only | information: the export writes the outlines of the pads, holes, lines and marks; copper, mask, paste, widths and the rotation of texts are not in the file |

## Enclosure check (`eh3d-phoenix-assembly3d`, `src/enclosure_codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/enclosure_codes.rs`); the detail of a message is in English. They come with the findings of the assembly (`PHOENIX-3D-NNN`) in the same report.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-ENCL-001 | Enclosure file is not valid | a line of the file of the enclosure that is not `key = value`, an unknown key, a box or a number that cannot be read or is out of range, a contour that is an absolute path or has `..`, both a box and a contour or neither, a contour with no height, more than 64 posts |
| PHOENIX-ENCL-002 | Input file of the enclosure check cannot be read | the board, the file of the enclosure, the table of models or the stack-up that cannot be opened or read |
| PHOENIX-ENCL-003 | An option of the enclosure command is not valid | an option of `phoenix enclosure` that the command does not know or needs a value for, an unknown language code, no `--enclosure` or a number of board files other than one |
| PHOENIX-ENCL-004 | Output file of the enclosure check exists or cannot be written | the file named by `--step` exists or cannot be created; nothing is written |
| PHOENIX-ENCL-005 | Board or component is outside the walls or too close to them | error: the board or the box of a component is outside the walls or crosses one; warning: it is nearer to a wall than `min_clearance_mm` |
| PHOENIX-ENCL-006 | Board or component is through the lid or too close to it | error: the top of the board or of a component on the top side is higher than the lid; warning: it is nearer to it than `min_clearance_mm` |
| PHOENIX-ENCL-007 | Board or component is through the floor or too close to it | error: the board or a component on the bottom side is lower than the floor; warning: it is nearer to it than `min_clearance_mm` |
| PHOENIX-ENCL-008 | Component collides with a screw post | error: the box of a component enters the cylinder of a post by more than `min_overlap_nm` in the board plane and in height |
| PHOENIX-ENCL-009 | Screw post has no hole in the board | warning: no hole of the board (a via or the drill of a pad) has its centre within `post_hole_tolerance_mm` of the post, or the hole is smaller than the one the screw needs |
| PHOENIX-ENCL-010 | Screw post is outside the board outline | warning: the centre of a post is not under the board |
| PHOENIX-ENCL-011 | Least clearances to the enclosure | information: the least clearance to the walls, the lid, the floor and the posts, always reported (a negative number is through) |
| PHOENIX-ENCL-012 | Hole left out of the STEP solid | warning: a hole of the board whose centre is outside the outline, that touches an edge or that overlaps a hole before it |
| PHOENIX-ENCL-013 | The STEP solid of the board is simplified | information: the solid is the prism of the outline with the holes through it; copper, pads, components and the real shape of the edges are not in it |
| PHOENIX-ENCL-014 | Inner contour of the enclosure cannot be used | the drawing named by `contour` that cannot be read, has no closed contour on the layer or has a contour that crosses itself; the detail gives the code of the drawing |

## Spatial index (`eh3d-phoenix-spatial`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SPATIAL-001 | Spatial index settings are not valid | a line of `config/spatial.cfg` that is not `key = value`, an unknown key, a key written twice, a value out of range or a file larger than 64 KiB |
| PHOENIX-SPATIAL-002 | The spatial index would need more cells than the limit | boxes that cover more cells, added up, than `max_cells`; the index is not built, the copper clearance check compares every pair instead and `phoenix perf` exits with 1 |
| PHOENIX-SPATIAL-003 | There are more items than the spatial index accepts | more boxes than `max_items`; the index is not built |
| PHOENIX-SPATIAL-004 | A command option is not valid | an option of `phoenix perf` that the command does not know or needs a value for, a clearance out of its range, an unknown language code, no board or more than one |
| PHOENIX-SPATIAL-005 | Input file cannot be read | the board or a settings file that cannot be opened or read |

## Autorouter (`eh3d-phoenix-autoroute`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English. The codes `011` to `015` are the reasons that a connection is left unrouted and are in the report of the plan; the others stop the run.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-AROUTE-001 | Autorouter settings are not valid | a line of `config/autoroute_native.cfg` that is not `key = value`, an unknown key, a key written twice, a value out of range or a file larger than 64 KiB |
| PHOENIX-AROUTE-002 | A command option is not valid | an option of `phoenix autoroute native` that the command does not know or needs a value for, or an unknown language code |
| PHOENIX-AROUTE-003 | Input file cannot be read | a settings file that cannot be opened or read |
| PHOENIX-AROUTE-004 | A net named in the request is not on the board | a name given to `--nets` that no net of the board has |
| PHOENIX-AROUTE-005 | A layer named in the request is not a copper layer of the board | a name given to `--layers` that no copper layer of the board has |
| PHOENIX-AROUTE-006 | The routing grid would have more nodes than the limit | a board whose outline, at the pitch of `grid_nm`, needs more nodes than `max_nodes` (nodes of one layer times layers), or an index of obstacles that would be too large |
| PHOENIX-AROUTE-007 | The board has no closed outline or no copper layer to route on | a board with fewer than three corners in its outline or with no copper layer |
| PHOENIX-AROUTE-008 | There are more connections to route than the limit | more open connections than `max_connections`, or a ratsnest that compared more pairs of points than its limit |
| PHOENIX-AROUTE-009 | The result cannot be written | the board with the routes that cannot be turned into bytes or written to a new file |
| PHOENIX-AROUTE-010 | The board refused the operations of the plan | an operation of the plan that the board does not accept |
| PHOENIX-AROUTE-011 | No free place near a pad to start or end a track | a connection that is not routed because no node near one of its ends is clear, or the end is on no layer that may be used |
| PHOENIX-AROUTE-012 | No path exists on the routing grid | a connection that is not routed because every node that can be reached was expanded and none is an end |
| PHOENIX-AROUTE-013 | The search reached its limit of expanded nodes | a connection that is not routed because the search reached `max_expansions_per_search` or `max_expansions_total` |
| PHOENIX-AROUTE-014 | Rip-up and reroute reached its limit | a connection that is not routed because `max_ripup_iterations` or `max_rips_per_connection` was reached |
| PHOENIX-AROUTE-015 | The routes of a net were refused because they add a rule error | a net whose routes add an error of the design rules that the board did not have; the net is left out of the plan |
| PHOENIX-AROUTE-016 | The plan leaves connections unrouted | `--require-complete` with a plan that leaves a connection of the nets asked for unrouted |
| PHOENIX-AROUTE-017 | A differential pair was routed as two ordinary nets | a pair asked for with `--pairs` that could not be routed as two coupled tracks (a net with more than one open connection, tracks of different widths, no layer shared by the four pads, nets that change sides, no room or a blocked track), a note of the report with the reason |
| PHOENIX-AROUTE-018 | A differential pair was routed across two layers with a via for each track | a pair coupled across two layers when `couple_via = true`: the two tracks change layer with one via each, side by side at the same relative position; a note of the report |
| PHOENIX-AROUTE-019 | An existing via off the grid was joined to the grid by a short piece | an existing via of the net within `via_snap_nm` of a node of the grid that the search used as a change of layer: a short piece of two right-angle steps joins the node to the via on both layers; a note of the report with the count |
| PHOENIX-AROUTE-020 | A net reached its own limit of expanded nodes | the searches for a net expanded `max_expansions_per_net` nodes (or the `net_max_expansions.NAME` of the net): the rest of its connections are left unrouted with this code and a note of the report names the net and the limit; the limits count nodes and never time |
| PHOENIX-AROUTE-021 | A weight or a limit names a net or a net class with no connection to route | a `net_weight.NAME`, `net_max_expansions.NAME` or `net_class_weight.CLASS` of `config/autoroute_native.cfg` whose net or class has no open connection in this run (a note of the report, the run goes on) |
| PHOENIX-AROUTE-022 | A differential pair was routed across several layers with a via for each track at every change | a pair coupled with more than one change of layer when `couple_via = true` and `couple_layers_max` is more than 1: at every change the two tracks have one via each, side by side; a note of the report with the number of changes |

## Printing (`eh3d-phoenix-print`, `src/codes.rs`)

The titles of these codes also exist in the eight languages of the suite (`TITLES` in `src/codes.rs`); the detail of a message is in English. Every code is an error except PHOENIX-PRINT-007, a warning.

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-PRINT-001 | Print settings are not valid | a line of `config/print.cfg` that is not `key = value`, an unknown key, a colour that is not `#rrggbb` or `#rrggbbaa`, a value out of range, or a margin and a title block that leave no room on the paper |
| PHOENIX-PRINT-002 | Paper size is not valid | a paper that is not `A4`, `A3`, `Letter` or `WIDTHxHEIGHT` with each side from 50 to 5000 mm, or a paper too small for the border and the title block |
| PHOENIX-PRINT-003 | The drawing does not fit the paper at that scale | a scale given in the settings or with `--scale` at which the drawing is larger than the room inside the border |
| PHOENIX-PRINT-004 | Nothing to print | a document without pages, or a choice of layers (`layers`, `--layers`) that holds nothing on a page |
| PHOENIX-PRINT-005 | The output may not be written there | an output that exists, a file name that does not end in the extension of the format, or several files asked for one file |
| PHOENIX-PRINT-006 | The file could not be written | a folder or a file that the system refuses to create |
| PHOENIX-PRINT-007 | Text drawn as boxes in the PDF (use SVG) | warning: characters that Helvetica and Courier cannot show (Japanese and Chinese text) are drawn as empty boxes in a PDF; the SVG shows them |
| PHOENIX-PRINT-008 | The PDF written failed its own check | a PDF whose table of offsets, lengths of streams, tree of pages or content the reader of the crate does not accept (a defect of the writer) |
| PHOENIX-PRINT-009 | Print option is not valid | an option of `phoenix print` that is unknown, has no value or a value that is not valid, or a document that is not of the kind asked |

## Reference cases (`eh3d-phoenix-oracle`, `src/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-ORACLE-001 | Oracle case file is not valid | a file of cases that is not JSON, has another format, an unknown field, an empty suite or title, or a case that is not valid |
| PHOENIX-ORACLE-002 | Oracle case is not valid | an id that is not `ORACLE-` followed by capitals, digits and dashes or is used twice, an empty source, a cited case without a check, a finite reference value, a tolerance or a note about it, a pending case without a reason or with a number |
| PHOENIX-ORACLE-003 | Oracle check is not known | a check name in a cited case that this program does not have |
| PHOENIX-ORACLE-004 | Input of an oracle case is missing or not valid | an input of a check that is missing, is not of the kind the check reads or has a value the check does not know |
| PHOENIX-ORACLE-005 | The calculation under test failed | the function under test returned an error, or its result is not a finite number |
| PHOENIX-ORACLE-006 | Result is outside the tolerance of the reference | a cited case whose result differs from the reference value by more than the tolerance |
| PHOENIX-ORACLE-007 | Case is pending | information: a case that has no value yet, with the reason it is pending |
| PHOENIX-ORACLE-008 | Case file or folder cannot be read | `--dir` that is not a folder, holds no `.json` file or has a file that cannot be read or is over 1 MiB |
| PHOENIX-ORACLE-009 | Command option is not valid | an option that is unknown, lacks its value or has a language code that is not one of the eight |
| PHOENIX-ORACLE-010 | Report file cannot be written | `--out` names a file that exists (an existing file is never replaced) or cannot be created |

The same titles in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese), as the program prints them:

| Code | en | es | de | fr | it | ja | zh | pt |
|---|---|---|---|---|---|---|---|---|
| PHOENIX-ORACLE-001 | Oracle case file is not valid | El archivo de casos del oráculo no es válido | Die Datei mit den Orakel-Fällen ist ungültig | Le fichier de cas de l'oracle n'est pas valide | Il file dei casi dell'oracolo non è valido | オラクルのケースファイルが無効です | 基准用例文件无效 | O ficheiro de casos do oráculo não é válido |
| PHOENIX-ORACLE-002 | Oracle case is not valid | El caso del oráculo no es válido | Der Orakel-Fall ist ungültig | Le cas de l'oracle n'est pas valide | Il caso dell'oracolo non è valido | オラクルのケースが無効です | 基准用例无效 | O caso do oráculo não é válido |
| PHOENIX-ORACLE-003 | Oracle check is not known | La comprobación del oráculo no se conoce | Die Orakel-Prüfung ist unbekannt | Le contrôle de l'oracle est inconnu | Il controllo dell'oracolo non è noto | オラクルのチェックが不明です | 未知的基准检查 | A verificação do oráculo não é conhecida |
| PHOENIX-ORACLE-004 | Input of an oracle case is missing or not valid | Falta un dato de entrada del caso o no es válido | Eine Eingabe des Falls fehlt oder ist ungültig | Une entrée du cas manque ou n'est pas valide | Un ingresso del caso manca o non è valido | ケースの入力が不足または無効です | 用例的输入缺失或无效 | Falta uma entrada do caso ou não é válida |
| PHOENIX-ORACLE-005 | The calculation under test failed | Falló el cálculo que se prueba | Die geprüfte Berechnung ist fehlgeschlagen | Le calcul testé a échoué | Il calcolo in prova è fallito | テスト対象の計算が失敗しました | 被测计算失败 | O cálculo em teste falhou |
| PHOENIX-ORACLE-006 | Result is outside the tolerance of the reference | El resultado está fuera de la tolerancia de la referencia | Das Ergebnis liegt außerhalb der Toleranz der Referenz | Le résultat est hors de la tolérance de la référence | Il risultato è fuori dalla tolleranza del riferimento | 結果が基準の許容範囲外です | 结果超出参考值的容差 | O resultado está fora da tolerância da referência |
| PHOENIX-ORACLE-007 | Case is pending | El caso está pendiente | Der Fall ist ausstehend | Le cas est en attente | Il caso è in sospeso | ケースは保留中です | 用例待处理 | O caso está pendente |
| PHOENIX-ORACLE-008 | Case file or folder cannot be read | No se puede leer el archivo o la carpeta de casos | Die Falldatei oder der Ordner kann nicht gelesen werden | Le fichier ou le dossier de cas ne peut pas être lu | Il file o la cartella dei casi non può essere letto | ケースのファイルまたはフォルダを読み込めません | 无法读取用例文件或文件夹 | Não é possível ler o ficheiro ou a pasta de casos |
| PHOENIX-ORACLE-009 | Command option is not valid | La opción del comando no es válida | Die Option des Befehls ist ungültig | L'option de la commande n'est pas valide | L'opzione del comando non è valida | コマンドのオプションが無効です | 命令选项无效 | A opção do comando não é válida |
| PHOENIX-ORACLE-010 | Report file cannot be written | No se puede escribir el archivo del informe | Die Berichtsdatei kann nicht geschrieben werden | Le fichier du rapport ne peut pas être écrit | Il file del rapporto non può essere scritto | レポートファイルを書き込めません | 无法写入报告文件 | Não é possível escrever o ficheiro do relatório |

## Export to KiCad (`eh3d-phoenix-kicad`, `src/write_codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-KICADX-001 | Part of the model left out of the KiCad file | part of the design that has no place in a KiCad file, counted by name: design rules, net classes and the stackup (kept in other documents of the project), the drawings of footprints, nets of placed parts (KiCad derives nets from the wires and labels), items over `max_items`, copper layers beyond the 32 of KiCad, wires with fewer than two points, zones and outlines with fewer than three points (a warning) |
| PHOENIX-KICADX-002 | Item written in a simpler form | an item written in a simpler form: layers of the model that share one layer of KiCad, nets and names that were renamed to be valid, silkscreen text written as text of KiCad, arcs without a usable circle written as straight pieces, vias between inner layers written as blind vias, footprints on the bottom side written as KiCad flips them, turns that are not a right angle, links to a part written as a path of the first sheet, labels that are local to their sheet (a warning) |
| PHOENIX-KICADX-003 | Value the model does not give, written as a default | a value the model does not give, written as a default: layers without a side, copper layers without an order, pads without a layer, zones written without their filled copper, pins without a position or an electrical type, symbols drawn as a rectangle around their pins, parts without a place in the drawing (a warning) |
| PHOENIX-KICADX-004 | Reference to something the model does not have | an item that names a layer, a net, a footprint, a symbol or a pin that the model does not have; the item is left out (a warning) |
| PHOENIX-KICADX-005 | The output of the export is not allowed | `--out` of `phoenix kicad-export` that is an input, already exists or cannot be written, or that names a file when several files have to be written; nothing is written (an error, exit code 2) |
| PHOENIX-KICADX-006 | The written file does not read back the same | the file that was written does not parse, or the reader of the crate finds other counts of footprints, pad nets, track pieces, vias, zones, nets, parts, symbols or pins than were written (an error, exit code 1) |
| PHOENIX-KICADX-007 | Summary of the export | the summary of the export: the state of the writers, which are a prototype whose files were never opened in KiCad (information) |

The same titles in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese), as the program prints them:

| Code | en | es | de | fr | it | ja | zh | pt |
|---|---|---|---|---|---|---|---|---|
| PHOENIX-KICADX-001 | Part of the model left out of the KiCad file | Parte del modelo omitida del archivo de KiCad | Teil des Modells nicht in die KiCad-Datei geschrieben | Partie du modèle omise du fichier KiCad | Parte del modello omessa dal file KiCad | モデルの一部は KiCad ファイルに書き出されませんでした | 模型的一部分未写入 KiCad 文件 | Parte do modelo omitida do ficheiro KiCad |
| PHOENIX-KICADX-002 | Item written in a simpler form | Elemento escrito de forma más simple | Element in einfacherer Form geschrieben | Élément écrit sous une forme plus simple | Elemento scritto in forma più semplice | 項目はより単純な形で書き出されました | 项目以更简单的形式写入 | Elemento escrito de forma mais simples |
| PHOENIX-KICADX-003 | Value the model does not give, written as a default | Valor que el modelo no da, escrito con un valor por defecto | Wert, den das Modell nicht liefert, als Standardwert geschrieben | Valeur absente du modèle, écrite par défaut | Valore assente nel modello, scritto come predefinito | モデルにない値は既定値で書き出されました | 模型未提供的值以默认值写入 | Valor que o modelo não dá, escrito por omissão |
| PHOENIX-KICADX-004 | Reference to something the model does not have | Referencia a algo que el modelo no tiene | Verweis auf etwas, das im Modell fehlt | Référence à un élément absent du modèle | Riferimento a qualcosa che il modello non ha | モデルに存在しないものへの参照 | 引用了模型中不存在的内容 | Referência a algo que o modelo não tem |
| PHOENIX-KICADX-005 | The output of the export is not allowed | La salida de la exportación no está permitida | Die Ausgabe des Exports ist nicht erlaubt | La sortie de l'export n'est pas autorisée | L'output dell'esportazione non è consentito | エクスポートの出力先は許可されていません | 不允许该导出输出 | A saída da exportação não é permitida |
| PHOENIX-KICADX-006 | The written file does not read back the same | El archivo escrito no se lee igual | Die geschriebene Datei wird nicht gleich zurückgelesen | Le fichier écrit n'est pas relu à l'identique | Il file scritto non viene riletto uguale | 書き出したファイルを読み戻すと内容が一致しません | 写出的文件读回后不一致 | O ficheiro escrito não é lido de volta da mesma forma |
| PHOENIX-KICADX-007 | Summary of the export | Resumen de la exportación | Zusammenfassung des Exports | Résumé de l'export | Riepilogo dell'esportazione | エクスポートの概要 | 导出摘要 | Resumo da exportação |

## Own corpus (`eh3d-phoenix-corpus`, `src/suite/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-CORPUS-001 | The folder of the corpus cannot be read | `--dir` that is not a folder, a folder that cannot be read, nesting deeper than the setting, more files or expectations than the settings allow |
| PHOENIX-CORPUS-002 | An expectation file is not valid | an expectation file that is not JSON, has another format, an unknown field, an id that is not `CORPUS-<AREA>-<NNN>`, an empty title or note of how the numbers were worked out, an unknown kind, an outcome that is not `read` or `refused`, a path that leaves the folder, or a field that does not belong to its kind |
| PHOENIX-CORPUS-003 | The corpus is not consistent | an id used twice, a data file that an expectation names and is not there, a data file that no expectation names, or a round trip that cannot start from its file |
| PHOENIX-CORPUS-004 | The result differs from the expectation | a piece whose result differs from its expectation: the outcome, the codes, a measure or a number of the solver |
| PHOENIX-CORPUS-005 | A reader or writer failed where success was expected | a reader or a writer that refused a file that the expectation says is read, or a piece that could not run |
| PHOENIX-CORPUS-006 | The file was refused, as the expectation says | information: a file that had to be refused and was, with the codes the expectation names |
| PHOENIX-CORPUS-007 | A round trip lost something that the writer says it keeps | a round trip or an export read back that differs: a count, a position outside the tolerance, a name, or a count that the writer reports and the reader does not find |
| PHOENIX-CORPUS-008 | A property of the formats does not hold for a seed | a property of the readers, the writers and the operations that fails for a seed; the report names the seed |
| PHOENIX-CORPUS-009 | A command option is not valid | an option of `phoenix corpus` that is unknown, has no value or has a language code that is not one of the eight |
| PHOENIX-CORPUS-010 | The report file cannot be written | `--out` names a file that exists (an existing file is never replaced) or cannot be created |
| PHOENIX-CORPUS-011 | The corpus settings are not valid | a line of `config/corpus.cfg` that is not `key = value`, an unknown key or a value out of range, or a settings file that cannot be read |
| PHOENIX-CORPUS-012 | A corpus file is larger than the limit | a data file or an expectation file larger than `max_file_bytes` |

The same titles in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese), as the program prints them:

| Code | en | es | de | fr | it | ja | zh | pt |
|---|---|---|---|---|---|---|---|---|
| PHOENIX-CORPUS-001 | The folder of the corpus cannot be read | No se puede leer la carpeta del corpus | Der Ordner des Korpus kann nicht gelesen werden | Le dossier du corpus ne peut pas être lu | La cartella del corpus non può essere letta | コーパスのフォルダを読み取れません | 无法读取语料库文件夹 | Não é possível ler a pasta do corpus |
| PHOENIX-CORPUS-002 | An expectation file is not valid | Un archivo de expectativas no es válido | Eine Erwartungsdatei ist ungültig | Un fichier d'attentes n'est pas valide | Un file delle attese non è valido | 期待値ファイルが無効です | 预期文件无效 | Um ficheiro de expectativas não é válido |
| PHOENIX-CORPUS-003 | The corpus is not consistent | El corpus no es coherente | Der Korpus ist nicht konsistent | Le corpus n'est pas cohérent | Il corpus non è coerente | コーパスに矛盾があります | 语料库不一致 | O corpus não é coerente |
| PHOENIX-CORPUS-004 | The result differs from the expectation | El resultado difiere de la expectativa | Das Ergebnis weicht von der Erwartung ab | Le résultat diffère de l'attente | Il risultato differisce dall'attesa | 結果が期待値と異なります | 结果与预期不同 | O resultado difere da expectativa |
| PHOENIX-CORPUS-005 | A reader or writer failed where success was expected | Un lector o escritor falló donde se esperaba éxito | Ein Leser oder Schreiber ist gescheitert, wo Erfolg erwartet wurde | Un lecteur ou un écrivain a échoué là où le succès était attendu | Un lettore o scrittore ha fallito dove era atteso il successo | 成功が期待される場所でリーダーまたはライターが失敗しました | 读取器或写入器在预期成功之处失败 | Um leitor ou escritor falhou onde se esperava sucesso |
| PHOENIX-CORPUS-006 | The file was refused, as the expectation says | El archivo fue rechazado, como dice la expectativa | Die Datei wurde abgelehnt, wie die Erwartung es sagt | Le fichier a été refusé, comme l'attente le dit | Il file è stato rifiutato, come dice l'attesa | 期待どおりファイルは拒否されました | 文件按预期被拒绝 | O ficheiro foi recusado, como a expectativa indica |
| PHOENIX-CORPUS-007 | A round trip lost something that the writer says it keeps | Una ida y vuelta perdió algo que el escritor dice conservar | Ein Hin- und Rückweg hat etwas verloren, das der Schreiber zu erhalten angibt | Un aller-retour a perdu ce que l'écrivain dit conserver | Un andata e ritorno ha perso qualcosa che lo scrittore dichiara di conservare | 往復変換で、ライターが保持すると述べたものが失われました | 往返转换丢失了写入器声明保留的内容 | Uma ida e volta perdeu algo que o escritor diz conservar |
| PHOENIX-CORPUS-008 | A property of the formats does not hold for a seed | Una propiedad de los formatos no se cumple para una semilla | Eine Eigenschaft der Formate gilt für einen Startwert nicht | Une propriété des formats n'est pas vérifiée pour une graine | Una proprietà dei formati non vale per un seme | あるシードでフォーマットの性質が成り立ちません | 某个种子下格式的性质不成立 | Uma propriedade dos formatos não se verifica para uma semente |
| PHOENIX-CORPUS-009 | A command option is not valid | Una opción del comando no es válida | Eine Befehlsoption ist ungültig | Une option de la commande n'est pas valide | Un'opzione del comando non è valida | コマンドのオプションが無効です | 命令选项无效 | Uma opção do comando não é válida |
| PHOENIX-CORPUS-010 | The report file cannot be written | No se puede escribir el archivo del informe | Die Berichtsdatei kann nicht geschrieben werden | Le fichier du rapport ne peut pas être écrit | Il file del rapporto non può essere scritto | レポートファイルを書き込めません | 无法写入报告文件 | Não é possível escrever o ficheiro do relatório |
| PHOENIX-CORPUS-011 | The corpus settings are not valid | La configuración del corpus no es válida | Die Einstellungen des Korpus sind ungültig | Les réglages du corpus ne sont pas valides | Le impostazioni del corpus non sono valide | コーパスの設定が無効です | 语料库设置无效 | As definições do corpus não são válidas |
| PHOENIX-CORPUS-012 | A corpus file is larger than the limit | Un archivo del corpus supera el límite | Eine Korpusdatei ist größer als das Limit | Un fichier du corpus dépasse la limite | Un file del corpus supera il limite | コーパスのファイルが上限を超えています | 语料库文件超过限制 | Um ficheiro do corpus excede o limite |

## Deep reading of EAGLE files (`eh3d-phoenix-eagle-deep`, `src/codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-EAGLEX-001 | Design rule file is not valid | a design rule file that is larger than 1 MiB, has more than 20000 lines, holds a NUL byte, has a line without `=`, a key that is not letters, digits and `_`, or a `description[...]` that is not closed or has no language tag |
| PHOENIX-EAGLEX-002 | Design rule parameter is kept as it is and not understood | warning: a key that is not an EAGLE parameter (it is kept in the file and not used) |
| PHOENIX-EAGLEX-003 | Design rule parameter has no equivalent in the rules of this tool | information: the EAGLE parameters that the rules of this tool do not use (the layer set-up, the thicknesses, the ratios of the pads and vias, the shapes of the pads, the mask and cream frames, the thermals, the checks of the file) |
| PHOENIX-EAGLEX-004 | Several design rule parameters were folded into one value | information: the nine clearances `md...` between wires, pads, vias and SMD pads, or the minimum rings `rlMin...`, differ and the largest is used for all |
| PHOENIX-EAGLEX-005 | Length cannot be read | warning: the value of a parameter that the rules use is not a length with `mil`, `mm`, `inch` or `micron` (or a bare number in millimetres) |
| PHOENIX-EAGLEX-006 | Rule of this tool has no equivalent among the EAGLE parameters | warning: writing the rules as a design rule file when a rule has no EAGLE parameter (the size of vias, the bridge of the mask, the tolerance of connections, the checks of the silkscreen) or net classes are given |
| PHOENIX-EAGLEX-007 | CAM job is not valid | a CAM job that is larger than 1 MiB, empty, has more than 1000 sections, a section without a device, a layer outside 1 to 255, a line without `=` or a section name that is not closed, or XML or JSON that cannot be read |
| PHOENIX-EAGLEX-008 | CAM option is not respected by the export profile | information or warning: an option of a section that the export profile does not respect (the file names, the unit, the wheel and the flags; as a warning the mirror, rotation, scale, offset and tolerance when they are not neutral, and a format that differs from the first section) |
| PHOENIX-EAGLEX-009 | Layer that a CAM job asks for is not exported | information: layers of a section that the export does not write as a layer of its own (the names, the values, the milling, the documentation) or user layers without a role |
| PHOENIX-EAGLEX-010 | Output device of a CAM job is not supported | warning: a device that is not Gerber or Excellon (the section is not converted) or a Gerber device that is not RS-274X |
| PHOENIX-EAGLEX-011 | Variant is not defined in the file | `--variant` or `eagle-convert variants --variant` names a variant that the file does not define (the names it defines are listed) |
| PHOENIX-EAGLEX-012 | Variant data of the file is inconsistent | warning: a variant defined twice, a part with a name used twice, a `<variant>` of a name that no `<variantdef>` defines or given twice in one part, or a part of a set that the file does not have |
| PHOENIX-EAGLEX-013 | Layer map settings are not valid | a line of `config/eagle_layers.cfg` that is not `key = value`, a key that is not `layer.<number>` or `name.<Name>`, a number outside 100 to 255, an unknown role, a number or name given twice, or a file larger than 1 MiB |
| PHOENIX-EAGLEX-014 | User layer has no mapping | warning: a user layer (100 to 255) that has no role in the layer map; it is kept in the file and not exported |
| PHOENIX-EAGLEX-015 | Layer number is out of range or repeated | warning: a layer number that is not a number from 1 to 255 or that is used by more than one layer (the first is kept) |
| PHOENIX-EAGLEX-016 | Layer has another name than the standard table gives | information: a standard layer (1 to 99) that has another name in the file than in the standard table |
| PHOENIX-EAGLEX-017 | Element is kept as opaque XML | information: a kind of element that nothing here interprets; it is kept byte for byte in the tree and left out by the design model |
| PHOENIX-EAGLEX-018 | Element is kept in the tree but simplified by the design model | information: an element that the design model reads in a simpler form (a pad shape, the roundness or stop of an SMD pad, a smashed or locked element, a net class, the pour, thermals, orphans, rank or spacing of a polygon, the contents of a module) |
| PHOENIX-EAGLEX-019 | Element is left out by the design model | warning: a kind of element that the records of this crate cover and the design model leaves out (attributes, variants, dimensions, holes, design rules, the autorouter, net classes, modules); reading the file into the model and writing it would lose it, rewriting the tree keeps it |
| PHOENIX-EAGLEX-020 | Rewriting the tree changed the file | error: writing the XML tree and reading it again gives another tree (this should never happen; the exit code of `eagle-check` is 1) |
| PHOENIX-EAGLEX-021 | File cannot be read as an EAGLE file | a file that is not well-formed XML, is over the limits of the reader, whose root is not `eagle` or whose drawing has no board, schematic or library |
| PHOENIX-EAGLEX-022 | A command option is not valid | an option that the command does not know or needs a value for, an unknown conversion, a number of input files other than the command takes |
| PHOENIX-EAGLEX-023 | The output file exists or cannot be written | `--out` names a file that exists (an existing file is never replaced) or cannot be created |
| PHOENIX-EAGLEX-024 | Input file cannot be read | an input file that cannot be opened, is larger than 256 MiB, or a rule file that is not valid |

The same titles in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese), as the program prints them:

| Code | en | es | de | fr | it | ja | zh | pt |
|---|---|---|---|---|---|---|---|---|
| PHOENIX-EAGLEX-001 | Design rule file is not valid | El fichero de reglas de diseño no es válido | Die Datei mit den Entwurfsregeln ist ungültig | Le fichier des règles de conception n'est pas valide | Il file delle regole di progetto non è valido | デザインルールファイルが無効です | 设计规则文件无效 | O ficheiro de regras de projeto não é válido |
| PHOENIX-EAGLEX-002 | Design rule parameter is kept as it is and not understood | Un parámetro de reglas de diseño se conserva tal cual y no se entiende | Ein Entwurfsregel-Parameter wird unverändert behalten und nicht verstanden | Un paramètre de règles de conception est conservé tel quel et non compris | Un parametro delle regole di progetto è conservato così com'è e non è compreso | デザインルールのパラメータはそのまま保持され、解釈されません | 设计规则参数原样保留，未被解析 | Um parâmetro de regras de projeto é conservado tal como está e não é entendido |
| PHOENIX-EAGLEX-003 | Design rule parameter has no equivalent in the rules of this tool | Un parámetro de reglas de diseño no tiene equivalente en las reglas de esta herramienta | Ein Entwurfsregel-Parameter hat in den Regeln dieses Werkzeugs keine Entsprechung | Un paramètre de règles de conception n'a pas d'équivalent dans les règles de cet outil | Un parametro delle regole di progetto non ha equivalente nelle regole di questo strumento | デザインルールのパラメータに対応するこのツールのルールがありません | 设计规则参数在本工具的规则中没有对应项 | Um parâmetro de regras de projeto não tem equivalente nas regras desta ferramenta |
| PHOENIX-EAGLEX-004 | Several design rule parameters were folded into one value | Varios parámetros de reglas de diseño se fundieron en un solo valor | Mehrere Entwurfsregel-Parameter wurden zu einem Wert zusammengefasst | Plusieurs paramètres de règles de conception ont été fondus en une seule valeur | Più parametri delle regole di progetto sono stati fusi in un solo valore | 複数のデザインルールのパラメータが1つの値にまとめられました | 多个设计规则参数被合并为一个值 | Vários parâmetros de regras de projeto foram fundidos num só valor |
| PHOENIX-EAGLEX-005 | Length cannot be read | No se puede leer la longitud | Die Länge kann nicht gelesen werden | La longueur ne peut pas être lue | La lunghezza non può essere letta | 長さを読み取れません | 无法读取长度 | Não é possível ler o comprimento |
| PHOENIX-EAGLEX-006 | Rule of this tool has no equivalent among the EAGLE parameters | Una regla de esta herramienta no tiene equivalente entre los parámetros de EAGLE | Eine Regel dieses Werkzeugs hat unter den EAGLE-Parametern keine Entsprechung | Une règle de cet outil n'a pas d'équivalent parmi les paramètres d'EAGLE | Una regola di questo strumento non ha equivalente tra i parametri di EAGLE | このツールのルールに対応するEAGLEのパラメータがありません | 本工具的规则在 EAGLE 参数中没有对应项 | Uma regra desta ferramenta não tem equivalente entre os parâmetros do EAGLE |
| PHOENIX-EAGLEX-007 | CAM job is not valid | El trabajo CAM no es válido | Der CAM-Auftrag ist ungültig | Le travail CAM n'est pas valide | Il lavoro CAM non è valido | CAMジョブが無効です | CAM 作业无效 | O trabalho CAM não é válido |
| PHOENIX-EAGLEX-008 | CAM option is not respected by the export profile | El perfil de exportación no respeta una opción CAM | Das Exportprofil berücksichtigt eine CAM-Option nicht | Le profil d'exportation ne respecte pas une option CAM | Il profilo di esportazione non rispetta un'opzione CAM | エクスポートプロファイルはCAMオプションを反映しません | 导出配置不遵循某个 CAM 选项 | O perfil de exportação não respeita uma opção CAM |
| PHOENIX-EAGLEX-009 | Layer that a CAM job asks for is not exported | Una capa que pide el trabajo CAM no se exporta | Eine vom CAM-Auftrag verlangte Lage wird nicht exportiert | Une couche demandée par le travail CAM n'est pas exportée | Un livello richiesto dal lavoro CAM non viene esportato | CAMジョブが要求したレイヤーはエクスポートされません | CAM 作业要求的层不会被导出 | Uma camada pedida pelo trabalho CAM não é exportada |
| PHOENIX-EAGLEX-010 | Output device of a CAM job is not supported | No se admite el dispositivo de salida de un trabajo CAM | Das Ausgabegerät eines CAM-Auftrags wird nicht unterstützt | Le périphérique de sortie d'un travail CAM n'est pas pris en charge | Il dispositivo di uscita di un lavoro CAM non è supportato | CAMジョブの出力デバイスには対応していません | 不支持 CAM 作业的输出设备 | O dispositivo de saída de um trabalho CAM não é suportado |
| PHOENIX-EAGLEX-011 | Variant is not defined in the file | La variante no está definida en el fichero | Die Variante ist in der Datei nicht definiert | La variante n'est pas définie dans le fichier | La variante non è definita nel file | バリアントがファイルに定義されていません | 文件中未定义该变体 | A variante não está definida no ficheiro |
| PHOENIX-EAGLEX-012 | Variant data of the file is inconsistent | Los datos de variantes del fichero son incoherentes | Die Variantendaten der Datei sind widersprüchlich | Les données de variantes du fichier sont incohérentes | I dati delle varianti del file sono incoerenti | ファイルのバリアントデータに矛盾があります | 文件中的变体数据不一致 | Os dados de variantes do ficheiro são incoerentes |
| PHOENIX-EAGLEX-013 | Layer map settings are not valid | La configuración del mapa de capas no es válida | Die Einstellungen der Lagenzuordnung sind ungültig | Les réglages de la table des couches ne sont pas valides | Le impostazioni della mappa dei livelli non sono valide | レイヤーマップの設定が無効です | 层映射设置无效 | As definições do mapa de camadas não são válidas |
| PHOENIX-EAGLEX-014 | User layer has no mapping | Una capa de usuario no tiene correspondencia | Eine Benutzerlage hat keine Zuordnung | Une couche utilisateur n'a pas de correspondance | Un livello utente non ha corrispondenza | ユーザーレイヤーに対応付けがありません | 用户层没有映射 | Uma camada de utilizador não tem correspondência |
| PHOENIX-EAGLEX-015 | Layer number is out of range or repeated | El número de capa está fuera de rango o repetido | Die Lagennummer liegt außerhalb des Bereichs oder ist doppelt | Le numéro de couche est hors limites ou répété | Il numero di livello è fuori intervallo o ripetuto | レイヤー番号が範囲外か重複しています | 层号超出范围或重复 | O número da camada está fora do intervalo ou repetido |
| PHOENIX-EAGLEX-016 | Layer has another name than the standard table gives | Una capa tiene otro nombre que el de la tabla estándar | Eine Lage hat einen anderen Namen als die Standardtabelle | Une couche porte un autre nom que celui de la table standard | Un livello ha un nome diverso da quello della tabella standard | レイヤー名が標準の表と異なります | 层名与标准表不同 | Uma camada tem outro nome que não o da tabela normalizada |
| PHOENIX-EAGLEX-017 | Element is kept as opaque XML | El elemento se conserva como XML opaco | Das Element bleibt als opakes XML erhalten | L'élément est conservé comme XML opaque | L'elemento è conservato come XML opaco | 要素は不透明なXMLとして保持されます | 该元素作为不透明 XML 保留 | O elemento é conservado como XML opaco |
| PHOENIX-EAGLEX-018 | Element is kept in the tree but simplified by the design model | El elemento se conserva en el árbol pero el modelo de diseño lo simplifica | Das Element bleibt im Baum erhalten, das Entwurfsmodell vereinfacht es aber | L'élément reste dans l'arbre mais le modèle de conception le simplifie | L'elemento resta nell'albero ma il modello di progetto lo semplifica | 要素はツリーに保持されますが、設計モデルでは簡略化されます | 该元素保留在树中，但设计模型对其做了简化 | O elemento fica na árvore mas o modelo de projeto simplifica-o |
| PHOENIX-EAGLEX-019 | Element is left out by the design model | El modelo de diseño deja fuera el elemento | Das Entwurfsmodell lässt das Element weg | Le modèle de conception laisse l'élément de côté | Il modello di progetto lascia fuori l'elemento | 設計モデルはこの要素を含めません | 设计模型不包含该元素 | O modelo de projeto deixa o elemento de fora |
| PHOENIX-EAGLEX-020 | Rewriting the tree changed the file | Reescribir el árbol cambió el fichero | Das Zurückschreiben des Baums hat die Datei verändert | Réécrire l'arbre a modifié le fichier | Riscrivere l'albero ha modificato il file | ツリーを書き戻すとファイルが変わりました | 重写树改变了文件 | Reescrever a árvore alterou o ficheiro |
| PHOENIX-EAGLEX-021 | File cannot be read as an EAGLE file | El fichero no se puede leer como fichero de EAGLE | Die Datei kann nicht als EAGLE-Datei gelesen werden | Le fichier ne peut pas être lu comme un fichier EAGLE | Il file non può essere letto come file EAGLE | ファイルをEAGLEファイルとして読み取れません | 无法将该文件作为 EAGLE 文件读取 | O ficheiro não pode ser lido como ficheiro do EAGLE |
| PHOENIX-EAGLEX-022 | A command option is not valid | Una opción del comando no es válida | Eine Befehlsoption ist ungültig | Une option de la commande n'est pas valide | Un'opzione del comando non è valida | コマンドのオプションが無効です | 命令选项无效 | Uma opção do comando não é válida |
| PHOENIX-EAGLEX-023 | The output file exists or cannot be written | El fichero de salida existe o no se puede escribir | Die Ausgabedatei existiert oder kann nicht geschrieben werden | Le fichier de sortie existe ou ne peut pas être écrit | Il file di uscita esiste o non può essere scritto | 出力ファイルが既に存在するか、書き込めません | 输出文件已存在或无法写入 | O ficheiro de saída existe ou não pode ser escrito |
| PHOENIX-EAGLEX-024 | Input file cannot be read | No se puede leer el fichero de entrada | Die Eingabedatei kann nicht gelesen werden | Le fichier d'entrée ne peut pas être lu | Il file di ingresso non può essere letto | 入力ファイルを読み取れません | 无法读取输入文件 | Não é possível ler o ficheiro de entrada |

## Checks on files and paths (`eh3d-phoenix-cli`, `src/sec_codes.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-SEC-001 | Output path is refused | a value of `--out`, `--dir`, `--into`, `--svg` or `--csv` that is empty, holds a NUL character, starts with `\\.\`, `\\?\` or `\\` (device, verbatim and network paths), holds a `:` after the drive letter (an alternate data stream of NTFS) or has a part named like a Windows device (`CON`, `PRN`, `AUX`, `NUL`, `COM1` to `COM9`, `LPT1` to `LPT9`, with any extension) |
| PHOENIX-SEC-002 | The side file of the write is already there | the atomic write of `sync --out` and `netlist --out` finds the side file `NAME.tmp` already there; it is not written over and not followed if it is a link |
| PHOENIX-SEC-003 | Settings file is larger than its limit | a settings file of more than 1 MiB given to `--config` of `inventory`, `corpus --settings`, `dxf-import --config`, `perf --config` or the window |

The same titles in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese), as the program prints them:

| Code | en | es | de | fr | it | ja | zh | pt |
|---|---|---|---|---|---|---|---|---|
| PHOENIX-SEC-001 | Output path is refused | Se rechaza la ruta de salida | Der Ausgabepfad wird abgelehnt | Le chemin de sortie est refusé | Il percorso di uscita è rifiutato | 出力パスは拒否されました | 输出路径被拒绝 | O caminho de saída é recusado |
| PHOENIX-SEC-002 | The side file of the write is already there | Ya existe el fichero auxiliar de la escritura | Die Hilfsdatei des Schreibvorgangs ist bereits vorhanden | Le fichier auxiliaire de l'écriture existe déjà | Il file ausiliario della scrittura esiste già | 書き込み用の補助ファイルが既に存在します | 写入用的辅助文件已存在 | O ficheiro auxiliar da escrita já existe |
| PHOENIX-SEC-003 | Settings file is larger than its limit | El fichero de ajustes supera su límite | Die Einstellungsdatei ist größer als ihr Grenzwert | Le fichier de réglages dépasse sa limite | Il file delle impostazioni supera il suo limite | 設定ファイルが上限を超えています | 设置文件超过其限制 | O ficheiro de definições excede o seu limite |
