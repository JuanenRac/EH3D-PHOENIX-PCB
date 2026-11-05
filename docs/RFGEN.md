# EH3D-PHOENIX-PCB: RF structures

The crate `eh3d-phoenix-rfgen` draws microstrip structures for RF on the board: from the stack-up of the board and a frequency it calculates the dimensions, draws the copper and returns it as one reversible operation of the board together with a report. It builds on the calculator of lines (`eh3d-phoenix-stackup`), on the RF tools (`eh3d-phoenix-rf`: matching and filter prototypes), on the board model and its operations (`eh3d-phoenix-board`) and on the rules (`eh3d-phoenix-rules`).

**State: prototype.** Every formula is a published one (the source is in this page and in the report of each structure) and the tests use values of textbooks, but nothing here has been compared with an electromagnetic simulator or with the measurement of a board. A result is a first draft to be simulated, not a finished design. Discontinuities (steps of width, T junctions, the corners of a ring, the open ends of coupled lines, the inductance of a via) are not modelled, the substrate is taken as non-dispersive and the stack-up is the quasi-static model of `STACKUP.md`.

## What it does

* **Lines** (`line`, `bend`, `curve`, `taper`): a microstrip of an electrical length (the width comes from the synthesis of the stack-up crate, the length from the guided wavelength), a right-angle bend with the optimum miter, a right-angle bend as an arc of a track with straight arms of copper at each end, and a linear transition between the widths of two impedances.
* **Stubs and matching** (`stub-open`, `stub-short`, `stub-match`, `quarter-wave`, `l-match`): a shunt stub with an open end (shortened by the fringing field) or short-circuited by a via; the single stub that the RF tools find for a complex load, drawn on the line; a quarter-wave transformer between two real impedances; and an L network of a series line and a shunt stub (distributed elements) for a complex load.
* **Filters** (`lowpass`, `bandpass`): a stepped-impedance low-pass filter (Butterworth or Chebyshev prototype of the RF tools, Richards and Kuroda) and a coupled-line band-pass filter with open ends, in the staggered layout (impedance inverters, the even and odd mode impedances of every coupled section, and a search for the width and the gap that give them).
* **Couplers** (`wilkinson`, `branch-line`): the Wilkinson divider with its two quarter-wave arms (mitered corners, a gap between the output leads for the resistor) and the branch-line coupler as a rectangular ring with a lead at each corner.
* **Antennas** (`patch`, `dipole`): a rectangular patch by the transmission-line model with an inset feed that brings the edge resistance down to the feed impedance, and a printed half-wave dipole on two nets with a balanced feed.
* **Connector** (`sma`): an edge SMA launch: the footprint of the connector (a component with a signal pad and two ground pads and the nets of its pads), the line that leaves it, two ground strips along the line and a fence of vias that joins them to the reference plane, the vias a configurable fraction (1/20 by default) of the guided wavelength apart.
* **The board**: the result is one `Operation::Batch` that the board applies all or nothing and whose inverse restores the board byte for byte; the copper shapes are polygons of copper (`AddZone`), arcs are tracks, vias are vias and the connector is a footprint with a component (`AddFootprint`, `AddComponent`, `SetPadNet`). Nets are found by name or added. Ids that the board already uses are skipped.
* **The report**: the dimensions, the formulas with their source and the range in which the program trusts them (a warning when an input leaves it), the warnings, the ports in board coordinates and the rule check of the result, as text in the eight languages and as JSON.

## What it does not do

* No simulation and no optimisation: the dimensions come from closed formulas. The result is not tuned on the response of the structure.
* Nothing is modelled that the formulas do not contain: no step, T-junction, corner of a ring or bend compensation beyond the miter, no coupling between neighbouring arms (the arms of a Wilkinson divider run side by side), no open-end effect between facing resonators beyond the gap that is kept, no via inductance, no loss in the choice of dimensions, no dispersion, no thickness of the copper in the coupled-line model.
* The isolation resistor of a Wilkinson divider and the land pattern of a connector are generic: the resistor is not placed (a gap is left between the output leads) and the pads of the SMA launch have the sizes of `config/rfgen.cfg`, not those of a data sheet.
* A short-circuited stub grounds the signal net at DC. The via has the signal net and the report says so; add a DC block when the net must not be grounded.
* A series capacitance cannot be drawn: an L network is made only from a solution whose series element is an inductance that a line of `lmatch_series_line_ohm` and at most `lmatch_max_series_degrees` can stand for.
* A structure goes on the first copper layer of the stack-up (or the layer given); stripline, coplanar waveguide and inner layers are not drawn. The reference plane is taken as continuous; `ground_pour` can add a pour on it under the structure.
* Only the `.json` board and the EAGLE `.brd` can be written; a `.brd` written from a board with copper polygons is the output of the EAGLE writer of this repository, whose limits are those of `EXPORT.md` and `KICAD.md`.

## Formulas and sources

The numbers of the sections are those of the fourth edition of Pozar, *Microwave Engineering*.

| What | Formula | Source | Range checked |
|---|---|---|---|
| Microstrip impedance, effective permittivity, guided wavelength, track width for an impedance | quasi-static, with the synthesis of the stack-up crate | Hammerstad and Jensen 1980, through `eh3d-phoenix-stackup` (`STACKUP.md`) | `0.01 <= W/h <= 100`, `1 <= er <= 128`; no dispersion |
| Optimum miter of a right-angle bend | `M = 52 + 65 exp(-1.35 W/h)` per cent of the diagonal of the corner square; the cut crosses the outer edges at `2 M W` from the outer corner | Douville and James, IEEE Trans. MTT, 1978 | `W/h >= 0.25`, `er <= 25` |
| Open-end extension of a microstrip (stubs, resonators, patch) | `dL / h = 0.412 (e + 0.3)/(e - 0.258) * (W/h + 0.264)/(W/h + 0.8)` | Hammerstad, 5th European Microwave Conference, 1975; Balanis, *Antenna Theory*, section 14.2 | `0.1 <= W/h <= 100`, `1 <= er <= 20` |
| Open and short-circuited stub | `Zin = -j Z0 cot(theta)` and `j Z0 tan(theta)` | Pozar, section 2.3 | lossless line |
| Single-stub match of a complex load | distance and stub length solved by the RF tools | Pozar, section 5.2 (`eh3d-phoenix-rf`) | shunt stub of the impedance of the line |
| L network | the two topologies solved by the RF tools; the series inductance as a line `X = Zs sin(theta)` and the shunt element as an open stub `theta = atan(B Z0)` or a short stub `theta = atan(1 / (-B Z0))` | Pozar, section 5.1 (`eh3d-phoenix-rf`); the line as an inductance is an approximation | `theta` up to `lmatch_max_series_degrees` |
| Quarter-wave transformer | `Zt = sqrt(Z1 Z2)`, a quarter of the guided wavelength of the section | Pozar, section 5.4 (`eh3d-phoenix-rf`) | narrow band |
| Low-pass prototype values | Butterworth and Chebyshev `g` | Matthaei, Young and Jones; Pozar, section 8.3 (`eh3d-phoenix-rf`) | order up to `max_filter_order` |
| Stepped-impedance low-pass filter | inductor `beta l = g Z0 / Zh`, capacitor `beta l = g Zl / Z0` | Pozar, sections 8.5 and 8.6, example 8.5 | exact at the cutoff; sections up to 45 degrees (rule of thumb, a warning above) |
| Impedance inverters of the coupled-line band-pass filter | `Z0 J1 = sqrt(pi D / 2 g1)`, `Z0 Jn = (pi D / 2) / sqrt(g(n-1) g(n))`, `Z0 J(N+1) = sqrt(pi D / 2 gN g(N+1))` | Pozar, section 8.7; Hong and Lancaster, *Microstrip Filters for RF/Microwave Applications*, chapter 5 | narrow band (a warning above 20 per cent) |
| Even and odd mode impedances of a section | `Z0e = Z0 (1 + Z0 J + (Z0 J)^2)`, `Z0o = Z0 (1 - Z0 J + (Z0 J)^2)` | Pozar, section 8.7 | |
| Even and odd mode impedances and permittivities of coupled microstrips (marked prototype) | plate, fringing and air-gap capacitances of the pair, `Z = 1 / (c sqrt(C C_air))`, `eps = C / C_air` | Hammerstad and Jensen 1980 as collected by Garg and Bahl, IEEE Trans. MTT, 1979, and Gupta, Garg, Bahl and Bhartia, *Microstrip Lines and Slotlines* | `0.1 <= W/h <= 10`, `0.1 <= s/h <= 3`, `1 <= er <= 18`; zero thickness |
| Length of a coupled section | a quarter of the free-space wavelength over the square root of the mean of the even and odd permittivities, less the open-end extension | Hong and Lancaster, chapter 5 | |
| Wilkinson divider | arms `sqrt(2) Z0` a quarter wave long, resistor `2 Z0` | Pozar, section 7.3 | narrow band |
| Branch-line coupler | series sections `Z0 / sqrt(2)`, shunt sections `Z0`, each a quarter wave | Pozar, section 7.4 | narrow band |
| Patch antenna | `W = c / (2 f) sqrt(2 / (er + 1))`, `e = (er + 1)/2 + (er - 1)/2 (1 + 12 h/W)^(-1/2)`, `L = c / (2 f sqrt(e)) - 2 dL` | Balanis, *Antenna Theory*, section 14.2 (transmission-line model) | `h / lambda0 < 0.1`, `W/h >= 1` |
| Edge resistance and inset of the patch | `Rin = 1 / (2 (G1 + G12))` with `G1 = W / (120 lambda0) (1 - (k0 h)^2 / 24)` and `G12` by the integral of the mutual conductance (Simpson); `Rin(y0) = Rin(0) cos^2(pi y0 / L)` | Balanis, section 14.2 | `h << lambda0`; the feed impedance must not exceed the edge resistance |
| Ground plane of the patch | at least 6 h larger than the patch in both directions (reported, not drawn unless `ground_pour` is on) | Balanis | |
| Printed dipole | arm `= shortening * lambda0 / (4 sqrt((er + 1) / 2))` | rule of thumb for a conductor on a dielectric surface, no single source | no plane near the antenna |
| Curved bend | radius of a few widths (`curve_radius_over_width`) | rule of thumb of the microstrip literature, no single source | a warning below the setting |
| Taper | linear between the two widths, the length from the mean guided wavelength | tapered lines, Pozar, chapter 5 | a warning below a quarter wave |
| Via fence | vias a fraction (`fence_spacing_wavelengths`, 1/20) of the guided wavelength apart | common design practice, no single source | |

## The command

```text
phoenix rfgen --list [--json]
phoenix rfgen <type> --stackup FILE --freq-ghz GHZ [--layer NAME] [--board FILE] [--out FILE]
              [--settings FILE] [--stackup-settings FILE] [--net NAME] [--ground-net NAME]
              [--x-mm X] [--y-mm Y] [--rotation DEG] [--name TEXT] [--json] [--lang CODE]
              [--<parameter> VALUE ...]
```

`phoenix rfgen --list` prints the types with the parameters of each. The stack-up file is JSON with `stackup` and `materials` as in the job file of `phoenix stackup` (which is accepted as it is; its lines are ignored). `--freq-ghz` is the frequency of the design; for a filter it is the cutoff or the centre frequency, and the materials are interpolated at it. Without `--board` a new board is made with one copper layer for each copper layer of the stack-up; with it, the structure is added to a board read from an EAGLE `.brd` or a `.json` file, whose copper layers must have the names of the stack-up. With `--out` the board with the structure is written to a new file (`.json` canonical JSON, `.brd` EAGLE XML); an existing file is never replaced and without `--out` nothing is written. A parameter of the type is written `--length-mm 12.5` or `--length_mm 12.5`. `--lang` selects the language of the text of the report and of the titles of the errors.

Exit codes: 0 done, 1 the structure was made (with `rules_gate = false`) and the rule check finds a new error, 2 the arguments, the input or the design are unusable.

Examples:

```text
phoenix rfgen line         --stackup fr4.json --freq-ghz 1   --out line.json
phoenix rfgen bend         --stackup fr4.json --freq-ghz 2   --turn right --board line.json --y-mm 20 --out two.json
phoenix rfgen bandpass     --stackup fr4.json --freq-ghz 2.4 --order 3 --ripple-db 0.5 --bandwidth-percent 10
phoenix rfgen patch        --stackup fr4.json --freq-ghz 2.4
phoenix rfgen sma          --stackup fr4.json --freq-ghz 2.4 --length-mm 10
```

On 1.6 mm of FR-4 (`er = 4.4`) the first draws a 50 ohm line about 3.0 mm wide and 41 mm long, a quarter of the guided wavelength at 1 GHz; the fourth a patch of about 38.0 by 29.4 mm with a 50 ohm feed that enters 9.2 mm; the third the filter of the example of Pozar (`Z0e`, `Z0o` of 70.6 and 39.2 ohm in the outer sections and 56.6 and 44.8 ohm in the inner ones).

## The geometry

* **Frame.** A generator draws in a frame where the signal runs along x. The placement (`--x-mm`, `--y-mm`, `--rotation`) moves the origin and turns the frame by a multiple of 90 degrees; the turn is exact in integers.
* **Units.** The model is in integer nanometres. The only place where a length in metres becomes nanometres is `nm_from_m`, which rounds to the nearest nanometre with halves away from zero and refuses a number that is not finite or does not fit the range of the model. Widths of lines are made even (one nanometre is added when needed) so that a line is centred exactly on its axis.
* **Shapes.** Straight pieces, tapers, bends, patches and the arms of a divider are polygons of copper with their exact outline (so that the length is the length and a track does not add the half width of its round end). A curved bend is the only arc: a track of one arc whose round ends lie inside the straight arms. Parts (the SMA connector) are components with pads.
* **Nets.** The signal net is `RF` and the ground net `GND` unless `--net` and `--ground-net` say otherwise. Copper that is one piece is one net; the resonators of a band-pass filter are separate pieces and have the nets `RF_R1`, `RF_R2`, ... between `RF` (input) and `RF_OUT`; the second arm of a dipole is `RF_B`.
* **Rules.** The board is checked by `eh3d-phoenix-rules` before and after, with the limits of `config/rfgen.cfg`. A structure that adds an error is refused (`rules_gate`), and the report says how many errors there were before and after. The rules do not yet judge the clearance between copper polygons, only the overlap of polygons of different nets, the clearance written in a zone, tracks, vias and pads (`RULES.md`).

## Settings

Every default and limit is in `config/rfgen.cfg` and none is fixed in the code: the default impedance and electrical length, the length of the leads, the minimum arm of a mitered bend, the radius of a curve, the length of a taper, the main line of a stub, the series line of an L network, the drill and diameter of the vias, the impedances of the low-pass sections, the gap between facing resonators, the default bandwidth, the highest filter order, the tolerance and the step limit of the search for a coupled pair, the pitch of the arms of a divider and the gap for its resistor, the notch of a patch, the shortening and the strips of the dipole, the pads of the SMA launch, the spacing of the fence, the ground pour, the clearance of the zones, the smallest shape, the limits of the rule check of the result, the gate, the most items and the number of ids tried.

## Errors

The codes `PHOENIX-RFGEN-001` to `PHOENIX-RFGEN-019` are in `ERROR_CODES.md`; 017 to 019 are warnings that appear in the report. Their titles exist in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese); the detail of a message is in English.

## How it is tested

The formulas are checked against values of the books (the miter of a line as wide as the substrate is high, the open-end extension, the inverters and the impedances of the band-pass example of Pozar, the lengths of the stepped-impedance filter of Pozar's example 8.5, the width, length, edge resistance and inset of a patch at 2.4 GHz on FR-4, the zeros and values of the Bessel function used in the integral) and against properties (the even mode is the higher impedance, the search returns the impedances it was given, the coupled pair tends to the single line when the gap grows). On a whole board, every type of structure on a two-layer board of 1.6 mm FR-4, and the types that fit it on a four-layer board with a thin prepreg, must pass the rules, leave a consistent board and be undone exactly (the board after the inverse has the same bytes as before), and the same request must give the same bytes and the same report. The operations that add zones and footprints have their own tests in the board crate, and the command is tested on stack-up files written by the tests.
