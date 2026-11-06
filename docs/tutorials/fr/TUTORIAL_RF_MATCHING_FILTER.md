# Adapter une impédance et synthétiser un filtre

Les outils RF travaillent avec des formules de manuels et des composants idéaux. Vous concevez un réseau en L qui adapte 100 Ω à 50 Ω à 100 MHz, un transformateur quart d'onde et des filtres en échelle à partir des tables de prototypes. Les résultats sont un prototype jamais comparé à un analyseur de réseau ni à un solveur de champ.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les tutoriels](README.md).

1. Adaptez une charge de 100 Ω à une source de 50 Ω à 0,1 GHz. Le réseau en L a une bobine série de 79,5775 nH et un condensateur parallèle de 15,9155 pF en passe-bas, et la forme duale en passe-haut.

```console
# test
$ phoenix rf match l --source 50 --load 100 --freq-ghz 0.1
~ 79.5775 nH
~ 15.9155 pF
~ low-pass
~ high-pass
```

2. Un transformateur quart d'onde entre 50 Ω et une charge de 200 Ω a pour impédance √(50 × 200) = 100 Ω.

```console
# test
$ phoenix rf match qw --load 200 --z0 50 --freq-ghz 1
~ transformer: 100.0000 ohm
```

3. Synthétisez un filtre passe-bas : Butterworth, ordre 3, coupure à 0,1 GHz, avec un balayage de trois points. Les valeurs du prototype sont 1, 2 et 1 : le filtre a des condensateurs de 31,8310 pF et, entre eux, une bobine de 159,1549 nH.

```console
# test
$ phoenix rf filter butterworth 3 lowpass --fc-ghz 0.1 --sweep-ghz 0.01,0.2,3
~ prototype g: 1.0000 2.0000 1.0000
~ capacitor 31.8310 pF
~ inductor 159.1549 nH
~ frequency, insertion loss, return loss
```

4. Un filtre passe-bande : Tchebychev, ordre 5, ondulation de 0,1 dB, de 90 MHz à 110 MHz, dans un système de 75 Ω.

```console
# test
$ phoenix rf filter chebyshev 5 bandpass --f1-ghz 0.09 --f2-ghz 0.11 --ripple-db 0.1 --z0 75
~ Chebyshev (0.1 dB ripple) filter of order 5, band-pass
```

5. Dessinez un réseau d'adaptation sur une carte à partir de l'empilage et de la fréquence, en opérations réversibles avec un rapport. La liste des structures (lignes, coudes, stubs, filtres, coupleurs, antennes) vient avec `--list`.

```console
$ phoenix rfgen --list
```

## Voir aussi

* [docs/RF.md](../../RF.md)
* [docs/RFGEN.md](../../RFGEN.md)
* [docs/STACKUP.md](../../STACKUP.md)
* [docs/ORACLE.md](../../ORACLE.md)
