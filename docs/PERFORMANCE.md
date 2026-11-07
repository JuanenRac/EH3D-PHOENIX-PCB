# EH3D-PHOENIX-PCB: performance of the readers and of the rule check

This page says how the time of the readers (KiCad, EAGLE, DXF, Gerber and Excellon) and of the rule check is tested, which limits keep it in bounds, which work was found to grow with the square of a count and was changed, and which work still does.

**State: prototype.** The benchmarks run on boards and files that the test makes from fixed formulas, never on the files of a real design. The limits of time are upper bounds that a slow computer still meets; no table of measured times is kept in this repository, because a time depends on the computer. The test prints the times of every run (`cargo test -p eh3d-phoenix-cli --test benchmarks -- --nocapture`), and they are read on the computer where the question is asked.

## The benchmark test

`crates/eh3d-phoenix-cli/tests/benchmarks.rs` makes an input from a count (the same text for the same count; nothing large is kept in the repository), gives it to a reader, and measures the time. Each case is run on a **small** and on a **large** input, four times as many objects, several times, and the shortest time of the runs counts, so that a pause of the computer does not decide the result.

| Case | The input | What the reader must have found |
|---|---|---|
| KiCad board | tracks on a lattice in three nets, a quarter as many vias, a rectangular outline with openings, a zone with many corners | at least half as many tracks and vias as the count |
| KiCad schematic | one bus of 128 members and as many bus entries as the count, each ending at the label of a member | at least half the entries |
| EAGLE board | wires in signals of 100, a via at every fifth wire, a pour with many corners | at least half as many tracks and vias as the count |
| Gerber layer | as many draws and flashes as the count, three apertures | a Gerber file |
| Excellon file | as many holes as the count, three tools | a drill file |
| Rule check | tracks on a lattice in two nets, written as a board file, read and checked with the copper clearance | at least half the tracks, and no error |
| DXF outline | a large outline with `dxf_contours` small squares inside it, read and searched for the outline | all the squares found as cutouts |

A run that found less than it should would be fast and prove nothing, so the test fails on it.

A case fails when

* the large run takes longer than `max_ms`, or
* the large run takes more than `max_growth_percent` of the time of the small run (four times the work is 400 percent; work that grows with the square of the count takes about 1600 percent), when the small run took at least `min_ms_for_growth`; a shorter run is too short to compare.

The numbers are in `config/performance.cfg`, which is also the built-in default of the test (a test reads the file and requires that it is the default, that every line is required and that an unknown, a repeated or an out-of-range line is refused with its line number). Another file is named with the environment variable `PHOENIX_BENCH_CFG`, for example with tighter limits on a computer of your own.

| Key | Default | Meaning |
|---|---|---|
| `small_size` | 5000 | the objects of the small run |
| `large_size` | 20000 | the objects of the large run |
| `repeats` | 3 | how many times each run is made; the shortest counts |
| `max_ms` | 20000 | the longest time of the large run, in milliseconds |
| `max_growth_percent` | 1200 | the most the large run may take, in percent of the small run |
| `min_ms_for_growth` | 50 | a small run shorter than this is not compared |
| `dxf_contours` | 4000 | the closed contours of the DXF run; the reader refuses more than `max_contours` of `config/dxf.cfg` (5000) |
| `max_candidates_per_item` | 64 | the most candidates that the spatial index hands out for each track in the clearance check |

The tests of the benchmarks run one at a time (a lock in the test file), so that they do not slow each other down.

## Counting instead of timing

A time is noisy; a count is not. The copper clearance check is also tested by counting (`clearance_work`, the same count as `phoenix perf`): on the lattice board of the test, at both sizes, the candidates the index hands out stay below `max_candidates_per_item` for each track, and below one hundredth of the pairs a pass over every pair would compare. The same test requires that the check with the index and the check of every pair (`IndexMode::Naive`) give the same report, byte for byte in the JSON, on a board whose rows of tracks are too close.

## Work that grew with the square of a count, and was changed

Each change keeps the results: the order and the text of the findings do not move. The tests that cover them are named in the changelog.

| Where | What grew with the square | Now |
|---|---|---|
| Rule check (`check_board`) | the board library's own pass over every pair of tracks and of a track and a via ran as well, and its findings were dropped because the engine measures the same clearances with the spatial index | `Board::check_rules_with(rules, false)` leaves that pass out; `Board::check_rules` is unchanged |
| Pad clearance, mask bridge and silkscreen over pads (`Board::check_rules`) | a message was built for every pair of a pad and another object, even for a pair far apart that gives no finding; the straight pieces of every track were collected again for every pad | the distance test comes first and a message is built only for a finding; the pieces of every track are collected once |
| EAGLE XML reader | the test for an attribute name used twice compared each new name with all the earlier ones of the tag | from 32 attributes on, a set of the names is used |
| KiCad schematic reader | each bus entry searched every label, every wire and every bus; each pin of a symbol was compared with the earlier pins of the symbol | the labels, the ends of the wires and the members of the buses are looked up in maps built once, and the pins in a set |
| DXF outline search | the edges of every cutout were compared with the edges of every cutout after it, whatever their distance | contours whose boxes do not meet are not compared; the crossings found are the same, because edges of contours whose boxes do not meet cannot cross |

## Limits that bound the work that still grows with the square

| Work | Limit | File |
|---|---|---|
| Crossing test of the edges of one polygon | `max_polygon_points` (10000) | `config/board.cfg` |
| Rule areas that keep the pour out, tested against the zones (KiCad) | `max_keepout_checks` | `config/kicad.cfg` |
| Rings of the outline tested as openings (KiCad, EAGLE) | `max_outline_rings` | `config/kicad.cfg`, `config/eagle_board.cfg` |
| Restrict areas against the pours, ends of nets against the wires of a bus (EAGLE) | `max_restrict_checks`, `max_bus_checks` | `config/eagle_board.cfg` |
| Closed contours of a DXF drawing put in order by containment | `max_contours` (5000) | `config/dxf.cfg` |
| Edges of a DXF contour tested for crossings | `max_check_edges` | `config/dxf.cfg` |
| Objects, commands, region points of a Gerber file | `max_objects`, `max_commands`, `max_region_points` | `config/gerber_import.cfg` |

`--limits FILE` of every command sets the limits of the EAGLE readers and of the checks of a board for one run (`docs/tutorials/guides/TUTORIAL_LIMITS.md`).

## What still grows with the square of a count

These parts compare every object with every other one or with every object of another kind. They were read, not changed, and the benchmark does not cover them:

* the pad checks of the board library (pad against pad, pad against track, pad against via, the mask bridge between pads, silkscreen against pads), which are exact and need the whole list of pads;
* the overlap of zones of different nets on one layer, and the tracks, vias and pads against the edges of the board;
* the depth of the contours of a DXF drawing (which contour holds which), which compares the boxes of all contours and is bounded by `max_contours`;
* the arcs of the outline of a KiCad board, matched to the corners of the ring one by one;
* the fill of zones, the sharing of nets when a group is pasted and the order of objects when the layers are exported (`docs/SPATIAL.md`, `docs/NOT_YET.md`).

Moving these to the spatial index is work for a later version; each would need its own comparison of the results with and without the index, as the copper clearance check has.

## What is not verified

* No time was measured on a real board or on a file of a real design. The limits of the benchmark are generous, and a slow computer or a debug build of the test does not reach them; whether they are tight enough to catch a slowdown of a few times is not known.
* The growth test looks at two sizes only and at the shortest of a few runs; it can miss a slowdown that is not a power of the count, and it can fail on a computer that is busy with something else.
* The reader of EAGLE binary files does not exist, so it has no benchmark.
