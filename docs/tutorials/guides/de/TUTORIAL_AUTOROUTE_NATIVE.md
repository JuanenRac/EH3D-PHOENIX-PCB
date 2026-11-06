# Eine Platine mit dem eingebauten Autorouter verdrahten, mit einem Paar und mit den schon vorhandenen Durchkontaktierungen

`phoenix autoroute native` verdrahtet die offenen Verbindungen einer Platine mit einer eigenen Suche und braucht kein externes Programm. `examples/guides/open.brd` hat ein Netz, dessen zwei Pads nicht verbunden sind; `examples/guides/open_pair.brd` hat die beiden Netze `D_P` und `D_N` eines Paars, ebenfalls unverbunden. Der Plan fügt nur Bahnen und Durchkontaktierungen hinzu, und die Regeln werden davor und danach geprüft. Die Regeldatei wird angegeben, weil die EAGLE-Platinen der Beispiele keine eigenen Entwurfsregeln haben.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Funktionsanleitungen aus](README.md).

1. Planen Sie die Verdrahtung. Es ist ein Probelauf: Der Bericht listet die verdrahteten Verbindungen, die hinzugefügten Bahnen, Durchkontaktierungen und Länge sowie die noch offenen Verbindungen und meldet im Fehlerstrom, dass nichts geschrieben wurde.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json examples/guides/open.brd
```

2. Schreiben Sie die Platine mit den Verdrahtungen in eine neue Datei. Die Datei ist das JSON des Entwurfsmodells, gleich wie sie heißt, und eine vorhandene Datei wird nie ersetzt.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --out build/open_routed.json examples/guides/open.brd
```

3. Verdrahten Sie die geschriebene Platine erneut: Es bleibt keine offene Verbindung, also gibt es nichts hinzuzufügen.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json build/open_routed.json
```

4. Fordern Sie das Paar an. `pair_dp_dn.json` sagt, dass `D_P` und `D_N` ein Paar mit Bahnen von 0,2 mm und 0,3 mm Abstand sind. Ist Platz, werden die beiden Bahnen nebeneinander auf einer Lage gelegt, und der Bericht nennt `coupled pair: p`; sonst werden beide Netze als gewöhnliche Netze verdrahtet, und ein Hinweis mit dem Code `PHOENIX-AROUTE-017` nennt den Grund.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json examples/guides/open_pair.brd
```

5. Verdrahten Sie nur die beiden Netze des Paars und lehnen Sie einen Plan ab, der eine Verbindung offen lässt: Mit `--require-complete` endet der Befehl mit Exitcode 2 und dem Code `PHOENIX-AROUTE-016`, wenn eine Verbindung übrig bleibt.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json --nets D_P,D_N --require-complete examples/guides/open_pair.brd
```

## So lesen Sie das Ergebnis

* Eine bereits auf der Platine vorhandene Durchkontaktierung des Netzes auf einem Knoten des Rasters ist ein kostenloser Lagenwechsel: Die Bahn wechselt dort die Lage und setzt keine neue Durchkontaktierung. Mit `via_snap_nm` über 0 in `config/autoroute_native.cfg` wird auch eine Durchkontaktierung genutzt, die höchstens so weit von einem Knoten entfernt ist, mit dem Hinweis `PHOENIX-AROUTE-019`. Das ist die Wiederverwendung von Durchkontaktierungen; der Plan ändert nie eine vorhandene Bahn, Durchkontaktierung oder ein Pad.
* Ein Paar wird nur gekoppelt, wenn jedes Netz genau eine offene Verbindung hat und die vier Pads eine Lage teilen. Die beiden Bahnen sind gleich lang und halten den Abstand des Paars. Ohne `--pairs` oder `--pair-config` werden die Netze eines Paars als zwei gewöhnliche Netze verdrahtet, und die Paarprüfung des Platineneditors beurteilt das Ergebnis danach.
* Ein Prototyp, erprobt an kleinen, von Hand gemachten Platinen und mit den Tests. Die Suche, ihre Kosten und ihre Grenzen (die Knoten zählen, nie Zeit) stehen in `config/autoroute_native.cfg`. Fügen die Verdrahtungen eines Netzes zusammen einen Regelfehler hinzu, wird dieses Netz ausgelassen und mit `PHOENIX-AROUTE-015` gemeldet: Der Plan fügt nie einen Fehler hinzu, den die Platine nicht hatte.

## Siehe auch

* [docs/AUTOROUTE_NATIVE.md](../../../../docs/AUTOROUTE_NATIVE.md)
* [docs/DIFFPAIRS.md](../../../../docs/DIFFPAIRS.md)
* [docs/AUTOROUTE.md](../../../../docs/AUTOROUTE.md)
