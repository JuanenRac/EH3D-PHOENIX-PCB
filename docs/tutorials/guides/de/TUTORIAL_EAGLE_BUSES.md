# Die Busse eines EAGLE-Schaltplans lesen

`examples/guides/bus.sch` ist der Schaltplan der ersten Anleitung mit einem Bus `D[0..1]` auf dem Blatt und zwei Netzen, `D0` und `D1`, deren Leitungen auf der Leitung des Busses enden. Der Leser macht aus dem Namen des Busses ein Busobjekt mit seinen Mitgliedern und setzt einen Eintrag dort, wo jedes Netz den Bus trifft. Ein Bus ist eine Art der Benennung: Seine Mitglieder sind gewöhnliche Netze.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Funktionsanleitungen aus](README.md).

1. Lesen Sie die Hierarchie als JSON. Der Schlüssel `buses` zählt die Busse des Blatts: 1. Die Problemliste enthält keinen Code der Familie `PHOENIX-BUS`, weil beide Mitglieder von einem Eintrag benutzt werden.

```console
$ phoenix hier --json examples/guides/bus.sch
```

2. Dasselbe als Text: Die Netzliste enthält nur Netze, die mindestens zwei Pins verbinden, hier `MID`; die Mitglieder `D0` und `D1` des Busses haben keinen Pin und stehen nicht darin. Die letzte Zeile sagt, ob es Probleme gibt.

```console
$ phoenix hier examples/guides/bus.sch
```

3. Schreiben Sie den Schaltplan als KiCad-Dateien. Der Bus wird als sein Label und eine Linie von seinem Anker durch die Enden seiner Einträge geschrieben, jeder Eintrag als `bus_entry` mit dem Label seines Mitglieds.

```console
$ phoenix kicad-export --out build/bus.kicad_sch schematic examples/guides/bus.sch
```

## So lesen Sie das Ergebnis

* Ein Vektor `Präfix[erster..letzter]` ergibt die Mitglieder `Präfix<erster>` bis `Präfix<letzter>` (`D[0..1]` ergibt `D0` und `D1`; ein Vektor kann abwärts laufen, etwa `A[2..0]`). Der Leser akzeptiert auch die Listen von EAGLE (`A[0..3],B[0..3]`) und einen Namen mit Doppelpunkt (`BUS:A[0..3]`), bei dem der Teil vor dem Doppelpunkt den Bus selbst benennt.
* Der Bus wird dort verankert, wo sein erster Draht beginnt. Ein Eintrag entsteht am Ende jedes Netzes des Blatts, das den Namen eines Mitglieds trägt und auf einem Draht des Busses endet. Die Drähte und Labels des Busses, Namen, die keine Vektoren sind, und ein Bus ohne Draht werden nicht gelesen und als ausgelassene Teile gezählt. Die Codes `PHOENIX-BUS-001` bis `PHOENIX-BUS-007` sagen, was an einem Bus falsch ist, und `phoenix hier` listet sie auf.

## Siehe auch

* [docs/BUSES.md](../../../../docs/BUSES.md)
* [docs/HIERARCHY.md](../../../../docs/HIERARCHY.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
