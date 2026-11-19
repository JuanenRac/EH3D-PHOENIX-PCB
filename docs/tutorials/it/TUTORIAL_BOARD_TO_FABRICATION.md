# Dalla scheda ai file di fabbricazione

Il progetto KiCad di `examples/tutorials/kicad/` ha due resistenze su una scheda di 30 mm per 20 mm collegate dalla rete `SIG`. Si confrontano schema e scheda, si eseguono le regole di progetto e si scrivono i file di fabbricazione con un manifesto di checksum.

Eseguite i comandi dalla radice del repository; vedere [come eseguire i tutorial](README.md).

1. Eseguite il controllo delle regole elettriche dello schema. Ogni rilievo ha un codice e quelli confermati sono separati dai non conclusivi.

```console
$ phoenix erc examples/tutorials/kicad/demo.kicad_sch
```

2. Confrontate lo schema con la scheda per identità dei componenti. `sync` è una prova a secco: elenca ciò che il trasferimento cambierebbe e non scrive nulla a meno di aggiungere `--apply --out FILE`.

```console
$ phoenix sync examples/tutorials/kicad/demo.kicad_sch examples/tutorials/kicad/demo.kicad_pcb
```

3. Eseguite le regole di progetto con i limiti di un file di regole. La pista è larga 0,25 mm e il file chiede almeno 0,2 mm: non c'è alcun errore e il codice di uscita è 0.

```console
# test
$ phoenix check --rules examples/tutorials/rules_relaxed.json examples/tutorials/kicad/demo.kicad_pcb
Confirmed: 0
Inconclusive: 0
```

4. Scrivete i file di fabbricazione in una nuova cartella: un Gerber X2 per livello, i file di foratura, quello di posizione, la distinta base e `manifest.json` con lo SHA-256 di ogni file. Prima della scrittura i file vengono riletti e confrontati con la scheda.

```console
# test
$ phoenix export --out build/fab examples/tutorials/kicad/demo.kicad_pcb
~ files written: build/fab
```

5. Create il pacchetto completo, con lo stencil e il posizionamento di ogni lato, in uno zip deterministico: la stessa scheda dà gli stessi byte. Senza `--out` è una prova a secco che elenca i file.

```console
$ phoenix fabpack --out build/demo_pack.zip examples/tutorials/kicad/demo.kicad_pcb
```

6. Guardate la scheda in numeri prima di inviarla: dimensioni, conteggi, rame di ogni strato, forature e densità. Non viene scritto nulla.

```console
# test
$ phoenix stats examples/tutorials/kicad/demo.kicad_pcb
Board statistics:
Board: 30.000000 x 20.000000 mm; 600.000000 mm2; Cutouts: 0.000000 mm2
Components: 2 (top: 2; bottom: 0)
```

7. Segnate un pad come punto di test con un'operazione dell'editor di schede (`examples/tutorials/testpoint_ops.json` segna il secondo pad di R1, sulla rete `MID`) e ottenete il rapporto dei punti di test e delle reti che nessuna sonda raggiunge. Il file di ingresso non viene mai sovrascritto.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/testpoint_ops.json --out build/with_test_point.json examples/tutorials/tut.brd
~ written: build/with_test_point.json
$ phoenix testpoints build/with_test_point.json
Test points of the board:
Test points: 1 (probe side: both)
Nets asked: 1; covered: 1; coverage: 100.00 %
```

8. Aggiungete due quote a una scheda che ha uno strato di documentazione (`examples/tutorials/dimension_ops.json`: la larghezza e l'altezza della scheda, con la linea di quota a 3 mm all'esterno) e scrivetele in un disegno DXF; nella finestra si fa lo stesso con lo strumento delle quote, e *Modifica quota* sposta i loro punti. Il disegno è una prova a secco finché `--out` non indica un file nuovo.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/dimension_ops.json --out build/with_dimensions.json examples/tutorials/notes_board.json
~ written: build/with_dimensions.json
$ phoenix dxf-export board --dimensions build/with_dimensions.json
entities: 21
layer DIMENSIONS: 16
```

9. Chiedete all'assistente fornitori dove acquistare la distinta. Legge un file dei prezzi (`examples/tutorials/prices.csv`, una riga per fornitore e scaglione di quantità) e i fornitori, le spedizioni e l'ordine minimo da un file di impostazioni (`examples/tutorials/sourcing.cfg`), e valuta la distinta presso ogni fornitore e come acquisto suddiviso. I prezzi sono quelli del file: non si consulta alcuna rete.

```console
# test
$ phoenix bom --sourcing --config examples/tutorials/sourcing.cfg --prices examples/tutorials/prices.csv examples/tutorials/kicad/demo.kicad_pcb
~ suppliers: 2 considered, 1 lines, EUR
~ cheapest single supplier: Shop A for 2.210000
~ split purchase: 1 orders for 2.210000
```

## Vedere anche

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
* [docs/STATS.md](../../STATS.md)
* [docs/TESTPOINTS.md](../../TESTPOINTS.md)
* [docs/DIMENSIONS.md](../../DIMENSIONS.md)
* [docs/BOM.md](../../BOM.md)
