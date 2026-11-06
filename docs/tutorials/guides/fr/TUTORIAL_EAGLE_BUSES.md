# Lire les bus d'un schéma EAGLE

`examples/guides/bus.sch` est le schéma du premier tutoriel avec un bus `D[0..1]` dessiné sur la feuille et deux réseaux, `D0` et `D1`, dont les fils se terminent sur le fil du bus. Le lecteur transforme le nom du bus en objet bus avec ses membres et place une entrée là où chaque réseau rencontre le bus. Un bus est une façon de nommer : ses membres sont des réseaux ordinaires.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Lisez la hiérarchie en JSON. La clé `buses` compte les bus de la feuille : 1. La liste des problèmes ne contient aucun code de la famille `PHOENIX-BUS`, parce que les deux membres sont utilisés par une entrée.

```console
$ phoenix hier --json examples/guides/bus.sch
```

2. La même chose en texte : la liste des réseaux ne contient que les réseaux qui relient au moins deux broches, ici `MID`, donc les membres `D0` et `D1` du bus, qui n'ont pas de broche, n'y figurent pas, et la dernière ligne dit s'il y a des problèmes.

```console
$ phoenix hier examples/guides/bus.sch
```

3. Écrivez le schéma en fichiers KiCad. Le bus est écrit comme son étiquette et une ligne depuis son ancre à travers les extrémités de ses entrées, et chaque entrée comme un `bus_entry` avec l'étiquette de son membre.

```console
$ phoenix kicad-export --out build/bus.kicad_sch schematic examples/guides/bus.sch
```

## Comment lire le résultat

* Un vecteur `préfixe[premier..dernier]` donne les membres `préfixe<premier>` à `préfixe<dernier>` (`D[0..1]` donne `D0` et `D1` ; un vecteur peut descendre, comme `A[2..0]`). Le lecteur accepte aussi les listes d'EAGLE (`A[0..3],B[0..3]`) et un nom avec deux-points (`BUS:A[0..3]`), où ce qui précède les deux-points nomme le bus lui-même.
* Le bus est ancré là où son premier fil commence. Une entrée est créée à l'extrémité de chaque réseau de la feuille qui porte le nom d'un membre et finit sur un fil du bus. Les fils et étiquettes du bus, les noms qui ne sont pas des vecteurs et un bus sans fil ne sont pas lus et sont comptés comme parties omises. Les codes `PHOENIX-BUS-001` à `PHOENIX-BUS-007` disent ce qui ne va pas dans un bus, et `phoenix hier` les liste.

## Voir aussi

* [docs/BUSES.md](../../../../docs/BUSES.md)
* [docs/HIERARCHY.md](../../../../docs/HIERARCHY.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
