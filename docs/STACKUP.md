# EH3D-PHOENIX-PCB: stack-up and transmission lines

The crate `eh3d-phoenix-stackup` holds the layers of a board and the analytic calculation of transmission lines on them: impedance, effective permittivity, conductor and dielectric loss, delay, guided wavelength, and the track width that gives a wanted impedance. The command `phoenix stackup <job.json> --freq-ghz <GHz>` runs it.

**State: prototype.** The formulas are published ones, applied with checks of their range. Nothing here has been compared with a field solver or with the measurement of a real board, and the results are quasi-static estimates, not a simulation. Requirement PPCB-024 (RF tools by levels, analytic calculators first) is `prototype` because of this crate, and no further.

## What it does

* A stack-up of layers from the top of the board to the bottom: copper (in ounces per square foot or in micrometres, with an RMS roughness), cores, prepregs and solder mask. Every thickness is an integer in nanometres.
* The dielectric constant and the loss tangent of a material at a frequency, interpolated between the points a data sheet publishes.
* Microstrip, stripline (centred or offset), coplanar waveguide with and without a ground plane under the substrate, and the edge-coupled pair of microstrips or striplines.
* Conductor loss (skin effect and roughness) and dielectric loss in dB/cm at a frequency, the propagation delay in ps/mm and the guided wavelength.
* The synthesis of a track width (or the differential impedance of a pair) by bisection on the direct calculation.

## What it does not do

* No dispersion, no radiation, no frequency-dependent impedance: the models are quasi-static. A warning is raised when the substrate is a large part of the guided wavelength.
* No solder mask in the field: a mask over the line is reported (`PHOENIX-STACK-015`) and left out.
* No coupled-line even and odd modes, no common-mode impedance, no crosstalk, no via, no discontinuity, no frequency sweep.
* No plane with holes, no split planes and no return path: a reference plane is a continuous copper layer.
* No etch factor: a track is a rectangle of the drawn width.

## The job file

A JSON document with three parts. Unknown fields are refused.

```json
{
  "stackup": {
    "name": "two-layer",
    "layers": [
      {"kind": "copper", "name": "top", "thickness": {"oz": 1.0}, "roughness_rms_nm": 500},
      {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4"},
      {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}
    ]
  },
  "materials": [
    {"name": "fr4", "dk_points": [[1.0, 4.5], [10.0, 4.3]], "df_points": [[1.0, 0.02], [10.0, 0.022]]}
  ],
  "lines": [
    {"name": "clock", "kind": "microstrip", "layer": "top", "target_ohm": 50.0},
    {"name": "bus", "kind": "differential_microstrip", "layer": "top", "width_nm": 150000, "gap_nm": 150000}
  ]
}
```

The values above are an example of the format, not the data of a laminate.

* **Layers.** `copper` (a `thickness` of `{"oz": x}` or `{"um": x}`, and an optional `roughness_rms_nm`), `core`, `prepreg` and `solder_mask` (a `thickness_nm` and a `material`). At most 64 layers; unique names; at least one copper layer; no two copper layers next to each other; solder mask only as the first or the last layer; every thickness from 1 nm to 10 mm; roughness from 0 to 50 000 nm. One ounce is 34 798 nm (the convention of IPC-2221: 1.37 mil per ounce).
* **Materials.** `dk_points` and `df_points` are lists of `[frequency in GHz, value]` by increasing frequency (at most 256 points), a Dk from 1 to 100 and a Df from 0 to 1. `thickness_nm` is optional and informative.
* **Lines.** `kind` is one of `microstrip`, `stripline`, `coplanar`, `coplanar_grounded`, `differential_microstrip` and `differential_stripline`. Give exactly one of `width_nm` and `target_ohm` (for a pair the target is the differential impedance). `gap_nm` is the width of each slot of a coplanar line and the edge-to-edge spacing of a pair; the other kinds must not have one.

The file is limited to 4 MiB, 1000 lines and 256 materials.

## How a line is taken from the stack-up

The track is on a copper layer. From that layer the crate walks up and down through the dielectric layers until the next copper layer (the reference plane) or the end of the board.

* **Microstrip** and the pair of microstrips: dielectric and a copper plane on one side, only air or solder mask on the other.
* **Stripline** and the pair of striplines: dielectric and a copper plane on both sides.
* **Coplanar:** the dielectric is on one side and air on the other. Without the plane (`coplanar`) a plane under the substrate is not part of the model and the crate says so (`PHOENIX-STACK-016`); with it (`coplanar_grounded`) the plane must exist.
* The dielectric layers between the track and the plane are combined as capacitors in series: the permittivity of the run is `sum(h) / sum(h / er)` and its loss tangent is `sum(h tan / er) / sum(h / er)`. Dk and Df are those of each material at the frequency of the calculation.
* The roughness used for the conductor loss is the mean of the roughness of the track layer and of the planes that carry the return current.

## Models, sources and ranges

All lengths are in metres inside the formulas. Where a source gives a range, the crate raises `PHOENIX-STACK-012` outside it and still returns the number. Where the formula has no meaning at all (a slot closed by the thickness of the copper, a track as thick as the gap between planes) the result is the error `PHOENIX-STACK-011`. A limit that is a choice of this implementation and not a published bound is named as such in the code.

### Microstrip

E. Hammerstad and O. Jensen, "Accurate models for microstrip computer-aided design", IEEE MTT-S International Microwave Symposium Digest, 1980. With `u = W/h`:

```text
F       = 6 + (2 pi - 6) exp(-(30.666 / u)^0.7528)
Z01     = (eta0 / 2 pi) ln(F / u + sqrt(1 + (2 / u)^2))
a       = 1 + ln((u^4 + (u / 52)^2) / (u^4 + 0.432)) / 49 + ln(1 + (u / 18.1)^3) / 18.7
b       = 0.564 ((er - 0.9) / (er + 3))^0.053
eps_eff = (er + 1) / 2 + (er - 1) / 2 (1 + 10 / u)^(-a b)
Z0      = Z01 / sqrt(eps_eff)
```

Published accuracy: 0.2 % for the permittivity with `0.01 <= u <= 100` and `1 <= er <= 128`; 0.01 % for `Z01` up to `u = 1` and 0.03 % up to `u = 1000`. Outside `u` from 0.01 to 100 or `er` from 1 to 128 the crate warns.

Thickness `t` of the copper (K. C. Gupta, R. Garg, I. Bahl and P. Bhartia, "Microstrip Lines and Slotlines", 2nd edition, Artech House, 1996): the width in `u` is the effective width `W + (t/pi)(1 + ln(4 pi W / t))` when `W/h <= 1/(2 pi)` and `W + (t/pi)(1 + ln(2h / t))` above it, and the permittivity is lowered by `(er - 1)/4.6 (t/h) / sqrt(W/h)`. The crate warns above `t/h = 0.2` (a limit of this implementation).

### Stripline

Planes `b` apart, a centred track of width `W` and thickness `t`, `x = W / (b - t)`.

* Wide strip (S. B. Cohn, IRE Transactions on Microwave Theory and Techniques, 1954 and 1955): `Z0 = (eta0 / 4) / (sqrt(er) (x + Cf))` with `Cf = (1/pi)(2 ln(1 / (1 - t/b) + 1) - (t/b) ln(1 / (1 - t/b)^2 - 1))`; for `t = 0`, `Cf = 2 ln 2 / pi`. It is the usual `94.15 / (sqrt(er) (x + Cf))`.
* Narrow strip (the round-wire equivalent, as collected in B. C. Wadell, "Transmission Line Design Handbook", Artech House, 1991): `Z0 = (eta0 / (2 pi sqrt(er))) ln(4b / (pi d))` with `d = (W/2)(1 + (t / (pi W))(1 + ln(4 pi W / t) + 0.51 pi (t / W)^2))`; for `t = 0`, `d = W/2`.
* The boundary between the two is `x = 0.35` in IPC-2141 and in Wadell. Here the narrow form is used up to `x = 0.30`, the wide form from `x = 0.40`, and the two are blended linearly between them so that the impedance has no step (the width of the band is a choice of this implementation). Both forms are within about 1 % of the exact value for a strip of zero thickness (Cohn 1954, `Z0 = (eta0 / (4 sqrt(er))) K(k) / K(k')` with `k = sech(pi W / 2b)`), which the crate also provides as a reference and checks in its tests.
* IPC-2141 gives `Z0 = (60 / sqrt(er)) ln(4b / (0.67 pi (0.8 W + t)))` for `x < 0.35` and `t/b < 0.25`; the crate checks its result against it within 3 %. The crate warns above `t/b = 0.25`.

**Offset stripline.** The track is a distance `h1` from one plane and `h2` from the other, with the dielectric of each side. It is taken as two half structures, each a centred stripline of plane spacing `2 h_i + t` with the permittivity of its side, whose capacitances add: `Z0 = 1 / sqrt(A E)` with `A = sum(1 / (2 Zair_i))` and `E = sum(er_i / (2 Zair_i))`, and `eps_eff = E / A`. For equal distances and one dielectric this is exactly the centred stripline. For unequal distances it is an approximation, exact for the parallel-plate part of the field and not for the fringing; its error has not been measured here and the crate says so with `PHOENIX-STACK-016`.

### Coplanar waveguide

R. N. Simons, "Coplanar Waveguide Circuits, Components, and Systems", Wiley, 2001 (conformal mapping). With `a = W/2`, `b = a + G`, `k = a/b` and `K'(k) = K(sqrt(1 - k^2))`, `K` the complete elliptic integral of the first kind (computed by the arithmetic-geometric mean, Abramowitz and Stegun 17.6):

* Without a ground plane under a substrate of height `h`: `k1 = sinh(pi a / 2h) / sinh(pi b / 2h)`, `eps_eff = 1 + (er - 1)/2 (K'(k)/K(k)) (K(k1)/K'(k1))`, `Z0 = (eta0 / 4) / sqrt(eps_eff) K'(k)/K(k)`.
* With a ground plane (grounded coplanar waveguide): `k3 = tanh(pi a / 2h) / tanh(pi b / 2h)`, `eps_eff = (1 + er (K'(k)/K(k)) (K(k3)/K'(k3))) / (1 + (K'(k)/K(k)) (K(k3)/K'(k3)))`, `Z0 = (eta0 / 2) / (sqrt(eps_eff) (K(k)/K'(k) + K(k3)/K'(k3)))`.
* The formulas are for a strip of zero thickness. The thickness `t` widens the strip and narrows both slots by `delta = (1.25 t / pi)(1 + ln(4 pi W / t))` (Gupta et al.). The permittivity is not corrected for the thickness. The crate warns above `t / min(W, G) = 0.1` (a limit of this implementation).

The complement of the tangent ratio is computed without the loss of precision of `1 - k3^2` when both arguments are large, so a wide strip on a thin substrate stays finite.

### Coupled pair

The differential impedance of an edge-coupled pair comes from the fits of IPC-2141, in terms of the impedance `Z0` of one track alone and the spacing `s` between the edges:

* microstrip: `Zdiff = 2 Z0 (1 - 0.48 exp(-0.96 s / h))`, with `h` the height of the substrate;
* stripline: `Zdiff = 2 Z0 (1 - 0.347 exp(-2.9 s / b))`, with `b` the spacing of the planes.

The odd-mode impedance is `Zdiff / 2`. The even mode, the common-mode impedance and the crosstalk are not computed. The effective permittivity and the losses of a pair are those of one track alone. The crate warns outside `s/h` (or `s/b`) from 0.1 to 3 (a window chosen by this implementation) and always says that the pair is an approximation (`PHOENIX-STACK-016`).

## Losses, delay and wavelength

* **Skin depth** `delta = 1 / sqrt(pi f mu0 sigma)` and surface resistance `Rs = 1 / (sigma delta)`, with `sigma` from the settings (5.8e7 S/m, 100 % IACS, by default). `mu0 = 1.25663706212e-6 H/m` and `c = 299 792 458 m/s`.
* **Conductor loss** by the incremental inductance rule of H. A. Wheeler, "Formulas for the skin effect", Proceedings of the IRE, 1942: the surfaces of the copper are moved into the metal and the rise of the inductance of the line in air gives the resistance. `alpha_c = Rs / (eta0 Z0) dZair/ds` in Np/m, where `Zair` is the impedance of the same section with air in place of every dielectric, and `dZair/ds` its rate of growth when every width and thickness of copper shrinks, and every gap between conductors grows, by `s`. The derivative is taken numerically (a step of 1e-4 of the smallest dimension) on the same model that gives `Z0`, so it is as good as that model. The rule assumes copper thick compared with the skin depth: the crate warns below three skin depths (a setting).
* **Roughness** by the Hammerstad-Jensen factor `Kr = 1 + (2/pi) atan(1.4 (D / delta)^2)`, `D` the RMS roughness; it reaches two at most. It multiplies the whole conductor loss.
* **Dielectric loss** `alpha_d = (k0 / 2) sum(q_i er_i tan_i) / sqrt(eps_eff)` in Np/m, `k0 = 2 pi f / c`, `q_i` the share of the field that is in the dielectric `i`. For a microstrip `q = (eps_eff - 1)/(er - 1)` (D. M. Pozar, "Microwave Engineering"); for a homogeneous line it is `pi f sqrt(er) tan / c`.
* **Delay** `sqrt(eps_eff) / c` in ps/mm and **guided wavelength** `c / (f sqrt(eps_eff))`.
* Conversion to dB/cm: `20 / ln(10) / 100` times Np/m.

## Dk and Df between published points

A material gives points `[frequency in GHz, value]`. The value at a frequency is interpolated linearly in the logarithm of the frequency between the two points that enclose it. It is never extrapolated: a frequency below the lowest or above the highest point is the error `PHOENIX-STACK-004`. A material with one point serves only that frequency. This is the same rule as the materials database of the suite.

## Synthesis of a width

The impedance falls as the track widens. The crate searches the width in whole nanometres between `synth_min_width_nm` and `synth_max_width_nm` (settings) by bisection on the direct calculation, until the impedance is within `synth_tolerance_ohm` of the target. A target that the two limits do not enclose is `PHOENIX-STACK-007` (the error names the impedances the limits give); a search that runs out of steps or of nanometres is `PHOENIX-STACK-008`. For a pair the gap is fixed and the width is searched for the differential impedance.

## Settings

`config/stackup.cfg`, one `key = value` per line, read with `phoenix stackup --settings FILE`:

| Key | Default | Meaning |
|---|---|---|
| `copper_conductivity_s_per_m` | 5.8e7 | conductivity of the copper |
| `thin_conductor_skin_depths` | 3.0 | warn when the copper is thinner than this many skin depths |
| `quasi_static_height_fraction` | 0.05 | warn when the substrate height exceeds this fraction of the guided wavelength |
| `max_frequency_ghz` | 1000.0 | the highest frequency accepted |
| `synth_tolerance_ohm` | 0.01 | distance to the target at which the synthesis stops |
| `synth_max_iterations` | 100 | most steps of the synthesis |
| `synth_min_width_nm` | 50000 | narrowest track tried |
| `synth_max_width_nm` | 20000000 | widest track tried |

## Errors and warnings

Every error and warning has a code `PHOENIX-STACK-<NNN>` (see `ERROR_CODES.md`) and its title in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese); the detail is in English. `phoenix stackup --lang xx` prints the titles in a language. The calculation never panics on its input: a bad number, a bad stack-up, a line that does not fit the stack-up or a formula outside its domain is a typed error.

## The command

```text
phoenix stackup <job.json> --freq-ghz <GHz> [--material ID ...] [--settings FILE] [--field2d] [--lang xx] [--json]
```

`--material ID` adds a material of the database of the suite (see below); it may be repeated and the job file may then leave out its `materials` list.

`--field2d` solves the same lines with the two-dimensional field solver (`docs/FIELD2D.md`) and adds its result after the report, with the difference in percent from the closed formulas (a key `field2d` in the JSON). It is the second opinion on the formulas of this page: the even and odd modes, the common impedance and the mutual capacitance and inductance of a pair are in that report and not here.

Exit code 0 when every line was computed, 1 when at least one line could not be (the others are still printed) and 2 when the arguments, the settings or the job file cannot be used. The report holds no date, host name or user name, and the same job gives the same bytes.

## The materials database of the suite

The type `MaterialSample` is the input of the calculation. With the feature `materials` (on in the command line, off by default in the crate) the crate depends on `eh3d-materials` of EH3D-COMMON, the database of board materials (laminates, prepregs, flex, metal core, PTFE and ceramic filled boards; every value as its datasheet publishes it). `src/materials_adapter.rs` turns a record into a `MaterialSample`:

* Dk and Df are the points of the record that have a frequency, sorted, the first listed winning for a repeated frequency. A value the sheet gives with no frequency (the design value of a band, for example) is not a point and is left out, so the Dk of `rogers/ro4350b` is 3.48 at 10 GHz, the one frequency its sheet gives, and 3.66, which the sheet gives for a band, is not used. A frequency outside the published points is `PHOENIX-STACK-004`; nothing is extrapolated.
* A record with no Dk (or no Df) point with a frequency is refused (`PHOENIX-STACK-005`), because no value could be interpolated.
* The name of the sample is the identifier of the record (`isola/370hr`), which is what the layers of the job name. The nominal thickness is set only when the record lists exactly one; `thicknesses_nm` gives all the thicknesses the record lists, in nanometres, and `copper_thicknesses` the copper foils (micrometres when the sheet gives them, else ounces).
* An identifier the database does not have is the error `UnknownMaterial` (`PHOENIX-STACK-003`).

In the command line, `phoenix stackup job.json --freq-ghz 10 --material rogers/ro4350b` adds the material to the job; a material the job file lists with the same name wins. `phoenix materials list` and `phoenix materials show ID` read the database:

```text
phoenix materials list [--manufacturer NAME] [--kind KIND] [--min-tg C] [--min-dk X] [--max-dk X]
                       [--min-df X] [--max-df X] [--at-ghz F] [--phoenix] [--json]
phoenix materials show <id> [--at-ghz F] [--json]
```

The kinds are `laminate`, `prepreg`, `bonding_film`, `flex_laminate`, `coverlay`, `metal_core`, `ptfe`, `ceramic_filled`, `copper_foil` and `other`. Dk and Df are taken at `--at-ghz` between the published points (a material with no point that reaches it does not match a bound on them), and without it the value nearest 1 GHz is shown and the first one listed is filtered. `--phoenix` keeps the materials that a document marked as serving the design tool defines or is the source of. A bad argument or an unknown identifier exits 2.

The feature is on by default in `eh3d-phoenix-cli`. `cargo build --no-default-features -p eh3d-phoenix-cli` builds without it; the command and the option then answer that the build has no materials database. In the stack-up crate alone, `cargo test -p eh3d-phoenix-stackup --features materials` runs the tests of the adapter.

## How it is checked

* Reference cases: a 50 ohm microstrip on 1.6 mm of a laminate of permittivity 4.5 comes out about 3 mm wide, and on 0.508 mm of a laminate of permittivity 3.66 about 1.1 mm wide; the closed-form values are compared with the exact zero-thickness stripline, with IPC-2141 and with known limits of the coplanar waveguide.
* Properties: the impedance falls as the track widens for every kind of line; the synthesis inverts the direct calculation; a higher permittivity lowers the impedance and raises the delay; the losses grow with the frequency.
* Edges and errors: tracks from 1 nm to 1 m, slots of 1 nm, frequencies outside the points of a material, missing materials and layers, and every typed error.
* The materials database: the adapter and the command are tested with real records (RO4350B at 10 GHz and no value at 2.5 GHz, the FR-4 of Bungard interpolated between its points and refused beyond them, a record with no Dk, an identifier that does not exist, filters by manufacturer, kind, glass transition and Dk).

What is not checked: any comparison with a field solver or with a measurement. Until there is one, the numbers are estimates.
