# EH3D-PHOENIX-PCB: extended RF tools

The extended RF tools sit on top of the first level described in `RF.md` (`eh3d-phoenix-rf`, command `phoenix rf`). They add pi and T matching networks with a chosen loaded Q, the bandwidth of a matching network, the double stub and the equivalents of a match with transmission lines (with lengths in metres on a line of the stack-up), the path of a match on the Smith chart as data and as a small picture, filters with components of the standard series E12, E24 and E96, the S parameters of a network of lumped elements and lines written as text, the check of a response against a mask, and the stability circles and the simultaneous conjugate match of an amplifier.

**State: prototype.** Everything here is an **estimate from ideal lumped elements and lossless quasi-static lines**: there is no loss, no parasitic, no tolerance of the components, no dispersion and no coupling. Nothing has been compared with a network analyser, with a field solver or with another RF program. The tests use values of the textbooks, results worked by hand, properties (a network built for a match gives the conjugate of the source at its input, a lossless network conserves power) and hostile input. Requirement PPCB-024 stays `prototype`.

## What it does

* **Matching networks** (`matchnet`): the L network of `RF.md` now reports its Q, `sqrt(R_high / R_low - 1)`, and its bandwidth. The **pi** network (shunt, series, shunt) and the **T** network (series, shunt, series) are designed for a complex source and a complex load at one frequency, low-pass or high-pass, with the **loaded Q chosen** (`--q`, 5 by default, never below the Q of the L network and never above `max_loaded_q`). The reactance of the source and of the load is taken into the element next to it. The three elements, the Q of each of the two L sections that make the network and the virtual resistance they meet at are reported. The check of every network rebuilds it from its elements and gives the input impedance, which has to be the conjugate of the source.
* **Bandwidth**: the rule of thumb `f0 / Q` (an order of magnitude only) and, for a real source and a real load, the two frequencies where the return loss falls to `return_loss_bandwidth_db` (10 dB by default), found by a scan of the ABCD matrices from a quarter to four times the design frequency with the values of the elements kept fixed, and then by bisection. A complex termination has only the rule of thumb. An edge outside the scan range is reported as not found.
* **Equivalents with lines** (`match lines`): for the same load, the single shunt stub (short-circuited and open, both solutions), the **double shunt stub** (two stubs a chosen distance apart, the first at the load, short-circuited and open, both solutions) and the quarter-wave transformer, each with its lengths in degrees and in metres. The line is given by its impedance and effective permittivity (`--z0`, `--eps-eff`) or taken from a line of a stack-up file (`--stackup FILE --line NAME`, the same file `phoenix stackup` reads), so the lengths are those of the real stack-up at the frequency. A load in the forbidden region of the double stub (a normalised conductance above `1 / sin^2` of the spacing) has no solution with that spacing: the report says so and the other kinds go on.
* **The Smith chart as data and as a picture** (`trace`): the path of a match is a list of steps from the load to the source side (a series reactance moves along a circle of constant resistance, a shunt susceptance along a circle of constant conductance, a line turns around the centre). The report gives, after every step, the impedance, the admittance and the reflection coefficient, and the reflection coefficients along every step (`smith_arc_points` per step) so that any program can draw them. `--svg FILE` writes a small SVG of the path of one network (`--pick N`): the chart with circles of constant resistance and reactance, the path, a marker at every point and a legend. The picture is written by this crate (the printing crate draws boards, not charts) and is the same text for the same input.
* **Filters with standard values** (`filterx`, `eseries`): the prototypes of `RF.md` (Butterworth, Chebyshev and Bessel) with the components taken to the nearest value (nearest in ratio) of **E12, E24 or E96** (`--snap E24`; `--snap default` takes the series of `default_series`). The report gives the value of the design, the value of the series and the error of each component, how far each **band edge moves** (the edge is where the insertion loss crosses the loss that the ideal filter has at its design edge, found by a scan and a bisection) and the largest change of the insertion loss in the pass band. The Legendre and the elliptic approximations are not implemented (see "What it does not do").
* **Response from the simulator** (`--simulate`, needs `--sweep-ghz`): the netlist of the filter is made, simulated in AC by the simulator of the suite and the output is scaled to the `S21` of the filter, with its magnitude, phase and group delay beside the ones computed from the ABCD matrices. `--netlist` adds the SPICE text of the filter (the one with standard values when `--snap` is used). The two methods are unrelated, so a difference would mean a bug in one of them; neither is a comparison with another program.
* **S parameters of a network** (`netdesc`, `phoenix rf sparams`): a network of series and shunt resistors, inductors and capacitors and of lossless lines, written as text (below), is turned into ABCD matrices in cascade and then into S parameters against a real reference impedance, over a linear or logarithmic sweep. The table gives `|S11|` and `|S21|` in dB with their phase, the return loss, the VSWR and the insertion loss. `--s2p-out FILE` writes the response as a Touchstone two-port file (`--format ri|ma|db`, frequencies in GHz); a file of that kind is read back by `phoenix rf touchstone`. The command also reads a Touchstone `.s2p` file (`--s2p FILE`) instead of a description.
* **Masks** (`mask`, `--mask FILE`): the response (of the network or of the file) is checked against a template of segments, each limiting the magnitude of one S parameter in dB over a range of frequencies with a floor (`min`) or a ceiling (`max`), constant or in a straight line from one limit to another. The report gives for each segment the number of points, the worst margin, the frequency where it happens and pass or fail; a segment with no point inside it fails. The exit code is 1 when the mask is not met.
* **Stability** (`circles`, `phoenix rf stability`): from the S parameters of an amplifier (a Touchstone file at a frequency or at all its points, or the four values on the command line), K, the determinant, mu and mu' (as in `RF.md`), and at one frequency the **source and load stability circles as data** (centre, radius, which side is stable, whether the circle cuts the Smith chart), the maximum stable gain, the maximum available gain and, when the amplifier is unconditionally stable, the **simultaneous conjugate match** (the two reflection coefficients and the gain, which is the maximum available gain).

## What it does not do

* **The Legendre and the elliptic (Cauer) approximations are pending**: they have no closed formula for the elements, only tables, and the tables are not at hand to copy values from (the oracle lists them as pending). The command refuses them by name. Bessel keeps the table of `RF.md`.
* No loss, parasitics, tolerance or temperature in the elements; no dispersion or discontinuities in the lines (no steps, bends or T junctions); no coupling between elements. The standard values change the response only through the rounding of the values.
* The pi and T networks have one loaded Q chosen for the whole network; they are not optimised for a bandwidth, a harmonic rejection or a component range.
* The bandwidth by scan keeps the values of the elements fixed (they do not follow the frequency) and needs a real source and a real load.
* The double stub is made of two shunt stubs of the same kind, the first at the load; series stubs, stubs of other impedance than the line and three stubs are not done. The equivalents with lines match the load to the line (a real source of the impedance of the line), not to a complex source.
* The Smith picture is one chart for impedances (no admittance chart on top) and one path; it is not a plotting tool and does not draw frequency sweeps.
* The description of a network has no coupled elements, no transformers, no mixed-mode ports, no loss in the lines and no frequency-dependent models.
* A Touchstone file is read as `RF.md` says (version 1.0 and 1.1, one reference resistance); the stability of a file that is not a two-port is refused.
* Noise circles and gain circles are not done.

## The description of a network

One element per line, from port 1 to port 2; a `#` or a `!` starts a comment.

```text
reference 50          # once, before the elements; 50 ohm when missing (rf.cfg: default_z0_ohm)
series L 10n          # series or shunt, then R, L or C, then the value
shunt C 2.2p
line 50 90deg@1G      # impedance, then an electrical length at a frequency ...
line 50 12mm 3.2      # ... or a physical length (mm, cm, um or m) and the effective permittivity
```

Values take the prefixes `f p n u m k meg g t` (a lower-case `m` is milli, an upper-case `M` or `meg` is mega) and may carry a unit after the prefix (`10nH`, `2.2pF`, `4.7kohm`, `2.4GHz`). A resistor, inductor or capacitor must be above zero. Every error names the line (`PHOENIX-RF-011`).

## The mask

One segment per line: `parameter min|max start stop limit [limit_at_stop]`.

```text
s21 min 0 100M -3         # at least -3 dB up to 100 MHz
s21 max 300M 1G -40 -60   # below a line that goes from -40 dB at 300 MHz to -60 dB at 1 GHz
s11 max 0 100M -10
```

The parameter is `s11`, `s12`, `s21` or `s22`; frequencies take the prefixes `k`, `M` and `G`. Every error names the line (`PHOENIX-RF-012`).

## Formulas and sources

* L network Q: `Q = sqrt(R_high / R_low - 1)` (C. Bowick, *RF Circuit Design*, matching networks).
* Pi network: two L sections that meet at a virtual resistance `R_v = R_high / (1 + Q^2)` below both parallel resistances, with `Q_s = sqrt(R_s / R_v - 1)` and `Q_l = sqrt(R_l / R_v - 1)`; the shunt susceptances are `Q_s / R_s` and `Q_l / R_l` and the series reactance `R_v (Q_s + Q_l)`, capacitive and inductive for the low-pass form and the other way round for the high-pass one (C. Bowick, *RF Circuit Design*). The reactance of a complex source or load is subtracted from the element next to it so that the input impedance is the conjugate of the source.
* T network: the dual, with `R_v = R_low (1 + Q^2)` above both series resistances, `Q_s = sqrt(R_v / R_s - 1)`, `Q_l = sqrt(R_v / R_l - 1)`, series reactances `Q_s R_s` and `Q_l R_l` and a shunt susceptance `(Q_s + Q_l) / R_v`.
* Double stub (D. M. Pozar, *Microwave Engineering*, section 5.3): with `g + jb` the normalised admittance of the load, `t = tan(spacing)` and `B` the susceptance after the first stub, `g_d = g (1 + t^2) / ((1 - t B)^2 + t^2 g^2)` has to be 1, which gives `B = (1 -+ sqrt(g (1 + t^2) - g^2 t^2)) / t`; the second stub cancels the susceptance left at the second position. A real root exists only if `g <= 1 / sin^2(spacing)`.
* Stub lengths: open `tan(theta) = b`, short-circuited `cot(theta) = -b` for a normalised susceptance `b` to produce (as in `RF.md`).
* Butterworth `g_k = 2 sin((2k - 1) pi / 2n)`; Chebyshev with `beta = ln coth(L / 17.37)` and `gamma = sinh(beta / 2n)` (D. M. Pozar, *Microwave Engineering*, section 8.3; as in `RF.md`).
* Standard series: IEC 60063, E12 and E24 as published; E96 is `10^(k/96)` rounded to three digits (a test checks every value).
* S parameters from ABCD against a real reference impedance, `S11 = (A + B/Z0 - C Z0 - D) / (A + B/Z0 + C Z0 + D)`, `S21 = 2 / (A + B/Z0 + C Z0 + D)` (D. M. Pozar, *Microwave Engineering*, table 4.2).
* Stability circles (D. M. Pozar, *Microwave Engineering*, section 11.3): load plane `C_L = (S22 - D S11*)* / (|S22|^2 - |D|^2)`, `r_L = |S12 S21| / ||S22|^2 - |D|^2|`; source plane `C_S = (S11 - D S22*)* / (|S11|^2 - |D|^2)`, `r_S = |S12 S21| / ||S11|^2 - |D|^2|`, with `D = S11 S22 - S12 S21`. The stable side is the one that holds the centre of the chart when `|S11| < 1` (load plane) or `|S22| < 1` (source plane), and the other side otherwise. K is Rollett's factor (J. M. Rollett, 1962); mu and mu' are the geometric factors of M. L. Edwards and J. H. Sinsky (1992).
* Simultaneous conjugate match (D. M. Pozar, section 11.4): `G_s = (B1 -+ sqrt(B1^2 - 4 |C1|^2)) / (2 C1)` with `B1 = 1 + |S11|^2 - |S22|^2 - |D|^2` and `C1 = S11 - D S22*` (the sign is minus when `B1` is positive), and the same for the load with `B2` and `C2`; it exists when `K > 1` and `|D| < 1`.

## The commands

```text
phoenix rf match l|pi|t --load Z [--source Z] --freq-ghz F [--q Q] [--pass low|high]
                  [--svg FILE] [--pick N] [--json] [--out FILE] [--settings FILE] [--lang xx]
phoenix rf match lines --load Z --freq-ghz F [--z0 R --eps-eff E | --stackup FILE --line NAME]
                  [--spacing-deg D] [--json] [--out FILE] [--settings FILE] [--lang xx]
phoenix rf filter <butterworth|chebyshev|bessel> <order> <lowpass|highpass|bandpass|bandstop>
                  (--fc-ghz F | --f1-ghz F --f2-ghz F) [--ripple-db R] [--sweep-ghz START,STOP,POINTS]
                  [--snap E12|E24|E96|default] [--simulate] [--netlist] [--json] [--out FILE]
phoenix rf sparams (--net FILE --sweep-ghz START,STOP,POINTS [--log] | --s2p FILE)
                  [--mask FILE] [--s2p-out FILE] [--format ri|ma|db] [--json] [--out FILE]
phoenix rf stability (--s2p FILE [--freq-ghz F] | --s11 X --s12 X --s21 X --s22 X)
                  [--json] [--out FILE]
```

`--load` and `--source` take an impedance as in `RF.md` (`50`, `50+j25`, `j25`). A scattering value for `--s11` and the others is `0.6@-95` (magnitude and angle in degrees) or `0.3+j0.4`. Without `--pass`, `pi` and `t` give both the low-pass and the high-pass network. `match l` without the new options prints the report of `RF.md`; with `--json`, `--svg`, `--out`, `--pass` or `--pick` it prints the extended one.

The commands only read files. `--out`, `--svg` and `--s2p-out` write to a file that **does not exist yet** and never overwrite one. The text tables are in the eight languages of the suite (`--lang en|es|de|fr|it|ja|zh|pt`, European Portuguese for `pt`); the JSON has fixed English keys, is the same text every time for the same input and says `"estimate": true`. A number that is not finite is written as `null`. Exit codes: 0 done, 1 there is no result (no matching network, a mask that is not met, a limit passed), 2 the arguments or the input are unusable.

```text
phoenix rf match pi --source 50 --load 100 --freq-ghz 0.1 --q 5 --pass low --svg path.svg
phoenix rf filter butterworth 3 lowpass --fc-ghz 0.1 --sweep-ghz 0.01,0.3,30 --snap E24 --simulate
phoenix rf sparams --net lowpass.txt --sweep-ghz 0.01,1,50 --mask mask.txt --s2p-out lowpass.s2p
phoenix rf stability --s11 0.65@-95 --s12 0.035@40 --s21 5@115 --s22 0.8@-35
```

The first designs the low-pass pi network from 50 to 100 ohm with a loaded Q of 5 at 100 MHz: a shunt capacitor of about 110 pF, a series inductor of about 51.8 nH and a shunt capacitor of about 79.6 pF, with a virtual resistance of 3.846 ohm, and draws its path. The second takes the third-order Butterworth low-pass of `RF.md` to E24 (33 pF, 160 nH, 33 pF), says that the cutoff moves by about -1.55 percent and compares the simulated response with the one from the ABCD matrices.

## Settings

The new limits are in `config/rf.cfg`, with the ones of `RF.md`: `max_net_elements`, `max_net_bytes`, `max_mask_segments`, `smith_arc_points`, `default_loaded_q`, `max_loaded_q`, `scan_points`, `return_loss_bandwidth_db`, `default_series`, `default_stub_spacing_deg` and `mask_margin_tolerance_db`. A key that does not exist is refused with its line (`PHOENIX-RF-008`), a value beyond a limit with `PHOENIX-RF-009`.

## Errors

`PHOENIX-RF-011` (the network description has a syntax error), `PHOENIX-RF-012` (the mask has a syntax error) and `PHOENIX-RF-013` (the series of standard values is not known) are new; the older codes are in `RF.md`. All are in `ERROR_CODES.md` and their titles exist in the eight languages. A file never makes the program panic.

## How it is tested

Results worked by hand (the Q of an L network, the virtual resistance of a pi network, the g values of Butterworth and Chebyshev filters against the table, the loss of a third-order Butterworth filter at its cutoff and at twice its cutoff, the VSWR of a series resistor, K and mu of a two-port with real parameters, the radius of a stability circle, the nearest value of E24), properties on generated networks (every pi and T network, in both passes and with complex terminations, gives the conjugate of the source; every double stub gives the characteristic impedance; the points of a stability circle give an input reflection of magnitude 1; the conjugate match gives the maximum available gain; a lossless network conserves power), the picture and the JSON against themselves (the same text every time), the simulated response against the ABCD one, the eight languages of the tables, the exit codes and the files that are never overwritten, and hostile input. The reference cases are in `tests/oracle/rf.json` (`docs/ORACLE.md`); the comparison with a network analyser or with another RF program is pending. A high number of tests is not proof of correctness.
