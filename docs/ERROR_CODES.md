# EH3D-PHOENIX-PCB: diagnostic codes

Codes have the form `PHOENIX-<AREA>-<NNN>`. A code is stable: it never changes its meaning and is never reused. Programs show the text through the key `diag.<code>`; the codes themselves are never translated. The object identity (UUID v7) is the one of the shared container, with its own codes `EH3D-ID-001` and `EH3D-ID-002`. The container codes `EH3D-PKG-001` to `EH3D-PKG-014` belong to the shared container and are listed in the registry of the documentation repository.

Each table repeats the `REGISTRY` of a crate (`eh3d-phoenix-model` in `src/diagnostic.rs`, `eh3d-phoenix-project` and `eh3d-phoenix-eagle` in `src/codes.rs`, `eh3d-phoenix-hub` in `src/lib.rs`, `eh3d-phoenix-export` in `src/codes.rs`, `eh3d-phoenix-board` in `src/validate.rs`, `src/connectivity.rs` and `src/rules.rs`); a test of each crate checks that every code of its registry is in the table with the same title.

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
| PHOENIX-EAGLE-012 | EAGLE binary format is not read | a file that starts with the bytes 0x10 0x80 like the binary files of versions 3 to 5, which this reader does not read |

## Rule check (`eh3d-phoenix-board`, `src/rules.rs`)

| Code | Title | Raised by |
|---|---|---|
| PHOENIX-RULE-001 | Rule file line not understood | a line of the rule file that is not `name = value`, has an unknown name or a value that is not a non-negative number of millimetres |
| PHOENIX-RULE-002 | Track narrower than the minimum | a track whose width is below `min_track_width` |
| PHOENIX-RULE-003 | Drill or via smaller than the minimum | a via or pad drill below `min_drill`, or a via diameter below `min_via_diameter` |
| PHOENIX-RULE-004 | Annular ring thinner than the minimum | a via or pad whose ring of copper around the drill is thinner than `min_annular_ring` |
| PHOENIX-RULE-005 | Clearance violated | straight track pieces of different nets on one layer, or such a piece and a via of another net, closer than `min_clearance` edge to edge (proven with integer arithmetic) |
| PHOENIX-RULE-006 | Clearance could not be judged | a track with an arc, or numbers too large for the exact test (information, listed apart from the confirmed findings) |

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
