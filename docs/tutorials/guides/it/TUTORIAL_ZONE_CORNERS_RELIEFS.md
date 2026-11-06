# Arrotondare gli angoli di un riempimento di rame e scrivere sfoghi termici e limiti delle isole nei file Gerber

La scheda `examples/guides/pour.brd` ha un riempimento di massa sullo strato `Top` attorno a due resistenze e a una pista. Lo si riempie con angoli vivi e con angoli arrotondati e si confronta, poi si scrivono i file Gerber con un profilo che dà uno sfogo termico ai pad rettangolari del riempimento e lascia fuori le sue isole piccole. Si dà il file di regole `rules_relaxed.json` perché una scheda EAGLE senza regole di progetto non ha limiti propri.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Riempite la zona con le impostazioni del repository, che lasciano vivi tutti gli angoli (`corner_radius_nm = 0`). Il rapporto dà i poligoni e i nanometri quadrati di rame.

```console
$ phoenix zones fill --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

2. Riempitela di nuovo con `zones_round.cfg`, che imposta `corner_radius_nm = 1000000` (un arco di 1 mm). Si arrotondano gli angoli del riempimento rivolti verso l'esterno e gli angoli dei vuoti dello sfogo termico; confrontate l'area di rame con la prima esecuzione.

```console
$ phoenix zones fill --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

3. Misurate il riempimento arrotondato rispetto alla scheda e alle regole. L'arrotondamento toglie rame al riempimento o lo aggiunge solo dentro un vuoto di uno sfogo termico, quindi il riempimento non si avvicina al rame di un'altra rete e il controllo non trova errori di distanza.

```console
$ phoenix zones check --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

4. Scrivete prima i file Gerber con il profilo generico. Il pad `R2.2` della rete di massa è dentro il riempimento e resta unito a esso senza sfogo.

```console
$ phoenix export --out build/pour_plain examples/guides/pour.brd
```

5. Ora con il profilo `pour` di `export_pour.json`: quattro bracci (`gerber.zone_thermal_spokes`), un vuoto e un braccio da 0,25 mm, lo stesso sfogo per i pad rettangolari e oblunghi (`gerber.zone_thermal_non_round`) e nessuna isola sotto 1 millimetro quadrato (`gerber.zone_island_min_area_nm2`). Confrontate il file di rame dello strato `Top` nelle due cartelle: il secondo ha i vuoti dello sfogo scritti come regioni a polarità chiara attorno al pad.

```console
$ phoenix export --profiles examples/guides/export_pour.json --profile pour --out build/pour_relief examples/guides/pour.brd
```

## Come leggere il risultato

* Nelle impostazioni del riempimento, `corner_radius_nm` è il raggio dell'arco (zero lascia vivi tutti gli angoli) e `corner_tolerance_nm` quanto le corde dell'arco possono stare lontane da esso. Un arco prende al massimo metà di ogni lato accanto all'angolo, quindi un raggio che non ci sta viene ridotto. Gli angoli dei buchi che altro rame lascia nel riempimento non si arrotondano.
* Nell'esportazione uno sfogo ha quattro bracci o due (`gerber.zone_thermal_spokes` è 0, 2 o 4; 0 lascia i pad uniti in pieno). Un pad ruotato di un angolo non retto resta pieno e lo dice (`PHOENIX-EXPORT-022`). Un'isola più piccola di `gerber.zone_island_min_area_nm2` non viene scritta e il rapporto lo dice (`PHOENIX-EXPORT-021`); zero scrive tutte le zone.

## Vedere anche

* [docs/ZONES.md](../../../../docs/ZONES.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
