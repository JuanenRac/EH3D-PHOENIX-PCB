# Buses as objects: vectors, groups, aliases and entries

`eh3d-phoenix-schematic` (module `bus`) has buses as objects of the drawing: a bus such as `D[0..7]` or `USB{DP DM}`, the entries that take a member out of it, and the aliases that give a name to a group. A bus is a way of naming: its members are ordinary nets, and the expansion of a bus into the names of its members is fixed, so the same bus always gives the same nets.

**State: prototype.** It has been tried on schematics made by hand and by the tests only, never on a design from a real project and never compared with the buses of KiCad or EAGLE. The window that places a bus has never been seen on a screen.

## Names

| Form | Members | Example |
|---|---|---|
| Vector `prefix[first..last]` | `prefix<first>` to `prefix<last>`, up or down, at most 256 | `D[0..7]` gives `D0` to `D7`; `A[2..0]` gives `A2`, `A1`, `A0` |
| Group `name{item item ...}` | one member `name.item` for each item; an item is a plain name or a vector | `USB{DP DM}` gives `USB.DP` and `USB.DM`; `P{D[0..1] CLK}` gives `P.D0`, `P.D1`, `P.CLK` |
| Alias | the name of an alias expands like the group of that name | the alias `SPI` with `MOSI MISO SCK` gives `SPI.MOSI`, `SPI.MISO`, `SPI.SCK` |

A bus may expand to at most 4096 members. A name with a prefix (`CPU_D[0..7]`) is a vector with that prefix. The older operation `add_bus` of the editor, which puts one label for each member, is still there and uses the same vectors.

## The objects

* `Bus`: a name (`spec`), the sheet and an anchor point where it is drawn.
* `BusEntry`: a bus, a member and a point of the sheet of the bus. It acts as a net label of the name of the member: a wire that ends at its point is on the net of the member, and two entries of one member join their nets. In a hierarchy the member is local to its instance like any label (`docs/HIERARCHY.md`).
* `BusAlias`: a name and the list of items. Replacing an alias changes what the buses that use it expand to.

None is written when a schematic has none of them.

## Operations

`place_bus`, `remove_bus` (with its entries), `add_bus_entry`, `remove_bus_entry`, `set_bus_alias` (creates the alias or replaces the one of that name) and `remove_bus_alias`. They are `EditOp` variants like all the others (all or nothing, one step for undo and redo, ids that do not depend on the clock). A name that does not expand, or a member that the bus does not have, is refused with `PHOENIX-SCH-010`.

## Checks

`PHOENIX-BUS-001` to `007` (`docs/ERROR_CODES.md`, with the titles in the eight languages): a bus name that is not valid, members that no entry uses (a warning), a bus with two members of one name, a range that is negative or too long, a bus whose anchor is a pin or a point of a wire (joined to a simple net), an entry for a bus or a member that does not exist, and an alias that cannot be used. `Schematic::validate` runs them, and `phoenix hier` lists them.

## Window

The panel of the schematic editor has the tools *Bus* (type the name, click where it goes) and *Bus entry* (choose the bus and the member, click where the member leaves the bus; the member that comes next is chosen after each click). The logic is in `schematic_bus` and is tested without a screen; the window (`schematic_bus_gui`) has never been seen on one, and its texts are in the eight languages.

## Not done

The list is in `docs/NOT_YET.md`. In short: the bus drawing is a mark at the anchor, with no wire along it; a bus entry is a point and not a diagonal piece with a size; a bus has no direction; the KiCad reader reads a bus label and the entries that end on a member label into these objects and the KiCad writer writes them (the bus as its label and a line from the anchor through the ends of its entries, the entry as a `bus_entry` with the label of its member), and the EAGLE reader reads a `bus` into a bus of the sheet for each vector in its name (`D[0..7]`, `A[0..3],B[0..3]`, `BUS:A[0..3]`), anchored where its first wire starts and expanded to its members, with an entry at the end of each net of the sheet that has the name of a member and ends on a wire of the bus; the wires and labels of the bus, the names in a bus that are not vectors, buses without a wire and names that cannot be expanded are counted (`PHOENIX-EAGLE-022`), and the EAGLE writer reports the buses and entries it does not write (`PHOENIX-EAGLE-021`).
