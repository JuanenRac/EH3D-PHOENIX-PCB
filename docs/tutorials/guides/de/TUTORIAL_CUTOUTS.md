# Die Aussparungen der Platinenkontur und die Ausschnitte einer Kupferfläche durch die Dateiformate führen

`examples/guides/cutout.brd` ist eine EAGLE-Platine mit zwei Arten von Öffnungen: ein geschlossenes Quadrat aus Linien auf Lage 20 innerhalb der Kontur (eine Aussparung der Platine, ein Schlitz oder Fenster) und ein Polygon auf Lage 41 innerhalb der Kupferfläche der Lage `Top` (ein Ausschnitt der Fläche, wo kein Kupfer sein darf). Sie schreiben die Platine als DXF, lesen die DXF-Datei zurück, schreiben KiCad- und Gerber-Dateien und sehen, wohin jede Öffnung gelangt.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Funktionsanleitungen aus](README.md).

1. Listen Sie auf, was eine DXF-Zeichnung der Platine enthielte. Kontur und Aussparung sind zwei geschlossene Konturen auf der Ebene `OUTLINE`; der Bericht nennt `layer OUTLINE: 2` und dass nichts geschrieben wurde.

```console
# test
$ phoenix dxf-export board examples/guides/cutout.brd
~ layer OUTLINE: 2
~ dry run: nothing was written
```

2. Schreiben Sie die Zeichnung in eine neue Datei.

```console
# test
$ phoenix dxf-export board examples/guides/cutout.brd --out build/cutout.dxf
~ written: build/cutout.dxf
```

3. Lesen Sie die Zeichnung in die Platine zurück. Die größte Außenkontur ist die Platinenkontur, die darin liegende Kontur bleibt als Aussparung der Kontur erhalten (`PHOENIX-DXF-015`, Information). Die Operation ist ein Probelauf, vor und nach der Ausführung mit den Entwurfsregeln geprüft, daher wird die Regeldatei angegeben.

```console
$ phoenix dxf-import build/cutout.dxf --board examples/guides/cutout.brd --layer OUTLINE --rules examples/tutorials/rules_relaxed.json
```

4. Schreiben Sie eine KiCad-Platine. Kontur und Aussparung kommen auf `Edge.Cuts`, und der Ausschnitt der Fläche wird zu einem Regelbereich auf seiner Lage, der die Fläche fernhält und Bahnen, Durchkontaktierungen und Pads erlaubt. Der Bericht listet, was geschrieben wurde und was das Format nicht ausdrücken kann.

```console
$ phoenix kicad-export --out build/cutout.kicad_pcb board examples/guides/cutout.brd
```

5. Schreiben Sie die Fertigungsdateien. Die Profildatei enthält die Kontur und dann die Aussparung, jeweils eine geschlossene Kontur, und in der Kupferdatei wird der Ausschnitt der Fläche nach der Fläche als Bereich mit heller Polarität geschrieben.

```console
$ phoenix export --out build/cutout_fab examples/guides/cutout.brd
```

## So lesen Sie das Ergebnis

* Woher eine Öffnung kommt: In einer EAGLE-Platine sind Linien der Lage 20, die aufeinander folgen und sich schließen, innerhalb des größten solchen Rings Aussparungen der Platine; ein Polygon auf Lage 41 (`tRestrict`, oben) oder 42 (unten) innerhalb einer Fläche dieser Lage ist ein Ausschnitt der Fläche. In einer KiCad-Platine sind die geschlossenen Konturen auf `Edge.Cuts` innerhalb der größten die Aussparungen, in einer DXF-Datei die Konturen innerhalb der größten Außenkontur.
* Jedes Format sagt, was es kann. Die Ausschnitte einer Fläche sind in KiCad Regelbereiche, in EAGLE Sperrflächen und in der Specctra-Datei des Autorouters Sperrbereiche (sie hat kein Loch in einer Ebene, daher werden auch Bahnen und Durchkontaktierungen ferngehalten). Der Nutzen trägt beide Arten von Öffnungen in jede Kopie, mit ihr verschoben, gedreht und gespiegelt.

## Siehe auch

* [docs/DXF.md](../../../../docs/DXF.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/PANEL.md](../../../../docs/PANEL.md)
