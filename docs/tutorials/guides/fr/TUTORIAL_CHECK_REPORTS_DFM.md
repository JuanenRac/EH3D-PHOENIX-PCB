# Écrire les constats d'une vérification sous forme de tableau et de page, et vérifier une carte avec le profil d'un fabricant

Vous exécutez les règles de conception, conservez les constats sous forme de tableau CSV et de page HTML groupée par catégorie, puis vous vérifiez une petite carte EAGLE avec un profil de capacités d'un fabricant. Le profil de l'exemple est inventé : il montre la forme du fichier et n'est la capacité d'aucun fabricant.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Exécutez les règles d'un fichier de règles et écrivez les deux rapports. Chaque option écrit un nouveau fichier : un fichier existant n'est jamais remplacé, et rien n'est écrit si l'une des sorties est refusée (code de sortie 2). La piste de la carte est plus fine que ce que demande le fichier : un constat confirmé et le code de sortie est 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json --csv build/check.csv --html build/check.html examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. Écrivez les mêmes rapports dans une autre langue. La colonne du titre du CSV et les titres de la page suivent `--lang` ; les clés des colonnes, des catégories et des gravités restent en anglais pour qu'un script les lise dans toutes les langues.

```console
$ phoenix check --lang es --rules examples/tutorials/rules_strict.json --csv build/check_es.csv --html build/check_es.html examples/tutorials/kicad/demo.kicad_pcb
```

3. La comparaison d'un schéma et d'une carte accepte les deux mêmes options.

```console
$ phoenix connectivity --csv build/connectivity.csv --html build/connectivity.html examples/tutorials/tut.sch examples/tutorials/tut.brd
```

4. Vérifiez la carte EAGLE avec le profil `examples/guides/dfm_small_shop.cfg`. Avec `--dfm`, les limites du profil remplacent les règles de la carte ; on ne donne donc pas `--rules`. La carte mesure 30 mm sur 20 mm et le profil autorise 10 mm sur le petit côté : une erreur de code `PHOENIX-DFM-003` et le code de sortie 1. Une carte de plus d'une couche de cuivre reçoit aussi `PHOENIX-DFM-001`, et l'information `PHOENIX-DFM-006` signale que le profil n'est pas vérifié.

```console
$ phoenix check --dfm examples/guides/dfm_small_shop.cfg --csv build/dfm.csv --html build/dfm.html examples/tutorials/tut.brd
```

5. Voici maintenant le profil d'exemple du dépôt, `config/dfm/example.cfg`. La carte est dans toutes ses limites ; le seul constat de la catégorie des capacités est donc l'information que le profil n'est pas vérifié.

```console
$ phoenix check --dfm config/dfm/example.cfg --html build/dfm_example.html examples/tutorials/tut.brd
```

## Comment lire le résultat

* Le CSV a l'en-tête `list,category,severity,code,title,location,x_nm,y_nm,measured_nm,limit_nm,detail` et une ligne par constat, les prouvés d'abord (`list` vaut `confirmed` ou `inconclusive`). Une cellule de texte qu'un tableur exécuterait comme formule reçoit une apostrophe devant.
* La page HTML n'a ni script ni fichier externe et échappe tout texte venant de la carte ; on peut donc l'ouvrir sans risque dans un navigateur. Elle commence par le nombre de constats prouvés et non concluants de chaque catégorie, puis un tableau par catégorie qui a des constats ; ceux du profil sont dans la catégorie `capability`.
* Le profil est un fichier texte de lignes `clé = valeur` en nanomètres ; une ligne absente ou à 0 est désactivée. Copiez `config/dfm/example.cfg`, mettez-y les chiffres lus dans la documentation de votre fabricant, notez dans `source` où vous les avez lus et seulement alors mettez `verified = true`. Rien ne vérifie le fichier auprès d'un fabricant.

## Voir aussi

* [docs/DRC_ADVANCED.md](../../../../docs/DRC_ADVANCED.md)
* [docs/DFM.md](../../../../docs/DFM.md)
* [docs/RULES.md](../../../../docs/RULES.md)
