# Instradare una scheda con l'autorouter integrato, con una coppia e con le via già presenti sulla scheda

`phoenix autoroute native` instrada le connessioni aperte di una scheda con una ricerca propria e non richiede alcun programma esterno. `examples/guides/open.brd` ha una rete i cui due pad non sono uniti; `examples/guides/open_pair.brd` ha le due reti `D_P` e `D_N` di una coppia, anch'esse non unite. Il piano aggiunge solo piste e via, e le regole sono verificate prima e dopo. Si dà il file di regole perché le schede EAGLE degli esempi non hanno regole di progetto proprie.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Pianificate gli instradamenti. È una prova a vuoto: il rapporto elenca le connessioni instradate, le piste, le via e la lunghezza aggiunte e le connessioni ancora aperte, e dice sul flusso di errore che non si è scritto nulla.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json examples/guides/open.brd
```

2. Scrivete la scheda con gli instradamenti in un file nuovo. Il file è il JSON del modello di progetto, qualunque sia il suo nome, e un file esistente non viene mai sostituito.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --out build/open_routed.json examples/guides/open.brd
```

3. Instradate di nuovo la scheda scritta: non le resta alcuna connessione aperta, quindi non c'è nulla da aggiungere.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json build/open_routed.json
```

4. Chiedete la coppia. `pair_dp_dn.json` dice che `D_P` e `D_N` sono una coppia con piste da 0,2 mm e 0,3 mm tra di esse. Se c'è spazio, le due piste si posano affiancate su uno strato e il rapporto dice `coupled pair: p`; altrimenti le due reti sono instradate come reti normali e una nota con il codice `PHOENIX-AROUTE-017` ne dà il motivo.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json examples/guides/open_pair.brd
```

5. Instradate solo le due reti della coppia e rifiutate un piano che lascia una connessione aperta: con `--require-complete` il comando termina con codice di uscita 2 e il codice `PHOENIX-AROUTE-016` se resta qualche connessione.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json --nets D_P,D_N --require-complete examples/guides/open_pair.brd
```

## Come leggere il risultato

* Una via della rete già presente sulla scheda, su un nodo della griglia, è un cambio di strato gratuito: il percorso cambia strato lì e non mette nessuna via nuova. Con `via_snap_nm` maggiore di 0 in `config/autoroute_native.cfg` si usa anche una via che disti al massimo tanto da un nodo, con una nota `PHOENIX-AROUTE-019`. È il riuso delle via; il piano non cambia mai una pista, una via o un pad che c'erano già.
* Una coppia è accoppiata solo quando ogni rete ha esattamente una connessione aperta e i quattro pad condividono uno strato. Le due piste hanno la stessa lunghezza e mantengono la distanza della coppia. Senza `--pairs` né `--pair-config` le reti di una coppia sono instradate come due reti normali e il controllo delle coppie dell'editor di schede giudica il risultato dopo.
* Un prototipo provato su piccole schede fatte a mano e dai test. La ricerca, i suoi costi e i suoi limiti (che contano nodi e mai tempo) sono in `config/autoroute_native.cfg`. Se gli instradamenti di una rete insieme aggiungono un errore di regole, quella rete viene lasciata fuori e segnalata con `PHOENIX-AROUTE-015`: il piano non aggiunge mai un errore che la scheda non aveva.

## Vedere anche

* [docs/AUTOROUTE_NATIVE.md](../../../../docs/AUTOROUTE_NATIVE.md)
* [docs/DIFFPAIRS.md](../../../../docs/DIFFPAIRS.md)
* [docs/AUTOROUTE.md](../../../../docs/AUTOROUTE.md)
