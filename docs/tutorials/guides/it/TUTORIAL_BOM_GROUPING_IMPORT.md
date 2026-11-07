# Raggruppare ed esportare la distinta base e importare quella di un altro programma

La scheda `examples/guides/bom_four.brd` ha tre resistenze da 10 k e una da 4k7. Si sceglie nel file delle impostazioni come si raggruppano i componenti e quali colonne ha il file CSV, poi si legge una distinta scritta da un altro programma con una mappa delle sue colonne. Nulla viene scaricato da una rete e la tariffa delle impostazioni è solo un esempio della forma di un preventivo.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Elencate la distinta con il raggruppamento predefinito (valore, impronta, produttore e codice) e i dati di `bom_parts.csv`. R1 e R2 condividono produttore e codice, R3 ha un altro produttore: le righe sono tre. Non c'è un file dei prezzi, quindi ogni riga è segnalata con `PHOENIX-BOM-005`.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

2. La stessa scheda con `group_by = value,footprint` in `bom_costs_joined.cfg`: produttore e codice escono dal raggruppamento, quindi R1, R2 e R3 diventano una riga di quantità 3 che non ha né l'uno né l'altro.

```console
$ phoenix bom --config examples/guides/bom_costs_joined.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

3. Scrivete il file CSV. Le sue colonne sono le `csv_columns` delle impostazioni, in quell'ordine (qui `references,quantity,value,footprint,manufacturer,mpn,status`); la pagina HTML e il file JSON mantengono tutte le colonne. Un file esistente non viene mai sostituito.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv --csv build/bom.csv examples/guides/bom_four.brd
```

4. Importate la distinta di un altro programma. `bom_other_program.csv` usa il punto e virgola e nomi di colonna propri; `bom_import_map.cfg` dice come leggerla. Il rapporto confronta la distinta con la scheda: R5 è nella distinta e non sulla scheda (`PHOENIX-BOM-021`), e R4 è segnato come non montato, quindi è escluso dalla distinta e mostrato come informazione (`PHOENIX-BOM-009`). `--parts-out` salva ciò che la distinta dice su produttori e codici come file di dati dei componenti.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --import examples/guides/bom_other_program.csv --import-map examples/guides/bom_import_map.cfg --parts-out build/parts_from_bill.csv examples/guides/bom_four.brd
```

5. Usate il file di dati dei componenti salvato nelle esecuzioni successive al posto della distinta originale.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts build/parts_from_bill.csv examples/guides/bom_four.brd
```

## Come leggere il risultato

* Il file delle impostazioni predefinito è `config/costs.cfg` e la mappa predefinita è `config/bom_import.cfg`, se esistono. Una mappa indica il delimitatore (virgola, punto e virgola o tabulazione) e, per ogni colonna, i nomi che può avere nel file; solo i riferimenti sono obbligatori. I nomi forniti sono una supposizione e non sono il formato di alcun programma: copiate il file e scrivete i nomi del vostro file.
* I codici `PHOENIX-BOM-019`, `PHOENIX-BOM-020` e `PHOENIX-BOM-021` sono avvisi e non cambiano nulla: una quantità che non è il numero di riferimenti della sua riga, un riferimento in due righe (si usa la prima) e un componente che ha solo un lato o il cui valore differisce. Un intervallo come `R1-R5` viene espanso nei suoi singoli componenti; un intervallo malformato, invertito o più lungo di `max_range` ferma l'importazione (da `PHOENIX-BOM-022` a `PHOENIX-BOM-024`). `--import-format` legge anche una lista componenti di EAGLE o una distinta di KiCad (`docs/BOM.md`). Ciò che dice `--parts` prevale sulla distinta importata.
* Senza `--csv`, `--json-file` e `--html` il comando non scrive nulla. Il codice di uscita è 0 se tutto va bene, 1 se un componente nomina un'impronta assente dalla scheda e 2 se gli argomenti o l'ingresso non sono utilizzabili.

## Vedere anche

* [docs/BOM.md](../../../../docs/BOM.md)
* [docs/PARTS.md](../../../../docs/PARTS.md)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
