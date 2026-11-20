# EH3D-PHOENIX-PCB: server mode

The crate `eh3d-phoenix-server` and the command `phoenix server` send a board, or a file of measurements, to EH3D-SERVER as a job of its plugin `pcb` and read back the report of the station. The program keeps working alone as it did before; this is another way to get a report, through the server that the machines and the other programs of the suite share.

**State: prototype.** It has been tried against a server in memory (the double of the client of the suite) and against a small HTTP server on the loopback address of the computer that answers the routes of the work queue the way the documentation of EH3D-SERVER describes them. It has never talked to a running EH3D-SERVER, and the station of the server is a simulator of the time a station takes: the reports themselves are made from the data the request brings.

```
phoenix server [--config FILE] [--kind rules|drill|bom|inspect] [--send] [--json] [--lang CODE] <board | measurements>
phoenix server [--config FILE] --health [--json] [--lang CODE]
phoenix server [--config FILE] --job ID [--json] [--lang CODE]
phoenix server [--config FILE] --save-project PROJECT [--name NAME] [--tag TAG] [--json] [--lang CODE] <file>
phoenix server [--config FILE] --load-project PROJECT --name NAME --out FILE [--version N] [--json] [--lang CODE]
```

The board is a `.json` board, an EAGLE `.brd` or a KiCad `.kicad_pcb`, as for the other commands; for `--kind inspect` the file is a text of lines `name, nominal, tolerance, value[, unit]` (blank lines and lines that start with `#` are ignored; the unit has up to 8 characters). The exit code is 0 for a dry run and for a report whose verdict is a pass, 1 when the verdict of the server is not a pass, and 2 when the arguments, the input or the link are unusable.

## Nothing is sent by chance

Without `--send` the command makes the request and shows it (`PHOENIX-SERVER-012`); it opens no connection. To send it, **both** must hold: the settings say `allow_network = true` and the command is given `--send`. The program makes no other use of the network. The settings that come with the program have `allow_network = false` and no address.

## Asking the server

* `--health` asks the server whether it is there and well (`GET /api/v1/health`, which needs no login, so no token is sent) and prints its name, its version, its state and the revision of its API; the exit code is 0 when the state is `ok`, 1 when the server answered something else and 2 when it could not be reached. Only the switch `allow_network` and an address are needed.
* `--job ID` reads a job that is already on the server (`GET /api/v1/work/jobs/{id}`), once, and reports it like a run that sent the request: the state, and the verdict and the problems when the job has a report. It is for a job that was made with `start_job = false` and started by a person, or by another program, and for finding out how a job ended after the command that sent it was closed. A job that has not ended is not an error: its state says so, and as it has no verdict the exit code is 1. The id is letters, digits, `_` and `-`, up to 64 (anything else is refused before a request is made, `PHOENIX-SERVER-002`), and the settings must allow sending as for `--send`.

Neither takes a file, `--send` or the other.

## The project store

`--save-project` keeps a file of a board in the project store of the server, under the application `phoenix-pcb`, and `--load-project` reads it back. The store keeps every change as a new version and names the current one with a tag (its `ETag`). A write over a file that exists says which version it was made from (`If-Match`), so two people never overwrite each other without knowing:

```
phoenix server --save-project board-7 --name board.json board.json
phoenix server --load-project board-7 --name board.json --out back.json
phoenix server --save-project board-7 --name board.json --tag TAG board.json
```

* The first save of a file needs no tag and makes it (`created`). The command prints the tag of the version now current, and `--load-project` prints the tag of the version it read; the next save names it with `--tag`.
* A save over a file that exists without `--tag` is refused with `PHOENIX-SERVER-015` (the server's `428`), and one whose tag is no longer the current version with `PHOENIX-SERVER-016` (the `412`): nothing is overwritten, the file is read again and the person decides. A write is sent once, never again after a time-out, because the first try may have been applied.
* The same bytes again make no version (`unchanged`). `--version N` reads an older version. The name of the file defaults to the name of the file given; the project and the name are checked before anything is sent (`PHOENIX-SERVER-017`): letters, digits, `_`, `-` and `.`, the name in pieces joined by `/`.
* The file is read from, or written to, a path of the computer: `--out` must be a new file (`PHOENIX-SERVER-018`), and a file larger than `project_max_mib` is not sent. The settings must allow sending (`allow_network = true`), give an address and a token; `--send` does not belong to this use.
* `--json` prints the format `phoenix-server-store/1` (the action, the project, the name, the size, the tag, the version, and whether it was created or unchanged); the text is in the eight languages.

Tried against the server in memory of the client of the suite (the rule of the tag, the `428` and the `412`) and, by hand, against a small HTTP server on this computer; never against a running EH3D-SERVER.

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
| `project_max_mib` | 16 | 1 to 256: the largest file, in MiB, that is saved to the project store |

## The report

The text report names the kind, whether the request was sent, the number of items, the job and its state, the verdict (`pass` or `fail`), the errors and warnings, the figures of the report (holes, travel, parts) and the problems one by one (the rule of the station, the item and its sentence), in the language chosen with `--lang` (the eight languages of the suite). `--json` prints the format `phoenix-server/1`: for a dry run the request itself, for a run the job, the verdict, the figures and the problems, the same bytes for the same answer.

## Codes

`PHOENIX-SERVER-001` to `PHOENIX-SERVER-018`, with titles in the eight languages (`src/codes.rs`) and the table in `docs/ERROR_CODES.md`.

## Limits

* Never tried against a running EH3D-SERVER, and the plugin of the server is a simulator: a `pass` here says that the request satisfied the rules of that station, not that a maker will build the board.
* The gaps are between the centre lines of straight pieces less the half widths: pads, vias and zones are not part of them, and the full rule check of this program (`phoenix check`) is the one to trust for the design.
* The request holds sizes, not the design: it carries the names of nets, layers, references, values and footprints, which leave the computer when it is sent.
