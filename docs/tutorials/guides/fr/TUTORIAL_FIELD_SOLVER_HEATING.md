# Résoudre l'impédance d'une ligne avec le solveur de champ et estimer l'échauffement d'une piste

`examples/guides/stackup_job.json` décrit un empilage de deux couches avec une ligne. Vous résolvez la section avec le solveur de champ 2D et comparez le résultat aux formules fermées, puis vous prenez la ligne d'une carte, et enfin vous estimez l'échauffement de la piste d'un réseau pour deux courants. Les deux outils sont des prototypes quasi statiques, jamais comparés à un autre solveur ni à une mesure.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Résolvez la ligne de la tâche. Le rapport donne le maillage, les itérations et le résidu, puis l'impédance, la permittivité effective, l'inductance, la capacité, le retard et la perte diélectrique, avec à côté la formule fermée et l'écart en pourcentage. C'est un essai à blanc : rien n'est écrit.

```console
$ phoenix field2d examples/guides/stackup_job.json
```

2. Affinez le maillage d'un facteur 2 pour voir de combien la réponse bouge ; un résultat qui change peu a convergé. Une solution qui n'atteint pas la tolérance donne `PHOENIX-FIELD-004` et aucun nombre.

```console
$ phoenix field2d examples/guides/stackup_job.json --refine 2
```

3. Prenez la ligne d'une carte. La couche et la largeur sur lesquelles court l'essentiel du cuivre du réseau `MID` (une piste de 0,2 mm sur `Top`) donnent une microruban, résolue sur l'empilage de la tâche.

```console
$ phoenix field2d examples/tutorials/tut.brd --stackup examples/guides/stackup_job.json --net MID
```

4. Ajoutez le solveur de champ au rapport de `stackup`, pour que la formule fermée et la solution de champ figurent dans un même rapport.

```console
$ phoenix stackup examples/guides/stackup_job.json --freq-ghz 1 --field2d
```

5. Estimez l'échauffement du réseau `MID` avec 0,5 A. Le rapport liste la piste par couche, largeur et épaisseur de cuivre, et donne l'élévation selon l'ajustement d'IPC-2221 (environ 4 K pour une piste de 0,2 mm sur 35 µm). Le modèle de convection ne donne pas de nombre, car avec le refroidissement des réglages une piste si étroite dépasserait 125 C (`PHOENIX-HEAT-005`, information). L'élévation est sous la limite des réglages et le code de sortie est 0.

```console
$ phoenix thermal --currents examples/guides/currents_low.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

6. Le même réseau avec 3 A. L'élévation dépasse la limite, la piste est marquée `[above the limit]` avec l'avertissement `PHOENIX-HEAT-004` et le code de sortie est 1.

```console
$ phoenix thermal --currents examples/guides/currents_high.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

## Comment lire le résultat

* Le solveur prend la ligne comme quasi-TEM sur un maillage rectangulaire dans une boîte aux parois à potentiel nul, ce qui rend l'impédance un peu plus basse qu'en région ouverte. Les conducteurs sont des rectangles ; le vernis épargne et la perte du cuivre sont omis. Un écart avec la formule fermée au-delà de `compare_warn_percent` est l'avertissement `PHOENIX-FIELD-009`.
* L'estimation de l'échauffement fait passer tout le courant du réseau dans chaque piste et chaque via ; un réseau qui répartit son courant est donc jugé du côté sûr. IPC-2152 n'est pas appliquée et chaque rapport le dit (`PHOENIX-HEAT-012`). Une couche que l'empilage ne nomme pas prend les valeurs par défaut des réglages (`PHOENIX-HEAT-008`).
* Les deux commandes sont des essais à blanc et n'écrivent un rapport JSON que dans un nouveau fichier nommé par `--out`. Les constantes et les limites sont dans `config/field2d.cfg` et `config/heating.cfg`, pas dans le code. Le code de sortie est 0 si tout va bien, 1 si une ligne n'a pu être résolue ou si une piste dépasse la limite, et 2 pour des arguments ou une entrée inutilisables.

## Voir aussi

* [docs/FIELD2D.md](../../../../docs/FIELD2D.md)
* [docs/THERMAL.md](../../../../docs/THERMAL.md)
* [docs/STACKUP.md](../../../../docs/STACKUP.md)
