# Grouper et exporter la nomenclature, et importer celle d'un autre programme

La carte `examples/guides/bom_four.brd` a trois résistances de 10 k et une de 4k7. Vous choisissez, dans le fichier de réglages, comment les composants sont groupés et quelles colonnes a le fichier CSV, puis vous lisez une nomenclature écrite par un autre programme avec une table de ses colonnes. Rien n'est téléchargé d'un réseau et le tarif des réglages n'est qu'un exemple de la forme d'un devis.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Listez la nomenclature avec le groupement par défaut (valeur, empreinte, fabricant et référence fabricant) et les données de `bom_parts.csv`. R1 et R2 partagent fabricant et référence, R3 a un autre fabricant : il y a donc trois lignes. Sans fichier de prix, chaque ligne est signalée par `PHOENIX-BOM-005`.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

2. La même carte avec `group_by = value,footprint` dans `bom_costs_joined.cfg` : le fabricant et la référence sortent du groupement, et R1, R2 et R3 deviennent une ligne de quantité 3 qui n'a ni l'un ni l'autre.

```console
$ phoenix bom --config examples/guides/bom_costs_joined.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

3. Écrivez le fichier CSV. Ses colonnes sont les `csv_columns` des réglages, dans cet ordre (ici `references,quantity,value,footprint,manufacturer,mpn,status`) ; la page HTML et le fichier JSON gardent toutes les colonnes. Un fichier existant n'est jamais remplacé.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv --csv build/bom.csv examples/guides/bom_four.brd
```

4. Importez la nomenclature d'un autre programme. `bom_other_program.csv` utilise des points-virgules et ses propres noms de colonnes ; `bom_import_map.cfg` dit comment la lire. Le rapport compare la nomenclature à la carte : R5 est dans la nomenclature mais pas sur la carte (`PHOENIX-BOM-021`), et R4 est marqué non monté ; il est donc retiré de la nomenclature et listé comme information (`PHOENIX-BOM-009`). `--parts-out` enregistre ce que la nomenclature dit des fabricants et des références sous forme de fichier de données de composants.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --import examples/guides/bom_other_program.csv --import-map examples/guides/bom_import_map.cfg --parts-out build/parts_from_bill.csv examples/guides/bom_four.brd
```

5. Utilisez le fichier de données de composants enregistré dans les exécutions suivantes à la place de la nomenclature d'origine.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts build/parts_from_bill.csv examples/guides/bom_four.brd
```

## Comment lire le résultat

* Le fichier de réglages par défaut est `config/costs.cfg` et la table par défaut `config/bom_import.cfg`, s'ils existent. Une table indique le délimiteur (virgule, point-virgule ou tabulation) et, pour chaque colonne, les noms qu'elle peut avoir dans le fichier ; seules les références sont obligatoires. Les noms fournis sont une supposition et ne sont le format d'aucun programme : copiez le fichier et écrivez les noms de votre propre fichier.
* Les codes `PHOENIX-BOM-019`, `PHOENIX-BOM-020` et `PHOENIX-BOM-021` sont des avertissements et ne changent rien : une quantité qui n'est pas le nombre de références de sa ligne, une référence dans deux lignes (la première est utilisée) et un composant que seul un côté a ou dont la valeur diffère. Une plage comme `R1-R5` est développée en ses composants individuels ; une plage mal formée, inversée ou plus longue que `max_range` arrête l'import (`PHOENIX-BOM-022` à `PHOENIX-BOM-024`). Un tiret après le nom d'une pièce dont la fin n'est pas un nombre (`C1-A`, `J1-B`) est conservé comme une seule référence et signalé seulement par `PHOENIX-BOM-022`. `--import-format` lit aussi une liste de pièces d'EAGLE ou une nomenclature de KiCad (`docs/BOM.md`). Ce que dit `--parts` l'emporte sur la nomenclature importée.
* Sans `--csv`, `--json-file` ni `--html`, la commande n'écrit rien. Le code de sortie est 0 si tout va bien, 1 si un composant nomme une empreinte absente de la carte et 2 si les arguments ou l'entrée sont inutilisables.

## Voir aussi

* [docs/BOM.md](../../../../docs/BOM.md)
* [docs/PARTS.md](../../../../docs/PARTS.md)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
