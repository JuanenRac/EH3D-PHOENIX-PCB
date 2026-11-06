# Utiliser des scripts : une matrice de LED et un panneau

Les scripts du langage `phoenix-script/2` ont des variables, des boucles bornées, des requêtes sur une carte et des éditions qui forment une seule étape réversible, dans un bac à sable : pas de réseau, pas de programme, pas de fichier hors du dossier de données des réglages. Une exécution est un essai à blanc sauf si `--out` nomme un nouveau fichier. Les exemples sont dans `examples/scripts/`.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les tutoriels](README.md).

1. Listez les scripts d'exemple fournis avec le programme et affichez-en un.

```console
# test
$ phoenix script list
~ led_matrix
~ panelize_matrix
$ phoenix script list --show led_matrix
phoenix-script/2 board
```

2. Vérifiez un script sans l'exécuter : syntaxe, noms, nombre d'arguments, document auquel chaque fonction s'applique et estimation des étapes.

```console
# test
$ phoenix script check examples/scripts/led_matrix.phs
```

3. Exécutez la matrice de LED sur une carte dont l'empreinte est `LED0603` : six LED au pas de 2,5 mm, avec la deuxième pastille de chacune sur un même réseau. La carte ne change pas : c'est un essai à blanc.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --board examples/tutorials/led_board.brd
~ placed 6 LEDs on net LEDCOM
~ pads on the net: 6
~ dry run: nothing was written
```

4. Le script de mise en panneau copie les deux pièces de la carte dans une matrice de 2 sur 2 : de 2 composants à 8.

```console
# test
$ phoenix script run examples/scripts/panelize_matrix.phs --board examples/tutorials/tut.brd
~ copies: 4
~ components: 2 -> 8
```

5. Avec `--out`, le résultat est écrit une fois dans un nouveau fichier. Un fichier qui existe n'est jamais remplacé.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
~ written
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
# exit 2
~~ never replaced
```

## Voir aussi

* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/SCRIPT_API.md](../../SCRIPT_API.md)
* [docs/SCRIPT_EXAMPLES.md](../../SCRIPT_EXAMPLES.md)
