# Risolvere l'impedenza di una linea con il risolutore di campo e stimare quanto si scalda una pista

`examples/guides/stackup_job.json` descrive uno stackup a due strati con una linea. Si risolve la sezione con il risolutore di campo 2D e si confronta il risultato con le formule chiuse, poi si prende la linea da una scheda e infine si stima l'aumento di temperatura della pista di una rete con due correnti. Entrambi gli strumenti sono prototipi quasi statici mai confrontati con un altro risolutore né con una misura.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Risolvete la linea del lavoro. Il rapporto dà la mesh, le iterazioni e il residuo, poi impedenza, permittività effettiva, induttanza, capacità, ritardo e perdita dielettrica, con accanto la formula chiusa e la differenza in percentuale. È una prova a vuoto: non si scrive nulla.

```console
$ phoenix field2d examples/guides/stackup_job.json
```

2. Affinate la mesh di un fattore 2 per vedere quanto si sposta la risposta; un risultato che cambia poco è convergente. Una soluzione che non raggiunge la tolleranza dà `PHOENIX-FIELD-004` e nessun numero.

```console
$ phoenix field2d examples/guides/stackup_job.json --refine 2
```

3. Prendete la linea da una scheda. Lo strato e la larghezza su cui corre la maggior parte del rame della rete `MID` (una pista da 0,2 mm su `Top`) danno una microstriscia, risolta sullo stackup del lavoro.

```console
$ phoenix field2d examples/tutorials/tut.brd --stackup examples/guides/stackup_job.json --net MID
```

4. Aggiungete il risolutore di campo al rapporto di `stackup`, così la formula chiusa e la soluzione di campo compaiono in un solo rapporto.

```console
$ phoenix stackup examples/guides/stackup_job.json --freq-ghz 1 --field2d
```

5. Stimate il riscaldamento della rete `MID` con 0,5 A. Il rapporto elenca la pista per strato, larghezza e spessore del rame, e dà l'aumento secondo l'adattamento di IPC-2221 e secondo il modello di convezione. L'aumento è sotto il limite delle impostazioni e il codice di uscita è 0.

```console
$ phoenix thermal --currents examples/guides/currents_low.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

6. La stessa rete con 3 A. L'aumento supera il limite, la pista è segnata `[above the limit]` con l'avviso `PHOENIX-HEAT-004` e il codice di uscita è 1.

```console
$ phoenix thermal --currents examples/guides/currents_high.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

## Come leggere il risultato

* Il risolutore prende la linea come quasi-TEM su una mesh rettangolare in una scatola con pareti a potenziale zero, il che rende l'impedenza un po' più bassa che in una regione aperta. I conduttori sono rettangoli e la maschera di saldatura e la perdita del rame sono omesse. Una differenza dalla formula chiusa oltre `compare_warn_percent` è l'avviso `PHOENIX-FIELD-009`.
* La stima del riscaldamento fa passare tutta la corrente della rete in ogni pista e in ogni via; una rete che divide la corrente è quindi giudicata dal lato sicuro. IPC-2152 non è applicata e ogni rapporto lo dice (`PHOENIX-HEAT-012`). Uno strato che lo stackup non nomina prende i valori predefiniti delle impostazioni (`PHOENIX-HEAT-008`).
* I due comandi sono prove a vuoto e scrivono un rapporto JSON solo in un file nuovo indicato con `--out`. Le costanti e i limiti stanno in `config/field2d.cfg` e `config/heating.cfg`, non nel codice. Il codice di uscita è 0 se tutto va bene, 1 se una linea non è stata risolta o una pista supera il limite, e 2 per argomenti o ingresso non utilizzabili.

## Vedere anche

* [docs/FIELD2D.md](../../../../docs/FIELD2D.md)
* [docs/THERMAL.md](../../../../docs/THERMAL.md)
* [docs/STACKUP.md](../../../../docs/STACKUP.md)
