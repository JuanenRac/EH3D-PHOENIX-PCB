# Borner le travail des vérifications avec un fichier de limites

Certaines vérifications prennent un temps proportionnel au carré d'un nombre, par exemple le test que les arêtes d'un polygone ne se croisent pas. Une limite borne ce temps : un polygone au-delà de la limite n'est pas testé et le rapport le dit comme information. Vous lancez la vérification d'une carte avec les valeurs par défaut puis avec un fichier de limites beaucoup trop bas, et vous voyez la note.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Lancez le fichier de règles sur la carte KiCad avec les limites par défaut. La piste de la carte est plus fine que ce que demande le fichier : il y a un constat confirmé et le code de sortie est 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. Relancez avec `--limits examples/guides/limits_small.cfg`, qui contient `max_polygon_points = 3`. Le contour de la carte a quatre points ; il est donc listé comme information avec le code `PHOENIX-BOARD-017`, et le code de sortie reste 1 car une information ne le change jamais.

```console
# test
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ PHOENIX-BOARD-017
~ max_polygon_points = 3
~ PHOENIX-CHECK-001
```

3. La même note dans une autre langue et en JSON pour un script. Le code reste en anglais ; le titre suit `--lang`.

```console
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --lang pt examples/tutorials/kicad/demo.kicad_pcb
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --json examples/tutorials/kicad/demo.kicad_pcb
```

## Comment lire le résultat

* `--limits FICHIER` est accepté par toute commande, avant ou après son nom. Le fichier contient des lignes `clé = valeur` de `config/board.cfg` et de `config/eagle_board.cfg` dans n'importe quel ordre ; une ligne inconnue, ou une valeur hors de son intervalle, est refusée avec le numéro de la ligne et le code de sortie 2.
* Une limite ne change que ce qu'elle borne. Avec les valeurs par défaut aucune sortie ne change, et un polygone au-delà de `max_polygon_points` voit toujours ses points répétés vérifiés.

## Voir aussi

* [config/board.cfg](../../../../config/board.cfg)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
* [docs/RULES.md](../../../../docs/RULES.md)
