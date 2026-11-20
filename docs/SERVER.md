# EH3D-PHOENIX-PCB: server mode

The crate `eh3d-phoenix-server` and the command `phoenix server` send a board, or a file of measurements, to EH3D-SERVER as a job of its plugin `pcb` and read back the report of the station. The program keeps working alone as it did before; this is another way to get a report, through the server that the machines and the other programs of the suite share.

**State: prototype.** It has been tried against a server in memory (the double of the client of the suite) and against a small HTTP server on the loopback address of the computer that answers the routes of the work queue the way the documentation of EH3D-SERVER describes them. It has never talked to a running EH3D-SERVER, and the station of the server is a simulator of the time a station takes: the reports themselves are made from the data the request brings.

```
phoenix server [--config FILE] [--kind rules|drill|bom|inspect] [--send] [--json] [--lang CODE] <board | measurements>
```

The board is a `.json` board, an EAGLE `.brd` or a KiCad `.kicad_pcb`, as for the other commands; for `--kind inspect` the file is a text of lines `name, nominal, tolerance, value[, unit]` (blank lines and lines that start with `#` are ignored; the unit has up to 8 characters). The exit code is 0 for a dry run and for a report whose verdict is a pass, 1 when the verdict of the server is not a pass, and 2 when the arguments, the input or the link are unusable.

## Nothing is sent by chance

Without `--send` the command makes the request and shows it (`PHOENIX-SERVER-012`); it opens no connection. To send it, **both** must hold: the settings say `allow_network = true` and the command is given `--send`. The program makes no other use of the network. The settings that come with the program have `allow_network = false` and no address.

## The kinds of report

| `--kind` | The request | What the station reports |
|---|---|---|
| `rules` (the default) | `rules_check`: the box of the outline, the width of every track, the smallest gaps between copper of different nets and every hole | `track_width`, `clearance`, `drill_small`, `drill_outside` and `drill_edge` errors and `drill_large`, `drill_duplicate` and `no_outline` warnings against the rules of the station |
| `drill` | `drill_report`: the holes of the vias and of the drilled pads, with their place and whether they are plated | the tools, the hits, the travel and an estimate of the time |
| `bom` | `bom_report`: the reference, value and footprint of every part | the parts grouped by value and footprint, repeated references (error) and parts with no value or footprint (warning) |
| `inspect` | `inspect`: the measurements of a file | every measurement against its tolerance |

The request is plain JSON, the same every time for the same board and settings; lengths are millimetres written from the integer nanometres of the board without rounding. The places of the holes are measured from the lower left corner of the box of the outline (from the origin of the board when the outline has no area, which is said). A **gap** is the distance between the copper of two tracks of different nets on the same layer, less the half widths of both, rounded down to a whole nanometre and never below 0; only gaps smaller than `clearance_below_nm` are listed, the smallest `clearance_max` of them, and the arcs of the tracks are compared as their chords (said in the report). A board with more than `clearance_max_tracks` tracks lists none, and the comparison stops at `clearance_pair_budget` pieces (a warning), so a very large board cannot hold the program.

The rules of the station (`min_track_mm`, `min_gap_mm`, `min_drill_mm`, `max_drill_mm`, `min_edge_mm`) are the ones of the machine on the server. The lines `rule_min_track_nm` and the like put the owner's own over them for this request.

## The link

The link is the client every program of the suite shares (`eh3d-serverclient` of EH3D-COMMON): the token of a program (made on the server for the scopes `jobs:prepare` and `jobs:start`), retries with growing pauses (a `POST` is never sent again once it may have reached the server), time limits, the state of the connection and the refusals of the server by their code, in the language of the report. The token cannot be printed by mistake: the settings never show it and no report holds it. A token for an address that is not local travels as plain text, which is a warning (`PHOENIX-SERVER-014`): use a tunnel or a network you trust.

The job is made with `POST /api/v1/work/jobs` (`{"machine", "request", "name", "app"}`, the application is `phoenix-pcb`, the name is the name of the board file), started with `POST /api/v1/work/jobs/{id}/start` and asked about with `GET /api/v1/work/jobs/{id}` every `poll_ms` until it ends `done`, `failed` or `cancelled`; the report is in the `view` of the job. A job that does not end within `max_polls` questions is stopped (`POST .../cancel`) and reported as timed out (`PHOENIX-SERVER-009`). With `start_job = false` the job is only made and waits in the queue until a person starts it.

## Settings

`config/server.cfg` holds every value and is also the built-in default.

| Key | Default | Meaning |
|---|---|---|
| `server_address` | empty | `host:port` of the server; empty means there is none |
| `server_token` | empty | the token of this program |
| `server_machine` | empty | the id of the board station the jobs are for |
| `server_timeout_s` | 15 | 1 to 120: how long a request may take |
| `server_retries` | 2 | 0 to 5: how many more times a request that failed on the network is tried |
| `server_mdns` | off | `on` or `off`: look for the server on the local network (the program has no resolver of its own) |
| `server_max_answer_mib` | 8 | 1 to 256 |
| `allow_network` | false | whether anything may be sent |
| `start_job` | true | whether the job is started after it is made |
| `poll_ms`, `max_polls` | 300, 400 | 10 to 60000 and 1 to 100000 |
| `max_items` | 50000 | 1 to 200000: the most tracks, gaps, holes, parts or measurements of a request |
| `clearance_max`, `clearance_below_nm`, `clearance_max_tracks`, `clearance_pair_budget` | 200, 1000000, 3000, 4000000 | the gaps of a rules request (0 lists none) |
| `rule_min_track_nm`, `rule_min_gap_nm`, `rule_min_drill_nm`, `rule_max_drill_nm`, `rule_min_edge_nm` | 0 | rules put over the ones of the station; 0 leaves the rule of the station |
| `max_listed` | 100 | 1 to 10000: the most problems a report lists |

## The report

The text report names the kind, whether the request was sent, the number of items, the job and its state, the verdict (`pass` or `fail`), the errors and warnings, the figures of the report (holes, travel, parts) and the problems one by one (the rule of the station, the item and its sentence), in the language chosen with `--lang` (the eight languages of the suite). `--json` prints the format `phoenix-server/1`: for a dry run the request itself, for a run the job, the verdict, the figures and the problems, the same bytes for the same answer.

## Codes

`PHOENIX-SERVER-001` to `PHOENIX-SERVER-014`, with titles in the eight languages (`src/codes.rs`) and the table in `docs/ERROR_CODES.md`.

## Limits

* Never tried against a running EH3D-SERVER, and the plugin of the server is a simulator: a `pass` here says that the request satisfied the rules of that station, not that a maker will build the board.
* The gaps are between the centre lines of straight pieces less the half widths: pads, vias and zones are not part of them, and the full rule check of this program (`phoenix check`) is the one to trust for the design.
* The request holds sizes, not the design: it carries the names of nets, layers, references, values and footprints, which leave the computer when it is sent.
