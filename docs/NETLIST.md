# The netlist made from a schematic

The simulator (`docs/SIM.md`) reads SPICE-style netlists. This page is about the other way in: the netlist made from a schematic, with no netlist written by hand. It is part of the crate `eh3d-phoenix-sim` (modules `schgen`, `modeltable`, `partvalue`, `sketch` and `netcodes`), of the command `phoenix netlist` and of the option `--schematic` of `phoenix sim`. It is a prototype: the result is an own numerical approximation that has been tried on small schematics written by the tests and has never been compared with SPICE or with a measurement.

## What goes in and what comes out

* A schematic: an EAGLE `.sch` file, a KiCad `.kicad_sch` file or the JSON of the design model (`design/schematic.json`), read into the model by the readers that already exist.
* A table of models (`config/sim_models.cfg`, which is also the built-in table, or another file given with `--models`).
* The settings of the simulator (`config/sim.cfg`), for the names of the ground nets and the generic models.

Out come the SPICE text of the netlist, a report of what was translated and of what could not be, and, for `phoenix sim --schematic`, the analysis asked for, with no file in between.

## The table of models

Each line is a rule, an attribute, a model or a setting; blank lines and what follows a `#` are ignored.

```
upper_m_is_mega = true
refdes R = resistor
value  2N3904 = npn model=q2n3904
symbol LED* = led
attr   R7 SIM.VALUE = 4k7
.model q2n3904 NPN(IS=6.7e-15 BF=416)
```

* `refdes PREFIX = DEVICE` gives a device to the parts whose reference designator starts with the prefix and then has a digit or a sign, not a letter: `R` matches `R1` and `R_load` but not `RV1`. The longest prefix wins.
* `value TEXT = DEVICE` and `symbol NAME = DEVICE` match the value of the part and the name of its symbol, in any case; a `*` at the end stands for any ending. A rule by value beats a rule by symbol, which beats a rule by prefix.
* `attr REFDES KEY = TEXT` is an attribute of one part. The keys are `SIM.DEVICE` (the device), `SIM.VALUE` (the value the simulator uses, instead of the text of the part), `SPICE_MODEL` (the name of the model, or of the subcircuit), `SIM.FILE` and `SIM.PINS`. An attribute beats every rule. The same attributes can be passed by a program in the argument `props` of `generate_netlist`.
* `.model NAME TYPE(PARAMS)` is a model that is written to the netlist when a part uses it.

The devices are `resistor`, `capacitor`, `inductor`, `vsource`, `isource`, `diode`, `led`, `zener`, `npn`, `pnp`, `nmos`, `pmos`, `switch`, `opamp`, `comparator`, `subckt`, `ammeter` (a source of zero volts that lets the current of a branch be read), `voltmeter` (reads the voltage between two nets and adds nothing to the circuit) and `skip` (a part that is not part of the circuit: a test point, a mounting hole). After the device come options: `model=NAME`, `file=NAME.lib`, `subckt=NAME`, `pins=A,K` and, for a MOSFET, `w=` and `l=`.

The readers of EAGLE and KiCad files do not yet carry the properties of a symbol into the model of the schematic, so the attributes of a part come from `attr` lines or from the program that calls the generator (`docs/NOT_YET.md`).

### The pins

The pins of the symbol go to the roles of the device by their names first (`A` and `K` of a diode, `C B E` of a bipolar transistor, `D G S` of a MOSFET, `IN+ IN- OUT` or `+ - OUT` of an op-amp) and by their order otherwise, with the pins sorted by number. A MOSFET with three pins has its bulk joined to its source. `pins=` gives the pins in the order the device takes them and beats both, which is the way to correct a symbol whose pin 1 is the cathode (`pins=2,1`). A symbol whose pins fit neither way is reported (`PHOENIX-NETLIST-007`) and the part is left out.

### The values

A value is read as a number with the suffixes of SPICE (`4.7k`, `100n`, `2.2meg`) and with the letter of the marking of parts in the place of the decimal point (IEC 60062): `4k7` is 4.7 kilo, `1u5` is 1.5 micro, `4R7` is 4.7 and `R47` is 0.47. A capital `M` is mega, as on the marking of a resistor (`2M2` is 2.2 mega, `10M` is 10 mega) and a small `m` is milli; the setting `upper_m_is_mega = false` of the table gives the reading of SPICE, where both are milli. Units after the number (`F`, `H`, `V`, `Ohm`) are ignored, as a bare `F` is femto in SPICE. The value of a source is a number or the words of a netlist that follow the nodes: `DC 5`, `AC 1`, `SIN(0 1 1k)`, `PULSE(0 5 0 1n 1n 1m 2m)`, `PWL(0 0 1m 1)`.

### Models and subcircuits

A diode, LED, transistor, switch, op-amp or comparator uses the model named by the rule (`model=`) or by the attribute `SPICE_MODEL`; without either, the name of the model is the value of the part, in lower case. When the table has a `.model` line of that name it is written to the netlist. When it has not and the name came from the value, the generic model of the type is used, with the parameters of `config/sim.cfg` (`diode_is`, `led_is`, `bjt_bf`, `mos_vto` and the others), and the report says so as information, because the part is not the part its value names. A model that a rule or an attribute names and that nothing defines is an error (`PHOENIX-NETLIST-011`), and so is a Zener diode with no model, because it needs the breakdown voltage.

A `subckt` rule needs `file=NAME.lib`: the netlist gets an `.include` of that file and an `X` line whose nodes follow the pins in the order of the symbol, or of `pins=`. The file must be in the folder of the netlist; the rule is the one of the reader of netlists (`docs/SIM.md`): a plain file name, no folder, no `..`, no drive, nesting and the number of files bounded by the settings. `phoenix netlist` takes the folder of the table (or of the schematic) and `phoenix sim --schematic` reads the files from it, with no netlist file in between. The number of ports of the subcircuit is checked when the circuit is compiled, not when the netlist is made.

## The names of the nets

* A net named `0`, `GND`, `AGND`, `DGND` or `PGND` (the setting `schematic_ground`) is node 0.
* A net with a name is that name in lower case, with anything but letters, digits and underscores written as an underscore (`+5 V` is `_5_v`). Nets with the same cleaned name are one node.
* A net with no name is `n_<designator>_<pin>` of its first pin in the order of the designators and the pin numbers (`n_r1_2`). The name does not depend on the identities of the objects or on the order they were drawn, so it does not change when something else is edited.
* An element is named by the letter of its kind and the designator: `R1` stays `R1`, and `LED1`, which is a `D` element, is `D_LED1`.

## The report

Everything that cannot be simulated is in the report with a code `PHOENIX-NETLIST-<NNN>` (`docs/ERROR_CODES.md`, with titles in the eight languages), a severity and the part, pin or net it is about. An error stops a simulation; a warning and information do not.

| Code | What it says | Severity |
|---|---|---|
| `001` | the table of models cannot be read (the message names the line) | error |
| `002` | a part no rule matches (left out), or a value that names no model (generic model used) | warning, information |
| `003` | a pin that the device uses and no net lists | warning |
| `004` | a net, other than ground, with a single connection | warning |
| `005` | a source with both pins on one net, or two voltage sources in parallel | error |
| `006` | a value or an attribute that cannot be read | error |
| `007` | the pins of the symbol do not fit the device | error |
| `008` | two parts that give one element name | error |
| `009` | no ground net | error |
| `010` | the electrical rule check found errors | error |
| `011` | a model that is named and not defined | error |
| `012` | an output file that is not allowed | error |

## The commands

```
phoenix netlist [--models FILE] [--out NEW.cir] [--json] [--lang xx] [--config FILE]
                [--erc-config FILE] [--no-erc] <schematic>
phoenix sim --schematic <schematic> --op | --dc SRC START STOP STEP | --ac KIND N F1 F2 | --tran STEP STOP
            [--models FILE] [--erc-config FILE] [--no-erc] [options]
```

* The electrical rule check (`docs/ERC.md`) runs first; its errors are listed as `PHOENIX-NETLIST-010` and `--no-erc` skips it.
* `phoenix netlist` is a dry run: the netlist is printed, the report goes to the error output, and a file is written only when `--out` names a new one. It never writes over the input, the table or a file that exists, and not to a file of a design format. With `--json` the netlist and the report are one JSON object (`phoenix-netlist/1`).
* `phoenix sim --schematic` makes the netlist in memory and runs the analysis on it. A report with an error stops the run before the solver starts and the exit code is 2. The options of the analysis are the ones of `docs/SIM.md`.
* Exit codes of `phoenix netlist`: 0 nothing stops a simulation, 1 the report has an error, 2 the arguments, the input or the output are unusable.

## In the schematic editor

The editor without a screen (`eh3d-phoenix-schedit`, `docs/SCHEMATIC_EDIT.md`) exports the netlist of the schematic it holds through this generator (`export_netlist`, with the built-in table), and gives the operating point net by net as data (`SchEdit::results`, `netlist_results`): the voltage of every net, the current of every voltage source and ammeter and the reading of every voltmeter, with the iterations of the solver. A window can show that table; none has been made.

## What was tried

The tests build small schematics and check the netlist and the solution: a divider with values in the marking of parts, nets with no name, a LED and a diode with a resistor, an NMOS inverter, an inverting op-amp amplifier, a subcircuit from a file, an ammeter and a voltmeter, and every kind of report. The reference cases of `docs/ORACLE.md` (`tests/oracle/netlist.json`) write small schematics as a few lines and compare the solution with closed forms of the textbooks. Nothing here has been compared with SPICE.

Not done: the properties of the symbols of EAGLE and KiCad are not read into the model, so a model chosen in those programs is not seen; the table can name a footprint only through the name of the symbol; the models of a manufacturer are not shipped (the table holds generic models and examples, not data of any part); a pin has one text in the model (the name EAGLE gives it or the number KiCad gives it), so a KiCad diode whose pin `1` is the cathode needs `pins=2,1`; hierarchical sheets and buses are not expanded; a part with a model of its own that needs more than the supported devices is left out and reported.
