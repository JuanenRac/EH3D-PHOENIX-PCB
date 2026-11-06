# Éditer une bibliothèque et générer des empreintes IPC

Vous vérifiez une bibliothèque EAGLE, vous lui ajoutez une empreinte avec un fichier d'opérations (tout ou rien, une copie de l'original est conservée) et vous générez des empreintes selon IPC-7351B, avec leurs noms canoniques. Les dimensions des empreintes générées sont un prototype et n'ont pas été vérifiées avec une fiche technique.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les tutoriels](README.md).

1. Vérifiez la bibliothèque. Rien n'est écrit sans `--out` ; le rapport compte les empreintes, symboles et jeux de composants et liste les constats.

```console
# test
$ phoenix libedit examples/tutorials/parts.lbr
~ 1 footprints, 1 symbols, 1 device sets
~ 0 errors, 0 warnings
~ dry run: nothing was written
```

2. Appliquez un fichier d'opérations qui crée le boîtier `HDR1X2` et y ajoute deux pastilles. La nouvelle bibliothèque est écrite dans un nouveau fichier, relue et comparée ; une copie de l'original est gardée à côté.

```console
# test
$ phoenix libedit --apply examples/tutorials/lib_ops.json --out build/parts_hdr.lbr examples/tutorials/parts.lbr
~ operation 3: add pad to HDR1X2
~ the file was read back and is the same library
```

3. Générez l'empreinte d'une résistance puce 0603 au niveau de densité B (nominal). Son nom canonique est `RESC1608X55N` : le corps fait 1,6 mm sur 0,8 mm et 0,55 mm de haut. Sans `--out`, c'est un essai à blanc qui liste les pastilles.

```console
# test
$ phoenix ipc generate chip 0603 --density B
RESC1608X55N (density B,
~ dry run: nothing was written
```

4. Écrivez-la dans un nouveau fichier `.lbr` (un nom `.kicad_mod` écrit à la place une empreinte KiCad).

```console
# test
$ phoenix ipc generate chip 0603 --density B --out build/RESC1608X55N.lbr
~ written: build/RESC1608X55N.lbr
```

5. Le générateur plus simple `footprint` crée une empreinte à partir de paramètres (familles puce, SOIC, TSSOP, QFN, QFP, DIP et barrette).

```console
# test
$ phoenix footprint chip --param size=0603 --out build/chip.lbr
~ footprint CHIP_0603
~ written:
```

6. Vérifiez les bibliothèques créées : chacune a une empreinte.

```console
# test
$ phoenix libedit build/chip.lbr
~ 1 footprints
$ phoenix libedit build/RESC1608X55N.lbr
~ 1 footprints
```

7. Comparez une empreinte de votre bibliothèque avec celle générée, pastille par pastille (code de sortie 1 si elles diffèrent), et listez les familles de la bibliothèque.

```console
$ phoenix ipc compare build/chip.lbr chip 0603
$ phoenix ipc list
```

## Voir aussi

* [docs/LIBRARY_EDIT.md](../../LIBRARY_EDIT.md)
* [docs/IPC_LIBRARY.md](../../IPC_LIBRARY.md)
* [docs/LIBINDEX.md](../../LIBINDEX.md)
* [docs/SANITIZER.md](../../SANITIZER.md)
