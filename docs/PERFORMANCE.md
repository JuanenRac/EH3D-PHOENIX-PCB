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
| Rule check with pads | one pad for each part on a lattice 2 mm by 3 mm, half as many short tracks and an eighth as many vias between the rows, written as a board file, read and checked with the copper clearance and the bridge of solder mask | all the parts, and no error |
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
| `outline_edges` | 400 | the pieces of the round outline of the board of the edge clearance test |
| `max_edge_candidates_per_item` | 16 | the most pieces of the outline that the index hands out for each track in the edge clearance check |

The tests of the benchmarks run one at a time (a lock in the test file), so that they do not slow each other down.

## Counting instead of timing

A time is noisy; a count is not. The copper clearance check is also tested by counting (`clearance_work`, the same count as `phoenix perf`): on the lattice board of the test, at both sizes, the candidates the index hands out stay below `max_candidates_per_item` for each track, and below one hundredth of the pairs a pass over every pair would compare. The same test requires that the check with the index and the check of every pair (`IndexMode::Naive`) give the same report, byte for byte in the JSON, on a board whose rows of tracks are too close.

The checks of the pads are counted the same way (`Board::pad_check_work`): on the board of pads, at both sizes, the pads, tracks and vias that the grids hand out stay below `max_candidates_per_item` for each pad and below one hundredth of the comparisons of the pass over every pair, and a clearance whose reach is too large for a grid (`pad_check_work` gives nothing) is checked by the pass over every pair. The same test requires that the grids and the pass over every pair give the same report, byte for byte in the JSON, on a board whose pads are too close.

The edge clearance check of tracks and vias is counted too (`edge_work`): on the lattice board with a round outline of `outline_edges` pieces, at both sizes, the pieces of the outline that the index hands out stay below `max_edge_candidates_per_item` for each track and below a fifth of the pieces a pass over every piece would compare, and the check with the index and the check of every piece give the same report, byte for byte in the JSON. The outline is looked up in the index from `edge_index_min_edges` pieces on (`config/spatial.cfg`); a track or via beyond 537 mm from the origin is compared with every piece, as it always was, so that a number too large for the exact test is still reported as inconclusive.

## Work that grew with the square of a count, and was changed

Each change keeps the results: the order and the text of the findings do not move. The tests that cover them are named in the changelog.

| Where | What grew with the square | Now |
|---|---|---|
| Edge clearance of tracks and vias in the rule check (`check_board`) | each piece of copper was compared with every piece of the outline and of its openings, which is slow for a round outline of hundreds of pieces | the pieces are looked up in the spatial index from `edge_index_min_edges` pieces on; the report is the same |
| Rule check (`check_board`) | the board library's own pass over every pair of tracks and of a track and a via ran as well, and its findings were dropped because the engine measures the same clearances with the spatial index | `Board::check_rules_with(rules, false)` leaves that pass out; `Board::check_rules` is unchanged |
| Pad clearance, mask bridge and silkscreen over pads (`Board::check_rules`) | a message was built for every pair of a pad and another object, even for a pair far apart that gives no finding; the straight pieces of every track were collected again for every pad | the distance test comes first and a message is built only for a finding; the pieces of every track are collected once |
| Pad checks (`Board::check_rules`) | each pad was compared with every other pad (clearance and bridge of solder mask), with every track and with every via, which is how a board of several thousand pads took seconds | the pads and the vias are looked up in a grid of cells as wide as the reach of the rule, the pieces of the tracks in the cells their box grown by the reach covers (a piece that would need more than 256 cells is tried for every pad); everything left out is certainly too far to give a finding, so the findings, their order and their text are those of the pass over every pair, which is still made when the reach is above `2^28` nanometres. The unit tests compare the two on random boards, with long slanted pieces and pads that are not round |
| Touching copper (`analyse_board`, part of the check after each operation of the board editor and of opening a board in it) | every piece of copper was compared with every other one | the pieces are looked up in the spatial index; the report is the same (a test compares the two on random boards at several tolerances) |
| Islands of a net (`Board::connectivity`, the ratsnest) | every item of a net was compared with every other item of that net, which is slow for a net of power or ground | only items whose boxes meet are compared, from left to right; a test checks that two items that touch always have boxes that meet |
| EAGLE XML reader | the test for an attribute name used twice compared each new name with all the earlier ones of the tag | from 32 attributes on, a set of the names is used |
| KiCad schematic reader | each bus entry searched every label, every wire and every bus; each pin of a symbol was compared with the earlier pins of the symbol | the labels, the ends of the wires and the members of the buses are looked up in maps built once, and the pins in a set |
| DXF outline search | the edges of every cutout were compared with the edges of every cutout after it, whatever their distance | contours whose boxes do not meet are not compared; the crossings found are the same, because edges of contours whose boxes do not meet cannot cross |

## Times of the engine of the board editor

`crates/eh3d-phoenix-cli/tests/engine_bench.rs` makes boards of 1000, 4000 and 12000 pads with their tracks and vias and prints the time of the connectivity, the touching copper, the rule check, the findings of the editor, the ratsnest, the preparation of the drawing, opening the board in the editor, one edit, its undo and its redo: `cargo test --release -p eh3d-phoenix-cli --test engine_bench -- --ignored --nocapture`. It has no limit and is left out of a normal run; it is meant to be run on one computer before and after a change.

## Times of the push of the board editor

Measured by `crates/eh3d-phoenix-boardedit/tests/shove_bench.rs` (a long test, left out of a normal run) in a release build on a computer of the CI, on boards that the test makes: pairs of long tracks 0.5 mm apart between pads, with a via of another net 0.65 mm below each pair. Every push of the table moves the two tracks of a pair and is kept by the checks.

The two times of a push are given before and after only the zone that changed was checked, both measured in one run of the CI.

| Board (tracks, vias, pads) | The design rules of the whole board | Preview of a push, whole board | Preview of a push, zone | The push applied, whole board | The push applied, zone |
|---|---|---|---|---|---|
| 200, 100, 400 | 3 ms | 4 ms | 1 ms | 7 ms | 2 ms |
| 1000, 500, 2000 | 15 ms | 22 ms | 5 ms | 39 ms | 9 ms |
| 4000, 2000, 8000 | 80 ms | 143 ms | 44 ms | 234 ms | 55 ms |

Before, most of the time of a push was the design rule check of the whole board, one on the copy in a preview and another when it was applied. Now the rules judge the zone that changed and the connectivity is worked out for the nets of the push only (`local_checks`, `docs/BOARD_EDIT.md`), with the same answer; most of what is left in these pushes is the planning of the walk around the via. On the same boards the version before refused every one of these pushes, because the lower track would have been pushed into the via (`PHOENIX-SHOVE-001`), so its times are not compared.

**The walk looks only at what is near.** The pads, vias and tracks near a place are found in grids. Measured before and after in one run of the CI, on the same boards (the preview and the push applied):

| Board (tracks, vias, pads) | Preview, every object | Preview, grids | Applied, every object | Applied, grids |
|---|---|---|---|---|
| 200, 100, 400 | 1.1 ms | 1.3 ms | 1.7 ms | 1.9 ms |
| 1000, 500, 2000 | 6.4 ms | 6.1 ms | 9.2 ms | 8.8 ms |
| 4000, 2000, 8000 | 58 ms | 27 ms | 69 ms | 38 ms |

On small boards the grids cost a little more than they save.

**The check while dragging.** `engine_bench.rs` also times the check of one place of a drag (a component moved, the zone that changed judged), in one run of the CI:

| Board (pads, tracks, vias) | Check of one place of a drag |
|---|---|
| 1000, 490, 250 | 0.5 ms |
| 4000, 1960, 1000 | 2.0 ms |
| 8000, 3920, 2000 | 4.1 ms |
| 12000, 5880, 3000 | 6.6 ms |

The largest board has about 21 000 components, tracks and vias, so `drag_drc_max_objects` is 25 000 by default. The times are those of a computer of the CI and of synthetic boards; nothing has been measured on a screen or on a real design.

**What is done more often.** On 300 random pushes on small boards made with a fixed seed (`shove_props.rs`), pushing in parallel alone does 154 and the push with walks and via moves does 247; 24 of the 53 that are refused have an end of the new track at a pad of another net, which no push can solve. These are synthetic boards: they say nothing about how often a push succeeds on a real design.

## The autorouter

Measured by `crates/eh3d-phoenix-autoroute/tests/quality.rs` (the long table) in a release build, before and after the autorouter made room for the connections that are left, in one run of the CI. The boards are synthetic.

| Board | Connections routed before | Time before | Connections routed after | Time after |
|---|---|---|---|---|
| small | 56 of 56 | 0.7 s | 56 of 56 | 0.7 s |
| medium | 149 of 149 | 2.2 s | 149 of 149 | 2.3 s |
| dense | 155 of 179 (86.6 %) | 4.9 s | 167 of 179 (93.3 %) | 9.2 s |
| large | 429 of 440 (97.5 %) | 17.9 s | 431 of 440 (98.0 %) | 20.2 s |

A search that makes room may expand `rescue_search_expansions` nodes (50000). With the limit of an ordinary search instead, the large board reached 437 of 440 in some settings tried and took about a third longer than before; the results of this search change a lot with its settings, so these are not general figures.

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

* the silkscreen against the pads, and the pads and the pieces of copper of `Board::check_rules` against the pieces of the edge of the board (each against each);
* the overlap of zones of different nets on one layer, and the tracks, vias and pads against the edges of the board;
* the depth of the contours of a DXF drawing (which contour holds which), which compares the boxes of all contours and is bounded by `max_contours`;
* the arcs of the outline of a KiCad board, matched to the corners of the ring one by one;
* the fill of zones, the sharing of nets when a group is pasted and the order of objects when the layers are exported (`docs/SPATIAL.md`, `docs/NOT_YET.md`).

Moving these to the spatial index is work for a later version; each would need its own comparison of the results with and without the index, as the copper clearance check has.

## What is not verified

* No time was measured on a real board or on a file of a real design. The limits of the benchmark are generous, and a slow computer or a debug build of the test does not reach them; whether they are tight enough to catch a slowdown of a few times is not known.
* The growth test looks at two sizes only and at the shortest of a few runs; it can miss a slowdown that is not a power of the count, and it can fail on a computer that is busy with something else.
* The reader of EAGLE binary files does not exist, so it has no benchmark.
