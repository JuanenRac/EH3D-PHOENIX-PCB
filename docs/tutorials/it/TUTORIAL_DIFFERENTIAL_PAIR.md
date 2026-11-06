# Rivedere l'integrità del segnale di una coppia differenziale

La scheda `examples/tutorials/diffpair.kicad_pcb` ha due piste da 0,25 mm, `D_P` e `D_N`, affiancate per 20 mm con 0,75 mm tra i bordi. Si controlla la coppia con un file che dice come deve essere, si vede il rilievo quando la distanza è sbagliata, lo si legge in spagnolo e si stima la diafonia. Gli strumenti di integrità del segnale sono stime quasi statiche da formule chiuse, non un solutore di campo.

Eseguite i comandi dalla radice del repository; vedere [come eseguire i tutorial](README.md).

1. Il file `pairs.json` dice che la coppia è `D_P` e `D_N`, con piste da 0,25 mm e una distanza di 0,75 mm. La scheda corrisponde: nessun errore e nessun avviso.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs.json examples/tutorials/diffpair.kicad_pcb
~ pairs: 1, errors: 0, warnings: 0
~ pair p (D_P / D_N)
```

2. Ora il file chiede una distanza di 0,3 mm e la scheda ha 0,75 mm: la distanza è fuori tolleranza, il codice è `PHOENIX-PAIR-003` e il codice di uscita è 1.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ PHOENIX-PAIR-003
```

3. Lo stesso rapporto in spagnolo. `--lang` accetta en, es, de, fr, it, ja, zh e pt.

```console
# test
$ phoenix pairs --lang es --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ La separación del par está fuera de tolerancia
```

4. Stimate la diafonia tra le due piste dalla lunghezza in parallelo, dalla distanza tra i centri e dallo stack-up. La stima è in percentuale del gradino dell'aggressore.

```console
# test
$ phoenix si crosstalk examples/tutorials/diffpair.kicad_pcb
~ crosstalk: 1 coupled pairs of nets
```

5. Le altre analisi: riflessioni e terminazioni di una rete, un diagramma a occhio semplificato e tutte insieme con il percorso di ritorno (codice di uscita 1 se si supera una soglia di `config/si.cfg`).

```console
$ phoenix si reflect --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si eye --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si check examples/tutorials/diffpair.kicad_pcb
```

## Vedere anche

* [docs/DIFFPAIRS.md](../../DIFFPAIRS.md)
* [docs/SI.md](../../SI.md)
* [docs/LENGTHS.md](../../LENGTHS.md)
* [docs/STACKUP.md](../../STACKUP.md)
