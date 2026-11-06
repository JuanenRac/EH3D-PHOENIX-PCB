# Sélectionner plusieurs objets dans l'éditeur de carte et les déplacer, les aligner ou les supprimer en une seule étape

La sélection de l'éditeur de carte est un ensemble ordonné de composants, pistes, vias, zones et éléments de sérigraphie. Sans écran, on la pilote avec `phoenix boardedit` et un fichier d'opérations : `select` change la sélection, et `move_selection`, `delete_selection` et `align_selection` agissent sur elle en une seule étape de l'historique avec un inverse exact. `examples/guides/select_ops.json` sélectionne avec un rectangle ce qui est entièrement dedans, ici la résistance R1 et non la piste, et la déplace de 2 mm vers le haut.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Appliquez les opérations en essai à blanc. Le rapport liste les trois opérations et les erreurs de règles avant et après ; rien n'est écrit. Le fichier de règles est donné parce que la carte EAGLE n'a pas de règles de conception propres.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json examples/tutorials/tut.brd
```

2. Écrivez la carte modifiée dans un nouveau fichier (`.json` pour le modèle de conception, `.brd` pour EAGLE). L'entrée n'est jamais écrasée. Le code de sortie est 0 si les règles ne prouvent aucune erreur sur le résultat et 1 si elles en prouvent une.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --out build/moved.json examples/tutorials/tut.brd
```

3. Affichez le rapport en JSON, qu'un script peut lire : la clé `rule_errors` est le nombre d'erreurs que les règles prouvent sur le résultat.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --json examples/tutorials/tut.brd
```

## Comment lire le résultat

* Les étapes de `select` sont `add`, `remove` et `toggle` (les objets de `items`, écrits `{"kind": "component", "id": "..."}`), `net` (tous les objets d'un réseau, par son id), `rect` (un rectangle entre `from` et `to`, en nanomètres sous la forme `x_nm` et `y_nm` ; `mode` vaut `contained` ou `crossed`, qui prend aussi ce que le rectangle touche), `all`, `invert` et `clear`. Elles s'appliquent dans l'ordre, tout ou rien, et la sélection ne fait pas partie de l'historique.
* `move_selection` déplace de `dx_nm` et `dy_nm` (le haut est positif), arrondi à la grille sauf si `snap` vaut false ; les pistes non sélectionnées ne suivent pas un composant, sélectionnez donc d'abord le réseau pour emporter le cuivre. `delete_selection` supprime la sélection. `align_selection` accepte `left`, `right`, `top`, `bottom`, `center_x`, `center_y` (deux objets au moins) et `distribute_x`, `distribute_y` (trois au moins). Un composant verrouillé refuse toute l'opération (`PHOENIX-BEDIT-006`), une sélection vide est `PHOENIX-BEDIT-009`.
* Dans la fenêtre, avec l'outil Sélection : Maj ou Ctrl+clic bascule un objet ; Maj ou Ctrl+glisser à un endroit vide ajoute ce qui est entièrement dans le rectangle (zones et éléments de sérigraphie aussi) ; Alt+glisser ajoute aussi ce que le rectangle touche ; `N` sélectionne le réseau sous le pointeur ; Ctrl+A sélectionne tout ; Échap abandonne la sélection ; les flèches la déplacent d'un pas de grille (Maj : dix pas) ; Suppr la supprime ; Alt+Gauche, Droite, Haut, Bas, Alt+C et Alt+M l'alignent ; Alt+H et Alt+V la répartissent à intervalles égaux (trois objets au moins) ; Ctrl+I inverse la sélection ; un clic sur un élément de sérigraphie ou sur le bord d'une zone le prend ; glisser un objet sélectionné déplace toute la sélection sur la grille ; la barre d'état donne le nombre d'objets avec leur cadre et leur taille. Chacune est une étape de l'historique et Ctrl+Z rend les mêmes octets. La fenêtre n'a jamais été vue sur un écran ni utilisée à la souris.

## Voir aussi

* [docs/BOARD_EDIT.md](../../../../docs/BOARD_EDIT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
* [docs/VIEWER.md](../../../../docs/VIEWER.md)
