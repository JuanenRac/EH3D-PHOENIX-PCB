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

## Voir aussi

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
