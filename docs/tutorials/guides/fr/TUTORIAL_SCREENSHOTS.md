# Prendre des images des fenêtres avec le mode image de phoenix

`phoenix --shot FICHIER.png` ouvre la fenêtre, attend quelques images, en enregistre une image au format PNG et se ferme seule. C'est ainsi que sont faites les images du README. Dans ce mode, la fenêtre ne lit ni n'écrit de préférences ni de liste de fichiers récents ; une exécution laisse donc vos réglages tels quels. `phoenix` est le programme de la fenêtre, à côté de `phoenix` dans l'archive de la version ; il se construit avec la fonctionnalité `gui`.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Prenez l'écran de démarrage, avec une taille, un thème et un nombre d'images à attendre. La taille est `LARGEURxHAUTEUR` en pixels, chaque côté de 320 à 8192 ; le thème est `dark`, `light` ou `contrast`.

```console
phoenix --shot build/start.png --size 1280x720 --theme dark --wait 6
```

2. Prenez la fenêtre d'une carte : `--open` donne le fichier et la fenêtre montrée est `document` par défaut. Une carte demande `--open` ; l'écran de démarrage ne peut pas en avoir.

```console
phoenix --shot build/board.png --open examples/tutorials/led_board.brd --theme light --wait 12
```

3. Prenez une autre fenêtre avec `--window` : `3d`, `panel`, `gerber`, `search`, `measures`, `layers`, `history`, `appearance`, `shortcuts` ou `help` (`3d`, `panel` et `gerber` demandent `--open`). `--lang` fixe la langue de la fenêtre.

```console
phoenix --shot build/help_pt.png --window help --lang pt --size 1280x720
```

4. Prenez tout le jeu d'images d'une version avec l'outil du dépôt. `--dry-run` affiche les lignes de commande et n'exécute rien ; sans lui, l'outil lance le programme une fois par fenêtre, vérifie chaque PNG (il ne doit pas être vide ni d'une seule couleur, et doit peser moins de 400 Ko) et écrit les fichiers `phoenix-pcb-NN-FENETRE.png`.

```console
py -3.14 -I tools/make_shots.py --exe PATH/phoenix --out build/screens --dry-run
```

## Comment lire le résultat

* L'image est ce que la fenêtre a dessiné, en pixels physiques ; sur un écran avec mise à l'échelle, elle peut donc être plus grande que `--size`. Le code de sortie est 0 si l'image a été écrite, 1 sinon et 2 pour une erreur dans les arguments. Un fichier existant n'est écrasé que si `--replace` est donné ; sans lui, l'exécution s'arrête avant l'ouverture de la fenêtre.
* Chaque exécution de l'outil a son propre dossier de réglages dans un dossier temporaire ; vos réglages et vos fichiers récents ne sont donc ni lus ni écrits. La prise d'image elle-même n'a pas encore été exécutée sur un écran, et l'on ne prétend pas qu'une image soit correcte tant que vous ne l'avez pas vue.

## Voir aussi

* [docs/VIEWER.md](../../../../docs/VIEWER.md)
* [tools/make_shots.py](../../../../tools/make_shots.py)
