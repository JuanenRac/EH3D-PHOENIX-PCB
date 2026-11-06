# Mettre une carte en panneau et écrire les fichiers du panneau

Vous disposez deux fois la carte de 30 mm sur 20 mm côte à côte, reliées par une rainure en V, et vous écrivez les fichiers Gerber et de perçage de tout le panneau. Les réglages sont dans `examples/tutorials/panel.cfg` (colonnes, lignes, type de jonction, espace et cadre) ; la liste complète est `config/panel.cfg`.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les tutoriels](README.md).

1. Planifiez le panneau. Un essai à blanc affiche sa taille, la jonction, les trous et les fichiers qui seraient écrits, et n'écrit rien.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ joint: V-score, 1 lines
~ dry run: nothing written
```

2. Écrivez les couches de chaque copie, les fichiers de perçage, le contour avec la rainure en V, le placement et la nomenclature dans un nouveau dossier.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ written
```

3. On n'écrit jamais dans un dossier qui existe : la deuxième exécution est refusée avec le code `PHOENIX-PANEL-008` et le code de sortie 2.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
# exit 2
~~ PHOENIX-PANEL-008
```

4. Vérifiez la structure d'un fichier Gerber du panneau et dessinez le panneau en DXF pour la partie mécanique.

```console
$ phoenix gerber-check build/panel/panel_01_top.gbr
$ phoenix dxf-export panel --out build/panel.dxf --panel-config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
```

## Voir aussi

* [docs/PANEL.md](../../PANEL.md)
* [docs/GERBER_IMPORT.md](../../GERBER_IMPORT.md)
* [docs/DXF.md](../../DXF.md)
* [docs/FABPACK.md](../../FABPACK.md)
