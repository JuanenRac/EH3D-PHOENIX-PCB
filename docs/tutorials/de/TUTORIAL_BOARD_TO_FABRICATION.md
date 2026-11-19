# Von der Platine zu den Fertigungsdateien

Das KiCad-Projekt in `examples/tutorials/kicad/` hat zwei Widerstände auf einer Platine von 30 mm × 20 mm, verbunden durch das Netz `SIG`. Sie gleichen Schaltplan und Platine ab, lassen die Entwurfsregeln laufen und schreiben die Fertigungsdateien mit einem Prüfsummen-Manifest.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Anleitungen aus](README.md).

1. Lassen Sie die elektrische Regelprüfung des Schaltplans laufen. Jeder Befund hat einen Code; bestätigte und nicht schlüssige Befunde stehen getrennt.

```console
$ phoenix erc examples/tutorials/kicad/demo.kicad_sch
```

2. Vergleichen Sie Schaltplan und Platine über die Identität der Bauteile. `sync` ist ein Probelauf: Es listet, was die Übertragung ändern würde, und schreibt nichts, außer Sie ergänzen `--apply --out DATEI`.

```console
$ phoenix sync examples/tutorials/kicad/demo.kicad_sch examples/tutorials/kicad/demo.kicad_pcb
```

3. Lassen Sie die Entwurfsregeln mit den Grenzen einer Regeldatei laufen. Die Leiterbahn ist 0,25 mm breit, die Datei verlangt mindestens 0,2 mm, also gibt es keinen Fehler und der Exit-Code ist 0.

```console
# test
$ phoenix check --rules examples/tutorials/rules_relaxed.json examples/tutorials/kicad/demo.kicad_pcb
Confirmed: 0
Inconclusive: 0
```

4. Schreiben Sie die Fertigungsdateien in einen neuen Ordner: je Lage eine Gerber-X2-Datei, die Bohrdateien, die Bestückungsdatei, die Stückliste und `manifest.json` mit dem SHA-256 jeder Datei. Vor dem Schreiben werden die Dateien zurückgelesen und mit der Platine verglichen.

```console
# test
$ phoenix export --out build/fab examples/tutorials/kicad/demo.kicad_pcb
~ files written: build/fab
```

5. Erstellen Sie das ganze Paket mit Schablone und Bestückung jeder Seite als deterministisches Zip: Dieselbe Platine ergibt dieselben Bytes. Ohne `--out` ist es ein Probelauf, der die Dateien auflistet.

```console
$ phoenix fabpack --out build/demo_pack.zip examples/tutorials/kicad/demo.kicad_pcb
```

6. Sehen Sie sich die Platine vor dem Versand in Zahlen an: Größe, Anzahl der Objekte, Kupfer jeder Lage, Bohrungen und Dichte. Es wird nichts geschrieben.

```console
# test
$ phoenix stats examples/tutorials/kicad/demo.kicad_pcb
Board statistics:
Board: 30.000000 x 20.000000 mm; 600.000000 mm2; Cutouts: 0.000000 mm2
Components: 2 (top: 2; bottom: 0)
```

7. Markieren Sie ein Pad mit einer Operation des Platinen-Editors als Testpunkt (`examples/tutorials/testpoint_ops.json` markiert das zweite Pad von R1 im Netz `MID`) und lassen Sie den Bericht über die Testpunkte und die Netze ausgeben, die keine Sonde erreicht. Die Eingabedatei wird nie überschrieben.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/testpoint_ops.json --out build/with_test_point.json examples/tutorials/tut.brd
~ written: build/with_test_point.json
$ phoenix testpoints build/with_test_point.json
Test points of the board:
Test points: 1 (probe side: both)
Nets asked: 1; covered: 1; coverage: 100.00 %
```

8. Fügen Sie einer Platine mit Dokumentationslage zwei Bemaßungen hinzu (`examples/tutorials/dimension_ops.json`: Breite und Höhe der Platine, mit der Maßlinie 3 mm außerhalb) und schreiben Sie sie in eine DXF-Zeichnung; im Fenster geht das mit dem Bemaßungswerkzeug, und *Bemaßung bearbeiten* verschiebt ihre Punkte. Die Zeichnung ist ein Probelauf, bis `--out` eine neue Datei nennt.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/dimension_ops.json --out build/with_dimensions.json examples/tutorials/notes_board.json
~ written: build/with_dimensions.json
$ phoenix dxf-export board --dimensions build/with_dimensions.json
entities: 21
layer DIMENSIONS: 16
```

9. Fragen Sie den Lieferantenassistenten, wo die Stückliste zu kaufen ist. Er liest eine Preisdatei (`examples/tutorials/prices.csv`, eine Zeile je Lieferant und Mengenstaffel) sowie Lieferanten, Versand und Mindestbestellwert aus einer Einstellungsdatei (`examples/tutorials/sourcing.cfg`) und bepreist die Stückliste bei jedem Lieferanten und als aufgeteilten Einkauf. Die Preise stammen aus der Datei: Es wird nichts aus einem Netzwerk abgerufen.

```console
# test
$ phoenix bom --sourcing --config examples/tutorials/sourcing.cfg --prices examples/tutorials/prices.csv examples/tutorials/kicad/demo.kicad_pcb
~ suppliers: 2 considered, 1 lines, EUR
~ cheapest single supplier: Shop A for 2.210000
~ split purchase: 1 orders for 2.210000
```

## Siehe auch

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
* [docs/STATS.md](../../STATS.md)
* [docs/TESTPOINTS.md](../../TESTPOINTS.md)
* [docs/DIMENSIONS.md](../../DIMENSIONS.md)
* [docs/BOM.md](../../BOM.md)
