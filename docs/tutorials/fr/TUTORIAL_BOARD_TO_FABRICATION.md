# De la carte aux fichiers de fabrication

Le projet KiCad de `examples/tutorials/kicad/` a deux résistances sur une carte de 30 mm sur 20 mm reliées par le réseau `SIG`. Vous comparez schéma et carte, vous passez les règles de conception et vous écrivez les fichiers de fabrication avec un manifeste de sommes.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les tutoriels](README.md).

1. Lancez la vérification des règles électriques du schéma. Chaque constat porte un code et les constats confirmés sont séparés des non concluants.

```console
$ phoenix erc examples/tutorials/kicad/demo.kicad_sch
```

2. Comparez le schéma et la carte par l'identité des pièces. `sync` est un essai à blanc : il liste ce que le transfert changerait et n'écrit rien sauf si vous ajoutez `--apply --out FICHIER`.

```console
$ phoenix sync examples/tutorials/kicad/demo.kicad_sch examples/tutorials/kicad/demo.kicad_pcb
```

3. Passez les règles de conception avec les limites d'un fichier de règles. La piste fait 0,25 mm et le fichier demande 0,2 mm au moins : rien n'est faux et le code de sortie est 0.

```console
# test
$ phoenix check --rules examples/tutorials/rules_relaxed.json examples/tutorials/kicad/demo.kicad_pcb
Confirmed: 0
Inconclusive: 0
```

4. Écrivez les fichiers de fabrication dans un nouveau dossier : un Gerber X2 par couche, les fichiers de perçage, celui de placement, la nomenclature et `manifest.json` avec le SHA-256 de chaque fichier. Avant l'écriture, les fichiers sont relus et comparés à la carte.

```console
# test
$ phoenix export --out build/fab examples/tutorials/kicad/demo.kicad_pcb
~ files written: build/fab
```

5. Créez le paquet complet, avec le pochoir et le placement de chaque face, en un zip déterministe : la même carte donne les mêmes octets. Sans `--out`, c'est un essai à blanc qui liste les fichiers.

```console
$ phoenix fabpack --out build/demo_pack.zip examples/tutorials/kicad/demo.kicad_pcb
```

6. Regardez la carte en chiffres avant de l'envoyer : la taille, les décomptes, le cuivre de chaque couche, les perçages et la densité. Rien n'est écrit.

```console
# test
$ phoenix stats examples/tutorials/kicad/demo.kicad_pcb
Board statistics:
Board: 30.000000 x 20.000000 mm; 600.000000 mm2; Cutouts: 0.000000 mm2
Components: 2 (top: 2; bottom: 0)
```

7. Marquez un pad comme point de test avec une opération de l'éditeur de cartes (`examples/tutorials/testpoint_ops.json` marque le deuxième pad de R1, sur le réseau `MID`) et obtenez le rapport des points de test et des réseaux qu'aucune sonde n'atteint. Le fichier d'entrée n'est jamais écrasé.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/testpoint_ops.json --out build/with_test_point.json examples/tutorials/tut.brd
~ written: build/with_test_point.json
$ phoenix testpoints build/with_test_point.json
Test points of the board:
Test points: 1 (probe side: both)
Nets asked: 1; covered: 1; coverage: 100.00 %
```

8. Ajoutez deux cotes à une carte qui a une couche de documentation (`examples/tutorials/dimension_ops.json` : la largeur et la hauteur de la carte, avec la ligne de cote à 3 mm à l'extérieur) et écrivez-les dans un dessin DXF ; dans la fenêtre, on fait la même chose avec l'outil de cote, et *Modifier la cote* déplace leurs points. Le dessin est un essai à blanc tant que `--out` ne nomme pas un nouveau fichier.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/dimension_ops.json --out build/with_dimensions.json examples/tutorials/notes_board.json
~ written: build/with_dimensions.json
$ phoenix dxf-export board --dimensions build/with_dimensions.json
entities: 21
layer DIMENSIONS: 16
```

9. Demandez à l'assistant fournisseurs où acheter la nomenclature. Il lit un fichier de prix (`examples/tutorials/prices.csv`, une ligne par fournisseur et palier de quantité) ainsi que les fournisseurs, les frais d'envoi et la commande minimale d'un fichier de réglages (`examples/tutorials/sourcing.cfg`), et chiffre la nomenclature chez chaque fournisseur et en achat réparti. Les prix sont ceux du fichier : rien n'est récupéré sur un réseau.

```console
# test
$ phoenix bom --sourcing --config examples/tutorials/sourcing.cfg --prices examples/tutorials/prices.csv examples/tutorials/kicad/demo.kicad_pcb
~ suppliers: 2 considered, 1 lines, EUR
~ cheapest single supplier: Shop A for 2.210000
~ split purchase: 1 orders for 2.210000
```

## Voir aussi

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
* [docs/STATS.md](../../STATS.md)
* [docs/TESTPOINTS.md](../../TESTPOINTS.md)
* [docs/DIMENSIONS.md](../../DIMENSIONS.md)
* [docs/BOM.md](../../BOM.md)
