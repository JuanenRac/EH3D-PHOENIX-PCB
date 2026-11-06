# Bilder der Fenster mit dem Bildmodus von phoenix aufnehmen

`phoenix --shot DATEI.png` öffnet das Fenster, wartet einige Bilder, speichert ein Bild davon als PNG-Datei und schließt sich selbst. So entstehen die Bilder des READMEs. In diesem Modus liest und schreibt das Fenster weder Einstellungen noch eine Liste zuletzt benutzter Dateien, ein Lauf lässt Ihre Einstellungen also unverändert. `phoenix` ist das Programm des Fensters, neben `phoenix` im Archiv der Version; es wird mit dem Feature `gui` gebaut.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Funktionsanleitungen aus](README.md).

1. Nehmen Sie den Startbildschirm auf, mit Größe, Thema und Zahl der zu wartenden Bilder. Die Größe ist `BREITExHÖHE` in Pixeln, jede Seite von 320 bis 8192; das Thema ist `dark`, `light` oder `contrast`.

```console
phoenix --shot build/start.png --size 1280x720 --theme dark --wait 6
```

2. Nehmen Sie das Fenster einer Platine auf: `--open` gibt die Datei an, und das gezeigte Fenster ist standardmäßig `document`. Eine Platine braucht `--open`; der Startbildschirm darf keines haben.

```console
phoenix --shot build/board.png --open examples/tutorials/led_board.brd --theme light --wait 12
```

3. Nehmen Sie mit `--window` ein anderes Fenster auf: `3d`, `panel`, `gerber`, `search`, `measures`, `layers`, `history`, `appearance`, `shortcuts` oder `help` (`3d`, `panel` und `gerber` brauchen `--open`). `--lang` legt die Sprache des Fensters fest.

```console
phoenix --shot build/help_pt.png --window help --lang pt --size 1280x720
```

4. Nehmen Sie den ganzen Satz Bilder einer Version mit dem Werkzeug des Repositorys auf. `--dry-run` gibt die Befehlszeilen aus und führt nichts aus; ohne es startet das Werkzeug das Programm einmal je Fenster, prüft jede PNG-Datei (sie darf nicht leer oder einfarbig sein und muss unter 400 KB wiegen) und schreibt die Dateien `phoenix-pcb-NN-FENSTER.png`.

```console
py -3.14 -I tools/make_shots.py --exe PATH/phoenix --out build/screens --dry-run
```

## So lesen Sie das Ergebnis

* Das Bild ist das, was das Fenster gezeichnet hat, in physischen Pixeln; auf einem Bildschirm mit Skalierung kann es also größer als `--size` sein. Der Exitcode ist 0, wenn das Bild geschrieben wurde, 1, wenn nicht, und 2 bei einem Fehler in den Argumenten. Eine vorhandene Datei wird nur mit `--replace` überschrieben; ohne es bricht der Lauf ab, bevor das Fenster sich öffnet.
* Jeder Lauf des Werkzeugs hat einen eigenen Einstellungsordner in einem temporären Ordner, daher werden Ihre Einstellungen und zuletzt benutzten Dateien weder gelesen noch geschrieben. Die Aufnahme selbst wurde noch nicht auf einem Bildschirm ausgeführt, und es wird nicht behauptet, dass ein Bild richtig aussieht, bevor Sie es gesehen haben.

## Siehe auch

* [docs/VIEWER.md](../../../../docs/VIEWER.md)
* [tools/make_shots.py](../../../../tools/make_shots.py)
