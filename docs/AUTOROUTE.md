# EH3D-PHOENIX-PCB: routing by an external autorouter

The exchange with an external autorouter is the part of requirement PPCB-016 that was called the remote auto-routing task. It goes by files and has three parts, all in the crate `eh3d-phoenix-route` and in the command `phoenix autoroute`: the export of the board as a Specctra DSN file, the import of a Specctra session (SES) as a plan that is checked before anything is changed, and an optional adapter that starts a program. It reuses the settings of the router (`config/route.cfg`: default width, size of the vias, margin of the clearances) and the rules and net classes of a rule file.

**State: prototype.** It has been tried with boards and sessions made by hand and by the tests only. No autorouter has ever read a file written here, and no file written by an autorouter has ever been read. The adapter has never started a real program. Nothing here has been compared with the files of the reference tool. To route with no outside program see `docs/AUTOROUTE_NATIVE.md`.

## The DSN file

`export_dsn(board, context, settings)` returns the text of the file and a count of what is in it. The text depends only on the board, the rules and the settings: the same input always gives the same bytes (the maps of the board are ordered by id and everything is sorted by name). The coordinates are integer nanometres, written with `(unit um)` and `(resolution um 1000)`, so nothing is rounded on the way out.

* **Structure.** The copper layers in the order of the stack (`(type signal)`, with their index), the outline as a closed path, the vias that may be used, the general width and clearance, and every copper zone as a plane of its net.
* **Rules.** The width of a track is the one the net class asks for or the default of the settings, never below the narrowest track of the rules; the clearance is the largest of the clearance of the rules, the one of the class and the contact tolerance plus one nanometre, plus the margin of the settings. These are the values the interactive router uses. The edge clearance of the rules is not written (the file has no place for it that every router reads); it is checked on the result, when the session is imported.
* **Parts.** One image per component with its pads already in their place on the board: the side and the rotation of the component are applied while writing, and every component is placed on the front with no rotation. One padstack for each distinct pad shape and set of layers (`Pad_1`, `Pad_2`, ...); a pad with a drill is on every copper layer, a surface-mount pad on the layers it names; an oblong pad is written as a path of its width. Half of an odd length goes up to the next nanometre.
* **Nets.** Each net that has pads, with its pins (`U1-2`), and one class for every net class of the rules and one (`eh3d_default`) for the other nets, with the width, the clearance and the via of the class.
* **Wiring.** The tracks and vias that already exist, as fixed obstacles of their nets.
* **What is refused.** A name with a quote or a control character, two nets or two copper layers with one name, two pads of one component with one number, a reference designator with a hyphen, a board with no outline or no copper layer, a pad on no copper layer, a track with an arc piece and a via that does not join the first and the last copper layer. Each is `PHOENIX-ROUTE-021` with the cause in the detail.

## The session

`import_ses(board, context, settings, text, new_id)` reads a Specctra session and returns a `SesPlan`; the board that was read is never changed. The plan is all or nothing.

1. The text is read by a small reader of the bracketed syntax (depth and size limited). A text that is not a session, with no `routes`, no `resolution` or no `network_out` is `PHOENIX-ROUTE-022`.
2. The units of the session are converted exactly (`inch`, `mil`, `cm`, `mm` or `um`, with any integer resolution); a value is rounded to the nearest nanometre, halves away from zero.
3. Every net must be a net of the board, every layer a copper layer of the board and every wire a path of straight pieces with a width and a length. A via is read from the name of its padstack, which is the name the export wrote (`Via_<diameter>_<drill>_nm`); a via joins the first and the last copper layer. Any name or shape the board or the export does not know is `PHOENIX-ROUTE-023` and no part of the session is used.
4. A wire or via that the board already has (the fixed copper that a router writes back, a wire written the other way round) is counted and left out.
5. The new copper gets ids in a fixed order (net name, tracks before vias, layer, points), added as one operation to a copy of the board, and the rule check runs on the copy. If the result has an error that the board did not have, the plan is refused with `PHOENIX-ROUTE-009` and the first findings in the detail; if the board refuses the operation it is `PHOENIX-ROUTE-013`.
6. `SesPlan::require_complete` refuses (`PHOENIX-ROUTE-010`) a plan that leaves connections unrouted.

The report is the same for the same plan: the new tracks and vias and their length, what was left out, the connections to route before and after, the findings added and removed and one line per net. `SesPlan::apply` puts the plan on a board (a copy first, so a failure leaves it as it was).

## The adapter

`config/autorouter.cfg` holds the settings, and the adapter is off in the file that ships: `enabled = false`. To start a program two things are needed, `enabled = true` in the file and the option `--execute` on the command line. Anything else is a dry run that prints the command that would be started.

* The program is the absolute path in `executable`. It is started directly, never through a shell, with the arguments of the `arg` lines (`{dsn}` and `{ses}` stand for the two files), in a new folder inside `temp_dir`, with an empty environment except for the variables named in `env`. A variable whose name suggests a secret (key, token, secret, pass, credential, auth) is refused.
* `timeout_s` ends the program (`PHOENIX-ROUTE-027`). The program must leave the session where `{ses}` says (`PHOENIX-ROUTE-028` if it does not); a session larger than `max_session_bytes` is not read. An exit status other than success is `PHOENIX-ROUTE-026` with the end of its error output.
* The adapter opens no network connection and the settings accept no address (`://` or a network path). It does not stop the program from opening one: that is the choice of whoever sets `executable`, and the program is not confined by this code.
* The folder of the run is named after the design (a hash of the text) and is removed at the end unless `keep_temp = true`. Settings that cannot work are `PHOENIX-ROUTE-024`; a switched-off adapter asked to run is `PHOENIX-ROUTE-025`.

## The command

```
phoenix autoroute export-dsn [--settings FILE] [--rules FILE] [--json] [--lang CODE]
                             [--out FILE] <board>
phoenix autoroute import-ses [--settings FILE] [--rules FILE] [--json] [--lang CODE]
                             [--require-complete] [--out FILE] <board> <session>
phoenix autoroute run [--config FILE] [--settings FILE] [--rules FILE] [--json] [--lang CODE]
                      [--require-complete] [--execute] [--out FILE] [--ses-out FILE] <board>
```

Every sub-command is a dry run until an output file is named: `export-dsn` prints a summary, `import-ses` prints the report, `run` prints the command it would start. An output file is written only if it does not exist yet; the input is never overwritten. Exit code 0 is done and 2 is any refusal, with its code in the language of `--lang`.

## The codes

`PHOENIX-ROUTE-021` to `PHOENIX-ROUTE-028` are in `docs/ERROR_CODES.md` with their titles in the eight languages; the codes of the router (`001` to `020`) are used where they fit (`009`, `010`, `013`, `017`, `018`, `019` and `020`).

## What is not here

See `docs/NOT_YET.md`: arcs and blind or buried vias in the export, the shapes other than paths in a session, the confinement of the program, and every comparison with a real autorouter.
