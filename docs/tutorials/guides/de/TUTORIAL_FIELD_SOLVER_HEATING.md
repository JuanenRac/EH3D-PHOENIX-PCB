# Die Impedanz einer Leitung mit dem Feldlöser berechnen und abschätzen, wie stark sich eine Leiterbahn erwärmt

`examples/guides/stackup_job.json` beschreibt einen zweilagigen Lagenaufbau mit einer Leitung. Sie lösen den Querschnitt mit dem 2D-Feldlöser und vergleichen das Ergebnis mit den geschlossenen Formeln, entnehmen dann die Leitung einer Platine und schätzen schließlich die Temperaturerhöhung der Leiterbahn eines Netzes für zwei Ströme ab. Beide Werkzeuge sind quasistatische Prototypen, die weder mit einem anderen Löser noch mit einer Messung verglichen wurden.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Funktionsanleitungen aus](README.md).

1. Lösen Sie die Leitung des Auftrags. Der Bericht nennt das Netz, die Iterationen und das Residuum, dann Impedanz, effektive Permittivität, Induktivität, Kapazität, Laufzeit und dielektrischen Verlust und daneben die geschlossene Formel mit der Abweichung in Prozent. Es ist ein Probelauf: Es wird nichts geschrieben.

```console
$ phoenix field2d examples/guides/stackup_job.json
```

2. Verfeinern Sie das Netz um den Faktor 2, um zu sehen, wie stark sich die Antwort bewegt; ein Ergebnis, das sich wenig ändert, ist konvergiert. Eine Lösung, die die Toleranz nicht erreicht, ergibt `PHOENIX-FIELD-004` und keine Zahl.

```console
$ phoenix field2d examples/guides/stackup_job.json --refine 2
```

3. Entnehmen Sie die Leitung einer Platine. Lage und Breite, auf der der größte Teil des Kupfers des Netzes `MID` verläuft (eine Bahn von 0,2 mm auf `Top`), ergeben eine Mikrostreifenleitung, die auf dem Lagenaufbau des Auftrags gelöst wird.

```console
$ phoenix field2d examples/tutorials/tut.brd --stackup examples/guides/stackup_job.json --net MID
```

4. Fügen Sie den Feldlöser dem Bericht von `stackup` hinzu, damit die geschlossene Formel und die Feldlösung in einem Bericht stehen.

```console
$ phoenix stackup examples/guides/stackup_job.json --freq-ghz 1 --field2d
```

5. Schätzen Sie die Erwärmung des Netzes `MID` bei 0,5 A ab. Der Bericht führt die Bahn nach Lage, Breite und Kupferdicke auf und nennt den Anstieg nach der Anpassung von IPC-2221 und nach dem Konvektionsmodell. Der Anstieg liegt unter der Grenze der Einstellungen, der Exitcode ist 0.

```console
$ phoenix thermal --currents examples/guides/currents_low.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

6. Dasselbe Netz mit 3 A. Der Anstieg liegt über der Grenze, die Bahn wird mit `[above the limit]` und der Warnung `PHOENIX-HEAT-004` markiert, der Exitcode ist 1.

```console
$ phoenix thermal --currents examples/guides/currents_high.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

## So lesen Sie das Ergebnis

* Der Löser nimmt die Leitung als quasi-TEM auf einem rechteckigen Netz in einer Box, deren Wände auf Nullpotential liegen; das macht die Impedanz etwas niedriger als bei offenem Raum. Die Leiter sind Rechtecke, Lötstopplack und Kupferverlust fehlen. Eine Abweichung von der geschlossenen Formel über `compare_warn_percent` ist die Warnung `PHOENIX-FIELD-009`.
* Die Erwärmungsschätzung führt den gesamten Strom des Netzes durch jede Bahn und jede Durchkontaktierung; ein Netz, das seinen Strom aufteilt, wird also auf der sicheren Seite beurteilt. IPC-2152 wird nicht angewendet, und jeder Bericht sagt das (`PHOENIX-HEAT-012`). Eine Lage, die der Lagenaufbau nicht nennt, erhält die Standardwerte der Einstellungen (`PHOENIX-HEAT-008`).
* Beide Befehle sind Probeläufe und schreiben einen JSON-Bericht nur in eine neue, mit `--out` genannte Datei. Die Konstanten und Grenzen stehen in `config/field2d.cfg` und `config/heating.cfg`, nicht im Code. Der Exitcode ist 0 bei Erfolg, 1, wenn eine Leitung nicht gelöst werden konnte oder eine Bahn über der Grenze liegt, und 2 bei unbrauchbaren Argumenten oder Eingaben.

## Siehe auch

* [docs/FIELD2D.md](../../../../docs/FIELD2D.md)
* [docs/THERMAL.md](../../../../docs/THERMAL.md)
* [docs/STACKUP.md](../../../../docs/STACKUP.md)
