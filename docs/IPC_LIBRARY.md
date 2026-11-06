# EH3D-PHOENIX-PCB: the own library of footprints (IPC-7351B)

**State: prototype.** A library of footprints made by the method of IPC-7351B from tables of dimensions kept in data files, with a generator that gives each footprint its canonical name, a checker of the library and a comparison with a footprint of your own library. It is the crate `eh3d-phoenix-ipclib` (modules `family`, `generate`, `fp`, `check`, `compare`, `pads` and `settings`), the files `config/ipc_families/*.json` and `config/ipc.cfg`, and the command `phoenix ipc`. The bulk import of other libraries is a separate page (`docs/LIBINDEX.md`).

**None of the dimensions has been checked against a datasheet or against the text of the standard.** They were written by hand from the usual outline drawings of each package, and every family file says so (`"verified": false`). The names follow the pattern of IPC-7351B but have not been compared with the names of any other library, and the names of the headers and of the connectors are our own. No EAGLE or KiCad has opened a file written here. Before a board is made, compare every footprint with the datasheet of the part.

## What a family is

A family is one JSON file in `config/ipc_families/` (format `phoenix-ipc-family/1`; another folder is named with `--dir`). It holds:

* the **layout** (how the pads are placed): `two_terminal`, `dual`, `sot`, `quad`, `grid`, `tht_dual`, `tht_row` or `explicit`, and the **naming**;
* the **prefixes** of the names (`RESC`, `CAPC`, `INDC` and `LEDC` for the chips);
* the **tolerances** of the method, fabrication and placement, in millimetres;
* the **goals** of each density level: the fillet at the toe, the heel and the side of the lead and the excess of the courtyard (for a ball grid the ratio between the pad and the ball, for a through-hole family the extra size of the hole and the ring of copper);
* the **variants**: the sizes or packages, each with its dimensions and tolerances, and, if the dimensions of a variant need other goals, the ones it changes.

The loader refuses a file with an unknown key, another format, a missing density level, a variant without a dimension its layout needs, a negative number or a repeated family (`PHOENIX-IPC-001`).

| Family | Layout | Variants | Name |
|---|---|---|---|
| `chip` | two terminals | 0201, 0402, 0603, 0805, 1206, 1210, 1812, 2010, 2512 | `RESC1608X55N` |
| `melf` | two terminals | micromelf, minimelf, melf | `RESMELF3614X140N` |
| `sod` | two terminals | sod123, sod323, sod523 | `SODFL3716X110N` |
| `sot` | rows of pads, some places skipped | SOT-23 with 3, 5 and 6 pins, SC-70 with 3 and 6 pins, SOT-223 | `SOT95P240X110-3N` |
| `sot89` | pads written out | SOT-89 | `SOT150P450X150-4N` |
| `soic`, `ssop`, `tssop` | gull-wing leads on two sides | narrow and wide, two pitches, two widths | `SOIC127P600X175-8N` |
| `qfp` | gull-wing leads on four sides | LQFP of 32 to 100 pins, TQFP of 44 pins | `QFP50P900X900X160-48N` |
| `qfn` | flat terminals on four sides, exposed pad | 16 to 48 pins | `QFN50P300X300X80-17T170X170N` |
| `dfn` | flat terminals on two sides, exposed pad | 6, 8 and 10 pins | `DFN65P300X300X80-9T190X150N` |
| `bga` | regular grid of round balls | 10 x 10 at 0.8 mm, 8 x 8 at 0.5 mm, 12 x 12 at 1 mm | `BGA100C80P10X10_900X900X120N` |
| `dip` | through-hole, two rows | narrow and wide | `DIP762W50P254L927H508Q8N` |
| `header` | through-hole, one row or two | pitch 2.54, 2.0 and 1.27 mm | `HDR1X05P254N` |
| `connector` | through-hole, one row | screw terminals at 5.08 and 3.5 mm, 2.5 mm and 2.0 mm | `CONN_SCREW1X03P508N` |

## The method

The land of a lead is the dimension of the lead plus a fillet goal at the toe, the heel and the side, with the tolerances of the component (length across the leads, length of the foot, width), of the fabrication and of the placement added in quadrature:

* the outer edge of the land is at `Zmax = Lmin + 2 * toe + sqrt(CL^2 + F^2 + P^2)`;
* the inner edge is at `Gmin = Smax - 2 * heel - sqrt(CS^2 + F^2 + P^2)`;
* the width is `Xmax = Wmin + 2 * side + sqrt(CW^2 + F^2 + P^2)`;

where `L` is the overall length across the leads, `S` the space between the feet (`Lmax - 2 * Tmin`), `W` the width of a lead, `CL`, `CS` and `CW` their combined tolerances, `F` the fabrication tolerance and `P` the placement tolerance. The length of the land is `(Zmax - Gmin) / 2`, its centre is at `(Zmax + Gmin) / 4` and both are rounded to the grid of the family (0.01 mm). A family whose dimensions give lands that touch, have no size or cross the axis is refused (`PHOENIX-IPC-024`), and so is a pitch that is not larger than the width of the lands.

The three **density levels** are the ones of the standard: A (most copper, name ends in `M`), B (nominal, `N`) and C (least, `L`). They change the goals, and so the size of the lands, and the excess of the courtyard (0.5, 0.25 and 0.1 mm for the surface-mount families, 0.5, 0.25 and 0.12 mm for the through-hole ones). The goals that come with the program are the ones remembered for each kind of package and have not been checked against the tables of the standard.

## What a footprint has

Every footprint is made of integers in nanometres, with y up, and has:

* **pads** with a name, a centre, a size, a shape, a hole (zero for surface mount), the opening of the solder mask and the percentage of paste (100 for a lead; 60 for an exposed pad, which is a value of the family);
* **silkscreen** drawn outside the box of the pads at the distance of the family (0.2 mm) and with its width (0.15 mm): two lines for a chip with silkscreen, lines beside the pads of a package with leads on two sides, corner marks for a package with leads on four sides or a ball grid, a frame for a through-hole one, and a dot for pin 1 where it fits (the chips of 0201 and 0402 have none);
* the **assembly outline** (the body of the component) and the **courtyard**, which is the box of the pads and the body grown by the excess of the density level;
* the **origin** at the centre of the box of the pads.

Pin 1 of a package with leads on two sides is at the top left, the numbers go down the left side and up the right one. A package with leads on four sides is numbered counter-clockwise from the top of the left side, and its exposed pad has the next number. A ball grid names its balls by a letter for the row (the alphabet without I, O, Q, S, X and Z) and a number for the column. In a header with two rows the odd pins are in the upper row.

## The commands

All are dry runs: nothing is written unless `--out` names a new file, and a file that exists is never written over.

| Command | What it does |
|---|---|
| `phoenix ipc list` | The families, their state and their variants. |
| `phoenix ipc show <family> [<variant>]` | The dimensions of a family or of one variant, with the goals of each density level. |
| `phoenix ipc generate <family> <variant> [--param NAME=VALUE ...] [--density A\|B\|C] [--out NEW.lbr\|NEW.kicad_mod]` | Makes a footprint, checks it and prints its pads. The parameters are `pins` (families with a variable number of pins), `rows` (headers) and `prefix`. `--out` writes it as an EAGLE library, which is read back and compared before the command ends, or as a KiCad footprint. |
| `phoenix ipc check [--family NAME]` | Makes every variant of every family (with the least and the most pins) at each density level and checks the whole library. Exit 1 when it finds something. |
| `phoenix ipc compare <library.lbr\|file.kicad_mod> <family> <variant> [--name FOOTPRINT]` | Compares a footprint of your own library with the generated one. |

`--dir FOLDER` reads other family files, `--config FILE` other settings (`config/ipc.cfg` shows every key), `--json` prints JSON and `--lang xx` writes the reports in `en`, `es`, `de`, `fr`, `it`, `ja`, `zh` or `pt`. The density level is `default_density` of the settings when none is given.

## The check of the library

The codes are in `docs/ERROR_CODES.md`, each with its title in the eight languages. The checker finds:

| Code | What |
|---|---|
| `PHOENIX-IPC-002` | Two footprints with one name (without regard to case). |
| `PHOENIX-IPC-003` | Pads whose boxes overlap. |
| `PHOENIX-IPC-004` | The closest two pad centres are not at the pitch of the family (or, for the families that skip places, are closer than it). |
| `PHOENIX-IPC-005` | The middle of the box of the pads is not at the origin. |
| `PHOENIX-IPC-006` | A surface-mount pad without mask opening or without paste, paste above 100 per cent, a reduced paste above the limit of the settings, or a through-hole pad with paste or without a mask opening. |
| `PHOENIX-IPC-007` | Silkscreen closer to a pad than the clearance of the settings. |
| `PHOENIX-IPC-008` | A courtyard that is not the box of the pads and the body grown by the excess of the density level, or an excess smaller than the level asks for. |
| `PHOENIX-IPC-009` | A name with characters other than upper case letters, digits, `_` and `-`, or that does not start with its prefix or end with the letter of its density level. |
| `PHOENIX-IPC-010` | A pad without size, or a hole that is not smaller than the pad. |

The library that comes with the program passes its own check; that proves that the rules and the generators agree, not that a footprint is right for a part.

## The comparison with your own footprints

`phoenix ipc compare` reads one footprint (a package of an EAGLE library by its name, or a `.kicad_mod` file), makes the one the library would generate for the family and variant, and pairs the pads by name. For each pair it gives the differences of position, size and hole; a pair within `compare_tol_mm` of the settings agrees. It also tries the generated footprint turned by 90, 180 and 270 degrees and keeps the turn where most pads agree, because the same package may have been drawn the other way round. The report lists, with the codes `PHOENIX-IPC-040` (a pad differs), `041` (a pad of the generated footprint is missing from yours) and `042` (a pad of yours has no counterpart). Nothing is replaced and nothing is written; the exit code is 1 when pads differ.

## The names for the parts base

`phoenix parts check` (`docs/PARTS.md`) asks whether the footprint of each part of the base is a footprint that this library generates (or one of the libraries `.lbr` given with `--lib`). The names it knows are exactly those of the library, which are a prototype, so a name of the datasheets of a maker may have to come from your own library.

## What it does not do

* Dimensions checked against datasheets and against the tables of the standard (`docs/NOT_YET.md`).
* Round, trapezoidal or custom pads, thermal vias in an exposed pad, solder mask defined balls, pads with a missing ball in a grid, packages with leads of unequal pitch and the families that the standard has and this library does not (`SON`, `LGA`, `SOJ`, `PLCC`, `TO-` packages, crystals, inductors with their own outline, press-fit connectors).
* 3D models of the footprints.
* Writing into a library you already have: the generated footprints go to a new file.
