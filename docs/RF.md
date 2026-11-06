# EH3D-PHOENIX-PCB: RF tools

The crate `eh3d-phoenix-rf` holds the RF calculations of everyday use: Touchstone files, the parameters of two-ports and the conversions between them, impedance matching, the coordinates of the Smith chart and ladder filters. The command `phoenix rf` runs them. It sits next to `eh3d-phoenix-stackup`, which gives the impedance and the effective permittivity of a track: a line from the stack-up calculator can be turned into a line of the RF tools (`line::Line::from_stackup`) and lengths in degrees can be turned into metres on it.

The extension of these tools (pi and T networks, the bandwidth of a match, the double stub, the equivalents with lines, the Smith path, filters with standard values, S parameters of a network, masks and stability circles) is described in `RF_ADVANCED.md`.

**State: prototype.** The formulas are published ones and the tests use values of the textbooks (an L network from 50 to 100 ohm, a third-order Butterworth filter at 100 MHz, a quarter-wave line with `z = sqrt(z1 z2)`), properties (round trips between S, Z, Y and ABCD, reciprocity, passivity, conservation of power in a lossless filter) and hostile input. Nothing here has been compared with a network analyser, with a field solver or with another RF program, and the filters are ideal. Requirement PPCB-024 (RF tools by levels, analytic calculators first) is `prototype` because of this crate and of the stack-up one, and no further.

## What it does

* **Touchstone files** (`touchstone`): `.s1p`, `.s2p` and `.sNp`, version 1.0 and 1.1. The option line `# <unit> <parameter> <format> R <ohm>` is read with the units Hz, kHz, MHz and GHz, the kinds S, Y, Z, H and G, the formats RI, MA and DB, and the reference resistance (the defaults are `GHz S MA R 50`). Comments with `!` anywhere in a line, records that run over several lines (three ports and more), the order 11, 21, 12, 22 of a two-port, and the noise data of a two-port are read. The writer gives a file that reads back to the same values (exactly for RI). The number of ports comes from the extension of the name. Every error names the line.
* **Two-port parameters** (`network`): the 2 x 2 matrix, the conversions S to Z, Y and ABCD and back (with one real reference impedance for both ports), the cascade of ABCD and of S matrices, and the ABCD matrices of a series impedance, a shunt admittance and a line (lossy or lossless). Figures: reflection coefficient and impedance, VSWR, return loss, mismatch loss, insertion loss, group delay, transducer gain, maximum stable gain, maximum available gain, the Rollett factor K, the determinant, mu and mu', passivity (largest singular value) and reciprocity.
* **Matching** (`matching`): the L network (both topologies, each with its two solutions: low-pass, high-pass or mixed) for a complex source and a complex load, with the value of the inductor or capacitor of each element; the single stub (shunt or series, open or short-circuited) with the distance from the load and the length of the stub in degrees and in wavelengths; the quarter-wave transformer, with `z = sqrt(z1 z2)` for real impedances and, for a complex load, the line that brings it to a real value first. Every result carries a check function that rebuilds the network and gives the input impedance.
* **Smith chart** (`smith`): for a point or a sweep, the reflection coefficient (the position), the normalised impedance and admittance, and the circles of constant resistance, reactance, conductance and susceptance (centre and radius). Nothing is drawn: another module draws.
* **Filters** (`filter`): Butterworth, Chebyshev (ripple in dB) and Bessel prototypes by order, transformed to low-pass, high-pass, band-pass and band-stop for any source impedance, with the first element in shunt or in series; the response (insertion loss, return loss, group delay) from the ABCD matrices of the sections; the least order for a specification (Butterworth and Chebyshev).
* **Text** (`report`) and **command** (`phoenix rf`): see below.

## What it does not do

* No version 2 keyword syntax (a line that starts with `[` is refused), no reference resistance per port, no mixed-mode parameters. Z and Y files are taken as normalised to the reference resistance; H and G files are read and refused when they have to be converted.
* No loss, no parasitics, no tolerance of the components in the filters; no de-embedding; no noise circles and no gain circles; a stub is a lossless line of the characteristic impedance of the main line; no tapered or multi-section broadband match (the double stub is in `RF_ADVANCED.md`).
* No frequency-dependent models of the elements: a design is exact at the frequency asked (matching) or for ideal elements (filters).
* An L network needs the right relation between the resistances (the series element can only transform in one direction for each topology); when none exists the tool says so and does not invent a network.
* A group delay from a file is the phase step between the two points around the frequency: the points must be close enough for the phase to move by less than half a turn between them.

## Formulas and sources

* Conversions between S, Z, Y and ABCD: Pozar, *Microwave Engineering*, chapter 4 (table 4.2); the matrix forms `Z = z0 (I + S)(I - S)^-1` and `S = (Z - z0 I)(Z + z0 I)^-1` for the others.
* Stability: Rollett's `K` and the geometric factors `mu` and `mu'` of Edwards and Sinsky (`mu > 1` if and only if `K > 1` and `|delta| < 1`).
* L network: the series element and the shunt element are solved for the conjugate match of the complex source, `Zin = Zs*`, in the two topologies. With the load admittance `GL + jBL`, the source resistance `Rs` and the source reactance `Xs`, the shunt susceptance is `B = -BL +- sqrt(GL / Rs - GL^2)` and the series reactance `X = -Xs + (BL + B) Rs / GL`; the other topology is the dual. A positive reactance is an inductor `L = X / w`, a negative one a capacitor `C = -1 / (w X)`, a positive susceptance a capacitor `C = B / w`, a negative one an inductor `L = -1 / (w B)`.
* Single stub: Pozar, section 5.2. With `t = tan(theta)` and the normalised admittance (shunt) or impedance (series) `g + jb`, the distance solves `(g - g^2 - b^2) t^2 + 2 b t + (g - 1) = 0`; the stub cancels the remaining susceptance or reactance: `tan(theta_s) = -b` for an open shunt or a short series stub, `cot(theta_s) = b` for the other two.
* Quarter-wave transformer: `z = sqrt(z1 z2)`; for a complex load `x t^2 + (|z|^2 - 1) t - x = 0` gives the two lengths of line (a quarter wave apart) at which the impedance is real.
* Filters: Butterworth `g_k = 2 sin((2k - 1) pi / 2n)`; Chebyshev `beta = ln coth(Lr / 17.37)`, `gamma = sinh(beta / 2n)`, `g_1 = 2 a_1 / gamma`, `g_k = 4 a_(k-1) a_k / (b_(k-1) g_(k-1))`, with `a_k = sin((2k - 1) pi / 2n)` and `b_k = gamma^2 + sin^2(k pi / n)`, and a load of `coth^2(beta / 4)` for an even order (Pozar, section 8.3; Matthaei, Young and Jones). Bessel: the published table of maximally flat group delay (orders 1 to 8), normalised to a delay of 1 s at zero frequency, so the cutoff given for a Bessel filter is the inverse of the delay wanted and not the 3 dB point. Transformations to high-pass, band-pass and band-stop: Pozar, table 8.7, with the geometric centre `sqrt(f1 f2)` and the fractional width `(f2 - f1) / f0`.

## The command

```text
phoenix rf touchstone <file.sNp> [--freq-ghz X] [--settings FILE] [--lang xx]
phoenix rf match l    --load Z [--source Z] --freq-ghz F
phoenix rf match stub --load Z [--z0 R] --freq-ghz F [--stub shunt|series] [--end open|short] [--eps-eff E]
phoenix rf match qw   --load Z [--z0 R] --freq-ghz F [--eps-eff E]
phoenix rf filter <butterworth|chebyshev|bessel> <order> <lowpass|highpass|bandpass|bandstop>
                  (--fc-ghz F | --f1-ghz F --f2-ghz F) [--ripple-db R] [--z0 R]
                  [--first shunt|series] [--sweep-ghz START,STOP,POINTS] [--log]
```

An impedance `Z` is written `50`, `50+j25`, `50-j25` or `j25`, in ohm. `--lang` is one of en, es, de, fr, it, ja, zh, pt and selects the language of the title of an error; the reports are in English. `--eps-eff` gives the physical lengths of stubs and sections on a line of that effective permittivity (take it from `phoenix stackup`).

Exit codes: 0 done, 1 the calculation asked for has no result (no matching network, a frequency outside the file, a limit passed), 2 the arguments or the input are unusable (a file that cannot be read, a number that cannot be read, a settings file that is wrong).

Examples:

```text
phoenix rf match l --source 50 --load 100 --freq-ghz 0.1
phoenix rf filter butterworth 3 lowpass --fc-ghz 0.1 --sweep-ghz 0.01,0.3,30
phoenix rf touchstone amplifier.s2p --freq-ghz 2.4
```

The first prints the two L networks of the book case: a series inductor of 79.6 nH with a shunt capacitor of 15.9 pF (low-pass), or a series capacitor of 31.8 pF with a shunt inductor of 159 nH (high-pass). The second prints the three components of the third-order Butterworth low-pass at 100 MHz (a shunt capacitor of 31.8 pF, a series inductor of 159 nH and a shunt capacitor of 31.8 pF) and the response over the sweep.

## Settings

The limits and tolerances are in `config/rf.cfg` and are never fixed in the code: the most ports, points and bytes of a Touchstone file, the highest frequency, the highest filter order, the range of the ripple, the range of the impedances, the most points of a sweep, the step of the numerical group delay, the tolerance below which a reactance counts as zero, the tolerance of the passivity check and the reference impedance used when none is given. A line that is wrong is refused with its number (`PHOENIX-RF-008`); a value beyond a limit is refused with `PHOENIX-RF-009`.

## Errors

The codes `PHOENIX-RF-001` to `PHOENIX-RF-013` are in `ERROR_CODES.md`. Their titles exist in the eight languages of the suite (English, Spanish, German, French, Italian, Japanese, Chinese and European Portuguese); the detail is in English. A file never makes the program panic: what cannot be read becomes an error with the line.

## How it is tested

Textbook values (the L network from 50 to 100 ohm, the g values of Butterworth and Chebyshev, the response of a third-order Butterworth filter, the quarter-wave section), properties on generated networks (round trips between the four kinds of parameters, associativity of the cascade, a determinant of one for a reciprocal network, passive networks stay passive when cascaded, a lossless filter conserves power, every matching network gives the conjugate of the source), the writer against the reader for every format, unit and size, a filter written as a Touchstone file and read back, the numbers of the command and its exit codes, and hostile input. A high number of tests is not proof of correctness: the next step is a comparison with measured data and with another program.
