# Dallo schema alla netlist e alla simulazione

Il file `examples/tutorials/divider.json` è uno schema con una sorgente da 10 V e due resistenze da 1 kΩ e 3 kΩ in serie. Si crea la netlist di simulazione senza scriverla a mano, la si simula e si verifica il risultato con la formula del partitore: 10 V × 3 / (1 + 3) = 7,5 V.

Eseguite i comandi dalla radice del repository; vedere [come eseguire i tutorial](README.md).

1. Stampate la netlist dello schema. Non viene scritto nulla (prova a secco) e prima si esegue il controllo delle regole elettriche. La prima riga è il titolo; seguono gli elementi, con i valori nella notazione del simulatore.

```console
# test
$ phoenix netlist examples/tutorials/divider.json
divider
~ R1 in out 1e3
~ R2 out 0 3e3
~ V1 in 0 1e1
```

2. Scrivetela in un file nuovo. Un file esistente non viene mai sovrascritto.

```console
# test
$ phoenix netlist examples/tutorials/divider.json --out build/divider.cir
~ written:
```

3. Calcolate il punto di lavoro. La formula del partitore dà 7,5 V al nodo `out`.

```console
# test
$ phoenix sim build/divider.cir --op --probe "v(out)"
name,value
v(out),7.5e0
```

4. Senza file: `--schematic` crea la netlist dallo schema e la passa direttamente al solutore.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --op --probe "v(out)"
name,value
v(out),7.5e0
```

5. Variate la sorgente da 0 V a 10 V a passi di 5 V: l'uscita è sempre tre quarti dell'ingresso.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --dc V1 0 10 5 --probe "v(out)"
~ 5e0,3.75e0
~ 1e1,7.5e0
```

## Vedere anche

* [docs/SIM.md](../../SIM.md)
* [docs/NETLIST.md](../../NETLIST.md)
* [docs/ERC.md](../../ERC.md)
