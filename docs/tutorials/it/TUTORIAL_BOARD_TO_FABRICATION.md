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

## Vedere anche

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
