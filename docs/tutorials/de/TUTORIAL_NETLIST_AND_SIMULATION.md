# Vom Schaltplan zur Netzliste und zur Simulation

Die Datei `examples/tutorials/divider.json` ist ein Schaltplan mit einer 10-V-Quelle und zwei in Reihe liegenden Widerständen von 1 kΩ und 3 kΩ. Sie erstellen die Simulations-Netzliste, ohne sie von Hand zu schreiben, simulieren sie und prüfen das Ergebnis mit der Teilerformel: 10 V × 3 / (1 + 3) = 7,5 V.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Anleitungen aus](README.md).

1. Geben Sie die Netzliste des Schaltplans aus. Es wird nichts geschrieben (Probelauf), und vorher läuft die elektrische Regelprüfung. Die erste Zeile ist der Titel, danach folgen die Elemente mit den Werten in der Schreibweise des Simulators.

```console
# test
$ phoenix netlist examples/tutorials/divider.json
divider
~ R1 in out 1e3
~ R2 out 0 3e3
~ V1 in 0 1e1
```

2. Schreiben Sie sie in eine neue Datei. Eine vorhandene Datei wird nie überschrieben.

```console
# test
$ phoenix netlist examples/tutorials/divider.json --out build/divider.cir
~ written:
```

3. Berechnen Sie den Arbeitspunkt. Die Teilerformel ergibt 7,5 V am Knoten `out`.

```console
# test
$ phoenix sim build/divider.cir --op --probe "v(out)"
name,value
v(out),7.5e0
```

4. Ohne Datei: `--schematic` erstellt die Netzliste aus dem Schaltplan und übergibt sie direkt an den Löser.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --op --probe "v(out)"
name,value
v(out),7.5e0
```

5. Fahren Sie die Quelle von 0 V bis 10 V in 5-V-Schritten: Der Ausgang ist immer drei Viertel des Eingangs.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --dc V1 0 10 5 --probe "v(out)"
~ 5e0,3.75e0
~ 1e1,7.5e0
```

## Siehe auch

* [docs/SIM.md](../../SIM.md)
* [docs/NETLIST.md](../../NETLIST.md)
* [docs/ERC.md](../../ERC.md)
