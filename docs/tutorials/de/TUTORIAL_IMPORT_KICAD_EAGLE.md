# Ein KiCad- oder EAGLE-Projekt importieren und prüfen

Sie lesen ein ganzes KiCad-Projekt in das Entwurfsmodell des Programms ein, sehen, was erhalten blieb und was nicht, schreiben das Modell als neue Dateien und prüfen die Platine mit einer Regeldatei. Danach prüfen Sie ebenso eine EAGLE-Platine.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Anleitungen aus](README.md).

1. Lesen Sie das Projekt (das `.kicad_pro` mit dem gleichnamigen `.kicad_sch` und `.kicad_pcb`). Es ist ein Probelauf: Der Bericht sagt, was gelesen wurde, wie viel erhalten blieb und was das Modell nicht enthält.

```console
# test
$ phoenix kicad-import examples/tutorials/kicad/demo.kicad_pro
~ usable: yes
~ components: 2
~ tracks: 1
~ vias: 1
~ dry run: nothing was written
```

2. Schreiben Sie das Modell als neue Dateien mit dem angegebenen Namen. Eine vorhandene Datei oder eine Eingabe wird nie überschrieben.

```console
# test
$ phoenix kicad-import --out build/demo.json examples/tutorials/kicad/demo.kicad_pro
~ written:
```

3. Prüfen Sie die importierte Platine. Eine KiCad-Platine liefert dem Programm keine Entwurfsregeln, die Grenzen kommen daher aus einer Regeldatei. Die Datei verlangt Leiterbahnen von mindestens 0,3 mm, die Bahn der Platine hat 0,25 mm: ein bestätigter Befund und Exit-Code 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
Confirmed: 1
~ PHOENIX-CHECK-001
```

4. Eine EAGLE-Datei: Fragen Sie, was ein Neuschreiben erhält. Der Bericht beweist, dass das Schreiben des XML-Baums und erneutes Lesen nichts ändert, und listet, was das Entwurfsmodell vereinfacht oder auslässt.

```console
# test
$ phoenix eagle-check examples/tutorials/tut.brd
board (version 9.6.2)
~ rewrite of the tree: identical
```

5. Lassen Sie dieselbe Regeldatei auf der EAGLE-Platine laufen. Die Bahn des Netzes `MID` ist 0,2 mm breit.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/tut.brd
# exit 1
Confirmed: 1
```

6. Vergleichen Sie den EAGLE-Schaltplan mit der EAGLE-Platine: Der Bericht listet fehlende Netze, nicht verbundene Pins und noch nicht verlegtes Kupfer auf.

```console
$ phoenix connectivity examples/tutorials/tut.sch examples/tutorials/tut.brd
```

## Siehe auch

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD.md](../../KICAD.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/RULES.md](../../RULES.md)
