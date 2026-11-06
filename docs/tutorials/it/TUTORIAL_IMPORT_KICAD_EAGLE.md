# Importare un progetto KiCad o EAGLE e rivederlo

Si legge un intero progetto KiCad nel modello di progetto del programma, si vede ciò che è rimasto e ciò che no, si scrive il modello come nuovi file e si controlla la scheda con un file di regole. Poi si fa la stessa revisione per una scheda EAGLE.

Eseguite i comandi dalla radice del repository; vedere [come eseguire i tutorial](README.md).

1. Leggete il progetto (il `.kicad_pro` con il `.kicad_sch` e il `.kicad_pcb` dello stesso nome). È una prova a secco: il rapporto dice cosa è stato letto, quanto è rimasto e cosa il modello non contiene.

```console
# test
$ phoenix kicad-import examples/tutorials/kicad/demo.kicad_pro
~ usable: yes
~ components: 2
~ tracks: 1
~ vias: 1
~ dry run: nothing was written
```

2. Scrivete il modello come nuovi file con il nome indicato. Un file esistente, o un ingresso, non viene mai sovrascritto.

```console
# test
$ phoenix kicad-import --out build/demo.json examples/tutorials/kicad/demo.kicad_pro
~ written:
```

3. Controllate la scheda importata. Una scheda KiCad non dà regole di progetto al programma, quindi i limiti vengono da un file di regole. Il file chiede piste di almeno 0,3 mm e la pista della scheda è di 0,25 mm: un rilievo confermato e codice di uscita 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
Confirmed: 1
~ PHOENIX-CHECK-001
```

4. Un file EAGLE: chiedete cosa conserva una riscrittura. Il rapporto prova che scrivere l'albero XML e rileggerlo non cambia nulla, ed elenca ciò che il modello di progetto semplifica o lascia fuori.

```console
# test
$ phoenix eagle-check examples/tutorials/tut.brd
board (version 9.6.2)
~ rewrite of the tree: identical
```

5. Eseguite lo stesso file di regole sulla scheda EAGLE. La pista della rete `MID` è larga 0,2 mm.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/tut.brd
# exit 1
Confirmed: 1
```

6. Confrontate lo schema EAGLE con la scheda EAGLE: il rapporto elenca le reti mancanti, i pin non collegati e il rame non ancora instradato.

```console
$ phoenix connectivity examples/tutorials/tut.sch examples/tutorials/tut.brd
```

## Vedere anche

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD.md](../../KICAD.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/RULES.md](../../RULES.md)
