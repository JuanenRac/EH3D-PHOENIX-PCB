# Mehrere Nutzenanordnungen mit Vorlagen in einer Einstellungsdatei halten

`examples/guides/panel_templates.cfg` enthält eine Grundanordnung (zwei Platinen nebeneinander, durch eine Ritzlinie verbunden, ohne Rahmen) und zwei benannte Vorlagen. Jede Vorlage ändert nur die Zeilen, die sie nennt, und mit `--template` wählen Sie eine aus. Die Platine ist die der Nutzen-Anleitung, 30 mm × 20 mm; alle Befehle hier sind Probeläufe.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Funktionsanleitungen aus](README.md).

1. Ohne Vorlage wird nur die Grundanordnung gelesen, die Zeilen unter den Vorlagenabschnitten werden ignoriert: 60 mm × 20 mm, 2 x 1 Platinen.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg examples/tutorials/kicad/demo.kicad_pcb
```

2. Die Vorlage `four` ändert `columns` auf 4 und sonst nichts: 120 mm × 20 mm, 4 x 1 Platinen.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four examples/tutorials/kicad/demo.kicad_pcb
```

3. Die Vorlage `grid` ändert `rows` auf 2: 60 mm × 40 mm, 2 x 2 Platinen.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template grid examples/tutorials/kicad/demo.kicad_pcb
```

4. `--columns` und `--rows` haben weiterhin Vorrang vor einer Vorlage: mit `four` und `--columns 3` ist der Nutzen 90 mm × 20 mm, 3 x 1 Platinen.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --columns 3 examples/tutorials/kicad/demo.kicad_pcb
```

5. Ein Name, den die Datei nicht hat, wird mit dem Code `PHOENIX-PANEL-001` und Exitcode 2 abgelehnt.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template ring examples/tutorials/kicad/demo.kicad_pcb
```

6. Schreiben Sie die Dateien der gewählten Anordnung in einen neuen Ordner, wie in der Nutzen-Anleitung.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --out build/panel_four examples/tutorials/kicad/demo.kicad_pcb
```

## So lesen Sie das Ergebnis

* Die Zeilen vor der ersten Zeile `[template NAME]` sind die Grundanordnung. Ein Name hat 1 bis 32 Buchstaben, Ziffern, `_` oder `-`; ein anders geschriebener Abschnitt wird mit `PHOENIX-PANEL-001` und der Zeilennummer abgelehnt.
* Die Größenzeile lautet `panel W mm by H mm, C x R boards`. Bei Ritzverbindung und Spalt 0 berühren sich die Platinen, die Breite ist also die Spaltenzahl mal 30 mm und die Höhe die Zeilenzahl mal 20 mm. Eine Vorlage ist eine Möglichkeit, eine Anordnung unter einem Namen zu halten; sie ist keine Einstellung eines Herstellers.

## Siehe auch

* [docs/PANEL.md](../../../../docs/PANEL.md)
* [config/panel.cfg](../../../../config/panel.cfg)
