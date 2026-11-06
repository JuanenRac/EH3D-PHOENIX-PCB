# EH3D-PHOENIX-PCB: the bulk import of libraries

**State: prototype.** `phoenix libindex <folder>` walks a folder of EAGLE and KiCad libraries, reads every footprint and symbol with the readers that already exist, judges each one, finds the ones that appear more than once and writes a deterministic index with the hash of the content of every item. It is the module `bulk` of the crate `eh3d-phoenix-ipclib`, the settings `config/import_bulk.cfg` and the codes `PHOENIX-LIBIDX-<NNN>` (`docs/ERROR_CODES.md`). It has only been run on libraries made by its own tests, never on a real collection of libraries.

The files are **only read**: nothing is copied, moved or changed, and the index holds names, hashes and findings, not the drawings. Nothing is written unless `--out` names a new `.json` file, and a file that exists is never written over.

## What it reads

| Extension | Kind | Items |
|---|---|---|
| `.lbr` | EAGLE XML library (`eagle_library`) | every footprint and every symbol of the file |
| `.kicad_mod` | KiCad footprint (`kicad_footprint`) | the footprint of the file (a `.pretty` folder is a folder of these files) |
| `.kicad_sym` | KiCad symbol library (`kicad_symbols`) | every symbol of the file, with the pins of every unit |

Other files are ignored. The walk is in the order of the sorted paths, so the same folder gives the same index. A file that cannot be read, or is not what its extension says, is listed with `PHOENIX-LIBIDX-020` and does not stop the others.

## The limits

The bounds are in `config/import_bulk.cfg`: the most files (`max_files`), the largest file (`max_file_bytes`), the deepest folder (`max_depth`) and the most items (`max_items`). What is left out is listed in `skipped` of the index, with its code, and never silently: `PHOENIX-LIBIDX-021` (a file that is too large), `022` (more files than the limit), `023` (a folder that is too deep) and `025` (more items than the limit).

**Links are never followed.** A symbolic link is skipped (`PHOENIX-LIBIDX-024`), and so is any path whose real path is outside the folder that was given, for example a junction. The index holds paths relative to the folder, written with `/`, and no path of the computer it was made on.

## The quality report

Every footprint and symbol has its findings (all of them are information for you to decide on; none stops the import):

| Code | About | What |
|---|---|---|
| `PHOENIX-LIBIDX-001` | footprint | Pads with the same name (the empty names of mounting holes do not count). |
| `PHOENIX-LIBIDX-002` | footprint | A pad without copper: no size, no copper layer. |
| `PHOENIX-LIBIDX-003` | footprint | A hole larger than the smaller side of its pad. |
| `PHOENIX-LIBIDX-004` | footprint | An origin farther than `origin_far_mm` from the box of the pad centres. |
| `PHOENIX-LIBIDX-005` | both | An empty name. |
| `PHOENIX-LIBIDX-006` | both | A footprint without pads or a symbol without pins. |
| `PHOENIX-LIBIDX-007` | symbol | Pins with the same name (EAGLE) or number (KiCad). |
| `PHOENIX-LIBIDX-010` | symbol | A pin with no name (EAGLE) or number (KiCad). |
| `PHOENIX-LIBIDX-008` | both | The same name in more than one place, in one library or in several. |
| `PHOENIX-LIBIDX-009` | footprint | The same geometry under another name. |

The reader of KiCad symbols keeps the number of each pin and not its name, so the findings about the pins of a KiCad symbol are about numbers, and a number that is used again in another unit of the same symbol counts as repeated.

### Duplicates

* **By name**: footprints with one name (and symbols with one name), in one library or in several. The group lists its members as `file::name`.
* **By geometry**: footprints whose pads have the same positions, sizes and holes, each rounded to `geometry_round_mm` (0.01 mm by default), whatever their names and whatever the format they came in; a footprint of an EAGLE library and one of a KiCad file with the same land pattern have the same geometry hash. The names of the pads are not part of it, and neither are the drawings (silkscreen, courtyard). A group is reported when its members have at least two different names (the same name is already in the first group).

The geometry is compared as it is drawn: a footprint turned or mirrored, or with another origin, has another hash. The sanitizer (`docs/SANITIZER.md`) is the tool that looks for near duplicates with turns, mirror and shift.

## The index

`phoenix-libindex/1` is JSON with the keys sorted, and holds:

* `summary`: the counts of files, footprints, symbols, items with findings, skipped paths and groups;
* `files`: for each file its path, kind, size, SHA-256, number of items and, if it could not be read, the code;
* `items`: for each footprint or symbol its file, kind, name, **hash of the content**, hash of the geometry (footprints with pads), number of pads or pins and findings (code, English title and detail);
* `duplicates`: `same_name` and `same_geometry`, each a list of groups with their members;
* `skipped`: the paths that were not read, with the code and the reason.

The hash of the content of an item is the SHA-256 of its name and of its drawing items in their order (EAGLE), or of its name and its sorted pads or pins (KiCad), so it changes when the item is edited and is the same for two copies. It is the same in two runs; the index has no date and no path of the computer.

## The command

```
phoenix libindex <folder> [--json] [--out INDEX.json] [--config FILE] [--lang xx]
```

Without options it prints the quality report as text (`--lang` writes the titles in `en`, `es`, `de`, `fr`, `it`, `ja`, `zh` or `pt`) and says that it was a dry run. `--json` prints the index instead. `--out` writes the index to a new file that must end in `.json`. The exit code is 0 when the index was made, whatever the report says, and 2 when it could not be (a folder that cannot be read, a bad option or settings file, an output file that exists).

## What it does not do

* Copy the libraries, merge them into one or choose a master among duplicates; the sanitizer and the library editor do that, with a confirmation.
* Read EAGLE binary files, KiCad 5 libraries (`.lib`, `.mod`) or other formats.
* Judge the drawings: silkscreen, courtyards and 3D models are not part of the quality report.
* Know whether a footprint is the right one for a part; it only finds faults in how it is drawn.
