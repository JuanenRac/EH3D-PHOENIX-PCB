# Die Signalintegrität eines Differenzialpaars prüfen

Die Platine `examples/tutorials/diffpair.kicad_pcb` hat zwei Leiterbahnen von 0,25 mm, `D_P` und `D_N`, 20 mm lang nebeneinander mit 0,75 mm zwischen den Kanten. Sie prüfen das Paar gegen eine Datei, die sagt, wie das Paar sein soll, sehen den Befund bei falschem Abstand, lesen ihn auf Spanisch und schätzen das Übersprechen. Die Werkzeuge zur Signalintegrität sind quasistatische Schätzungen aus geschlossenen Formeln, kein Feldlöser.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Anleitungen aus](README.md).

1. Die Datei `pairs.json` sagt, dass das Paar `D_P` und `D_N` ist, mit Bahnen von 0,25 mm und einem Abstand von 0,75 mm. Die Platine stimmt überein: kein Fehler und keine Warnung.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs.json examples/tutorials/diffpair.kicad_pcb
~ pairs: 1, errors: 0, warnings: 0
~ pair p (D_P / D_N)
```

2. Jetzt verlangt die Datei einen Abstand von 0,3 mm, die Platine hat 0,75 mm: Der Abstand liegt außerhalb der Toleranz, der Code ist `PHOENIX-PAIR-003`, der Exit-Code 1.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ PHOENIX-PAIR-003
```

3. Derselbe Bericht auf Spanisch. `--lang` akzeptiert en, es, de, fr, it, ja, zh und pt.

```console
# test
$ phoenix pairs --lang es --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ La separación del par está fuera de tolerancia
```

4. Schätzen Sie das Übersprechen zwischen den beiden Bahnen aus der Länge des Parallellaufs, dem Mittenabstand und dem Lagenaufbau. Die Schätzung steht in Prozent des Sprungs des Störers.

```console
# test
$ phoenix si crosstalk examples/tutorials/diffpair.kicad_pcb
~ crosstalk: 1 coupled pairs of nets
```

5. Die übrigen Analysen: Reflexionen und Abschlüsse eines Netzes, ein vereinfachtes Augendiagramm und alle zusammen mit dem Rückweg (Exit-Code 1, wenn eine Schwelle aus `config/si.cfg` überschritten wird).

```console
$ phoenix si reflect --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si eye --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si check examples/tutorials/diffpair.kicad_pcb
```

## Siehe auch

* [docs/DIFFPAIRS.md](../../DIFFPAIRS.md)
* [docs/SI.md](../../SI.md)
* [docs/LENGTHS.md](../../LENGTHS.md)
* [docs/STACKUP.md](../../STACKUP.md)
