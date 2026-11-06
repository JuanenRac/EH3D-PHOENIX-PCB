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

## Siehe auch

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
