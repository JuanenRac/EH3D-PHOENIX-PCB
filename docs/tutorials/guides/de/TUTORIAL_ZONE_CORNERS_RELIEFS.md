# Die Ecken einer Kupferfläche runden und Wärmefallen sowie Inselgrenzen in die Gerber-Dateien schreiben

Die Platine `examples/guides/pour.brd` hat eine Massefläche auf der Lage `Top` um zwei Widerstände und eine Bahn. Sie füllen sie mit scharfen und mit gerundeten Ecken und vergleichen, und schreiben dann die Gerber-Dateien mit einem Profil, das den rechteckigen Pads der Fläche eine Wärmefalle gibt und ihre kleinen Inseln weglässt. Die Regeldatei `rules_relaxed.json` wird angegeben, weil eine EAGLE-Platine ohne Entwurfsregeln keine eigenen Grenzen hat.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Funktionsanleitungen aus](README.md).

1. Füllen Sie die Zone mit den Einstellungen des Repositorys, die alle Ecken scharf lassen (`corner_radius_nm = 0`). Der Bericht nennt die Polygone und die Quadratnanometer Kupfer.

```console
$ phoenix zones fill --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

2. Füllen Sie sie erneut mit `zones_round.cfg`, das `corner_radius_nm = 1000000` setzt (ein Bogen von 1 mm). Die nach außen zeigenden Ecken der Fläche und die Ecken der Lücken der Wärmefalle werden gerundet; vergleichen Sie die Kupferfläche mit dem ersten Lauf.

```console
$ phoenix zones fill --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

3. Messen Sie die gerundete Füllung gegen Platine und Regeln. Das Runden nimmt Kupfer von der Fläche weg oder fügt es nur innerhalb einer Lücke einer Wärmefalle hinzu; die Füllung kommt dem Kupfer eines anderen Netzes also nicht näher, und die Prüfung findet keinen Abstandsfehler.

```console
$ phoenix zones check --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

4. Schreiben Sie die Gerber-Dateien zuerst mit dem generischen Profil. Das Pad `R2.2` des Massenetzes liegt in der Fläche und bleibt ohne Wärmefalle mit ihr verbunden.

```console
$ phoenix export --out build/pour_plain examples/guides/pour.brd
```

5. Nun mit dem Profil `pour` aus `export_pour.json`: vier Stege (`gerber.zone_thermal_spokes`), eine Lücke und ein Steg von 0,25 mm, dieselbe Wärmefalle für rechteckige und längliche Pads (`gerber.zone_thermal_non_round`) und keine Insel unter 1 Quadratmillimeter (`gerber.zone_island_min_area_nm2`). Vergleichen Sie die Kupferdatei der Lage `Top` in beiden Ordnern: Die zweite enthält die Lücken der Wärmefalle als Bereiche mit heller Polarität um das Pad.

```console
$ phoenix export --profiles examples/guides/export_pour.json --profile pour --out build/pour_relief examples/guides/pour.brd
```

## So lesen Sie das Ergebnis

* In den Einstellungen der Füllung ist `corner_radius_nm` der Radius des Bogens (null lässt alle Ecken scharf) und `corner_tolerance_nm` der Abstand, den die Sehnen des Bogens von ihm haben dürfen. Ein Bogen nimmt höchstens die Hälfte jeder Seite an der Ecke ein; ein Radius, der nicht passt, wird verkleinert. Die Ecken der Löcher, die anderes Kupfer in der Fläche lässt, werden nicht gerundet.
* Beim Export hat eine Wärmefalle vier Stege oder zwei (`gerber.zone_thermal_spokes` ist 0, 2 oder 4; 0 lässt die Pads massiv verbunden). Ein um einen nicht rechten Winkel gedrehtes Pad bleibt massiv und meldet das (`PHOENIX-EXPORT-022`). Eine Insel kleiner als `gerber.zone_island_min_area_nm2` wird nicht geschrieben, und der Bericht sagt es (`PHOENIX-EXPORT-021`); null schreibt jede Zone.

## Siehe auch

* [docs/ZONES.md](../../../../docs/ZONES.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
