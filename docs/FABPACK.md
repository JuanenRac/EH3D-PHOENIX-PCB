# EH3D-PHOENIX-PCB: fabrication package

The module `fabpack` of the crate `eh3d-phoenix-export` and the command `phoenix fabpack` put the files a manufacturer or an assembler asks for in one deterministic `.zip` file, or in a new folder, with a manifest that says what is in it.

**State: prototype.** It has been tried on boards made by hand and by the tests only. The package has never been sent to a manufacturer or an assembler and the files in it have never been opened by an independent program; the zip file has been read back by the reader written for it and nothing else.

## What goes in

| Folder | Files | From |
|---|---|---|
| `gerber/` | one Gerber X2 file for each layer | the export profile (`docs/EXPORT.md`, `docs/GERBER_X2.md`) |
| `drill/` | the plated and not plated Excellon files | the export profile |
| `pnp/` | `pnp_top.csv`, `pnp_top.json`, `pnp_bottom.csv`, `pnp_bottom.json` (each face that has parts) | `docs/PNP.md` |
| `stencil/` | `stencil_top.gbr`, `stencil_bottom.gbr`, `stencil_report.csv` | `docs/STENCIL.md` |
| `bom/` | `bom_top.csv`, `bom_bottom.csv` (each face that has parts), `bom_board.csv` and `bom_assembly.json` | the bills of the assembly (`docs/BOM.md`) |
| (root) | `manifest.json` | this page |

The simple placement and bill files of the export profile are left out: the files of `pnp/` and `bom/` take their place.

## The manifest

`manifest.json` has the format `phoenix-fab-package/1`:

```
{
  "format": "phoenix-fab-package/1",
  "software": "EH3D PHOENIX-PCB <version>",
  "board": "<name>",
  "profile": "<export profile>",
  "units": "mm",
  "files": [{"bytes": 123, "path": "bom/bom_board.csv", "sha256": "..."}, ...],
  "notes": ["...", ...]
}
```

The files are listed in the order of their paths (bytewise), with their size and SHA-256; the manifest is not in its own list. `units` is the unit of the pick and place files. The notes say what the files cannot say: the package is a prototype that no manufacturer has accepted, a copper zone is written as its outline, the origin and the rotation rule of the pick and place files, and that the stencil settings are not the rules of a stencil maker; the warnings of the run (a part without a footprint, an opening below a ratio) are added to them.

## The zip file

No dependency is used: the zip file is written here, with every entry stored (method 0), the CRC-32 worked out by hand, UTF-8 names, no extra field, no comment and no attributes, and the date and time of every entry fixed at 1980-01-01 00:00:00, the first date a zip file can hold. The entries are in the order of the paths with the manifest last. Nothing reads the clock, the user or the system, so the same board, settings and profile give the same bytes. The limits are those of a plain zip file: 65 535 files and 4 GiB less one byte (`PHOENIX-FABPACK-006`); a name that is not a plain relative path (a `..` part, a backslash, a colon, a control character) is refused (`PHOENIX-FABPACK-002`).

Before the package is returned it is read back: the zip file is parsed, every CRC-32 and size is checked, and every file is compared with the manifest by size and SHA-256; a file the manifest does not list or lacks is a difference (`PHOENIX-FABPACK-003`). A package that is returned is one whose manifest agrees with its files.

## The command

```
phoenix fabpack [--profile NAME] [--profiles FILE] [--pnp-config FILE] [--stencil-config FILE]
                [--costs FILE] [--parts FILE] [--prices FILE] [--maker NAME] [--panel-boards N]
                [--dnp R1,R2,...] [--lang CODE] [--out FOLDER|FILE.zip] <board>
```

It is a dry run: it makes the package and reads it back in memory, and prints the list of files with their size and the start of their SHA-256, the findings and the number of notes. `--out` names a destination that must not exist: a name ending in `.zip` (any case) is a zip file, anything else a new folder with the files and the manifest. An existing destination is never replaced and nothing is written then (`PHOENIX-FABPACK-004`). `--panel-boards N` sets the boards of a panel for the quantities of the panel in the bills. Exit code 0 is done and 2 is any refusal. The codes `PHOENIX-FABPACK-001` to `008` are in `docs/ERROR_CODES.md` with their titles in the eight languages.

## What it does not do

* It does not send anything, sign anything or encrypt anything; the SHA-256 sums find damage, not tampering.
* It does not compress: a Gerber file is text and the package is larger than a compressed one.
* It does not put the files of a panel (`docs/PANEL.md`) in the package yet; a panel is written by `phoenix panel`.
* It does not write the formats that a particular manufacturer asks for (a fixed set of file names, a drill file in an old format).
