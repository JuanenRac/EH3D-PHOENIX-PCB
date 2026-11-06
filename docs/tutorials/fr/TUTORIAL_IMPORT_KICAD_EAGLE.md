# Importer un projet KiCad ou EAGLE et le revoir

Vous lisez un projet KiCad entier dans le modèle de conception du programme, vous voyez ce qui a été gardé et ce qui a été laissé de côté, vous écrivez le modèle en nouveaux fichiers et vous vérifiez la carte avec un fichier de règles. Puis vous faites la même revue pour une carte EAGLE.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les tutoriels](README.md).

1. Lisez le projet (le `.kicad_pro` avec le `.kicad_sch` et le `.kicad_pcb` du même nom). C'est un essai à blanc : le rapport dit ce qui a été lu, ce qui a été gardé et ce que le modèle ne contient pas.

```console
# test
$ phoenix kicad-import examples/tutorials/kicad/demo.kicad_pro
~ usable: yes
~ components: 2
~ tracks: 1
~ vias: 1
~ dry run: nothing was written
```

2. Écrivez le modèle en nouveaux fichiers au nom indiqué. Un fichier existant, ou une entrée, n'est jamais écrasé.

```console
# test
$ phoenix kicad-import --out build/demo.json examples/tutorials/kicad/demo.kicad_pro
~ written:
```

3. Vérifiez la carte importée. Une carte KiCad ne donne pas de règles de conception au programme : les limites viennent d'un fichier de règles. Le fichier demande des pistes de 0,3 mm au moins et la piste de la carte fait 0,25 mm : un constat confirmé et le code de sortie 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
Confirmed: 1
~ PHOENIX-CHECK-001
```

4. Un fichier EAGLE : demandez ce qu'une réécriture garde. Le rapport prouve qu'écrire l'arbre XML et le relire ne change rien, et liste ce que le modèle de conception simplifie ou laisse de côté.

```console
# test
$ phoenix eagle-check examples/tutorials/tut.brd
board (version 9.6.2)
~ rewrite of the tree: identical
```

5. Passez le même fichier de règles sur la carte EAGLE. La piste du réseau `MID` fait 0,2 mm.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/tut.brd
# exit 1
Confirmed: 1
```

6. Comparez le schéma EAGLE avec la carte EAGLE : le rapport liste les réseaux manquants, les broches non connectées et le cuivre pas encore routé.

```console
$ phoenix connectivity examples/tutorials/tut.sch examples/tutorials/tut.brd
```

## Voir aussi

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD.md](../../KICAD.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/RULES.md](../../RULES.md)
