# Die Arbeit der Prüfungen mit einer Grenzwertdatei begrenzen

Manche Prüfungen brauchen Zeit im Quadrat einer Anzahl, zum Beispiel der Test, dass sich die Kanten eines Polygons nicht kreuzen. Ein Grenzwert begrenzt diese Zeit: Ein Polygon über dem Grenzwert wird nicht auf Kreuzungen geprüft, und der Bericht nennt das als Information. Sie führen die Prüfung einer Platine mit den Standardwerten aus und dann mit einer viel zu niedrigen Grenzwertdatei und sehen den Hinweis.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Anleitungen aus](README.md).

1. Führen Sie die Regeldatei auf der KiCad-Platine mit den Standardgrenzen aus. Die Leiterbahn der Platine ist dünner, als die Datei verlangt; es gibt also einen bestätigten Befund und der Exitcode ist 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. Führen Sie sie noch einmal mit `--limits examples/guides/limits_small.cfg` aus, die `max_polygon_points = 3` enthält. Der Umriss der Platine hat vier Punkte; er wird daher als Information mit dem Code `PHOENIX-BOARD-017` aufgeführt, und der Exitcode bleibt 1, weil Informationen ihn nie ändern.

```console
# test
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ PHOENIX-BOARD-017
~ max_polygon_points = 3
~ PHOENIX-CHECK-001
```

3. Derselbe Hinweis in einer anderen Sprache und als JSON für ein Skript. Der Code bleibt englisch; der Titel folgt `--lang`.

```console
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --lang pt examples/tutorials/kicad/demo.kicad_pcb
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --json examples/tutorials/kicad/demo.kicad_pcb
```

## So lesen Sie das Ergebnis

* `--limits DATEI` akzeptiert jeder Befehl, vor oder nach seinem Namen. Die Datei enthält Zeilen `Schlüssel = Wert` aus `config/board.cfg` und `config/eagle_board.cfg` in beliebiger Reihenfolge; eine unbekannte Zeile oder ein Wert außerhalb seines Bereichs wird mit der Zeilennummer und dem Exitcode 2 abgelehnt. Die beiden Schlüssel von `config/outline_arcs.cfg` (`tolerance_nm` und `max_pieces`, die bestimmen, wie die Bögen der Umrandung in Sehnen zerlegt werden) dürfen in derselben Datei stehen.
* Ein Grenzwert ändert nur, was er begrenzt. Mit den Standardwerten ändert sich keine Ausgabe, und bei einem Polygon über `max_polygon_points` werden weiterhin die wiederholten Punkte geprüft.

## Siehe auch

* [config/board.cfg](../../../../config/board.cfg)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
* [docs/RULES.md](../../../../docs/RULES.md)
