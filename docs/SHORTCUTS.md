# EH3D-PHOENIX-PCB: keyboard shortcuts of the window

State: **prototype**. The window has never been seen on a screen or used with a mouse; the table that maps keys to actions is tested without one (`crates/eh3d-phoenix-viewer/src/keymap.rs`).

## The table

The list of shortcuts that the window shows (Ctrl+/) is made from the same table as this page. Each action has a word, which names it in `config/viewer.cfg` after `key.`, and the keys it has when the settings do not change them. The keys of the first nineteen actions are the ones that the window has always answered to.

| Action | What it does | Default keys |
|---|---|---|
| `undo` | Undo the last edit | Ctrl+Z |
| `redo` | Redo the last edit that was undone | Ctrl+Y, Ctrl+Shift+Z |
| `save` | Ask where to save the document | Ctrl+S |
| `rotate` | Turn the selected component by one step | R |
| `rotate_back` | Turn the selected component backwards | Shift+R |
| `tool_select` | The select tool | V |
| `tool_measure` | The measure tool | M |
| `tool_move` | The tool that moves components | G |
| `tool_rotate` | The tool that turns components | O |
| `tool_delete_track` | The tool that deletes tracks | D |
| `tool_move_vertex` | The tool that moves vertices of tracks | E |
| `tool_width` | The tool that changes the width of tracks | W |
| `tool_via` | The tool that adds vias | I |
| `tool_route` | The tool that routes tracks | T |
| `fit` | Fit the drawing to the window | F |
| `delete` | Delete the selected track | Delete |
| `cancel` | Cancel what is in progress and clear the selection | Escape |
| `width_up` | Make the selected track wider | + |
| `width_down` | Make the selected track narrower | - |
| `search` | Go to the search box | Ctrl+F |
| `search_next` | Go to the next result of the search | F3 |
| `search_previous` | Go to the previous result of the search | Shift+F3 |
| `help` | Show the help of the active mode | F1 |
| `shortcuts` | Show the list of shortcuts | Ctrl+/ |
| `history` | Show the history of the document | Ctrl+H |
| `solo_layer` | Show only the active layer, and then all again | L |
| `highlight_net` | Light the net of the selected item | N |
| `clear_measures` | Delete every measure | Shift+M |
| `toggle_unit` | Change the unit of the measures | U |
| `preset_1` | Apply the layer preset number 1 | 1 |
| `preset_2` | Apply the layer preset number 2 | 2 |
| `preset_3` | Apply the layer preset number 3 | 3 |
| `preset_4` | Apply the layer preset number 4 | 4 |
| `preset_5` | Apply the layer preset number 5 | 5 |
| `preset_6` | Apply the layer preset number 6 | 6 |
| `preset_7` | Apply the layer preset number 7 | 7 |
| `preset_8` | Apply the layer preset number 8 | 8 |
| `preset_9` | Apply the layer preset number 9 | 9 |

## Changing the keys

A line of `config/viewer.cfg` such as `key.undo = ctrl+u` gives the action `undo` that key and no other. Several keys are separated by commas (`key.redo = ctrl+y, ctrl+shift+z`) and `none` leaves the action without a key (`key.help = none`).

* A key is written as the modifiers `ctrl` and `shift`, in any order, joined with plus signs to the key, in any case: `ctrl+shift+z`. `ctrl` is the Command key on macOS.
* The keys are the letters `a` to `z`, the digits `0` to `9`, `f1` to `f12`, `delete`, `escape`, `plus`, `minus`, `enter`, `tab`, `space`, `up`, `down`, `left`, `right`, `pageup`, `pagedown`, `home`, `end`, `slash`, `comma`, `period` and `backspace`. The plus sign is written `plus`.
* The modifiers must be exactly the ones of the key: `shift+v` is not `v`. The exception is `plus`, which on many keyboards is typed with Shift and is also taken from the equals key.
* An unknown action, a key that does not exist, an action given twice and two actions with the same key are errors (`PHOENIX-VIEW-005`) that name the line. To move a key from one action to another, free it first (`key.tool_select = none` and then `key.fit = v`).
* The keys are not read while a text box has the keyboard.
* The keys 1 to 9 apply the first nine layer presets that were saved (`docs/VIEWER.md`).

## What is not done

* Keys cannot be changed in the window, only in the settings file; the list in the window shows the keys that are in use.
* There are no sequences of keys, no keys with Alt and no keys for the buttons of the panels.
* The editors of boards and schematics keep keys of their own, which are not in this table; while one of them is open only the keys of the search, the history, the list of shortcuts and the help (`search`, `search_next`, `search_previous`, `history`, `shortcuts` and `help`) work from this table.
