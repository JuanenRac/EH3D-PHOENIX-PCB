# Arrondir les angles d'un plan de cuivre et écrire des dégagements thermiques et des limites d'îlots dans les fichiers Gerber

La carte `examples/guides/pour.brd` a un plan de masse sur la couche `Top` autour de deux résistances et d'une piste. Vous le remplissez avec des angles vifs puis arrondis et vous comparez, puis vous écrivez les fichiers Gerber avec un profil qui donne un dégagement thermique aux pastilles rectangulaires du plan et omet ses petits îlots. Le fichier de règles `rules_relaxed.json` est donné parce qu'une carte EAGLE sans règles de conception n'a pas de limites propres.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Remplissez la zone avec les réglages du dépôt, qui laissent tous les angles vifs (`corner_radius_nm = 0`). Le rapport donne les polygones et les nanomètres carrés de cuivre.

```console
$ phoenix zones fill --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

2. Remplissez-la à nouveau avec `zones_round.cfg`, qui fixe `corner_radius_nm = 1000000` (un arc de 1 mm). Les angles du plan tournés vers l'extérieur et les angles des vides du dégagement thermique sont arrondis ; comparez la surface de cuivre avec la première exécution.

```console
$ phoenix zones fill --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

3. Mesurez le remplissage arrondi par rapport à la carte et aux règles. L'arrondi retire du cuivre au plan ou n'en ajoute qu'à l'intérieur d'un vide de dégagement thermique ; le remplissage ne se rapproche donc pas du cuivre d'un autre réseau et la vérification ne trouve aucune erreur de dégagement.

```console
$ phoenix zones check --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

4. Écrivez d'abord les fichiers Gerber avec le profil générique. La pastille `R2.2` du réseau de masse est dans le plan et reste reliée à lui sans dégagement.

```console
$ phoenix export --out build/pour_plain examples/guides/pour.brd
```

5. Maintenant avec le profil `pour` de `export_pour.json` : quatre bras (`gerber.zone_thermal_spokes`), un vide et un bras de 0,25 mm, le même dégagement pour les pastilles rectangulaires et oblongues (`gerber.zone_thermal_non_round`) et aucun îlot sous 1 millimètre carré (`gerber.zone_island_min_area_nm2`). Comparez le fichier de cuivre de la couche `Top` dans les deux dossiers : le second a les vides du dégagement écrits comme régions de polarité claire autour de la pastille.

```console
$ phoenix export --profiles examples/guides/export_pour.json --profile pour --out build/pour_relief examples/guides/pour.brd
```

## Comment lire le résultat

* Dans les réglages du remplissage, `corner_radius_nm` est le rayon de l'arc (zéro laisse tous les angles vifs) et `corner_tolerance_nm` l'écart que les cordes de l'arc peuvent avoir avec lui. Un arc prend au plus la moitié de chaque côté contre l'angle ; un rayon qui ne tient pas est réduit. Les angles des trous que d'autre cuivre laisse dans le plan ne sont pas arrondis.
* À l'export, un dégagement a quatre bras ou deux (`gerber.zone_thermal_spokes` vaut 0, 2 ou 4 ; 0 laisse les pastilles reliées en plein). Une pastille tournée d'un angle non droit reste en plein et le signale (`PHOENIX-EXPORT-022`). Un îlot plus petit que `gerber.zone_island_min_area_nm2` n'est pas écrit et le rapport le dit (`PHOENIX-EXPORT-021`) ; zéro écrit toutes les zones.

## Voir aussi

* [docs/ZONES.md](../../../../docs/ZONES.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
