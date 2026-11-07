# EH3D-PHOENIX-PCB: copper zones in the board editor

**State: prototype.** The board editor (`docs/BOARD_EDIT.md`) draws, edits, fills and removes copper zones, keeps the priority and the other properties of each zone, copies and pastes a net or a group of copper, and hands the filled copper to the writers of files. It has been tried on boards made by hand and by the tests only. The fill is the one of `eh3d-phoenix-zones` (`docs/ZONES.md`), which has never been compared with the fill of the reference tool, and the window of the tool has never been seen on a screen or used with a mouse. Nothing here has been opened by a viewer of Gerber files other than the import of this repository.

## What the editor keeps of a zone

The board model holds a zone as a net, a layer, an outline and a clearance. The rest lives in the data of the zones, which belongs to the editor, goes through undo and redo together with the board and is written as a file of its own (format `phoenix-zone-data/1`):

| Property | Meaning |
|---|---|
| `priority` | A zone of a higher priority is filled first; the zones below it keep away from its whole outline. The default is 0. |
| `min_width_nm`, `min_island_area_nm2` | The narrowest and the smallest island of copper that is kept. They default to `config/zones.cfg`. |
| `connection` | How the pads of the net are joined: `thermal`, `solid` or `none`. |
| `thermal_gap_nm`, `thermal_spoke_width_nm`, `thermal_spokes` | The thermal relief of this zone alone (the gap, the width of the spokes, 2 or 4 spokes). A relief that cannot be made is `PHOENIX-ZONE-011` and the pad is joined solid. |
| `cutouts` | Holes: closed polygons that lie inside the outline and that the fill keeps away from. |
| the fill | The polygons of copper that the last fill made (`FilledZone` of `eh3d-phoenix-zones`, the format `phoenix-zone-fill/1`). |

The model has no room for any of it, so the fill is kept as data of the editor and not as polygons of the model. A zone that is edited, moved to another priority or cut loses its fill, and the notes of the operation say so (`PHOENIX-BZONE-007`).

## The operations

They are operations of the file of `phoenix boardedit --apply`, like all the others (`docs/BOARD_EDIT.md`): reversible, all or nothing, one step of the history each, and the design rules are checked before and after.

| Operation (`"op"`) | What it does |
|---|---|
| `zone_add` | Adds a zone: a net, a copper layer, an outline of at least three points that is a polygon with an area whose sides do not touch, the clearance (the clearance of the rules when left out) and the `props` above. The fill is made by `zone_fill`. |
| `zone_edit` | Changes the net, the layer, the outline (its vertices), the clearance or the properties of a zone; what is left out stays. A cutout is taken out with `"remove_cutout": <index>`. The cutouts must stay inside the new outline. |
| `zone_remove` | Removes a zone with its properties and its fill. |
| `zone_fill` | Fills all the zones together (the priorities depend on each other) with the settings of `config/zones.cfg`, proves the result with the check of `eh3d-phoenix-zones` and keeps the fills. With `"zone": "<id>"` only the fill of that zone is kept. A fill that breaks a clearance, an outline that cannot be filled or a zone that cannot be found is refused and changes nothing. |
| `zone_priority` | Gives a zone a priority. |
| `zone_cutout` | Cuts a hole: a polygon of at least three points, with an area, inside the outline of the zone and crossing none of its sides. |
| `paste_group` | Pastes a net or a group, see below. |

The rules of the board call two zones of different nets that overlap on one layer an error (`PHOENIX-CHECK-010`). The editor does not count the overlap of two zones whose priorities differ, because the fill of the one above makes room for the other; two zones of one priority that overlap are still refused.

### The report

Every operation of the fill carries notes with stable codes, and `BoardEdit::zone_issues` gives the report of the board as it is now:

* `PHOENIX-BZONE-007`: a zone with no fill (it is written as its outline);
* `PHOENIX-BZONE-006`: an island that the fill dropped (too small, too narrow, joined to no copper of its net) or a zone that is empty after the fill; with `unconnected_island_action = warn` an island joined to no copper of its net is kept and comes with the warning `PHOENIX-ZONE-020` of the fill, which the editor passes on as it is;
* `PHOENIX-BZONE-005`: a proven break: copper that was added after the fill and lies inside it or nearer than the clearance, a polygon outside the zone or the board; the notes of a fill add two zones of one priority that overlap. A fill that is kept never short-circuits two nets: the check of the fill proves that no pad, track or via of another net is inside a polygon or nearer than the clearance, and a fill that fails is refused. The report shows what a later edit broke, until the fill is made again.

## Copy and paste

`Clipboard::copy_net` and `Clipboard::copy_items` copy the tracks, vias and zones (with their properties and cutouts, not their fills) of a net or of the items given. The clipboard names nets and layers and keeps the points relative to its reference point, the lower left corner of its box, so it can go to another board; it is a JSON file (`phoenix-clipboard/1`).

`paste_group` pastes a clipboard, or copies the net `from_net` of the same board, with:

* `at`: where the reference point goes;
* `quarter_turns` (0 to 3, counter-clockwise) and `mirror` (about the vertical axis of the reference point, before the turn): the points stay exact integers, and the sweep of an arc changes its sign in a mirror;
* `net_map`: names to give the copies instead of their own. A net the board does not have is made; a layer it does not have is `PHOENIX-BZONE-008`.

The ids of the copies come from the supply of the editor, so the same board and the same paste give the same bytes. A paste is refused (`PHOENIX-BZONE-008`) whenever the design rules find a new error, also when `reject_new_errors` is off; wrapped in `forced` it is kept and reported.

## Writing the filled copper

* **Gerber.** `BoardEdit::poured_board` (and `phoenix boardedit --poured-out FILE`) gives the board with every filled zone replaced by one zone for each polygon of its copper. The export writes each as a region of the copper layer (with the net attribute when the profile writes it); the import reads the regions back (`docs/GERBER_IMPORT.md`), one for each polygon. The polygons are convex, touch along their sides and are all dark. A zone with no fill stays as its outline, and the export clears out of every zone the `cutouts` of its model and a keep-away around the copper of other nets, with objects of clear polarity (`docs/EXPORT.md`).
* **Specctra DSN and KiCad.** Both carry the outline of a zone (the router sees it as a plane of its net, KiCad fills it again). `export_issues` and the notes of `phoenix boardedit` name, for each zone, what a format cannot say (`PHOENIX-BZONE-010`): the fill, the cutouts, the priority, the connection and the thermal relief. Nothing is dropped without a note.

## The commands

```text
phoenix boardedit [...] [--zones FILE] [--zones-config FILE] [--zones-out FILE] [--poured-out FILE] --apply OPERATIONS.json <board>
phoenix zones fill|check [...] --edit OPERATIONS.json <board>
```

* `--zones` starts the editor from the data of the zones that an earlier run wrote; `--zones-config` is `config/zones.cfg` when it is left out and the file exists; `--zones-out` and `--poured-out` write a **new** `.json` file, never over an input file or a file that exists (`PHOENIX-BEDIT-003`). The text and the JSON report print the notes of the zones and what each format cannot say.
* `phoenix zones fill|check --edit` applies the file of operations in memory, to the board that was read, and fills (or checks) the result: a dry run that shows what the fill would be after them. Nothing is written but a fill asked for with `--out` to a new file.

## The window

The tool *Zone* of the board mode (`board_zone`, `board_zone_text`, `board_zone_gui` in `eh3d-phoenix-viewer`) is state without a screen: the polygon is drawn by clicks on the grid (snapping follows the setting of the editor), a click on the first point closes it, a vertex is picked and dragged, the preview of the fill is made on a copy of the editor and drawn as thin outlines, the zones are listed by priority with buttons to raise, lower, edit the vertices and remove, and the polygon is a new zone, the new outline of the zone chosen or a cutout. The panel is behind the `gui` feature, with its texts in the eight languages. The state is tested; the window has never been seen on a screen.

## Codes

`PHOENIX-BZONE-001` to `PHOENIX-BZONE-011` are in `docs/ERROR_CODES.md`, with their titles in the eight languages in `eh3d-phoenix-boardedit` (`src/zone_codes.rs`).

## Limits

* The properties and fills are data of the editor, not of the model: a board written without the file of zone data has its zones as they are, with the defaults.
* Mirroring is geometric: the layers are not swapped between the top and the bottom side.
* The check of a fill does not look at the cutouts, which the fill keeps away from by construction.
* The fill is the one of `docs/ZONES.md`: a set of convex polygons per island, circles as polygons around them, a pad at another angle as the smallest circle around it. The speed of the fill on a large board has not been measured, and the tool makes the preview on demand only.
* Keep-out areas, zones on several layers at once and the merging of the polygons into one outline per island are not done (`docs/NOT_YET.md`).
