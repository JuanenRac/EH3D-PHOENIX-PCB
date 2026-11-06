# Eine Platine nutzen und die Nutzendateien schreiben

Sie ordnen die Platine von 30 mm × 20 mm zweimal nebeneinander an, verbunden durch eine Ritzlinie, und schreiben die Gerber- und Bohrdateien des ganzen Nutzens. Die Einstellungen stehen in `examples/tutorials/panel.cfg` (Spalten, Zeilen, Art der Verbindung, Spalt und Rahmen); die vollständige Liste ist `config/panel.cfg`.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Anleitungen aus](README.md).

1. Planen Sie den Nutzen. Ein Probelauf gibt Größe, Verbindung, Bohrungen und die Dateien aus, die geschrieben würden, und schreibt nichts.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ joint: V-score, 1 lines
~ dry run: nothing written
```

2. Schreiben Sie die Lagen jeder Kopie, die Bohrdateien, den Umriss mit der Ritzlinie, die Bestückung und die Stückliste in einen neuen Ordner.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ written
```

3. In einen vorhandenen Ordner wird nie geschrieben: Der zweite Lauf wird mit dem Code `PHOENIX-PANEL-008` und Exit-Code 2 abgelehnt.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
# exit 2
~~ PHOENIX-PANEL-008
```

4. Prüfen Sie den Aufbau einer Gerber-Datei des Nutzens und zeichnen Sie den Nutzen als DXF-Zeichnung für die Mechanik.

```console
$ phoenix gerber-check build/panel/panel_01_top.gbr
$ phoenix dxf-export panel --out build/panel.dxf --panel-config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
```

## Siehe auch

* [docs/PANEL.md](../../PANEL.md)
* [docs/GERBER_IMPORT.md](../../GERBER_IMPORT.md)
* [docs/DXF.md](../../DXF.md)
* [docs/FABPACK.md](../../FABPACK.md)
