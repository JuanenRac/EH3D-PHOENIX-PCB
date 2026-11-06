# Garder plusieurs dispositions de panneau dans un seul fichier de réglages avec des modèles

`examples/guides/panel_templates.cfg` contient une disposition de base (deux cartes côte à côte, reliées par une rainure en V, sans cadre) et deux modèles nommés. Chaque modèle ne change que les lignes qu'il nomme, et `--template` en choisit un. La carte est celle du tutoriel du panneau, 30 mm sur 20 mm ; toutes les commandes ici sont des essais à blanc.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Sans modèle, seule la base est lue et les lignes sous les sections de modèle sont ignorées : 60 mm sur 20 mm, 2 x 1 cartes.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg examples/tutorials/kicad/demo.kicad_pcb
```

2. Le modèle `four` passe `columns` à 4 et rien d'autre : 120 mm sur 20 mm, 4 x 1 cartes.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four examples/tutorials/kicad/demo.kicad_pcb
```

3. Le modèle `grid` passe `rows` à 2 : 60 mm sur 40 mm, 2 x 2 cartes.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template grid examples/tutorials/kicad/demo.kicad_pcb
```

4. `--columns` et `--rows` l'emportent toujours sur un modèle : avec `four` et `--columns 3`, le panneau fait 90 mm sur 20 mm, 3 x 1 cartes.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --columns 3 examples/tutorials/kicad/demo.kicad_pcb
```

5. Un nom que le fichier n'a pas est refusé avec le code `PHOENIX-PANEL-001` et le code de sortie 2.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template ring examples/tutorials/kicad/demo.kicad_pcb
```

6. Écrivez les fichiers de la disposition choisie dans un nouveau dossier, comme dans le tutoriel du panneau.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --out build/panel_four examples/tutorials/kicad/demo.kicad_pcb
```

## Comment lire le résultat

* Les lignes avant la première ligne `[template NOM]` forment la base. Un nom compte de 1 à 32 lettres, chiffres, `_` ou `-` ; une section écrite autrement est refusée avec `PHOENIX-PANEL-001` et le numéro de la ligne.
* La ligne de taille est `panel W mm by H mm, C x R boards`. Avec la jonction en V et un espace de 0, les cartes se touchent : la largeur est le nombre de colonnes fois 30 mm et la hauteur le nombre de lignes fois 20 mm. Un modèle est une façon de garder une disposition sous un nom ; ce n'est le réglage d'aucun fabricant.

## Voir aussi

* [docs/PANEL.md](../../../../docs/PANEL.md)
* [config/panel.cfg](../../../../config/panel.cfg)
