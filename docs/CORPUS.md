# The own corpus: small pieces written by hand, with the answers worked out by hand

The corpus is the test material of this repository for the readers and the writers: small boards, schematics, libraries and manufacturing files that were **written by hand for this repository**, in every format that the suite reads or writes, each with an expectation file that says what the real readers and writers must give and how the numbers were worked out. It is run by `phoenix corpus` (crate `eh3d-phoenix-corpus`, module `suite`) and by the tests of the crate and of the command line.

**State: prototype.** The corpus holds no file of a maker, of a user or of a public project: every piece is small and written here. Passing it shows that the readers and the writers give what was worked out by hand for these pieces. It does **not** show that they agree with EAGLE, KiCad, a CAM program or a SPICE program: that comparison is planned in `ORACLE.md` and has not been made.

## What is in `tests/corpus`

| Folder | Pieces | Formats |
|---|---|---|
| `eagle/` | a board of two resistors and a header, a divider (schematic), a library with a missing symbol, a missing package and a pad that does not exist, a board and a schematic with part attributes, assembly variants and a module, boards with user layers, with polygons of several pours, holes and a dimension, with net classes, design rules and an autorouter, and with odd attributes (`docs/EAGLE_DEEP.md`), a board with the restrict layers 41 and 42, pads of several shapes and a hole in the package, a schematic with buses, and the adversarial files below | EAGLE XML `.brd`, `.sch`, `.lbr` |
| `kicad/` | a board of two resistors and a header, a divider with labels and a ground symbol, a footprint, a symbol library, a project with two net classes, outlines with openings drawn as rectangles, polygons and loose lines, a footprint with rounded rectangle, trapezoid and mounting-hole pads placed twice, zones with rule areas that keep the pour out, a bus with entries, a top sheet with sheet symbols, and the adversarial files below | `.kicad_pcb`, `.kicad_sch`, `.kicad_mod`, `.kicad_sym`, `.kicad_pro` |
| `gerber/` | a top copper layer with the attributes of Gerber X2, a layer in inches without attributes, an Excellon file of plated holes with a slot, and the adversarial files below | RS-274X with and without X2, Excellon |
| `model/` | a small board with a track, vias and a zone, a board for the rule check, a board for the connectivity, a board for the round trips, and the adversarial files below | the board as JSON (`design/board.json` of a project) |
| `route/` | the same small board and three sessions of an autorouter, and the same board with cutouts in its pour and an opening in its outline | the file for the autorouter (DSN, written from the board), the session (SES, read) |
| `spice/` | a divider, a netlist with a byte order mark, mixed line ends and a continuation, and the netlists that must be refused | SPICE-style netlists |

There are 112 expectations (71 files that must be read and 41 that must be refused) over 84 data files. `tests/corpus/.gitattributes` marks the folder as `-text`, so that git never changes a byte order mark or a line end.

### The adversarial files

| Case | Pieces |
|---|---|
| Truncated files | an EAGLE board cut inside a tag, a KiCad board, footprint and schematic cut inside a list, a KiCad project cut inside the JSON, a Gerber layer cut inside an aperture, a board as JSON cut in half, a session cut inside a list |
| Numbers that are too large | EAGLE coordinates beyond the range of the model, a KiCad track that ends 99999999999999 mm away, a Gerber coordinate of 14 digits, a JSON coordinate of 1073741824 nm (refused) next to 1073741823 nm (read), a coordinate with a fraction, a coordinate written as text |
| Deep nesting | EAGLE elements, KiCad lists and JSON arrays nested deeper than the limit |
| Names that are not ASCII | accents, Greek, Japanese and an emoji in the names of libraries, packages, parts, values and nets of EAGLE, KiCad and JSON, and in the title of a netlist |
| Line ends and marks | a byte order mark, lines ended by CRLF and LF in turn, in EAGLE, KiCad, Gerber, JSON and SPICE files; bytes that are not UTF-8 in EAGLE and KiCad |
| Other programs and other formats | a KiCad file with the extension of an EAGLE file, the two forms of the old binary header, XML that is not EAGLE, a schematic given to the board reader, a file with nothing in it, a second list after the first, a closing parenthesis too many |
| At the limit | a file as large as the limit and one byte larger, an element count and a depth at the limit and one over, a text as long as the limit and one byte longer, as many objects or apertures of a Gerber file as the limit allows and one more |

The limits of each reader are the ones the reader has in its own settings (`EAGLE`: `max_bytes`, `max_depth`, `max_nodes`; KiCad: `config/kicad.cfg`; Gerber: `config/gerber_import.cfg`; the simulator: `config/sim.cfg`). A piece may lower a limit for itself with `limits`, so that the file stays small: `@size` is the size of the data file and `@size-1` one byte less, which puts a file exactly at the limit and one byte over without counting bytes by hand.

One behaviour that the corpus pins today and that is worth knowing: a Gerber file that starts with a byte order mark is refused with `PHOENIX-GBR-004`, while the EAGLE, KiCad and SPICE readers take the mark off and the JSON reader of the model refuses it without a code. A change of any of these is a decision, and the corpus then changes with it.

## The expectation file

An expectation is a file `NAME.expect.json` (or `NAME.TAG.expect.json` when one data file has several) in the folder of its data file. Format `phoenix-corpus/1`; unknown fields are refused.

| Field | Meaning |
|---|---|
| `id` | `CORPUS-<AREA>-<NNN>`, unique in the corpus |
| `title` | what the piece shows |
| `kind` | the reader or the chain that runs, see below |
| `input` | the data file, relative to the folder, never outside it |
| `session` | the second data file of `dsn-ses` |
| `how` | **how the numbers were worked out**, one line each; required, never empty |
| `limits` | limits of the reader for this piece: a number, `@size` or `@size-1` |
| `rules` | the text of a rule file, for the rule check of a board |
| `erc` | true to run the electrical rule check on a schematic with `config/erc.cfg` |
| `via`, `compare`, `tolerance_nm` | the formats a round trip goes through, what it compares and how far two positions may differ |
| `expect.outcome` | `read` or `refused` |
| `expect.codes` | every diagnostic code that must be seen, and no other |
| `expect.codes_include` | codes that must be among the ones seen |
| `expect.measures` | numbers, texts and flags worked out by hand; a measure that the result does not have counts as 0 |
| `expect.probes` | numbers of the solver with the distance accepted from each |

| Kind | What runs | Measures |
|---|---|---|
| `eagle` | the summary reader of EAGLE XML | kind, version, layers, packages with pads and smds, symbols with pins, device sets, parts, nets, routed items, sheets, embedded libraries, elements left uninterpreted |
| `eagle-board` | the reader of boards into the model | the counts of a board (layers, copper layers, footprints, components, bottom components, placed pads, nets, pad nets, tracks and their pieces, vias, zones, silkscreen, outline points) |
| `eagle-schematic` | the reader of schematics into the model (and with `erc` the electrical rule check) | symbols, parts, nets, sheets, wires, buses, bus entries; `erc.confirmed`, `erc.<code>` |
| `kicad-board`, `kicad-schematic`, `kicad-footprint`, `kicad-symbols`, `kicad-project` | the KiCad readers with the limits of `config/kicad.cfg` | the counts of the model, among them the cutouts of the zones (`zone_cutouts`) and the openings of the outline (`outline_cutouts`), and the kept, changed and dropped data of the quality report (`kept.pads`, `dropped.setup (...)`, `kept.zone cutouts`, `dropped.keep-out zones`) |
| `gerber` | the import of one Gerber or Excellon file | content, units, objects, flashes, lines, arcs, apertures, holes, slots, tools, whether it ended |
| `board-json` | the board as JSON, its consistency, its rule check with `rules` and its connectivity | the counts of a board, `consistent`, `validate.<code>`, `drc.<code>`, `connectivity.islands` |
| `dsn` | the file for the autorouter | layers, components, pads, nets, padstacks, tracks, vias, `zone_cutouts` (written as keep-out areas) |
| `dsn-ses` | the session is read against the board, the plan is applied and the file is written again | tracks, vias, `already_there`, length, per net, the counts before and after |
| `spice` | the netlist is read, compiled and solved at the operating point | the numbers of `expect.probes` |
| `roundtrip` | a chain of writers and readers | `differences`, `declared_mismatches`, `eagle_left_out`, `eagle_signature_identical` |
| `gerber-roundtrip` | the files of the export are read back and compared with the board | `files`, `differences`, `imported_failed` |

### How a number was worked out

Every expectation says how, in `how`. The rule is the one of `ORACLE.md`: a number comes from counting the file, from a definition or from a closed formula, never from running the program and copying the answer. Examples:

* a divider of 1 kohm and 3 kohm on 10 V gives `10 x 3k / (1k + 3k) = 7.5 V`;
* two tracks 0.4 mm apart (centre to centre) with a width of 0.25 mm each have their edges 0.4 - 0.25 = 0.15 mm apart, under a clearance rule of 0.2 mm, so the rule check finds one clearance finding;
* a via of 0.4 mm over a drill of 0.15 mm under the rules "via 0.5 mm, drill 0.2 mm" breaks two rules (`PHOENIX-RULE-003` twice) and has a ring of (0.4 - 0.15) / 2 = 0.125 mm, which passes the ring rule of 0.1 mm;
* the wires of one signal that share an end point and a width join into one track, so two wires are one track of two pieces;
* a session wire that repeats a track the board already has is counted in `already_there` and not added, and a vertical wire of 5 mm has the length 5000000 nm.

## The command

```
phoenix corpus run   [--dir FOLDER] [--filter TEXT] [--properties] [--settings FILE] [--limits FILE] [--out FILE] [--json] [--lang CODE]
phoenix corpus list  [--dir FOLDER] [--filter TEXT] [--settings FILE]
phoenix corpus check [--dir FOLDER] [--settings FILE] [--lang CODE]
```

`--limits FILE` gives the readers of EAGLE files and the checks of a board the limits of `config/eagle_board.cfg` and `config/board.cfg` (see `docs/SECURITY_REVIEW.md`); without it the defaults are used and the verdicts do not change.

`run` gives each piece a verdict: **ok**, **difference** (the result is not what the expectation says), **expected error** (the file had to be refused and was) or **failed** (a reader failed where it had to succeed, or the piece could not run). The report is the same every time, in text (eight languages) and in JSON (`phoenix-corpus-report/1`). The exit code is 0 when no piece is a difference or a failure, 1 otherwise and 2 when the arguments or the folder are unusable. `check` finds, without running anything, the expectations that are not valid, an id used twice, a data file that an expectation names and is not there, and a data file that no expectation names. `--properties` adds the properties below. The command starts no other program and writes nothing unless `--out` names a new file; an existing file is never replaced. The default folder is `tests/corpus`; the settings are in `config/corpus.cfg` (the largest file, the most expectations, the depth of folders, the tolerance of the round trips, the number of seeds).

## Round trips between formats

A chain reads a board from one format into the model, writes it in another and reads that back, and then compares the board that comes back with the board that went in. The facts that are compared are chosen in `compare`: the counts of components, placed pads, nets, pad nets, track pieces, vias, zones and outline points; the positions in nanometres of the pads, the vias, the ends of the tracks, the outline points and the parts, with the tolerance `tolerance_nm`; the references with their values, the net names and the names of the pad nets.

| Chain | Pieces | What the writer declares and the corpus checks |
|---|---|---|
| EAGLE, model, KiCad, model | a hand-written EAGLE board | the KiCad writer reports how many footprints, pad nets, track pieces, vias, zones and nets it wrote; each count must equal what the reader finds, and the report must hold no problem |
| EAGLE, model, EAGLE, model | the same board and the board of the round trips | the signature of the EAGLE writer (everything a file can say, without ids) is the same before and after, and the writer leaves nothing out |
| KiCad, model, EAGLE, model | a hand-written KiCad board | the facts that EAGLE keeps |
| KiCad, model, KiCad, model | the same board | the declared counts and the facts |
| model, EAGLE, KiCad, JSON, model | the board of the round trips | all of the above in one chain, and the canonical JSON written again gives the same bytes |
| model, Gerber and Excellon, import | a small board | the files of the export read back as the board: no difference, no file that fails |
| model, DSN, SES, model | a small board and a session | the plan of the session adds what the session says, the file written again counts the new tracks and vias |

The routes of the schematic (EAGLE to KiCad and back) are not in the corpus yet; see `NOT_YET.md`.

## Properties on generated boards

`phoenix corpus run --properties` and the tests of the crate try these properties with the seeds of `config/corpus.cfg` (48 by default, the first is 1). The generator is our own (xorshift64 started through splitmix64, no dependency): the same seed gives the same board on every computer, and a failure names the seed.

| Property | What is checked |
|---|---|
| `undo` | after a sequence of changes, some of them refused, undoing every change gives the bytes the board had at the start, doing them again gives the bytes it had after them, and a refused change altered nothing |
| `rejection` | eleven changes that cannot apply (an id that exists, a part that is not there, a footprint in use, a track on a net that is not there, a batch whose second change fails, and others) leave the board as it was and are not recorded by the history |
| `json` | the board read from its canonical bytes is the same board and writes the same bytes |
| `eagle` | writing as EAGLE, reading and writing again gives the same file; the signature is the same; the facts that EAGLE keeps are the same |
| `kicad` | writing as KiCad and reading keeps the facts that KiCad keeps, and doing it twice gives the same board as doing it once |
| `drc-eagle`, `drc-kicad` | the rule check (track width, via size, drill and ring) gives the same findings by code after the round trip |
| `dsn` | the file for the autorouter is the same twice and counts what the board holds |

The boards have two copper layers, an outline, two to five components (a chip and a header, turned by right angles), nets on most pads, up to three tracks and two vias. A property stops at its first failing seed.

## Codes

The codes `PHOENIX-CORPUS-001` to `PHOENIX-CORPUS-012` are in `ERROR_CODES.md` with their titles in the eight languages: the folder cannot be read (001), an expectation is not valid (002), the corpus is not consistent (003), the result differs from the expectation (004), a reader failed where success was expected (005), the file was refused as the expectation says (006, information), a round trip lost something that the writer says it keeps (007), a property does not hold for a seed (008), an option is not valid (009), the report cannot be written (010), the settings are not valid (011) and a file is larger than the limit (012).

## Adding a piece

1. Write the data file by hand, as small as it can be. Do not copy a file from a maker, a user or a project: if the piece needs a fixture that exists elsewhere, write its own smaller version.
2. Work the numbers out from the file, a definition or a formula, and write how in `how`. Choose the tolerance of a number before running anything.
3. Write `NAME.expect.json` next to it, run `phoenix corpus check` and then `phoenix corpus run --filter ID`.
4. A piece that fails is not changed until it passes: first find whether the file, the expectation or the code is wrong, and write the cause in the commit.
5. When a limit is the point of the piece, put the file at the limit and one over with `@size` and `@size-1` or with a count that you can check by hand.

## What the corpus does not do

* It does not compare anything with EAGLE, KiCad, a CAM program or a SPICE program, and it holds no file of a maker. Its answers are the answers of this repository to files of this repository.
* It does not run the readers on files bigger than a few kilobytes; the sizes that break a reader are tried through the limits, not through large files.
* The schematics are read and checked (the electrical rule check of an EAGLE schematic is in it), but they are not yet taken through the writers; the properties work on boards only.

## The inventory of a private folder

`phoenix inventory` (the same crate, module `inventory`) counts the design files of a folder that must not be shown, in redacted form: files by extension, kind and declared version, and one fingerprint of the content, never a name or a value. Its limits are a plain `key = value` file given with `phoenix --config FILE inventory <folder>`: `max_files`, `max_depth`, `sniff_bytes`, `max_file_bytes` and `fingerprint`. `config/phoenix.cfg` holds the default values; a missing line keeps its default and a file of more than 1 MiB is refused (`PHOENIX-SEC-003`).
