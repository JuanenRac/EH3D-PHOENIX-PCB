# Die Befunde einer Prüfung als Tabelle und als Seite schreiben und eine Platine gegen das Profil eines Herstellers prüfen

Sie führen die Entwurfsregeln aus, speichern die Befunde als CSV-Tabelle und als nach Kategorien gegliederte HTML-Seite und prüfen dann eine kleine EAGLE-Platine gegen ein Fähigkeitsprofil eines Herstellers. Das Profil des Beispiels ist erfunden: Es zeigt die Form der Datei und ist nicht die Fähigkeit eines Herstellers.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Funktionsanleitungen aus](README.md).

1. Führen Sie die Regeln einer Regeldatei aus und schreiben Sie beide Berichte. Jede Option schreibt eine neue Datei: Eine vorhandene wird nie ersetzt, und es wird nichts geschrieben, wenn eine der Ausgaben abgelehnt wird (Exitcode 2). Die Leiterbahn ist dünner als verlangt: ein bestätigter Befund, Exitcode 1.

```console
$ phoenix check --rules examples/tutorials/rules_strict.json --csv build/check.csv --html build/check.html examples/tutorials/kicad/demo.kicad_pcb
```

2. Schreiben Sie dieselben Berichte in einer anderen Sprache. Die Titelspalte der CSV-Datei und die Überschriften der Seite folgen `--lang`; die Schlüssel der Spalten, Kategorien und Schweregrade bleiben englisch, damit ein Skript sie in jeder Sprache lesen kann.

```console
$ phoenix check --lang es --rules examples/tutorials/rules_strict.json --csv build/check_es.csv --html build/check_es.html examples/tutorials/kicad/demo.kicad_pcb
```

3. Der Vergleich von Schaltplan und Platine nimmt dieselben beiden Optionen.

```console
$ phoenix connectivity --csv build/connectivity.csv --html build/connectivity.html examples/tutorials/tut.sch examples/tutorials/tut.brd
```

4. Prüfen Sie die EAGLE-Platine gegen das Profil `examples/guides/dfm_small_shop.cfg`. Mit `--dfm` ersetzen die Grenzen des Profils die Regeln der Platine, daher wird `--rules` nicht angegeben. Die Platine misst 30 mm × 20 mm, das Profil erlaubt 10 mm an der kurzen Seite: ein Fehler mit dem Code `PHOENIX-DFM-003` und Exitcode 1. Eine Platine mit mehr als einer Kupferlage erhält zusätzlich `PHOENIX-DFM-001`, und die Information `PHOENIX-DFM-006` weist darauf hin, dass das Profil nicht verifiziert ist.

```console
$ phoenix check --dfm examples/guides/dfm_small_shop.cfg --csv build/dfm.csv --html build/dfm.html examples/tutorials/tut.brd
```

5. Nun das Beispielprofil des Repositorys, `config/dfm/example.cfg`. Die Platine liegt innerhalb aller seiner Grenzen, daher ist der einzige Befund der Kategorie Fähigkeit die Information, dass das Profil nicht verifiziert ist.

```console
$ phoenix check --dfm config/dfm/example.cfg --html build/dfm_example.html examples/tutorials/tut.brd
```

## So lesen Sie das Ergebnis

* Die CSV-Datei hat die Kopfzeile `list,category,severity,code,title,location,x_nm,y_nm,measured_nm,limit_nm,detail` und eine Zeile je Befund, die bewiesenen zuerst (`list` ist `confirmed` oder `inconclusive`). Eine Textzelle, die eine Tabellenkalkulation als Formel ausführen würde, erhält ein Anführungszeichen davor.
* Die HTML-Seite enthält kein Skript und keine externe Datei und maskiert jeden Text aus der Platine, sie lässt sich also gefahrlos im Browser öffnen. Sie beginnt mit der Zahl der bewiesenen und unentschiedenen Befunde je Kategorie und enthält dann eine Tabelle für jede Kategorie mit Befunden; die Befunde des Profils stehen in der Kategorie `capability`.
* Das Profil ist eine Textdatei mit Zeilen `Schlüssel = Wert` in Nanometern; eine fehlende Zeile oder 0 bedeutet aus. Kopieren Sie `config/dfm/example.cfg`, tragen Sie die Zahlen aus der Dokumentation Ihres Herstellers ein, notieren Sie in `source`, wo Sie sie gelesen haben, und setzen Sie erst dann `verified = true`. Nichts prüft die Datei gegen einen Hersteller.

## Siehe auch

* [docs/DRC_ADVANCED.md](../../../../docs/DRC_ADVANCED.md)
* [docs/DFM.md](../../../../docs/DFM.md)
* [docs/RULES.md](../../../../docs/RULES.md)
