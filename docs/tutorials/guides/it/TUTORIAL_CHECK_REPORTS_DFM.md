# Scrivere i riscontri di un controllo come tabella e come pagina, e controllare una scheda con il profilo di un produttore

Si eseguono le regole di progetto, si conservano i riscontri come tabella CSV e come pagina HTML raggruppata per categoria, poi si controlla una piccola scheda EAGLE con un profilo di capacità di un produttore. Il profilo dell'esempio è inventato: mostra la forma del file e non è la capacità di alcun produttore.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Eseguite le regole di un file di regole e scrivete i due rapporti. Ogni opzione scrive un file nuovo: uno esistente non viene mai sostituito e non si scrive nulla se una delle uscite è rifiutata (codice di uscita 2). La pista della scheda è più sottile di quanto chiede il file: un riscontro confermato e il codice di uscita è 1.

```console
$ phoenix check --rules examples/tutorials/rules_strict.json --csv build/check.csv --html build/check.html examples/tutorials/kicad/demo.kicad_pcb
```

2. Scrivete gli stessi rapporti in un'altra lingua. La colonna del titolo del CSV e le intestazioni della pagina seguono `--lang`; le chiavi delle colonne, delle categorie e delle gravità restano in inglese perché uno script le legga in ogni lingua.

```console
$ phoenix check --lang es --rules examples/tutorials/rules_strict.json --csv build/check_es.csv --html build/check_es.html examples/tutorials/kicad/demo.kicad_pcb
```

3. Il confronto di uno schema e di una scheda accetta le stesse due opzioni.

```console
$ phoenix connectivity --csv build/connectivity.csv --html build/connectivity.html examples/tutorials/tut.sch examples/tutorials/tut.brd
```

4. Controllate la scheda EAGLE con il profilo `examples/guides/dfm_small_shop.cfg`. Con `--dfm` i limiti del profilo sostituiscono le regole della scheda, quindi non si dà `--rules`. La scheda misura 30 mm per 20 mm e il profilo ammette 10 mm sul lato corto: un errore con il codice `PHOENIX-DFM-003` e codice di uscita 1. Una scheda con più di uno strato di rame riceve anche `PHOENIX-DFM-001`, e l'informazione `PHOENIX-DFM-006` avverte che il profilo non è verificato.

```console
$ phoenix check --dfm examples/guides/dfm_small_shop.cfg --csv build/dfm.csv --html build/dfm.html examples/tutorials/tut.brd
```

5. Ora il profilo di esempio del repository, `config/dfm/example.cfg`. La scheda è entro tutti i suoi limiti, quindi l'unico riscontro della categoria capacità è l'informazione che il profilo non è verificato.

```console
$ phoenix check --dfm config/dfm/example.cfg --html build/dfm_example.html examples/tutorials/tut.brd
```

## Come leggere il risultato

* Il CSV ha l'intestazione `list,category,severity,code,title,location,x_nm,y_nm,measured_nm,limit_nm,detail` e una riga per riscontro, prima quelli provati (`list` è `confirmed` o `inconclusive`). Una cella di testo che un foglio di calcolo eseguirebbe come formula riceve un apice davanti.
* La pagina HTML non ha script né file esterni e protegge ogni testo che viene dalla scheda, quindi si apre senza rischi in un browser. Comincia con il conteggio dei riscontri provati e inconcludenti di ogni categoria e prosegue con una tabella per ogni categoria che ha riscontri; quelli del profilo sono nella categoria `capability`.
* Il profilo è un file di testo con righe `chiave = valore` in nanometri; una riga mancante o a 0 è disattivata. Copiate `config/dfm/example.cfg`, inseritevi le cifre lette nella documentazione del vostro produttore, annotate in `source` dove le avete lette e solo allora impostate `verified = true`. Nulla controlla il file rispetto a un produttore.

## Vedere anche

* [docs/DRC_ADVANCED.md](../../../../docs/DRC_ADVANCED.md)
* [docs/DFM.md](../../../../docs/DFM.md)
* [docs/RULES.md](../../../../docs/RULES.md)
