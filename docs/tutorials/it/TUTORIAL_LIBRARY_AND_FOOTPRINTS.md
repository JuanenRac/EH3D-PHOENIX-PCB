# Modificare una libreria e generare impronte IPC

Si controlla una libreria EAGLE, le si aggiunge un'impronta con un file di operazioni (tutto o niente, e si conserva una copia dell'originale) e si generano impronte con il metodo IPC-7351B, con i loro nomi canonici. Le dimensioni delle impronte generate sono un prototipo e non sono state verificate con un datasheet.

Eseguite i comandi dalla radice del repository; vedere [come eseguire i tutorial](README.md).

1. Controllate la libreria. Senza `--out` non si scrive nulla; il rapporto conta impronte, simboli e set di dispositivi ed elenca i rilievi.

```console
# test
$ phoenix libedit examples/tutorials/parts.lbr
~ 1 footprints, 1 symbols, 1 device sets
~ 0 errors, 0 warnings
~ dry run: nothing was written
```

2. Applicate un file di operazioni che crea il package `HDR1X2` e vi aggiunge due pad. La nuova libreria viene scritta in un file nuovo, riletta e confrontata; accanto resta una copia dell'originale.

```console
# test
$ phoenix libedit --apply examples/tutorials/lib_ops.json --out build/parts_hdr.lbr examples/tutorials/parts.lbr
~ operation 3: add pad to HDR1X2
~ the file was read back and is the same library
```

3. Generate l'impronta di una resistenza chip 0603 al livello di densità B (nominale). Il suo nome canonico è `RESC1608X55N`: il corpo è 1,6 mm per 0,8 mm e alto 0,55 mm. Senza `--out` è una prova a secco che elenca i pad.

```console
# test
$ phoenix ipc generate chip 0603 --density B
RESC1608X55N (density B,
~ dry run: nothing was written
```

4. Scrivetela in un nuovo file `.lbr` (un nome `.kicad_mod` scrive invece un'impronta KiCad).

```console
# test
$ phoenix ipc generate chip 0603 --density B --out build/RESC1608X55N.lbr
~ written: build/RESC1608X55N.lbr
```

5. Il generatore più semplice `footprint` crea un'impronta da parametri (famiglie chip, SOIC, TSSOP, QFN, QFP, DIP e strip).

```console
# test
$ phoenix footprint chip --param size=0603 --out build/chip.lbr
~ footprint CHIP_0603
~ written:
```

6. Controllate le librerie create: ciascuna ha un'impronta.

```console
# test
$ phoenix libedit build/chip.lbr
~ 1 footprints
$ phoenix libedit build/RESC1608X55N.lbr
~ 1 footprints
```

7. Confrontate un'impronta della vostra libreria con quella generata, pad per pad (codice di uscita 1 se differiscono), ed elencate le famiglie della libreria.

```console
$ phoenix ipc compare build/chip.lbr chip 0603
$ phoenix ipc list
```

## Vedere anche

* [docs/LIBRARY_EDIT.md](../../LIBRARY_EDIT.md)
* [docs/IPC_LIBRARY.md](../../IPC_LIBRARY.md)
* [docs/LIBINDEX.md](../../LIBINDEX.md)
* [docs/SANITIZER.md](../../SANITIZER.md)
