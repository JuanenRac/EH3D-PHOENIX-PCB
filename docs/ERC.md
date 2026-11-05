# EH3D-PHOENIX-PCB: electrical rule check and synchronisation

Phase F7 of the roadmap asks for a schematic editor, electrical rule checks, transfer to the board, forward and backward annotation by stable identity and net classes. This page describes the parts that need no window: the electrical rule check of a schematic (`eh3d-phoenix-erc`, command `phoenix erc`), the synchronisation between a schematic and a board (`eh3d-phoenix-sync`, command `phoenix sync`) and the net classes of the rule file (`eh3d-phoenix-rules`). The schematic editor (`docs/SCHEMATIC_EDIT.md`) runs the check after every change. All of it is a prototype: it has been tried on schematics and boards made by hand and by the tests and it has never been compared with the reference tool, so requirement PPCB-017 stays at `prototype`.

## Pin types

The electrical rule check judges the connections by the type of the pins. The schematic holds them in `pin_kinds` (a map from the id of a pin of a symbol to its type; nothing is written when the map is empty, so a schematic made before it existed reads and writes as before). The types are `input`, `output`, `bidirectional`, `tri_state`, `passive`, `power_in`, `power_out`, `open_collector` and `not_connected`.

The EAGLE reader fills the types from the `direction` of the pins of the library in the file (`in`, `out`, `io`, `hiz`, `pas`, `pwr`, `sup`, `oc`, `nc`) and the EAGLE writer writes them back; a pin whose type is not known is written without a direction, so reading the file again never invents one. The KiCad reader does not give types yet. A pin without a type is left out of every check that needs one and the report says so: it is never taken for a passive pin.

## The electrical rule check

`phoenix erc [--config FILE] [--json] [--lang CODE] <schematic> [<board>]` reads a schematic (an EAGLE `.sch` file, or the JSON of the model) and prints the report. The board is optional and only tells a `.sch` file which footprints exist.

| Rule | Code | Default | What it finds |
|---|---|---|---|
| `pin_unconnected` | PHOENIX-ERC-001 | error | a pin of a placed part that is on no net (a pin of type `not_connected` is meant to stay open) |
| `single_pin_net` | PHOENIX-ERC-002 | warning | a net with one pin or none |
| `pin_type_conflict` | PHOENIX-ERC-003 | by the matrix | two pins on one net whose types the matrix does not allow together; one finding per net and pair of types, with the first two pins and the number of other pairs |
| `input_without_driver` | PHOENIX-ERC-004 | error | a net with input pins and no pin of a type that drives |
| `power_without_source` | PHOENIX-ERC-005 | error | a net with power input pins and no power source |
| `similar_net_names` | PHOENIX-ERC-006 | warning | names that differ only by case or by a number on the end (`GND` and `GND1`, `VCC` and `Vcc`); the nets `D0`, `D1` of a bus are not mistaken for each other |
| `duplicate_refdes` | PHOENIX-ERC-007 | error | a reference designator used by more than one part |
| `empty_value` | PHOENIX-ERC-008 | warning | a part with an empty value |
| `empty_refdes` | PHOENIX-ERC-009 | error | a part with an empty reference designator |
| `part_without_footprint` | PHOENIX-ERC-010 | warning | a part with no footprint chosen |
| `pin_without_number` | PHOENIX-ERC-011 | error | a pin of a symbol with an empty number |
| `duplicate_pin_number` | PHOENIX-ERC-012 | error | a symbol with one pin number on more than one pin |
| `floating_wires` | PHOENIX-ERC-013 | warning | wires that are joined to each other but to no pin |
| `junction_on_floating_wires` | PHOENIX-ERC-014 | warning | a point where three or more wire pieces meet, on wires that join no pin |

What the net list says is the truth for the connections. The wires of the drawing are only looked at for loose ends, with the rules of the drawing (`draw.rs` of the schematic crate): wires and pins are connected where they share a point, and a point in the middle of a wire's piece is not a connection. A wire group is loose when no pin of a placed part with a given position lies on any of its points.

### The report: confirmed and inconclusive

Like the board checks, the report keeps two lists apart. `confirmed` holds what is proven, with its level (`info`, `warning`, `error`). `inconclusive` (code PHOENIX-ERC-015) holds what could not be judged and is never turned into a failure or a pass: a net whose driver or source check depends on pins without a type (the pins are named), loose wires when a placed part has pins without a position, a single line saying how many pins on nets have no type, and a note when the list was cut at `max_findings`. The lists are sorted in a fixed order (rule, place, detail), by names and never by ids, so the same schematic and settings always give the same text and the same JSON (`phoenix-erc/1`), whatever the order in which the objects were made. The text is in en, es, de, fr, it, ja, zh and pt (the Portuguese of Portugal). Exit code: 0 no error is proven, 1 an error is proven, 2 the arguments or the input are unusable.

### Settings

Everything that is tuned is in `config/erc.cfg`, which is also the default that is built into the program. A line `severity.<rule> = off | info | warning | error` sets the level of a rule; for `pin_type_conflict` it is only a switch and the matrix decides the level of each pair. A line `matrix.<type>.<type> = ok | info | warning | error` sets what two pins of those types mean on one net; the matrix is symmetric and a pair that is not listed is allowed. `drivers` lists the types that drive a net, `power_sources` the types that supply a power input, `similar_name_min_stem` the shortest stem of a name that is compared, and `max_findings` bounds the list. The default matrix is written by hand from the usual meaning of the types (two outputs, an output and a supply output, and any pin with a pin meant to stay open are errors; an output with a bidirectional pin is a warning) and has not been compared with the matrix of the reference tool.

## Synchronisation by identity

A component of the board is made from a part of the schematic and holds its id (`Component::symbol`). That link is how the two documents are compared, never a name, so a renumbered reference designator is a change and not a new part. `phoenix sync [--backward] [--apply --out FILE] [--json | --package] <schematic> <board>` compares them.

| Change | Code | Forward (schematic to board) |
|---|---|---|
| a part with no component | PHOENIX-SYNC-001 | a component is added with the value and footprint of the part, in a row at the place the settings give |
| a component whose part is gone | PHOENIX-SYNC-002 | listed and kept, or removed when the settings say so |
| other reference designator | PHOENIX-SYNC-003 | the component takes the one of the part |
| other value | PHOENIX-SYNC-004 | the component takes the one of the part |
| other footprint | PHOENIX-SYNC-005 | the pads lose their nets, the component takes the footprint, the nets are given again by the pins |
| a net the board does not have | PHOENIX-SYNC-006 | a net is added |
| a net with another name | PHOENIX-SYNC-007 | the net of the board that all the pads of the schematic net sit on is renamed (when no other net of the schematic claims it), so its tracks stay on it |
| a pad on another net | PHOENIX-SYNC-008 | the pad is put on the net of its pin, or on none when its pin is on none |

Backward, the reference designators, values and footprints of the board are written into the parts (the footprint also into a part that has none chosen, unless the settings say otherwise). The nets of the schematic are never changed by the annotation back.

What the plan cannot judge or do goes to the inconclusive list and is never guessed: a footprint that is not chosen or not in the board (009), a pin whose pad does not exist (010), a part made into two components (011), a reference designator that would be empty or used twice (012), two nets of the schematic with one name (013). The settings are in `config/sync.cfg`.

### The package

The result of planning is a package: the changes in a fixed order, what was left out and the operations that make them. Planning is a dry run, so the command writes nothing unless `--apply --out FILE` is given, and an input file is never overwritten. Applying a package to a board is one step of the undo/redo history of the board (`History`), applying it to a schematic one step of `SchematicHistory`; both are all-or-nothing, undoing gives back the original byte for byte and redoing gives the same result. Applying the package a second time finds nothing to do. What the package does not name is not touched: no track, via, zone, silkscreen item or position is changed or removed. The package is also available as JSON (`--package`). Exit code: 0 the documents agree or the package was applied, 1 a dry run found changes to make, 2 the arguments or the input are unusable.

The result is written as the JSON of the model (`.json`), or as EAGLE XML when the name ends in `.brd` (the board) or `.sch` (the schematic, backward). The EAGLE writers keep less than the model does, as their own pages say.

## Net classes

A rule file of the format `phoenix-rules/2` is a rule file of the first format with an optional list of net classes; a file of the first format is read as before. A class has a name, the names of its nets (a net belongs to at most one class) and limits of its own: `min_track_width_nm`, `min_via_diameter_nm` and `min_via_drill_nm`, checked by `phoenix check --rules FILE` (codes PHOENIX-CHECK-014 and PHOENIX-CHECK-015), and `min_clearance_nm`, which is read and kept but not checked yet (the check says so in the inconclusive list). A class only adds a limit: it never relaxes the general rules, which are still checked for every net. A class that is not valid has the code PHOENIX-RULEFILE-005.

## What is not done

Buses as objects of their own, hierarchical sheets and labels, power flags, a type chosen per part instead of per pin, the pin types of the KiCad files, net ties, the assignment of a class to a net in the schematic, the clearance between nets of different classes and the comparison of any of this with the reference tool are listed in `NOT_YET.md`.
