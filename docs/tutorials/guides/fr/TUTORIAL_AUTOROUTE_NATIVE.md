# Router une carte avec l'autorouteur intégré, avec une paire et avec les vias déjà présents sur la carte

`phoenix autoroute native` route les connexions ouvertes d'une carte avec une recherche propre et n'a besoin d'aucun programme extérieur. `examples/guides/open.brd` a un réseau dont les deux pastilles ne sont pas reliées ; `examples/guides/open_pair.brd` a les deux réseaux `D_P` et `D_N` d'une paire, eux aussi non reliés. Le plan n'ajoute que des pistes et des vias, et les règles sont vérifiées avant et après. Le fichier de règles est donné parce que les cartes EAGLE des exemples n'ont pas de règles de conception propres.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Planifiez les routes. C'est un essai à blanc : le rapport liste les connexions routées, les pistes, vias et longueur ajoutés et les connexions encore ouvertes, et dit sur le flux d'erreur que rien n'a été écrit.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json examples/guides/open.brd
```

2. Écrivez la carte avec les routes dans un nouveau fichier. Le fichier est le JSON du modèle de conception, quel que soit son nom, et un fichier existant n'est jamais remplacé.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --out build/open_routed.json examples/guides/open.brd
```

3. Routez de nouveau la carte écrite : il ne lui reste aucune connexion ouverte, il n'y a donc rien à ajouter.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json build/open_routed.json
```

4. Demandez la paire. `pair_dp_dn.json` dit que `D_P` et `D_N` forment une paire avec des pistes de 0,2 mm et 0,3 mm entre elles. S'il y a de la place, les deux pistes sont posées côte à côte sur une couche et le rapport dit `coupled pair: p` ; sinon les deux réseaux sont routés comme des réseaux ordinaires et une note de code `PHOENIX-AROUTE-017` donne la raison.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json examples/guides/open_pair.brd
```

5. Routez seulement les deux réseaux de la paire et refusez un plan qui laisse une connexion ouverte : avec `--require-complete`, la commande se termine avec le code de sortie 2 et le code `PHOENIX-AROUTE-016` s'il reste une connexion.

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json --nets D_P,D_N --require-complete examples/guides/open_pair.brd
```

## Comment lire le résultat

* Un via du réseau déjà présent sur la carte, sur un nœud de la grille, est un changement de couche gratuit : la route y change de couche et ne pose aucun nouveau via. Avec `via_snap_nm` supérieur à 0 dans `config/autoroute_native.cfg`, un via situé à cette distance au plus d'un nœud est utilisé aussi, avec une note `PHOENIX-AROUTE-019`. C'est la réutilisation des vias ; le plan ne change jamais une piste, un via ou une pastille déjà présents.
* Une paire n'est couplée que si chaque réseau a exactement une connexion ouverte et si les quatre pastilles partagent une couche. Les deux pistes ont la même longueur et gardent l'espacement de la paire. Sans `--pairs` ni `--pair-config`, les réseaux d'une paire sont routés comme deux réseaux ordinaires et la vérification des paires de l'éditeur de carte juge le résultat ensuite.
* Un prototype essayé sur de petites cartes faites à la main et par les tests. La recherche, ses coûts et ses limites (qui comptent des nœuds et jamais du temps) sont dans `config/autoroute_native.cfg`. Si les routes d'un réseau ajoutent ensemble une erreur de règles, ce réseau est laissé de côté et signalé par `PHOENIX-AROUTE-015` : le plan n'ajoute jamais une erreur que la carte n'avait pas.

## Voir aussi

* [docs/AUTOROUTE_NATIVE.md](../../../../docs/AUTOROUTE_NATIVE.md)
* [docs/DIFFPAIRS.md](../../../../docs/DIFFPAIRS.md)
* [docs/AUTOROUTE.md](../../../../docs/AUTOROUTE.md)
