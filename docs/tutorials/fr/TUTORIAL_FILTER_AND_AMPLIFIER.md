# Simuler un filtre RC et un amplificateur inverseur

Deux circuits écrits comme netlists de style SPICE : un passe-bas RC de 1 kΩ et 1 µF (`rc_step.cir` et `rc_ac.cir`) et un amplificateur inverseur de gain −R2/R1 avec un amplificateur opérationnel (`inv_amp.cir`). Vous prenez des mesures avec `--measure`, vous parcourez une liste de valeurs avec `--step`, vous trouvez la fréquence de coupure et le pôle, et vous faites une étude de Monte Carlo avec une graine fixe.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les tutoriels](README.md).

1. Mesurez le temps de montée de 10 % à 90 % de la réponse à l'échelon. Le tableau a la colonne `run` et une colonne pour la mesure. Pour un circuit RC, c'est ln 9 · R · C, environ 2,197 ms.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 10u 10m --measure "tran rise10 rise v(out)"
run,rise10
~ nominal,
```

2. Exécutez le circuit une fois pour chaque valeur d'une résistance avec `--step`. Avec 2 kΩ le temps de montée double. Le tableau a une ligne par valeur.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 20u 40m --step "R1 list 1k 2k" --measure "tran rise10 rise v(out)"
r1,run,rise10
```

3. Balayez la fréquence et demandez les figures de mérite de la réponse. La fréquence de coupure du passe-bas est 1 / (2π R C), environ 159 Hz.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 100 1 100k --figures "v(out)"
name,value,unit
~ cutoff_3db,
```

4. Ajustez les pôles et les zéros à la réponse alternative. Le pôle de ce circuit est réel et vaut −1 / (R C) = −1000 rad/s.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 20 10 100k --poles "v(out)"
~ pole,
```

5. Voici l'amplificateur inverseur. Avec R1 = 1 kΩ et R2 = 10 kΩ le gain est −10 ; une entrée de 1 V donne −9,999 V en sortie, un peu moins de 10 V en valeur absolue, parce que le gain du modèle d'amplificateur opérationnel est fini (1e5).

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --op --probe "v(out)"
name,value
~ v(out),-9.99
```

6. Parcourez trois valeurs de R2 et mesurez la sortie au seul point d'un balayage continu. Les gains sont proches de −2, −10 et −20.

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --dc V1 1 1 1 --step "R2 list 2k 10k 20k" --measure "dc vout at v(out) 1"
r2,run,vout
```

7. Monte Carlo : un diviseur dont les résistances ont une tolérance de 1 % (`tol=1%` dans la netlist). La même graine donne le même tableau sur tout ordinateur. Après les exécutions viennent les lignes `count`, `mean`, `std`, `min`, `max` et les percentiles.

```console
# test
$ phoenix sim examples/tutorials/div_tol.cir --mc 200 --seed 5
run,vb
```

## Voir aussi

* [docs/SIM.md](../../SIM.md)
* [docs/SIM_ADVANCED.md](../../SIM_ADVANCED.md)
* [docs/SIM_NONLINEAR.md](../../SIM_NONLINEAR.md)
* [docs/ORACLE.md](../../ORACLE.md)
