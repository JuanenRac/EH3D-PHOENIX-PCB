# EH3D-PHOENIX-PCB: library sanitizer

The crate `eh3d-phoenix-sanitizer` and the command `phoenix sanitize` are the first step of phase F9 (requirement PPCB-021). Old EAGLE libraries pile up footprints, symbols and device sets that are duplicated or nearly duplicated. The sanitizer **analyses first**: it reads a folder of `.lbr` files, finds the duplicates, proposes which item of each group to keep and writes a report and a plan. It never rewrites a library and never merges anything by itself; the plan can be applied to a **copy** of the libraries, in a new folder, only with the confirmation code of the plan (see "Applying the plan"). It is a prototype tried only on libraries written by its own tests, never on real libraries.

## What it does

1. **Inventory in batch.** It walks the folder (links are never followed), reads every `.lbr` file with the EAGLE reader and the limits of the corpus crate (`max_files`, `max_depth`, `max_file_bytes`), and counts footprints, symbols and device sets per library. A file that is too large, cannot be read, is not XML, is an EAGLE binary file, belongs to another program or is a board or a schematic is noted with a code and left out; the walk goes on. A limit of the walk stops the run with an error, because a partial inventory would read as a complete one. Library files with the same SHA-256 are listed together.
2. **Canonical geometric hash** of every footprint and symbol (see below).
3. **Fuzzy matching** of the items whose hashes differ (see below), with a threshold from 0 to 100.
4. **Groups of duplicates.** Items with the same hash form an exact group; items joined by a score at or above the threshold form a close group. Device sets that use the same footprints and symbols (or equivalent ones, from the groups) with the same prefix and the same pin-to-pad connections form a group of device sets. Every group has an id (`P001` footprints, `S001` symbols, `D001` device sets), the item to keep, a score per member and the facts the owner needs to confirm it: whether the pads or pins are named the same way, whether a replacement would turn, mirror or shift the part, whether a member is below the threshold.
5. **Report** as one HTML page with no external resource (the style and the small script that sorts the tables are inline, a content policy forbids everything else, and every name read from a library is escaped) and the same report as JSON. The page is in one of the eight languages of the suite. Both are deterministic: the same libraries and settings give the same bytes, with no date, host name, user name or program version. The report names libraries and items, so it is for the owner of the libraries and is not meant to be published.
6. **Plan of the master library** as a JSON file: for each group, the item that would be kept and the items that would be replaced by it, with the points to review and the list of checks that the application of the plan makes for every step. The file says `"applied": false`: the file of a plan is never changed, and applying it makes a copy of the libraries.

## The command

```
phoenix sanitize [--out BASE] [--threshold N] [--config FILE] [--lang xx] [--json] <folder>
```

* Without `--out` it prints a short summary (or the JSON report with `--json`) and writes nothing.
* With `--out BASE` it writes `BASE.html`, `BASE.json` and `BASE.plan.json` (a temporary file and a rename, so a reader never sees half a file; the folder is created when it is missing). The libraries are only read.
* `--threshold N` overrides the threshold of the settings (0 to 100).
* `--config FILE` reads a settings file; `config/sanitizer.cfg` shows every key with its default.
* `--lang` is one of `en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt` (European Portuguese) and chooses the language of the page and of the error messages.
* Exit code 0 when done, 2 when the arguments, the settings, the folder or the output are unusable.
* `--apply` (with `--group`, `--allow-near`, `--confirm` and `--out`) works out and writes the copy described in "Applying the plan".

## The canonical geometric hash

The hash is a SHA-256 of a canonical text of the drawing items of a footprint or a symbol:

* Coordinates are rounded to a grid of `round_nm` nanometres (half away from zero) and angles to `angle_round_udeg` micro-degrees before anything else, so differences smaller than the grid disappear and a mirror or a turn commutes with the rounding.
* The order of the items does not matter: the lines of the canonical text are sorted.
* Names do not matter: the name of the item, of its pads and of its pins are left out. The text of a text item is kept, because it is drawing.
* A line is read in a fixed direction (an arc read backwards flips the sign of its angle), a rectangle is reduced to its centre, its sides and its angle modulo half a turn (a quarter turn swaps the sides), a pad to its normal orientation by the symmetry of its shape, and a polygon to the sorted set of its edges, so the vertex where it starts and the direction it runs do not matter.
* When the settings allow it, the item is also hashed under the eight symmetries of the square (`allow_rotation` for the turns by 90 degrees, `allow_mirror` for a mirror in X) and, with `ignore_origin`, with the corner of its pads moved to the origin; the smallest hash wins. All three are off by default, because a replacement that turns or shifts a part would change the board. The member of a group that needed a different symmetry from the item to keep is marked `turned`.
* A second, **named** hash adds the names of the pads and the pins and the electrical attributes of the pins, taken in the same frame. Two items with the same geometric hash and different named hashes number their pads in another way: the group says `names_agree: false` and the code `PHOENIX-SAN-009` is noted, because replacing one by the other would rewire the board.

## The score between two items

For two footprints (or two symbols) with the same number of pads (or pins) the score is a weighted mean, from 0 to 100, of:

* **pads or pins**: matched one to one by position within `tolerance_position_nm` (nearest pairs first, each used once); a pair is worth its closeness times the closeness of its size (`tolerance_size_nm`, drill included); an unmatched pad is worth nothing;
* **outline and silkscreen** of a footprint (the layers are lists in the settings), or the **body** of a symbol: lines, arcs, circles, rectangles and polygons sampled every `sample_step_nm` and compared as two sets of points within `tolerance_outline_nm`, the worse of the two directions;
* a part that is empty in both items is left out of the mean; a part that is empty in only one counts as zero.

The weights are `weight_pads`, `weight_outline` and `weight_silk` for footprints and `weight_symbol_pins` and `weight_symbol_body` for symbols. Items are compared only inside a bucket of the same number and kinds of pads, a pair whose boxes differ so much that it cannot reach the threshold is skipped by an exact bound, and the whole run is limited to `max_pairs` pairs (the limit is reported with `PHOENIX-SAN-007`). A group is the connected set of items joined by scores at or above the threshold; the score shown for a member is its score against the item to keep, and a member below the threshold is marked weak.

## The item to keep

`master_pick = richest` keeps the member with a description, then the one used by the most device sets of its own library, then the one with the most drawing items, then the first by library path and name. `master_pick = first` keeps the first by library path and name. The choice is a proposal; the plan can be edited by hand.

## The plan and its checks

The plan carries, under `preconditions`, the checks that every step passes before a file is written. The application below makes them for each step:

* `confirmed`: the plan was confirmed with its code, and a group can be chosen by its id;
* `backup`: the libraries are never written; the copy goes to a new folder and the bytes of the files that changed are kept by the undo;
* `footprint-same`: after the replacement every placed part keeps the position, size, shape and layer of each pad, because the footprint read back is exactly the master footprint;
* `pad-names`: the pads and pins of the kept item are named like those of the replaced one (the same names, counted), and so the pin-to-pad connections of every device set stay valid;
* `no-turn`: no replacement needs a turn, a mirror or a shift;
* `layout-unchanged`: nothing that uses the item by name moves, because the item keeps its name;
* `rules-unchanged`: the design rule check and the connectivity check of every board give the same findings before and after; **not made by this program**, because the boards are not touched: run `phoenix check` on a board that uses the copy;
* `reread`: the library written is read back and must be exactly the library before with the one item drawn as the master.

## Applying the plan

```
phoenix sanitize --apply [--group ID ...] [--allow-near] [--threshold N] [--config FILE] [--json] <folder>
phoenix sanitize --apply --confirm CODE --out NEWFOLDER [--group ID ...] [--allow-near] <folder>
```

The first form is the **dry run**, and it is what happens by default: it reads the folder, makes the plan, prints its **confirmation code** (12 hexadecimal digits in three groups, from the SHA-256 of the plan file, so a plan that has changed has another code) and lists, step by step, what would be done and what would be left out. Nothing is written. The second form needs the code of the plan that was read and a folder that does not exist; it writes there a copy of every library of the folder (the ones with a step and the ones without), with the same relative paths. The folder that was read is never written, a folder that exists is never written over, and the new folder cannot be inside the one that was read (`PHOENIX-SAN-008`). A wrong code writes nothing (`PHOENIX-SAN-014`).

A step replaces a footprint or a symbol of one library by the master item of its group **under the name it had**, so the device sets, schematics and boards that use it by name keep working and no part moves. Steps that cannot be made are left out and listed with their reason (`PHOENIX-SAN-016`, in the report):

| Reason | Why |
|---|---|
| `device-set` | only footprints and symbols are replaced |
| `turned` | the replacement would turn, mirror or shift the part |
| `not-exact` | the geometry is only close and not the same; `--allow-near` replaces these too |
| `names-differ` | the pads (or pins) of the master are not named the same way |
| `library-missing`, `library-unreadable`, `item-missing`, `item-ambiguous`, `not-utf8` | a library or an item of the step is not there, cannot be read, is named twice or is not UTF-8 text |
| `reread-differs` | the library read back is not the library before with the item drawn as the master |
| `same-item`, `not-chosen` | the item is the master itself, or the group was not chosen with `--group` |

The element is replaced in the text of the library, so what the sanitizer does not read (comments, the order of the sections, the layers, the other items) stays as it was. The layer numbers of the master are copied as they are: a master that comes from a library with other layers should be checked by the person.

In the library of the program, `apply_plan` returns the new bytes and an `Undo`: giving it the new files back restores exactly the old bytes, file by file. `preview_plan` is the dry run.

## Settings

`config/sanitizer.cfg` holds every limit, tolerance, weight and layer list. A missing line keeps the default; an unknown key or a value out of range is refused with its line number (`PHOENIX-SAN-006`).

## Codes

`PHOENIX-SAN-001` to `PHOENIX-SAN-016`, in `docs/ERROR_CODES.md`, with titles in the eight languages.

## Tests and limits

The tests write small libraries by hand (a generator in the test code); no library of a third party is copied into the repository. They cover the hash (order, names, rounding, arcs, rectangles, polygons, turns, mirror, shift), the score, the groups and the choice of the item to keep, the damaged and oversize files, the limits, the escaping of names in the page, the determinism of the report and that the libraries are not changed. The application of the plan is tested with the confirmation code (derived from the plan, wrong codes write nothing), the steps made and the steps left out with each reason, the footprints and symbols replaced under their own names, the device sets left as they were, the undo that gives back the same bytes, the copy in a new folder only and the refusal of a folder that exists or that is inside the one read.

Not done: it has not been run on real libraries, the thresholds and tolerances are defaults that nobody has tuned on them, the score does not look at texts and treats a pad of an unusual shape as the box of its size, the report is not available in redacted form, and the application of the plan has only been tried on libraries written by the tests: it does not relink the boards and schematics (they are not touched, and an item that keeps its name needs no relinking), it does not check the boards before and after, and EAGLE has not opened a library written by it.
