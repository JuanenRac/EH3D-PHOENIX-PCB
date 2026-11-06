# Guide de migration : si vous venez d'EAGLE ou de KiCad

Cette page place les notions et commandes d'EAGLE et de KiCad à côté de celles de ce programme, dit ce qui est lu et écrit, et ce qui est perdu. Le programme est un prototype : rien de ce qui suit n'a été comparé à EAGLE ou KiCad sur de vrais projets. Commencez par `phoenix help`.

## Commandes équivalentes

| Ce que vous faites | Dans EAGLE | Dans KiCad | Ici |
|---|---|---|---|
| Vérification des règles de conception | DRC | Design Rules Checker | `phoenix check`, `phoenix check --advanced` |
| Vérification des règles électriques | ERC | Electrical Rules Checker | `phoenix erc` |
| Carte face au schéma | Forward & Back Annotation | Update PCB from Schematic | `phoenix connectivity`, `phoenix sync`, `phoenix annotate` |
| Connexions à router | RATSNEST | ratsnest | `phoenix ratsnest` |
| Router | ROUTE, AUTO | interactive router, Freerouting | `phoenix route`, `phoenix boardedit`, `phoenix autoroute` |
| Fichiers de fabrication | CAM Processor | Plot, Fabrication Outputs | `phoenix export`, `phoenix fabpack` |
| Nomenclature et placement | ULP | BOM and position files | `phoenix bom`, `phoenix pnp`, `phoenix stencil` |
| Panneau | ULP | KiKit and similar | `phoenix panel` |
| Exécuter un script | SCRIPT, RUN | Python console | `phoenix script run` |
| Éditer une bibliothèque | Library editor | Symbol and Footprint editors | `phoenix libedit`, `phoenix footprint`, `phoenix ipc` |
| Simuler | SPICE export | ngspice | `phoenix netlist`, `phoenix sim` |

## Ce qui est lu et écrit

| Fichier | Lecture | Écriture |
|---|---|---|
| EAGLE `.sch`, `.brd`, `.lbr` (XML) | `eagle-check` et toute commande qui prend une carte ou un schéma | cartes et schémas en XML EAGLE: `boardedit`, `sync --apply`, `annotate --apply`; bibliothèques: `libedit`, `footprint`, `ipc` |
| EAGLE binary | non | non |
| EAGLE `.dru` | `check --rules-dru`, `eagle-convert dru-to-rules` | `eagle-convert rules-to-dru` |
| EAGLE `.cam` | `cam-import`, `eagle-convert cam-to-profile` | - |
| EAGLE `.scr` | `script run`, `script report` | - |
| EAGLE ULP | non | - |
| KiCad 6 - 9 `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_sym`, `.kicad_mod` | `kicad-read`, `kicad-import`; `.kicad_pcb` et `.kicad_sch` aussi dans les commandes qui prennent une carte ou un schéma | KiCad 8: `kicad-export` |
| Gerber X2, Excellon | `gerber-import`, `gerber-check` | `export`, `fabpack`, `panel` |
| DXF R12 | `dxf-import` | `dxf-export` |
| Specctra DSN, SES | SES: `autoroute` | DSN: `autoroute` |

## Ce qui est perdu

* Le modèle de conception ne contient pas les graphiques KiCad, les bus, les marques de non-connexion, la configuration d'une carte, le cuivre rempli des zones, les zones d'exclusion ni les modèles 3D ; les lecteurs les comptent comme laissés de côté et le disent dans le rapport.
* EAGLE : les fichiers binaires ne sont pas lus, les programmes ULP ne s'exécutent pas, et `phoenix eagle-check` liste les éléments que le modèle simplifie, avec des codes en huit langues.
* Rien n'est écrit sur une entrée : une conversion crée toujours un nouveau fichier.

## Premiers pas

1. Lisez votre projet avec `phoenix kicad-import` (KiCad) ou `phoenix eagle-check` (EAGLE) et lisez le rapport.
2. Vérifiez-le avec `phoenix check --rules FICHIER` (les limites de vos règles, ou `phoenix eagle-convert dru-to-rules` pour un fichier de règles EAGLE) et `phoenix erc`.
3. Écrivez les fichiers de fabrication avec `phoenix export` et comparez-les à ceux de votre outil habituel avant de leur faire confiance.

## Voir aussi

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD_EXPORT.md](../../KICAD_EXPORT.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/EXPORT.md](../../EXPORT.md)
* [Glossaire](../../glossary/fr/GLOSSARY.md)
* [Tutoriels](../../tutorials/fr/README.md)
* [English](../../MIGRATION.md)
