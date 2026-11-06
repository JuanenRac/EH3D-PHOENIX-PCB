# Revoir l'intégrité du signal d'une paire différentielle

La carte `examples/tutorials/diffpair.kicad_pcb` a deux pistes de 0,25 mm, `D_P` et `D_N`, côte à côte sur 20 mm avec 0,75 mm entre leurs bords. Vous vérifiez la paire avec un fichier qui dit ce qu'elle doit être, vous voyez le constat quand l'écart est faux, vous le lisez en espagnol et vous estimez la diaphonie. Les outils d'intégrité du signal sont des estimations quasi statiques à partir de formules fermées, pas un solveur de champ.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les tutoriels](README.md).

1. Le fichier `pairs.json` dit que la paire est `D_P` et `D_N`, avec des pistes de 0,25 mm et un écart de 0,75 mm. La carte correspond : ni erreur ni avertissement.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs.json examples/tutorials/diffpair.kicad_pcb
~ pairs: 1, errors: 0, warnings: 0
~ pair p (D_P / D_N)
```

2. Maintenant le fichier demande un écart de 0,3 mm et la carte a 0,75 mm : l'écart est hors tolérance, le code est `PHOENIX-PAIR-003` et le code de sortie 1.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ PHOENIX-PAIR-003
```

3. Le même rapport en espagnol. `--lang` accepte en, es, de, fr, it, ja, zh et pt.

```console
# test
$ phoenix pairs --lang es --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ La separación del par está fuera de tolerancia
```

4. Estimez la diaphonie entre les deux pistes à partir de la longueur en parallèle, de la distance entre centres et de l'empilage. L'estimation est en pourcentage de l'échelon de l'agresseur.

```console
# test
$ phoenix si crosstalk examples/tutorials/diffpair.kicad_pcb
~ crosstalk: 1 coupled pairs of nets
```

5. Les autres analyses : les réflexions et terminaisons d'un réseau, un diagramme de l'œil simplifié et toutes ensemble avec le chemin de retour (code de sortie 1 si un seuil de `config/si.cfg` est dépassé).

```console
$ phoenix si reflect --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si eye --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si check examples/tutorials/diffpair.kicad_pcb
```

## Voir aussi

* [docs/DIFFPAIRS.md](../../DIFFPAIRS.md)
* [docs/SI.md](../../SI.md)
* [docs/LENGTHS.md](../../LENGTHS.md)
* [docs/STACKUP.md](../../STACKUP.md)
