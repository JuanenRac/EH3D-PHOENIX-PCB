# Du schéma à la netlist et à la simulation

Le fichier `examples/tutorials/divider.json` est un schéma avec une source de 10 V et deux résistances de 1 kΩ et 3 kΩ en série. Vous créez sa netlist de simulation sans l'écrire à la main, vous la simulez et vous vérifiez le résultat avec la formule du diviseur : 10 V × 3 / (1 + 3) = 7,5 V.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les tutoriels](README.md).

1. Affichez la netlist du schéma. Rien n'est écrit (essai à blanc) et la vérification des règles électriques s'exécute d'abord. La première ligne est le titre ; les éléments suivent, avec les valeurs dans la notation du simulateur.

```console
# test
$ phoenix netlist examples/tutorials/divider.json
divider
~ R1 in out 1e3
~ R2 out 0 3e3
~ V1 in 0 1e1
```

2. Écrivez-la dans un nouveau fichier. Un fichier qui existe n'est jamais écrasé.

```console
# test
$ phoenix netlist examples/tutorials/divider.json --out build/divider.cir
~ written:
```

3. Calculez le point de repos. La formule du diviseur donne 7,5 V au nœud `out`.

```console
# test
$ phoenix sim build/divider.cir --op --probe "v(out)"
name,value
v(out),7.5e0
```

4. Sans fichier : `--schematic` crée la netlist à partir du schéma et la donne directement au solveur.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --op --probe "v(out)"
name,value
v(out),7.5e0
```

5. Balayez la source de 0 V à 10 V par pas de 5 V : la sortie vaut toujours les trois quarts de l'entrée.

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --dc V1 0 10 5 --probe "v(out)"
~ 5e0,3.75e0
~ 1e1,7.5e0
```

## Voir aussi

* [docs/SIM.md](../../SIM.md)
* [docs/NETLIST.md](../../NETLIST.md)
* [docs/ERC.md](../../ERC.md)
