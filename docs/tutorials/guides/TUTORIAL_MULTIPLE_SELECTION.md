# Select several objects in the board editor and move, align or delete them as one step

The selection of the board editor is an ordered set of components, tracks, vias, zones and silkscreen items. Without a screen you drive it with `phoenix boardedit` and a file of operations: `select` changes the selection, and `move_selection`, `delete_selection` and `align_selection` act on it as one step of the history with an exact inverse. `examples/guides/select_ops.json` selects with a rectangle what lies wholly inside it, which here is the resistor R1 and not the track, and moves it 2 mm up.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. Apply the operations as a dry run. The report lists the three operations and the rule errors before and after; nothing is written. The rule file is given because the EAGLE board has no design rules of its own.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json examples/tutorials/tut.brd
```

2. Write the edited board to a new file (`.json` for the design model, `.brd` for EAGLE). The input is never written over. The exit code is 0 when the rules prove no error on the result and 1 when they prove one.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --out build/moved.json examples/tutorials/tut.brd
```

3. Print the report as JSON, which a script can read: the key `rule_errors` is the number of errors that the rules prove on the result.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --json examples/tutorials/tut.brd
```

## How to read the result

* The steps of `select` are `add`, `remove` and `toggle` (the objects of `items`, written `{"kind": "component", "id": "..."}`), `net` (every object of a net, by its id), `rect` (a rectangle between `from` and `to`, in nanometres as `x_nm` and `y_nm`; `mode` is `contained` or `crossed`, which also takes what the rectangle touches), `all`, `invert` and `clear`. They are applied in order and all or nothing, and the selection is not part of the history.
* `move_selection` moves by `dx_nm` and `dy_nm` (up is positive), rounded to the grid unless `snap` is false; the tracks that are not selected do not follow a component, so select the net first to carry the copper along. `delete_selection` removes the selection. `align_selection` takes `left`, `right`, `top`, `bottom`, `center_x`, `center_y` (two objects at least) and `distribute_x`, `distribute_y` (three at least). A locked component refuses the whole operation (`PHOENIX-BEDIT-006`), an empty selection is `PHOENIX-BEDIT-009`.
* In the window, with the tool Select: Shift or Ctrl+click toggles an object; Shift or Ctrl+drag on an empty place adds what lies wholly inside the rectangle (zones and silkscreen items too); Alt+drag also adds what the rectangle touches; `N` selects the net under the pointer; Ctrl+A selects everything; Escape drops the selection; the arrows move it one step of the grid (Shift: ten steps); Delete deletes it; Alt+Left, Right, Up, Down, Alt+C and Alt+M align it. Each of these is one step of the history, and Ctrl+Z gives back the same bytes. The window has not been seen on a screen or used with a mouse.

## See also

* [docs/BOARD_EDIT.md](../../../docs/BOARD_EDIT.md)
* [docs/BOARD_ZONES.md](../../../docs/BOARD_ZONES.md)
* [docs/VIEWER.md](../../../docs/VIEWER.md)
