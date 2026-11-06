# Mehrere Objekte im Platineneditor auswählen und als einen Schritt verschieben, ausrichten oder löschen

Die Auswahl des Platineneditors ist eine geordnete Menge von Bauteilen, Bahnen, Durchkontaktierungen, Zonen und Bestückungsdruck-Elementen. Ohne Bildschirm steuern Sie sie mit `phoenix boardedit` und einer Operationsdatei: `select` ändert die Auswahl, und `move_selection`, `delete_selection` und `align_selection` wirken darauf als ein Schritt des Verlaufs mit exakter Umkehrung. `examples/guides/select_ops.json` wählt mit einem Rechteck, was ganz darin liegt, hier den Widerstand R1 und nicht die Bahn, und verschiebt ihn um 2 mm nach oben.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Funktionsanleitungen aus](README.md).

1. Wenden Sie die Operationen als Probelauf an. Der Bericht listet die drei Operationen und die Regelfehler davor und danach; es wird nichts geschrieben. Die Regeldatei wird angegeben, weil die EAGLE-Platine keine eigenen Entwurfsregeln hat.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json examples/tutorials/tut.brd
```

2. Schreiben Sie die bearbeitete Platine in eine neue Datei (`.json` für das Entwurfsmodell, `.brd` für EAGLE). Die Eingabe wird nie überschrieben. Der Exitcode ist 0, wenn die Regeln im Ergebnis keinen Fehler beweisen, und 1, wenn sie einen beweisen.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --out build/moved.json examples/tutorials/tut.brd
```

3. Geben Sie den Bericht als JSON aus, das ein Skript lesen kann: Der Schlüssel `rule_errors` ist die Zahl der Fehler, die die Regeln im Ergebnis beweisen.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --json examples/tutorials/tut.brd
```

## So lesen Sie das Ergebnis

* Die Schritte von `select` sind `add`, `remove` und `toggle` (die Objekte von `items`, geschrieben `{"kind": "component", "id": "..."}`), `net` (alle Objekte eines Netzes, über seine Id), `rect` (ein Rechteck zwischen `from` und `to`, in Nanometern als `x_nm` und `y_nm`; `mode` ist `contained` oder `crossed`, das auch erfasst, was das Rechteck berührt), `all`, `invert` und `clear`. Sie werden der Reihe nach und ganz oder gar nicht angewendet, und die Auswahl gehört nicht zum Verlauf.
* `move_selection` verschiebt um `dx_nm` und `dy_nm` (oben ist positiv), auf das Raster gerundet, außer `snap` ist false; nicht ausgewählte Bahnen folgen einem Bauteil nicht, wählen Sie also zuerst das Netz, um das Kupfer mitzunehmen. `delete_selection` entfernt die Auswahl. `align_selection` kennt `left`, `right`, `top`, `bottom`, `center_x`, `center_y` (mindestens zwei Objekte) und `distribute_x`, `distribute_y` (mindestens drei). Ein gesperrtes Bauteil lehnt die ganze Operation ab (`PHOENIX-BEDIT-006`), eine leere Auswahl ist `PHOENIX-BEDIT-009`.
* Im Fenster, mit dem Werkzeug Auswählen: Umschalt oder Strg+Klick schaltet ein Objekt um; Umschalt oder Strg+Ziehen an einer leeren Stelle fügt hinzu, was ganz im Rechteck liegt (auch Zonen und Bestückungsdruck-Elemente); Alt+Ziehen fügt auch hinzu, was das Rechteck berührt; `N` wählt das Netz unter dem Zeiger; Strg+A wählt alles; Escape hebt die Auswahl auf; die Pfeiltasten verschieben sie um einen Rasterschritt (Umschalt: zehn Schritte); Entf löscht sie; Alt+Links, Rechts, Oben, Unten, Alt+C und Alt+M richten sie aus; Alt+H und Alt+V verteilen sie mit gleichen Abständen (mindestens drei Objekte); Strg+I kehrt die Auswahl um; ein Klick auf einen Bestückungsdruck oder auf den Rand einer Fläche wählt ihn; das Ziehen eines gewählten Objekts verschiebt die ganze Auswahl im Raster; die Statusleiste zeigt die Zahl der Objekte mit ihrem Rahmen und ihrer Größe. Jedes davon ist ein Schritt des Verlaufs, und Strg+Z liefert dieselben Bytes zurück. Das Fenster wurde weder auf einem Bildschirm gesehen noch mit einer Maus benutzt.

## Siehe auch

* [docs/BOARD_EDIT.md](../../../../docs/BOARD_EDIT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
* [docs/VIEWER.md](../../../../docs/VIEWER.md)
